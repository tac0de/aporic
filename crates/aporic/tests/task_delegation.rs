use aporic::{
    Hub,
    domain::{
        DelegationChoice, DelegationDecisionRequest, DelegationDimension, DelegationDisposition,
        DelegationReportRequest, DelegationRunOutcome, DelegationStatusRequest, EvidenceGrade,
        OpenRequest, TaskClaimRequest, TaskCreateRequest,
    },
};

fn choice(disposition: DelegationDisposition, reason: &str) -> DelegationChoice {
    DelegationChoice {
        disposition,
        reason: reason.into(),
    }
}

#[test]
fn delegation_history_survives_restart_and_never_gates_claim() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.clone(),
            objective: "Implement delegated work".into(),
            idempotency_key: "open".into(),
        })
        .unwrap()
        .session_id;
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id,
            objective: "Implement and review".into(),
            acceptance_criteria: vec!["Verified behavior".into()],
            write_scope: vec!["src".into()],
            depends_on: vec![],
            idempotency_key: "task".into(),
        })
        .unwrap()
        .task
        .task_id;
    let status_request = DelegationStatusRequest {
        workspace: workspace.clone(),
        task_id: task_id.clone(),
    };
    assert_eq!(
        hub.delegation_status(&status_request)
            .unwrap()
            .advisory_gaps,
        ["delegation_assessment_missing"]
    );
    // Missing advisory coordination does not change the task lease contract.
    hub.claim_task(&TaskClaimRequest {
        task_id: task_id.clone(),
        worker_id: "host-worker".into(),
        lease_seconds: 300,
        idempotency_key: "claim".into(),
    })
    .unwrap();
    let decision = DelegationDecisionRequest {
        task_id: task_id.clone(),
        parallel_paths: 2,
        material_change: true,
        worker: choice(
            DelegationDisposition::Delegate,
            "Independent implementation path",
        ),
        reviewer: choice(
            DelegationDisposition::Skip,
            "No reviewer available this turn",
        ),
        idempotency_key: "assess".into(),
    };
    let assessed = hub.assess_delegation(&decision).unwrap();
    assert_eq!(assessed.decision.evidence_grade, EvidenceGrade::Reported);
    assert!(hub.assess_delegation(&decision).unwrap().duplicate);
    assert!(
        hub.assess_delegation(&DelegationDecisionRequest {
            parallel_paths: 3,
            ..decision.clone()
        })
        .is_err()
    );
    assert!(
        hub.report_delegation(&DelegationReportRequest {
            task_id: task_id.clone(),
            decision_id: assessed.decision.decision_id.clone(),
            dimension: DelegationDimension::Reviewer,
            host_agent_id: "host-reviewer".into(),
            model: "gpt-6-astra".into(),
            reasoning_effort: "high".into(),
            outcome: DelegationRunOutcome::Started,
            result_summary: "Reviewer started".into(),
            idempotency_key: "invalid-review-report".into(),
        })
        .is_err()
    );
    let report = DelegationReportRequest {
        task_id: task_id.clone(),
        decision_id: assessed.decision.decision_id.clone(),
        dimension: DelegationDimension::Worker,
        host_agent_id: "host-worker".into(),
        model: "gpt-6-luna".into(),
        reasoning_effort: "low".into(),
        outcome: DelegationRunOutcome::Started,
        result_summary: "Worker started implementation".into(),
        idempotency_key: "worker-start".into(),
    };
    hub.report_delegation(&report).unwrap();
    assert!(hub.report_delegation(&report).unwrap().duplicate);
    drop(hub);
    let restarted = Hub::open(&database).unwrap();
    let status = restarted.delegation_status(&status_request).unwrap();
    assert_eq!(status.decisions.len(), 1);
    assert_eq!(status.reports.len(), 1);
    assert!(status.advisory_gaps.is_empty());
    assert!(status.advisory);
    assert!(!status.executable);
    assert!(
        restarted
            .export_project(&workspace)
            .unwrap()
            .events
            .iter()
            .any(|event| event.kind == "task_delegation_reported")
    );
    let reassessed = restarted
        .assess_delegation(&DelegationDecisionRequest {
            idempotency_key: "reassess".into(),
            ..decision
        })
        .unwrap();
    // A new plan must not erase the ability to report an already running agent.
    restarted
        .report_delegation(&DelegationReportRequest {
            outcome: DelegationRunOutcome::Completed,
            result_summary: "Worker finished after reassessment".into(),
            idempotency_key: "worker-complete".into(),
            ..report.clone()
        })
        .unwrap();
    for (agent, outcome) in [
        ("worker-a", DelegationRunOutcome::Failed),
        ("worker-b", DelegationRunOutcome::Completed),
    ] {
        restarted
            .report_delegation(&DelegationReportRequest {
                decision_id: reassessed.decision.decision_id.clone(),
                host_agent_id: agent.into(),
                outcome,
                result_summary: format!("{agent} result"),
                idempotency_key: format!("{agent}-result"),
                ..report.clone()
            })
            .unwrap();
    }
    assert!(
        restarted
            .delegation_status(&status_request)
            .unwrap()
            .advisory_gaps
            .contains(&"worker_host_execution_reported_failed".to_owned())
    );
}

#[test]
fn required_dimensions_need_delegate_or_skip() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Assess delegation".into(),
            idempotency_key: "open".into(),
        })
        .unwrap()
        .session_id;
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id,
            objective: "Do work".into(),
            acceptance_criteria: vec!["Done".into()],
            write_scope: vec![],
            depends_on: vec![],
            idempotency_key: "task".into(),
        })
        .unwrap()
        .task
        .task_id;
    let decision = DelegationDecisionRequest {
        task_id,
        parallel_paths: 2,
        material_change: true,
        worker: choice(DelegationDisposition::NotRequired, "Not assessed"),
        reviewer: choice(DelegationDisposition::NotRequired, "Not assessed"),
        idempotency_key: "invalid".into(),
    };
    assert!(hub.assess_delegation(&decision).is_err());
    assert!(
        hub.assess_delegation(&DelegationDecisionRequest {
            worker: choice(DelegationDisposition::Skip, "Paths overlap in one file"),
            reviewer: choice(
                DelegationDisposition::Skip,
                "No independent reviewer available"
            ),
            idempotency_key: "skip".into(),
            ..decision
        })
        .is_ok()
    );
}
