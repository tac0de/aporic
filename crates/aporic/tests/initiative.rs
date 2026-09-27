use aporic::{
    Hub,
    domain::{
        ClaimRequest, ClaimStatus, CriterionProof, DelegationChoice, DelegationDecisionRequest,
        DelegationDisposition, EvidenceKind, EvidenceRequest, InitiativeArtifactKind,
        InitiativeArtifactRequest, InitiativePlanRequest, InitiativeRequirement,
        InitiativeStatusRequest, InitiativeTaskLinkRequest, OpenRequest, TaskClaimRequest,
        TaskCompleteRequest, TaskCreateRequest, WorkflowAdvanceRequest, WorkflowPlanRequest,
        WorkflowStage, WorkflowStepDisposition, WorkflowStepRecordRequest, workspace_file_claim,
    },
};
use sha2::{Digest, Sha256};

fn file_evidence(hub: &Hub, session: &str, path: &std::path::Path, key: &str) -> String {
    std::fs::write(path, key).unwrap();
    hub.add_evidence(&EvidenceRequest {
        session_id: session.into(),
        kind: EvidenceKind::WorkspaceFile,
        locator: path.to_string_lossy().into_owned(),
        summary: key.into(),
        content_sha256: None,
        idempotency_key: key.into(),
    })
    .unwrap()
    .evidence
    .evidence_id
}

fn step(hub: &Hub, task_id: &str, step_id: &str, evidence_id: String) {
    hub.record_workflow_step(&WorkflowStepRecordRequest {
        task_id: task_id.into(),
        step_id: step_id.into(),
        disposition: WorkflowStepDisposition::Completed,
        evidence_ids: vec![evidence_id],
        reason: None,
        idempotency_key: format!("workflow-step-{step_id}"),
    })
    .unwrap();
}

fn advance(hub: &Hub, task_id: &str, stage: WorkflowStage, artifact: Option<String>) {
    let key = format!("workflow-advance-{stage:?}");
    hub.advance_workflow(&WorkflowAdvanceRequest {
        task_id: task_id.into(),
        expected_stage: stage,
        artifact_evidence_ids: artifact.into_iter().collect(),
        user_decision_evidence_id: None,
        idempotency_key: key,
    })
    .unwrap();
}

