use serde_json::Value;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

fn temp_path(name: &str) -> PathBuf {
    let unique = format!(
        "aporic-cli-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    std::env::temp_dir().join(unique)
}

fn run(args: &[&str], input: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_aporic"));
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if input.is_some() {
        command.stdin(Stdio::piped());
    }
    let mut child = command.spawn().unwrap();
    if let Some(input) = input {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
    }
    child.wait_with_output().unwrap()
}

#[test]
fn init_and_status_are_wired_through_the_binary() {
    let store = temp_path("init-status").join("events.jsonl");
    let store_arg = store.to_str().unwrap();

    let initialized = run(&["init", "--store", store_arg], None);
    assert!(initialized.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&initialized.stdout).unwrap()["status"],
        "initialized"
    );

    let status = run(&["status", "--store", store_arg], None);
    assert!(status.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&status.stdout).unwrap()["revision"],
        0
    );

    let duplicate = run(&["init", "--store", store_arg], None);
    assert_eq!(duplicate.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&duplicate.stderr).starts_with("aporic:"));
}

#[test]
fn policy_rejection_uses_exit_two_and_structured_json() {
    let store = temp_path("policy-rejection").join("events.jsonl");
    let store_arg = store.to_str().unwrap();
    assert!(run(&["init", "--store", store_arg], None).status.success());

    let request = serde_json::json!({
        "schema_version": 2,
        "event_id": "decision-event",
        "idempotency_key": "decision-key",
        "expected_revision": 0,
        "actor": { "kind": "agent", "id": "agent", "provenance": "cli-test" },
        "scope": "repo",
        "event": {
            "type": "decision_committed",
            "decision_id": "decision-1",
            "question": "Ship?",
            "value": "yes",
            "authority_ref": null
        }
    });
    let output = run(
        &["commit", "--store", store_arg],
        Some(&serde_json::to_string(&request).unwrap()),
    );
    assert_eq!(output.status.code(), Some(2));
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["status"], "rejected");
    assert_eq!(response["evaluation"]["reason_code"], "DELEGATION_REQUIRED");
}

