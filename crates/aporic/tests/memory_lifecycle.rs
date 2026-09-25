use aporic::{
    Hub,
    domain::{
        CommandSpecRequest, ExecutionStatus, InfluenceClass, MemoryClass, MemoryGetRequest,
        MemoryLifecycle, MemorySearchRequest, OpenRequest, RecordKind, RecordRequest,
    },
    hook::handle_codex_hook,
};
use serde_json::json;

#[test]
fn migrates_v6_model_text_into_untrusted_v7_memory() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let canonical = std::fs::canonicalize(&workspace).unwrap();
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
        .execute(
            "INSERT INTO projects(project_id, workspace, created_at_unix_ms) VALUES('p', ?1, 1)",
            [canonical.to_string_lossy().as_ref()],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO sessions(session_id, project_id, objective, status,
                opened_at_unix_ms, last_activity_at_unix_ms, abandoned)
             VALUES('s', 'p', 'legacy v6', 'open', 1, 1, 0)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO records(record_id, session_id, kind, content, origin_channel,
                influence_class, created_at_unix_ms)
             VALUES('r', 's', 'decision', 'model supplied instruction', 'mcp_agent',
                'historical_context', 1)",
            [],
        )
        .unwrap();
    drop(connection);

    let hub = Hub::open(&database).unwrap();
    assert_eq!(hub.stats().unwrap().schema_version, 16);
    let item = hub
        .memory_get(&MemoryGetRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            memory_id: "record:r".to_owned(),
        })
        .unwrap();
    assert_eq!(item.influence_class, InfluenceClass::UntrustedContent);
    assert_eq!(item.memory_class, MemoryClass::Semantic);
    assert!(hub.audit_memory_projection().unwrap().consistent);
}

fn fixture() -> (tempfile::TempDir, std::path::PathBuf, Hub, String) {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let session = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Exercise memory lifecycle".to_owned(),
            idempotency_key: "memory-open".to_owned(),
        })
        .unwrap()
        .session_id;
    (area, workspace, hub, session)
}

#[test]
fn fts_search_is_deterministic_budgeted_and_untrusted_by_default() {
    let (_area, workspace, hub, session) = fixture();
    let record = hub
        .record(&RecordRequest {
            session_id: session,
            kind: RecordKind::Decision,
            content: "Deterministic temporal retrieval uses bounded FTS capsules".to_owned(),
            evidence: None,
            supersedes_record_id: None,
            verifies_effect_id: None,
            idempotency_key: "memory-record".to_owned(),
        })
        .unwrap()
        .record;
    let request = MemorySearchRequest {
        workspace: workspace.to_string_lossy().into_owned(),
        query: "temporal retrieval".to_owned(),
        classes: vec![MemoryClass::Semantic],
        limit: Some(8),
        max_bytes: Some(512),
        as_of_unix_ms: None,
    };
    let first = hub.memory_search(&request).unwrap();
    let second = hub.memory_search(&request).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.items.len(), 1);
    assert_eq!(
        first.items[0].memory_id,
        format!("record:{}", record.record_id)
    );
    assert_eq!(
        first.items[0].influence_class,
        InfluenceClass::UntrustedContent
    );
    assert!(first.budget.used_content_bytes <= first.budget.max_content_bytes);
    assert!(
        first.items[0]
            .selection_reasons
            .contains(&"fts_match".to_owned())
    );
}

