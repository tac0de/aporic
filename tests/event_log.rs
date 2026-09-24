use aporic::{
    Actor, ActorKind, CommitRequest, CommitStatus, Delegation, Error, Event, EvidenceKind,
    InformationRequest, Plan, SCHEMA_VERSION, State, StoredEvent, TransitionKind, Verdict, commit,
    evaluate, initialize, load, load_nonblocking,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};

fn temp_log(name: &str) -> PathBuf {
    let unique = format!(
        "aporic-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    std::env::temp_dir().join(unique).join("events.jsonl")
}

#[test]
fn initialization_creates_empty_store_and_refuses_overwrite() {
    let path = temp_log("initialize");
    initialize(&path).unwrap();
    assert_eq!(load(&path).unwrap().state().revision, 0);
    let error = initialize(&path).unwrap_err();
    assert!(matches!(error, Error::Io(error) if error.kind() == std::io::ErrorKind::AlreadyExists));
}

#[test]
fn tool_hold_lifecycle_requires_authorized_release() {
    let path = temp_log("tool-hold");
    let placed = human_request(
        "hold-event-1",
        "key-1",
        0,
        Event::ToolHoldPlaced {
            tool_hold_id: "hold-1".into(),
            tool_name: "apply_patch".into(),
            reason: "plan direction is unresolved".into(),
        },
    );
    assert_eq!(
        commit(&path, placed).unwrap().status,
        CommitStatus::Committed
    );

    let mut unauthorized = human_request(
        "release-event-1",
        "key-2",
        1,
        Event::ToolHoldReleased {
            tool_hold_id: "hold-1".into(),
        },
    );
    unauthorized.actor.kind = ActorKind::Agent;
    let denied = commit(&path, unauthorized).unwrap();
    assert_eq!(denied.evaluation.reason_code, "RELEASE_AUTHORITY_REQUIRED");
    assert!(load(&path).unwrap().state().tool_holds["hold-1"].active);

    let release = human_request(
        "release-event-2",
        "key-3",
        1,
        Event::ToolHoldReleased {
            tool_hold_id: "hold-1".into(),
        },
    );
    assert_eq!(
        commit(&path, release).unwrap().status,
        CommitStatus::Committed
    );
    assert!(!load(&path).unwrap().state().tool_holds["hold-1"].active);
}

#[test]
fn nonblocking_load_reports_busy_store() {
    let path = temp_log("nonblocking");
    initialize(&path).unwrap();
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    file.lock().unwrap();
    let error = load_nonblocking(&path).unwrap_err();
    assert!(matches!(error, Error::Io(error) if error.kind() == std::io::ErrorKind::WouldBlock));
    file.unlock().unwrap();
}

#[test]
fn unsupported_stored_schema_stops_replay_and_further_writes() {
    let path = temp_log("unsupported-schema");
    initialize(&path).unwrap();
    let stored = StoredEvent {
        sequence: 1,
        request: CommitRequest {
            schema_version: 99,
            event_id: "future-event".into(),
            idempotency_key: "future-key".into(),
            expected_revision: 0,
            actor: Actor {
                kind: ActorKind::Human,
                id: "user".into(),
                provenance: "future-host".into(),
            },
            scope: "repo".into(),
            event: Event::PlanRegistered {
                plan_id: "future-plan".into(),
                objective: "Future semantics".into(),
                acceptance_checks: vec!["future check".into()],
                unresolved_questions: vec![],
                intent_id: None,
            },
        },
    };
    let mut bytes = serde_json::to_vec(&stored).unwrap();
    bytes.push(b'\n');
    std::fs::write(&path, &bytes).unwrap();

    let replay_error = load(&path).unwrap_err();
    assert!(matches!(
        replay_error,
        Error::CorruptLog { line: 1, reason }
            if reason == "unsupported schema version 99; expected 7"
    ));

    let append_error = commit(&path, open_aporia("a1", "k1", 0)).unwrap_err();
    assert!(matches!(append_error, Error::CorruptLog { line: 1, .. }));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}

#[test]
fn composite_plan_approval_replay_rejects_derived_id_collisions() {
    let path = temp_log("approval-derived-id-collision");
    initialize(&path).unwrap();
    let first = StoredEvent {
        sequence: 1,
        request: human_request(
            "intent-event",
            "intent-key",
            0,
            Event::IntentEnvelopeRecorded {
                intent_id: "intent:approval-1".into(),
                source_ref: "conversation:first".into(),
                goal: "first goal".into(),
                explicit_items: vec![],
                inferred_items: vec![],
                unknown_items: vec![],
            },
        ),
    };
    let second = StoredEvent {
        sequence: 2,
        request: human_request(
            "approval:approval-1",
            "approval:approval-1",
            1,
            Event::PlanApprovalRecorded {
                approval_id: "approval-1".into(),
                source_ref: "conversation:second".into(),
                goal: "second goal".into(),
                explicit_items: vec![],
                inferred_items: vec![],
                unknown_items: vec![],
                objective: "second objective".into(),
                acceptance_checks: vec!["test passes".into()],
                unresolved_questions: vec![],
                session_id: "session-1".into(),
                tool_name: "apply_patch".into(),
                authority_ref: "explicit approval".into(),
            },
        ),
    };
    let mut bytes = serde_json::to_vec(&first).unwrap();
    bytes.push(b'\n');
    bytes.extend(serde_json::to_vec(&second).unwrap());
    bytes.push(b'\n');
    std::fs::write(&path, bytes).unwrap();

    let error = load(&path).unwrap_err();
    assert!(matches!(
        error,
        Error::Invariant(reason)
            if reason == "plan approval approval-1 conflicts with an existing derived id"
    ));
}

#[test]
fn delegation_revocation_must_use_the_delegation_scope() {
    let path = temp_log("delegation-revocation-scope");
    let grant = human_request(
        "grant-event",
        "grant-key",
        0,
        Event::DelegationGranted {
            delegation_id: "delegation-1".into(),
            grantee: "agent".into(),
            transition_kinds: vec![TransitionKind::DecisionCommit],
        },
    );
    assert_eq!(
        commit(&path, grant).unwrap().status,
        CommitStatus::Committed
    );

    let mut revoke = human_request(
        "revoke-event",
        "revoke-key",
        1,
        Event::DelegationRevoked {
            delegation_id: "delegation-1".into(),
        },
    );
    revoke.scope = "another-repo".into();
    let outcome = commit(&path, revoke).unwrap();

    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.evaluation.reason_code, "SCOPE_MISMATCH");
    assert!(load(&path).unwrap().state().delegations["delegation-1"].active);
}

