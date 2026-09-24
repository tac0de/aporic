use serde_json::{Value, json};
use sha2::{Digest, Sha256};
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
    invoke_with_args(command, connection, &[], input)
}

fn invoke_with_args(
    command: &str,
    connection: &Path,
    arguments: &[&Path],
    input: Option<Value>,
) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aporicctl"));
    child
        .arg(command)
        .arg(connection)
        .args(arguments)
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
            "default_routing": "balanced",
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
                },
                "apply_patch": {
                    "capability": "workspace.write",
                    "delegates": false
                }
            }
        },
        "model_control": {
            "schema_version": 1,
            "enforcement": "required",
            "codex_executable": "/usr/bin/true",
            "tiers": {
                "economy": {"model": "economy-model", "reasoning_effort": "low"},
                "balanced": {"model": "balanced-model", "reasoning_effort": "medium"},
                "deep": {"model": "deep-model", "reasoning_effort": "high"}
            }
        }
    })
}

#[test]
fn plans_and_applies_a_forced_codex_launch_with_an_append_only_audit() {
    let area = TestArea::new();
    setup(&area);
    let connection = area.connection();
    success(invoke("bind", &connection, Some(bind_input(&area))));

    let signals = area.state().join("routing-signals.json");
    std::fs::write(
        &signals,
        b"{\"requested\":null,\"multi_step\":false,\"uncertainty\":false,\"verification_failed\":true,\"retry_count\":0,\"burn_budget\":false}\n",
    )
    .unwrap();
    let planned = success(invoke(
        "model-plan",
        &connection,
        Some(json!({
            "requested": null,
            "multi_step": false,
            "uncertainty": false,
            "verification_failed": true,
            "retry_count": 0,
            "burn_budget": false
        })),
    ));
    assert_eq!(planned["result"]["model"], "deep-model");
    assert_eq!(planned["result"]["reasoning_effort"], "high");

    let launched = success(invoke_with_args(
        "codex-launch",
        &connection,
        &[signals.as_path()],
        None,
    ));
    assert_eq!(launched["result"]["outcome"]["exited"]["code"], 0);
    assert_eq!(launched["result"]["plan"]["tier"], "deep");

    let override_attempt = invoke_with_args(
        "codex-launch",
        &connection,
        &[signals.as_path(), Path::new("--model=caller-choice")],
        None,
    );
    assert_eq!(override_attempt.status.code(), Some(1));

    let status = success(invoke("status", &connection, None));
    assert_eq!(status["result"]["model_control_records"], 2);
}

fn action() -> Value {
    json!({
        "principal": "agent-1",
        "task_ref": "task-1",
        "session_ref": "session-1",
        "action": "write_file",
        "input": {"path": "README.md", "content": "hello"}
    })
}

