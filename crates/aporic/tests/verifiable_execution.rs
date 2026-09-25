use std::path::PathBuf;

use aporic::{
    Hub,
    domain::{
        CommandSpecRequest, CriterionProof, ExecutionGetRequest, ExecutionListRequest,
        ExecutionSandboxProfile, ExecutionStatus, OpenRequest, SandboxEnforcement,
        SandboxNetworkAccess, SandboxWorkspaceAccess, TaskClaimRequest, TaskCompleteRequest,
        TaskCreateRequest, TaskStatus,
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

#[cfg(unix)]
fn success_command() -> (String, Vec<String>) {
    (executable(&["/usr/bin/true", "/bin/true"]), Vec::new())
}

#[cfg(windows)]
fn success_command() -> (String, Vec<String>) {
    (
        std::env::var("COMSPEC").expect("Windows provides COMSPEC"),
        vec!["/D".to_owned(), "/C".to_owned(), "exit 0".to_owned()],
    )
}

#[cfg(unix)]
fn failure_command() -> (String, Vec<String>) {
    (executable(&["/usr/bin/false", "/bin/false"]), Vec::new())
}

#[cfg(windows)]
fn failure_command() -> (String, Vec<String>) {
    (
        std::env::var("COMSPEC").expect("Windows provides COMSPEC"),
        vec!["/D".to_owned(), "/C".to_owned(), "exit 1".to_owned()],
    )
}

#[cfg(unix)]
fn create_file_command(path: &str) -> (String, Vec<String>) {
    (
        executable(&["/usr/bin/touch", "/bin/touch"]),
        vec![path.to_owned()],
    )
}

#[cfg(windows)]
fn create_file_command(path: &str) -> (String, Vec<String>) {
    (
        std::env::var("COMSPEC").expect("Windows provides COMSPEC"),
        vec![
            "/D".to_owned(),
            "/C".to_owned(),
            format!("type nul > {path}"),
        ],
    )
}

#[cfg(unix)]
fn sleep_command(seconds: u64) -> (String, Vec<String>) {
    (
        executable(&["/bin/sleep", "/usr/bin/sleep"]),
        vec![seconds.to_string()],
    )
}

#[cfg(windows)]
fn sleep_command(seconds: u64) -> (String, Vec<String>) {
    (
        std::env::var("COMSPEC").expect("Windows provides COMSPEC"),
        vec![
            "/D".to_owned(),
            "/C".to_owned(),
            format!("ping 127.0.0.1 -n {} > NUL", seconds.saturating_add(1)),
        ],
    )
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
        sandbox_profile: Default::default(),
        idempotency_key: key.to_owned(),
    })
    .unwrap()
}

fn required_sandbox(workspace_access: SandboxWorkspaceAccess) -> ExecutionSandboxProfile {
    ExecutionSandboxProfile {
        enforcement: SandboxEnforcement::Required,
        workspace_access,
        network_access: SandboxNetworkAccess::Deny,
    }
}

