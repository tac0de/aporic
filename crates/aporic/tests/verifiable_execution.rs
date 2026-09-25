use std::path::PathBuf;

use aporic::{
    Hub,
    domain::{
        CommandSpecRequest, CriterionProof, ExecutionGetRequest, ExecutionListRequest,
        ExecutionStatus, OpenRequest, TaskClaimRequest, TaskCompleteRequest, TaskCreateRequest,
        TaskStatus,
    },
};

fn executable(candidates: &[&str]) -> String {
    candidates
        .iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
        .unwrap_or_else(|| panic!("none of the test executables exist: {candidates:?}"))
        .to_string_lossy()
        .into_owned()
}

fn setup(objective: &str) -> (tempfile::TempDir, PathBuf, PathBuf, Hub, String) {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let session = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: objective.to_owned(),
            idempotency_key: format!("open-{objective}"),
        })
        .unwrap();
    (area, workspace, database, hub, session.session_id)
}

fn register(
    hub: &Hub,
    session_id: &str,
    program: String,
    args: Vec<String>,
    timeout_seconds: u64,
    artifact_paths: Vec<String>,
    key: &str,
) -> aporic::domain::CommandSpecOutcome {
    hub.register_command_spec(&CommandSpecRequest {
        session_id: session_id.to_owned(),
        program,
        args,
        workspace_relative_cwd: ".".to_owned(),
        expected_exit_code: 0,
        timeout_seconds,
        artifact_paths,
        idempotency_key: key.to_owned(),
    })
    .unwrap()
}

#[tokio::test]
async fn successful_runner_receipt_is_the_only_path_to_a_verified_task_proof() {
    let (_area, workspace, _database, hub, session_id) = setup("verified execution");
    let artifact = "proof.txt".to_owned();
    let registered = register(
        &hub,
        &session_id,
        executable(&["/usr/bin/touch", "/bin/touch"]),
        vec![artifact.clone()],
        5,
        vec![artifact.clone()],
        "spec-success",
    );
    assert!(
        hub.register_command_spec(&CommandSpecRequest {
            session_id: session_id.clone(),
            program: registered.spec.program.clone(),
            args: registered.spec.args.clone(),
            workspace_relative_cwd: ".".to_owned(),
            expected_exit_code: 0,
            timeout_seconds: 5,
            artifact_paths: vec![artifact.clone()],
            idempotency_key: "spec-success".to_owned(),
        })
        .unwrap()
        .duplicate
    );

    let task = hub
        .create_task(&TaskCreateRequest {
            session_id: session_id.clone(),
            objective: "Produce a mechanically checked artifact".to_owned(),
            acceptance_criteria: vec![registered.spec.success_claim.clone()],
            write_scope: vec![artifact.clone()],
            depends_on: Vec::new(),
            idempotency_key: "task".to_owned(),
        })
        .unwrap();
    hub.claim_task(&TaskClaimRequest {
        task_id: task.task.task_id.clone(),
        worker_id: "local-runner".to_owned(),
        lease_seconds: 60,
        idempotency_key: "task-claim".to_owned(),
    })
    .unwrap();

    let outcome = hub.verify(&registered.spec.spec_id).await.unwrap();
    assert_eq!(outcome.run.status, ExecutionStatus::Succeeded);
    assert_eq!(outcome.artifacts.len(), 1);
    assert!(workspace.join(&artifact).is_file());
    let claim_id = outcome
        .verified_claim_id
        .clone()
        .expect("runner issued a verified claim");
    let receipt = outcome.receipt.as_ref().expect("runner stored a receipt");
    assert_eq!(
        receipt.command_spec_sha256,
        registered.spec.canonical_sha256
    );
    assert_eq!(receipt.stdout_sha256.len(), 64);

    let completed = hub
        .complete_task(&TaskCompleteRequest {
            task_id: task.task.task_id,
            worker_id: "local-runner".to_owned(),
            outcome_summary: "Artifact produced by the local runner".to_owned(),
            criterion_proofs: vec![CriterionProof {
                criterion: registered.spec.success_claim,
                verified_claim_id: claim_id.clone(),
            }],
            idempotency_key: "task-complete".to_owned(),
        })
        .unwrap();
    assert_eq!(completed.task.status, TaskStatus::Completed);

    let exported = hub.export_project(workspace.to_str().unwrap()).unwrap();
    assert_eq!(exported.command_specs.len(), 1);
    assert_eq!(exported.execution_runs.len(), 1);
    assert_eq!(exported.execution_receipts.len(), 1);
    assert_eq!(exported.receipt_artifacts.len(), 1);
    let runner_claim = exported
        .claims
        .iter()
        .find(|claim| claim.claim_id == claim_id)
        .unwrap();
    assert_eq!(runner_claim.receipt_ids, vec![receipt.receipt_id.clone()]);
    let replay = hub.audit_execution_replay().unwrap();
    assert_eq!(replay.replayed_run_count, 1);
    assert!(replay.mismatches.is_empty());
}

