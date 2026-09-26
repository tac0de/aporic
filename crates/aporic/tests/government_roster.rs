use aporic::{
    Hub,
    domain::{
        GovernmentBootstrapRequest, GovernmentPersonRegisterRequest, GovernmentTermAppointRequest,
        GovernmentTermEndRequest, GovernmentWorkspaceRequest, OfficeAppointmentCreateRequest,
        OpenRequest, ProductCellCreateRequest, ProductCellDuty, ProductCellMemberRequest,
        RoleAppointmentCreateRequest, TaskCreateRequest,
    },
};

fn setup() -> (tempfile::TempDir, String, Hub) {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    (area, workspace.to_string_lossy().into_owned(), hub)
}

fn appoint_role(
    hub: &Hub,
    session_id: &str,
    task_id: Option<&str>,
    role: &str,
    person: &str,
    key: &str,
) -> String {
    hub.create_role_appointment(&RoleAppointmentCreateRequest {
        session_id: session_id.to_owned(),
        task_id: task_id.map(str::to_owned),
        role_id: role.to_owned(),
        role_version: 1,
        assignee_id: person.to_owned(),
        model_hint: None,
        capability_refs: vec![],
        idempotency_key: key.to_owned(),
    })
    .unwrap()
    .appointment
    .appointment_id
}

#[test]
fn named_terms_bind_product_minister_and_product_cell_leads() {
    let (_area, workspace, hub) = setup();
    let session_id = open(&hub, &workspace, "open");
    let roster = hub
        .bootstrap_government(&GovernmentBootstrapRequest {
            session_id: session_id.clone(),
            idempotency_key: "bootstrap".to_owned(),
        })
        .unwrap()
        .roster;
    let person_for = |position: &str| {
        roster
            .terms
            .iter()
            .find(|term| term.position_id == position)
            .unwrap()
            .person_id
            .clone()
    };
    let minister = person_for("product.experiment.minister");
    let designer = person_for("product.design.lead");
    let frontend = person_for("product.frontend.lead");
    let wrong = appoint_role(
        &hub,
        &session_id,
        None,
        "portfolio.steward",
        "outsider",
        "wrong-minister-role",
    );
    assert!(
        hub.create_office_appointment(&OfficeAppointmentCreateRequest {
            session_id: session_id.clone(),
            office_id: "product.experiment".to_owned(),
            office_version: 1,
            role_appointment_id: wrong,
            idempotency_key: "wrong-minister".to_owned(),
        })
        .is_err()
    );
    let correct = appoint_role(
        &hub,
        &session_id,
        None,
        "portfolio.steward",
        &minister,
        "minister-role",
    );
    let office = hub
        .create_office_appointment(&OfficeAppointmentCreateRequest {
            session_id: session_id.clone(),
            office_id: "product.experiment".to_owned(),
            office_version: 1,
            role_appointment_id: correct,
            idempotency_key: "minister".to_owned(),
        })
        .unwrap()
        .appointment;
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id: session_id.clone(),
            objective: "Prototype a screen".to_owned(),
            acceptance_criteria: vec!["Prototype is reviewed".to_owned()],
            write_scope: vec!["crates".to_owned()],
            depends_on: vec![],
            idempotency_key: "task".to_owned(),
        })
        .unwrap()
        .task
        .task_id;
    let design_role = appoint_role(
        &hub,
        &session_id,
        Some(&task_id),
        "delivery.worker",
        &designer,
        "design-role",
    );
    let frontend_role = appoint_role(
        &hub,
        &session_id,
        Some(&task_id),
        "delivery.worker",
        &frontend,
        "frontend-role",
    );
    let outsider_role = appoint_role(
        &hub,
        &session_id,
        Some(&task_id),
        "delivery.worker",
        "outsider",
        "outsider-role",
    );
    let mut request = ProductCellCreateRequest {
        session_id: session_id.clone(),
        task_id,
        office_appointment_id: office.office_appointment_id,
        title: "화면 제품반".to_owned(),
        problem_statement: "화면 흐름을 검증한다".to_owned(),
        hypothesis: "새 흐름이 탐색을 쉽게 한다".to_owned(),
        success_measures: vec!["관찰 결과를 기록한다".to_owned()],
        members: vec![
            ProductCellMemberRequest {
                role_appointment_id: design_role,
                duty: ProductCellDuty::ProductPlanning,
            },
            ProductCellMemberRequest {
                role_appointment_id: outsider_role,
                duty: ProductCellDuty::PrototypeDelivery,
            },
        ],
        idempotency_key: "cell".to_owned(),
    };
    assert!(hub.create_product_cell(&request).is_err());
    request.members[1] = ProductCellMemberRequest {
        role_appointment_id: frontend_role,
        duty: ProductCellDuty::PrototypeDelivery,
    };
    let cell = hub.create_product_cell(&request).unwrap().cell;
    assert_eq!(cell.members.len(), 2);
    assert_eq!(cell.members[0].assignee_id, designer);
    assert_eq!(cell.members[1].assignee_id, frontend);
}

