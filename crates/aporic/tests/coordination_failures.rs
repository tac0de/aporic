use aporic::{
    Hub,
    domain::{
        CriterionEvidence, OpenRequest, TaskCancelRequest, TaskClaimRequest, TaskCompleteRequest,
        TaskCreateRequest, TaskStatus,
    },
};
use rusqlite::Connection;
use serde_json::json;

#[test]
fn deterministic_coordination_failure_suite() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Coordinate independent implementation work".to_owned(),
            idempotency_key: "coordination-session".to_owned(),
        })
        .unwrap()
        .session_id;

    let foundation = create_task(
        &hub,
        &session_id,
        "Implement the durable task schema",
        &["Migration reaches schema version 3"],
        &["crates/aporic/src/store.rs"],
        &[],
        "create-foundation",
    );
    let dependent = create_task(
        &hub,
        &session_id,
        "Expose task coordination over MCP",
        &["All task tools are listed by an MCP client"],
        &["crates/aporic/src/mcp.rs"],
        std::slice::from_ref(&foundation),
        "create-dependent",
    );
    let dependency_gate = claim(&hub, &dependent, "worker-b", "claim-dependent-early").is_err();

    let conflicting = create_task(
        &hub,
        &session_id,
        "Refactor the Aporic crate",
        &["The crate compiles"],
        &["crates/aporic"],
        &[],
        "create-conflicting",
    );
    claim(&hub, &foundation, "worker-a", "claim-foundation").unwrap();
    let overlapping_scope_blocked =
        claim(&hub, &conflicting, "worker-c", "claim-conflicting").is_err();
    let cancelled_without_completion = hub
        .cancel_task(&TaskCancelRequest {
            task_id: conflicting,
            reason: "Write scope conflicts with the foundation task".to_owned(),
            idempotency_key: "cancel-conflicting".to_owned(),
        })
        .unwrap()
        .task
        .status
        == TaskStatus::Cancelled;

    let unsupported_completion_rejected = hub
        .complete_task(&TaskCompleteRequest {
            task_id: foundation.clone(),
            worker_id: "worker-a".to_owned(),
            outcome_summary: "Schema implemented".to_owned(),
            criterion_evidence: Vec::new(),
            idempotency_key: "complete-without-evidence".to_owned(),
        })
        .is_err();
    hub.complete_task(&TaskCompleteRequest {
        task_id: foundation.clone(),
        worker_id: "worker-a".to_owned(),
        outcome_summary: "Schema implemented and migrated".to_owned(),
        criterion_evidence: vec![CriterionEvidence {
            criterion: "Migration reaches schema version 3".to_owned(),
            evidence: "Migration integration test observed user_version=3".to_owned(),
        }],
        idempotency_key: "complete-foundation".to_owned(),
    })
    .unwrap();

    claim(&hub, &dependent, "worker-b", "claim-dependent").unwrap();
    let duplicate_objective_rejected = hub
        .create_task(&TaskCreateRequest {
            session_id: session_id.clone(),
            objective: " expose TASK coordination over mcp ".to_owned(),
            acceptance_criteria: vec!["Duplicate must not exist".to_owned()],
            write_scope: vec![],
            depends_on: vec![],
            idempotency_key: "duplicate-task-objective".to_owned(),
        })
        .is_err();

    Connection::open(&database)
        .unwrap()
        .execute(
            "UPDATE tasks SET lease_expires_at_unix_ms = 0 WHERE task_id = ?1",
            [&dependent],
        )
        .unwrap();
    let expired_lease_reclaimed = claim(&hub, &dependent, "worker-d", "reclaim-dependent").is_ok();

    let checks = [
        dependency_gate,
        overlapping_scope_blocked,
        cancelled_without_completion,
        unsupported_completion_rejected,
        duplicate_objective_rejected,
        expired_lease_reclaimed,
    ];
    let hardened_score = checks.iter().filter(|passed| **passed).count();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "suite": "coordination_failure_regressions_v1",
            "baseline_score": 0,
            "hardened_score": hardened_score,
            "max_score": checks.len(),
            "checks": {
                "dependency_gate": dependency_gate,
                "overlapping_scope_blocked": overlapping_scope_blocked,
                "cancelled_without_completion": cancelled_without_completion,
                "unsupported_completion_rejected": unsupported_completion_rejected,
                "duplicate_objective_rejected": duplicate_objective_rejected,
                "expired_lease_reclaimed": expired_lease_reclaimed
            }
        }))
        .unwrap()
    );
    assert_eq!(hardened_score, checks.len());
}

fn create_task(
    hub: &Hub,
    session_id: &str,
    objective: &str,
    criteria: &[&str],
    scope: &[&str],
    dependencies: &[String],
    key: &str,
) -> String {
    hub.create_task(&TaskCreateRequest {
        session_id: session_id.to_owned(),
        objective: objective.to_owned(),
        acceptance_criteria: criteria.iter().map(|value| (*value).to_owned()).collect(),
        write_scope: scope.iter().map(|value| (*value).to_owned()).collect(),
        depends_on: dependencies.to_vec(),
        idempotency_key: key.to_owned(),
    })
    .unwrap()
    .task
    .task_id
}

fn claim(
    hub: &Hub,
    task_id: &str,
    worker_id: &str,
    key: &str,
) -> aporic::store::Result<aporic::domain::TaskOutcome> {
    hub.claim_task(&TaskClaimRequest {
        task_id: task_id.to_owned(),
        worker_id: worker_id.to_owned(),
        lease_seconds: 300,
        idempotency_key: key.to_owned(),
    })
}
