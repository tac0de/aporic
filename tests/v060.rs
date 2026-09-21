use aporic::analysis::{
    AnalyzerInput, DEFAULT_MEMORY_BYTES, MAX_ANALYZER_MODULE_BYTES, MAX_FUEL, MAX_MEMORY_BYTES,
    MAX_TABLE_ELEMENTS, run_wasm_analyzer,
};
use aporic::codex::{GatePolicy, SessionSource, SessionStartInput, session_start_output};
use aporic::{
    Actor, ActorKind, ArgumentRelationKind, BeliefRevision, CommitRequest, CommitStatus,
    DecisionReview, DecisionReviewOutcome, EpistemicStatus, Event, EvidenceKind, SCHEMA_VERSION,
    State, commit, initialize, load, migrate_v3_to_v4,
};
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "aporic-v060-{label}-{}-{}",
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
            provenance: "v060-test".into(),
        },
        scope: "repo".into(),
        event,
    }
}

fn record_claim(store: &PathBuf, revision: u64, id: &str, status: EpistemicStatus) {
    commit(
        store,
        request(
            revision,
            id,
            Event::ClaimRecorded {
                claim_id: id.into(),
                statement: format!("statement for {id}"),
                status,
                evidence_refs: vec![],
            },
        ),
    )
    .unwrap();
}

#[test]
fn argument_relations_require_distinct_active_same_scope_claims() {
    let store = path("relations").join("events.jsonl");
    initialize(&store).unwrap();
    record_claim(&store, 0, "premise", EpistemicStatus::Hypothesized);
    record_claim(&store, 1, "conclusion", EpistemicStatus::Inferred);

    assert_eq!(
        commit(
            &store,
            request(
                2,
                "relation",
                Event::ArgumentRelationRecorded {
                    relation_id: "r1".into(),
                    source_claim_id: "conclusion".into(),
                    target_claim_id: "premise".into(),
                    kind: ArgumentRelationKind::DependsOn,
                },
            ),
        )
        .unwrap()
        .status,
        CommitStatus::Committed
    );
    let duplicate = commit(
        &store,
        request(
            3,
            "duplicate",
            Event::ArgumentRelationRecorded {
                relation_id: "r2".into(),
                source_claim_id: "conclusion".into(),
                target_claim_id: "premise".into(),
                kind: ArgumentRelationKind::DependsOn,
            },
        ),
    )
    .unwrap();
    assert_eq!(
        duplicate.evaluation.reason_code,
        "DUPLICATE_ARGUMENT_RELATION"
    );
    commit(
        &store,
        request(
            3,
            "evidence",
            Event::EvidenceRecorded {
                evidence_id: "refutation".into(),
                kind: EvidenceKind::TestResult,
                locator: "focused contradiction".into(),
                digest: None,
            },
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            4,
            "refute-premise",
            Event::BeliefRevisionRecorded {
                revision_id: "refute-premise".into(),
                claim_id: "premise".into(),
                prior_status: EpistemicStatus::Hypothesized,
                revised_status: EpistemicStatus::Refuted,
                trigger_claim_ids: vec![],
                evidence_refs: vec!["refutation".into()],
                rationale: "test refuted the premise".into(),
            },
        ),
    )
    .unwrap();
    let refuted_endpoint = commit(
        &store,
        request(
            5,
            "refuted-endpoint",
            Event::ArgumentRelationRecorded {
                relation_id: "r3".into(),
                source_claim_id: "conclusion".into(),
                target_claim_id: "premise".into(),
                kind: ArgumentRelationKind::Attacks,
            },
        ),
    )
    .unwrap();
    assert_eq!(
        refuted_endpoint.evaluation.reason_code,
        "INVALID_ARGUMENT_CLAIM_REFERENCE"
    );
    record_claim(&store, 5, "stale", EpistemicStatus::Inferred);
    commit(
        &store,
        request(
            6,
            "stale-claim",
            Event::BeliefRevisionRecorded {
                revision_id: "stale-claim".into(),
                claim_id: "stale".into(),
                prior_status: EpistemicStatus::Inferred,
                revised_status: EpistemicStatus::Stale,
                trigger_claim_ids: vec![],
                evidence_refs: vec!["refutation".into()],
                rationale: "evidence is no longer current".into(),
            },
        ),
    )
    .unwrap();
    let stale_endpoint = commit(
        &store,
        request(
            7,
            "stale-endpoint",
            Event::ArgumentRelationRecorded {
                relation_id: "r4".into(),
                source_claim_id: "stale".into(),
                target_claim_id: "conclusion".into(),
                kind: ArgumentRelationKind::Supports,
            },
        ),
    )
    .unwrap();
    assert_eq!(
        stale_endpoint.evaluation.reason_code,
        "INVALID_ARGUMENT_CLAIM_REFERENCE"
    );
}