fn open(hub: &Hub, workspace: &str, key: &str) -> String {
    hub.open_session(&OpenRequest {
        workspace: workspace.to_owned(),
        objective: "Initialize an advisory government roster".to_owned(),
        idempotency_key: key.to_owned(),
    })
    .unwrap()
    .session_id
}

#[test]
fn initial_cabinet_bootstraps_once_and_survives_restart_and_export() {
    let (area, workspace, hub) = setup();
    let session_id = open(&hub, &workspace, "open");
    let request = GovernmentBootstrapRequest {
        session_id: session_id.clone(),
        idempotency_key: "bootstrap".to_owned(),
    };
    let first = hub.bootstrap_government(&request).unwrap();
    assert!(!first.duplicate);
    assert!(first.roster.initialized);
    assert_eq!(first.roster.people.len(), 9);
    assert_eq!(first.roster.terms.len(), 9);
    assert_eq!(first.roster.current.len(), 9);
    assert!(
        first
            .roster
            .terms
            .iter()
            .all(|term| term.ended_at_unix_ms.is_none() && term.advisory && !term.grants_authority)
    );
    assert!(hub.bootstrap_government(&request).unwrap().duplicate);
    assert!(
        hub.bootstrap_government(&GovernmentBootstrapRequest {
            idempotency_key: "bootstrap-again".to_owned(),
            ..request.clone()
        })
        .is_err()
    );
    let names = first
        .roster
        .people
        .iter()
        .map(|person| person.full_name.as_str())
        .collect::<Vec<_>>();
    assert!(names.contains(&"김민준"));
    assert!(names.contains(&"한예진"));
    assert_eq!(hub.government_definition().offices.len(), 3);
    assert_eq!(hub.government_definition().positions.len(), 9);

    drop(hub);
    let restarted = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    assert_eq!(restarted.stats().unwrap().schema_version, 25);
    let roster = restarted
        .government_roster(&GovernmentWorkspaceRequest {
            workspace: workspace.clone(),
            limit: Some(50),
        })
        .unwrap();
    assert_eq!(roster.people.len(), 9);
    assert_eq!(roster.terms.len(), 9);
    assert_eq!(roster.current.len(), 9);
    let bounded = restarted
        .government_roster(&GovernmentWorkspaceRequest {
            workspace: workspace.clone(),
            limit: Some(1),
        })
        .unwrap();
    assert_eq!(bounded.current.len(), 9);
    assert_eq!(bounded.people.len(), 1);
    assert_eq!(bounded.terms.len(), 1);
    let export = restarted.export_project(&workspace).unwrap();
    assert_eq!(export.government_people.len(), 9);
    assert_eq!(export.government_terms.len(), 9);
    assert!(
        export
            .events
            .iter()
            .any(|event| event.kind == "government_roster_bootstrapped")
    );
    assert!(restarted.audit_government().unwrap().consistent);
}

#[test]
fn successor_requires_retirement_and_handoff_and_preserves_history() {
    let (_area, workspace, hub) = setup();
    let session_id = open(&hub, &workspace, "open");
    hub.bootstrap_government(&GovernmentBootstrapRequest {
        session_id: session_id.clone(),
        idempotency_key: "bootstrap".to_owned(),
    })
    .unwrap();
    let roster = hub
        .government_roster(&GovernmentWorkspaceRequest {
            workspace: workspace.clone(),
            limit: None,
        })
        .unwrap();
    let former = roster
        .terms
        .iter()
        .find(|term| term.position_id == "product.experiment.minister")
        .unwrap();
    let former_id = former.term_id.clone();
    let successor = hub
        .register_government_person(&GovernmentPersonRegisterRequest {
            session_id: session_id.clone(),
            full_name: "김서현".to_owned(),
            idempotency_key: "register-successor".to_owned(),
        })
        .unwrap()
        .person;
    let appointment = GovernmentTermAppointRequest {
        session_id: session_id.clone(),
        person_id: successor.person_id.clone(),
        position_id: "product.experiment.minister".to_owned(),
        handoff_note: Some("진행 중인 제품 의제와 검증 근거를 인계".to_owned()),
        idempotency_key: "appoint-successor".to_owned(),
    };
    assert!(hub.appoint_government_term(&appointment).is_err());
    let ended = hub
        .end_government_term(&GovernmentTermEndRequest {
            session_id: session_id.clone(),
            term_id: former_id.clone(),
            reason: "교체".to_owned(),
            idempotency_key: "end-former".to_owned(),
        })
        .unwrap();
    assert_eq!(ended.term.end_reason.as_deref(), Some("교체"));
    assert!(
        hub.appoint_government_term(&GovernmentTermAppointRequest {
            handoff_note: None,
            ..appointment.clone()
        })
        .is_err()
    );
    let new_term = hub.appoint_government_term(&appointment).unwrap().term;
    assert_eq!(
        new_term.predecessor_term_id.as_deref(),
        Some(former_id.as_str())
    );
    assert!(hub.appoint_government_term(&appointment).unwrap().duplicate);
    assert!(
        hub.appoint_government_term(&GovernmentTermAppointRequest {
            person_id: successor.person_id.clone(),
            position_id: "oversight.inspector".to_owned(),
            idempotency_key: "conflicting-inspector".to_owned(),
            ..appointment
        })
        .is_err()
    );
    let latest = hub
        .government_roster(&GovernmentWorkspaceRequest {
            workspace,
            limit: Some(50),
        })
        .unwrap();
    assert_eq!(latest.terms.len(), 10);
    assert_eq!(latest.current.len(), 9);
    assert_eq!(
        latest
            .terms
            .iter()
            .filter(|term| term.position_id == "product.experiment.minister"
                && term.ended_at_unix_ms.is_none())
            .count(),
        1
    );
    assert!(
        latest
            .terms
            .iter()
            .any(|term| term.term_id == former_id && term.ended_at_unix_ms.is_some())
    );
}

