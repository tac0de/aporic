use aporic::{
    Hub,
    domain::{
        CloseDisposition, CloseRequest, OpenRequest, RecallRequest, RecordKind, RecordRequest,
    },
};

#[test]
fn persists_context_across_hub_restarts_and_deduplicates_retries() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let database = area.path().join("state/aporic.sqlite3");

    let open_request = OpenRequest {
        workspace: workspace.to_string_lossy().into_owned(),
        objective: "Preserve useful continuity".to_owned(),
        idempotency_key: "open-1".to_owned(),
    };
    let hub = Hub::open(&database).unwrap();
    let opened = hub.open_session(&open_request).unwrap();
    assert!(!opened.duplicate);
    let duplicate = hub.open_session(&open_request).unwrap();
    assert!(duplicate.duplicate);
    assert_eq!(duplicate.session_id, opened.session_id);

    let conflicting_retry = hub.open_session(&OpenRequest {
        objective: "A different objective".to_owned(),
        ..open_request.clone()
    });
    assert!(
        conflicting_retry
            .unwrap_err()
            .to_string()
            .contains("already used for a different request")
    );

    let record_request = RecordRequest {
        session_id: opened.session_id.clone(),
        kind: RecordKind::Decision,
        content: "Use local stdio MCP as the first transport.".to_owned(),
        evidence: Some("Current product decision".to_owned()),
        supersedes_record_id: None,
        verifies_effect_id: None,
        idempotency_key: "record-1".to_owned(),
    };
    let recorded = hub.record(&record_request).unwrap();
    assert!(!recorded.duplicate);
    assert!(hub.record(&record_request).unwrap().duplicate);

    hub.close_session(&CloseRequest {
        session_id: opened.session_id,
        disposition: CloseDisposition::Handoff,
        summary: "The first design decision was recorded.".to_owned(),
        next_action: Some("Implement the MCP adapter.".to_owned()),
        idempotency_key: "close-1".to_owned(),
    })
    .unwrap();
    assert_eq!(hub.stats().unwrap().event_count, 3);
    drop(hub);

    let restarted = Hub::open(&database).unwrap();
    let recalled = restarted
        .recall(&RecallRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            limit: Some(10),
        })
        .unwrap();
    assert!(recalled.active_sessions.is_empty());
    assert_eq!(recalled.recent_handoffs.len(), 1);
    assert_eq!(
        recalled.recent_handoffs[0].next_action,
        "Implement the MCP adapter."
    );
    assert_eq!(recalled.recent_records.len(), 1);
    assert_eq!(
        recalled.recent_records[0].content,
        "Use local stdio MCP as the first transport."
    );
    assert_eq!(restarted.stats().unwrap().event_count, 3);
}

#[test]
fn handoff_requires_a_next_action_and_closed_sessions_reject_new_records() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let opened = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Test lifecycle failures".to_owned(),
            idempotency_key: "open".to_owned(),
        })
        .unwrap();

    let error = hub
        .close_session(&CloseRequest {
            session_id: opened.session_id.clone(),
            disposition: CloseDisposition::Handoff,
            summary: "Not enough information".to_owned(),
            next_action: None,
            idempotency_key: "bad-close".to_owned(),
        })
        .unwrap_err();
    assert!(error.to_string().contains("next_action is required"));

    hub.close_session(&CloseRequest {
        session_id: opened.session_id.clone(),
        disposition: CloseDisposition::Completed,
        summary: "Lifecycle checked".to_owned(),
        next_action: None,
        idempotency_key: "close".to_owned(),
    })
    .unwrap();

    let error = hub
        .record(&RecordRequest {
            session_id: opened.session_id,
            kind: RecordKind::Observation,
            content: "This must not be stored".to_owned(),
            evidence: None,
            supersedes_record_id: None,
            verifies_effect_id: None,
            idempotency_key: "late-record".to_owned(),
        })
        .unwrap_err();
    assert!(error.to_string().contains("already completed"));
    assert_eq!(hub.stats().unwrap().event_count, 2);
}

#[test]
fn concurrent_process_equivalent_writers_do_not_lose_sessions() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();

    let handles = (0..8)
        .map(|index| {
            let hub = hub.clone();
            let workspace = workspace.clone();
            std::thread::spawn(move || {
                hub.open_session(&OpenRequest {
                    workspace,
                    objective: format!("Concurrent task {index}"),
                    idempotency_key: format!("concurrent-open-{index}"),
                })
                .unwrap()
            })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        handle.join().unwrap();
    }

    let recalled = hub
        .recall(&RecallRequest {
            workspace,
            limit: Some(20),
        })
        .unwrap();
    assert_eq!(recalled.active_sessions.len(), 8);
    assert_eq!(hub.stats().unwrap().event_count, 8);
}

#[test]
fn migrates_v1_state_and_exports_complete_project_history() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let database = area.path().join("aporic.sqlite3");
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE projects (
                project_id TEXT PRIMARY KEY,
                workspace TEXT NOT NULL UNIQUE,
                created_at_unix_ms INTEGER NOT NULL
            );
            CREATE TABLE sessions (
                session_id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(project_id),
                objective TEXT NOT NULL,
                status TEXT NOT NULL CHECK(status IN ('open', 'completed', 'handoff')),
                opened_at_unix_ms INTEGER NOT NULL,
                closed_at_unix_ms INTEGER,
                summary TEXT,
                next_action TEXT
            );
            CREATE TABLE records (
                record_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL REFERENCES sessions(session_id),
                kind TEXT NOT NULL,
                content TEXT NOT NULL,
                evidence TEXT,
                created_at_unix_ms INTEGER NOT NULL
            );
            CREATE TABLE events (
                sequence INTEGER PRIMARY KEY AUTOINCREMENT,
                event_id TEXT NOT NULL UNIQUE,
                idempotency_key TEXT NOT NULL UNIQUE,
                stream_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                result_json TEXT NOT NULL,
                occurred_at_unix_ms INTEGER NOT NULL
            );
            PRAGMA user_version = 1;",
        )
        .unwrap();
    drop(connection);

    let hub = Hub::open(&database).unwrap();
    assert_eq!(hub.stats().unwrap().schema_version, 5);
    let workspace = workspace.to_string_lossy().into_owned();
    let opened = hub
        .open_session(&OpenRequest {
            workspace: workspace.clone(),
            objective: "Verify migration and export".to_owned(),
            idempotency_key: "migration-open".to_owned(),
        })
        .unwrap();
    hub.record(&RecordRequest {
        session_id: opened.session_id,
        kind: RecordKind::Observation,
        content: "The migrated database accepted a record.".to_owned(),
        evidence: Some("Successful transaction".to_owned()),
        supersedes_record_id: None,
        verifies_effect_id: None,
        idempotency_key: "migration-record".to_owned(),
    })
    .unwrap();

    let exported = hub.export_project(&workspace).unwrap();
    assert_eq!(exported.format_version, 3);
    assert_eq!(exported.sessions.len(), 1);
    assert_eq!(exported.records.len(), 1);
    assert!(exported.tasks.is_empty());
    assert!(exported.evidence.is_empty());
    assert!(exported.claims.is_empty());
    assert!(exported.command_specs.is_empty());
    assert!(exported.execution_runs.is_empty());
    assert!(exported.execution_receipts.is_empty());
    assert!(exported.receipt_artifacts.is_empty());
    assert_eq!(exported.events.len(), 2);
}
