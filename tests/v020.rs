use aporic::codex::{GatePolicy, PreToolUseInput, explain_action, pre_tool_use_transaction};
use aporic::policy::{PolicyDocument, ToolPolicy};
use aporic::{
    Actor, ActorKind, CommitRequest, CommitStatus, Event, SCHEMA_VERSION, commit, initialize, load,
    migrate_v1_to_v3,
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "aporic-v020-{name}-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
}

fn request(expected_revision: u64, event_id: &str, event: Event) -> CommitRequest {
    CommitRequest {
        schema_version: SCHEMA_VERSION,
        event_id: event_id.into(),
        idempotency_key: event_id.into(),
        expected_revision,
        actor: Actor {
            kind: ActorKind::Human,
            id: "user".into(),
            provenance: "test".into(),
        },
        scope: "repo".into(),
        event,
    }
}

fn grant_policy() -> GatePolicy {
    GatePolicy::from_document(PolicyDocument {
        schema_version: 1,
        tools: BTreeMap::from([
            (
                "apply_patch".into(),
                ToolPolicy {
                    require_plan: true,
                    require_grant: true,
                },
            ),
            (
                "exec_command".into(),
                ToolPolicy {
                    require_plan: false,
                    require_grant: false,
                },
            ),
        ]),
    })
    .unwrap()
}

#[test]
fn policy_tool_names_are_bounded_visible_ascii() {
    for invalid in ["", " ", "tool name", "도구"] {
        let document = PolicyDocument {
            schema_version: 1,
            tools: BTreeMap::from([(
                invalid.into(),
                ToolPolicy {
                    require_plan: true,
                    require_grant: true,
                },
            )]),
        };
        assert!(document.validate().is_err(), "{invalid:?}");
    }
    let too_long = "x".repeat(257);
    let document = PolicyDocument {
        schema_version: 1,
        tools: BTreeMap::from([(
            too_long,
            ToolPolicy {
                require_plan: true,
                require_grant: true,
            },
        )]),
    };
    assert!(document.validate().is_err());
}

fn input(tool_use_id: &str, tool_input: serde_json::Value) -> PreToolUseInput {
    PreToolUseInput {
        session_id: "session-1".into(),
        hook_event_name: "PreToolUse".into(),
        cwd: "/repo".into(),
        turn_id: "turn-1".into(),
        tool_name: "apply_patch".into(),
        tool_use_id: tool_use_id.into(),
        tool_input,
        model: None,
        permission_mode: None,
    }
}

fn store_with_grant(max_uses: u32, tool_name: &str) -> PathBuf {
    let store = temp_path("grant").join("events.jsonl");
    initialize(&store).unwrap();
    assert_eq!(
        commit(
            &store,
            request(
                0,
                "plan",
                Event::PlanRegistered {
                    plan_id: "plan-1".into(),
                    objective: "bounded patch".into(),
                    acceptance_checks: vec!["tests pass".into()],
                    unresolved_questions: vec![],
                },
            ),
        )
        .unwrap()
        .status,
        CommitStatus::Committed
    );
    assert_eq!(
        commit(
            &store,
            request(
                1,
                "grant",
                Event::ExecutionGrantIssued {
                    grant_id: "grant-1".into(),
                    plan_id: "plan-1".into(),
                    session_id: "session-1".into(),
                    tool_name: tool_name.into(),
                    tool_input: serde_json::json!({"patch": "exact"}),
                    max_uses,
                    authority_ref: Some("test-user".into()),
                },
            ),
        )
        .unwrap()
        .status,
        CommitStatus::Committed
    );
    store
}

fn store_with_one_shot_grant() -> PathBuf {
    store_with_grant(1, "apply_patch")
}

