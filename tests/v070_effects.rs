use aporic::codex::{
    GatePolicy, PostToolUseInput, SessionSource, SessionStartInput, post_tool_use_transaction,
    session_start_output,
};
use aporic::verifier::{VerifierReportInput, ingest_verifier_report};
use aporic::{
    Actor, ActorKind, CommitRequest, CommitStatus, Event, EvidenceKind, SCHEMA_VERSION,
    VerificationResult, commit, initialize, load,
};
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "aporic-v070-effects-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn request(
    revision: u64,
    id: &str,
    scope: &str,
    actor_kind: ActorKind,
    event: Event,
) -> CommitRequest {
    CommitRequest {
        schema_version: SCHEMA_VERSION,
        event_id: id.into(),
        idempotency_key: id.into(),
        expected_revision: revision,
        actor: Actor {
            kind: actor_kind,
            id: "tester".into(),
            provenance: "v070-effect-test".into(),
        },
        scope: scope.into(),
        event,
    }
}

fn post_input() -> PostToolUseInput {
    PostToolUseInput {
        session_id: "session-1".into(),
        hook_event_name: "PostToolUse".into(),
        cwd: "/workspace/repo".into(),
        turn_id: "turn-1".into(),
        tool_name: "apply_patch".into(),
        tool_use_id: "use-1".into(),
        tool_input: json!({"patch": "sensitive raw input"}),
        tool_response: json!({"result": "sensitive raw response"}),
        model: None,
        permission_mode: None,
    }
}

fn report(
    verification_id: &str,
    receipt_id: &str,
    result: VerificationResult,
    evidence_ref: &str,
) -> VerifierReportInput {
    VerifierReportInput {
        schema_version: 1,
        verification_id: verification_id.into(),
        receipt_id: receipt_id.into(),
        verifier_id: "repository-state-verifier".into(),
        provenance: "independent-test-adapter".into(),
        plan_id: "plan".into(),
        check_index: 0,
        result,
        evidence_refs: vec![evidence_ref.into()],
    }
}

#[test]
fn effect_verifier_requires_exact_receipt_scope_actor_and_direct_evidence() {
    let store = path("boundaries").join("events.jsonl");
    initialize(&store).unwrap();
    commit(
        &store,
        request(
            0,
            "plan",
            "repo",
            ActorKind::Human,
            Event::PlanRegistered {
                plan_id: "plan".into(),
                objective: "verify an effect".into(),
                acceptance_checks: vec!["repository has the intended state".into()],
                unresolved_questions: vec![],
                intent_id: None,
            },
        ),
    )
    .unwrap();
    post_tool_use_transaction(&store, &post_input(), "repo").unwrap();
    let receipt_id = load(&store)
        .unwrap()
        .state()
        .effect_receipts
        .values()
        .next()
        .unwrap()
        .id
        .clone();
    commit(
        &store,
        request(
            2,
            "inference",
            "repo",
            ActorKind::Agent,
            Event::EvidenceRecorded {
                evidence_id: "inference".into(),
                kind: EvidenceKind::AgentInference,
                locator: "agent:reasoning".into(),
                digest: None,
            },
        ),
    )
    .unwrap();

    assert!(
        ingest_verifier_report(
            &store,
            "repo",
            &report(
                "inference-report",
                &receipt_id,
                VerificationResult::Passed,
                "inference",
            ),
        )
        .is_err()
    );
    assert!(
        ingest_verifier_report(
            &store,
            "other",
            &report(
                "cross-scope-report",
                &receipt_id,
                VerificationResult::Passed,
                "inference",
            ),
        )
        .is_err()
    );
    commit(
        &store,
        request(
            3,
            "direct-evidence",
            "repo",
            ActorKind::Evidence,
            Event::EvidenceRecorded {
                evidence_id: "direct-evidence".into(),
                kind: EvidenceKind::RepositoryState,
                locator: "git:worktree".into(),
                digest: None,
            },
        ),
    )
    .unwrap();

    let unauthorized = commit(
        &store,
        request(
            4,
            "human-verifier",
            "repo",
            ActorKind::Human,
            Event::EffectVerificationRecorded {
                verification_id: "human-verifier".into(),
                receipt_id,
                verifier_id: "tester".into(),
                plan_id: "plan".into(),
                check_index: 0,
                result: VerificationResult::Inconclusive,
                evidence_refs: vec!["direct-evidence".into()],
            },
        ),
    )
    .unwrap();
    assert_eq!(unauthorized.status, CommitStatus::Rejected);
    assert_eq!(
        unauthorized.evaluation.reason_code,
        "INDEPENDENT_VERIFIER_REQUIRED"
    );
}