#[test]
fn belief_revision_is_stale_safe_and_requires_direct_evidence_for_refutation() {
    let store = path("revision").join("events.jsonl");
    initialize(&store).unwrap();
    record_claim(&store, 0, "belief", EpistemicStatus::Hypothesized);
    commit(
        &store,
        request(
            1,
            "evidence",
            Event::EvidenceRecorded {
                evidence_id: "test".into(),
                kind: EvidenceKind::TestResult,
                locator: "cargo test --test v060".into(),
                digest: None,
            },
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            2,
            "revise",
            Event::BeliefRevisionRecorded {
                revision_id: "br1".into(),
                claim_id: "belief".into(),
                prior_status: EpistemicStatus::Hypothesized,
                revised_status: EpistemicStatus::Refuted,
                trigger_claim_ids: vec![],
                evidence_refs: vec!["test".into()],
                rationale: "focused test contradicted the hypothesis".into(),
            },
        ),
    )
    .unwrap();
    let stale = commit(
        &store,
        request(
            3,
            "stale",
            Event::BeliefRevisionRecorded {
                revision_id: "br2".into(),
                claim_id: "belief".into(),
                prior_status: EpistemicStatus::Hypothesized,
                revised_status: EpistemicStatus::Observed,
                trigger_claim_ids: vec![],
                evidence_refs: vec!["test".into()],
                rationale: "stale writer".into(),
            },
        ),
    )
    .unwrap();
    assert_eq!(stale.evaluation.reason_code, "STALE_BELIEF_REVISION");
}

#[test]
fn decision_review_requires_direct_evidence() {
    let store = path("review").join("events.jsonl");
    initialize(&store).unwrap();
    commit(
        &store,
        request(
            0,
            "inference",
            Event::EvidenceRecorded {
                evidence_id: "guess".into(),
                kind: EvidenceKind::AgentInference,
                locator: "agent:reasoning".into(),
                digest: None,
            },
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            1,
            "decision",
            Event::DecisionCommitted {
                decision_id: "d1".into(),
                question: "Which runtime?".into(),
                value: "wasmtime".into(),
                authority_ref: None,
            },
        ),
    )
    .unwrap();
    let no_basis = commit(
        &store,
        request(
            2,
            "review-without-basis",
            Event::DecisionReviewRecorded {
                review_id: "review-without-basis".into(),
                decision_id: "d1".into(),
                outcome: DecisionReviewOutcome::Inconclusive,
                evidence_refs: vec!["guess".into()],
                lessons: vec!["record the original basis".into()],
                follow_up_claim_ids: vec![],
            },
        ),
    )
    .unwrap();
    assert_eq!(no_basis.evaluation.reason_code, "DECISION_BASIS_REQUIRED");
    commit(
        &store,
        request(
            2,
            "basis",
            Event::DecisionBasisLinked {
                decision_id: "d1".into(),
                claim_ids: vec![],
                evidence_refs: vec!["guess".into()],
            },
        ),
    )
    .unwrap();
    let review = commit(
        &store,
        request(
            3,
            "review",
            Event::DecisionReviewRecorded {
                review_id: "review-1".into(),
                decision_id: "d1".into(),
                outcome: DecisionReviewOutcome::Inconclusive,
                evidence_refs: vec!["guess".into()],
                lessons: vec!["measure runtime cost".into()],
                follow_up_claim_ids: vec![],
            },
        ),
    )
    .unwrap();
    assert_eq!(review.evaluation.reason_code, "DIRECT_EVIDENCE_REQUIRED");
}