#[test]
fn one_shot_grant_is_consumed_once_and_reuse_is_denied() {
    let store = store_with_one_shot_grant();
    let policy = grant_policy();
    let action = input("tool-use-1", serde_json::json!({"patch": "exact"}));

    assert!(
        pre_tool_use_transaction(&store, &action, "repo", &policy)
            .unwrap()
            .is_none()
    );
    let denied = pre_tool_use_transaction(&store, &action, "repo", &policy)
        .unwrap()
        .unwrap();
    assert_eq!(
        denied.hook_specific_output.permission_decision_reason,
        "APORIC_TOOL_USE_ALREADY_CONSUMED: apply_patch cannot reuse a consumed tool_use_id."
    );
    let state = load(&store).unwrap();
    assert_eq!(state.state().revision, 3);
    assert_eq!(state.state().execution_grants["grant-1"].consumed_uses, 1);
}

#[test]
fn consumed_tool_use_id_remains_denied_after_policy_no_longer_requires_a_grant() {
    let store = store_with_one_shot_grant();
    let action = input("tool-use-1", serde_json::json!({"patch": "exact"}));
    assert!(
        pre_tool_use_transaction(&store, &action, "repo", &grant_policy())
            .unwrap()
            .is_none()
    );
    let unbounded_policy = GatePolicy::new("apply_patch", false).unwrap();
    let denied = pre_tool_use_transaction(&store, &action, "repo", &unbounded_policy)
        .unwrap()
        .unwrap();
    assert!(
        denied
            .hook_specific_output
            .permission_decision_reason
            .starts_with("APORIC_TOOL_USE_ALREADY_CONSUMED")
    );
    assert_eq!(load(&store).unwrap().state().revision, 3);
}

#[test]
fn exact_input_binding_denies_mismatch_without_consuming() {
    let store = store_with_one_shot_grant();
    let action = input("tool-use-1", serde_json::json!({"patch": "different"}));
    let denied = pre_tool_use_transaction(&store, &action, "repo", &grant_policy())
        .unwrap()
        .unwrap();
    assert!(
        denied
            .hook_specific_output
            .permission_decision_reason
            .starts_with("APORIC_PLAN_AUTHORIZATION_REQUIRED")
    );
    assert_eq!(load(&store).unwrap().state().revision, 2);
}

#[test]
fn optional_execution_grant_does_not_replace_or_bypass_required_plan_authorization() {
    let store = store_with_one_shot_grant();
    let action = input("tool-use-1", serde_json::json!({"patch": "exact"}));
    let legacy_plan_policy = GatePolicy::new("apply_patch", true).unwrap();
    let denied = pre_tool_use_transaction(&store, &action, "repo", &legacy_plan_policy)
        .unwrap()
        .unwrap();
    assert!(
        denied
            .hook_specific_output
            .permission_decision_reason
            .starts_with("APORIC_PLAN_AUTHORIZATION_REQUIRED")
    );
    assert_eq!(load(&store).unwrap().state().revision, 2);
}

#[test]
fn concurrent_one_shot_admission_allows_at_most_one_and_records_one_consumption() {
    let store = Arc::new(store_with_one_shot_grant());
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for tool_use_id in ["tool-use-a", "tool-use-b"] {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            let action = input(tool_use_id, serde_json::json!({"patch": "exact"}));
            barrier.wait();
            pre_tool_use_transaction(&*store, &action, "repo", &grant_policy())
        }));
    }
    barrier.wait();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Ok(None)))
            .count(),
        1
    );
    let state = load(&*store).unwrap();
    assert_eq!(state.state().revision, 3);
    assert_eq!(state.state().consumed_tool_uses.len(), 1);
}

