use std::{path::Path, process::Command};

use aporic::{
    Hub,
    domain::{
        ActualTaskOutcome, AdvisoryDisposition, AdvisoryMode, AdvisoryRoleKind,
        AdvisoryRoleReportRequest, AdvisorySourceKind, ClaimRequest, ClaimStatus, CriterionProof,
        EvidenceKind, EvidenceRequest, GitObserveRequest, OpenRequest,
        OrchestrationRunCreateRequest, OrchestrationRunGetRequest, OrchestrationRunListRequest,
        PredictedTaskOutcome, RoleAppointmentCreateRequest, RuntimeTraceListRequest,
        ShadowEvaluationRequest, TaskCancelRequest, TaskClaimRequest, TaskCompleteRequest,
        TaskCreateRequest, workspace_file_claim,
    },
    hook::handle_codex_hook,
};
use serde_json::json;
use sha2::{Digest, Sha256};

fn git(workspace: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(workspace)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

struct Fixture {
    area: tempfile::TempDir,
    workspace: std::path::PathBuf,
    hub: Hub,
    session_id: String,
    snapshot_id: String,
    exposure_id: String,
    task_id: String,
    criterion: String,
}

fn fixture() -> Fixture {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    git(&workspace, &["init", "-b", "main"]);
    std::fs::write(workspace.join("proof.txt"), "verified outcome\n").unwrap();
    git(&workspace, &["add", "."]);
    git(
        &workspace,
        &[
            "-c",
            "user.name=Aporic Test",
            "-c",
            "user.email=aporic@example.invalid",
            "commit",
            "-m",
            "initial",
        ],
    );
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Exercise zero-effect advisory orchestration".to_owned(),
            idempotency_key: "open-orchestration".to_owned(),
        })
        .unwrap()
        .session_id;
    handle_codex_hook(
        &hub,
        &json!({
            "hook_event_name": "UserPromptSubmit",
            "cwd": workspace,
            "session_id": "host-session",
            "turn_id": "host-turn",
            "prompt": "Evaluate the shadow role"
        })
        .to_string(),
    );
    let exposure_id = hub
        .list_runtime_events(&RuntimeTraceListRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            limit: None,
            event_kinds: Vec::new(),
        })
        .unwrap()[0]
        .exposure_id
        .clone()
        .unwrap();
    let snapshot_id = hub
        .observe_git(&GitObserveRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            base_ref: None,
        })
        .unwrap()
        .snapshot_id;
    let proof_path = workspace.join("proof.txt");
    let digest = format!("{:x}", Sha256::digest(std::fs::read(&proof_path).unwrap()));
    let criterion = workspace_file_claim(proof_path.to_string_lossy().as_ref(), &digest);
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id: session_id.clone(),
            objective: "Produce a directly verified result".to_owned(),
            acceptance_criteria: vec![criterion.clone()],
            write_scope: Vec::new(),
            depends_on: Vec::new(),
            idempotency_key: "create-shadow-task".to_owned(),
        })
        .unwrap()
        .task
        .task_id;
    Fixture {
        area,
        workspace,
        hub,
        session_id,
        snapshot_id,
        exposure_id,
        task_id,
        criterion,
    }
}

fn create_run(fixture: &Fixture, mode: AdvisoryMode, key: &str) -> String {
    fixture
        .hub
        .create_orchestration_run(&OrchestrationRunCreateRequest {
            session_id: fixture.session_id.clone(),
            task_id: fixture.task_id.clone(),
            git_snapshot_id: fixture.snapshot_id.clone(),
            context_exposure_id: fixture.exposure_id.clone(),
            mode,
            role_kind: AdvisoryRoleKind::Worker,
            role_id: "worker.implementation".to_owned(),
            role_appointment_id: None,
            objective: "Predict whether the bound task will complete".to_owned(),
            model: Some("gpt-6-sol".to_owned()),
            reasoning_effort: Some("high".to_owned()),
            max_input_tokens: 20_000,
            max_output_tokens: 4_000,
            max_duration_seconds: 600,
            idempotency_key: key.to_owned(),
        })
        .unwrap()
        .view
        .run
        .run_id
}

