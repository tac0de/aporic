use aporic::codex::{PostToolUseInput, post_tool_use_transaction};
use aporic::policy::MAX_POLICY_TOOLS;
use aporic::project::{initialize_project, store_path};
use aporic::{
    Actor, ActorKind, CommitRequest, Event, EvidenceKind, SCHEMA_VERSION, commit, initialize, load,
};
use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "aporic-mcp-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn run_mcp(data_root: &Path, requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aporic"))
        .arg("mcp-serve")
        .env("APORIC_DATA_HOME", data_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        for request in requests {
            serde_json::to_writer(&mut *stdin, request).unwrap();
            stdin.write_all(b"\n").unwrap();
        }
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn mcp_lists_bounded_tools_and_read_only_calls_do_not_mutate_state() {
    let workspace = temp_path("workspace");
    let data_root = temp_path("data");
    std::fs::create_dir_all(&workspace).unwrap();
    initialize_project(&workspace, "repo").unwrap();
    let workspace = std::fs::canonicalize(workspace).unwrap();
    let store = store_path(&data_root, &workspace).unwrap();
    initialize(&store).unwrap();

    let responses = run_mcp(
        &data_root,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "1"}}}),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "project_status", "arguments": {"workspace": workspace}}}),
            json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "explain_action", "arguments": {"workspace": workspace, "session_id": "session", "tool_name": "apply_patch", "tool_input": {"patch": "bounded"}}}}),
        ],
    );

    assert_eq!(responses.len(), 4);
    let tools = responses[1]["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 4);
    assert_eq!(
        tools
            .iter()
            .filter(|tool| tool["annotations"]["readOnlyHint"] == false)
            .map(|tool| tool["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["ingest_verifier_report"]
    );
    assert_eq!(responses[2]["result"]["structuredContent"]["revision"], 0);
    assert_eq!(
        responses[2]["result"]["structuredContent"]["protected_tool_count"],
        1
    );
    assert_eq!(
        responses[3]["result"]["structuredContent"]["evaluation"]["reason_code"],
        "PLAN_AUTHORIZATION_REQUIRED"
    );
    assert_eq!(load(&store).unwrap().state().revision, 0);
}

#[test]
fn project_status_stays_count_only_at_the_policy_limit() {
    let workspace = temp_path("large-policy-workspace");
    let data_root = temp_path("large-policy-data");
    std::fs::create_dir_all(&workspace).unwrap();
    initialize_project(&workspace, "repo").unwrap();
    let workspace = std::fs::canonicalize(workspace).unwrap();
    let store = store_path(&data_root, &workspace).unwrap();
    initialize(&store).unwrap();
    let tools = (0..MAX_POLICY_TOOLS)
        .map(|index| {
            (
                format!("attacker_controlled_tool_{index}"),
                json!({
                    "require_plan": false,
                    "require_grant": false,
                    "require_intent": false
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    std::fs::write(
        workspace.join(".aporic/policy.json"),
        serde_json::to_vec(&json!({"schema_version": 2, "tools": tools})).unwrap(),
    )
    .unwrap();

    let responses = run_mcp(
        &data_root,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "project_status", "arguments": {"workspace": workspace}}}),
        ],
    );
    let encoded = serde_json::to_vec(&responses[1]).unwrap();
    assert!(encoded.len() < 4_096);
    assert_eq!(
        responses[1]["result"]["structuredContent"]["protected_tool_count"],
        MAX_POLICY_TOOLS
    );
    assert!(
        !String::from_utf8(encoded)
            .unwrap()
            .contains("attacker_controlled_tool")
    );
}

#[test]
fn mcp_rejects_unbound_and_unknown_tool_calls_as_tool_errors() {
    let data_root = temp_path("unbound-data");
    let workspace = temp_path("unbound-workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let responses = run_mcp(
        &data_root,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "project_status", "arguments": {"workspace": workspace}}}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "authorize_plan", "arguments": {}}}),
        ],
    );
    assert_eq!(responses[1]["result"]["isError"], true);
    assert_eq!(responses[2]["result"]["isError"], true);
}

#[test]
fn verifier_ingestion_through_mcp_is_idempotent_and_invalid_input_does_not_append() {
    let workspace = temp_path("verifier-workspace");
    let data_root = temp_path("verifier-data");
    std::fs::create_dir_all(&workspace).unwrap();
    initialize_project(&workspace, "repo").unwrap();
    let workspace = std::fs::canonicalize(workspace).unwrap();
    let store = store_path(&data_root, &workspace).unwrap();
    initialize(&store).unwrap();
    let request = |revision, id: &str, actor_kind, event| CommitRequest {
        schema_version: SCHEMA_VERSION,
        event_id: id.into(),
        idempotency_key: id.into(),
        expected_revision: revision,
        actor: Actor {
            kind: actor_kind,
            id: "mcp-test".into(),
            provenance: "mcp-test".into(),
        },
        scope: "repo".into(),
        event,
    };
    commit(
        &store,
        request(
            0,
            "plan",
            ActorKind::Human,
            Event::PlanRegistered {
                plan_id: "plan".into(),
                objective: "verify effect".into(),
                acceptance_checks: vec!["state matches".into()],
                unresolved_questions: vec![],
                intent_id: None,
            },
        ),
    )
    .unwrap();
    post_tool_use_transaction(
        &store,
        &PostToolUseInput {
            session_id: "session".into(),
            hook_event_name: "PostToolUse".into(),
            cwd: workspace.to_string_lossy().into_owned(),
            turn_id: "turn".into(),
            tool_name: "apply_patch".into(),
            tool_use_id: "use".into(),
            tool_input: json!({"patch": "..."}),
            tool_response: json!({"ok": true}),
            model: None,
            permission_mode: None,
        },
        "repo",
    )
    .unwrap();
    let receipt_id = load(&store)
        .unwrap()
        .state()
        .effect_receipts
        .values()
        .next()
        .unwrap()
        .id
        .clone();
    commit(
        &store,
        request(
            2,
            "evidence",
            ActorKind::Evidence,
            Event::EvidenceRecorded {
                evidence_id: "evidence".into(),
                kind: EvidenceKind::RepositoryState,
                locator: "git:worktree".into(),
                digest: None,
            },
        ),
    )
    .unwrap();
    let report = json!({
        "workspace": workspace,
        "schema_version": 1,
        "verification_id": "verification",
        "receipt_id": receipt_id,
        "verifier_id": "repo-verifier",
        "provenance": "mcp-test",
        "plan_id": "plan",
        "check_index": 0,
        "result": "passed",
        "evidence_refs": ["evidence"]
    });
    let mut invalid_report = report.clone();
    invalid_report["verification_id"] = json!("invalid-verification");
    invalid_report["receipt_id"] = json!("missing-receipt");
    let responses = run_mcp(
        &data_root,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "ingest_verifier_report", "arguments": report}}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "ingest_verifier_report", "arguments": report}}),
            json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "ingest_verifier_report", "arguments": invalid_report}}),
        ],
    );
    assert_eq!(responses[1]["result"]["isError"], false);
    assert_eq!(responses[2]["result"]["isError"], false);
    assert_eq!(responses[3]["result"]["isError"], true);
    let state = load(&store).unwrap();
    assert_eq!(state.state().revision, 4);
    assert_eq!(state.state().verifications.len(), 1);
}
