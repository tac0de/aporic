use std::path::PathBuf;

use aporic::{
    Hub,
    domain::{
        AccountabilityListRequest, AccountabilityOpenRequest, AccountabilityPlanRequest,
        AccountabilityResolveRequest, AccountabilityStatus, ClaimRequest, ClaimStatus,
        CriterionProof, EvidenceGrade, EvidenceKind, EvidenceRequest, OpenRequest,
        TaskClaimRequest, TaskCompleteRequest, TaskCreateRequest, workspace_file_claim,
    },
};

fn setup() -> (tempfile::TempDir, PathBuf, Hub, String, String, String) {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Track one repair obligation".to_owned(),
            idempotency_key: "open".to_owned(),
        })
        .unwrap()
        .session_id;
    let source_task_id = hub
        .create_task(&TaskCreateRequest {
            session_id: session_id.clone(),
            objective: "Original delivery".to_owned(),
            acceptance_criteria: vec!["Original task check".to_owned()],
            write_scope: vec!["src".to_owned()],
            depends_on: vec![],
            idempotency_key: "source-task".to_owned(),
        })
        .unwrap()
        .task
        .task_id;
    let evidence_id = hub
        .add_evidence(&EvidenceRequest {
            session_id: session_id.clone(),
            kind: EvidenceKind::UserStatement,
            locator: "user-report-1".to_owned(),
            summary: "A reported user-visible mistake".to_owned(),
            content_sha256: Some("a".repeat(64)),
            idempotency_key: "failure-evidence".to_owned(),
        })
        .unwrap()
        .evidence
        .evidence_id;
    (
        area,
        workspace,
        hub,
        session_id,
        source_task_id,
        evidence_id,
    )
}

fn case_request(
    session_id: &str,
    source_task_id: &str,
    evidence_id: &str,
) -> AccountabilityOpenRequest {
    AccountabilityOpenRequest {
        session_id: session_id.to_owned(),
        source_task_id: source_task_id.to_owned(),
        evidence_id: evidence_id.to_owned(),
        reported_assignee_id: Some("agent.worker".to_owned()),
        expected_behavior: "The task result should preserve the user's data".to_owned(),
        observed_behavior: "The reported result omitted one field".to_owned(),
        impact: "The user had to repeat the work".to_owned(),
        idempotency_key: "case-open".to_owned(),
    }
}