#[test]
fn hermes_run_binds_matching_role_appointment_without_granting_authority() {
    let fixture = fixture();
    let appointment = fixture
        .hub
        .create_role_appointment(&RoleAppointmentCreateRequest {
            session_id: fixture.session_id.clone(),
            task_id: Some(fixture.task_id.clone()),
            role_id: "delivery.worker".to_owned(),
            role_version: 1,
            assignee_id: "agent.sol".to_owned(),
            model_hint: Some("gpt-6-sol".to_owned()),
            capability_refs: vec![],
            idempotency_key: "appoint-hermes-worker".to_owned(),
        })
        .unwrap()
        .appointment;
    let request = OrchestrationRunCreateRequest {
        session_id: fixture.session_id.clone(),
        task_id: fixture.task_id.clone(),
        git_snapshot_id: fixture.snapshot_id.clone(),
        context_exposure_id: fixture.exposure_id.clone(),
        mode: AdvisoryMode::VisibleAdvisory,
        role_kind: AdvisoryRoleKind::Worker,
        role_id: "worker.implementation".to_owned(),
        role_appointment_id: Some(appointment.appointment_id.clone()),
        objective: "Evaluate a bounded worker report".to_owned(),
        model: Some("gpt-6-sol".to_owned()),
        reasoning_effort: Some("medium".to_owned()),
        max_input_tokens: 10_000,
        max_output_tokens: 2_000,
        max_duration_seconds: 300,
        idempotency_key: "linked-run".to_owned(),
    };
    let run = fixture
        .hub
        .create_orchestration_run(&request)
        .unwrap()
        .view
        .run;
    assert_eq!(run.role_appointment_id, Some(appointment.appointment_id));
    assert!(!run.executable);
    assert!(fixture.hub.audit_orchestration().unwrap().consistent);
    let mismatched = OrchestrationRunCreateRequest {
        model: Some("gpt-6-astra".to_owned()),
        idempotency_key: "mismatched-model".to_owned(),
        ..request
    };
    assert!(fixture.hub.create_orchestration_run(&mismatched).is_err());
}

fn report(fixture: &Fixture, run_id: &str, key: &str) -> AdvisoryRoleReportRequest {
    AdvisoryRoleReportRequest {
        run_id: run_id.to_owned(),
        source_kind: AdvisorySourceKind::HostReported,
        disposition: AdvisoryDisposition::Recommend,
        predicted_task_outcome: PredictedTaskOutcome::Completion,
        summary: "The bounded task is likely to complete.".to_owned(),
        public_rationale: "The report addresses the declared acceptance criterion.".to_owned(),
        recommended_next_action: Some("Run the declared verification path.".to_owned()),
        uncertainties: vec!["The role output is not evidence.".to_owned()],
        addressed_criteria: vec![fixture.criterion.clone()],
        reported_input_tokens: Some(2_000),
        reported_output_tokens: Some(500),
        reported_duration_ms: Some(5_000),
        claims_task_complete: false,
        idempotency_key: key.to_owned(),
    }
}

#[test]
fn migrates_v13_to_v14_without_orchestration_state() {
    let area = tempfile::tempdir().unwrap();
    let database = area.path().join("aporic.sqlite3");
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute_batch(include_str!("../../../migrations/0001_initial.sql"))
        .unwrap();
    for migration in [
        include_str!("../../../migrations/0006_authority_bound_context.sql"),
        include_str!("../../../migrations/0007_memory_lifecycle.sql"),
        include_str!("../../../migrations/0008_runtime_trace.sql"),
        include_str!("../../../migrations/0009_git_governance.sql"),
        include_str!("../../../migrations/0010_token_efficiency.sql"),
        include_str!("../../../migrations/0011_commit_bound_deliberation.sql"),
        include_str!("../../../migrations/0012_secure_capability_fabric.sql"),
        include_str!("../../../migrations/0013_execution_governance.sql"),
    ] {
        connection.execute_batch(migration).unwrap();
    }
    drop(connection);
    let hub = Hub::open(database).unwrap();
    assert_eq!(hub.stats().unwrap().schema_version, 19);
    let audit = hub.audit_orchestration().unwrap();
    assert_eq!(audit.run_count, 0);
    assert!(audit.consistent);
}