#[test]
fn latest_effect_verifier_report_controls_completion_and_is_idempotent() {
    let store = path("completion").join("events.jsonl");
    initialize(&store).unwrap();
    commit(
        &store,
        request(
            0,
            "plan",
            "repo",
            ActorKind::Human,
            Event::PlanRegistered {
                plan_id: "plan".into(),
                objective: "verify an effect".into(),
                acceptance_checks: vec!["repository has the intended state".into()],
                unresolved_questions: vec![],
                intent_id: None,
            },
        ),
    )
    .unwrap();
    post_tool_use_transaction(&store, &post_input(), "repo").unwrap();
    let receipt_id = load(&store)
        .unwrap()
        .state()
        .effect_receipts
        .values()
        .next()
        .unwrap()
        .id
        .clone();
    commit(
        &store,
        request(
            2,
            "evidence",
            "repo",
            ActorKind::Evidence,
            Event::EvidenceRecorded {
                evidence_id: "repository-state".into(),
                kind: EvidenceKind::RepositoryState,
                locator: "git:worktree".into(),
                digest: Some("sha256:example".into()),
            },
        ),
    )
    .unwrap();

    let passed = report(
        "passed",
        &receipt_id,
        VerificationResult::Passed,
        "repository-state",
    );
    ingest_verifier_report(&store, "repo", &passed).unwrap();
    ingest_verifier_report(&store, "repo", &passed).unwrap();
    assert_eq!(load(&store).unwrap().state().revision, 4);

    ingest_verifier_report(
        &store,
        "repo",
        &report(
            "failed",
            &receipt_id,
            VerificationResult::Failed,
            "repository-state",
        ),
    )
    .unwrap();
    let incomplete = commit(
        &store,
        request(
            5,
            "incomplete",
            "repo",
            ActorKind::Human,
            Event::PlanCompleted {
                plan_id: "plan".into(),
                residual_risk_refs: vec![],
            },
        ),
    )
    .unwrap();
    assert_eq!(incomplete.status, CommitStatus::Rejected);
    assert_eq!(
        incomplete.evaluation.reason_code,
        "PLAN_VERIFICATION_INCOMPLETE"
    );

    ingest_verifier_report(
        &store,
        "repo",
        &report(
            "passed-again",
            &receipt_id,
            VerificationResult::Passed,
            "repository-state",
        ),
    )
    .unwrap();
    assert_eq!(
        commit(
            &store,
            request(
                6,
                "complete",
                "repo",
                ActorKind::Human,
                Event::PlanCompleted {
                    plan_id: "plan".into(),
                    residual_risk_refs: vec![],
                },
            ),
        )
        .unwrap()
        .status,
        CommitStatus::Committed
    );
}

#[test]
fn projection_exposes_current_session_receipts_and_verifier_reports() {
    let store = path("projection").join("events.jsonl");
    initialize(&store).unwrap();
    commit(
        &store,
        request(
            0,
            "plan",
            "repo",
            ActorKind::Human,
            Event::PlanRegistered {
                plan_id: "plan".into(),
                objective: "verify an effect".into(),
                acceptance_checks: vec!["repository has the intended state".into()],
                unresolved_questions: vec![],
                intent_id: None,
            },
        ),
    )
    .unwrap();
    post_tool_use_transaction(&store, &post_input(), "repo").unwrap();
    let receipt_id = load(&store)
        .unwrap()
        .state()
        .effect_receipts
        .values()
        .next()
        .unwrap()
        .id
        .clone();
    commit(
        &store,
        request(
            2,
            "evidence",
            "repo",
            ActorKind::Evidence,
            Event::EvidenceRecorded {
                evidence_id: "repository-state".into(),
                kind: EvidenceKind::RepositoryState,
                locator: "git:worktree".into(),
                digest: None,
            },
        ),
    )
    .unwrap();
    ingest_verifier_report(
        &store,
        "repo",
        &report(
            "verified",
            &receipt_id,
            VerificationResult::Passed,
            "repository-state",
        ),
    )
    .unwrap();

    let output = session_start_output(
        load(&store).unwrap().state(),
        &SessionStartInput {
            session_id: "session-1".into(),
            hook_event_name: "SessionStart".into(),
            cwd: "/workspace/repo".into(),
            source: SessionSource::Resume,
            model: None,
            permission_mode: None,
        },
        "repo",
        &GatePolicy::new("apply_patch", false).unwrap(),
        6_000,
    )
    .unwrap();
    let context = output.hook_specific_output.additional_context;
    let start = context.find("<aporic-recorded-data>").unwrap() + "<aporic-recorded-data>".len();
    let end = context.find("</aporic-recorded-data>").unwrap();
    let projection: serde_json::Value = serde_json::from_str(&context[start..end]).unwrap();
    assert_eq!(projection["recent_effect_receipts"][0]["id"], receipt_id);
    assert_eq!(
        projection["recent_effect_receipts"][0]["identity_kind"],
        "informational_non_cryptographic"
    );
    assert_eq!(
        projection["recent_effect_verifications"][0]["id"],
        "verified"
    );
    assert_eq!(projection["retained"]["effect_receipts"], 1);
    assert_eq!(projection["retained"]["effect_verifications"], 1);
}
