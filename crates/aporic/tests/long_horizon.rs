use aporic::{
    Hub,
    domain::{
        CloseDisposition, CloseRequest, CriterionEvidence, OpenRequest, RecallRequest, RecordKind,
        RecordRequest, TaskClaimRequest, TaskCompleteRequest, TaskCreateRequest, TaskListRequest,
        TaskStatus,
    },
};
use serde_json::json;

#[test]
fn long_horizon_frontier_workload_does_not_accumulate_known_failures() {
    const REVISIONS: usize = 30;
    const TASKS: usize = 24;
    let area = tempfile::tempdir().unwrap();
    let workspace_path = area.path().join("workspace");
    std::fs::create_dir(&workspace_path).unwrap();
    let workspace = workspace_path.to_string_lossy().into_owned();
    let database = area.path().join("aporic.sqlite3");
    let mut hub = Hub::open(&database).unwrap();
    let mut previous_decision = None;
    let mut restart_count = 0;

    for revision in 0..REVISIONS {
        let opened = hub
            .open_session(&OpenRequest {
                workspace: workspace.clone(),
                objective: format!("Revise the operating decision to revision {revision}"),
                idempotency_key: format!("open-revision-{revision}"),
            })
            .unwrap();
        let decision = hub
            .record(&RecordRequest {
                session_id: opened.session_id.clone(),
                kind: RecordKind::Decision,
                content: format!("The active operating decision is revision {revision}."),
                evidence: Some(format!("Synthetic revision evidence {revision}")),
                supersedes_record_id: previous_decision,
                verifies_effect_id: None,
                idempotency_key: format!("record-revision-{revision}"),
            })
            .unwrap()
            .record;
        previous_decision = Some(decision.record_id);
        hub.close_session(&CloseRequest {
            session_id: opened.session_id,
            disposition: CloseDisposition::Completed,
            summary: format!("Revision {revision} recorded"),
            next_action: None,
            idempotency_key: format!("close-revision-{revision}"),
        })
        .unwrap();
        if revision % 5 == 4 {
            drop(hub);
            hub = Hub::open(&database).unwrap();
            restart_count += 1;
        }
    }

    let capsule = hub
        .recall(&RecallRequest {
            workspace: workspace.clone(),
            limit: Some(100),
        })
        .unwrap();
    let active_decisions = capsule
        .recent_records
        .iter()
        .filter(|record| record.kind == RecordKind::Decision)
        .collect::<Vec<_>>();
    assert_eq!(active_decisions.len(), 1);
    assert!(active_decisions[0].content.contains("revision 29"));

    let coordination = hub
        .open_session(&OpenRequest {
            workspace: workspace.clone(),
            objective: "Run the long-horizon coordination workload".to_owned(),
            idempotency_key: "open-long-coordination".to_owned(),
        })
        .unwrap();
    let mut duplicate_rejections = 0;
    let mut premature_completion_rejections = 0;
    for index in 0..TASKS {
        let objective = format!("Implement synthetic task {index}");
        let criterion = format!("Synthetic task {index} passes its deterministic check");
        let created = hub
            .create_task(&TaskCreateRequest {
                session_id: coordination.session_id.clone(),
                objective: objective.clone(),
                acceptance_criteria: vec![criterion.clone()],
                write_scope: vec!["crates/aporic/src".to_owned()],
                depends_on: Vec::new(),
                idempotency_key: format!("create-long-task-{index}"),
            })
            .unwrap();
        if hub
            .create_task(&TaskCreateRequest {
                session_id: coordination.session_id.clone(),
                objective: objective.to_uppercase(),
                acceptance_criteria: vec!["Duplicate should be rejected".to_owned()],
                write_scope: Vec::new(),
                depends_on: Vec::new(),
                idempotency_key: format!("duplicate-long-task-{index}"),
            })
            .is_err()
        {
            duplicate_rejections += 1;
        }
        let task_id = created.task.task_id;
        hub.claim_task(&TaskClaimRequest {
            task_id: task_id.clone(),
            worker_id: format!("worker-{}", index % 4),
            lease_seconds: 300,
            idempotency_key: format!("claim-long-task-{index}"),
        })
        .unwrap();
        if hub
            .complete_task(&TaskCompleteRequest {
                task_id: task_id.clone(),
                worker_id: format!("worker-{}", index % 4),
                outcome_summary: "Unsupported completion attempt".to_owned(),
                criterion_evidence: Vec::new(),
                idempotency_key: format!("premature-long-task-{index}"),
            })
            .is_err()
        {
            premature_completion_rejections += 1;
        }
        hub.complete_task(&TaskCompleteRequest {
            task_id,
            worker_id: format!("worker-{}", index % 4),
            outcome_summary: format!("Synthetic task {index} completed"),
            criterion_evidence: vec![CriterionEvidence {
                criterion,
                evidence: format!("Deterministic check {index} returned true"),
            }],
            idempotency_key: format!("complete-long-task-{index}"),
        })
        .unwrap();
    }
    hub.close_session(&CloseRequest {
        session_id: coordination.session_id,
        disposition: CloseDisposition::Completed,
        summary: "Long-horizon coordination workload completed".to_owned(),
        next_action: None,
        idempotency_key: "close-long-coordination".to_owned(),
    })
    .unwrap();

    let tasks = hub
        .list_tasks(&TaskListRequest {
            workspace,
            limit: Some(200),
        })
        .unwrap();
    let completed_tasks = tasks
        .iter()
        .filter(|task| task.status == TaskStatus::Completed)
        .count();
    let prevented_failures =
        (REVISIONS - 1) + duplicate_rejections + premature_completion_rejections;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "suite": "long_horizon_frontier_workload_v1",
            "decision_revisions": REVISIONS,
            "active_decisions_after_recall": active_decisions.len(),
            "process_restarts": restart_count,
            "tasks_completed": completed_tasks,
            "duplicate_attempts_rejected": duplicate_rejections,
            "unsupported_completions_rejected": premature_completion_rejections,
            "known_failures_prevented": prevented_failures
        }))
        .unwrap()
    );
    assert_eq!(completed_tasks, TASKS);
    assert_eq!(duplicate_rejections, TASKS);
    assert_eq!(premature_completion_rejections, TASKS);
    assert_eq!(prevented_failures, 77);
}