#[test]
fn multi_use_grant_allows_exactly_its_declared_limit() {
    let store = store_with_grant(2, "apply_patch");
    for tool_use_id in ["tool-use-1", "tool-use-2"] {
        let action = input(tool_use_id, serde_json::json!({"patch": "exact"}));
        assert!(
            pre_tool_use_transaction(&store, &action, "repo", &grant_policy())
                .unwrap()
                .is_none()
        );
    }
    let third = input("tool-use-3", serde_json::json!({"patch": "exact"}));
    let denied = pre_tool_use_transaction(&store, &third, "repo", &grant_policy())
        .unwrap()
        .unwrap();
    assert!(
        denied
            .hook_specific_output
            .permission_decision_reason
            .starts_with("APORIC_PLAN_AUTHORIZATION_REQUIRED")
    );
    let state = load(&store).unwrap();
    assert_eq!(state.state().revision, 4);
    assert_eq!(state.state().execution_grants["grant-1"].consumed_uses, 2);
}

#[test]
fn grant_revocation_requires_authority_and_prevents_admission() {
    let store = store_with_one_shot_grant();
    let mut unauthorized = request(
        2,
        "unauthorized-revoke",
        Event::ExecutionGrantRevoked {
            grant_id: "grant-1".into(),
        },
    );
    unauthorized.actor.kind = ActorKind::Agent;
    assert_eq!(
        commit(&store, unauthorized).unwrap().evaluation.reason_code,
        "REVOCATION_AUTHORITY_REQUIRED"
    );
    assert_eq!(
        commit(
            &store,
            request(
                2,
                "revoke",
                Event::ExecutionGrantRevoked {
                    grant_id: "grant-1".into(),
                },
            ),
        )
        .unwrap()
        .status,
        CommitStatus::Committed
    );
    let action = input("tool-use-1", serde_json::json!({"patch": "exact"}));
    let denied = pre_tool_use_transaction(&store, &action, "repo", &grant_policy())
        .unwrap()
        .unwrap();
    assert!(
        denied
            .hook_specific_output
            .permission_decision_reason
            .starts_with("APORIC_PLAN_AUTHORIZATION_REQUIRED")
    );
    assert_eq!(load(&store).unwrap().state().revision, 3);
}

#[test]
fn grant_issuance_requires_plan_authority() {
    let store = temp_path("unauthorized-grant").join("events.jsonl");
    initialize(&store).unwrap();
    commit(
        &store,
        request(
            0,
            "plan",
            Event::PlanRegistered {
                plan_id: "plan-1".into(),
                objective: "bounded patch".into(),
                acceptance_checks: vec!["tests pass".into()],
                unresolved_questions: vec![],
            },
        ),
    )
    .unwrap();
    let mut unauthorized = request(
        1,
        "grant",
        Event::ExecutionGrantIssued {
            grant_id: "grant-1".into(),
            plan_id: "plan-1".into(),
            session_id: "session-1".into(),
            tool_name: "apply_patch".into(),
            tool_input: serde_json::json!({"patch": "exact"}),
            max_uses: 1,
            authority_ref: None,
        },
    );
    unauthorized.actor.kind = ActorKind::Evidence;
    let outcome = commit(&store, unauthorized).unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.evaluation.reason_code, "PLAN_AUTHORITY_REQUIRED");
    assert_eq!(load(&store).unwrap().state().revision, 1);
}

#[test]
fn delegated_grant_issuance_requires_an_exact_active_delegation() {
    let store = temp_path("delegated-grant").join("events.jsonl");
    initialize(&store).unwrap();
    commit(
        &store,
        request(
            0,
            "plan",
            Event::PlanRegistered {
                plan_id: "plan-1".into(),
                objective: "bounded patch".into(),
                acceptance_checks: vec!["tests pass".into()],
                unresolved_questions: vec![],
            },
        ),
    )
    .unwrap();
    let mut unauthorized = request(
        1,
        "grant",
        Event::ExecutionGrantIssued {
            grant_id: "grant-1".into(),
            plan_id: "plan-1".into(),
            session_id: "session-1".into(),
            tool_name: "apply_patch".into(),
            tool_input: serde_json::json!({"patch": "exact"}),
            max_uses: 1,
            authority_ref: Some("missing-delegation".into()),
        },
    );
    unauthorized.actor.kind = ActorKind::Agent;
    unauthorized.actor.id = "agent-1".into();
    let outcome = commit(&store, unauthorized).unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.evaluation.reason_code, "UNKNOWN_DELEGATION");
    assert_eq!(load(&store).unwrap().state().revision, 1);
}