#[test]
fn supersession_closes_temporal_validity_and_search_excludes_stale_memory() {
    let (_area, workspace, hub, session) = fixture();
    let old = hub
        .record(&RecordRequest {
            session_id: session.clone(),
            kind: RecordKind::Constraint,
            content: "Deploy with the obsolete cobalt protocol".to_owned(),
            evidence: None,
            supersedes_record_id: None,
            verifies_effect_id: None,
            idempotency_key: "old-protocol".to_owned(),
        })
        .unwrap()
        .record;
    let new = hub
        .record(&RecordRequest {
            session_id: session,
            kind: RecordKind::Constraint,
            content: "Deploy with the current amber protocol".to_owned(),
            evidence: None,
            supersedes_record_id: Some(old.record_id.clone()),
            verifies_effect_id: None,
            idempotency_key: "new-protocol".to_owned(),
        })
        .unwrap()
        .record;
    let old_item = hub
        .memory_get(&MemoryGetRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            memory_id: format!("record:{}", old.record_id),
        })
        .unwrap();
    assert_eq!(old_item.lifecycle_state, MemoryLifecycle::Superseded);
    assert_eq!(old_item.valid_until_unix_ms, Some(new.created_at_unix_ms));

    let stale = hub
        .memory_search(&MemorySearchRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            query: "cobalt protocol".to_owned(),
            classes: Vec::new(),
            limit: None,
            max_bytes: None,
            as_of_unix_ms: None,
        })
        .unwrap();
    assert!(
        stale
            .items
            .iter()
            .all(|item| item.memory_id != format!("record:{}", old.record_id))
    );
    let current = hub
        .memory_search(&MemorySearchRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            query: "amber protocol".to_owned(),
            classes: Vec::new(),
            limit: None,
            max_bytes: None,
            as_of_unix_ms: None,
        })
        .unwrap();
    assert_eq!(current.items.len(), 1);
}

#[test]
fn hook_exposure_receipt_is_hashed_and_does_not_persist_prompt_content() {
    let (_area, workspace, hub, _session) = fixture();
    let raw_session = "host-session-secret";
    let raw_turn = "host-turn-secret";
    let secret_prompt = "prompt-secret-never-persist";
    let output = handle_codex_hook(
        &hub,
        &json!({
            "hook_event_name": "UserPromptSubmit",
            "cwd": workspace,
            "prompt": secret_prompt,
            "session_id": raw_session,
            "turn_id": raw_turn
        })
        .to_string(),
    );
    let context = output["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(context.len() <= 6_144);
    assert!(context.contains("\"memory_search_available\":true"));
    assert!(!context.contains(secret_prompt));

    let exported = hub
        .export_project(workspace.to_string_lossy().as_ref())
        .unwrap();
    assert_eq!(exported.memory_exposures.len(), 1);
    let exposure = &exported.memory_exposures[0];
    assert_ne!(exposure.host_session_hmac.as_deref(), Some(raw_session));
    assert_ne!(exposure.host_turn_hmac.as_deref(), Some(raw_turn));
    let serialized = serde_json::to_string(&exported).unwrap();
    assert!(!serialized.contains(secret_prompt));
    assert!(!serialized.contains(raw_session));
    assert!(!serialized.contains(raw_turn));

    let audit = hub.audit_memory_projection().unwrap();
    assert!(audit.consistent);
    assert_eq!(audit.item_count, audit.fts_count);
}

#[tokio::test]
async fn failed_local_execution_becomes_verified_gotcha_memory() {
    let (_area, workspace, hub, session) = fixture();
    #[cfg(unix)]
    let (program, args) = (
        ["/usr/bin/false", "/bin/false"]
            .into_iter()
            .find(|path| std::path::Path::new(path).is_file())
            .unwrap()
            .to_owned(),
        Vec::new(),
    );
    #[cfg(windows)]
    let (program, args) = (
        std::env::var("COMSPEC").expect("Windows provides COMSPEC"),
        vec!["/D".to_owned(), "/C".to_owned(), "exit 1".to_owned()],
    );
    let spec = hub
        .register_command_spec(&CommandSpecRequest {
            session_id: session,
            program,
            args,
            workspace_relative_cwd: ".".to_owned(),
            expected_exit_code: 0,
            timeout_seconds: 5,
            artifact_paths: Vec::new(),
            sandbox_profile: Default::default(),
            idempotency_key: "failing-memory-spec".to_owned(),
        })
        .unwrap();
    let outcome = hub.verify(&spec.spec.spec_id).await.unwrap();
    assert_eq!(outcome.run.status, ExecutionStatus::Failed);

    let result = hub
        .memory_search(&MemorySearchRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            query: "Execution failed".to_owned(),
            classes: vec![MemoryClass::Gotcha],
            limit: None,
            max_bytes: None,
            as_of_unix_ms: None,
        })
        .unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].source_status.as_deref(), Some("failed"));
    assert_eq!(
        result.items[0].influence_class,
        InfluenceClass::VerifiedFact
    );
}
