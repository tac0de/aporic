use std::{path::Path, process::Command};

use aporic::{
    Hub,
    domain::{GitObserveRequest, GitSnapshotGetRequest, GitSnapshotListRequest, OpenRequest},
};

fn git(workspace: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(workspace)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?}: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn fixture() -> (tempfile::TempDir, std::path::PathBuf, Hub) {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    git(&workspace, &["init", "-b", "main"]);
    std::fs::write(workspace.join("README.md"), "initial\n").unwrap();
    git(&workspace, &["add", "README.md"]);
    git(
        &workspace,
        &[
            "-c",
            "user.name=Aporic Test",
            "-c",
            "user.email=aporic@example.invalid",
            "commit",
            "-m",
            "initial",
        ],
    );
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    hub.open_session(&OpenRequest {
        workspace: workspace.to_string_lossy().into_owned(),
        objective: "Observe Git without mutating it".to_owned(),
        idempotency_key: "git-open".to_owned(),
    })
    .unwrap();
    (area, workspace, hub)
}

#[test]
fn migrates_v8_to_v9_without_git_snapshots() {
    let area = tempfile::tempdir().unwrap();
    let database = area.path().join("aporic.sqlite3");
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute_batch(include_str!("../../../migrations/0001_initial.sql"))
        .unwrap();
    connection
        .execute_batch(include_str!(
            "../../../migrations/0006_authority_bound_context.sql"
        ))
        .unwrap();
    connection
        .execute_batch(include_str!(
            "../../../migrations/0007_memory_lifecycle.sql"
        ))
        .unwrap();
    connection
        .execute_batch(include_str!("../../../migrations/0008_runtime_trace.sql"))
        .unwrap();
    drop(connection);
    let hub = Hub::open(database).unwrap();
    assert_eq!(hub.stats().unwrap().schema_version, 13);
    let audit = hub.audit_git_snapshots().unwrap();
    assert_eq!(audit.snapshot_count, 0);
    assert!(audit.consistent);
}

#[test]
fn observation_is_append_only_and_does_not_mutate_git() {
    let (_area, workspace, hub) = fixture();
    let head_before = git(&workspace, &["rev-parse", "HEAD"]);
    let status_before = git(&workspace, &["status", "--porcelain=v1"]);
    let snapshot = hub
        .observe_git(&GitObserveRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            base_ref: None,
        })
        .unwrap();
    assert_eq!(snapshot.head_commit.as_deref(), Some(head_before.as_str()));
    assert_eq!(snapshot.branch.as_deref(), Some("main"));
    assert!(!snapshot.detached);
    assert!(!snapshot.dirty);
    assert!(!snapshot.remote_state_fresh);
    assert!(!snapshot.approval_proven);
    assert!(!snapshot.successful_receipt_bound);
    assert!(snapshot.snapshot_sha256.len() == 64);
    assert_eq!(git(&workspace, &["rev-parse", "HEAD"]), head_before);
    assert_eq!(
        git(&workspace, &["status", "--porcelain=v1"]),
        status_before
    );

    let listed = hub
        .list_git_snapshots(&GitSnapshotListRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            limit: Some(10),
        })
        .unwrap();
    assert_eq!(listed.as_slice(), std::slice::from_ref(&snapshot));
    assert_eq!(
        hub.get_git_snapshot(&GitSnapshotGetRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            snapshot_id: snapshot.snapshot_id.clone(),
        })
        .unwrap(),
        snapshot
    );
    assert!(hub.audit_git_snapshots().unwrap().consistent);
}

#[test]
fn governance_detects_sensitive_dirty_changes_without_claiming_approval() {
    let (_area, workspace, hub) = fixture();
    let workflow = workspace.join(".github/workflows/check.yml");
    std::fs::create_dir_all(workflow.parent().unwrap()).unwrap();
    std::fs::write(&workflow, "name: untrusted\n").unwrap();
    let snapshot = hub
        .observe_git(&GitObserveRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            base_ref: Some("main".to_owned()),
        })
        .unwrap();
    let codes = snapshot
        .findings
        .iter()
        .map(|finding| finding.code.as_str())
        .collect::<Vec<_>>();
    assert!(snapshot.dirty);
    assert!(codes.contains(&"dirty_worktree"));
    assert!(codes.contains(&"governance_sensitive_paths_changed"));
    assert!(codes.contains(&"no_successful_receipt_bound_to_head"));
    assert!(!snapshot.approval_proven);
    assert!(
        snapshot
            .changed_paths
            .iter()
            .any(|change| change.path == ".github/workflows/check.yml"
                && change.governance_sensitive)
    );
}

#[test]
fn option_shaped_base_ref_is_rejected_without_running_it() {
    let (_area, workspace, hub) = fixture();
    let error = hub
        .observe_git(&GitObserveRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            base_ref: Some("--upload-pack=evil".to_owned()),
        })
        .unwrap_err();
    assert!(error.to_string().contains("safe Git ref"));
    assert_eq!(hub.stats().unwrap().git_snapshot_count, 0);
}

#[test]
fn snapshot_audit_detects_post_capture_tampering() {
    let (_area, workspace, hub) = fixture();
    hub.observe_git(&GitObserveRequest {
        workspace: workspace.to_string_lossy().into_owned(),
        base_ref: None,
    })
    .unwrap();
    let connection = rusqlite::Connection::open(hub.database_path()).unwrap();
    connection
        .execute("UPDATE git_snapshots SET branch = 'tampered'", [])
        .unwrap();
    let audit = hub.audit_git_snapshots().unwrap();
    assert_eq!(audit.snapshot_count, 1);
    assert_eq!(audit.digest_mismatch_count, 1);
    assert!(!audit.consistent);
}
