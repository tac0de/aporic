use aporic::{
    Hub,
    domain::{
        ClaimRequest, ClaimStatus, CriterionProof, DelegationChoice, DelegationDecisionRequest,
        DelegationDimension, DelegationDisposition, DelegationReportRequest, DelegationRunOutcome,
        EvidenceKind, EvidenceRequest, OpenRequest, TaskCancelRequest, TaskClaimRequest,
        TaskCompleteRequest, TaskCreateRequest, WorkflowAdvanceRequest, WorkflowPlanRequest,
        WorkflowProcedureDepth, WorkflowProcedureProfile, WorkflowStage, WorkflowStatusRequest,
        WorkflowStepDisposition, WorkflowStepRecordRequest, WorkflowStepsRequest,
        WorkflowUnknownResolution, workspace_file_claim,
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

fn step(
    task_id: &str,
    step_id: &str,
    disposition: WorkflowStepDisposition,
    evidence_ids: Vec<String>,
    reason: Option<&str>,
    key: &str,
) -> WorkflowStepRecordRequest {
    WorkflowStepRecordRequest {
        task_id: task_id.into(),
        step_id: step_id.into(),
        disposition,
        evidence_ids,
        reason: reason.map(str::to_owned),
        idempotency_key: key.into(),
    }
}

#[test]
fn new_material_plan_defaults_to_versioned_standard_procedure() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let session = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Default procedure".into(),
            idempotency_key: "default-open".into(),
        })
        .unwrap()
        .session_id;
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id: session,
            objective: "Material change".into(),
            acceptance_criteria: vec!["Checked".into()],
            write_scope: vec![],
            depends_on: vec![],
            idempotency_key: "default-task".into(),
        })
        .unwrap()
        .task
        .task_id;
    let status = hub
        .plan_workflow(&WorkflowPlanRequest {
            task_id: task_id.clone(),
            objective: "Improve behavior".into(),
            target_user: "Operator".into(),
            constraints: "Local".into(),
            success_measure: "Scenario works".into(),
            material_unknowns: vec![],
            unknown_resolutions: vec![],
            scope_change_evidence_id: None,
            requires_user_decision: false,
            material_change: true,
            procedure_profile: None,
            procedure_depth: None,
            idempotency_key: "default-plan".into(),
        })
        .unwrap()
        .status;
    let plan = status.plan.unwrap();
    assert_eq!(
        plan.procedure_profile,
        Some(WorkflowProcedureProfile::General)
    );
    assert_eq!(plan.procedure_depth, Some(WorkflowProcedureDepth::Standard));
    assert_eq!(plan.procedure_template_version, Some(1));
    assert!(
        hub.advance_workflow(&advance(
            &task_id,
            WorkflowStage::Intake,
            "default-bypass",
            vec![],
            None
        ))
        .is_err()
    );
    rusqlite::Connection::open(&database)
        .unwrap()
        .execute(
            "UPDATE events SET result_json = json_set(result_json,
         '$.status.plan.procedure_template_version', 99)
         WHERE idempotency_key = 'default-plan'",
            [],
        )
        .unwrap();
    let status = hub
        .workflow_status(&WorkflowStatusRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            task_id: task_id.clone(),
        })
        .unwrap();
    assert!(
        status
            .missing_for_next_stage
            .contains(&"procedure_template_version_unsupported".into())
    );
    assert!(
        hub.advance_workflow(&advance(
            &task_id,
            WorkflowStage::Intake,
            "unsupported-template-advance",
            vec![],
            None
        ))
        .is_err()
    );
}

