use aporic::{Hub, core_flow::*, domain::*};
fn begin(workspace: &std::path::Path, key: &str) -> BeginRequest {
    BeginRequest {
        workspace: workspace.to_string_lossy().into_owned(),
        objective: key.into(),
        idempotency_key: key.into(),
        work_shape: WorkShape {
            parallel_paths: 1,
            material_change: true,
            worker: DelegationChoice {
                disposition: DelegationDisposition::Skip,
                reason: "One sequential path".into(),
            },
            reviewer: DelegationChoice {
                disposition: DelegationDisposition::Skip,
                reason: "Bounded fixture verified mechanically".into(),
            },
        },
    }
}
fn finish(id: &str, key: &str) -> FinishRequest {
    FinishRequest {
        session_id: id.into(),
        disposition: CloseDisposition::Completed,
        summary: "Verified fixture".into(),
        next_action: None,
        notes: vec![],
        idempotency_key: key.into(),
    }
}
fn note(kind: RecordKind, content: &str) -> FinishNote {
    FinishNote {
        kind,
        content: content.into(),
        evidence: None,
        supersedes_record_id: None,
        verifies_effect_id: None,
    }
}
#[test]
fn atomic_begin_and_finish_replay_legacy_events_and_idempotency() {
    let area = tempfile::tempdir().unwrap();
    let db = area.path().join("state.sqlite");
    let hub = Hub::open(&db).unwrap();
    let request = begin(area.path(), "begin-one");
    let opened = hub.begin(&request).unwrap();
    assert_eq!(hub.stats().unwrap().event_count, 3);
    assert_eq!(opened.runtime.package_version, env!("CARGO_PKG_VERSION"));
    assert!(opened.context.active_sessions.is_empty());
    assert!(response_bytes(&opened).unwrap() <= MAX_RESPONSE_BYTES);
    assert_eq!(hub.begin(&request).unwrap().session_id, opened.session_id);
    assert!(hub.begin(&request).unwrap().duplicate);
    let mut different = request.clone();
    different.objective = "changed".into();
    assert!(hub.begin(&different).is_err());
    let mut closed = finish(&opened.session_id, "finish-one");
    closed
        .notes
        .push(note(RecordKind::Decision, "Keep the old APIs"));
    let outcome = hub.finish(&closed).unwrap();
    assert_eq!(outcome.record_ids.len(), 1);
    let events = hub.stats().unwrap().event_count;
    assert_eq!(events, 6);
    drop(hub);
    let hub = Hub::open(&db).unwrap();
    assert!(hub.finish(&closed).unwrap().duplicate);
    assert_eq!(hub.stats().unwrap().event_count, events);
    closed.summary = "changed".into();
    assert!(hub.finish(&closed).is_err());
    assert!(hub.audit_core_events().unwrap().covered_consistent);
    let export = hub.export_project(&request.workspace).unwrap();
    let kinds = export
        .events
        .iter()
        .map(|e| e.kind.as_str())
        .collect::<Vec<_>>();
    assert!(kinds.contains(&"session_opened"));
    assert!(kinds.contains(&"session_delegation_assessed"));
    assert!(kinds.contains(&"record_added"));
    assert!(kinds.contains(&"session_closed"));
}
#[test]
fn invalid_assessment_rolls_back_open_and_can_retry() {
    let area = tempfile::tempdir().unwrap();
    let hub = Hub::open(area.path().join("state.sqlite")).unwrap();
    let mut request = begin(area.path(), "bad-assessment");
    request.work_shape.parallel_paths = 33;
    assert!(hub.begin(&request).is_err());
    assert_eq!(hub.stats().unwrap().event_count, 0);
    assert_eq!(hub.stats().unwrap().open_session_count, 0);
    request.work_shape.parallel_paths = 1;
    assert!(hub.begin(&request).is_ok());
}
#[test]
fn invalid_note_or_completion_gate_rolls_back_every_write_and_resume_keeps_open() {
    let area = tempfile::tempdir().unwrap();
    let hub = Hub::open(area.path().join("state.sqlite")).unwrap();
    let request = begin(area.path(), "unfinished");
    let opened = hub.begin(&request).unwrap();
    let initial = hub.stats().unwrap().event_count;
    let mut close = finish(&opened.session_id, "bad-finish");
    close.notes = vec![
        note(RecordKind::Decision, "Must roll back"),
        note(RecordKind::Verification, "Missing effect link"),
    ];
    assert!(hub.finish(&close).is_err());
    assert_eq!(hub.stats().unwrap().event_count, initial);
    assert_eq!(hub.stats().unwrap().record_count, 0);
    close.notes = vec![note(RecordKind::Effect, "Unverified effect")];
    assert!(hub.finish(&close).is_err());
    assert_eq!(hub.stats().unwrap().record_count, 0);
    close.notes = vec![note(
        RecordKind::MaterialUnknown,
        "Unresolved material question",
    )];
    assert!(hub.finish(&close).is_err());
    assert_eq!(hub.stats().unwrap().record_count, 0);
    let resume = hub
        .resume(&ResumeRequest {
            workspace: request.workspace.clone(),
        })
        .unwrap();
    assert_ne!(resume.status, ResumeStatus::None);
    assert_eq!(resume.selected.unwrap().source_id, opened.session_id);
    close.disposition = CloseDisposition::Handoff;
    close.next_action = Some("Resolve material question".into());
    assert!(hub.finish(&close).is_ok());
    assert!(hub.audit_core_events().unwrap().covered_consistent);
}
#[test]
fn bounded_json_escaping_and_recovery_preserve_omission_disclosure() {
    let area = tempfile::tempdir().unwrap();
    let hub = Hub::open(area.path().join("state.sqlite")).unwrap();
    let old = hub
        .open_session(&OpenRequest {
            workspace: area.path().to_string_lossy().into_owned(),
            objective: "legacy".into(),
            idempotency_key: "legacy".into(),
        })
        .unwrap();
    for i in 0..25 {
        hub.record(&RecordRequest {
            session_id: old.session_id.clone(),
            kind: RecordKind::Constraint,
            content: format!("{i}{}", "\"\\\n".repeat(300)),
            evidence: None,
            supersedes_record_id: None,
            verifies_effect_id: None,
            idempotency_key: format!("record-{i}"),
        })
        .unwrap();
    }
    let opened = hub.begin(&begin(area.path(), "bounded")).unwrap();
    assert!(response_bytes(&opened).unwrap() <= MAX_RESPONSE_BYTES);
    assert!(opened.context.budget.omitted_items > 0);
    assert_eq!(opened.recovery[0].session_id, old.session_id);
    assert_eq!(opened.recovery[0].status, "open_unfinished");
    assert!(opened.context.recent_records.is_empty());
    let mut close = finish(&opened.session_id, "too-many");
    close.notes = vec![note(RecordKind::Observation, "x"); 9];
    assert!(hub.finish(&close).is_err());
    close.notes = vec![note(RecordKind::Observation, &"x".repeat(2049))];
    assert!(hub.finish(&close).is_err());
}

