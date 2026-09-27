use aporic::{
    Hub,
    domain::{
        CloseDisposition, CloseRequest, OpenRequest, ResumeRequest, ResumeStatus, TaskCreateRequest,
    },
};

fn setup() -> (tempfile::TempDir, String, Hub) {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(database).unwrap();
    (area, workspace.to_string_lossy().into_owned(), hub)
}

fn open(hub: &Hub, workspace: &str, objective: &str, key: &str) -> String {
    hub.open_session(&OpenRequest {
        workspace: workspace.to_owned(),
        objective: objective.to_owned(),
        idempotency_key: key.to_owned(),
    })
    .unwrap()
    .session_id
}

#[test]
fn resume_prefers_one_unfinished_task_then_detects_ambiguity() {
    let (_area, workspace, hub) = setup();
    assert_eq!(
        hub.resume(&ResumeRequest {
            workspace: workspace.clone()
        })
        .unwrap()
        .status,
        ResumeStatus::None
    );
    let session = open(&hub, &workspace, "Product iteration", "open");
    let first = hub
        .create_task(&TaskCreateRequest {
            session_id: session.clone(),
            objective: "Implement role catalog".to_owned(),
            acceptance_criteria: vec!["Role catalog is readable".to_owned()],
            write_scope: vec!["crates/aporic".to_owned()],
            depends_on: vec![],
            idempotency_key: "task-1".to_owned(),
        })
        .unwrap();
    let brief = hub
        .resume(&ResumeRequest {
            workspace: workspace.clone(),
        })
        .unwrap();
    assert_eq!(brief.status, ResumeStatus::Ready);
    assert_eq!(brief.selected.unwrap().source_id, first.task.task_id);
    hub.create_task(&TaskCreateRequest {
        session_id: session,
        objective: "Write continuation guide".to_owned(),
        acceptance_criteria: vec!["Guide exists".to_owned()],
        write_scope: vec!["docs".to_owned()],
        depends_on: vec![],
        idempotency_key: "task-2".to_owned(),
    })
    .unwrap();
    let brief = hub.resume(&ResumeRequest { workspace }).unwrap();
    assert_eq!(brief.status, ResumeStatus::Ambiguous);
    assert!(brief.selected.is_none());
}

#[test]
fn resume_uses_fresh_handoff_and_ignores_finished_history() {
    let (_area, workspace, hub) = setup();
    let first = open(&hub, &workspace, "Design roles", "first");
    hub.close_session(&CloseRequest {
        session_id: first,
        disposition: CloseDisposition::Handoff,
        summary: "Role design is ready".to_owned(),
        next_action: Some("Implement appointments".to_owned()),
        idempotency_key: "handoff".to_owned(),
    })
    .unwrap();
    let brief = hub
        .resume(&ResumeRequest {
            workspace: workspace.clone(),
        })
        .unwrap();
    assert_eq!(brief.status, ResumeStatus::Ready);
    assert_eq!(
        brief.selected.unwrap().next_action,
        "Implement appointments"
    );
    let next = open(&hub, &workspace, "Implement appointments", "second");
    hub.close_session(&CloseRequest {
        session_id: next,
        disposition: CloseDisposition::Completed,
        summary: "Appointments complete".to_owned(),
        next_action: None,
        idempotency_key: "completed".to_owned(),
    })
    .unwrap();
    assert_eq!(
        hub.resume(&ResumeRequest { workspace }).unwrap().status,
        ResumeStatus::None
    );
}

#[test]
fn export_keeps_retired_feature_events_as_history() {
    let (area, workspace, hub) = setup();
    let session_id = open(&hub, &workspace, "Continue historical work", "open");
    let database = area.path().join("aporic.sqlite3");
    let connection = rusqlite::Connection::open(&database).unwrap();
    let project_id: String = connection
        .query_row(
            "SELECT project_id FROM sessions WHERE session_id = ?1",
            [&session_id],
            |row| row.get(0),
        )
        .unwrap();
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id: session_id.clone(),
            objective: "Historical prompt run".to_owned(),
            acceptance_criteria: vec!["Retain the event".to_owned()],
            write_scope: vec!["history".to_owned()],
            depends_on: vec![],
            idempotency_key: "historical-prompt-task".to_owned(),
        })
        .unwrap()
        .task
        .task_id;
    connection
        .execute(
            "INSERT INTO role_appointments (
            appointment_id, project_id, session_id, task_id, role_id, role_version,
            assignee_id, model_hint, capability_refs_json, appointment_sha256,
            created_at_unix_ms, revoked_at_unix_ms
         ) VALUES (?1, ?2, ?3, NULL, 'portfolio.steward', 1, 'legacy-agent',
                   NULL, '[]', 'legacy-role-digest', 1, NULL)",
            rusqlite::params!["legacy-role", project_id, session_id],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO government_people (
            person_id, project_id, full_name, person_sha256, created_at_unix_ms
         ) VALUES ('legacy-person', ?1, 'Legacy Person', 'legacy-person-digest', 1)",
            [&project_id],
        )
        .unwrap();
    for (event_id, key, stream_id, kind) in [
        (
            "legacy-role-event",
            "legacy-role-event-key",
            "legacy-role",
            "role_appointment_created",
        ),
        (
            "legacy-government-event",
            "legacy-government-event-key",
            "legacy-person",
            "government_person_registered",
        ),
        (
            "legacy-capability-event",
            "legacy-capability-event-key",
            session_id.as_str(),
            "capability_registered",
        ),
        (
            "legacy-prompt-trial-event",
            "legacy-prompt-trial-event-key",
            task_id.as_str(),
            "prompt_trial_recorded",
        ),
    ] {
        connection
            .execute(
                "INSERT INTO events (
                event_id, idempotency_key, stream_id, kind, payload_json, result_json,
                occurred_at_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, '{\"historical\":true}', '{\"preserved\":true}', 1)",
                rusqlite::params![event_id, key, stream_id, kind],
            )
            .unwrap();
    }
    connection
        .execute(
            "INSERT INTO events (
                event_id, idempotency_key, stream_id, kind, payload_json, result_json,
                occurred_at_unix_ms
             ) VALUES ('legacy-improvement-event', 'legacy-improvement-event-key',
                'legacy-improvement', 'improvement_submitted',
                ?1, '{\"preserved\":true}', 1)",
            [serde_json::json!({"source_session_id": session_id}).to_string()],
        )
        .unwrap();
    drop(connection);

    let exported = hub.export_project(&workspace).unwrap();
    assert!(
        exported
            .events
            .iter()
            .any(|event| event.kind == "role_appointment_created"
                && event.result["preserved"] == true)
    );
    assert!(
        exported
            .events
            .iter()
            .any(|event| event.kind == "government_person_registered"
                && event.payload["historical"] == true)
    );
    for kind in [
        "capability_registered",
        "prompt_trial_recorded",
        "improvement_submitted",
    ] {
        assert!(
            exported.events.iter().any(|event| event.kind == kind),
            "missing retired event {kind}"
        );
    }
}
