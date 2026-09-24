use aporic::codex::{
    GatePolicy, PreToolUseInput, SessionSource, SessionStartInput, evaluate_gate,
    pre_tool_use_output, session_start_output,
};
use aporic::governance::GateStatus;
use aporic::policy::{PolicyDocument, ToolPolicy};
use aporic::{
    Actor, ActorKind, CommitRequest, CommitStatus, Event, SCHEMA_VERSION, commit, initialize, load,
    migrate_v4_to_v7, migrate_v5_to_v7,
};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "aporic-v070-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn request(revision: u64, id: &str, scope: &str, event: Event) -> CommitRequest {
    CommitRequest {
        schema_version: SCHEMA_VERSION,
        event_id: id.into(),
        idempotency_key: id.into(),
        expected_revision: revision,
        actor: Actor {
            kind: ActorKind::Human,
            id: "tester".into(),
            provenance: "v070-test".into(),
        },
        scope: scope.into(),
        event,
    }
}

fn intent(id: &str, goal: &str) -> Event {
    Event::IntentEnvelopeRecorded {
        intent_id: id.into(),
        source_ref: format!("conversation:{id}"),
        goal: goal.into(),
        explicit_items: vec!["change only the requested behavior".into()],
        inferred_items: vec!["preserve compatibility".into()],
        unknown_items: vec!["release timing is unknown".into()],
    }
}

#[test]
fn superseded_intent_invalidates_bound_plan_authority() {
    let store = path("stale-authority").join("events.jsonl");
    initialize(&store).unwrap();
    commit(
        &store,
        request(0, "intent-1", "repo", intent("i1", "first goal")),
    )
    .unwrap();
    commit(
        &store,
        request(
            1,
            "plan",
            "repo",
            Event::PlanRegistered {
                plan_id: "p1".into(),
                objective: "implement the first goal".into(),
                acceptance_checks: vec!["focused test passes".into()],
                unresolved_questions: vec![],
                intent_id: Some("i1".into()),
            },
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            2,
            "auth-1",
            "repo",
            Event::PlanAuthorized {
                authorization_id: "a1".into(),
                plan_id: "p1".into(),
                session_id: "s1".into(),
                tool_name: "apply_patch".into(),
                authority_ref: Some("conversation:approval".into()),
            },
        ),
    )
    .unwrap();

    commit(
        &store,
        request(
            3,
            "grant-1",
            "repo",
            Event::ExecutionGrantIssued {
                grant_id: "g1".into(),
                plan_id: "p1".into(),
                session_id: "s1".into(),
                tool_name: "apply_patch".into(),
                tool_input: json!({"patch": "bound"}),
                max_uses: 1,
                authority_ref: Some("conversation:approval".into()),
            },
        ),
    )
    .unwrap();

    let policy = GatePolicy::new("apply_patch", true).unwrap();
    assert_eq!(
        evaluate_gate(
            load(&store).unwrap().state(),
            "repo",
            "s1",
            "apply_patch",
            &policy
        )
        .unwrap()
        .unwrap()
        .status,
        GateStatus::Allowed
    );

    let grant_policy = GatePolicy::from_document(aporic::policy::PolicyDocument {
        schema_version: 1,
        lifecycle_mode: None,
        tools: [(
            "apply_patch".into(),
            aporic::policy::ToolPolicy {
                require_plan: true,
                require_grant: true,
                require_intent: None,
                auto_allow_low_risk_profiles: None,
            },
        )]
        .into_iter()
        .collect(),
    })
    .unwrap();
    let tool_input = PreToolUseInput {
        session_id: "s1".into(),
        hook_event_name: "PreToolUse".into(),
        cwd: "/workspace/repo".into(),
        turn_id: "turn-1".into(),
        tool_name: "apply_patch".into(),
        tool_use_id: "tool-use-1".into(),
        tool_input: json!({"patch": "bound"}),
        model: None,
        permission_mode: None,
    };
    assert!(
        pre_tool_use_output(
            load(&store).unwrap().state(),
            &tool_input,
            "repo",
            &grant_policy,
        )
        .unwrap()
        .is_none()
    );

    commit(
        &store,
        request(4, "intent-2", "repo", intent("i2", "replacement goal")),
    )
    .unwrap();
    commit(
        &store,
        request(
            5,
            "supersede",
            "repo",
            Event::IntentEnvelopeSuperseded {
                intent_id: "i1".into(),
                replacement_intent_id: "i2".into(),
                reason: "the user corrected the requested goal".into(),
            },
        ),
    )
    .unwrap();

    assert_eq!(
        evaluate_gate(
            load(&store).unwrap().state(),
            "repo",
            "s1",
            "apply_patch",
            &policy
        )
        .unwrap()
        .unwrap()
        .status,
        GateStatus::PlanAuthorizationRequired
    );
    let denied = pre_tool_use_output(
        load(&store).unwrap().state(),
        &tool_input,
        "repo",
        &grant_policy,
    )
    .unwrap()
    .unwrap();
    assert!(
        denied
            .hook_specific_output
            .permission_decision_reason
            .starts_with("APORIC_PLAN_AUTHORIZATION_REQUIRED:")
    );
    let stale = commit(
        &store,
        request(
            6,
            "auth-2",
            "repo",
            Event::PlanAuthorized {
                authorization_id: "a2".into(),
                plan_id: "p1".into(),
                session_id: "s2".into(),
                tool_name: "apply_patch".into(),
                authority_ref: Some("conversation:approval".into()),
            },
        ),
    )
    .unwrap();
    assert_eq!(stale.status, CommitStatus::Rejected);
    assert_eq!(stale.evaluation.reason_code, "STALE_PLAN_INTENT");

    let stale_grant = commit(
        &store,
        request(
            6,
            "grant",
            "repo",
            Event::ExecutionGrantIssued {
                grant_id: "g2".into(),
                plan_id: "p1".into(),
                session_id: "s2".into(),
                tool_name: "apply_patch".into(),
                tool_input: json!({"patch": "stale"}),
                max_uses: 1,
                authority_ref: Some("conversation:approval".into()),
            },
        ),
    )
    .unwrap();
    assert_eq!(stale_grant.status, CommitStatus::Rejected);
    assert_eq!(stale_grant.evaluation.reason_code, "STALE_PLAN_INTENT");

    let stale_completion = commit(
        &store,
        request(
            6,
            "complete",
            "repo",
            Event::PlanCompleted {
                plan_id: "p1".into(),
                residual_risk_refs: vec![],
            },
        ),
    )
    .unwrap();
    assert_eq!(stale_completion.status, CommitStatus::Rejected);
    assert_eq!(stale_completion.evaluation.reason_code, "STALE_PLAN_INTENT");
}

