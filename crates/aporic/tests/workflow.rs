use aporic::{
    Hub,
    domain::{
        ClaimRequest, ClaimStatus, CriterionProof, DelegationChoice, DelegationDecisionRequest,
        DelegationDimension, DelegationDisposition, DelegationReportRequest, DelegationRunOutcome,
        EvidenceKind, EvidenceRequest, OpenRequest, TaskCancelRequest, TaskClaimRequest,
        TaskCompleteRequest, TaskCreateRequest, WorkflowAdvanceRequest, WorkflowPlanRequest,
        WorkflowStage, WorkflowStatusRequest, WorkflowUnknownResolution, workspace_file_claim,
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

fn advance(
    task_id: &str,
    stage: WorkflowStage,
    key: &str,
    artifacts: Vec<String>,
    choice: Option<String>,
) -> WorkflowAdvanceRequest {
    WorkflowAdvanceRequest {
        task_id: task_id.into(),
        expected_stage: stage,
        artifact_evidence_ids: artifacts,
        user_decision_evidence_id: choice,
        idempotency_key: key.into(),
    }
}

#[test]
fn missing_requirements_and_stage_evidence_prevent_advancement_across_restart() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let session = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Staged delivery".into(),
            idempotency_key: "open".into(),
        })
        .unwrap()
        .session_id;
    let final_file = workspace.join("verified.txt");
    std::fs::write(&final_file, "verified").unwrap();
    let final_digest = format!("{:x}", Sha256::digest(std::fs::read(&final_file).unwrap()));
    let criterion = workspace_file_claim(&final_file.to_string_lossy(), &final_digest);
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id: session.clone(),
            objective: "Staged implementation".into(),
            acceptance_criteria: vec![criterion.clone()],
            write_scope: vec![],
            depends_on: vec![],
            idempotency_key: "task".into(),
        })
        .unwrap()
        .task
        .task_id;
    let initial = WorkflowPlanRequest {
        task_id: task_id.clone(),
        objective: "Build a useful slice".into(),
        target_user: "".into(),
        constraints: "local only".into(),
        success_measure: "user can finish the flow".into(),
        material_unknowns: vec!["interaction direction".into()],
        unknown_resolutions: vec![],
        scope_change_evidence_id: None,
        requires_user_decision: true,
        material_change: true,
        idempotency_key: "plan-1".into(),
    };
    let status = hub.plan_workflow(&initial).unwrap().status;
    assert_eq!(
        status.missing_for_next_stage,
        ["target_user_missing", "material_unknowns_unresolved"]
    );
    assert!(
        hub.advance_workflow(&advance(
            &task_id,
            WorkflowStage::Intake,
            "early",
            vec![],
            None
        ))
        .is_err()
    );
    assert!(
        hub.plan_workflow(&WorkflowPlanRequest {
            requires_user_decision: false,
            idempotency_key: "relax-without-user".into(),
            ..initial.clone()
        })
        .is_err()
    );
    let resolution = file_evidence(
        &hub,
        &session,
        &workspace.join("resolution.md"),
        "resolution",
    );
    assert!(
        hub.plan_workflow(&WorkflowPlanRequest {
            target_user: "citizen".into(),
            material_unknowns: vec![],
            idempotency_key: "unresolved-removal".into(),
            ..initial.clone()
        })
        .is_err()
    );
    let revised = WorkflowPlanRequest {
        target_user: "citizen".into(),
        material_unknowns: vec![],
        unknown_resolutions: vec![WorkflowUnknownResolution {
            unknown: "interaction direction".into(),
            evidence_id: resolution,
        }],
        idempotency_key: "plan-2".into(),
        ..initial
    };
    assert!(
        hub.plan_workflow(&revised)
            .unwrap()
            .status
            .missing_for_next_stage
            .is_empty()
    );
    assert!(hub.plan_workflow(&revised).unwrap().duplicate);
    hub.advance_workflow(&advance(
        &task_id,
        WorkflowStage::Intake,
        "to-planning",
        vec![],
        None,
    ))
    .unwrap();
    assert!(
        hub.advance_workflow(&advance(
            &task_id,
            WorkflowStage::Planning,
            "no-plan-artifact",
            vec![],
            None
        ))
        .is_err()
    );
    let planning = file_evidence(&hub, &session, &workspace.join("plan.md"), "planning");
    hub.advance_workflow(&advance(
        &task_id,
        WorkflowStage::Planning,
        "to-design",
        vec![planning],
        None,
    ))
    .unwrap();
    assert_eq!(
        hub.workflow_status(&WorkflowStatusRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            task_id: task_id.clone(),
        })
        .unwrap()
        .missing_for_next_stage,
        ["delegation_assessment_missing_for_plan"]
    );
    let design = file_evidence(&hub, &session, &workspace.join("design.md"), "design");
    assert!(
        hub.advance_workflow(&advance(
            &task_id,
            WorkflowStage::Design,
            "no-delegation",
            vec![design.clone()],
            None
        ))
        .is_err()
    );
    let decision = hub
        .assess_delegation(&DelegationDecisionRequest {
            task_id: task_id.clone(),
            parallel_paths: 1,
            material_change: true,
            worker: DelegationChoice {
                disposition: DelegationDisposition::NotRequired,
                reason: "One coupled path".into(),
            },
            reviewer: DelegationChoice {
                disposition: DelegationDisposition::Delegate,
                reason: "Independent inspection".into(),
            },
            idempotency_key: "assess".into(),
        })
        .unwrap()
        .decision;
    let review_start = DelegationReportRequest {
        task_id: task_id.clone(),
        decision_id: decision.decision_id.clone(),
        dimension: DelegationDimension::Reviewer,
        host_agent_id: "inspector".into(),
        model: "host-selected".into(),
        reasoning_effort: "high".into(),
        outcome: DelegationRunOutcome::Started,
        result_summary: "review-start".into(),
        idempotency_key: "review-start".into(),
    };
    hub.report_delegation(&review_start).unwrap();
    assert!(
        hub.advance_workflow(&advance(
            &task_id,
            WorkflowStage::Design,
            "no-choice",
            vec![design.clone()],
            None
        ))
        .is_err()
    );
    let choice = hub
        .add_evidence(&EvidenceRequest {
            session_id: session.clone(),
            kind: EvidenceKind::UserStatement,
            locator: "conversation:choice".into(),
            summary: "User chose direction".into(),
            content_sha256: Some("a".repeat(64)),
            idempotency_key: "choice".into(),
        })
        .unwrap()
        .evidence
        .evidence_id;
    hub.advance_workflow(&advance(
        &task_id,
        WorkflowStage::Design,
        "to-implementation",
        vec![design],
        Some(choice),
    ))
    .unwrap();
    let implementation = file_evidence(
        &hub,
        &session,
        &workspace.join("source.rs"),
        "implementation",
    );
    hub.advance_workflow(&advance(
        &task_id,
        WorkflowStage::Implementation,
        "to-verification",
        vec![implementation],
        None,
    ))
    .unwrap();
    hub.assess_delegation(&DelegationDecisionRequest {
        task_id: task_id.clone(),
        parallel_paths: 1,
        material_change: false,
        worker: DelegationChoice {
            disposition: DelegationDisposition::NotRequired,
            reason: "New assessment".into(),
        },
        reviewer: DelegationChoice {
            disposition: DelegationDisposition::NotRequired,
            reason: "Later assessment".into(),
        },
        idempotency_key: "later-assess".into(),
    })
    .unwrap();
    let verification = hub
        .workflow_status(&WorkflowStatusRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            task_id: task_id.clone(),
        })
        .unwrap();
    assert!(
        verification
            .missing_for_next_stage
            .contains(&"reviewer_completion_not_reported".into())
    );
    assert!(
        hub.advance_workflow(&advance(
            &task_id,
            WorkflowStage::Verification,
            "early-done",
            vec![],
            None
        ))
        .is_err()
    );
    hub.report_delegation(&DelegationReportRequest {
        outcome: DelegationRunOutcome::Completed,
        result_summary: "review-complete".into(),
        idempotency_key: "review-complete".into(),
        ..review_start
    })
    .unwrap();
    hub.claim_task(&TaskClaimRequest {
        task_id: task_id.clone(),
        worker_id: "worker".into(),
        lease_seconds: 300,
        idempotency_key: "claim".into(),
    })
    .unwrap();
    let proof = hub
        .add_evidence(&EvidenceRequest {
            session_id: session.clone(),
            kind: EvidenceKind::WorkspaceFile,
            locator: final_file.to_string_lossy().into_owned(),
            summary: "Verified result".into(),
            content_sha256: None,
            idempotency_key: "final-evidence".into(),
        })
        .unwrap()
        .evidence
        .evidence_id;
    let verified_claim_id = hub
        .assert_claim(&ClaimRequest {
            session_id: session,
            status: ClaimStatus::Verified,
            statement: criterion.clone(),
            material: true,
            evidence_ids: vec![proof],
            supersedes_claim_id: None,
            idempotency_key: "verified-claim".into(),
        })
        .unwrap()
        .claim
        .claim_id;
    hub.complete_task(&TaskCompleteRequest {
        task_id: task_id.clone(),
        worker_id: "worker".into(),
        outcome_summary: "Verified".into(),
        criterion_proofs: vec![CriterionProof {
            criterion,
            verified_claim_id,
        }],
        idempotency_key: "task-complete".into(),
    })
    .unwrap();
    drop(hub);
    let restarted = Hub::open(&database).unwrap();
    let request = WorkflowStatusRequest {
        workspace: workspace.to_string_lossy().into_owned(),
        task_id: task_id.clone(),
    };
    let status = restarted.workflow_status(&request).unwrap();
    assert_eq!(status.stage, WorkflowStage::Verification);
    assert!(status.missing_for_next_stage.is_empty());
    let complete = restarted
        .advance_workflow(&advance(
            &task_id,
            WorkflowStage::Verification,
            "workflow-complete",
            vec![],
            None,
        ))
        .unwrap();
    assert_eq!(complete.status.stage, WorkflowStage::Completed);
    assert_eq!(complete.status.transitions.len(), 5);
    assert!(complete.status.advisory);
    assert!(!complete.status.executable);
    assert!(
        restarted
            .export_project(&request.workspace)
            .unwrap()
            .events
            .iter()
            .any(|event| event.kind == "task_workflow_advanced")
    );
}