#[test]
fn frontend_browser_module_is_versioned_and_survives_restart() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let session = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Review a frontend".into(),
            idempotency_key: "frontend-open".into(),
        })
        .unwrap()
        .session_id;
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id: session.clone(),
            objective: "Review the browser flow".into(),
            acceptance_criteria: vec!["Browser review recorded".into()],
            write_scope: vec![],
            depends_on: vec![],
            idempotency_key: "frontend-task".into(),
        })
        .unwrap()
        .task
        .task_id;
    let plan = WorkflowPlanRequest {
        task_id: task_id.clone(),
        objective: "Make the primary flow usable".into(),
        target_user: "Browser user".into(),
        constraints: "Local review".into(),
        success_measure: "Primary flow works at desktop and narrow widths".into(),
        material_unknowns: vec![],
        unknown_resolutions: vec![],
        scope_change_evidence_id: None,
        requires_user_decision: false,
        material_change: true,
        procedure_profile: Some(WorkflowProcedureProfile::Frontend),
        procedure_depth: Some(WorkflowProcedureDepth::Standard),
        idempotency_key: "frontend-plan".into(),
    };
    let status = hub.plan_workflow(&plan).unwrap().status;
    assert_eq!(status.plan.unwrap().procedure_template_version, Some(2));
    assert!(
        hub.plan_workflow(&WorkflowPlanRequest {
            procedure_profile: Some(WorkflowProcedureProfile::General),
            idempotency_key: "frontend-downgrade-without-user".into(),
            ..plan.clone()
        })
        .is_err()
    );
    let steps = hub
        .workflow_steps(&WorkflowStepsRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            task_id: task_id.clone(),
        })
        .unwrap();
    assert_eq!(steps.template_version, Some(2));
    for id in [
        "browser_scenarios",
        "rendered_browser_review",
        "responsive_accessibility_review",
    ] {
        let definition = steps
            .definitions
            .iter()
            .find(|step| step.step_id == id)
            .unwrap();
        assert_eq!(
            definition.module_id.as_deref(),
            Some("frontend.browser_review")
        );
        assert!(!definition.skippable);
    }
    assert!(
        steps
            .definitions
            .iter()
            .all(|step| step.step_id != "interaction_spec")
    );
    let scenario = steps
        .definitions
        .iter()
        .position(|step| step.step_id == "browser_scenarios")
        .unwrap();
    let rendered = steps
        .definitions
        .iter()
        .position(|step| step.step_id == "rendered_browser_review")
        .unwrap();
    assert!(scenario < rendered);
    assert!(
        hub.record_workflow_step(&step(
            &task_id,
            "rendered_browser_review",
            WorkflowStepDisposition::Completed,
            vec![],
            None,
            "frontend-no-evidence",
        ))
        .is_err()
    );
    let evidence = file_evidence(
        &hub,
        &session,
        &workspace.join("frontend-brief.md"),
        "frontend-brief",
    );
    hub.record_workflow_step(&step(
        &task_id,
        "problem_and_outcome",
        WorkflowStepDisposition::Completed,
        vec![evidence],
        None,
        "frontend-problem-step",
    ))
    .unwrap();
    hub.advance_workflow(&advance(
        &task_id,
        WorkflowStage::Intake,
        "frontend-to-planning",
        vec![],
        None,
    ))
    .unwrap();
    let browser_evidence = file_evidence(
        &hub,
        &session,
        &workspace.join("browser-scenarios.md"),
        "browser-scenarios-evidence",
    );
    hub.record_workflow_step(&step(
        &task_id,
        "browser_scenarios",
        WorkflowStepDisposition::Completed,
        vec![browser_evidence],
        None,
        "browser-scenarios-step",
    ))
    .unwrap();
    drop(hub);
    let restarted = Hub::open(&database).unwrap();
    let restored = restarted
        .workflow_steps(&WorkflowStepsRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            task_id,
        })
        .unwrap();
    assert_eq!(restored.template_version, Some(2));
    assert_eq!(restored.status.step_statuses.len(), 2);
    assert_eq!(
        restored.status.step_statuses[1].step_id,
        "browser_scenarios"
    );
    assert_eq!(restored.status.stage, WorkflowStage::Planning);
    assert!(
        !restored
            .status
            .missing_for_next_stage
            .contains(&"procedure_step_missing:browser_scenarios".into())
    );
}