#[tokio::test]
async fn failure_timeout_and_missing_artifact_never_issue_verified_claims() {
    let (_area, workspace, _database, hub, session_id) = setup("negative executions");
    let failed = register(
        &hub,
        &session_id,
        executable(&["/usr/bin/false", "/bin/false"]),
        Vec::new(),
        5,
        Vec::new(),
        "spec-fail",
    );
    let failed = hub.verify(&failed.spec.spec_id).await.unwrap();
    assert_eq!(failed.run.status, ExecutionStatus::Failed);
    assert!(failed.verified_claim_id.is_none());

    let timed = register(
        &hub,
        &session_id,
        executable(&["/bin/sleep", "/usr/bin/sleep"]),
        vec!["2".to_owned()],
        1,
        Vec::new(),
        "spec-timeout",
    );
    let timed = hub.verify(&timed.spec.spec_id).await.unwrap();
    assert_eq!(timed.run.status, ExecutionStatus::TimedOut);
    assert!(timed.verified_claim_id.is_none());

    let missing = register(
        &hub,
        &session_id,
        executable(&["/usr/bin/true", "/bin/true"]),
        Vec::new(),
        5,
        vec!["missing.txt".to_owned()],
        "spec-missing",
    );
    let missing = hub.verify(&missing.spec.spec_id).await.unwrap();
    assert_eq!(missing.run.status, ExecutionStatus::Failed);
    assert_eq!(
        missing.receipt.unwrap().termination,
        "missing_or_invalid_artifact"
    );
    assert!(missing.verified_claim_id.is_none());

    assert_eq!(
        hub.list_executions(&ExecutionListRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            limit: Some(10),
        })
        .unwrap()
        .len(),
        3
    );
}

#[test]
fn registration_rejects_paths_that_escape_the_workspace() {
    let (_area, _workspace, _database, hub, session_id) = setup("path confinement");
    let error = hub
        .register_command_spec(&CommandSpecRequest {
            session_id,
            program: executable(&["/usr/bin/true", "/bin/true"]),
            args: Vec::new(),
            workspace_relative_cwd: "..".to_owned(),
            expected_exit_code: 0,
            timeout_seconds: 5,
            artifact_paths: vec!["../outside".to_owned()],
            idempotency_key: "escape".to_owned(),
        })
        .unwrap_err();
    assert!(error.to_string().contains("workspace-relative"));
}

