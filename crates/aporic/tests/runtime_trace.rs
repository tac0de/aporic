use aporic::{
    Hub,
    domain::{
        CapabilityClass, OpenRequest, RuntimeEventKind, RuntimeOutcomeStatus,
        RuntimeTraceGetRequest, RuntimeTraceListRequest, RuntimeWorkspaceRequest,
        ShadowDisposition,
    },
    hook::handle_codex_hook,
};
use serde_json::json;

fn fixture() -> (tempfile::TempDir, std::path::PathBuf, Hub) {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    hub.open_session(&OpenRequest {
        workspace: workspace.to_string_lossy().into_owned(),
        objective: "Observe runtime hooks without controlling them".to_owned(),
        idempotency_key: "runtime-open".to_owned(),
    })
    .unwrap();
    (area, workspace, hub)
}

fn hook(hub: &Hub, value: serde_json::Value) -> serde_json::Value {
    handle_codex_hook(hub, &value.to_string())
}

#[test]
fn migrates_v7_to_v8_without_runtime_events() {
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
    drop(connection);
    let hub = Hub::open(database).unwrap();
    assert_eq!(hub.stats().unwrap().schema_version, 24);
    assert!(hub.audit_runtime_projection().unwrap().consistent);
}

#[test]
fn hook_trace_is_correlated_hashed_and_content_free() {
    let (_area, workspace, hub) = fixture();
    let secret_prompt = "PROMPT-SECRET-2de31";
    let secret_input = "TOOL-INPUT-SECRET-871b";
    let secret_output = "TOOL-OUTPUT-SECRET-331a";
    let context = hook(
        &hub,
        json!({
            "hook_event_name": "UserPromptSubmit", "cwd": workspace,
            "session_id": "host-session", "turn_id": "host-turn",
            "prompt": secret_prompt
        }),
    );
    assert!(context["hookSpecificOutput"]["additionalContext"].is_string());
    assert_eq!(
        hook(
            &hub,
            json!({
                "hook_event_name": "PreToolUse", "cwd": workspace,
                "session_id": "host-session", "turn_id": "host-turn",
                "tool_use_id": "tool-call-1", "tool_name": "Bash",
                "tool_input": {"command": format!("echo {secret_input}")}
            })
        ),
        json!({})
    );
    hook(
        &hub,
        json!({
            "hook_event_name": "PostToolUse", "cwd": workspace,
            "session_id": "host-session", "turn_id": "host-turn",
            "tool_use_id": "tool-call-1", "tool_name": "Bash",
            "tool_response": {"stdout": secret_output}, "duration_ms": 17
        }),
    );

    let events = hub
        .list_runtime_events(&RuntimeTraceListRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            limit: None,
            event_kinds: Vec::new(),
        })
        .unwrap();
    assert_eq!(events.len(), 3);
    let pre = events
        .iter()
        .find(|event| event.event_kind == RuntimeEventKind::PreTool)
        .unwrap();
    let post = events
        .iter()
        .find(|event| event.event_kind == RuntimeEventKind::PostTool)
        .unwrap();
    assert_eq!(pre.host_session_hmac, post.host_session_hmac);
    assert_eq!(pre.host_tool_call_hmac, post.host_tool_call_hmac);
    assert_eq!(pre.capability_class, CapabilityClass::Execute);
    assert_eq!(pre.outcome_status, RuntimeOutcomeStatus::Proposed);
    assert_eq!(post.outcome_status, RuntimeOutcomeStatus::Succeeded);
    assert_eq!(post.latency_ms, Some(17));
    assert!(pre.input_hmac.is_some());
    assert!(post.output_hmac.is_some());
    let exposure_id = events
        .iter()
        .find(|event| event.event_kind == RuntimeEventKind::UserPrompt)
        .unwrap()
        .exposure_id
        .as_ref()
        .unwrap();
    assert_eq!(pre.exposure_id.as_ref(), Some(exposure_id));
    assert_eq!(post.exposure_id.as_ref(), Some(exposure_id));
    let serialized = serde_json::to_string(
        &hub.export_project(workspace.to_string_lossy().as_ref())
            .unwrap(),
    )
    .unwrap();
    for secret in [
        secret_prompt,
        secret_input,
        secret_output,
        "host-session",
        "tool-call-1",
    ] {
        assert!(!serialized.contains(secret));
    }

    let request = RuntimeWorkspaceRequest {
        workspace: workspace.to_string_lossy().into_owned(),
    };
    let health = hub.hook_health(&request).unwrap();
    assert!(health.no_detected_gaps);
    assert!(!health.coverage_proven);
    let capabilities = hub.capability_report(&request).unwrap();
    assert_eq!(capabilities.observations.len(), 1);
    assert_eq!(capabilities.observations[0].event_count, 2);
    assert_eq!(capabilities.observations[0].succeeded_count, 1);
    assert!(capabilities.authority_notice.contains("not permissions"));
}