#[test]
fn protected_tool_fails_closed_when_store_is_missing() {
    let workspace = temp_path("missing-store-workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let missing_store = workspace.join("missing").join("events.jsonl");
    let input = serde_json::json!({
        "session_id": "session-1",
        "hook_event_name": "PreToolUse",
        "cwd": workspace,
        "turn_id": "turn-1",
        "tool_name": "apply_patch",
        "tool_use_id": "tool-use-1",
        "tool_input": { "patch": "ignored" }
    });
    let output = run(
        &[
            "codex-pre-tool-use",
            "--store",
            missing_store.to_str().unwrap(),
            "--scope",
            "repo",
            "--workspace",
            workspace.to_str().unwrap(),
            "--protected-tool",
            "apply_patch",
        ],
        Some(&serde_json::to_string(&input).unwrap()),
    );
    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["hookSpecificOutput"]["permissionDecision"], "deny");
    assert!(
        response["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .unwrap()
            .starts_with("APORIC_STORE_NOT_FOUND")
    );
}

#[test]
fn protected_tool_fails_closed_for_an_unsupported_stored_schema() {
    let workspace = temp_path("unsupported-schema-workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let store = workspace.join("events.jsonl");
    let stored = serde_json::json!({
        "sequence": 1,
        "schema_version": 99,
        "event_id": "future-event",
        "idempotency_key": "future-key",
        "expected_revision": 0,
        "actor": { "kind": "human", "id": "user", "provenance": "future-host" },
        "scope": "repo",
        "event": {
            "type": "plan_registered",
            "plan_id": "future-plan",
            "objective": "Future semantics",
            "acceptance_checks": ["future check"],
            "unresolved_questions": []
        }
    });
    std::fs::write(&store, format!("{stored}\n")).unwrap();
    let input = serde_json::json!({
        "session_id": "session-1",
        "hook_event_name": "PreToolUse",
        "cwd": workspace,
        "turn_id": "turn-1",
        "tool_name": "apply_patch",
        "tool_use_id": "tool-use-1",
        "tool_input": { "patch": "ignored" }
    });
    let output = run(
        &[
            "codex-pre-tool-use",
            "--store",
            store.to_str().unwrap(),
            "--scope",
            "repo",
            "--workspace",
            workspace.to_str().unwrap(),
            "--protected-tool",
            "apply_patch",
        ],
        Some(&serde_json::to_string(&input).unwrap()),
    );

    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["hookSpecificOutput"]["permissionDecision"], "deny");
    assert!(
        response["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .unwrap()
            .starts_with("APORIC_STORE_INVALID")
    );
}

#[test]
fn session_start_reports_the_same_plan_gate_policy() {
    let workspace = temp_path("session-start-workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let store = workspace.join("events.jsonl");
    assert!(
        run(&["init", "--store", store.to_str().unwrap()], None)
            .status
            .success()
    );
    let input = serde_json::json!({
        "session_id": "session-1",
        "hook_event_name": "SessionStart",
        "cwd": workspace,
        "source": "startup"
    });
    let output = run(
        &[
            "codex-session-start",
            "--store",
            store.to_str().unwrap(),
            "--scope",
            "repo",
            "--workspace",
            workspace.to_str().unwrap(),
            "--protected-tool",
            "apply_patch",
            "--require-plan",
        ],
        Some(&serde_json::to_string(&input).unwrap()),
    );

    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    let context = response["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    let start = context.find("<aporic-recorded-data>").unwrap() + "<aporic-recorded-data>".len();
    let end = context.find("</aporic-recorded-data>").unwrap();
    let projection: Value = serde_json::from_str(&context[start..end]).unwrap();
    assert_eq!(projection["schema"], 3);
    assert_eq!(projection["budget"]["limit"], 6_000);
    assert_eq!(
        projection["executions"][0]["status"],
        "plan_authorization_required"
    );
}

#[test]
fn duplicate_protected_tool_option_fails_closed_instead_of_skipping_the_gate() {
    let workspace = temp_path("duplicate-option-workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let store = workspace.join("events.jsonl");
    assert!(
        run(&["init", "--store", store.to_str().unwrap()], None)
            .status
            .success()
    );
    let input = serde_json::json!({
        "session_id": "session-1",
        "hook_event_name": "PreToolUse",
        "cwd": workspace,
        "turn_id": "turn-1",
        "tool_name": "apply_patch",
        "tool_use_id": "tool-use-1",
        "tool_input": { "patch": "ignored" }
    });
    let output = run(
        &[
            "codex-pre-tool-use",
            "--store",
            store.to_str().unwrap(),
            "--scope",
            "repo",
            "--workspace",
            workspace.to_str().unwrap(),
            "--protected-tool",
            "apply_patch",
            "--protected-tool",
            "apply_patch",
        ],
        Some(&serde_json::to_string(&input).unwrap()),
    );

    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        response["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .unwrap()
            .starts_with("APORIC_POLICY_INVALID")
    );
}

#[test]
fn missing_protected_tool_value_fails_closed_instead_of_skipping_the_gate() {
    let workspace = temp_path("missing-option-value-workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let store = workspace.join("events.jsonl");
    assert!(
        run(&["init", "--store", store.to_str().unwrap()], None)
            .status
            .success()
    );
    let input = serde_json::json!({
        "session_id": "session-1",
        "hook_event_name": "PreToolUse",
        "cwd": workspace,
        "turn_id": "turn-1",
        "tool_name": "apply_patch",
        "tool_use_id": "tool-use-1",
        "tool_input": { "patch": "ignored" }
    });
    let output = run(
        &[
            "codex-pre-tool-use",
            "--store",
            store.to_str().unwrap(),
            "--scope",
            "repo",
            "--workspace",
            workspace.to_str().unwrap(),
            "--protected-tool",
            "--require-plan",
        ],
        Some(&serde_json::to_string(&input).unwrap()),
    );

    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        response["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .unwrap()
            .starts_with("APORIC_POLICY_INVALID")
    );
}

#[test]
fn json_policy_drives_explain_and_doctor_without_mutating_state() {
    let workspace = temp_path("policy-explain-workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let store = workspace.join("events.jsonl");
    let policy = workspace.join("policy.json");
    std::fs::write(
        &policy,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 1,
            "tools": {
                "apply_patch": {"require_plan": true, "require_grant": false},
                "exec_command": {"require_plan": false, "require_grant": false}
            }
        }))
        .unwrap(),
    )
    .unwrap();
    assert!(
        run(&["init", "--store", store.to_str().unwrap()], None)
            .status
            .success()
    );

    let doctor = run(
        &[
            "doctor",
            "--store",
            store.to_str().unwrap(),
            "--policy",
            policy.to_str().unwrap(),
        ],
        None,
    );
    assert!(doctor.status.success());
    let diagnosis: Value = serde_json::from_slice(&doctor.stdout).unwrap();
    assert_eq!(diagnosis["healthy"], true);
    assert_eq!(diagnosis["protected_tools"].as_array().unwrap().len(), 2);

    let input = serde_json::json!({
        "session_id": "session-1",
        "hook_event_name": "PreToolUse",
        "cwd": workspace,
        "turn_id": "turn-1",
        "tool_name": "apply_patch",
        "tool_use_id": "tool-use-1",
        "tool_input": {"patch": "exact"}
    });
    let explained = run(
        &[
            "explain",
            "--store",
            store.to_str().unwrap(),
            "--scope",
            "repo",
            "--policy",
            policy.to_str().unwrap(),
        ],
        Some(&serde_json::to_string(&input).unwrap()),
    );
    assert!(explained.status.success());
    let explanation: Value = serde_json::from_slice(&explained.stdout).unwrap();
    assert_eq!(
        explanation["evaluation"]["reason_code"],
        "PLAN_AUTHORIZATION_REQUIRED"
    );
    let status = run(&["status", "--store", store.to_str().unwrap()], None);
    assert_eq!(
        serde_json::from_slice::<Value>(&status.stdout).unwrap()["revision"],
        0
    );
}

#[test]
fn invalid_policy_fails_closed_for_pre_tool_use_and_is_structured_in_doctor() {
    let workspace = temp_path("invalid-policy-workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let store = workspace.join("events.jsonl");
    let policy = workspace.join("policy.json");
    std::fs::write(&policy, b"{not-json").unwrap();
    assert!(
        run(&["init", "--store", store.to_str().unwrap()], None)
            .status
            .success()
    );
    let input = serde_json::json!({
        "session_id": "session-1",
        "hook_event_name": "PreToolUse",
        "cwd": workspace,
        "turn_id": "turn-1",
        "tool_name": "apply_patch",
        "tool_use_id": "tool-use-1",
        "tool_input": {"patch": "exact"}
    });
    let gated = run(
        &[
            "codex-pre-tool-use",
            "--store",
            store.to_str().unwrap(),
            "--scope",
            "repo",
            "--workspace",
            workspace.to_str().unwrap(),
            "--policy",
            policy.to_str().unwrap(),
        ],
        Some(&serde_json::to_string(&input).unwrap()),
    );
    assert!(gated.status.success());
    let gate: Value = serde_json::from_slice(&gated.stdout).unwrap();
    assert!(
        gate["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .unwrap()
            .starts_with("APORIC_POLICY_INVALID")
    );

    let doctor = run(
        &[
            "doctor",
            "--store",
            store.to_str().unwrap(),
            "--policy",
            policy.to_str().unwrap(),
        ],
        None,
    );
    assert_eq!(doctor.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<Value>(&doctor.stdout).unwrap()["healthy"],
        false
    );
}

#[test]
fn unrelated_session_start_skips_before_loading_policy() {
    let configured_workspace = temp_path("configured-workspace");
    let other_workspace = temp_path("other-workspace");
    std::fs::create_dir_all(&configured_workspace).unwrap();
    std::fs::create_dir_all(&other_workspace).unwrap();
    let store = configured_workspace.join("events.jsonl");
    assert!(
        run(&["init", "--store", store.to_str().unwrap()], None)
            .status
            .success()
    );
    let input = serde_json::json!({
        "session_id": "session-1",
        "hook_event_name": "SessionStart",
        "cwd": other_workspace,
        "source": "startup"
    });
    let output = run(
        &[
            "codex-session-start",
            "--store",
            store.to_str().unwrap(),
            "--scope",
            "repo",
            "--workspace",
            configured_workspace.to_str().unwrap(),
            "--policy",
            configured_workspace
                .join("missing-policy.json")
                .to_str()
                .unwrap(),
        ],
        Some(&serde_json::to_string(&input).unwrap()),
    );
    assert!(output.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({"continue": true})
    );
}