#[test]
fn invalid_grant_targets_are_rejected() {
    let store = temp_path("invalid-grant").join("events.jsonl");
    initialize(&store).unwrap();
    commit(
        &store,
        request(
            0,
            "plan",
            Event::PlanRegistered {
                plan_id: "plan-1".into(),
                objective: "bounded patch".into(),
                acceptance_checks: vec!["tests pass".into()],
                unresolved_questions: vec![],
            },
        ),
    )
    .unwrap();

    let zero_use = request(
        1,
        "zero-use",
        Event::ExecutionGrantIssued {
            grant_id: "grant-zero".into(),
            plan_id: "plan-1".into(),
            session_id: "session-1".into(),
            tool_name: "apply_patch".into(),
            tool_input: serde_json::json!({"patch": "exact"}),
            max_uses: 0,
            authority_ref: None,
        },
    );
    assert_eq!(
        commit(&store, zero_use).unwrap().evaluation.reason_code,
        "INVALID_EXECUTION_GRANT"
    );

    let unknown_plan = request(
        1,
        "unknown-plan",
        Event::ExecutionGrantIssued {
            grant_id: "grant-unknown".into(),
            plan_id: "missing-plan".into(),
            session_id: "session-1".into(),
            tool_name: "apply_patch".into(),
            tool_input: serde_json::json!({"patch": "exact"}),
            max_uses: 1,
            authority_ref: None,
        },
    );
    assert_eq!(
        commit(&store, unknown_plan).unwrap().evaluation.reason_code,
        "UNKNOWN_PLAN"
    );

    let mut cross_scope = request(
        1,
        "cross-scope",
        Event::ExecutionGrantIssued {
            grant_id: "grant-cross-scope".into(),
            plan_id: "plan-1".into(),
            session_id: "session-1".into(),
            tool_name: "apply_patch".into(),
            tool_input: serde_json::json!({"patch": "exact"}),
            max_uses: 1,
            authority_ref: None,
        },
    );
    cross_scope.scope = "other".into();
    assert_eq!(
        commit(&store, cross_scope).unwrap().evaluation.reason_code,
        "SCOPE_MISMATCH"
    );

    assert_eq!(load(&store).unwrap().state().revision, 1);
}

#[test]
fn grant_consumption_requires_the_host_actor() {
    let store = store_with_one_shot_grant();
    let consumption = request(
        2,
        "consume",
        Event::ExecutionGrantConsumed {
            grant_id: "grant-1".into(),
            tool_use_id: "tool-use-1".into(),
        },
    );
    let outcome = commit(&store, consumption).unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.evaluation.reason_code, "HOST_CONSUMPTION_REQUIRED");
    assert_eq!(load(&store).unwrap().state().revision, 2);
}

#[test]
fn cross_scope_grant_revocation_is_rejected() {
    let store = store_with_one_shot_grant();
    let mut cross_scope = request(
        2,
        "cross-scope-revoke",
        Event::ExecutionGrantRevoked {
            grant_id: "grant-1".into(),
        },
    );
    cross_scope.scope = "other".into();
    let outcome = commit(&store, cross_scope).unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.evaluation.reason_code, "SCOPE_MISMATCH");
    assert_eq!(load(&store).unwrap().state().revision, 2);
}