#[test]
fn plan_binding_rejects_dangling_cross_scope_and_superseded_intents() {
    let store = path("invalid-binding").join("events.jsonl");
    initialize(&store).unwrap();
    commit(
        &store,
        request(0, "intent-1", "repo", intent("i1", "first goal")),
    )
    .unwrap();

    let dangling = commit(
        &store,
        request(
            1,
            "dangling",
            "repo",
            Event::PlanRegistered {
                plan_id: "dangling".into(),
                objective: "invalid".into(),
                acceptance_checks: vec!["never runs".into()],
                unresolved_questions: vec![],
                intent_id: Some("missing".into()),
            },
        ),
    )
    .unwrap();
    assert_eq!(dangling.evaluation.reason_code, "UNKNOWN_INTENT_ENVELOPE");

    let cross_scope = commit(
        &store,
        request(
            1,
            "cross-scope",
            "other",
            Event::PlanRegistered {
                plan_id: "cross-scope".into(),
                objective: "invalid".into(),
                acceptance_checks: vec!["never runs".into()],
                unresolved_questions: vec![],
                intent_id: Some("i1".into()),
            },
        ),
    )
    .unwrap();
    assert_eq!(cross_scope.evaluation.reason_code, "SCOPE_MISMATCH");

    commit(
        &store,
        request(1, "intent-2", "repo", intent("i2", "replacement goal")),
    )
    .unwrap();
    commit(
        &store,
        request(
            2,
            "supersede",
            "repo",
            Event::IntentEnvelopeSuperseded {
                intent_id: "i1".into(),
                replacement_intent_id: "i2".into(),
                reason: "the goal changed".into(),
            },
        ),
    )
    .unwrap();
    let cycle = commit(
        &store,
        request(
            3,
            "cycle",
            "repo",
            Event::IntentEnvelopeSuperseded {
                intent_id: "i2".into(),
                replacement_intent_id: "i1".into(),
                reason: "attempt to revive a stale intent".into(),
            },
        ),
    )
    .unwrap();
    assert_eq!(cycle.status, CommitStatus::Rejected);
    assert_eq!(cycle.evaluation.reason_code, "STALE_REPLACEMENT_INTENT");
}