#[test]
fn repair_obligation_remains_visible_until_a_verified_repair_task_completes() {
    let (area, workspace, hub, session_id, source_task_id, evidence_id) = setup();
    let request = case_request(&session_id, &source_task_id, &evidence_id);
    let opened = hub.open_accountability_case(&request).unwrap();
    assert_eq!(opened.case.status, AccountabilityStatus::Open);
    assert_eq!(opened.case.evidence_grade, EvidenceGrade::Reported);
    assert!(opened.case.advisory);
    assert!(hub.open_accountability_case(&request).unwrap().duplicate);
    assert!(
        hub.resolve_accountability_case(&AccountabilityResolveRequest {
            case_id: opened.case.case_id.clone(),
            idempotency_key: "too-early".to_owned(),
        })
        .is_err()
    );

    let next_open = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Another work session".to_owned(),
            idempotency_key: "next-open".to_owned(),
        })
        .unwrap();
    assert_eq!(next_open.open_repair_count, 1);
    assert_eq!(
        next_open.open_repair_obligations[0].case_id,
        opened.case.case_id
    );

    let repaired_file = workspace.join("repaired.txt");
    std::fs::write(&repaired_file, "fixed artifact\n").unwrap();
    let direct = hub
        .add_evidence(&EvidenceRequest {
            session_id: session_id.clone(),
            kind: EvidenceKind::WorkspaceFile,
            locator: repaired_file.to_string_lossy().into_owned(),
            summary: "Repair artifact read by Aporic".to_owned(),
            content_sha256: None,
            idempotency_key: "repair-evidence".to_owned(),
        })
        .unwrap()
        .evidence;
    let criterion = workspace_file_claim(&direct.locator, &direct.content_sha256);
    let claim_id = hub
        .assert_claim(&ClaimRequest {
            session_id: session_id.clone(),
            status: ClaimStatus::Verified,
            statement: criterion.clone(),
            material: true,
            evidence_ids: vec![direct.evidence_id],
            supersedes_claim_id: None,
            idempotency_key: "repair-claim".to_owned(),
        })
        .unwrap()
        .claim
        .claim_id;
    let repair_task_id = hub
        .create_task(&TaskCreateRequest {
            session_id: session_id.clone(),
            objective: "Repair omitted field".to_owned(),
            acceptance_criteria: vec![criterion.clone()],
            write_scope: vec!["repair".to_owned()],
            depends_on: vec![],
            idempotency_key: "repair-task".to_owned(),
        })
        .unwrap()
        .task
        .task_id;
    let first_plan = hub
        .plan_accountability_repair(&AccountabilityPlanRequest {
            case_id: opened.case.case_id.clone(),
            repair_task_id: repair_task_id.clone(),
            root_cause_hypothesis: "Serialization omitted the field".to_owned(),
            prevention_change: "Check the round trip".to_owned(),
            idempotency_key: "first-plan".to_owned(),
        })
        .unwrap();
    assert_eq!(first_plan.case.plan_revision, 1);
    let second_plan = hub
        .plan_accountability_repair(&AccountabilityPlanRequest {
            case_id: opened.case.case_id.clone(),
            repair_task_id: repair_task_id.clone(),
            root_cause_hypothesis: "Serialization omitted the field".to_owned(),
            prevention_change: "Check the round trip and error path".to_owned(),
            idempotency_key: "second-plan".to_owned(),
        })
        .unwrap();
    assert_eq!(second_plan.case.plan_revision, 2);
    assert!(
        hub.resolve_accountability_case(&AccountabilityResolveRequest {
            case_id: opened.case.case_id.clone(),
            idempotency_key: "still-early".to_owned(),
        })
        .is_err()
    );

    hub.claim_task(&TaskClaimRequest {
        task_id: repair_task_id.clone(),
        worker_id: "agent.worker".to_owned(),
        lease_seconds: 60,
        idempotency_key: "repair-claim-task".to_owned(),
    })
    .unwrap();
    hub.complete_task(&TaskCompleteRequest {
        task_id: repair_task_id.clone(),
        worker_id: "agent.worker".to_owned(),
        outcome_summary: "Repair artifact verified".to_owned(),
        criterion_proofs: vec![CriterionProof {
            criterion,
            verified_claim_id: claim_id,
        }],
        idempotency_key: "repair-complete".to_owned(),
    })
    .unwrap();
    let resolve_request = AccountabilityResolveRequest {
        case_id: opened.case.case_id.clone(),
        idempotency_key: "case-resolve".to_owned(),
    };
    let repaired = hub.resolve_accountability_case(&resolve_request).unwrap();
    assert_eq!(repaired.case.status, AccountabilityStatus::Repaired);
    assert!(
        hub.resolve_accountability_case(&resolve_request)
            .unwrap()
            .duplicate
    );
    assert!(
        hub.plan_accountability_repair(&AccountabilityPlanRequest {
            case_id: opened.case.case_id.clone(),
            repair_task_id: repair_task_id.clone(),
            root_cause_hypothesis: "revised after closure".to_owned(),
            prevention_change: "rewrite history".to_owned(),
            idempotency_key: "late-plan".to_owned(),
        })
        .is_err()
    );

    drop(hub);
    let restarted = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    assert_eq!(restarted.stats().unwrap().schema_version, 24);
    let report = restarted
        .list_accountability_cases(&AccountabilityListRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            reported_assignee_id: Some("agent.worker".to_owned()),
            limit: None,
        })
        .unwrap();
    assert_eq!((report.open_count, report.repaired_count), (0, 1));
    assert_eq!(report.cases[0].plan_revision, 2);
    assert!(report.advisory);
    let export = restarted
        .export_project(workspace.to_str().unwrap())
        .unwrap();
    assert_eq!(export.format_version, 20);
    assert_eq!(export.accountability_cases.len(), 1);
    assert_eq!(
        export
            .events
            .iter()
            .filter(|event| event.kind == "accountability_repair_planned")
            .count(),
        2
    );
    assert!(restarted.audit_accountability().unwrap().consistent);
    rusqlite::Connection::open(area.path().join("aporic.sqlite3"))
        .unwrap()
        .execute(
            "UPDATE accountability_cases SET prevention_change = 'forged' WHERE case_id = ?1",
            [&opened.case.case_id],
        )
        .unwrap();
    assert!(!restarted.audit_accountability().unwrap().consistent);
}

