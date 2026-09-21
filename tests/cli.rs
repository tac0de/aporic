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
fn one_global_adapter_discovers_only_explicitly_bound_projects() {
    let workspace = temp_path("global-workspace");
    let nested = workspace.join("src");
    let data_root = temp_path("global-data");
    std::fs::create_dir_all(&nested).unwrap();

    let setup = run(
        &[
            "project-init",
            "--workspace",
            workspace.to_str().unwrap(),
            "--scope",
            "global-test",
            "--data-root",
            data_root.to_str().unwrap(),
        ],
        None,
    );
    assert!(
        setup.status.success(),
        "{}",
        String::from_utf8_lossy(&setup.stderr)
    );

    let session_input = serde_json::json!({
        "session_id": "session-global",
        "hook_event_name": "SessionStart",
        "cwd": nested,
        "source": "startup"
    });
    let session = run(
        &[
            "codex-global-session-start",
            "--data-root",
            data_root.to_str().unwrap(),
        ],
        Some(&serde_json::to_string(&session_input).unwrap()),
    );
    assert!(session.status.success());
    let response: Value = serde_json::from_slice(&session.stdout).unwrap();
    let context = response["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(context.contains("\"scope\":\"global-test\""));
    assert!(data_root.join("workspaces").exists());

    let unrelated = temp_path("global-unrelated");
    std::fs::create_dir_all(&unrelated).unwrap();
    let unrelated_input = serde_json::json!({
        "session_id": "session-unrelated",
        "hook_event_name": "SessionStart",
        "cwd": unrelated,
        "source": "startup"
    });
    let skipped = run(
        &[
            "codex-global-session-start",
            "--data-root",
            data_root.to_str().unwrap(),
        ],
        Some(&serde_json::to_string(&unrelated_input).unwrap()),
    );
    assert!(skipped.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&skipped.stdout).unwrap(),
        serde_json::json!({"continue": true})
    );
}

#[test]
fn global_user_prompt_submit_is_bounded_advisory_and_opt_in() {
    let workspace = temp_path("global-intent-workspace");
    let nested = workspace.join("src");
    let data_root = temp_path("global-intent-data");
    std::fs::create_dir_all(&nested).unwrap();

    let setup = run(
        &[
            "project-init",
            "--workspace",
            workspace.to_str().unwrap(),
            "--scope",
            "intent-test",
            "--data-root",
            data_root.to_str().unwrap(),
        ],
        None,
    );
    assert!(setup.status.success());

    let input = serde_json::json!({
        "session_id": "session-intent",
        "hook_event_name": "UserPromptSubmit",
        "cwd": nested,
        "turn_id": "turn-intent",
        "prompt": "DO_NOT_ECHO_RAW_PROMPT"
    });
    let output = run(
        &[
            "codex-global-user-prompt-submit",
            "--data-root",
            data_root.to_str().unwrap(),
        ],
        Some(&serde_json::to_string(&input).unwrap()),
    );
    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["continue"], true);
    let context = response["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(context.contains("Intent Fidelity contract v1"));
    assert!(!context.contains("DO_NOT_ECHO_RAW_PROMPT"));

    let unrelated = temp_path("global-intent-unrelated");
    std::fs::create_dir_all(&unrelated).unwrap();
    let unrelated_input = serde_json::json!({
        "session_id": "session-unrelated",
        "hook_event_name": "UserPromptSubmit",
        "cwd": unrelated,
        "turn_id": "turn-unrelated",
        "prompt": "unbound"
    });
    let skipped = run(
        &[
            "codex-global-user-prompt-submit",
            "--data-root",
            data_root.to_str().unwrap(),
        ],
        Some(&serde_json::to_string(&unrelated_input).unwrap()),
    );
    assert!(skipped.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&skipped.stdout).unwrap(),
        serde_json::json!({"continue": true})
    );
}

#[test]
fn invalid_global_project_warns_but_does_not_block_user_prompt() {
    let workspace = temp_path("global-invalid-intent");
    let data_root = temp_path("global-invalid-intent-data");
    std::fs::create_dir_all(workspace.join(".aporic")).unwrap();
    std::fs::write(
        workspace.join(".aporic/config.json"),
        r#"{"schema_version":1,"scope":"invalid scope"}"#,
    )
    .unwrap();
    let input = serde_json::json!({
        "session_id": "session-global",
        "hook_event_name": "UserPromptSubmit",
        "cwd": workspace,
        "turn_id": "turn-global",
        "prompt": "DO_NOT_ECHO_RAW_PROMPT"
    });
    let output = run(
        &[
            "codex-global-user-prompt-submit",
            "--data-root",
            data_root.to_str().unwrap(),
        ],
        Some(&serde_json::to_string(&input).unwrap()),
    );
    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["continue"], true);
    assert!(
        response["systemMessage"]
            .as_str()
            .unwrap()
            .contains("PROJECT_INVALID")
    );
    assert!(
        !String::from_utf8(output.stdout)
            .unwrap()
            .contains("DO_NOT_ECHO_RAW_PROMPT")
    );
}