#[test]
fn blind_shadow_stays_sealed_until_verified_task_outcome() {
    let fixture = fixture();
    let run_id = create_run(&fixture, AdvisoryMode::BlindShadow, "create-blind");
    let submitted = fixture
        .hub
        .submit_advisory_role_report(&report(&fixture, &run_id, "submit-blind"))
        .unwrap();
    assert!(!submitted.view.report_revealed);
    assert!(submitted.view.report.is_none());
    let sealed_export = fixture
        .hub
        .export_project(fixture.workspace.to_string_lossy().as_ref())
        .unwrap();
    assert_eq!(sealed_export.sealed_advisory_report_count, 1);
    assert!(sealed_export.advisory_role_reports.is_empty());
    let sealed_events = serde_json::to_string(&sealed_export.events).unwrap();
    assert!(!sealed_events.contains("Recommend the bounded next step"));
    assert!(!sealed_events.contains("Public evidence and uncertainty only"));
    assert!(
        fixture
            .hub
            .evaluate_shadow_run(&ShadowEvaluationRequest {
                run_id: run_id.clone(),
                idempotency_key: "evaluate-too-early".to_owned(),
            })
            .unwrap_err()
            .to_string()
            .contains("completed or cancelled")
    );

    let evidence = fixture
        .hub
        .add_evidence(&EvidenceRequest {
            session_id: fixture.session_id.clone(),
            kind: EvidenceKind::WorkspaceFile,
            locator: fixture
                .workspace
                .join("proof.txt")
                .to_string_lossy()
                .into_owned(),
            summary: "Direct task proof".to_owned(),
            content_sha256: None,
            idempotency_key: "proof-evidence".to_owned(),
        })
        .unwrap()
        .evidence;
    let claim = fixture
        .hub
        .assert_claim(&ClaimRequest {
            session_id: fixture.session_id.clone(),
            status: ClaimStatus::Verified,
            statement: fixture.criterion.clone(),
            material: true,
            evidence_ids: vec![evidence.evidence_id],
            supersedes_claim_id: None,
            idempotency_key: "proof-claim".to_owned(),
        })
        .unwrap()
        .claim;
    fixture
        .hub
        .claim_task(&TaskClaimRequest {
            task_id: fixture.task_id.clone(),
            worker_id: "real-worker".to_owned(),
            lease_seconds: 300,
            idempotency_key: "claim-real-task".to_owned(),
        })
        .unwrap();
    fixture
        .hub
        .complete_task(&TaskCompleteRequest {
            task_id: fixture.task_id.clone(),
            worker_id: "real-worker".to_owned(),
            outcome_summary: "Verified independently of the shadow report.".to_owned(),
            criterion_proofs: vec![CriterionProof {
                criterion: fixture.criterion.clone(),
                verified_claim_id: claim.claim_id,
            }],
            idempotency_key: "complete-real-task".to_owned(),
        })
        .unwrap();
    let evaluated = fixture
        .hub
        .evaluate_shadow_run(&ShadowEvaluationRequest {
            run_id: run_id.clone(),
            idempotency_key: "evaluate-blind".to_owned(),
        })
        .unwrap();
    assert!(evaluated.view.report_revealed);
    assert!(evaluated.view.report.is_some());
    let evaluation = evaluated.view.evaluation.unwrap();
    assert_eq!(evaluation.actual_task_outcome, ActualTaskOutcome::Completed);
    assert_eq!(evaluation.prediction_match, Some(true));
    assert!(evaluation.evidence_eligible);
    assert!(!evaluation.git_stale);
    assert!(fixture.hub.audit_orchestration().unwrap().consistent);
    let exported = fixture
        .hub
        .export_project(fixture.workspace.to_string_lossy().as_ref())
        .unwrap();
    assert_eq!(exported.format_version, 15);
    assert_eq!(exported.orchestration_runs.len(), 1);
    assert_eq!(exported.sealed_advisory_report_count, 0);
    assert_eq!(exported.advisory_role_reports.len(), 1);
    assert_eq!(exported.shadow_evaluations.len(), 1);
}

