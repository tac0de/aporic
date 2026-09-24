use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_PATH: AtomicU64 = AtomicU64::new(1);

struct TestArea(PathBuf);

impl TestArea {
    fn new() -> Self {
        let sequence = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("aporic-cli-{}-{sequence}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn repo(&self) -> PathBuf {
        self.0.join("repo")
    }

    fn state(&self) -> PathBuf {
        self.0.join("state")
    }

    fn connection(&self) -> PathBuf {
        self.state().join("connection.json")
    }
}

impl Drop for TestArea {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn git(directory: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().into()
}

fn setup(area: &TestArea) {
    std::fs::create_dir(area.repo()).unwrap();
    std::fs::create_dir(area.state()).unwrap();
    git(&area.repo(), &["init", "--quiet"]);
    git(
        &area.repo(),
        &[
            "-c",
            "user.name=Aporic Test",
            "-c",
            "user.email=aporic@example.invalid",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "initial",
        ],
    );
    git(
        &area.repo(),
        &[
            "remote",
            "add",
            "origin",
            "https://example.invalid/acme/cli-demo.git",
        ],
    );
}

fn role_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../roles/generalist")
}

fn invoke(command: &str, connection: &Path, input: Option<Value>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aporicctl"));
    child
        .arg(command)
        .arg(connection)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = child.spawn().unwrap();
    if let Some(value) = input {
        let stdin = child.stdin.as_mut().unwrap();
        serde_json::to_writer(&mut *stdin, &value).unwrap();
        stdin.write_all(b"\n").unwrap();
    }
    child.wait_with_output().unwrap()
}

fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn bind_input(area: &TestArea) -> Value {
    json!({
        "schema_version": 1,
        "project_id": "demo",
        "workspace": area.repo(),
        "remote_name": "origin",
        "role_directory": role_path(),
        "host_policy": {
            "capabilities": ["workspace.read", "workspace.write"],
            "default_routing": "economy",
            "routing_ceiling": "deep",
            "max_tool_calls": 8,
            "max_parallel_tasks": 1,
            "may_delegate": false
        },
        "boundary_policy": {
            "actions": {
                "write_file": {
                    "capability": "workspace.write",
                    "delegates": false
                }
            }
        }
    })
}

#[test]
fn binds_role_package_and_reports_external_state() {
    let area = TestArea::new();
    setup(&area);
    let connection = area.connection();

    let bound = success(invoke("bind", &connection, Some(bind_input(&area))));
    assert_eq!(bound["result"]["project_id"], "demo");

    let status = success(invoke("status", &connection, None));
    assert_eq!(status["result"]["role_id"], "generalist");
    assert_eq!(status["result"]["kernel_revision"], 0);
    assert_eq!(status["result"]["handoff_revision"], 0);
    assert_eq!(git(&area.repo(), &["status", "--porcelain"]), "");
    assert!(!area.repo().join(".aporic").exists());
    assert!(area.state().join("kernel.jsonl").is_file());
    assert!(area.state().join("handoff.jsonl").is_file());
}

#[test]
fn routes_without_host_specific_model_or_plugin_control() {
    let area = TestArea::new();
    setup(&area);
    let connection = area.connection();
    success(invoke("bind", &connection, Some(bind_input(&area))));

    let routed = success(invoke(
        "route",
        &connection,
        Some(json!({
            "requested": null,
            "multi_step": true,
            "uncertainty": false,
            "verification_failed": false,
            "retry_count": 0,
            "burn_budget": false
        })),
    ));
    assert_eq!(routed["result"]["tier"], "balanced");
}

#[test]
fn semantic_rejection_remains_structured_for_explicit_kernel_experiments() {
    let area = TestArea::new();
    setup(&area);
    let connection = area.connection();
    success(invoke("bind", &connection, Some(bind_input(&area))));

    let output = invoke(
        "grant",
        &connection,
        Some(json!({
            "event_id": "grant-event",
            "idempotency_key": "grant-key",
            "expected_revision": 0,
            "grant_id": "grant-1",
            "action": {
                "principal": "operator",
                "task_ref": "task-1",
                "session_ref": "session-1",
                "action": "unmapped_action",
                "input": {}
            },
            "authority_ref": "explicit-test"
        })),
    );
    assert_eq!(output.status.code(), Some(2));
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["result"]["reason_code"], "ACTION_NOT_MAPPED");
}

#[test]
fn refuses_to_place_connection_state_inside_the_project() {
    let area = TestArea::new();
    setup(&area);
    let state = area.repo().join("state");
    std::fs::create_dir(&state).unwrap();
    let connection = state.join("connection.json");

    let output = invoke("bind", &connection, Some(bind_input(&area)));
    assert_eq!(output.status.code(), Some(1));
    assert!(!connection.exists());
    assert!(!state.join("kernel.jsonl").exists());
    assert!(!state.join("handoff.jsonl").exists());
}
