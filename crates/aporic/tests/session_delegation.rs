use aporic::{
    Hub,
    domain::{
        CloseDisposition, CloseRequest, DelegationChoice, DelegationDecisionRequest,
        DelegationDimension, DelegationDisposition, DelegationRunOutcome, OpenRequest,
        SessionDelegationDecisionRequest, SessionDelegationReportRequest,
        SessionDelegationStatusRequest, TaskCreateRequest,
    },
};

fn choice(disposition: DelegationDisposition, reason: &str) -> DelegationChoice {
    DelegationChoice {
        disposition,
        reason: reason.into(),
    }
}

#[test]
fn event_limit_reserves_terminal_report_for_every_started_agent() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.clone(),
            objective: "Bounded reports".into(),
            idempotency_key: "open".into(),
        })
        .unwrap()
        .session_id;
    let decision_request = SessionDelegationDecisionRequest {
        session_id: session_id.clone(),
        parallel_paths: 2,
        material_change: false,
        worker: choice(DelegationDisposition::Delegate, "Independent runs"),
        reviewer: choice(DelegationDisposition::NotRequired, "No material change"),
        idempotency_key: "decision-1".into(),
    };
    let decision_id = hub
        .assess_session_delegation(&decision_request)
        .unwrap()
        .decision
        .decision_id;
    let mut last_start = None;
    for index in 0..31 {
        let start = SessionDelegationReportRequest {
            session_id: session_id.clone(),
            decision_id: decision_id.clone(),
            dimension: DelegationDimension::Worker,
            host_agent_id: format!("worker-{index}"),
            model: "gpt-6-sol".into(),
            reasoning_effort: "medium".into(),
            outcome: DelegationRunOutcome::Started,
            result_summary: "Started".into(),
            idempotency_key: format!("start-{index}"),
        };
        hub.report_session_delegation(&start).unwrap();
        if index == 30 {
            last_start = Some(start);
        } else {
            hub.report_session_delegation(&SessionDelegationReportRequest {
                outcome: DelegationRunOutcome::Completed,
                result_summary: "Completed".into(),
                idempotency_key: format!("complete-{index}"),
                ..start
            })
            .unwrap();
        }
    }
    let last_start = last_start.unwrap();
    assert!(
        hub.report_session_delegation(&SessionDelegationReportRequest {
            host_agent_id: "worker-extra".into(),
            idempotency_key: "extra-start".into(),
            ..last_start.clone()
        })
        .is_err()
    );
    hub.assess_session_delegation(&SessionDelegationDecisionRequest {
        idempotency_key: "decision-2".into(),
        ..decision_request
    })
    .unwrap();
    hub.report_session_delegation(&SessionDelegationReportRequest {
        outcome: DelegationRunOutcome::Completed,
        result_summary: "Final completion".into(),
        idempotency_key: "final-complete".into(),
        ..last_start
    })
    .unwrap();
    let status = hub
        .session_delegation_status(&SessionDelegationStatusRequest {
            workspace,
            session_id,
        })
        .unwrap();
    assert_eq!(status.decisions.len() + status.reports.len(), 64);
}

#[test]
fn session_assessment_before_task_and_report_lifecycle_replays() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let open = OpenRequest {
        workspace: workspace.clone(),
        objective: "Session delegation".into(),
        idempotency_key: "open-session".into(),
    };
    let opened = hub.open_session(&open).unwrap();
    let session_id = opened.session_id.clone();
    let initial = opened.session_delegation.unwrap();
    assert!(initial.assessment_missing);
    assert_eq!(initial.advisory_gaps, ["delegation_assessment_missing"]);
    assert!(initial.decisions.is_empty());
    assert!(!initial.executable);
    let invalid = SessionDelegationDecisionRequest {
        session_id: session_id.clone(),
        parallel_paths: 2,
        material_change: true,
        worker: choice(DelegationDisposition::NotRequired, "Not assessed"),
        reviewer: choice(DelegationDisposition::NotRequired, "Not assessed"),
        idempotency_key: "invalid".into(),
    };
    assert!(hub.assess_session_delegation(&invalid).is_err());
    let decision_request = SessionDelegationDecisionRequest {
        worker: choice(DelegationDisposition::Delegate, "Independent path"),
        reviewer: choice(DelegationDisposition::Skip, "No reviewer available"),
        idempotency_key: "assess-session".into(),
        ..invalid
    };
    let decision = hub
        .assess_session_delegation(&decision_request)
        .unwrap()
        .decision;
    assert!(
        hub.assess_session_delegation(&decision_request)
            .unwrap()
            .duplicate
    );
    let status_request = SessionDelegationStatusRequest {
        workspace: workspace.clone(),
        session_id: session_id.clone(),
    };
    let planned = hub.session_delegation_status(&status_request).unwrap();
    assert!(!planned.assessment_missing);
    assert!(
        planned
            .advisory_gaps
            .contains(&"worker_host_execution_not_reported".into())
    );
    assert!(planned.reports.is_empty());
    let report = SessionDelegationReportRequest {
        session_id: session_id.clone(),
        decision_id: decision.decision_id.clone(),
        dimension: DelegationDimension::Worker,
        host_agent_id: "host-worker".into(),
        model: "gpt-6-sol".into(),
        reasoning_effort: "medium".into(),
        outcome: DelegationRunOutcome::Started,
        result_summary: "Host started worker".into(),
        idempotency_key: "worker-start".into(),
    };
    assert!(
        hub.report_session_delegation(&SessionDelegationReportRequest {
            outcome: DelegationRunOutcome::Completed,
            idempotency_key: "premature-end".into(),
            ..report.clone()
        })
        .is_err()
    );
    hub.report_session_delegation(&report).unwrap();
    assert!(hub.report_session_delegation(&report).unwrap().duplicate);
    let started = hub.session_delegation_status(&status_request).unwrap();
    assert_eq!(started.reports.len(), 1);
    assert!(
        started
            .advisory_gaps
            .contains(&"worker_completion_not_reported".into())
    );
    assert!(
        hub.open_session(&open)
            .unwrap()
            .session_delegation
            .unwrap()
            .reports
            .len()
            == 1
    );
    hub.close_session(&CloseRequest {
        session_id: session_id.clone(),
        disposition: CloseDisposition::Completed,
        summary: "Host worker still running".into(),
        next_action: None,
        idempotency_key: "close-session".into(),
    })
    .unwrap();
    assert!(
        hub.assess_session_delegation(&SessionDelegationDecisionRequest {
            idempotency_key: "late-assessment".into(),
            ..decision_request
        })
        .is_err()
    );
    assert!(
        hub.report_session_delegation(&SessionDelegationReportRequest {
            host_agent_id: "late-agent".into(),
            idempotency_key: "late-start".into(),
            ..report.clone()
        })
        .is_err()
    );
    hub.report_session_delegation(&SessionDelegationReportRequest {
        outcome: DelegationRunOutcome::Completed,
        result_summary: "Host reports completion after close".into(),
        idempotency_key: "worker-complete".into(),
        ..report.clone()
    })
    .unwrap();
    assert!(
        hub.report_session_delegation(&SessionDelegationReportRequest {
            outcome: DelegationRunOutcome::Failed,
            result_summary: "Conflicting second terminal outcome".into(),
            idempotency_key: "worker-fail".into(),
            ..report
        })
        .is_err()
    );
    drop(hub);
    let restarted = Hub::open(&database).unwrap();
    let status = restarted
        .session_delegation_status(&status_request)
        .unwrap();
    assert_eq!(status.decisions.len(), 1);
    assert_eq!(status.reports.len(), 2);
    assert!(status.advisory_gaps.is_empty());
    assert!(
        restarted
            .export_project(&workspace)
            .unwrap()
            .events
            .iter()
            .any(|event| event.kind == "session_delegation_reported")
    );
}