#[test]
fn plan_authorization_lifecycle_is_session_bound_and_revocable() {
    let path = temp_log("plan-lifecycle");
    let register = human_request(
        "plan-event-1",
        "plan-key-1",
        0,
        Event::PlanRegistered {
            plan_id: "plan-1".into(),
            objective: "Implement the approved slice".into(),
            acceptance_checks: vec!["focused tests pass".into()],
            unresolved_questions: vec!["Which follow-up slice comes next?".into()],
            intent_id: None,
        },
    );
    assert_eq!(
        commit(&path, register).unwrap().status,
        CommitStatus::Committed
    );
    let authorize = human_request(
        "authorize-event-1",
        "authorize-key-1",
        1,
        Event::PlanAuthorized {
            authorization_id: "authorization-1".into(),
            plan_id: "plan-1".into(),
            session_id: "session-1".into(),
            tool_name: "apply_patch".into(),
            authority_ref: None,
        },
    );
    assert_eq!(
        commit(&path, authorize).unwrap().status,
        CommitStatus::Committed
    );
    let log = load(&path).unwrap();
    let authorization = &log.state().plan_authorizations["authorization-1"];
    assert_eq!(authorization.session_id, "session-1");
    assert!(authorization.active);

    let mut unauthorized_revoke = human_request(
        "revoke-event-1",
        "revoke-key-1",
        2,
        Event::PlanAuthorizationRevoked {
            authorization_id: "authorization-1".into(),
        },
    );
    unauthorized_revoke.actor.kind = ActorKind::Agent;
    let denied = commit(&path, unauthorized_revoke).unwrap();
    assert_eq!(
        denied.evaluation.reason_code,
        "REVOCATION_AUTHORITY_REQUIRED"
    );

    let mut revoke = human_request(
        "revoke-event-2",
        "revoke-key-2",
        2,
        Event::PlanAuthorizationRevoked {
            authorization_id: "authorization-1".into(),
        },
    );
    revoke.actor.kind = ActorKind::Evidence;
    assert_eq!(
        commit(&path, revoke).unwrap().status,
        CommitStatus::Committed
    );
    assert!(!load(&path).unwrap().state().plan_authorizations["authorization-1"].active);
}