#[test]
fn rejects_cross_workspace_repair_and_detects_case_tampering() {
    let (area, workspace, hub, session_id, source_task_id, evidence_id) = setup();
    let case = hub
        .open_accountability_case(&case_request(&session_id, &source_task_id, &evidence_id))
        .unwrap()
        .case;
    let other = area.path().join("other");
    std::fs::create_dir(&other).unwrap();
    let other_session = hub
        .open_session(&OpenRequest {
            workspace: other.to_string_lossy().into_owned(),
            objective: "Other workspace".to_owned(),
            idempotency_key: "other-open".to_owned(),
        })
        .unwrap()
        .session_id;
    let other_task = hub
        .create_task(&TaskCreateRequest {
            session_id: other_session,
            objective: "Unrelated repair".to_owned(),
            acceptance_criteria: vec!["proof".to_owned()],
            write_scope: vec!["src".to_owned()],
            depends_on: vec![],
            idempotency_key: "other-task".to_owned(),
        })
        .unwrap()
        .task
        .task_id;
    assert!(
        hub.plan_accountability_repair(&AccountabilityPlanRequest {
            case_id: case.case_id.clone(),
            repair_task_id: other_task,
            root_cause_hypothesis: "unknown".to_owned(),
            prevention_change: "wrong workspace".to_owned(),
            idempotency_key: "bad-plan".to_owned(),
        })
        .is_err()
    );
    assert_eq!(
        hub.list_accountability_cases(&AccountabilityListRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            reported_assignee_id: None,
            limit: None,
        })
        .unwrap()
        .open_count,
        1
    );

    let database = area.path().join("aporic.sqlite3");
    let connection = rusqlite::Connection::open(database).unwrap();
    connection
        .execute(
            "UPDATE accountability_cases SET observed_behavior = 'rewritten' WHERE case_id = ?1",
            [&case.case_id],
        )
        .unwrap();
    assert!(!hub.audit_accountability().unwrap().consistent);
    connection
        .execute(
            "UPDATE accountability_cases SET observed_behavior = ?1 WHERE case_id = ?2",
            rusqlite::params![case.observed_behavior, case.case_id],
        )
        .unwrap();
    assert!(hub.audit_accountability().unwrap().consistent);
    connection
        .execute(
            "DELETE FROM events WHERE stream_id = ?1 AND kind = 'accountability_case_opened'",
            [&case.case_id],
        )
        .unwrap();
    assert!(!hub.audit_accountability().unwrap().consistent);
}

#[test]
fn upgrades_schema_18_to_accountability_schema() {
    let area = tempfile::tempdir().unwrap();
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();
    drop(hub);
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute_batch(
            "DROP TABLE prompt_trials;
            DROP TABLE task_brief_receipts;
            DROP TABLE government_terms;
            DROP TABLE government_roster_state;
            DROP TABLE government_people;
            DROP TABLE task_memory_uses;
            DROP TABLE prototype_reviews; DROP TABLE prototype_briefs;
            DROP TABLE improvement_requests; DROP TABLE accountability_cases;
            PRAGMA user_version = 18;",
        )
        .unwrap();
    drop(connection);
    let upgraded = Hub::open(&database).unwrap();
    assert_eq!(upgraded.stats().unwrap().schema_version, 24);
    assert!(upgraded.audit_accountability().unwrap().consistent);
}