#[test]
fn roster_is_workspace_scoped_and_detects_tampering() {
    let (area, workspace, hub) = setup();
    let second = area.path().join("other");
    std::fs::create_dir(&second).unwrap();
    let second = second.to_string_lossy().into_owned();
    let first_session = open(&hub, &workspace, "open-one");
    let second_session = open(&hub, &second, "open-two");
    let first = hub
        .bootstrap_government(&GovernmentBootstrapRequest {
            session_id: first_session.clone(),
            idempotency_key: "bootstrap-one".to_owned(),
        })
        .unwrap();
    assert!(
        !hub.government_roster(&GovernmentWorkspaceRequest {
            workspace: second.clone(),
            limit: None,
        })
        .unwrap()
        .initialized
    );
    let second_bootstrap = hub
        .bootstrap_government(&GovernmentBootstrapRequest {
            session_id: second_session.clone(),
            idempotency_key: "bootstrap-one".to_owned(),
        })
        .unwrap();
    assert_ne!(
        first.roster.people[0].person_id,
        second_bootstrap.roster.people[0].person_id
    );
    let foreign = hub.appoint_government_term(&GovernmentTermAppointRequest {
        session_id: second_session,
        person_id: first.roster.people[0].person_id.clone(),
        position_id: "product.design.lead".to_owned(),
        handoff_note: None,
        idempotency_key: "foreign".to_owned(),
    });
    assert!(foreign.is_err());
    let db = rusqlite::Connection::open(area.path().join("aporic.sqlite3")).unwrap();
    db.execute(
        "UPDATE government_people SET full_name = '변조' WHERE person_id = ?1",
        [&first.roster.people[0].person_id],
    )
    .unwrap();
    let audit = hub.audit_government().unwrap();
    assert!(!audit.consistent);
    assert!(audit.invalid_count > 0);
}

#[test]
fn schema_21_upgrade_preserves_existing_product_minister_without_auto_bootstrap() {
    let (area, workspace, hub) = setup();
    let session_id = open(&hub, &workspace, "open");
    let steward = appoint_role(
        &hub,
        &session_id,
        None,
        "portfolio.steward",
        "legacy-minister",
        "legacy-role",
    );
    let office = hub
        .create_office_appointment(&OfficeAppointmentCreateRequest {
            session_id: session_id.clone(),
            office_id: "product.experiment".to_owned(),
            office_version: 1,
            role_appointment_id: steward,
            idempotency_key: "legacy-office".to_owned(),
        })
        .unwrap()
        .appointment;
    drop(hub);
    let database = area.path().join("aporic.sqlite3");
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute_batch(
            "DROP TABLE task_research_items;
            DROP TABLE prompt_trials;
            DROP TABLE task_brief_receipts;
            DROP TABLE government_terms;
         DROP TABLE government_roster_state;
         DROP TABLE government_people;
         PRAGMA user_version = 21;",
        )
        .unwrap();
    drop(connection);
    let upgraded = Hub::open(&database).unwrap();
    assert_eq!(upgraded.stats().unwrap().schema_version, 25);
    assert_eq!(
        upgraded
            .list_office_appointments(&GovernmentWorkspaceRequest {
                workspace: workspace.clone(),
                limit: None,
            })
            .unwrap()[0]
            .office_appointment_id,
        office.office_appointment_id
    );
    let roster = upgraded
        .government_roster(&GovernmentWorkspaceRequest {
            workspace,
            limit: None,
        })
        .unwrap();
    assert!(!roster.initialized);
    assert!(roster.current.is_empty());
    assert!(
        upgraded
            .bootstrap_government(&GovernmentBootstrapRequest {
                session_id,
                idempotency_key: "bootstrap-legacy".to_owned(),
            })
            .is_err()
    );
    assert!(upgraded.audit_government().unwrap().consistent);
}