#[test]
fn stale_running_execution_is_reconciled_as_interrupted_with_an_event() {
    let (_area, _workspace, database, hub, session_id) = setup("interrupted execution");
    let registered = register(
        &hub,
        &session_id,
        executable(&["/usr/bin/true", "/bin/true"]),
        Vec::new(),
        5,
        Vec::new(),
        "spec-stale",
    );
    let run_id = "stale-run";
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute(
            "INSERT INTO execution_runs
         (run_id, spec_id, session_id, status, attempt, started_at_unix_ms)
         VALUES (?1, ?2, ?3, 'running', 1, 0)",
            rusqlite::params![run_id, registered.spec.spec_id, session_id],
        )
        .unwrap();
    drop(connection);

    assert_eq!(hub.reconcile_executions(1).unwrap(), 1);
    let outcome = hub
        .get_execution(&ExecutionGetRequest {
            run_id: run_id.to_owned(),
        })
        .unwrap();
    assert_eq!(outcome.run.status, ExecutionStatus::Interrupted);
    assert!(outcome.receipt.is_none());
    assert_eq!(hub.stats().unwrap().interrupted_execution_count, 1);
    assert!(hub.audit_execution_replay().unwrap().mismatches.is_empty());

    let connection = rusqlite::Connection::open(&database).unwrap();
    let events: u64 = connection.query_row(
        "SELECT COUNT(*) FROM events WHERE kind = 'execution_run_interrupted' AND stream_id = ?1",
        [run_id], |row| row.get(0),
    ).unwrap();
    assert_eq!(events, 1);
}

#[test]
fn cli_executes_a_registered_spec_without_an_mcp_execution_tool() {
    let (_area, _workspace, database, hub, session_id) = setup("cli execution");
    let registered = register(
        &hub,
        &session_id,
        executable(&["/usr/bin/true", "/bin/true"]),
        Vec::new(),
        5,
        Vec::new(),
        "spec-cli",
    );
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_aporic"))
        .args(["verify", "--spec", &registered.spec.spec_id])
        .env("APORIC_DATABASE", &database)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let outcome: aporic::domain::ExecutionOutcome = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(outcome.run.status, ExecutionStatus::Succeeded);
    assert!(outcome.verified_claim_id.is_some());
}

#[tokio::test]
async fn a_spec_cannot_run_concurrently_but_can_be_retried_after_completion() {
    let (_area, _workspace, _database, hub, session_id) = setup("concurrent execution");
    let registered = register(
        &hub,
        &session_id,
        executable(&["/bin/sleep", "/usr/bin/sleep"]),
        vec!["1".to_owned()],
        5,
        Vec::new(),
        "spec-concurrent",
    );
    let first_hub = hub.clone();
    let first_spec = registered.spec.spec_id.clone();
    let first = tokio::spawn(async move { first_hub.verify(&first_spec).await });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let second = hub.verify(&registered.spec.spec_id).await.unwrap_err();
    assert!(second.to_string().contains("already has running execution"));
    let first = first.await.unwrap().unwrap();
    assert_eq!(first.run.status, ExecutionStatus::Succeeded);
    let retry = hub.verify(&registered.spec.spec_id).await.unwrap();
    assert_eq!(retry.run.attempt, 2);
    assert_eq!(
        retry.run.retry_of_run_id.as_deref(),
        Some(first.run.run_id.as_str())
    );
}

#[cfg(unix)]
#[tokio::test]
async fn raw_command_output_is_not_persisted() {
    use std::os::unix::fs::PermissionsExt;

    let (_area, workspace, database, hub, session_id) = setup("output minimization");
    let secret = "raw-output-must-not-enter-aporic-state-7f91c6";
    let script = workspace.join("emit-output");
    std::fs::write(&script, format!("#!/bin/sh\nprintf '%s' '{secret}'\n")).unwrap();
    let mut permissions = std::fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&script, permissions).unwrap();

    let registered = register(
        &hub,
        &session_id,
        "./emit-output".to_owned(),
        Vec::new(),
        5,
        Vec::new(),
        "spec-output",
    );
    let outcome = hub.verify(&registered.spec.spec_id).await.unwrap();
    assert_eq!(outcome.run.status, ExecutionStatus::Succeeded);
    assert_eq!(outcome.receipt.unwrap().stdout_bytes, secret.len() as u64);
    for path in [
        database.clone(),
        PathBuf::from(format!("{}-wal", database.display())),
    ] {
        if let Ok(database_bytes) = std::fs::read(path) {
            assert!(
                !database_bytes
                    .windows(secret.len())
                    .any(|window| window == secret.as_bytes())
            );
        }
    }
}
