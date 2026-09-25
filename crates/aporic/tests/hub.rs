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
        content: "Use a local stdio MCP vertical slice first.".to_owned(),
        evidence: Some("Current product decision".to_owned()),
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
    assert_eq!(hub.event_count().unwrap(), 3);
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
        "Use a local stdio MCP vertical slice first."
    );
    assert_eq!(restarted.event_count().unwrap(), 3);
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
            idempotency_key: "late-record".to_owned(),
        })
        .unwrap_err();
    assert!(error.to_string().contains("already completed"));
    assert_eq!(hub.event_count().unwrap(), 2);
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
    assert_eq!(hub.event_count().unwrap(), 8);
}