#[test]
fn grant_matching_is_exact_to_scope_session_and_tool() {
    let cases = [
        ("wrong-scope", "other", "session-1", "apply_patch"),
        ("wrong-session", "repo", "session-2", "apply_patch"),
    ];
    for (name, scope, session_id, tool_name) in cases {
        let store = store_with_one_shot_grant();
        let mut action = input("tool-use-1", serde_json::json!({"patch": "exact"}));
        action.session_id = session_id.into();
        action.tool_name = tool_name.into();
        let denied = pre_tool_use_transaction(&store, &action, scope, &grant_policy())
            .unwrap()
            .unwrap();
        assert!(
            denied
                .hook_specific_output
                .permission_decision_reason
                .starts_with("APORIC_PLAN_AUTHORIZATION_REQUIRED"),
            "{name}"
        );
        assert_eq!(load(&store).unwrap().state().revision, 2, "{name}");
    }

    let store = store_with_grant(1, "exec_command");
    let action = input("tool-use-1", serde_json::json!({"patch": "exact"}));
    let denied = pre_tool_use_transaction(&store, &action, "repo", &grant_policy())
        .unwrap()
        .unwrap();
    assert!(
        denied
            .hook_specific_output
            .permission_decision_reason
            .starts_with("APORIC_PLAN_AUTHORIZATION_REQUIRED")
    );
    assert_eq!(load(&store).unwrap().state().revision, 2);
}

#[test]
fn explain_uses_shared_evaluator_without_changing_revision() {
    let store = store_with_one_shot_grant();
    let action = input("tool-use-1", serde_json::json!({"patch": "exact"}));
    let before = load(&store).unwrap();
    let explanation = explain_action(before.state(), &action, "repo", &grant_policy()).unwrap();
    assert_eq!(explanation.evaluation.unwrap().reason_code, "ALLOW");
    assert!(explanation.tool_input_identity.starts_with("fnv1a64:"));
    assert_eq!(load(&store).unwrap().state().revision, 2);
}

#[test]
fn migration_preserves_v1_source_and_refuses_destination_overwrite() {
    let source = temp_path("migration-source").join("events-v1.jsonl");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    let record = serde_json::json!({
        "sequence": 1,
        "schema_version": 1,
        "event_id": "plan",
        "idempotency_key": "plan",
        "expected_revision": 0,
        "actor": {"kind": "human", "id": "user", "provenance": "legacy"},
        "scope": "repo",
        "event": {
            "type": "plan_registered",
            "plan_id": "plan-1",
            "objective": "legacy plan",
            "acceptance_checks": ["passes"],
            "unresolved_questions": []
        }
    });
    let source_bytes = format!("{record}\n").into_bytes();
    std::fs::write(&source, &source_bytes).unwrap();
    let destination = source.parent().unwrap().join("events-v2.jsonl");

    let outcome = migrate_v1_to_v3(&source, &destination).unwrap();
    assert_eq!(outcome.revision, 1);
    assert_eq!(outcome.records, 1);
    assert_eq!(std::fs::read(&source).unwrap(), source_bytes);
    assert_eq!(load(&destination).unwrap().state().revision, 1);
    assert!(migrate_v1_to_v3(&source, &destination).is_err());
}

#[test]
fn migration_rejects_v2_grant_events_mislabeled_as_v1() {
    let source = temp_path("migration-future-event").join("events-v1.jsonl");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    let record = serde_json::json!({
        "sequence": 1,
        "schema_version": 1,
        "event_id": "grant",
        "idempotency_key": "grant",
        "expected_revision": 0,
        "actor": {"kind": "human", "id": "user", "provenance": "mislabeled"},
        "scope": "repo",
        "event": {
            "type": "execution_grant_issued",
            "grant_id": "grant-1",
            "plan_id": "plan-1",
            "session_id": "session-1",
            "tool_name": "apply_patch",
            "tool_input": {"patch": "exact"},
            "max_uses": 1,
            "authority_ref": null
        }
    });
    std::fs::write(&source, format!("{record}\n")).unwrap();
    let destination = source.parent().unwrap().join("events-v2.jsonl");
    let error = migrate_v1_to_v3(&source, &destination).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("is not part of schema version 1")
    );
    assert!(!destination.exists());
}
