use aporic::{
    Hub,
    domain::{
        CloseDisposition, CloseRequest, OpenRequest, ResumeRequest, ResumeStatus,
        RoleAppointmentCreateRequest, RoleAppointmentListRequest, RoleAppointmentRevokeRequest,
        RoleCapabilityRef, TaskCreateRequest,
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
fn appointments_are_advisory_and_separate_worker_from_inspector() {
    let (area, workspace, hub) = setup();
    let session = open(&hub, &workspace, "Review implementation", "open");
    let task = hub
        .create_task(&TaskCreateRequest {
            session_id: session.clone(),
            objective: "Implement and inspect".to_owned(),
            acceptance_criteria: vec!["Verified".to_owned()],
            write_scope: vec!["src".to_owned()],
            depends_on: vec![],
            idempotency_key: "task".to_owned(),
        })
        .unwrap();
    assert_eq!(hub.role_definitions().len(), 4);
    let worker = RoleAppointmentCreateRequest {
        session_id: session.clone(),
        task_id: Some(task.task.task_id.clone()),
        role_id: "delivery.worker".to_owned(),
        role_version: 1,
        assignee_id: "agent.sol".to_owned(),
        model_hint: Some("gpt-6-sol".to_owned()),
        capability_refs: vec![],
        idempotency_key: "appoint-worker".to_owned(),
    };
    let appointed = hub.create_role_appointment(&worker).unwrap();
    assert!(!appointed.appointment.grants_authority);
    assert!(hub.create_role_appointment(&worker).unwrap().duplicate);
    let invented_capability = RoleAppointmentCreateRequest {
        idempotency_key: "invented-capability".to_owned(),
        capability_refs: vec![RoleCapabilityRef {
            capability_id: "missing.plugin".to_owned(),
            version: "1".to_owned(),
        }],
        ..worker.clone()
    };
    assert!(hub.create_role_appointment(&invented_capability).is_err());
    let inspector = RoleAppointmentCreateRequest {
        role_id: "oversight.inspector".to_owned(),
        idempotency_key: "appoint-inspector".to_owned(),
        ..worker.clone()
    };
    assert!(hub.create_role_appointment(&inspector).is_err());
    let inspector = RoleAppointmentCreateRequest {
        assignee_id: "agent.astra".to_owned(),
        ..inspector
    };
    hub.create_role_appointment(&inspector).unwrap();
    let revoked = hub
        .revoke_role_appointment(&RoleAppointmentRevokeRequest {
            appointment_id: appointed.appointment.appointment_id,
            idempotency_key: "revoke-worker".to_owned(),
        })
        .unwrap();
    assert!(revoked.appointment.revoked_at_unix_ms.is_some());
    let formerly_worker_as_inspector = RoleAppointmentCreateRequest {
        assignee_id: "agent.sol".to_owned(),
        idempotency_key: "revoked-worker-as-inspector".to_owned(),
        ..inspector.clone()
    };
    assert!(
        hub.create_role_appointment(&formerly_worker_as_inspector)
            .is_err()
    );
    let inspector_appointment = hub
        .list_role_appointments(&RoleAppointmentListRequest {
            workspace: workspace.clone(),
            limit: None,
        })
        .unwrap()
        .into_iter()
        .find(|item| item.role_id == "oversight.inspector")
        .unwrap();
    hub.revoke_role_appointment(&RoleAppointmentRevokeRequest {
        appointment_id: inspector_appointment.appointment_id,
        idempotency_key: "revoke-inspector".to_owned(),
    })
    .unwrap();
    let formerly_inspector_as_worker = RoleAppointmentCreateRequest {
        assignee_id: "agent.astra".to_owned(),
        idempotency_key: "revoked-inspector-as-worker".to_owned(),
        ..worker.clone()
    };
    assert!(
        hub.create_role_appointment(&formerly_inspector_as_worker)
            .is_err()
    );
    drop(hub);
    let restarted = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let listed = restarted
        .list_role_appointments(&RoleAppointmentListRequest {
            workspace: workspace.clone(),
            limit: None,
        })
        .unwrap();
    assert_eq!(listed.len(), 2);
    assert!(listed.iter().all(|item| item.revoked_at_unix_ms.is_some()));
    let exported = restarted.export_project(&workspace).unwrap();
    assert_eq!(exported.role_appointments.len(), 2);
    assert_eq!(
        exported
            .events
            .iter()
            .filter(|event| event.kind.starts_with("role_appointment_"))
            .count(),
        4
    );
    assert!(restarted.audit_role_appointments().unwrap().consistent);
    let connection = rusqlite::Connection::open(area.path().join("aporic.sqlite3")).unwrap();
    connection.execute("UPDATE role_appointments SET assignee_id = 'tampered' WHERE role_id = 'delivery.worker'", []).unwrap();
    assert!(!restarted.audit_role_appointments().unwrap().consistent);
}

#[test]
fn upgrades_existing_role_schema_without_recreating_appointments() {
    let (area, workspace, hub) = setup();
    let session = open(&hub, &workspace, "Keep a role appointment", "open");
    hub.create_role_appointment(&RoleAppointmentCreateRequest {
        session_id: session,
        task_id: None,
        role_id: "portfolio.steward".to_owned(),
        role_version: 1,
        assignee_id: "agent.sol".to_owned(),
        model_hint: None,
        capability_refs: vec![],
        idempotency_key: "appoint".to_owned(),
    })
    .unwrap();
    drop(hub);
    let database = area.path().join("aporic.sqlite3");
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute_batch(
            "DROP TABLE government_terms;
            DROP TABLE government_roster_state;
            DROP TABLE government_people;
            DROP TABLE task_memory_uses;
            DROP TABLE prototype_reviews;
             DROP TABLE prototype_briefs;
             DROP TABLE improvement_requests;
             DROP TABLE accountability_cases;
             DROP TRIGGER research_revisions_fts_insert;
             DROP TABLE research_fts;
             DROP TABLE research_revisions;
             DROP TABLE research_documents;
             DROP TABLE product_cells;
             DROP TABLE office_appointments;
             ALTER TABLE orchestration_runs DROP COLUMN role_appointment_id;
             PRAGMA user_version = 15;",
        )
        .unwrap();
    drop(connection);
    let upgraded = Hub::open(database).unwrap();
    assert_eq!(upgraded.stats().unwrap().schema_version, 22);
    assert_eq!(
        upgraded
            .list_role_appointments(&RoleAppointmentListRequest {
                workspace,
                limit: None
            })
            .unwrap()
            .len(),
        1
    );
}