#[test]
fn drives_a_project_from_bind_through_handoff_and_verified_effect() {
    let area = TestArea::new();
    setup(&area);
    let connection = area.connection();

    let bound = success(invoke("bind", &connection, Some(bind_input(&area))));
    assert_eq!(bound["ok"], true);
    assert!(connection.is_file());

    let initial = success(invoke("status", &connection, None));
    assert_eq!(initial["result"]["kernel_revision"], 0);
    assert_eq!(initial["result"]["handoff_revision"], 0);

    success(invoke(
        "day-open",
        &connection,
        Some(json!({
            "event_id": "day-open-event",
            "idempotency_key": "day-open-key",
            "expected_revision": 0,
            "day_id": "day-1",
            "lineage_ref": "demo-work",
            "task_ref": "task-1",
            "session_ref": "session-1",
            "predecessor": null
        })),
    ));
    let projected = success(invoke(
        "handoff-project",
        &connection,
        Some(json!({
            "event_id": "handoff-event",
            "idempotency_key": "handoff-key",
            "expected_revision": 1,
            "day_id": "day-1",
            "capsule": {
                "objective": "Exercise A0",
                "constraints": ["Keep state external"],
                "accepted_decisions": ["Use JSON commands"],
                "completed_checks": ["Project bound"],
                "open_questions": ["Which real project is first?"],
                "next_action": "Close the first day"
            }
        })),
    ));
    let digest = projected["result"]["handoff_sha256"].as_str().unwrap();
    success(invoke(
        "day-close",
        &connection,
        Some(json!({
            "event_id": "day-close-event",
            "idempotency_key": "day-close-key",
            "expected_revision": 2,
            "day_id": "day-1",
            "handoff_sha256": digest
        })),
    ));

    success(invoke(
        "grant",
        &connection,
        Some(json!({
            "event_id": "grant-event",
            "idempotency_key": "grant-key",
            "expected_revision": 0,
            "grant_id": "grant-1",
            "action": action(),
            "authority_ref": "human:approval-1"
        })),
    ));
    success(invoke(
        "reserve",
        &connection,
        Some(json!({
            "event_id": "reserve-event",
            "idempotency_key": "reserve-key",
            "expected_revision": 1,
            "reservation_id": "reservation-1",
            "grant_id": "grant-1",
            "action": action(),
            "routing_signals": {
                "requested": "balanced",
                "multi_step": false,
                "uncertainty": false,
                "verification_failed": false,
                "retry_count": 0,
                "burn_budget": false
            }
        })),
    ));
    success(invoke(
        "effect",
        &connection,
        Some(json!({
            "event_id": "effect-event",
            "idempotency_key": "effect-key",
            "expected_revision": 2,
            "reservation_id": "reservation-1",
            "outcome": "succeeded",
            "observation_ref": "host:receipt-1"
        })),
    ));
    success(invoke(
        "verify",
        &connection,
        Some(json!({
            "event_id": "verify-event",
            "idempotency_key": "verify-key",
            "expected_revision": 3,
            "reservation_id": "reservation-1",
            "verifier": "verifier-1",
            "result": "passed",
            "evidence_ref": "test:cli-e2e"
        })),
    ));
    success(invoke(
        "grant",
        &connection,
        Some(json!({
            "event_id": "grant-2-event",
            "idempotency_key": "grant-2-key",
            "expected_revision": 4,
            "grant_id": "grant-2",
            "action": action(),
            "authority_ref": "human:approval-2"
        })),
    ));
    success(invoke(
        "reserve",
        &connection,
        Some(json!({
            "event_id": "reserve-2-event",
            "idempotency_key": "reserve-2-key",
            "expected_revision": 5,
            "reservation_id": "reservation-2",
            "grant_id": "grant-2",
            "action": action(),
            "routing_signals": {
                "requested": null,
                "multi_step": false,
                "uncertainty": false,
                "verification_failed": false,
                "retry_count": 0,
                "burn_budget": false
            }
        })),
    ));
    success(invoke(
        "abandon",
        &connection,
        Some(json!({
            "event_id": "abandon-event",
            "idempotency_key": "abandon-key",
            "expected_revision": 6,
            "reservation_id": "reservation-2",
            "reason": "host cancelled before execution"
        })),
    ));

    let final_status = success(invoke("status", &connection, None));
    assert_eq!(final_status["result"]["kernel_revision"], 7);
    assert_eq!(final_status["result"]["handoff_revision"], 3);
    assert_eq!(git(&area.repo(), &["status", "--porcelain"]), "");
    assert!(!area.repo().join(".aporic").exists());
}

#[test]
fn semantic_rejection_is_json_with_exit_code_two() {
    let area = TestArea::new();
    setup(&area);
    let connection = area.connection();
    success(invoke("bind", &connection, Some(bind_input(&area))));

    let mut denied_action = action();
    denied_action["action"] = json!("unmapped_action");
    let output = invoke(
        "grant",
        &connection,
        Some(json!({
            "event_id": "grant-event",
            "idempotency_key": "grant-key",
            "expected_revision": 0,
            "grant_id": "grant-1",
            "action": denied_action,
            "authority_ref": "human:approval-1"
        })),
    );
    assert_eq!(output.status.code(), Some(2));
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["ok"], false);
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