#[test]
fn rejects_false_completion_budget_overrun_and_late_reports() {
    let fixture = fixture();
    let run_id = create_run(&fixture, AdvisoryMode::VisibleAdvisory, "create-visible");
    let mut false_completion = report(&fixture, &run_id, "false-completion");
    false_completion.claims_task_complete = true;
    assert!(
        fixture
            .hub
            .submit_advisory_role_report(&false_completion)
            .unwrap_err()
            .to_string()
            .contains("cannot claim task completion")
    );
    let mut over_budget = report(&fixture, &run_id, "over-budget");
    over_budget.reported_output_tokens = Some(4_001);
    assert!(
        fixture
            .hub
            .submit_advisory_role_report(&over_budget)
            .unwrap_err()
            .to_string()
            .contains("exceeds")
    );
    fixture
        .hub
        .cancel_task(&TaskCancelRequest {
            task_id: fixture.task_id.clone(),
            reason: "A real host cancelled the task.".to_owned(),
            idempotency_key: "cancel-real-task".to_owned(),
        })
        .unwrap();
    assert!(
        fixture
            .hub
            .submit_advisory_role_report(&report(&fixture, &run_id, "late-report"))
            .unwrap_err()
            .to_string()
            .contains("before the bound task")
    );
}

#[test]
fn visible_advisory_is_readable_but_cancelled_outcome_is_not_verified_success() {
    let fixture = fixture();
    let run_id = create_run(
        &fixture,
        AdvisoryMode::VisibleAdvisory,
        "create-visible-two",
    );
    let submitted = fixture
        .hub
        .submit_advisory_role_report(&report(&fixture, &run_id, "submit-visible"))
        .unwrap();
    assert!(submitted.view.report_revealed);
    assert!(submitted.view.report.is_some());
    fixture
        .hub
        .cancel_task(&TaskCancelRequest {
            task_id: fixture.task_id.clone(),
            reason: "Cancelled outside Hermes.".to_owned(),
            idempotency_key: "cancel-visible-task".to_owned(),
        })
        .unwrap();
    let evaluated = fixture
        .hub
        .evaluate_shadow_run(&ShadowEvaluationRequest {
            run_id: run_id.clone(),
            idempotency_key: "evaluate-visible".to_owned(),
        })
        .unwrap();
    let evaluation = evaluated.view.evaluation.unwrap();
    assert_eq!(evaluation.actual_task_outcome, ActualTaskOutcome::Cancelled);
    assert_eq!(evaluation.prediction_match, Some(false));
    assert!(!evaluation.evidence_eligible);
    let read = fixture
        .hub
        .get_orchestration_run(&OrchestrationRunGetRequest {
            workspace: fixture.workspace.to_string_lossy().into_owned(),
            run_id,
        })
        .unwrap();
    assert!(read.report.is_some());
    let listed = fixture
        .hub
        .list_orchestration_runs(&OrchestrationRunListRequest {
            workspace: fixture.workspace.to_string_lossy().into_owned(),
            limit: None,
        })
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].evidence_eligible, Some(false));
}

#[test]
fn audit_detects_tampered_advisory_state() {
    let fixture = fixture();
    let run_id = create_run(&fixture, AdvisoryMode::VisibleAdvisory, "create-tamper");
    fixture
        .hub
        .submit_advisory_role_report(&report(&fixture, &run_id, "submit-tamper"))
        .unwrap();
    rusqlite::Connection::open(fixture.area.path().join("aporic.sqlite3"))
        .unwrap()
        .execute(
            "UPDATE advisory_role_reports SET summary = 'tampered' WHERE run_id = ?1",
            [&run_id],
        )
        .unwrap();
    let audit = fixture.hub.audit_orchestration().unwrap();
    assert_eq!(audit.digest_mismatch_count, 1);
    assert!(!audit.consistent);
}