#[test]
fn schema_four_migration_preserves_legacy_unbound_plans() {
    let source = path("migration-source").join("events-v4.jsonl");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    let record = json!({
        "sequence": 1,
        "schema_version": 4,
        "event_id": "legacy-plan",
        "idempotency_key": "legacy-plan",
        "expected_revision": 0,
        "actor": {"kind": "human", "id": "user", "provenance": "test"},
        "scope": "repo",
        "event": {
            "type": "plan_registered",
            "plan_id": "legacy",
            "objective": "legacy plan",
            "acceptance_checks": ["legacy check"],
            "unresolved_questions": []
        }
    });
    let source_bytes = format!("{record}\n");
    std::fs::write(&source, &source_bytes).unwrap();
    let destination = path("migration-destination").join("events-v6.jsonl");

    let outcome = migrate_v4_to_v7(&source, &destination).unwrap();
    assert_eq!(outcome.from_schema, 4);
    assert_eq!(outcome.to_schema, SCHEMA_VERSION);
    assert_eq!(std::fs::read_to_string(&source).unwrap(), source_bytes);
    assert_eq!(
        load(&destination).unwrap().state().plans["legacy"].intent_id,
        None
    );
}

#[test]
fn schema_five_logs_migrate_to_schema_six_without_mutating_source() {
    let source = path("migration-source-v5").join("events-v5.jsonl");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    let record = json!({
        "sequence": 1,
        "schema_version": 5,
        "event_id": "intent",
        "idempotency_key": "intent",
        "expected_revision": 0,
        "actor": {"kind": "human", "id": "user", "provenance": "test"},
        "scope": "repo",
        "event": {
            "type": "intent_envelope_recorded",
            "intent_id": "intent-1",
            "source_ref": "conversation:test",
            "goal": "preserve the v5 record",
            "explicit_items": ["migrate"],
            "inferred_items": [],
            "unknown_items": []
        }
    });
    let source_bytes = format!("{record}\n");
    std::fs::write(&source, &source_bytes).unwrap();
    let destination = path("migration-destination-v6").join("events-v6.jsonl");

    let outcome = migrate_v5_to_v7(&source, &destination).unwrap();
    assert_eq!(outcome.from_schema, 5);
    assert_eq!(outcome.to_schema, SCHEMA_VERSION);
    assert_eq!(outcome.records, 1);
    assert_eq!(std::fs::read_to_string(&source).unwrap(), source_bytes);
    assert_eq!(load(&destination).unwrap().state().intents.len(), 1);
    let migrated: Value =
        serde_json::from_str(std::fs::read_to_string(&destination).unwrap().trim_end()).unwrap();
    assert_eq!(migrated["schema_version"], SCHEMA_VERSION);
}

#[test]
fn schema_five_migration_rejects_schema_six_approval_events() {
    let source = path("migration-invalid-v5").join("events-v5.jsonl");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    let record = json!({
        "sequence": 1,
        "schema_version": 5,
        "event_id": "approval:one",
        "idempotency_key": "approval:one",
        "expected_revision": 0,
        "actor": {"kind": "human", "id": "user", "provenance": "test"},
        "scope": "repo",
        "event": {
            "type": "plan_approval_recorded",
            "approval_id": "one",
            "source_ref": "conversation:test",
            "goal": "change",
            "explicit_items": ["change"],
            "inferred_items": [],
            "unknown_items": [],
            "objective": "change",
            "acceptance_checks": ["passes"],
            "unresolved_questions": [],
            "session_id": "session",
            "tool_name": "apply_patch",
            "authority_ref": "approval"
        }
    });
    std::fs::write(&source, format!("{record}\n")).unwrap();
    let destination = path("migration-invalid-v5-destination").join("events-v6.jsonl");

    let error = migrate_v5_to_v7(&source, &destination).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("is not part of schema version 5")
    );
    assert!(!destination.exists());
}

