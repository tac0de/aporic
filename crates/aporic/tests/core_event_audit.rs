use aporic::{
    Hub,
    domain::{
        ClaimRequest, ClaimStatus, EvidenceKind, EvidenceRequest, OpenRequest, RecordKind,
        RecordRequest, TaskCancelRequest, TaskClaimRequest, TaskCreateRequest,
    },
};

#[test]
fn event_covered_rows_are_checked_and_legacy_rows_are_not_attested() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Audit event covered core state".to_owned(),
            idempotency_key: "core-audit-open".to_owned(),
        })
        .unwrap()
        .session_id;
    let record_id = hub
        .record(&RecordRequest {
            session_id: session_id.clone(),
            kind: RecordKind::Decision,
            content: "Original decision".to_owned(),
            evidence: None,
            supersedes_record_id: None,
            verifies_effect_id: None,
            idempotency_key: "core-audit-record".to_owned(),
        })
        .unwrap()
        .record
        .record_id;
    hub.add_evidence(&EvidenceRequest {
        session_id: session_id.clone(),
        kind: EvidenceKind::UserStatement,
        locator: "user:reported".to_owned(),
        summary: "Reported evidence".to_owned(),
        content_sha256: Some("a".repeat(64)),
        idempotency_key: "core-audit-evidence".to_owned(),
    })
    .unwrap();
    hub.assert_claim(&ClaimRequest {
        session_id: session_id.clone(),
        status: ClaimStatus::Unknown,
        statement: "Whether this is verified".to_owned(),
        subject_key: None,
        material: false,
        evidence_ids: Vec::new(),
        supersedes_claim_id: None,
        idempotency_key: "core-audit-claim".to_owned(),
    })
    .unwrap();
    hub.create_task(&TaskCreateRequest {
        session_id: session_id.clone(),
        objective: "Check core event replay".to_owned(),
        acceptance_criteria: vec!["The core audit catches drift".to_owned()],
        write_scope: Vec::new(),
        depends_on: Vec::new(),
        idempotency_key: "core-audit-task".to_owned(),
    })
    .unwrap();

    let clean = hub.audit_core_events().unwrap();
    assert!(clean.covered_consistent);
    assert_eq!(clean.covered_records, 1);
    assert_eq!(clean.covered_evidence, 1);
    assert_eq!(clean.covered_claims, 1);
    assert_eq!(clean.covered_tasks, 1);
    assert_eq!(clean.uncovered_legacy_count, 0);

    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute(
            "UPDATE records SET content = 'Altered decision' WHERE record_id = ?1",
            [&record_id],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO records(record_id, session_id, kind, content, origin_channel,
                                 influence_class, created_at_unix_ms)
             VALUES('legacy-record', ?1, 'observation', 'Imported without an event',
                    'legacy', 'historical_context', 1)",
            [&session_id],
        )
        .unwrap();
    let audit = hub.audit_core_events().unwrap();
    assert!(!audit.covered_consistent);
    assert_eq!(audit.mismatch_count, 1);
    assert!(
        audit
            .mismatch_sample
            .contains(&format!("record:{record_id}"))
    );
    assert_eq!(audit.uncovered_legacy_count, 1);
    assert!(
        audit
            .uncovered_sample
            .contains(&"record:legacy-record".to_owned())
    );
}