#[tokio::test]
async fn successful_runner_receipt_is_the_only_path_to_a_verified_task_proof() {
    let (_area, workspace, _database, hub, session_id) = setup("verified execution");
    let artifact = "proof.txt".to_owned();
    let (program, args) = create_file_command(&artifact);
    let registered = register(
        &hub,
        &session_id,
        program,
        args,
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
            sandbox_profile: Default::default(),
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
    assert_eq!(
        outcome.run.status,
        ExecutionStatus::Succeeded,
        "host runner outcome: {outcome:?}"
    );
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
    let (program, args) = failure_command();
    let failed = register(&hub, &session_id, program, args, 5, Vec::new(), "spec-fail");
    let failed = hub.verify(&failed.spec.spec_id).await.unwrap();
    assert_eq!(failed.run.status, ExecutionStatus::Failed);
    assert!(failed.verified_claim_id.is_none());

    let (program, args) = sleep_command(2);
    let timed = register(
        &hub,
        &session_id,
        program,
        args,
        1,
        Vec::new(),
        "spec-timeout",
    );
    let timed = hub.verify(&timed.spec.spec_id).await.unwrap();
    assert_eq!(timed.run.status, ExecutionStatus::TimedOut);
    assert!(timed.verified_claim_id.is_none());

    let (program, args) = success_command();
    let missing = register(
        &hub,
        &session_id,
        program,
        args,
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

#[cfg(unix)]
#[tokio::test]
async fn timeout_terminates_the_normal_child_process_group() {
    use std::{thread, time::Duration};

    let (_area, workspace, _database, hub, session_id) = setup("process group timeout");
    let registered = register(
        &hub,
        &session_id,
        executable(&["/bin/sh"]),
        vec![
            "-c".to_owned(),
            "sleep 30 & echo $! > child.pid; wait".to_owned(),
        ],
        1,
        Vec::new(),
        "spec-process-group",
    );
    let outcome = hub.verify(&registered.spec.spec_id).await.unwrap();
    assert_eq!(outcome.run.status, ExecutionStatus::TimedOut);

    let child_pid = std::fs::read_to_string(workspace.join("child.pid"))
        .unwrap()
        .trim()
        .parse::<i32>()
        .unwrap();
    let gone = (0..20).any(|_| {
        let result = unsafe { libc::kill(child_pid, 0) };
        if result == -1 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
            true
        } else {
            thread::sleep(Duration::from_millis(50));
            false
        }
    });
    assert!(gone, "child process {child_pid} survived the timeout");
}

#[test]
fn registration_rejects_paths_that_escape_the_workspace() {
    let (_area, _workspace, _database, hub, session_id) = setup("path confinement");
    let (program, args) = success_command();
    let error = hub
        .register_command_spec(&CommandSpecRequest {
            session_id,
            program,
            args,
            workspace_relative_cwd: "..".to_owned(),
            expected_exit_code: 0,
            timeout_seconds: 5,
            artifact_paths: vec!["../outside".to_owned()],
            sandbox_profile: Default::default(),
            idempotency_key: "escape".to_owned(),
        })
        .unwrap_err();
    assert!(error.to_string().contains("workspace-relative"));
}

#[test]
fn registration_rejects_excessive_artifact_count() {
    let (_area, _workspace, _database, hub, session_id) = setup("artifact count limit");
    let (program, args) = success_command();
    let error = hub
        .register_command_spec(&CommandSpecRequest {
            session_id,
            program,
            args,
            workspace_relative_cwd: ".".to_owned(),
            expected_exit_code: 0,
            timeout_seconds: 5,
            artifact_paths: (0..65).map(|index| format!("artifact-{index}")).collect(),
            sandbox_profile: Default::default(),
            idempotency_key: "too-many-artifacts".to_owned(),
        })
        .unwrap_err();
    assert!(error.to_string().contains("64-item limit"));
}

#[test]
fn registration_rejects_profiles_that_claim_unenforced_restrictions() {
    let (_area, _workspace, _database, hub, session_id) = setup("sandbox profile validation");
    let (program, args) = success_command();
    let error = hub
        .register_command_spec(&CommandSpecRequest {
            session_id,
            program,
            args,
            workspace_relative_cwd: ".".to_owned(),
            expected_exit_code: 0,
            timeout_seconds: 5,
            artifact_paths: Vec::new(),
            sandbox_profile: ExecutionSandboxProfile {
                enforcement: SandboxEnforcement::Host,
                workspace_access: SandboxWorkspaceAccess::ReadWrite,
                network_access: SandboxNetworkAccess::Deny,
            },
            idempotency_key: "dishonest-host-profile".to_owned(),
        })
        .unwrap_err();
    assert!(error.to_string().contains("cannot claim unenforced"));
}

#[cfg(not(target_os = "linux"))]
#[tokio::test]
async fn required_sandbox_fails_closed_without_a_backend() {
    let (_area, _workspace, _database, hub, session_id) = setup("sandbox unavailable");
    let (program, args) = success_command();
    let registered = hub
        .register_command_spec(&CommandSpecRequest {
            session_id,
            program,
            args,
            workspace_relative_cwd: ".".to_owned(),
            expected_exit_code: 0,
            timeout_seconds: 5,
            artifact_paths: Vec::new(),
            sandbox_profile: required_sandbox(SandboxWorkspaceAccess::ReadOnly),
            idempotency_key: "required-unavailable".to_owned(),
        })
        .unwrap();

    let outcome = hub.verify(&registered.spec.spec_id).await.unwrap();
    assert_eq!(outcome.run.status, ExecutionStatus::Failed);
    let receipt = outcome.receipt.unwrap();
    assert_eq!(receipt.sandbox_backend, "unavailable");
    assert!(!receipt.sandbox_enforced);
    assert!(receipt.termination.contains("no backend"));
    assert!(outcome.verified_claim_id.is_none());
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn required_linux_sandbox_confines_files_and_attests_enforcement() {
    let (area, workspace, _database, hub, session_id) = setup("linux sandbox confinement");
    let secret = area.path().join("outside-secret.txt");
    std::fs::write(&secret, "secret").unwrap();
    let artifact = "proof.txt";
    let registered = hub
        .register_command_spec(&CommandSpecRequest {
            session_id,
            program: executable(&["/bin/sh", "/usr/bin/sh"]),
            args: vec![
                "-c".to_owned(),
                "test ! -r \"$1\" && printf isolated > proof.txt".to_owned(),
                "aporic-test".to_owned(),
                secret.to_string_lossy().into_owned(),
            ],
            workspace_relative_cwd: ".".to_owned(),
            expected_exit_code: 0,
            timeout_seconds: 5,
            artifact_paths: vec![artifact.to_owned()],
            sandbox_profile: required_sandbox(SandboxWorkspaceAccess::ReadWrite),
            idempotency_key: "linux-confined-write".to_owned(),
        })
        .unwrap();

    let outcome = hub.verify(&registered.spec.spec_id).await.unwrap();
    assert_eq!(
        outcome.run.status,
        ExecutionStatus::Succeeded,
        "filesystem sandbox outcome: {outcome:?}"
    );
    let receipt = outcome.receipt.unwrap();
    assert_eq!(receipt.sandbox_backend, "linux_bubblewrap");
    assert!(receipt.sandbox_enforced);
    assert_eq!(
        std::fs::read_to_string(workspace.join(artifact)).unwrap(),
        "isolated"
    );
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn required_linux_sandbox_denies_workspace_writes_and_network() {
    let (_area, _workspace, _database, hub, session_id) = setup("linux sandbox denials");
    let read_only = hub
        .register_command_spec(&CommandSpecRequest {
            session_id: session_id.clone(),
            program: executable(&["/usr/bin/touch", "/bin/touch"]),
            args: vec!["blocked.txt".to_owned()],
            workspace_relative_cwd: ".".to_owned(),
            expected_exit_code: 1,
            timeout_seconds: 5,
            artifact_paths: Vec::new(),
            sandbox_profile: required_sandbox(SandboxWorkspaceAccess::ReadOnly),
            idempotency_key: "linux-read-only".to_owned(),
        })
        .unwrap();
    let outcome = hub.verify(&read_only.spec.spec_id).await.unwrap();
    assert_eq!(
        outcome.run.status,
        ExecutionStatus::Succeeded,
        "read-only sandbox outcome: {outcome:?}"
    );
    assert!(outcome.receipt.unwrap().sandbox_enforced);

    let network = hub
        .register_command_spec(&CommandSpecRequest {
            session_id,
            program: executable(&["/usr/bin/python3", "/bin/python3"]),
            args: vec![
                "-c".to_owned(),
                "import socket,sys; s=socket.socket(); s.settimeout(.2); sys.exit(0 if s.connect_ex(('1.1.1.1',53)) != 0 else 1)".to_owned(),
            ],
            workspace_relative_cwd: ".".to_owned(),
            expected_exit_code: 0,
            timeout_seconds: 5,
            artifact_paths: Vec::new(),
            sandbox_profile: required_sandbox(SandboxWorkspaceAccess::ReadOnly),
            idempotency_key: "linux-network-deny".to_owned(),
        })
        .unwrap();
    let outcome = hub.verify(&network.spec.spec_id).await.unwrap();
    assert_eq!(
        outcome.run.status,
        ExecutionStatus::Succeeded,
        "network sandbox outcome: {outcome:?}"
    );
    assert!(outcome.receipt.unwrap().sandbox_enforced);
}

#[tokio::test]
async fn execution_rejects_a_legacy_spec_with_excessive_artifact_count() {
    let (_area, workspace, database, hub, session_id) = setup("legacy artifact count limit");
    let (program, args) = success_command();
    let registered = register(
        &hub,
        &session_id,
        program,
        args,
        5,
        Vec::new(),
        "legacy-artifacts",
    );
    let connection = rusqlite::Connection::open(database).unwrap();
    let paths: Vec<String> = (0..65).map(|index| format!("artifact-{index}")).collect();
    connection
        .execute(
            "UPDATE verification_specs SET artifact_paths_json = ?1 WHERE spec_id = ?2",
            rusqlite::params![
                serde_json::to_string(&paths).unwrap(),
                registered.spec.spec_id
            ],
        )
        .unwrap();
    drop(connection);

    let error = hub.verify(&registered.spec.spec_id).await.unwrap_err();
    assert!(error.to_string().contains("artifact count limit"));
    assert!(
        hub.list_executions(&ExecutionListRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            limit: None,
        })
        .unwrap()
        .is_empty()
    );
}

#[tokio::test]
async fn oversized_artifact_cannot_issue_a_verified_receipt() {
    let (_area, workspace, _database, hub, session_id) = setup("artifact byte limit");
    let artifact = "oversized.bin";
    std::fs::File::create(workspace.join(artifact))
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    let (program, args) = success_command();
    let registered = register(
        &hub,
        &session_id,
        program,
        args,
        5,
        vec![artifact.to_owned()],
        "oversized-artifact",
    );
    let outcome = hub.verify(&registered.spec.spec_id).await.unwrap();
    assert_eq!(outcome.run.status, ExecutionStatus::Failed);
    assert_eq!(
        outcome.receipt.unwrap().termination,
        "missing_or_invalid_artifact"
    );
    assert!(outcome.verified_claim_id.is_none());
}

#[tokio::test]
async fn aggregate_artifact_limit_stops_before_hashing_another_file() {
    let (_area, workspace, _database, hub, session_id) = setup("aggregate artifact byte limit");
    let paths = ["first.bin", "second.bin", "third.bin"];
    for path in paths {
        std::fs::File::create(workspace.join(path))
            .unwrap()
            .set_len(64 * 1024 * 1024)
            .unwrap();
    }
    let (program, args) = success_command();
    let registered = register(
        &hub,
        &session_id,
        program,
        args,
        5,
        paths.into_iter().map(str::to_owned).collect(),
        "aggregate-artifacts",
    );

    let outcome = hub.verify(&registered.spec.spec_id).await.unwrap();
    assert_eq!(outcome.run.status, ExecutionStatus::Failed);
    assert_eq!(outcome.artifacts.len(), 2);
    assert_eq!(
        outcome.receipt.unwrap().termination,
        "missing_or_invalid_artifact"
    );
    assert!(outcome.verified_claim_id.is_none());
}

#[cfg(unix)]
#[tokio::test]
async fn runner_git_snapshot_disables_repository_fsmonitor() {
    use std::os::unix::fs::PermissionsExt;

    let (_area, workspace, _database, hub, session_id) = setup("hardened git snapshot");
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&workspace)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init"]);
    let marker = workspace.join("fsmonitor-invoked");
    let helper = workspace.join("fsmonitor-helper");
    std::fs::write(
        &helper,
        format!(
            "#!/bin/sh\nprintf invoked > '{}'\nprintf '2\\n'\n",
            marker.display()
        ),
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&helper).unwrap().permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&helper, permissions).unwrap();
    git(&["config", "core.fsmonitor", helper.to_str().unwrap()]);

    let (program, args) = success_command();
    let registered = register(
        &hub,
        &session_id,
        program,
        args,
        5,
        Vec::new(),
        "fsmonitor-disabled",
    );
    let outcome = hub.verify(&registered.spec.spec_id).await.unwrap();
    assert_eq!(outcome.run.status, ExecutionStatus::Succeeded);
    assert!(!marker.exists());
}

#[test]
fn stale_running_execution_is_reconciled_as_interrupted_with_an_event() {
    let (_area, _workspace, database, hub, session_id) = setup("interrupted execution");
    let (program, args) = success_command();
    let registered = register(
        &hub,
        &session_id,
        program,
        args,
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
    let (program, args) = success_command();
    let registered = register(&hub, &session_id, program, args, 5, Vec::new(), "spec-cli");
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
    let (program, args) = sleep_command(1);
    let registered = register(
        &hub,
        &session_id,
        program,
        args,
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
