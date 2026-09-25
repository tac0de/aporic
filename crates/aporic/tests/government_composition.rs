use aporic::{
    Hub,
    domain::{
        GovernmentWorkspaceRequest, OfficeAppointmentCreateRequest, OpenRequest,
        ProductCellCreateRequest, ProductCellDuty, ProductCellMemberRequest,
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

fn open(hub: &Hub, workspace: &str) -> String {
    hub.open_session(&OpenRequest {
        workspace: workspace.to_owned(),
        objective: "Test the product experiment ministry".to_owned(),
        idempotency_key: "open".to_owned(),
    })
    .unwrap()
    .session_id
}

fn appoint(
    hub: &Hub,
    session_id: &str,
    task_id: Option<&str>,
    role_id: &str,
    assignee_id: &str,
    key: &str,
) -> String {
    hub.create_role_appointment(&RoleAppointmentCreateRequest {
        session_id: session_id.to_owned(),
        task_id: task_id.map(str::to_owned),
        role_id: role_id.to_owned(),
        role_version: 1,
        assignee_id: assignee_id.to_owned(),
        model_hint: None,
        capability_refs: vec![],
        idempotency_key: key.to_owned(),
    })
    .unwrap()
    .appointment
    .appointment_id
}

fn task(hub: &Hub, session_id: &str) -> String {
    hub.create_task(&TaskCreateRequest {
        session_id: session_id.to_owned(),
        objective: "Test a product hypothesis".to_owned(),
        acceptance_criteria: vec!["Prototype evidence exists".to_owned()],
        write_scope: vec!["crates".to_owned()],
        depends_on: vec![],
        idempotency_key: "task".to_owned(),
    })
    .unwrap()
    .task
    .task_id
}

#[test]
fn government_charter_is_advisory_and_product_focused() {
    let (_area, _workspace, hub) = setup();
    let government = hub.government_definition();
    assert_eq!(government.government_id, "aporic.government");
    assert_eq!(government.authority_source, "현재 인간 지시");
    assert!(government.advisory);
    assert!(!government.grants_authority);
    assert_eq!(government.offices.len(), 1);
    let office = &government.offices[0];
    assert_eq!(office.office_id, "product.experiment");
    assert_eq!(office.title, "제품실험부");
    assert_eq!(office.head_role_id, "portfolio.steward");
    assert!(office.cell_based);
    assert!(!office.grants_authority);
}

#[test]
fn product_minister_and_multidisciplinary_cell_preserve_independent_oversight() {
    let (area, workspace, hub) = setup();
    let session_id = open(&hub, &workspace);
    let task_id = task(&hub, &session_id);
    let steward_id = appoint(
        &hub,
        &session_id,
        None,
        "portfolio.steward",
        "agent.minister",
        "minister-role",
    );
    let office = hub
        .create_office_appointment(&OfficeAppointmentCreateRequest {
            session_id: session_id.clone(),
            office_id: "product.experiment".to_owned(),
            office_version: 1,
            role_appointment_id: steward_id,
            idempotency_key: "office".to_owned(),
        })
        .unwrap();
    assert!(office.appointment.advisory);
    assert!(!office.appointment.grants_authority);
    let second_steward = appoint(
        &hub,
        &session_id,
        None,
        "portfolio.steward",
        "agent.second-minister",
        "second-minister-role",
    );
    assert!(
        hub.create_office_appointment(&OfficeAppointmentCreateRequest {
            session_id: session_id.clone(),
            office_id: "product.experiment".to_owned(),
            office_version: 1,
            role_appointment_id: second_steward,
            idempotency_key: "duplicate-office-head".to_owned(),
        })
        .is_err()
    );
    assert!(
        hub.create_role_appointment(&RoleAppointmentCreateRequest {
            session_id: session_id.clone(),
            task_id: Some(task_id.clone()),
            role_id: "oversight.inspector".to_owned(),
            role_version: 1,
            assignee_id: "agent.minister".to_owned(),
            model_hint: None,
            capability_refs: vec![],
            idempotency_key: "conflicted-inspector".to_owned(),
        })
        .is_err()
    );
    let planner = appoint(
        &hub,
        &session_id,
        Some(&task_id),
        "delivery.worker",
        "agent.planner",
        "planner",
    );
    let builder = appoint(
        &hub,
        &session_id,
        Some(&task_id),
        "delivery.worker",
        "agent.builder",
        "builder",
    );
    let interaction_designer = appoint(
        &hub,
        &session_id,
        Some(&task_id),
        "delivery.worker",
        "agent.interaction-designer",
        "interaction-designer",
    );
    let visual_designer = appoint(
        &hub,
        &session_id,
        Some(&task_id),
        "delivery.worker",
        "agent.visual-designer",
        "visual-designer",
    );
    let motion_designer = appoint(
        &hub,
        &session_id,
        Some(&task_id),
        "delivery.worker",
        "agent.motion-designer",
        "motion-designer",
    );
    let backend_engineer = appoint(
        &hub,
        &session_id,
        Some(&task_id),
        "delivery.worker",
        "agent.backend-engineer",
        "backend-engineer",
    );
    let game_developer = appoint(
        &hub,
        &session_id,
        Some(&task_id),
        "delivery.worker",
        "agent.game-developer",
        "game-developer",
    );
    let level_designer = appoint(
        &hub,
        &session_id,
        Some(&task_id),
        "delivery.worker",
        "agent.level-designer",
        "level-designer",
    );
    appoint(
        &hub,
        &session_id,
        Some(&task_id),
        "oversight.inspector",
        "agent.inspector",
        "inspector",
    );
    let request = ProductCellCreateRequest {
        session_id: session_id.clone(),
        task_id: task_id.clone(),
        office_appointment_id: office.appointment.office_appointment_id.clone(),
        title: "첫 제품반".to_owned(),
        problem_statement: "사용자가 근거 없는 제품 결정을 피해야 한다.".to_owned(),
        hypothesis: "작은 프로토타입이 결정 불확실성을 낮춘다.".to_owned(),
        success_measures: vec!["직접 증거가 하나 이상 기록된다.".to_owned()],
        members: vec![
            ProductCellMemberRequest {
                role_appointment_id: planner,
                duty: ProductCellDuty::ProductPlanning,
            },
            ProductCellMemberRequest {
                role_appointment_id: builder,
                duty: ProductCellDuty::PrototypeDelivery,
            },
            ProductCellMemberRequest {
                role_appointment_id: interaction_designer,
                duty: ProductCellDuty::InteractionDesign,
            },
            ProductCellMemberRequest {
                role_appointment_id: visual_designer,
                duty: ProductCellDuty::VisualDesign,
            },
            ProductCellMemberRequest {
                role_appointment_id: motion_designer,
                duty: ProductCellDuty::MotionDesign,
            },
            ProductCellMemberRequest {
                role_appointment_id: backend_engineer,
                duty: ProductCellDuty::BackendEngineering,
            },
            ProductCellMemberRequest {
                role_appointment_id: game_developer,
                duty: ProductCellDuty::GameDevelopment,
            },
            ProductCellMemberRequest {
                role_appointment_id: level_designer,
                duty: ProductCellDuty::LevelDesign,
            },
        ],
        idempotency_key: "cell".to_owned(),
    };
    let created = hub.create_product_cell(&request).unwrap();
    assert_eq!(created.cell.members.len(), 8);
    assert!(created.cell.active);
    assert!(created.cell.advisory);
    assert!(!created.cell.grants_authority);
    assert!(hub.create_product_cell(&request).unwrap().duplicate);
    assert!(
        hub.create_product_cell(&ProductCellCreateRequest {
            idempotency_key: "second-cell".to_owned(),
            ..request
        })
        .is_err()
    );

    drop(hub);
    let restarted = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    assert_eq!(restarted.stats().unwrap().schema_version, 18);
    assert_eq!(
        restarted
            .list_office_appointments(&GovernmentWorkspaceRequest {
                workspace: workspace.clone(),
                limit: None,
            })
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        restarted
            .list_product_cells(&GovernmentWorkspaceRequest {
                workspace: workspace.clone(),
                limit: None,
            })
            .unwrap()
            .len(),
        1
    );
    let export = restarted.export_project(&workspace).unwrap();
    assert_eq!(export.format_version, 14);
    assert_eq!(export.office_appointments.len(), 1);
    assert_eq!(export.product_cells.len(), 1);
    assert_eq!(export.product_cells[0].members.len(), 8);
    assert!(
        export.product_cells[0]
            .members
            .iter()
            .any(|member| member.duty == ProductCellDuty::MotionDesign)
    );
    assert!(
        export.product_cells[0]
            .members
            .iter()
            .any(|member| member.duty == ProductCellDuty::BackendEngineering)
    );
    assert!(
        export.product_cells[0]
            .members
            .iter()
            .any(|member| member.duty == ProductCellDuty::GameDevelopment)
    );
    assert!(
        export.product_cells[0]
            .members
            .iter()
            .any(|member| member.duty == ProductCellDuty::LevelDesign)
    );
    assert!(restarted.audit_government().unwrap().consistent);
}

#[test]
fn product_cell_rejects_functional_silos_and_tampering() {
    let (area, workspace, hub) = setup();
    let session_id = open(&hub, &workspace);
    let task_id = task(&hub, &session_id);
    let steward_id = appoint(
        &hub,
        &session_id,
        None,
        "portfolio.steward",
        "agent.minister",
        "minister-role",
    );
    let office_id = hub
        .create_office_appointment(&OfficeAppointmentCreateRequest {
            session_id: session_id.clone(),
            office_id: "product.experiment".to_owned(),
            office_version: 1,
            role_appointment_id: steward_id,
            idempotency_key: "office".to_owned(),
        })
        .unwrap()
        .appointment
        .office_appointment_id;
    let one = appoint(
        &hub,
        &session_id,
        Some(&task_id),
        "delivery.worker",
        "agent.same",
        "worker-one",
    );
    let two = appoint(
        &hub,
        &session_id,
        Some(&task_id),
        "delivery.worker",
        "agent.same",
        "worker-two",
    );
    let base = ProductCellCreateRequest {
        session_id,
        task_id,
        office_appointment_id: office_id,
        title: "제품반".to_owned(),
        problem_statement: "문제".to_owned(),
        hypothesis: "가설".to_owned(),
        success_measures: vec!["척도".to_owned()],
        members: vec![
            ProductCellMemberRequest {
                role_appointment_id: one,
                duty: ProductCellDuty::ProductPlanning,
            },
            ProductCellMemberRequest {
                role_appointment_id: two,
                duty: ProductCellDuty::PrototypeDelivery,
            },
        ],
        idempotency_key: "same-person-cell".to_owned(),
    };
    assert!(hub.create_product_cell(&base).is_err());

    let connection = rusqlite::Connection::open(area.path().join("aporic.sqlite3")).unwrap();
    connection
        .execute(
            "UPDATE office_appointments SET assignee_id = 'tampered'",
            [],
        )
        .unwrap();
    assert!(!hub.audit_government().unwrap().consistent);
    assert!(
        hub.list_office_appointments(&GovernmentWorkspaceRequest {
            workspace,
            limit: None,
        })
        .is_err()
    );
}

#[test]
fn upgrades_schema_16_to_product_government() {
    let (area, _workspace, hub) = setup();
    drop(hub);
    let database = area.path().join("aporic.sqlite3");
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute_batch(
            "DROP TRIGGER research_revisions_fts_insert;
             DROP TABLE research_fts;
             DROP TABLE research_revisions;
             DROP TABLE research_documents;
             DROP TABLE product_cells;
             DROP TABLE office_appointments;
             PRAGMA user_version = 16;",
        )
        .unwrap();
    drop(connection);
    let upgraded = Hub::open(database).unwrap();
    assert_eq!(upgraded.stats().unwrap().schema_version, 18);
    assert!(upgraded.audit_government().unwrap().consistent);
}