#[test]
fn delegated_agent_plan_authorization_requires_exact_capability() {
    let path = temp_log("delegated-plan");
    commit(
        &path,
        human_request(
            "plan-event-1",
            "key-1",
            0,
            Event::PlanRegistered {
                plan_id: "plan-1".into(),
                objective: "Implement a bounded change".into(),
                acceptance_checks: vec!["tests pass".into()],
                unresolved_questions: vec![],
                intent_id: None,
            },
        ),
    )
    .unwrap();
    commit(
        &path,
        human_request(
            "grant-event-1",
            "key-2",
            1,
            Event::DelegationGranted {
                delegation_id: "delegation-1".into(),
                grantee: "agent".into(),
                transition_kinds: vec![TransitionKind::PlanAuthorize],
            },
        ),
    )
    .unwrap();
    let mut authorize = human_request(
        "authorize-event-1",
        "key-3",
        2,
        Event::PlanAuthorized {
            authorization_id: "authorization-1".into(),
            plan_id: "plan-1".into(),
            session_id: "session-1".into(),
            tool_name: "apply_patch".into(),
            authority_ref: Some("delegation-1".into()),
        },
    );
    authorize.actor = Actor {
        kind: ActorKind::Agent,
        id: "agent".into(),
        provenance: "codex-session".into(),
    };
    assert_eq!(
        commit(&path, authorize).unwrap().status,
        CommitStatus::Committed
    );

    assert_eq!(
        commit(
            &path,
            human_request(
                "revoke-delegation-event-1",
                "key-4",
                3,
                Event::DelegationRevoked {
                    delegation_id: "delegation-1".into(),
                },
            ),
        )
        .unwrap()
        .status,
        CommitStatus::Committed
    );
    assert_eq!(
        commit(
            &path,
            human_request(
                "later-aporia-event-1",
                "key-5",
                4,
                Event::AporiaOpened {
                    aporia_id: "aporia-after-authorization".into(),
                    question: "Should the already authorized plan be reconsidered?".into(),
                    blocks: vec![TransitionKind::PlanAuthorize],
                    information_request: None,
                },
            ),
        )
        .unwrap()
        .status,
        CommitStatus::Committed
    );
    assert!(
        load(&path).unwrap().state().plan_authorizations["authorization-1"].active,
        "delegation revocation and later Aporia are not retroactive authorization revocations"
    );
}