#[test]
fn injected_append_failure_rolls_back_session_and_records() {
    let area = tempfile::tempdir().unwrap();
    let db = area.path().join("state.sqlite");
    let hub = Hub::open(&db).unwrap();
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_begin BEFORE INSERT ON events WHEN NEW.kind='flow_begun' BEGIN SELECT RAISE(ABORT,'injected begin append failure'); END;").unwrap();
    let request = begin(area.path(), "fault-begin");
    assert!(hub.begin(&request).is_err());
    assert_eq!(hub.stats().unwrap().event_count, 0);
    assert_eq!(hub.stats().unwrap().open_session_count, 0);
    conn.execute_batch("DROP TRIGGER fail_begin;").unwrap();
    let opened = hub.begin(&request).unwrap();
    let count = hub.stats().unwrap().event_count;
    conn.execute_batch("CREATE TRIGGER fail_finish BEFORE INSERT ON events WHEN NEW.kind='flow_finished' BEGIN SELECT RAISE(ABORT,'injected finish append failure'); END;").unwrap();
    let mut close = finish(&opened.session_id, "fault-finish");
    close
        .notes
        .push(note(RecordKind::Decision, "Roll back even after close"));
    assert!(hub.finish(&close).is_err());
    assert_eq!(hub.stats().unwrap().event_count, count);
    assert_eq!(hub.stats().unwrap().record_count, 0);
    assert_eq!(hub.stats().unwrap().open_session_count, 1);
    conn.execute_batch("DROP TRIGGER fail_finish;").unwrap();
    assert!(hub.finish(&close).is_ok());
}