#[test]
fn oversized_global_user_prompt_input_skips_without_blocking_or_echoing() {
    let oversized = "DO_NOT_ECHO_OVERSIZED".repeat(55_189);
    let input = serde_json::json!({
        "session_id": "session-global",
        "hook_event_name": "UserPromptSubmit",
        "cwd": "/tmp",
        "turn_id": "turn-global",
        "prompt": oversized
    });
    let output = run(
        &["codex-global-user-prompt-submit"],
        Some(&serde_json::to_string(&input).unwrap()),
    );
    assert!(output.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({"continue": true})
    );
    assert!(
        !String::from_utf8(output.stdout)
            .unwrap()
            .contains("DO_NOT_ECHO_OVERSIZED")
    );
}

#[test]
fn oversized_global_post_tool_input_fails_with_a_bounded_diagnostic() {
    let oversized = "x".repeat(1_048_577);
    let output = run(&["codex-global-post-tool-use"], Some(&oversized));
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("hook input exceeds 1048576 UTF-8 bytes")
    );
}

#[test]
fn global_post_tool_use_reports_a_missing_bound_store() {
    let workspace = temp_path("global-post-missing-store");
    let data_root = temp_path("global-post-missing-store-data");
    std::fs::create_dir_all(&workspace).unwrap();
    let setup = run(
        &[
            "project-init",
            "--workspace",
            workspace.to_str().unwrap(),
            "--scope",
            "repo",
            "--data-root",
            data_root.to_str().unwrap(),
        ],
        None,
    );
    assert!(setup.status.success());
    let setup: Value = serde_json::from_slice(&setup.stdout).unwrap();
    std::fs::remove_file(setup["store"].as_str().unwrap()).unwrap();
    let input = serde_json::json!({
        "session_id": "session-global",
        "hook_event_name": "PostToolUse",
        "cwd": workspace,
        "turn_id": "turn-global",
        "tool_name": "apply_patch",
        "tool_use_id": "use-global",
        "tool_input": {},
        "tool_response": {}
    });
    let output = run(
        &[
            "codex-global-post-tool-use",
            "--data-root",
            data_root.to_str().unwrap(),
        ],
        Some(&serde_json::to_string(&input).unwrap()),
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("aporic:"));
}

#[test]
fn global_adapter_fails_closed_when_a_bound_project_is_invalid() {
    let workspace = temp_path("global-invalid");
    let data_root = temp_path("global-invalid-data");
    std::fs::create_dir_all(workspace.join(".aporic")).unwrap();
    std::fs::write(
        workspace.join(".aporic/config.json"),
        r#"{"schema_version":1,"scope":"repo"}"#,
    )
    .unwrap();
    let input = serde_json::json!({
        "session_id": "session-global",
        "hook_event_name": "PreToolUse",
        "cwd": workspace,
        "turn_id": "turn-global",
        "tool_name": "apply_patch",
        "tool_use_id": "tool-global",
        "tool_input": {"patch": "ignored"}
    });
    let output = run(
        &[
            "codex-global-pre-tool-use",
            "--data-root",
            data_root.to_str().unwrap(),
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
            .starts_with("APORIC_PROJECT_INVALID")
    );
}

#[test]
fn global_session_start_never_recreates_a_missing_bound_store() {
    let workspace = temp_path("global-missing-store");
    let data_root = temp_path("global-missing-store-data");
    std::fs::create_dir_all(&workspace).unwrap();
    let setup = run(
        &[
            "project-init",
            "--workspace",
            workspace.to_str().unwrap(),
            "--scope",
            "repo",
            "--data-root",
            data_root.to_str().unwrap(),
        ],
        None,
    );
    assert!(setup.status.success());
    let setup: Value = serde_json::from_slice(&setup.stdout).unwrap();
    let store = std::path::PathBuf::from(setup["store"].as_str().unwrap());
    std::fs::remove_file(&store).unwrap();

    let input = serde_json::json!({
        "session_id": "session-global",
        "hook_event_name": "SessionStart",
        "cwd": workspace,
        "source": "resume"
    });
    let output = run(
        &[
            "codex-global-session-start",
            "--data-root",
            data_root.to_str().unwrap(),
        ],
        Some(&serde_json::to_string(&input).unwrap()),
    );
    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    let context = response["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(context.contains("\"coverage\":\"unavailable\""));
    assert!(context.contains("\"reason_code\":\"STORE_NOT_FOUND\""));
    assert!(!store.exists());
}