#[test]
fn shadow_policy_never_blocks_but_marks_destructive_patterns() {
    let (_area, workspace, hub) = fixture();
    let output = hook(
        &hub,
        json!({
            "hook_event_name": "PreToolUse", "cwd": workspace,
            "session_id": "s", "turn_id": "t", "tool_use_id": "danger",
            "tool_name": "Bash", "tool_input": {"command": "rm -rf /tmp/fixture-only"}
        }),
    );
    assert_eq!(output, json!({}));
    let events = hub
        .list_runtime_events(&RuntimeTraceListRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            limit: None,
            event_kinds: vec![RuntimeEventKind::PreTool],
        })
        .unwrap();
    assert_eq!(events[0].shadow_disposition, ShadowDisposition::WouldDeny);
    assert_eq!(events[0].shadow_reasons, ["destructive_pattern"]);
}

#[test]
fn health_detects_gaps_duplicates_and_unknown_host_schema() {
    let (_area, workspace, hub) = fixture();
    let pre = json!({
        "hook_event_name": "PreToolUse", "cwd": workspace,
        "session_id": "s", "turn_id": "t", "tool_use_id": "unmatched",
        "tool_name": "MysteryQuantumTool", "tool_input": {"opaque": true}
    });
    hook(&hub, pre.clone());
    hook(&hub, pre);
    hook(
        &hub,
        json!({"hook_event_name": "FutureHookEvent", "cwd": workspace, "session_id": "s"}),
    );
    hook(
        &hub,
        json!({
            "hook_event_name": "PostToolUseFailure", "cwd": workspace,
            "session_id": "s", "tool_use_id": "orphan-terminal", "tool_name": "Bash"
        }),
    );
    let health = hub
        .hook_health(&RuntimeWorkspaceRequest {
            workspace: workspace.to_string_lossy().into_owned(),
        })
        .unwrap();
    assert!(!health.no_detected_gaps);
    assert!(!health.coverage_proven);
    assert_eq!(health.unmatched_pre_tool_count, 2);
    assert_eq!(health.terminal_without_pre_count, 1);
    assert_eq!(health.duplicate_count, 1);
    assert_eq!(health.unknown_event_count, 1);
    assert_eq!(health.unknown_capability_count, 2);
}

#[test]
fn trace_get_and_otel_export_are_read_only_and_payload_free() {
    let (_area, workspace, hub) = fixture();
    hook(
        &hub,
        json!({
            "hook_event_name": "PreToolUse", "cwd": workspace,
            "session_id": "s", "turn_id": "t", "tool_use_id": "read-1",
            "tool_name": "Read", "tool_input": {"file_path": "/private/secret.txt"}
        }),
    );
    let listed = hub
        .list_runtime_events(&RuntimeTraceListRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            limit: Some(1),
            event_kinds: Vec::new(),
        })
        .unwrap();
    let read = hub
        .get_runtime_event(&RuntimeTraceGetRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            event_id: listed[0].event_id.clone(),
        })
        .unwrap();
    assert_eq!(read, listed[0]);
    let exported = hub
        .export_runtime_otel(&RuntimeWorkspaceRequest {
            workspace: workspace.to_string_lossy().into_owned(),
        })
        .unwrap();
    assert_eq!(exported["network_exported"], false);
    assert_eq!(exported["content_recorded"], false);
    assert!(!exported.to_string().contains("/private/secret.txt"));
    assert_eq!(exported["spans"][0]["trace_id"].as_str().unwrap().len(), 32);
    assert_eq!(exported["spans"][0]["span_id"].as_str().unwrap().len(), 16);
    assert!(hub.audit_runtime_projection().unwrap().consistent);
}