#[test]
fn agent_plan_authorization_rejects_missing_or_inexact_delegation() {
    let mut state = State::default();
    state.plans.insert(
        "plan-1".into(),
        Plan {
            id: "plan-1".into(),
            objective: "Implement a bounded change".into(),
            scope: "repo".into(),
            acceptance_checks: vec!["tests pass".into()],
            unresolved_questions: vec![],
            intent_id: None,
            evidence_refs: vec![],
            assumption_claim_ids: vec![],
            completed: false,
        },
    );

    let request = |authority_ref: Option<&str>| CommitRequest {
        schema_version: SCHEMA_VERSION,
        event_id: "authorize-event-1".into(),
        idempotency_key: "authorize-key-1".into(),
        expected_revision: 0,
        actor: Actor {
            kind: ActorKind::Agent,
            id: "agent".into(),
            provenance: "codex-session".into(),
        },
        scope: "repo".into(),
        event: Event::PlanAuthorized {
            authorization_id: "authorization-1".into(),
            plan_id: "plan-1".into(),
            session_id: "session-1".into(),
            tool_name: "apply_patch".into(),
            authority_ref: authority_ref.map(str::to_owned),
        },
    };

    assert_eq!(
        evaluate(&state, &request(None)).reason_code,
        "DELEGATION_REQUIRED"
    );
    assert_eq!(
        evaluate(&state, &request(Some("missing"))).reason_code,
        "UNKNOWN_DELEGATION"
    );

    let delegation = Delegation {
        id: "delegation-1".into(),
        grantee: "agent".into(),
        scope: "repo".into(),
        transition_kinds: vec![TransitionKind::PlanAuthorize],
        active: true,
    };
    state
        .delegations
        .insert(delegation.id.clone(), delegation.clone());

    let mut invalid_variants = Vec::new();
    let mut inactive = delegation.clone();
    inactive.active = false;
    invalid_variants.push(inactive);
    let mut other_grantee = delegation.clone();
    other_grantee.grantee = "other-agent".into();
    invalid_variants.push(other_grantee);
    let mut other_scope = delegation.clone();
    other_scope.scope = "other".into();
    invalid_variants.push(other_scope);
    let mut other_transition = delegation;
    other_transition.transition_kinds = vec![TransitionKind::DecisionCommit];
    invalid_variants.push(other_transition);

    for invalid in invalid_variants {
        state.delegations.insert("delegation-1".into(), invalid);
        assert_eq!(
            evaluate(&state, &request(Some("delegation-1"))).reason_code,
            "DELEGATION_SCOPE_VIOLATION"
        );
    }
}

#[test]
fn material_aporia_blocks_plan_authorization() {
    let path = temp_log("plan-aporia");
    commit(
        &path,
        human_request(
            "plan-event-1",
            "key-1",
            0,
            Event::PlanRegistered {
                plan_id: "plan-1".into(),
                objective: "Implement a bounded change".into(),
                acceptance_checks: vec!["tests pass".into()],
                unresolved_questions: vec![],
                intent_id: None,
            },
        ),
    )
    .unwrap();
    commit(
        &path,
        human_request(
            "aporia-event-1",
            "key-2",
            1,
            Event::AporiaOpened {
                aporia_id: "aporia-1".into(),
                question: "Is the plan direction settled?".into(),
                blocks: vec![TransitionKind::PlanAuthorize],
                information_request: None,
            },
        ),
    )
    .unwrap();
    let outcome = commit(
        &path,
        human_request(
            "authorize-event-1",
            "key-3",
            2,
            Event::PlanAuthorized {
                authorization_id: "authorization-1".into(),
                plan_id: "plan-1".into(),
                session_id: "session-1".into(),
                tool_name: "apply_patch".into(),
                authority_ref: None,
            },
        ),
    )
    .unwrap();
    assert_eq!(outcome.evaluation.reason_code, "OPEN_MATERIAL_APORIA");
}

