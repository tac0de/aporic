use aporic::{
    Hub,
    domain::{
        ClaimRequest, ClaimStatus, Consequence, CriterionProof, EvidenceKind, EvidenceRequest,
        MemoryLifecycle, ModelRouteRequest, OpenRequest, RecordKind, RecordRequest,
        TaskClaimRequest, TaskCompleteRequest, TaskCreateRequest, TaskMemoryUseListRequest,
        TaskMemoryUseRequest, TaskWorkPacketRequest, WorkComplexity, WorkKind,
        workspace_file_claim,
    },
};
use sha2::{Digest, Sha256};

#[test]
fn planned_memory_is_linked_to_verified_task_criterion_across_restart() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let session = hub
        .open_session(&OpenRequest {
            workspace: workspace.clone(),
            objective: "Use project memory in a task".into(),
            idempotency_key: "open".into(),
        })
        .unwrap()
        .session_id;
    let memory = hub
        .record(&RecordRequest {
            session_id: session.clone(),
            kind: RecordKind::Decision,
            content: "Check the migration against a fresh workspace".into(),
            evidence: None,
            supersedes_record_id: None,
            verifies_effect_id: None,
            idempotency_key: "decision".into(),
        })
        .unwrap()
        .record;
    let memory_id = format!("record:{}", memory.record_id);
    let proof_file = std::path::Path::new(&workspace).join("proof.txt");
    std::fs::write(&proof_file, "fresh workspace migration passed").unwrap();
    let locator = proof_file.to_string_lossy().into_owned();
    let digest = format!("{:x}", Sha256::digest(std::fs::read(&proof_file).unwrap()));
    let criterion = workspace_file_claim(&locator, &digest);
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id: session.clone(),
            objective: "Verify migration".into(),
            acceptance_criteria: vec![criterion.clone()],
            write_scope: vec![],
            depends_on: vec![],
            idempotency_key: "task".into(),
        })
        .unwrap()
        .task
        .task_id;

    let request = TaskMemoryUseRequest {
        task_id: task_id.clone(),
        memory_id: memory_id.clone(),
        criterion: criterion.clone(),
        intended_action: "Run the migration check in a fresh workspace".into(),
        idempotency_key: "apply".into(),
    };
    assert!(
        hub.apply_task_memory(&TaskMemoryUseRequest {
            criterion: "unrelated".into(),
            idempotency_key: "bad-criterion".into(),
            ..request.clone()
        })
        .is_err()
    );
    let linked = hub.apply_task_memory(&request).unwrap();
    assert!(!linked.duplicate);
    assert!(hub.apply_task_memory(&request).unwrap().duplicate);
    assert!(
        hub.apply_task_memory(&TaskMemoryUseRequest {
            intended_action: "Different action".into(),
            ..request.clone()
        })
        .is_err()
    );
    let other_workspace = area.path().join("other");
    std::fs::create_dir(&other_workspace).unwrap();
    let other_session = hub
        .open_session(&OpenRequest {
            workspace: other_workspace.to_string_lossy().into_owned(),
            objective: "Separate project".into(),
            idempotency_key: "other-open".into(),
        })
        .unwrap()
        .session_id;
    let foreign = hub
        .record(&RecordRequest {
            session_id: other_session,
            kind: RecordKind::Decision,
            content: "Foreign workspace decision".into(),
            evidence: None,
            supersedes_record_id: None,
            verifies_effect_id: None,
            idempotency_key: "foreign-decision".into(),
        })
        .unwrap()
        .record;
    assert!(
        hub.apply_task_memory(&TaskMemoryUseRequest {
            memory_id: format!("record:{}", foreign.record_id),
            idempotency_key: "foreign-link".into(),
            ..request.clone()
        })
        .is_err()
    );
    let list_request = TaskMemoryUseListRequest {
        workspace: workspace.clone(),
        task_id: task_id.clone(),
    };
    let packet_request = TaskWorkPacketRequest {
        workspace: workspace.clone(),
        task_id: task_id.clone(),
        route: ModelRouteRequest {
            work_kind: WorkKind::Implementation,
            complexity: WorkComplexity::Bounded,
            consequence: Consequence::Low,
            ambiguity_high: false,
            independent_review: true,
        },
    };
    let packet = hub.task_work_packet(&packet_request).unwrap();
    assert_eq!(packet.task.task_id, task_id);
    assert_eq!(packet.memory_uses.len(), 1);
    assert_eq!(packet.route.model, "gpt-5.6-terra");
    assert_eq!(packet.route.verifier_model.as_deref(), Some("gpt-6-astra"));
    assert_eq!(packet.reviewer_reasoning_effort.as_deref(), Some("high"));
    assert!(packet.advisory);
    assert!(!packet.executable);
    assert!(
        hub.task_work_packet(&TaskWorkPacketRequest {
            workspace: other_workspace.to_string_lossy().into_owned(),
            ..packet_request
        })
        .is_err()
    );
    assert_eq!(
        hub.list_task_memory_uses(&list_request).unwrap()[0].criterion_verified_claim_id,
        None
    );
    assert!(
        hub.export_project(&workspace)
            .unwrap()
            .task_memory_uses
            .iter()
            .any(|item| item.memory_id == memory_id)
    );

    hub.claim_task(&TaskClaimRequest {
        task_id: task_id.clone(),
        worker_id: "worker".into(),
        lease_seconds: 300,
        idempotency_key: "claim".into(),
    })
    .unwrap();
    assert!(
        hub.apply_task_memory(&TaskMemoryUseRequest {
            idempotency_key: "late".into(),
            ..request.clone()
        })
        .is_err()
    );
    let evidence_id = hub
        .add_evidence(&EvidenceRequest {
            session_id: session.clone(),
            kind: EvidenceKind::WorkspaceFile,
            locator,
            summary: "Fresh migration result".into(),
            content_sha256: None,
            idempotency_key: "evidence".into(),
        })
        .unwrap()
        .evidence
        .evidence_id;
    let claim_id = hub
        .assert_claim(&ClaimRequest {
            session_id: session.clone(),
            status: ClaimStatus::Verified,
            statement: criterion.clone(),
            material: true,
            evidence_ids: vec![evidence_id],
            supersedes_claim_id: None,
            idempotency_key: "claim-proof".into(),
        })
        .unwrap()
        .claim
        .claim_id;
    hub.complete_task(&TaskCompleteRequest {
        task_id: task_id.clone(),
        worker_id: "worker".into(),
        outcome_summary: "Migration criterion verified".into(),
        criterion_proofs: vec![CriterionProof {
            criterion,
            verified_claim_id: claim_id.clone(),
        }],
        idempotency_key: "complete".into(),
    })
    .unwrap();
    drop(hub);

    let restarted = Hub::open(&database).unwrap();
    let uses = restarted.list_task_memory_uses(&list_request).unwrap();
    assert_eq!(uses.len(), 1);
    assert_eq!(
        uses[0].criterion_verified_claim_id.as_deref(),
        Some(claim_id.as_str())
    );
    assert_eq!(uses[0].memory_lifecycle_state, MemoryLifecycle::Active);
    assert!(uses[0].advisory);
    assert_eq!(restarted.stats().unwrap().schema_version, 22);
    restarted
        .record(&RecordRequest {
            session_id: session,
            kind: RecordKind::Decision,
            content: "Use a different migration check".into(),
            evidence: None,
            supersedes_record_id: Some(memory.record_id),
            verifies_effect_id: None,
            idempotency_key: "new-decision".into(),
        })
        .unwrap();
    let uses = restarted.list_task_memory_uses(&list_request).unwrap();
    assert_eq!(uses[0].memory_lifecycle_state, MemoryLifecycle::Superseded);
    assert_eq!(
        uses[0].criterion_verified_claim_id.as_deref(),
        Some(claim_id.as_str())
    );
}
