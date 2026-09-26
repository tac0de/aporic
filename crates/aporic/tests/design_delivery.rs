use std::{fs, path::Path};

use aporic::design::{DesignValidateRequest, validate};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const KINDS: [&str; 9] = [
    "brief",
    "reference_notes",
    "concept",
    "tokens",
    "components",
    "responsive",
    "asset_manifest",
    "implementation",
    "browser_review",
];

fn package(workspace: &Path) -> Value {
    fs::create_dir_all(workspace.join("design")).unwrap();
    let artifacts = KINDS
        .iter()
        .map(|kind| {
            let path = format!("design/{kind}.md");
            let contents = format!("{kind} evidence\n");
            fs::write(workspace.join(&path), &contents).unwrap();
            json!({
                "kind": kind,
                "label": if *kind == "concept" { "A" } else { *kind },
                "path": path,
                "sha256": format!("{:x}", Sha256::digest(contents.as_bytes()))
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema_version": 1,
        "task_id": "task-21",
        "objective": "Explain Aporic clearly",
        "target_user": "Potential contributor",
        "references": [{
            "url": "https://example.com/design",
            "observed_at": "2026-09-26T09:00:00+09:00",
            "useful_principle": "Clear reading order",
            "adaptation": "Short proof cards"
        }],
        "selected_direction": {
            "concept_label": "A",
            "source": "reported_user_choice",
            "rationale": "User preferred A"
        },
        "artifacts": artifacts
    })
}

fn write_manifest(workspace: &Path, manifest: &Value) {
    fs::write(
        workspace.join("design/manifest.json"),
        serde_json::to_vec_pretty(manifest).unwrap(),
    )
    .unwrap();
}

fn request(workspace: &Path) -> DesignValidateRequest {
    DesignValidateRequest {
        workspace: workspace.to_string_lossy().into_owned(),
        manifest: "design/manifest.json".into(),
    }
}

#[test]
fn complete_package_reports_local_integrity_without_claiming_approval() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path();
    write_manifest(workspace, &package(workspace));

    let report = validate(&request(workspace)).unwrap();
    assert!(report.design_artifacts_complete);
    assert!(report.delivery_artifacts_complete);
    assert!(report.all_declared_artifacts_match);
    assert_eq!(report.artifact_checks.len(), 9);
    assert!(report.notice.contains("unverified claims"));

    let cli = std::process::Command::new(env!("CARGO_BIN_EXE_aporic"))
        .args([
            "design",
            "validate",
            "--workspace",
            workspace.to_str().unwrap(),
            "--manifest",
            "design/manifest.json",
        ])
        .output()
        .unwrap();
    assert!(
        cli.status.success(),
        "{}",
        String::from_utf8_lossy(&cli.stderr)
    );
    let from_cli: Value = serde_json::from_slice(&cli.stdout).unwrap();
    assert_eq!(from_cli["manifest_sha256"], report.manifest_sha256);
    assert_eq!(from_cli["delivery_artifacts_complete"], true);
}

#[test]
fn missing_and_changed_files_are_visible_and_incomplete() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path();
    write_manifest(workspace, &package(workspace));
    fs::remove_file(workspace.join("design/browser_review.md")).unwrap();
    fs::write(workspace.join("design/tokens.md"), "changed\n").unwrap();

    let report = validate(&request(workspace)).unwrap();
    assert!(!report.design_artifacts_complete);
    assert!(!report.delivery_artifacts_complete);
    assert!(!report.all_declared_artifacts_match);
    assert_eq!(
        report.missing_design_artifacts,
        vec![aporic::design::DesignArtifactKind::Tokens]
    );
    assert_eq!(
        report.missing_delivery_artifacts,
        vec![aporic::design::DesignArtifactKind::BrowserReview]
    );
    let statuses = serde_json::to_value(&report.artifact_checks).unwrap();
    assert!(
        statuses
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["status"] == "missing")
    );
    assert!(
        statuses
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["status"] == "digest_mismatch")
    );
}

#[test]
fn design_stage_can_complete_before_implementation_and_browser_review() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path();
    write_manifest(workspace, &package(workspace));
    fs::remove_file(workspace.join("design/implementation.md")).unwrap();
    fs::remove_file(workspace.join("design/browser_review.md")).unwrap();

    let report = validate(&request(workspace)).unwrap();
    assert!(report.design_artifacts_complete);
    assert!(!report.delivery_artifacts_complete);
}

#[test]
fn unsupported_schema_and_parent_path_are_rejected() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path();
    let mut manifest = package(workspace);
    manifest["schema_version"] = json!(2);
    write_manifest(workspace, &manifest);
    assert!(
        validate(&request(workspace))
            .unwrap_err()
            .to_string()
            .contains("schema_version")
    );

    manifest["schema_version"] = json!(1);
    manifest["artifacts"][0]["path"] = json!("../outside.md");
    write_manifest(workspace, &manifest);
    assert!(
        validate(&request(workspace))
            .unwrap_err()
            .to_string()
            .contains("workspace-relative")
    );
}

#[cfg(unix)]
#[test]
fn symlink_to_external_file_is_rejected() {
    use std::os::unix::fs::symlink;

    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let mut manifest = package(&workspace);
    let external = area.path().join("external.md");
    fs::write(&external, "external").unwrap();
    symlink(&external, workspace.join("design/external.md")).unwrap();
    manifest["artifacts"][0]["path"] = json!("design/external.md");
    write_manifest(&workspace, &manifest);

    assert!(
        validate(&request(&workspace))
            .unwrap_err()
            .to_string()
            .contains("escapes workspace")
    );
}