#[test]
fn agent_unknowns_require_a_bounded_information_plan_and_direct_resolution_evidence() {
    let path = temp_log("agent-information-request");
    let agent_request = |id: &str, key: &str, expected_revision: u64, event: Event| CommitRequest {
        schema_version: SCHEMA_VERSION,
        event_id: id.into(),
        idempotency_key: key.into(),
        expected_revision,
        actor: Actor {
            kind: ActorKind::Agent,
            id: "researcher".into(),
            provenance: "codex-session".into(),
        },
        scope: "repo".into(),
        event,
    };

    let without_plan = commit(
        &path,
        agent_request(
            "missing-plan",
            "missing-plan-key",
            0,
            Event::AporiaOpened {
                aporia_id: "unknown-api".into(),
                question: "Which API behavior is authoritative?".into(),
                blocks: vec![TransitionKind::DecisionCommit],
                information_request: None,
            },
        ),
    )
    .unwrap();
    assert_eq!(
        without_plan.evaluation.reason_code,
        "AGENT_INFORMATION_REQUEST_REQUIRED"
    );

    let opened = commit(
        &path,
        agent_request(
            "open-request",
            "open-request-key",
            0,
            Event::AporiaOpened {
                aporia_id: "unknown-api".into(),
                question: "Which API behavior is authoritative?".into(),
                blocks: vec![TransitionKind::DecisionCommit],
                information_request: Some(InformationRequest {
                    requested_materials: vec![],
                    collection_method: Some(
                        "inspect the pinned official API reference and executable tests".into(),
                    ),
                    selection_criteria: vec![
                        "primary source".into(),
                        "matches the pinned version".into(),
                    ],
                    intended_use: "choose the compatible implementation path".into(),
                }),
            },
        ),
    )
    .unwrap();
    assert_eq!(opened.status, CommitStatus::Committed);

    let unresolved = commit(
        &path,
        human_request(
            "resolve-without-evidence",
            "resolve-without-evidence-key",
            1,
            Event::AporiaResolved {
                aporia_id: "unknown-api".into(),
                resolution_ref: "missing-evidence".into(),
            },
        ),
    )
    .unwrap();
    assert_eq!(
        unresolved.evaluation.reason_code,
        "UNKNOWN_RESOLUTION_EVIDENCE"
    );

    assert_eq!(
        commit(
            &path,
            agent_request(
                "record-source",
                "record-source-key",
                1,
                Event::EvidenceRecorded {
                    evidence_id: "official-reference".into(),
                    kind: EvidenceKind::ExternalSource,
                    locator: "official-docs:pinned-version".into(),
                    digest: None,
                },
            ),
        )
        .unwrap()
        .status,
        CommitStatus::Committed
    );
    assert_eq!(
        commit(
            &path,
            human_request(
                "resolve-with-evidence",
                "resolve-with-evidence-key",
                2,
                Event::AporiaResolved {
                    aporia_id: "unknown-api".into(),
                    resolution_ref: "official-reference".into(),
                },
            ),
        )
        .unwrap()
        .status,
        CommitStatus::Committed
    );
}

fn human_request(id: &str, key: &str, expected_revision: u64, event: Event) -> CommitRequest {
    CommitRequest {
        schema_version: SCHEMA_VERSION,
        event_id: id.into(),
        idempotency_key: key.into(),
        expected_revision,
        actor: Actor {
            kind: ActorKind::Human,
            id: "user".into(),
            provenance: "trusted-host-event".into(),
        },
        scope: "repo".into(),
        event,
    }
}

fn open_aporia(id: &str, key: &str, expected_revision: u64) -> CommitRequest {
    human_request(
        id,
        key,
        expected_revision,
        Event::AporiaOpened {
            aporia_id: id.into(),
            question: "Which persistence format?".into(),
            blocks: vec![TransitionKind::DecisionCommit],
            information_request: None,
        },
    )
}

#[test]
fn replays_committed_events() {
    let path = temp_log("replay");
    let outcome = commit(&path, open_aporia("a1", "k1", 0)).unwrap();
    assert_eq!(outcome.status, CommitStatus::Committed);
    let log = load(&path).unwrap();
    assert_eq!(log.state().revision, 1);
    assert!(log.state().aporias.contains_key("a1"));
}

#[test]
fn rejects_stale_revision_without_appending() {
    let path = temp_log("stale");
    commit(&path, open_aporia("a1", "k1", 0)).unwrap();
    let outcome = commit(&path, open_aporia("a2", "k2", 0)).unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.evaluation.reason_code, "STALE_REVISION");
    assert_eq!(load(&path).unwrap().state().revision, 1);
}