#[test]
fn task_assessment_covers_session_assessment_gap() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let open = OpenRequest {
        workspace: workspace.clone(),
        objective: "Task covers assessment".into(),
        idempotency_key: "open".into(),
    };
    let session_id = hub.open_session(&open).unwrap().session_id;
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id: session_id.clone(),
            objective: "Existing task workflow".into(),
            acceptance_criteria: vec!["Done".into()],
            write_scope: vec![],
            depends_on: vec![],
            idempotency_key: "task".into(),
        })
        .unwrap()
        .task
        .task_id;
    hub.assess_delegation(&DelegationDecisionRequest {
        task_id,
        parallel_paths: 1,
        material_change: false,
        worker: choice(DelegationDisposition::NotRequired, "One path"),
        reviewer: choice(DelegationDisposition::NotRequired, "No material change"),
        idempotency_key: "task-assess".into(),
    })
    .unwrap();
    let status = hub.open_session(&open).unwrap().session_delegation.unwrap();
    assert!(!status.assessment_missing);
    assert_eq!(status.task_assessment_count, 1);
    assert!(status.decisions.is_empty());
    assert!(status.advisory_gaps.is_empty());
}

#[test]
fn duplicate_open_reads_legacy_payload_and_current_session_status() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let database = area.path().join("aporic.sqlite3");
    let open = OpenRequest {
        workspace,
        objective: "Legacy open payload".into(),
        idempotency_key: "legacy-open".into(),
    };
    let hub = Hub::open(&database).unwrap();
    let session_id = hub.open_session(&open).unwrap().session_id;
    drop(hub);
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute(
            "UPDATE events SET result_json = json_remove(result_json, '$.session_delegation')
         WHERE kind = 'session_opened' AND stream_id = ?1",
            [&session_id],
        )
        .unwrap();
    drop(connection);
    let restarted = Hub::open(&database).unwrap();
    let duplicate = restarted.open_session(&open).unwrap();
    assert!(duplicate.duplicate);
    assert_eq!(duplicate.session_id, session_id);
    assert!(duplicate.session_delegation.unwrap().assessment_missing);
}

#[test]
fn session_required_dimensions_follow_typed_inputs() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Dimension rules".into(),
            idempotency_key: "open".into(),
        })
        .unwrap()
        .session_id;
    let request = SessionDelegationDecisionRequest {
        session_id,
        parallel_paths: 2,
        material_change: false,
        worker: choice(DelegationDisposition::NotRequired, "Not assessed"),
        reviewer: choice(DelegationDisposition::NotRequired, "No material change"),
        idempotency_key: "parallel-invalid".into(),
    };
    assert!(hub.assess_session_delegation(&request).is_err());
    hub.assess_session_delegation(&SessionDelegationDecisionRequest {
        worker: choice(DelegationDisposition::Skip, "Paths overlap"),
        idempotency_key: "parallel-skip".into(),
        ..request.clone()
    })
    .unwrap();
    assert!(
        hub.assess_session_delegation(&SessionDelegationDecisionRequest {
            parallel_paths: 1,
            material_change: true,
            worker: choice(DelegationDisposition::NotRequired, "One path"),
            idempotency_key: "material-invalid".into(),
            ..request.clone()
        })
        .is_err()
    );
    hub.assess_session_delegation(&SessionDelegationDecisionRequest {
        parallel_paths: 1,
        material_change: true,
        worker: choice(DelegationDisposition::NotRequired, "One path"),
        reviewer: choice(DelegationDisposition::Skip, "Reviewer unavailable"),
        idempotency_key: "material-skip".into(),
        ..request
    })
    .unwrap();
}