#[test]
fn initiative_requires_traceable_completion_and_reopens_on_revision() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let session = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Build a product".into(),
            idempotency_key: "initiative-open".into(),
        })
        .unwrap()
        .session_id;
    let proof_path = workspace.join("checkout.txt");
    std::fs::write(&proof_path, "checkout-proof").unwrap();
    let proof_digest = format!("{:x}", Sha256::digest(std::fs::read(&proof_path).unwrap()));
    let criterion = workspace_file_claim(&proof_path.to_string_lossy(), &proof_digest);
    let plan_request = InitiativePlanRequest {
        session_id: session.clone(),
        initiative_id: None,
        revision_evidence_id: None,
        objective: "Ship checkout".into(),
        target_user: "Buyer".into(),
        success_measure: "Buyer completes purchase".into(),
        requirements: vec![InitiativeRequirement {
            requirement_id: "R1".into(),
            statement: "Buyer can check out".into(),
            acceptance_criterion: criterion.clone(),
        }],
        idempotency_key: "initiative-plan".into(),
    };
    let initial = hub.plan_initiative(&plan_request).unwrap();
    let id = initial.status.initiative_id.clone();
    assert!(
        initial
            .status
            .missing
            .contains(&"requirement_unlinked:R1".into())
    );
    assert!(!initial.status.ready_to_claim);
    assert!(hub.plan_initiative(&plan_request).unwrap().duplicate);

    for kind in [
        InitiativeArtifactKind::Planning,
        InitiativeArtifactKind::Design,
    ] {
        let key = format!("artifact-{}", kind.as_str());
        let evidence = file_evidence(&hub, &session, &workspace.join(format!("{key}.md")), &key);
        hub.record_initiative_artifact(&InitiativeArtifactRequest {
            session_id: session.clone(),
            initiative_id: id.clone(),
            kind,
            evidence_id: evidence,
            idempotency_key: format!("record-{key}"),
        })
        .unwrap();
    }
    let task = hub
        .create_task(&TaskCreateRequest {
            session_id: session.clone(),
            objective: "Implement checkout".into(),
            acceptance_criteria: vec![criterion.clone()],
            write_scope: vec![],
            depends_on: vec![],
            idempotency_key: "checkout-task".into(),
        })
        .unwrap()
        .task;
    assert!(
        hub.link_initiative_task(&InitiativeTaskLinkRequest {
            session_id: session.clone(),
            initiative_id: id.clone(),
            requirement_id: "R1".into(),
            task_id: "missing".into(),
            idempotency_key: "bad-link".into(),
        })
        .is_err()
    );
    hub.link_initiative_task(&InitiativeTaskLinkRequest {
        session_id: session.clone(),
        initiative_id: id.clone(),
        requirement_id: "R1".into(),
        task_id: task.task_id.clone(),
        idempotency_key: "good-link".into(),
    })
    .unwrap();
    hub.plan_workflow(&WorkflowPlanRequest {
        task_id: task.task_id.clone(),
        objective: "Implement checkout".into(),
        target_user: "Buyer".into(),
        constraints: "Local".into(),
        success_measure: "Checkout scenario passes".into(),
        material_unknowns: vec![],
        unknown_resolutions: vec![],
        scope_change_evidence_id: None,
        requires_user_decision: false,
        material_change: false,
        procedure_profile: None,
        procedure_depth: None,
        idempotency_key: "checkout-workflow-plan".into(),
    })
    .unwrap();
    let problem = file_evidence(
        &hub,
        &session,
        &workspace.join("problem.md"),
        "problem-step",
    );
    step(&hub, &task.task_id, "problem_and_outcome", problem);
    advance(&hub, &task.task_id, WorkflowStage::Intake, None);
    let baseline = file_evidence(
        &hub,
        &session,
        &workspace.join("baseline.md"),
        "baseline-step",
    );
    step(
        &hub,
        &task.task_id,
        "baseline_and_constraints",
        baseline.clone(),
    );
    advance(&hub, &task.task_id, WorkflowStage::Planning, Some(baseline));
    let design = file_evidence(&hub, &session, &workspace.join("design.md"), "design-step");
    step(&hub, &task.task_id, "delivery_contract", design.clone());
    hub.assess_delegation(&DelegationDecisionRequest {
        task_id: task.task_id.clone(),
        parallel_paths: 1,
        material_change: false,
        worker: DelegationChoice {
            disposition: DelegationDisposition::NotRequired,
            reason: "One implementation path".into(),
        },
        reviewer: DelegationChoice {
            disposition: DelegationDisposition::NotRequired,
            reason: "Focused fixture".into(),
        },
        idempotency_key: "checkout-delegation".into(),
    })
    .unwrap();
    advance(&hub, &task.task_id, WorkflowStage::Design, Some(design));
    let implementation = file_evidence(&hub, &session, &workspace.join("impl.md"), "impl-step");
    step(
        &hub,
        &task.task_id,
        "vertical_slice",
        implementation.clone(),
    );
    advance(
        &hub,
        &task.task_id,
        WorkflowStage::Implementation,
        Some(implementation),
    );
    let status_request = InitiativeStatusRequest {
        workspace: workspace.to_string_lossy().into_owned(),
        initiative_id: id.clone(),
    };
    assert!(
        hub.initiative_status(&status_request)
            .unwrap()
            .missing
            .iter()
            .any(|gap| gap.starts_with("requirement_unverified:R1:"))
    );

    hub.claim_task(&TaskClaimRequest {
        task_id: task.task_id.clone(),
        worker_id: "worker".into(),
        lease_seconds: 300,
        idempotency_key: "claim-checkout".into(),
    })
    .unwrap();
    let proof = file_evidence(&hub, &session, &proof_path, "checkout-proof");
    let verified_claim_id = hub
        .assert_claim(&ClaimRequest {
            session_id: session.clone(),
            status: ClaimStatus::Verified,
            statement: criterion.clone(),
            material: true,
            evidence_ids: vec![proof],
            subject_key: None,
            supersedes_claim_id: None,
            idempotency_key: "checkout-verified".into(),
        })
        .unwrap()
        .claim
        .claim_id;
    let mechanical = file_evidence(&hub, &session, &workspace.join("checks.md"), "checks-step");
    step(&hub, &task.task_id, "mechanical_checks", mechanical);
    hub.complete_task(&TaskCompleteRequest {
        task_id: task.task_id.clone(),
        worker_id: "worker".into(),
        outcome_summary: "Verified".into(),
        criterion_proofs: vec![CriterionProof {
            criterion: criterion.clone(),
            verified_claim_id: verified_claim_id.clone(),
        }],
        idempotency_key: "checkout-complete".into(),
    })
    .unwrap();
    advance(&hub, &task.task_id, WorkflowStage::Verification, None);
    for kind in [
        InitiativeArtifactKind::Integration,
        InitiativeArtifactKind::Release,
    ] {
        let key = format!("artifact-{}", kind.as_str());
        let evidence = file_evidence(&hub, &session, &workspace.join(format!("{key}.md")), &key);
        hub.record_initiative_artifact(&InitiativeArtifactRequest {
            session_id: session.clone(),
            initiative_id: id.clone(),
            kind,
            evidence_id: evidence,
            idempotency_key: format!("record-{key}"),
        })
        .unwrap();
    }
    drop(hub);
    let hub = Hub::open(&database).unwrap();
    let status = hub.initiative_status(&status_request).unwrap();
    assert!(status.ready_to_claim, "{:?}", status.missing);
    assert!(status.missing.is_empty());
    std::fs::write(&proof_path, "changed after verification").unwrap();
    let stale = hub.initiative_status(&status_request).unwrap();
    assert!(!stale.ready_to_claim);
    assert!(
        stale
            .missing
            .iter()
            .any(|gap| gap.starts_with("requirement_unverified:R1:"))
    );
    std::fs::write(&proof_path, "checkout-proof").unwrap();
    assert!(
        hub.initiative_status(&status_request)
            .unwrap()
            .ready_to_claim
    );
    let release_path = workspace.join("artifact-release.md");
    std::fs::write(&release_path, "stale release record").unwrap();
    let stale_release = hub.initiative_status(&status_request).unwrap();
    assert!(!stale_release.ready_to_claim);
    assert!(
        stale_release
            .missing
            .contains(&"artifact_stale:release".into())
    );
    std::fs::write(&release_path, "artifact-release").unwrap();
    assert!(
        hub.initiative_status(&status_request)
            .unwrap()
            .ready_to_claim
    );
    let replacement_evidence = hub
        .add_evidence(&EvidenceRequest {
            session_id: session.clone(),
            kind: EvidenceKind::WorkspaceFile,
            locator: proof_path.to_string_lossy().into_owned(),
            summary: "Second observation of checkout proof".into(),
            content_sha256: None,
            idempotency_key: "replacement-checkout-proof".into(),
        })
        .unwrap()
        .evidence
        .evidence_id;
    // A newer claim retires the exact proof used by the completed task.
    hub.assert_claim(&ClaimRequest {
        session_id: session.clone(),
        status: ClaimStatus::Verified,
        statement: criterion.clone(),
        material: true,
        evidence_ids: vec![replacement_evidence],
        subject_key: None,
        supersedes_claim_id: Some(verified_claim_id),
        idempotency_key: "replacement-claim".into(),
    })
    .unwrap();
    assert!(
        !hub.initiative_status(&status_request)
            .unwrap()
            .ready_to_claim
    );
    assert!(
        hub.export_project(&status_request.workspace)
            .unwrap()
            .events
            .iter()
            .any(|event| event.kind == "initiative_task_linked")
    );

    let revision_evidence = hub
        .add_evidence(&EvidenceRequest {
            session_id: session.clone(),
            kind: EvidenceKind::UserStatement,
            locator: "conversation:revision".into(),
            summary: "User revised the product plan".into(),
            content_sha256: Some("a".repeat(64)),
            idempotency_key: "revision-user-statement".into(),
        })
        .unwrap()
        .evidence
        .evidence_id;
    assert!(
        hub.plan_initiative(&InitiativePlanRequest {
            session_id: session.clone(),
            initiative_id: Some(id.clone()),
            revision_evidence_id: None,
            idempotency_key: "revision-without-evidence".into(),
            ..plan_request.clone()
        })
        .is_err()
    );
    let revised = hub
        .plan_initiative(&InitiativePlanRequest {
            session_id: session,
            initiative_id: Some(id),
            revision_evidence_id: Some(revision_evidence),
            idempotency_key: "revision-with-evidence".into(),
            ..plan_request
        })
        .unwrap();
    assert!(!revised.status.ready_to_claim);
    assert!(revised.status.task_links.is_empty());
    assert!(revised.status.artifacts.is_empty());
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection.execute(
        "UPDATE events SET result_json = json_set(result_json, '$.status.plan.objective', 'Forged')
         WHERE idempotency_key = 'initiative-plan'",
        [],
    ).unwrap();
    assert!(hub.initiative_status(&status_request).is_err());
}