#[test]
fn identical_retry_returns_original_revision() {
    let path = temp_log("retry");
    let request = open_aporia("a1", "k1", 0);
    assert_eq!(
        commit(&path, request.clone()).unwrap().status,
        CommitStatus::Committed
    );
    let retry = commit(&path, request).unwrap();
    assert_eq!(retry.status, CommitStatus::Duplicate);
    assert_eq!(retry.revision, 1);
    assert_eq!(load(&path).unwrap().state().revision, 1);
}

#[test]
fn conflicting_idempotency_key_is_rejected() {
    let path = temp_log("key-conflict");
    commit(&path, open_aporia("a1", "same", 0)).unwrap();
    let outcome = commit(&path, open_aporia("a2", "same", 1)).unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.evaluation.reason_code, "IDEMPOTENCY_KEY_CONFLICT");
}

#[test]
fn incomplete_final_record_stops_replay_and_writes() {
    let path = temp_log("partial");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"{\"sequence\":1").unwrap();
    assert!(matches!(load(&path), Err(Error::CorruptLog { .. })));
    assert!(matches!(
        commit(&path, open_aporia("a1", "k1", 0)),
        Err(Error::CorruptLog { .. })
    ));
}

#[test]
fn concurrent_same_revision_has_one_winner_and_valid_log() {
    let path = Arc::new(temp_log("concurrent"));
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for index in 0..2 {
        let path = Arc::clone(&path);
        let barrier = Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            commit(
                Path::new(&*path),
                open_aporia(&format!("a{index}"), &format!("k{index}"), 0),
            )
            .unwrap()
        }));
    }
    barrier.wait();
    let outcomes: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| outcome.status == CommitStatus::Committed)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| outcome.evaluation.reason_code == "STALE_REVISION")
            .count(),
        1
    );
    assert_eq!(load(Path::new(&*path)).unwrap().state().revision, 1);
}

#[test]
fn agent_cannot_commit_decision_without_delegation() {
    let path = temp_log("forged-authority");
    let request = CommitRequest {
        schema_version: SCHEMA_VERSION,
        event_id: "d1".into(),
        idempotency_key: "k1".into(),
        expected_revision: 0,
        actor: Actor {
            kind: ActorKind::Agent,
            id: "agent".into(),
            provenance: "self-asserted".into(),
        },
        scope: "repo".into(),
        event: Event::DecisionCommitted {
            decision_id: "d1".into(),
            question: "Use JSONL?".into(),
            value: "yes".into(),
            authority_ref: None,
        },
    };
    let outcome = commit(&path, request).unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.evaluation.verdict, Verdict::Deny);
    assert_eq!(outcome.evaluation.reason_code, "DELEGATION_REQUIRED");
}

#[test]
fn delegated_agent_can_commit_only_in_exact_scope() {
    let path = temp_log("delegated");
    let grant = human_request(
        "grant-1",
        "key-1",
        0,
        Event::DelegationGranted {
            delegation_id: "delegation-1".into(),
            grantee: "agent".into(),
            transition_kinds: vec![TransitionKind::DecisionCommit],
        },
    );
    assert_eq!(
        commit(&path, grant).unwrap().status,
        CommitStatus::Committed
    );

    let mut decision = human_request(
        "decision-1",
        "key-2",
        1,
        Event::DecisionCommitted {
            decision_id: "decision-1".into(),
            question: "Use JSONL?".into(),
            value: "yes".into(),
            authority_ref: Some("delegation-1".into()),
        },
    );
    decision.actor = Actor {
        kind: ActorKind::Agent,
        id: "agent".into(),
        provenance: "codex-session".into(),
    };
    assert_eq!(
        commit(&path, decision.clone()).unwrap().status,
        CommitStatus::Committed
    );

    decision.event_id = "decision-2".into();
    decision.idempotency_key = "key-3".into();
    decision.expected_revision = 2;
    decision.scope = "other-repo".into();
    if let Event::DecisionCommitted { decision_id, .. } = &mut decision.event {
        *decision_id = "decision-2".into();
    }
    let rejected = commit(&path, decision).unwrap();
    assert_eq!(rejected.status, CommitStatus::Rejected);
    assert_eq!(
        rejected.evaluation.reason_code,
        "DELEGATION_SCOPE_VIOLATION"
    );
}