#[test]
fn projection_exposes_only_active_intent_envelopes() {
    let store = path("projection").join("events.jsonl");
    initialize(&store).unwrap();
    commit(
        &store,
        request(0, "intent-1", "repo", intent("i1", "first goal")),
    )
    .unwrap();
    commit(
        &store,
        request(1, "intent-2", "repo", intent("i2", "replacement goal")),
    )
    .unwrap();
    commit(
        &store,
        request(
            2,
            "supersede",
            "repo",
            Event::IntentEnvelopeSuperseded {
                intent_id: "i1".into(),
                replacement_intent_id: "i2".into(),
                reason: "the goal changed".into(),
            },
        ),
    )
    .unwrap();

    let input = SessionStartInput {
        session_id: "s1".into(),
        hook_event_name: "SessionStart".into(),
        cwd: "/workspace/repo".into(),
        source: SessionSource::Startup,
        model: None,
        permission_mode: None,
    };
    let intent_policy = GatePolicy::from_document(PolicyDocument {
        schema_version: 2,
        lifecycle_mode: None,
        tools: [(
            "apply_patch".into(),
            ToolPolicy {
                require_plan: true,
                require_grant: false,
                require_intent: Some(true),
                auto_allow_low_risk_profiles: None,
            },
        )]
        .into_iter()
        .collect(),
    })
    .unwrap();
    let output = session_start_output(
        load(&store).unwrap().state(),
        &input,
        "repo",
        &intent_policy,
        aporic::codex::DEFAULT_PROJECTION_LIMIT_BYTES,
    )
    .unwrap();
    let context = output.hook_specific_output.additional_context;
    let start = context.find("<aporic-recorded-data>").unwrap() + "<aporic-recorded-data>".len();
    let end = context.find("</aporic-recorded-data>").unwrap();
    let projection: serde_json::Value = serde_json::from_str(&context[start..end]).unwrap();

    assert_eq!(projection["schema"], 6);
    assert_eq!(projection["active_intents"].as_array().unwrap().len(), 1);
    assert_eq!(projection["active_intents"][0]["id"], "i2");
    assert_eq!(projection["retained"]["intents"], 1);
    assert_eq!(projection["executions"][0]["require_intent"], true);
}

#[test]
fn policy_v2_requires_intent_without_breaking_v1_documents() {
    let legacy: PolicyDocument = serde_json::from_value(json!({
        "schema_version": 1,
        "tools": {
            "apply_patch": {"require_plan": true, "require_grant": false}
        }
    }))
    .unwrap();
    assert!(!legacy.tools["apply_patch"].requires_intent());
    legacy.validate().unwrap();

    let invalid_v1 = PolicyDocument {
        schema_version: 1,
        lifecycle_mode: None,
        tools: [(
            "apply_patch".into(),
            ToolPolicy {
                require_plan: true,
                require_grant: false,
                require_intent: Some(true),
                auto_allow_low_risk_profiles: None,
            },
        )]
        .into_iter()
        .collect(),
    };
    assert!(invalid_v1.validate().is_err());

    let missing_v2 = PolicyDocument {
        schema_version: 2,
        lifecycle_mode: None,
        tools: [(
            "apply_patch".into(),
            ToolPolicy {
                require_plan: true,
                require_grant: false,
                require_intent: None,
                auto_allow_low_risk_profiles: None,
            },
        )]
        .into_iter()
        .collect(),
    };
    assert!(missing_v2.validate().is_err());

    let ineffective_v2 = PolicyDocument {
        schema_version: 2,
        lifecycle_mode: None,
        tools: [(
            "apply_patch".into(),
            ToolPolicy {
                require_plan: false,
                require_grant: false,
                require_intent: Some(true),
                auto_allow_low_risk_profiles: None,
            },
        )]
        .into_iter()
        .collect(),
    };
    assert!(ineffective_v2.validate().is_err());
}

