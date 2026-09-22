use aporic::codex::{
    GatePolicy, PostToolUseInput, SessionEndInput, SessionSource, SessionStartInput,
    claim_checkpoint_transaction, post_tool_use_transaction, publish_checkpoint_transaction,
    session_start_output,
};
use aporic::{
    ActionOutcome, Actor, ActorKind, CommitRequest, CommitStatus, EpistemicStatus, Event,
    EvidenceKind, SCHEMA_VERSION, VerificationResult, commit, initialize, load, migrate_v2_to_v5,
};
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "aporic-v050-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn request(revision: u64, id: &str, event: Event) -> CommitRequest {
    CommitRequest {
        schema_version: SCHEMA_VERSION,
        event_id: id.into(),
        idempotency_key: id.into(),
        expected_revision: revision,
        actor: Actor {
            kind: ActorKind::Human,
            id: "tester".into(),
            provenance: "v050-test".into(),
        },
        scope: "repo".into(),
        event,
    }
}

#[test]
fn observed_claim_rejects_inference_only_evidence() {
    let store = path("observed").join("events.jsonl");
    initialize(&store).unwrap();
    assert_eq!(
        commit(
            &store,
            request(
                0,
                "e1",
                Event::EvidenceRecorded {
                    evidence_id: "inference".into(),
                    kind: EvidenceKind::AgentInference,
                    locator: "agent:reasoning".into(),
                    digest: None,
                },
            ),
        )
        .unwrap()
        .status,
        CommitStatus::Committed
    );
    let outcome = commit(
        &store,
        request(
            1,
            "c1",
            Event::ClaimRecorded {
                claim_id: "claim".into(),
                statement: "This was directly observed".into(),
                status: EpistemicStatus::Observed,
                evidence_refs: vec!["inference".into()],
            },
        ),
    )
    .unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(
        outcome.evaluation.reason_code,
        "OBSERVATION_EVIDENCE_REQUIRED"
    );
}