#[test]
fn open_aporia_blocks_dependent_transition() {
    let path = temp_log("blocked");
    commit(&path, open_aporia("a1", "key-1", 0)).unwrap();
    let decision = human_request(
        "decision-1",
        "key-2",
        1,
        Event::DecisionCommitted {
            decision_id: "decision-1".into(),
            question: "Use JSONL?".into(),
            value: "yes".into(),
            authority_ref: None,
        },
    );
    let outcome = commit(&path, decision).unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.evaluation.reason_code, "OPEN_MATERIAL_APORIA");
}

#[test]
fn agent_cannot_resolve_aporia_with_self_asserted_reference() {
    let path = temp_log("resolution-authority");
    commit(&path, open_aporia("a1", "key-1", 0)).unwrap();
    let mut resolution = human_request(
        "resolve-1",
        "key-2",
        1,
        Event::AporiaResolved {
            aporia_id: "a1".into(),
            resolution_ref: "self-asserted".into(),
        },
    );
    resolution.actor = Actor {
        kind: ActorKind::Agent,
        id: "agent".into(),
        provenance: "self-asserted".into(),
    };
    let outcome = commit(&path, resolution).unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(
        outcome.evaluation.reason_code,
        "RESOLUTION_AUTHORITY_REQUIRED"
    );
    assert!(
        load(&path).unwrap().state().aporias["a1"]
            .resolution_ref
            .is_none()
    );
}

#[test]
fn unknown_aporia_resolution_is_structured_rejection() {
    let path = temp_log("unknown-resolution");
    commit(&path, open_aporia("a1", "key-1", 0)).unwrap();
    let outcome = commit(
        &path,
        human_request(
            "resolve-1",
            "key-2",
            1,
            Event::AporiaResolved {
                aporia_id: "missing".into(),
                resolution_ref: "evidence-1".into(),
            },
        ),
    )
    .unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.evaluation.reason_code, "UNKNOWN_APORIA");
}

#[test]
fn empty_nested_fields_are_rejected() {
    let path = temp_log("empty-fields");
    let outcome = commit(
        &path,
        human_request(
            "grant-1",
            "key-1",
            0,
            Event::DelegationGranted {
                delegation_id: "".into(),
                grantee: "".into(),
                transition_kinds: vec![],
            },
        ),
    )
    .unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.evaluation.reason_code, "MISSING_REQUIRED_FIELD");
}

#[test]
fn event_id_cannot_be_reused_with_another_key() {
    let path = temp_log("event-id");
    commit(&path, open_aporia("a1", "key-1", 0)).unwrap();
    let outcome = commit(&path, open_aporia("a1", "key-2", 1)).unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.evaluation.reason_code, "EVENT_ID_CONFLICT");
}

#[test]
fn accepted_risk_preserves_scope_actor_and_review_condition() {
    let path = temp_log("risk-state");
    let outcome = commit(
        &path,
        human_request(
            "risk-event-1",
            "key-1",
            0,
            Event::RiskAccepted {
                risk_id: "risk-1".into(),
                subject: "one test is failing".into(),
                review_condition: "recheck before release".into(),
            },
        ),
    )
    .unwrap();
    assert_eq!(outcome.status, CommitStatus::Committed);
    let log = load(&path).unwrap();
    let risk = &log.state().accepted_risks["risk-1"];
    assert_eq!(risk.scope, "repo");
    assert_eq!(risk.actor.id, "user");
    assert_eq!(risk.review_condition, "recheck before release");
}