#[test]
fn legal_supersession_and_task_transitions_keep_covered_audit_clean() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Exercise legal state transitions".to_owned(),
            idempotency_key: "transition-open".to_owned(),
        })
        .unwrap()
        .session_id;
    let first_claim = hub
        .assert_claim(&ClaimRequest {
            session_id: session_id.clone(),
            status: ClaimStatus::Assumed,
            statement: "Old assumption".to_owned(),
            subject_key: None,
            material: false,
            evidence_ids: Vec::new(),
            supersedes_claim_id: None,
            idempotency_key: "transition-first-claim".to_owned(),
        })
        .unwrap()
        .claim;
    hub.assert_claim(&ClaimRequest {
        session_id: session_id.clone(),
        status: ClaimStatus::Intended,
        statement: "Revised intention".to_owned(),
        subject_key: None,
        material: false,
        evidence_ids: Vec::new(),
        supersedes_claim_id: Some(first_claim.claim_id),
        idempotency_key: "transition-second-claim".to_owned(),
    })
    .unwrap();
    let task = hub
        .create_task(&TaskCreateRequest {
            session_id,
            objective: "Transition task".to_owned(),
            acceptance_criteria: vec!["Task was exercised".to_owned()],
            write_scope: Vec::new(),
            depends_on: Vec::new(),
            idempotency_key: "transition-task".to_owned(),
        })
        .unwrap()
        .task;
    hub.claim_task(&TaskClaimRequest {
        task_id: task.task_id.clone(),
        worker_id: "worker".to_owned(),
        lease_seconds: 300,
        idempotency_key: "transition-lease".to_owned(),
    })
    .unwrap();
    hub.cancel_task(&TaskCancelRequest {
        task_id: task.task_id,
        reason: "Cancelled after inspection".to_owned(),
        idempotency_key: "transition-cancel".to_owned(),
    })
    .unwrap();
    let audit = hub.audit_core_events().unwrap();
    assert!(audit.covered_consistent);
    assert_eq!(audit.covered_claims, 2);
    assert_eq!(audit.covered_tasks, 1);
    assert_eq!(audit.mismatch_count, 0);

    // Historical event shapes may not decode under the current struct. The
    // audit reports this limit without claiming those rows were replayed.
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute(
            "INSERT INTO events(event_id, idempotency_key, stream_id, kind,
                                payload_json, result_json, occurred_at_unix_ms)
             VALUES('old-event', 'old-event', 'old-task', 'task_created', '{}', '{}', 1)",
            [],
        )
        .unwrap();
    let limited = hub.audit_core_events().unwrap();
    assert!(limited.covered_consistent);
    assert_eq!(limited.unsupported_event_count, 1);
    assert!(limited.scope_notice.contains("not attested"));
}

#[test]
fn v6_record_authority_migration_does_not_create_a_false_mismatch() {
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
            "INSERT INTO projects(project_id, workspace, created_at_unix_ms)
             VALUES('p', ?1, 1)",
            [canonical.to_string_lossy().as_ref()],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO sessions(session_id, project_id, objective, status,
                                  opened_at_unix_ms, last_activity_at_unix_ms, abandoned)
             VALUES('s', 'p', 'old work', 'open', 1, 1, 0)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO records(record_id, session_id, kind, content, origin_channel,
                                 influence_class, created_at_unix_ms)
             VALUES('r', 's', 'decision', 'old model decision', 'mcp_agent',
                    'historical_context', 1)",
            [],
        )
        .unwrap();
    let result = serde_json::json!({
        "record": {
            "record_id": "r", "session_id": "s", "kind": "decision",
            "content": "old model decision", "evidence": null,
            "supersedes_record_id": null, "verifies_effect_id": null,
            "origin_channel": "mcp_agent", "influence_class": "historical_context",
            "created_at_unix_ms": 1
        },
        "duplicate": false
    });
    connection
        .execute(
            "INSERT INTO events(event_id, idempotency_key, stream_id, kind,
                                payload_json, result_json, occurred_at_unix_ms)
             VALUES('e', 'v6-record', 's', 'record_added', '{}', ?1, 1)",
            [result.to_string()],
        )
        .unwrap();
    drop(connection);

    let hub = Hub::open(&database).unwrap();
    let audit = hub.audit_core_events().unwrap();
    assert_eq!(audit.covered_records, 1);
    assert_eq!(audit.uncovered_legacy_count, 0);
    assert!(audit.covered_consistent);
}