#[test]
fn require_intent_excludes_unbound_plan_authorizations_and_grants() {
    let store = path("require-intent").join("events.jsonl");
    initialize(&store).unwrap();
    commit(
        &store,
        request(0, "intent", "repo", intent("i1", "bound goal")),
    )
    .unwrap();
    commit(
        &store,
        request(
            1,
            "unbound-plan",
            "repo",
            Event::PlanRegistered {
                plan_id: "unbound".into(),
                objective: "legacy-compatible plan".into(),
                acceptance_checks: vec!["test".into()],
                unresolved_questions: vec![],
                intent_id: None,
            },
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            2,
            "unbound-auth",
            "repo",
            Event::PlanAuthorized {
                authorization_id: "unbound-auth".into(),
                plan_id: "unbound".into(),
                session_id: "unbound-session".into(),
                tool_name: "apply_patch".into(),
                authority_ref: Some("test".into()),
            },
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            3,
            "bound-plan",
            "repo",
            Event::PlanRegistered {
                plan_id: "bound".into(),
                objective: "intent-bound plan".into(),
                acceptance_checks: vec!["test".into()],
                unresolved_questions: vec![],
                intent_id: Some("i1".into()),
            },
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            4,
            "bound-auth",
            "repo",
            Event::PlanAuthorized {
                authorization_id: "bound-auth".into(),
                plan_id: "bound".into(),
                session_id: "bound-session".into(),
                tool_name: "apply_patch".into(),
                authority_ref: Some("test".into()),
            },
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            5,
            "unbound-grant",
            "repo",
            Event::ExecutionGrantIssued {
                grant_id: "unbound-grant".into(),
                plan_id: "unbound".into(),
                session_id: "unbound-grant-session".into(),
                tool_name: "apply_patch".into(),
                tool_input: json!({"patch": "unbound"}),
                max_uses: 1,
                authority_ref: Some("test".into()),
            },
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            6,
            "bound-grant",
            "repo",
            Event::ExecutionGrantIssued {
                grant_id: "bound-grant".into(),
                plan_id: "bound".into(),
                session_id: "bound-grant-session".into(),
                tool_name: "apply_patch".into(),
                tool_input: json!({"patch": "bound"}),
                max_uses: 1,
                authority_ref: Some("test".into()),
            },
        ),
    )
    .unwrap();

    let required = GatePolicy::from_document(PolicyDocument {
        schema_version: 2,
        lifecycle_mode: None,
        tools: [(
            "apply_patch".into(),
            ToolPolicy {
                require_plan: true,
                require_grant: false,
                require_intent: Some(true),
                auto_allow_low_risk_profiles: None,
            },
        )]
        .into_iter()
        .collect(),
    })
    .unwrap();
    let state = load(&store).unwrap();
    assert_eq!(
        evaluate_gate(
            state.state(),
            "repo",
            "unbound-session",
            "apply_patch",
            &required,
        )
        .unwrap()
        .unwrap()
        .status,
        GateStatus::IntentBoundPlanRequired
    );
    assert_eq!(
        evaluate_gate(
            state.state(),
            "repo",
            "bound-session",
            "apply_patch",
            &required,
        )
        .unwrap()
        .unwrap()
        .status,
        GateStatus::Allowed
    );

    let grant_required = GatePolicy::from_document(PolicyDocument {
        schema_version: 2,
        lifecycle_mode: None,
        tools: [(
            "apply_patch".into(),
            ToolPolicy {
                require_plan: false,
                require_grant: true,
                require_intent: Some(true),
                auto_allow_low_risk_profiles: None,
            },
        )]
        .into_iter()
        .collect(),
    })
    .unwrap();
    let grant_input = |session_id: &str, patch: &str| PreToolUseInput {
        session_id: session_id.into(),
        hook_event_name: "PreToolUse".into(),
        cwd: "/workspace/repo".into(),
        turn_id: "turn-1".into(),
        tool_name: "apply_patch".into(),
        tool_use_id: format!("use-{session_id}"),
        tool_input: json!({"patch": patch}),
        model: None,
        permission_mode: None,
    };
    let denial = pre_tool_use_output(
        state.state(),
        &grant_input("unbound-grant-session", "unbound"),
        "repo",
        &grant_required,
    )
    .unwrap()
    .expect("unbound grant must be denied");
    assert!(
        denial
            .hook_specific_output
            .permission_decision_reason
            .starts_with("APORIC_INTENT_BOUND_PLAN_REQUIRED:")
    );
    assert!(
        pre_tool_use_output(
            state.state(),
            &grant_input("bound-grant-session", "bound"),
            "repo",
            &grant_required,
        )
        .unwrap()
        .is_none()
    );

    commit(
        &store,
        request(
            7,
            "mixed-bound-auth",
            "repo",
            Event::PlanAuthorized {
                authorization_id: "mixed-bound-auth".into(),
                plan_id: "bound".into(),
                session_id: "unbound-session".into(),
                tool_name: "apply_patch".into(),
                authority_ref: Some("test".into()),
            },
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            8,
            "mixed-bound-grant",
            "repo",
            Event::ExecutionGrantIssued {
                grant_id: "mixed-bound-grant".into(),
                plan_id: "bound".into(),
                session_id: "unbound-grant-session".into(),
                tool_name: "apply_patch".into(),
                tool_input: json!({"patch": "unbound"}),
                max_uses: 1,
                authority_ref: Some("test".into()),
            },
        ),
    )
    .unwrap();

    let mixed_state = load(&store).unwrap();
    assert_eq!(
        evaluate_gate(
            mixed_state.state(),
            "repo",
            "unbound-session",
            "apply_patch",
            &required,
        )
        .unwrap()
        .unwrap()
        .status,
        GateStatus::Allowed
    );
    assert!(
        pre_tool_use_output(
            mixed_state.state(),
            &grant_input("unbound-grant-session", "unbound"),
            "repo",
            &grant_required,
        )
        .unwrap()
        .is_none()
    );
}