#[test]
fn nested_ui_steps_require_evidence_and_rework_invalidates_downstream_progress() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let session = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Nested UI workflow".into(),
            idempotency_key: "open-nested".into(),
        })
        .unwrap()
        .session_id;
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id: session.clone(),
            objective: "Deliver an interface".into(),
            acceptance_criteria: vec!["UI reviewed".into()],
            write_scope: vec![],
            depends_on: vec![],
            idempotency_key: "nested-task".into(),
        })
        .unwrap()
        .task
        .task_id;
    let plan = WorkflowPlanRequest {
        task_id: task_id.clone(),
        objective: "Make the UI usable".into(),
        target_user: "Operator".into(),
        constraints: "Local only".into(),
        success_measure: "Operator completes primary flow".into(),
        material_unknowns: vec![],
        unknown_resolutions: vec![],
        scope_change_evidence_id: None,
        requires_user_decision: false,
        material_change: true,
        procedure_profile: Some(WorkflowProcedureProfile::Ui),
        procedure_depth: Some(WorkflowProcedureDepth::Standard),
        idempotency_key: "nested-plan".into(),
    };
    hub.plan_workflow(&plan).unwrap();
    assert!(
        hub.plan_workflow(&WorkflowPlanRequest {
            procedure_profile: Some(WorkflowProcedureProfile::Frontend),
            idempotency_key: "ui-to-frontend-without-user".into(),
            ..plan.clone()
        })
        .is_err()
    );
    assert!(
        hub.plan_workflow(&WorkflowPlanRequest {
            procedure_profile: Some(WorkflowProcedureProfile::General),
            procedure_depth: Some(WorkflowProcedureDepth::Light),
            idempotency_key: "weaken-without-user".into(),
            ..plan.clone()
        })
        .is_err()
    );
    let steps = hub
        .workflow_steps(&WorkflowStepsRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            task_id: task_id.clone(),
        })
        .unwrap();
    assert_eq!(steps.template_version, Some(1));
    assert!(
        steps
            .definitions
            .iter()
            .any(|step| step.step_id == "rendered_browser_review")
    );
    assert!(
        steps
            .status
            .missing_for_next_stage
            .contains(&"procedure_step_missing:problem_and_outcome".into())
    );
    assert!(
        hub.record_workflow_step(&step(
            &task_id,
            "problem_and_outcome",
            WorkflowStepDisposition::Skipped,
            vec![],
            Some("too much work"),
            "bad-skip"
        ))
        .is_err()
    );
    let problem = file_evidence(
        &hub,
        &session,
        &workspace.join("problem.md"),
        "problem-evidence",
    );
    let completed = step(
        &task_id,
        "problem_and_outcome",
        WorkflowStepDisposition::Completed,
        vec![problem],
        None,
        "problem-step",
    );
    assert!(
        hub.record_workflow_step(&completed)
            .unwrap()
            .status
            .missing_for_next_stage
            .is_empty()
    );
    assert!(hub.record_workflow_step(&completed).unwrap().duplicate);
    hub.advance_workflow(&advance(
        &task_id,
        WorkflowStage::Intake,
        "nested-to-planning",
        vec![],
        None,
    ))
    .unwrap();
    let baseline = file_evidence(
        &hub,
        &session,
        &workspace.join("baseline.md"),
        "baseline-evidence",
    );
    hub.record_workflow_step(&step(
        &task_id,
        "baseline_and_constraints",
        WorkflowStepDisposition::Completed,
        vec![baseline.clone()],
        None,
        "baseline-step",
    ))
    .unwrap();
    hub.record_workflow_step(&step(
        &task_id,
        "options_and_risks",
        WorkflowStepDisposition::Skipped,
        vec![],
        Some("One bounded option exists in this pilot"),
        "options-skip",
    ))
    .unwrap();
    hub.record_workflow_step(&step(
        &task_id,
        "verification_strategy",
        WorkflowStepDisposition::Skipped,
        vec![],
        Some("The pilot reuses the existing visual review contract"),
        "strategy-skip",
    ))
    .unwrap();
    hub.advance_workflow(&advance(
        &task_id,
        WorkflowStage::Planning,
        "nested-to-design",
        vec![baseline.clone()],
        None,
    ))
    .unwrap();
    let rework = hub
        .record_workflow_step(&step(
            &task_id,
            "baseline_and_constraints",
            WorkflowStepDisposition::ReworkRequired,
            vec![],
            Some("New evidence changed the baseline"),
            "baseline-rework",
        ))
        .unwrap()
        .status;
    assert_eq!(rework.stage, WorkflowStage::Planning);
    assert!(
        rework
            .missing_for_next_stage
            .contains(&"procedure_step_missing:baseline_and_constraints".into())
    );
    assert!(
        rework
            .missing_for_next_stage
            .contains(&"procedure_step_missing:options_and_risks".into())
    );
    assert!(
        !rework
            .transitions
            .iter()
            .any(|transition| transition.stage == WorkflowStage::Design)
    );
    assert!(
        hub.record_workflow_step(&step(
            &task_id,
            "baseline_and_constraints",
            WorkflowStepDisposition::Completed,
            vec![baseline],
            None,
            "stale-baseline",
        ))
        .is_err()
    );
    let fresh = file_evidence(
        &hub,
        &session,
        &workspace.join("fresh-baseline.md"),
        "fresh-baseline-evidence",
    );
    hub.record_workflow_step(&step(
        &task_id,
        "baseline_and_constraints",
        WorkflowStepDisposition::Completed,
        vec![fresh],
        None,
        "fresh-baseline-step",
    ))
    .unwrap();
    drop(hub);
    let restarted = Hub::open(&database).unwrap();
    let status = restarted
        .workflow_status(&WorkflowStatusRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            task_id,
        })
        .unwrap();
    assert_eq!(status.stage, WorkflowStage::Planning);
    assert_eq!(status.step_statuses.len(), 2);
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
        procedure_profile: Some(WorkflowProcedureProfile::General),
        procedure_depth: Some(WorkflowProcedureDepth::Light),
        idempotency_key: "plan-1".into(),
    };
    let status = hub.plan_workflow(&initial).unwrap().status;
    assert_eq!(
        status.missing_for_next_stage,
        [
            "procedure_step_missing:problem_and_outcome",
            "target_user_missing",
            "material_unknowns_unresolved"
        ]
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
    assert_eq!(
        hub.plan_workflow(&revised)
            .unwrap()
            .status
            .missing_for_next_stage,
        ["procedure_step_missing:problem_and_outcome"]
    );
    assert!(hub.plan_workflow(&revised).unwrap().duplicate);
    let problem = file_evidence(
        &hub,
        &session,
        &workspace.join("problem-legacy.md"),
        "problem-legacy",
    );
    hub.record_workflow_step(&step(
        &task_id,
        "problem_and_outcome",
        WorkflowStepDisposition::Completed,
        vec![problem],
        None,
        "legacy-problem-step",
    ))
    .unwrap();
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
    hub.record_workflow_step(&step(
        &task_id,
        "baseline_and_constraints",
        WorkflowStepDisposition::Completed,
        vec![planning.clone()],
        None,
        "legacy-baseline-step",
    ))
    .unwrap();
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
        [
            "procedure_step_missing:delivery_contract",
            "delegation_assessment_missing_for_plan"
        ]
    );
    let design = file_evidence(&hub, &session, &workspace.join("design.md"), "design");
    hub.record_workflow_step(&step(
        &task_id,
        "delivery_contract",
        WorkflowStepDisposition::Completed,
        vec![design.clone()],
        None,
        "legacy-design-step",
    ))
    .unwrap();
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
    hub.record_workflow_step(&step(
        &task_id,
        "vertical_slice",
        WorkflowStepDisposition::Completed,
        vec![implementation.clone()],
        None,
        "legacy-implementation-step",
    ))
    .unwrap();
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
    hub.record_workflow_step(&step(
        &task_id,
        "mechanical_checks",
        WorkflowStepDisposition::Completed,
        vec![proof.clone()],
        None,
        "legacy-checks-step",
    ))
    .unwrap();
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
    assert!(
        restarted
            .export_project(&request.workspace)
            .unwrap()
            .events
            .iter()
            .any(|event| event.kind == "task_workflow_step_recorded")
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
            procedure_profile: None,
            procedure_depth: None,
            idempotency_key: format!("plan-{i}"),
        })
        .unwrap();
        let problem = file_evidence(
            &hub,
            &session,
            &workspace.join(format!("problem-{i}.md")),
            &format!("problem-{i}"),
        );
        hub.record_workflow_step(&step(
            task_id,
            "problem_and_outcome",
            WorkflowStepDisposition::Completed,
            vec![problem],
            None,
            &format!("problem-step-{i}"),
        ))
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
    hub.record_workflow_step(&step(
        &tasks[0],
        "baseline_and_constraints",
        WorkflowStepDisposition::Completed,
        vec![artifact.clone()],
        None,
        "first-baseline",
    ))
    .unwrap();
    hub.advance_workflow(&advance(
        &tasks[0],
        WorkflowStage::Planning,
        "first-design",
        vec![artifact.clone()],
        None,
    ))
    .unwrap();
    let second_baseline = file_evidence(
        &hub,
        &session,
        &workspace.join("second-baseline.md"),
        "second-baseline",
    );
    hub.record_workflow_step(&step(
        &tasks[1],
        "baseline_and_constraints",
        WorkflowStepDisposition::Completed,
        vec![second_baseline],
        None,
        "second-baseline-step",
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