#[test]
fn evidence_cannot_cross_task_workflows_and_cancelled_tasks_stop() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let session = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Two tasks".into(),
            idempotency_key: "open".into(),
        })
        .unwrap()
        .session_id;
    let proof_path = workspace.join("proof.txt");
    std::fs::write(&proof_path, "first-proof").unwrap();
    let proof_digest = format!("{:x}", Sha256::digest(std::fs::read(&proof_path).unwrap()));
    let first_criterion = workspace_file_claim(&proof_path.to_string_lossy(), &proof_digest);
    let tasks = ["first", "second"].map(|name| {
        hub.create_task(&TaskCreateRequest {
            session_id: session.clone(),
            objective: name.into(),
            acceptance_criteria: vec![if name == "first" {
                first_criterion.clone()
            } else {
                "second done".into()
            }],
            write_scope: vec![],
            depends_on: vec![],
            idempotency_key: format!("task-{name}"),
        })
        .unwrap()
        .task
        .task_id
    });
    for (i, task_id) in tasks.iter().enumerate() {
        hub.plan_workflow(&WorkflowPlanRequest {
            task_id: task_id.clone(),
            objective: "Goal".into(),
            target_user: "User".into(),
            constraints: "Local".into(),
            success_measure: "Done".into(),
            material_unknowns: vec![],
            unknown_resolutions: vec![],
            scope_change_evidence_id: None,
            requires_user_decision: false,
            material_change: false,
            idempotency_key: format!("plan-{i}"),
        })
        .unwrap();
        hub.advance_workflow(&advance(
            task_id,
            WorkflowStage::Intake,
            &format!("to-planning-{i}"),
            vec![],
            None,
        ))
        .unwrap();
    }
    let artifact = file_evidence(&hub, &session, &workspace.join("plan.md"), "shared-plan");
    hub.advance_workflow(&advance(
        &tasks[0],
        WorkflowStage::Planning,
        "first-design",
        vec![artifact.clone()],
        None,
    ))
    .unwrap();
    assert!(
        hub.advance_workflow(&advance(
            &tasks[1],
            WorkflowStage::Planning,
            "reused-evidence",
            vec![artifact],
            None
        ))
        .is_err()
    );
    hub.cancel_task(&TaskCancelRequest {
        task_id: tasks[1].clone(),
        reason: "Stopped".into(),
        idempotency_key: "cancel".into(),
    })
    .unwrap();
    let second_artifact = file_evidence(&hub, &session, &workspace.join("other.md"), "other-plan");
    assert!(
        hub.advance_workflow(&advance(
            &tasks[1],
            WorkflowStage::Planning,
            "cancelled-advance",
            vec![second_artifact],
            None
        ))
        .is_err()
    );
    let proof = file_evidence(&hub, &session, &proof_path, "first-proof");
    let claim = hub
        .assert_claim(&ClaimRequest {
            session_id: session.clone(),
            status: ClaimStatus::Verified,
            statement: first_criterion.clone(),
            material: true,
            evidence_ids: vec![proof],
            supersedes_claim_id: None,
            idempotency_key: "first-claim".into(),
        })
        .unwrap()
        .claim
        .claim_id;
    hub.claim_task(&TaskClaimRequest {
        task_id: tasks[0].clone(),
        worker_id: "worker".into(),
        lease_seconds: 300,
        idempotency_key: "first-claim-task".into(),
    })
    .unwrap();
    hub.complete_task(&TaskCompleteRequest {
        task_id: tasks[0].clone(),
        worker_id: "worker".into(),
        outcome_summary: "Done".into(),
        criterion_proofs: vec![CriterionProof {
            criterion: first_criterion,
            verified_claim_id: claim,
        }],
        idempotency_key: "first-complete-task".into(),
    })
    .unwrap();
    let late_artifact = file_evidence(&hub, &session, &workspace.join("late.md"), "late-artifact");
    assert!(
        hub.advance_workflow(&advance(
            &tasks[0],
            WorkflowStage::Design,
            "completed-advance",
            vec![late_artifact],
            None
        ))
        .is_err()
    );
}