#[test]
fn plan_completion_requires_latest_passing_verification() {
    let store = path("verification").join("events.jsonl");
    initialize(&store).unwrap();
    commit(
        &store,
        request(
            0,
            "plan",
            Event::PlanRegistered {
                plan_id: "plan".into(),
                objective: "prove completion".into(),
                acceptance_checks: vec!["tests pass".into()],
                unresolved_questions: vec![],
                intent_id: None,
            },
        ),
    )
    .unwrap();
    let rejected = commit(
        &store,
        request(
            1,
            "early",
            Event::PlanCompleted {
                plan_id: "plan".into(),
                residual_risk_refs: vec![],
            },
        ),
    )
    .unwrap();
    assert_eq!(
        rejected.evaluation.reason_code,
        "PLAN_VERIFICATION_INCOMPLETE"
    );
    commit(
        &store,
        request(
            1,
            "inference-evidence",
            Event::EvidenceRecorded {
                evidence_id: "agent-guess".into(),
                kind: EvidenceKind::AgentInference,
                locator: "agent:reasoning".into(),
                digest: None,
            },
        ),
    )
    .unwrap();
    let inferred_verification = commit(
        &store,
        request(
            2,
            "inferred-verify",
            Event::VerificationRecorded {
                verification_id: "inferred-verify".into(),
                plan_id: "plan".into(),
                check_index: 0,
                result: VerificationResult::Passed,
                evidence_refs: vec!["agent-guess".into()],
            },
        ),
    )
    .unwrap();
    assert_eq!(
        inferred_verification.evaluation.reason_code,
        "DIRECT_EVIDENCE_REQUIRED"
    );
    commit(
        &store,
        request(
            2,
            "test-evidence",
            Event::EvidenceRecorded {
                evidence_id: "test-result".into(),
                kind: EvidenceKind::TestResult,
                locator: "cargo test --test v050".into(),
                digest: None,
            },
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            3,
            "verify",
            Event::VerificationRecorded {
                verification_id: "verify".into(),
                plan_id: "plan".into(),
                check_index: 0,
                result: VerificationResult::Passed,
                evidence_refs: vec!["test-result".into()],
            },
        ),
    )
    .unwrap();
    assert_eq!(
        commit(
            &store,
            request(
                4,
                "complete",
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
fn post_tool_use_records_bounded_idempotent_effect_receipts_without_inferring_success() {
    let store = path("post-tool").join("events.jsonl");
    initialize(&store).unwrap();
    let input = PostToolUseInput {
        session_id: "s1".into(),
        hook_event_name: "PostToolUse".into(),
        cwd: "/repo".into(),
        turn_id: "t1".into(),
        tool_name: "apply_patch".into(),
        tool_use_id: "u1".into(),
        tool_input: json!({"patch": "..."}),
        tool_response: json!({"tool_specific": true}),
        model: None,
        permission_mode: None,
    };
    post_tool_use_transaction(&store, &input, "repo").unwrap();
    post_tool_use_transaction(&store, &input, "repo").unwrap();
    let original_receipt = load(&store)
        .unwrap()
        .state()
        .effect_receipts
        .values()
        .next()
        .unwrap()
        .clone();
    let mut conflicting_response = input.clone();
    conflicting_response.tool_response = json!({"tool_specific": false});
    post_tool_use_transaction(&store, &conflicting_response, "repo").unwrap();
    let retained_receipt = load(&store)
        .unwrap()
        .state()
        .effect_receipts
        .values()
        .next()
        .unwrap()
        .clone();
    assert_eq!(retained_receipt, original_receipt);
    let mut conflicting = input.clone();
    conflicting.session_id = "s2".into();
    post_tool_use_transaction(&store, &conflicting, "repo").unwrap();
    post_tool_use_transaction(&store, &input, "other-scope").unwrap();
    let state = load(&store).unwrap();
    assert_eq!(state.state().revision, 3);
    assert_eq!(state.state().effect_receipts.len(), 3);
    assert!(
        state
            .state()
            .effect_receipts
            .values()
            .all(|receipt| receipt.outcome == ActionOutcome::Unknown)
    );
    let stored = std::fs::read_to_string(&store).unwrap();
    assert!(!stored.contains("tool_specific"));
    assert!(!stored.contains("..."));
}

#[test]
fn checkpoint_is_published_and_claimed_once_by_the_next_session() {
    let store = path("checkpoint").join("events.jsonl");
    initialize(&store).unwrap();
    let end = SessionEndInput {
        session_id: "z-older".into(),
        hook_event_name: "SessionEnd".into(),
        cwd: "/repo".into(),
        reason: "other".into(),
    };
    publish_checkpoint_transaction(&store, &end, "repo").unwrap();
    let newer_end = SessionEndInput {
        session_id: "a-newer".into(),
        hook_event_name: "SessionEnd".into(),
        cwd: "/repo".into(),
        reason: "other".into(),
    };
    publish_checkpoint_transaction(&store, &newer_end, "repo").unwrap();
    let start = SessionStartInput {
        session_id: "new".into(),
        hook_event_name: "SessionStart".into(),
        cwd: "/repo".into(),
        source: SessionSource::Startup,
        model: None,
        permission_mode: None,
    };
    claim_checkpoint_transaction(&store, &start, "repo").unwrap();
    claim_checkpoint_transaction(&store, &start, "repo").unwrap();
    let state = load(&store).unwrap();
    assert_eq!(state.state().revision, 3);
    assert_eq!(
        state.state().checkpoints["checkpoint:a-newer"]
            .claimed_by_session
            .as_deref(),
        Some("new")
    );
    assert_eq!(
        state.state().checkpoints["checkpoint:z-older"].state,
        aporic::CheckpointState::Open
    );
    let output = session_start_output(
        state.state(),
        &start,
        "repo",
        &GatePolicy::new("apply_patch", false).unwrap(),
        6_000,
    )
    .unwrap();
    assert!(
        output
            .hook_specific_output
            .additional_context
            .contains("checkpoint:a-newer")
    );
}

#[test]
fn schema_two_logs_migrate_to_schema_five_without_mutating_source() {
    let source = path("migration").join("events-v2.jsonl");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    let record = json!({
        "sequence": 1,
        "schema_version": 2,
        "event_id": "hold",
        "idempotency_key": "hold",
        "expected_revision": 0,
        "actor": {"kind": "human", "id": "user", "provenance": "test"},
        "scope": "repo",
        "event": {"type": "tool_hold_placed", "tool_hold_id": "h", "tool_name": "apply_patch", "reason": "pause"}
    });
    let bytes = format!("{record}\n");
    std::fs::write(&source, &bytes).unwrap();
    let destination = source.parent().unwrap().join("events-v3.jsonl");
    let outcome = migrate_v2_to_v5(&source, &destination).unwrap();
    assert_eq!(outcome.from_schema, 2);
    assert_eq!(outcome.to_schema, 5);
    assert_eq!(std::fs::read_to_string(source).unwrap(), bytes);
    assert_eq!(load(destination).unwrap().state().revision, 1);
}
