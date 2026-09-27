use aporic::{
    Hub,
    domain::{
        EvidenceGrade, EvidenceKind, EvidenceRequest, IntakeCreateRequest, IntakeGetRequest,
        OpenRequest, TaskCancelRequest, TaskCreateRequest, TaskStatus,
    },
};

#[test]
fn intake_links_source_evidence_and_live_task_status_across_restart() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.clone(),
            objective: "Triage a reproduced bug".into(),
            idempotency_key: "open".into(),
        })
        .unwrap()
        .session_id;
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id: session_id.clone(),
            objective: "Fix reproduced bug".into(),
            acceptance_criteria: vec!["Bug no longer reproduces".into()],
            write_scope: vec![],
            depends_on: vec![],
            idempotency_key: "task".into(),
        })
        .unwrap()
        .task
        .task_id;
    let repro = workspace.clone() + "/repro.txt";
    std::fs::write(&repro, "steps and observed result").unwrap();
    let evidence_id = hub
        .add_evidence(&EvidenceRequest {
            session_id: session_id.clone(),
            kind: EvidenceKind::WorkspaceFile,
            locator: repro,
            summary: "Reproduction steps".into(),
            content_sha256: None,
            idempotency_key: "evidence".into(),
        })
        .unwrap()
        .evidence
        .evidence_id;
    let request = IntakeCreateRequest {
        session_id: session_id.clone(),
        source_project: "upstream/widget".into(),
        reproduction_evidence_id: evidence_id.clone(),
        task_id: task_id.clone(),
        idempotency_key: "intake".into(),
    };
    let first = hub.create_intake(&request).unwrap();
    assert_eq!(first.intake.evidence_grade, EvidenceGrade::Direct);
    assert_eq!(first.intake.evidence_kind, EvidenceKind::WorkspaceFile);
    assert_eq!(first.intake.evidence_summary, "Reproduction steps");
    assert_eq!(first.intake.task_status, TaskStatus::Queued);
    assert_eq!(first.intake.source_project, "upstream/widget");
    assert!(hub.create_intake(&request).unwrap().duplicate);
    assert!(
        hub.create_intake(&IntakeCreateRequest {
            source_project: "different".into(),
            ..request.clone()
        })
        .is_err()
    );
    assert!(
        hub.create_intake(&IntakeCreateRequest {
            reproduction_evidence_id: "missing".into(),
            idempotency_key: "missing".into(),
            ..request.clone()
        })
        .is_err()
    );
    hub.cancel_task(&TaskCancelRequest {
        task_id,
        reason: "Superseded".into(),
        idempotency_key: "cancel".into(),
    })
    .unwrap();
    drop(hub);
    let hub = Hub::open(&database).unwrap();
    let export = hub.export_project(&workspace).unwrap();
    assert_eq!(export.intakes.len(), 1);
    assert!(
        export
            .events
            .iter()
            .any(|event| event.kind == "intake_created")
    );
    let read = hub
        .get_intake(&IntakeGetRequest {
            workspace: workspace.clone(),
            intake_id: first.intake.intake_id.clone(),
        })
        .unwrap();
    assert_eq!(read.task_status, TaskStatus::Cancelled);
    assert_eq!(read.reproduction_evidence_id, evidence_id);
    assert_eq!(
        hub.create_intake(&request).unwrap().intake.task_status,
        TaskStatus::Cancelled
    );
    let other = area.path().join("other");
    std::fs::create_dir(&other).unwrap();
    hub.open_session(&OpenRequest {
        workspace: other.to_string_lossy().into_owned(),
        objective: "Other".into(),
        idempotency_key: "other".into(),
    })
    .unwrap();
    assert!(
        hub.get_intake(&IntakeGetRequest {
            workspace: other.to_string_lossy().into_owned(),
            intake_id: read.intake_id
        })
        .is_err()
    );
    let other_session = hub
        .open_session(&OpenRequest {
            workspace: other.to_string_lossy().into_owned(),
            objective: "Other".into(),
            idempotency_key: "other".into(),
        })
        .unwrap()
        .session_id;
    let foreign_task = hub
        .create_task(&TaskCreateRequest {
            session_id: other_session.clone(),
            objective: "Foreign task".into(),
            acceptance_criteria: vec!["Done".into()],
            write_scope: vec![],
            depends_on: vec![],
            idempotency_key: "foreign-task".into(),
        })
        .unwrap()
        .task
        .task_id;
    assert!(
        hub.create_intake(&IntakeCreateRequest {
            task_id: foreign_task,
            idempotency_key: "foreign-task-intake".into(),
            ..request.clone()
        })
        .is_err()
    );
    let foreign_file = other.join("repro.txt");
    std::fs::write(&foreign_file, "foreign reproduction").unwrap();
    let foreign_evidence = hub
        .add_evidence(&EvidenceRequest {
            session_id: other_session,
            kind: EvidenceKind::WorkspaceFile,
            locator: foreign_file.to_string_lossy().into_owned(),
            summary: "Foreign reproduction".into(),
            content_sha256: None,
            idempotency_key: "foreign-evidence".into(),
        })
        .unwrap()
        .evidence
        .evidence_id;
    assert!(
        hub.create_intake(&IntakeCreateRequest {
            reproduction_evidence_id: foreign_evidence,
            idempotency_key: "foreign-evidence-intake".into(),
            ..request.clone()
        })
        .is_err()
    );
    let statement = hub
        .add_evidence(&EvidenceRequest {
            session_id,
            kind: EvidenceKind::UserStatement,
            locator: "user report".into(),
            summary: "The user reported a bug".into(),
            content_sha256: Some("a".repeat(64)),
            idempotency_key: "statement".into(),
        })
        .unwrap()
        .evidence
        .evidence_id;
    assert!(
        hub.create_intake(&IntakeCreateRequest {
            reproduction_evidence_id: statement,
            idempotency_key: "statement-intake".into(),
            ..request
        })
        .is_err()
    );
}