#[test]
fn decision_basis_rejects_post_decision_references() {
    let store = path("post-decision-basis").join("events.jsonl");
    initialize(&store).unwrap();
    commit(
        &store,
        request(
            0,
            "decision",
            Event::DecisionCommitted {
                decision_id: "d1".into(),
                question: "Which runtime?".into(),
                value: "wasmtime".into(),
                authority_ref: None,
            },
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            1,
            "outcome",
            Event::EvidenceRecorded {
                evidence_id: "post-outcome".into(),
                kind: EvidenceKind::TestResult,
                locator: "test:post-outcome".into(),
                digest: None,
            },
        ),
    )
    .unwrap();
    let result = commit(
        &store,
        request(
            2,
            "backfilled-basis",
            Event::DecisionBasisLinked {
                decision_id: "d1".into(),
                claim_ids: vec![],
                evidence_refs: vec!["post-outcome".into()],
            },
        ),
    )
    .unwrap();
    assert_eq!(
        result.evaluation.reason_code,
        "NONCONTEMPORANEOUS_DECISION_BASIS"
    );
}

#[test]
fn projection_exposes_argument_revision_and_review_records() {
    let store = path("projection").join("events.jsonl");
    initialize(&store).unwrap();
    record_claim(&store, 0, "premise", EpistemicStatus::Hypothesized);
    record_claim(&store, 1, "conclusion", EpistemicStatus::Inferred);
    commit(
        &store,
        request(
            2,
            "relation",
            Event::ArgumentRelationRecorded {
                relation_id: "depends".into(),
                source_claim_id: "conclusion".into(),
                target_claim_id: "premise".into(),
                kind: ArgumentRelationKind::DependsOn,
            },
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            3,
            "evidence",
            Event::EvidenceRecorded {
                evidence_id: "measurement".into(),
                kind: EvidenceKind::TestResult,
                locator: "focused measurement".into(),
                digest: None,
            },
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            4,
            "revision",
            Event::BeliefRevisionRecorded {
                revision_id: "revision-1".into(),
                claim_id: "premise".into(),
                prior_status: EpistemicStatus::Hypothesized,
                revised_status: EpistemicStatus::Observed,
                trigger_claim_ids: vec![],
                evidence_refs: vec!["measurement".into()],
                rationale: "measurement replaced the assumption".into(),
            },
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            5,
            "decision",
            Event::DecisionCommitted {
                decision_id: "decision-1".into(),
                question: "Proceed?".into(),
                value: "yes".into(),
                authority_ref: None,
            },
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            6,
            "basis",
            Event::DecisionBasisLinked {
                decision_id: "decision-1".into(),
                claim_ids: vec!["premise".into()],
                evidence_refs: vec!["measurement".into()],
            },
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            7,
            "review",
            Event::DecisionReviewRecorded {
                review_id: "review-1".into(),
                decision_id: "decision-1".into(),
                outcome: DecisionReviewOutcome::Confirmed,
                evidence_refs: vec!["measurement".into()],
                lessons: vec!["measure fragile premises before commitment".into()],
                follow_up_claim_ids: vec![],
            },
        ),
    )
    .unwrap();

    let state = load(&store).unwrap();
    let output = session_start_output(
        state.state(),
        &SessionStartInput {
            session_id: "session".into(),
            hook_event_name: "SessionStart".into(),
            cwd: "/repo".into(),
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
    assert!(context.contains("depends"));
    assert!(context.contains("revision-1"));
    assert!(context.contains("review-1"));
}

#[test]
fn projection_orders_by_sequence_and_accounts_for_more_than_eight_records() {
    let mut state = State::default();
    for sequence in 1..=9 {
        let id = if sequence == 9 {
            "a-newest".to_owned()
        } else {
            format!("z-older-{sequence}")
        };
        state.belief_revisions.insert(
            id.clone(),
            BeliefRevision {
                id,
                claim_id: format!("claim-{sequence}"),
                prior_status: EpistemicStatus::Hypothesized,
                revised_status: EpistemicStatus::Inferred,
                trigger_claim_ids: vec![format!("trigger-{sequence}")],
                evidence_refs: vec![],
                rationale: "short".into(),
                scope: "repo".into(),
                sequence,
            },
        );
    }
    state.decision_reviews.insert(
        "z-old-review".into(),
        DecisionReview {
            id: "z-old-review".into(),
            decision_id: "decision".into(),
            outcome: DecisionReviewOutcome::Inconclusive,
            evidence_refs: vec!["evidence".into()],
            lessons: vec!["old".into()],
            follow_up_claim_ids: vec![],
            scope: "repo".into(),
            sequence: 10,
        },
    );
    state.decision_reviews.insert(
        "a-new-review".into(),
        DecisionReview {
            id: "a-new-review".into(),
            decision_id: "decision".into(),
            outcome: DecisionReviewOutcome::Confirmed,
            evidence_refs: vec!["evidence".into()],
            lessons: vec!["new".into()],
            follow_up_claim_ids: vec![],
            scope: "repo".into(),
            sequence: 11,
        },
    );
    let output = session_start_output(
        &state,
        &SessionStartInput {
            session_id: "session".into(),
            hook_event_name: "SessionStart".into(),
            cwd: "/repo".into(),
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
    let data: serde_json::Value = serde_json::from_str(&context[start..end]).unwrap();
    assert_eq!(data["recent_belief_revisions"].as_array().unwrap().len(), 9);
    assert_eq!(data["recent_belief_revisions"][0]["id"], "a-newest");
    assert_eq!(data["recent_decision_reviews"][0]["id"], "a-new-review");
    assert_eq!(data["complete"], true);
    assert_eq!(data["omitted"]["belief_revisions"], 0);
}

#[test]
fn wasm_runtime_rejects_imports_and_exhausts_fuel_without_mutating_state() {
    let input = AnalyzerInput {
        schema_version: 1,
        revision: 0,
        claims: vec![],
        relations: vec![],
        decision_bases: vec![],
    };
    let import_module = path("import.wat");
    std::fs::write(
        &import_module,
        r#"(module (import "host" "capability" (func)) (memory (export "memory") 1) (func (export "alloc") (param i32) (result i32) i32.const 0) (func (export "analyze") (param i32 i32) (result i64) i64.const 0))"#,
    )
    .unwrap();
    let error =
        run_wasm_analyzer(&import_module, &input, 10_000, DEFAULT_MEMORY_BYTES).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("must not import host capabilities")
    );

    let loop_module = path("loop.wat");
    std::fs::write(
        &loop_module,
        r#"(module (memory (export "memory") 1) (func (export "alloc") (param i32) (result i32) i32.const 0) (func (export "analyze") (param i32 i32) (result i64) (loop br 0) i64.const 0))"#,
    )
    .unwrap();
    let error = run_wasm_analyzer(&loop_module, &input, 100, DEFAULT_MEMORY_BYTES).unwrap_err();
    assert!(error.to_string().contains("Wasm analyzer failure"));

    let oversized_module = path("oversized.wasm");
    std::fs::write(&oversized_module, vec![0_u8; MAX_ANALYZER_MODULE_BYTES + 1]).unwrap();
    let error =
        run_wasm_analyzer(&oversized_module, &input, 10_000, DEFAULT_MEMORY_BYTES).unwrap_err();
    assert!(error.to_string().contains("module exceeds byte limit"));

    let large_table_module = path("large-table.wat");
    std::fs::write(
        &large_table_module,
        format!(
            r#"(module (table {} funcref) (memory (export "memory") 1) (func (export "alloc") (param i32) (result i32) i32.const 0) (func (export "analyze") (param i32 i32) (result i64) i64.const 0))"#,
            MAX_TABLE_ELEMENTS + 1
        ),
    )
    .unwrap();
    let error =
        run_wasm_analyzer(&large_table_module, &input, 10_000, DEFAULT_MEMORY_BYTES).unwrap_err();
    assert!(error.to_string().contains("Wasm analyzer failure"));

    let error =
        run_wasm_analyzer(&loop_module, &input, MAX_FUEL + 1, DEFAULT_MEMORY_BYTES).unwrap_err();
    assert!(error.to_string().contains("within host limits"));
    let error = run_wasm_analyzer(&loop_module, &input, 10_000, MAX_MEMORY_BYTES + 1).unwrap_err();
    assert!(error.to_string().contains("within host limits"));
}

#[test]
fn wasm_runtime_validates_output_revision() {
    let input = AnalyzerInput {
        schema_version: 1,
        revision: 7,
        claims: vec![],
        relations: vec![],
        decision_bases: vec![],
    };
    let payload = r#"{"schema_version":1,"based_on_revision":6,"findings":[]}"#;
    let wat_payload = payload.replace('"', "\\\"");
    let packed = ((1024_u64) << 32) | payload.len() as u64;
    let module = path("stale.wat");
    std::fs::write(
        &module,
        format!(
            r#"(module (memory (export "memory") 1) (data (i32.const 1024) "{wat_payload}") (func (export "alloc") (param i32) (result i32) i32.const 0) (func (export "analyze") (param i32 i32) (result i64) i64.const {packed}))"#
        ),
    )
    .unwrap();
    let error = run_wasm_analyzer(&module, &input, 10_000, DEFAULT_MEMORY_BYTES).unwrap_err();
    assert!(error.to_string().contains("revision does not match"));

    let out_of_bounds = path("out-of-bounds.wat");
    let packed = ((65_530_u64) << 32) | 100;
    std::fs::write(
        &out_of_bounds,
        format!(
            r#"(module (memory (export "memory") 1) (func (export "alloc") (param i32) (result i32) i32.const 0) (func (export "analyze") (param i32 i32) (result i64) i64.const {packed}))"#
        ),
    )
    .unwrap();
    let error =
        run_wasm_analyzer(&out_of_bounds, &input, 10_000, DEFAULT_MEMORY_BYTES).unwrap_err();
    assert!(error.to_string().contains("Wasm analyzer failure"));
}

#[test]
fn schema_three_logs_migrate_to_schema_four_without_mutating_source() {
    let source = path("migration").join("events-v3.jsonl");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    let record = json!({
        "sequence": 1,
        "schema_version": 3,
        "event_id": "claim",
        "idempotency_key": "claim",
        "expected_revision": 0,
        "actor": {"kind": "agent", "id": "agent", "provenance": "test"},
        "scope": "repo",
        "event": {"type": "claim_recorded", "claim_id": "c1", "statement": "legacy", "status": "hypothesized", "evidence_refs": []}
    });
    let bytes = format!("{record}\n");
    std::fs::write(&source, &bytes).unwrap();
    let destination = path("migration-destination").join("events-v4.jsonl");
    let outcome = migrate_v3_to_v4(&source, &destination).unwrap();
    assert_eq!(outcome.from_schema, 3);
    assert_eq!(outcome.to_schema, 4);
    assert_eq!(std::fs::read_to_string(&source).unwrap(), bytes);
    assert_eq!(load(&destination).unwrap().state().claims.len(), 1);
}