#[test]
fn codex_hooks_open_gate_record_compact_and_close_automatically() {
    let area = TestArea::new();
    setup(&area);
    let connection = area.connection();
    success(invoke("bind", &connection, Some(bind_input(&area))));
    let catalog = area.state().join("catalog");
    success(invoke(
        "catalog-register",
        &catalog,
        Some(json!({"connection_file": connection})),
    ));

    let session_id = "thread-123";
    let start = success(invoke(
        "codex-hook",
        &catalog,
        Some(json!({
            "session_id": session_id,
            "cwd": area.repo(),
            "hook_event_name": "SessionStart",
            "model": "codex-test-model",
            "source": "startup",
            "permission_mode": "default"
        })),
    ));
    assert!(
        start["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap()
            .contains("routing_recommendation=Economy")
    );
    let start_context = start["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(start_context.contains("The user does not need to phrase a request precisely"));
    assert!(start_context.contains("Do not ask the human to collect or provide research material"));
    assert!(start_context.contains("behavioral defaults, not authority"));

    let prompt = success(invoke(
        "codex-hook",
        &catalog,
        Some(json!({
            "session_id": session_id,
            "cwd": area.repo(),
            "hook_event_name": "UserPromptSubmit"
        })),
    ));
    let prompt_context = prompt["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(prompt_context.contains("Resolve ordinary ambiguity autonomously"));
    assert!(prompt_context.contains("Do not ask the human to gather information"));

    let tool_input = json!({"command": "*** Begin Patch\n*** End Patch"});
    let denied = success(invoke(
        "codex-hook",
        &catalog,
        Some(json!({
            "session_id": session_id,
            "cwd": area.repo(),
            "hook_event_name": "PreToolUse",
            "model": "codex-test-model",
            "permission_mode": "default",
            "turn_id": "turn-1",
            "tool_name": "apply_patch",
            "tool_use_id": "tool-1",
            "tool_input": tool_input
        })),
    ));
    assert_eq!(denied["hookSpecificOutput"]["permissionDecision"], "deny");

    let session_ref = format!("codex:{:x}", Sha256::digest(session_id.as_bytes()));
    success(invoke(
        "grant",
        &connection,
        Some(json!({
            "event_id": "grant-hook-event",
            "idempotency_key": "grant-hook-key",
            "expected_revision": 0,
            "grant_id": "grant-hook-1",
            "action": {
                "principal": "codex",
                "task_ref": session_ref,
                "session_ref": session_ref,
                "action": "apply_patch",
                "input": tool_input
            },
            "authority_ref": "human:hook-test"
        })),
    ));
    let allowed = success(invoke(
        "codex-hook",
        &catalog,
        Some(json!({
            "session_id": session_id,
            "cwd": area.repo(),
            "hook_event_name": "PreToolUse",
            "model": "codex-test-model",
            "permission_mode": "default",
            "turn_id": "turn-1",
            "tool_name": "apply_patch",
            "tool_use_id": "tool-1",
            "tool_input": tool_input
        })),
    ));
    assert_eq!(allowed, json!({}));
    assert_eq!(
        success(invoke(
            "codex-hook",
            &catalog,
            Some(json!({
                "session_id": session_id,
                "cwd": area.repo(),
                "hook_event_name": "PostToolUse",
                "model": "codex-test-model",
                "permission_mode": "default",
                "turn_id": "turn-1",
                "tool_name": "apply_patch",
                "tool_use_id": "tool-1",
                "tool_input": tool_input,
                "tool_response": {"output": "done"}
            })),
        )),
        json!({})
    );
    success(invoke(
        "codex-hook",
        &catalog,
        Some(json!({
            "session_id": session_id,
            "cwd": area.repo(),
            "hook_event_name": "PreCompact",
            "model": "codex-test-model",
            "turn_id": "turn-1",
            "trigger": "auto"
        })),
    ));
    success(invoke(
        "codex-hook",
        &catalog,
        Some(json!({
            "session_id": session_id,
            "cwd": area.repo(),
            "hook_event_name": "SessionEnd",
            "model": "codex-test-model",
            "reason": "other"
        })),
    ));

    let config: Value = serde_json::from_slice(&std::fs::read(&connection).unwrap()).unwrap();
    let kernel = aporic_kernel::load(config["kernel_store"].as_str().unwrap()).unwrap();
    let reservation = kernel.state().reservations.values().next().unwrap();
    assert_eq!(reservation.reservation.routing_tier, "economy");
    assert_eq!(
        reservation.effect.as_ref().unwrap().outcome,
        aporic_kernel::EffectOutcome::Unknown
    );
    let handoff = aporic_handoff::load(config["handoff_store"].as_str().unwrap()).unwrap();
    assert!(handoff.state().days.values().next().unwrap().closed);

    let routed = success(invoke(
        "route",
        &connection,
        Some(json!({
            "requested": null,
            "multi_step": false,
            "uncertainty": false,
            "verification_failed": false,
            "retry_count": 0,
            "burn_budget": true
        })),
    ));
    assert_eq!(routed["result"]["tier"], "deep");
}