#[test]
fn invalid_global_data_root_fails_closed_for_pre_tool_use() {
    let workspace = temp_path("global-relative-data-root");
    std::fs::create_dir_all(&workspace).unwrap();
    let input = serde_json::json!({
        "session_id": "session-global",
        "hook_event_name": "PreToolUse",
        "cwd": workspace,
        "turn_id": "turn-global",
        "tool_name": "apply_patch",
        "tool_use_id": "tool-global",
        "tool_input": {"patch": "ignored"}
    });
    let output = run(
        &["codex-global-pre-tool-use", "--data-root", "relative/data"],
        Some(&serde_json::to_string(&input).unwrap()),
    );
    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["hookSpecificOutput"]["permissionDecision"], "deny");
    assert!(
        response["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .unwrap()
            .starts_with("APORIC_PROJECT_INVALID")
    );
}

#[test]
fn project_init_rejects_a_data_root_inside_the_workspace() {
    let workspace = temp_path("inside-workspace-data-root");
    std::fs::create_dir_all(&workspace).unwrap();
    let output = run(
        &[
            "project-init",
            "--workspace",
            workspace.to_str().unwrap(),
            "--scope",
            "repo",
            "--data-root",
            workspace.join("data").to_str().unwrap(),
        ],
        None,
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("outside the bound workspace"));
    assert!(!workspace.join(".aporic").exists());
}

#[test]
fn project_init_can_migrate_a_v1_store_without_changing_the_source() {
    let workspace = temp_path("project-init-migration");
    let data_root = temp_path("project-init-migration-data");
    let source = temp_path("project-init-migration-source").join("events-v1.jsonl");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    let record = serde_json::json!({
        "sequence": 1,
        "schema_version": 1,
        "event_id": "migration-hold",
        "idempotency_key": "migration-hold-key",
        "expected_revision": 0,
        "actor": {"kind": "human", "id": "user", "provenance": "cli-test"},
        "scope": "repo",
        "event": {
            "type": "tool_hold_placed",
            "tool_hold_id": "migration-hold",
            "tool_name": "apply_patch",
            "reason": "preserve this hold"
        }
    });
    let source_bytes = format!("{record}\n");
    std::fs::write(&source, &source_bytes).unwrap();

    let setup = run(
        &[
            "project-init",
            "--workspace",
            workspace.to_str().unwrap(),
            "--scope",
            "repo",
            "--data-root",
            data_root.to_str().unwrap(),
            "--migrate-from-v1-store",
            source.to_str().unwrap(),
        ],
        None,
    );
    assert!(
        setup.status.success(),
        "{}",
        String::from_utf8_lossy(&setup.stderr)
    );
    let setup: Value = serde_json::from_slice(&setup.stdout).unwrap();
    assert_eq!(setup["status"], "migrated");
    assert_eq!(setup["migration"]["from_schema"], 1);
    assert_eq!(setup["migration"]["to_schema"], 3);
    assert_eq!(std::fs::read_to_string(&source).unwrap(), source_bytes);

    let status = run(
        &["status", "--store", setup["store"].as_str().unwrap()],
        None,
    );
    assert!(status.status.success());
    let state: Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(state["revision"], 1);
    assert_eq!(state["tool_holds"]["migration-hold"]["active"], true);
}

#[test]
fn failed_project_migration_does_not_publish_a_binding_and_can_retry() {
    let workspace = temp_path("project-init-migration-retry");
    let data_root = temp_path("project-init-migration-retry-data");
    let source = temp_path("project-init-migration-retry-source").join("events-v1.jsonl");
    std::fs::create_dir_all(&workspace).unwrap();

    let args = [
        "project-init",
        "--workspace",
        workspace.to_str().unwrap(),
        "--scope",
        "repo",
        "--data-root",
        data_root.to_str().unwrap(),
        "--migrate-from-v1-store",
        source.to_str().unwrap(),
    ];
    let failed = run(&args, None);
    assert_eq!(failed.status.code(), Some(1));
    assert!(!workspace.join(".aporic").exists());

    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(&source, b"").unwrap();
    let retried = run(&args, None);
    assert!(
        retried.status.success(),
        "{}",
        String::from_utf8_lossy(&retried.stderr)
    );
    let result: Value = serde_json::from_slice(&retried.stdout).unwrap();
    assert_eq!(result["status"], "migrated");
    assert!(workspace.join(".aporic/config.json").is_file());
    assert!(std::path::Path::new(result["store"].as_str().unwrap()).is_file());
}

#[test]
fn policy_rejection_uses_exit_two_and_structured_json() {
    let store = temp_path("policy-rejection").join("events.jsonl");
    let store_arg = store.to_str().unwrap();
    assert!(run(&["init", "--store", store_arg], None).status.success());

    let request = serde_json::json!({
        "schema_version": 3,
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
    assert_eq!(projection["schema"], 4);
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
