use aporic::codex::{GatePolicy, PreToolUseInput, pre_tool_use_transaction};
use aporic::governance::{Action, GateStatus, evaluate_action};
use aporic::policy::{LifecycleMode, LowRiskProfileRef, PolicyDocument, ToolPolicy};
use aporic::{
    Actor, ActorKind, CommitRequest, CommitStatus, Error, Event, EvidenceKind, PlanRequirement,
    RequirementDispositionKind, RequirementState, RiskLevel, RiskSnapshot, SCHEMA_VERSION,
    StoredEvent, ToolHold, commit, initialize, load, migrate_v6_to_v7,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "aporic-v080-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn request(revision: u64, id: &str, actor_kind: ActorKind, event: Event) -> CommitRequest {
    CommitRequest {
        schema_version: SCHEMA_VERSION,
        event_id: id.into(),
        idempotency_key: id.into(),
        expected_revision: revision,
        actor: Actor {
            kind: actor_kind,
            id: "tester".into(),
            provenance: "v080-test".into(),
        },
        scope: "repo".into(),
        event,
    }
}

fn intent() -> Event {
    Event::IntentEnvelopeRecorded {
        intent_id: "intent-1".into(),
        source_ref: "conversation:test".into(),
        goal: "exercise governed plans".into(),
        explicit_items: vec!["test the state transitions".into()],
        inferred_items: vec![],
        unknown_items: vec![],
    }
}

fn requirements() -> Vec<PlanRequirement> {
    vec![
        PlanRequirement {
            requirement_id: "design".into(),
            control_id: "control:design".into(),
            depends_on: vec![],
            non_waivable: true,
        },
        PlanRequirement {
            requirement_id: "verify".into(),
            control_id: "control:verify".into(),
            depends_on: vec!["design".into()],
            non_waivable: false,
        },
        PlanRequirement {
            requirement_id: "docs".into(),
            control_id: "control:docs".into(),
            depends_on: vec![],
            non_waivable: false,
        },
    ]
}

fn governed_plan(requirements: Vec<PlanRequirement>) -> Event {
    Event::GovernedPlanRegistered {
        plan_id: "plan-1".into(),
        predecessor_plan_id: None,
        intent_id: "intent-1".into(),
        objective: "implement governed state".into(),
        acceptance_checks: vec!["state transitions pass".into()],
        unresolved_questions: vec![],
        risk_snapshot: RiskSnapshot {
            profile_id: "local-code".into(),
            profile_version: "1".into(),
            risk_level: RiskLevel::Medium,
            basis_refs: vec!["policy:local-code-v1".into()],
        },
        requirements,
    }
}

fn risk_policy(
    profiles: Vec<LowRiskProfileRef>,
    require_plan: bool,
    require_grant: bool,
) -> PolicyDocument {
    PolicyDocument {
        schema_version: 3,
        lifecycle_mode: None,
        tools: BTreeMap::from([(
            "apply_patch".into(),
            ToolPolicy {
                require_plan,
                require_grant,
                require_intent: Some(true),
                auto_allow_low_risk_profiles: Some(profiles),
            },
        )]),
    }
}

fn local_code_profile(version: &str) -> LowRiskProfileRef {
    LowRiskProfileRef {
        profile_id: "local-code".into(),
        profile_version: version.into(),
    }
}

fn evidence(id: &str) -> Event {
    Event::EvidenceRecorded {
        evidence_id: id.into(),
        kind: EvidenceKind::TestResult,
        locator: format!("test:{id}"),
        digest: None,
    }
}

fn disposition(
    id: &str,
    requirement_id: &str,
    kind: RequirementDispositionKind,
    evidence_refs: Vec<&str>,
    compensating_controls: Vec<&str>,
) -> Event {
    Event::RequirementDispositionRecorded {
        resolution_id: id.into(),
        plan_id: "plan-1".into(),
        requirement_id: requirement_id.into(),
        disposition: kind,
        rationale: "verified by the focused fixture".into(),
        evidence_refs: evidence_refs.into_iter().map(str::to_owned).collect(),
        compensating_control_ids: compensating_controls
            .into_iter()
            .map(str::to_owned)
            .collect(),
        accepted_risk_id: None,
        supersedes_resolution_id: None,
        authority_ref: None,
    }
}

#[test]
fn governed_plan_rejects_dangling_and_cyclic_dependencies() {
    let store = path("invalid-graphs").join("events.jsonl");
    initialize(&store).unwrap();
    commit(&store, request(0, "intent", ActorKind::Human, intent())).unwrap();

    let dangling = governed_plan(vec![PlanRequirement {
        requirement_id: "a".into(),
        control_id: "control:a".into(),
        depends_on: vec!["missing".into()],
        non_waivable: false,
    }]);
    let outcome = commit(&store, request(1, "dangling", ActorKind::Agent, dangling)).unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.evaluation.reason_code, "INVALID_GOVERNED_PLAN");

    let cycle = governed_plan(vec![
        PlanRequirement {
            requirement_id: "a".into(),
            control_id: "control:a".into(),
            depends_on: vec!["b".into()],
            non_waivable: false,
        },
        PlanRequirement {
            requirement_id: "b".into(),
            control_id: "control:b".into(),
            depends_on: vec!["a".into()],
            non_waivable: false,
        },
    ]);
    let outcome = commit(&store, request(1, "cycle", ActorKind::Agent, cycle)).unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(
        outcome.evaluation.reason_code,
        "REQUIREMENT_DEPENDENCY_CYCLE"
    );
    assert_eq!(load(&store).unwrap().state().revision, 1);
}

#[test]
fn governed_plan_rejects_oversized_graphs_before_traversal() {
    let store = path("oversized-graphs").join("events.jsonl");
    initialize(&store).unwrap();
    commit(&store, request(0, "intent", ActorKind::Human, intent())).unwrap();

    let wide = (0..=aporic::MAX_GOVERNED_PLAN_REQUIREMENTS)
        .map(|index| PlanRequirement {
            requirement_id: format!("requirement-{index}"),
            control_id: format!("control:{index}"),
            depends_on: vec![],
            non_waivable: false,
        })
        .collect();
    let outcome = commit(
        &store,
        request(1, "wide", ActorKind::Agent, governed_plan(wide)),
    )
    .unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(
        outcome.evaluation.reason_code,
        "GOVERNED_PLAN_GRAPH_TOO_LARGE"
    );

    let mut high_fan_in = (0..=aporic::MAX_REQUIREMENT_DEPENDENCIES)
        .map(|index| PlanRequirement {
            requirement_id: format!("dependency-{index}"),
            control_id: format!("control:dependency-{index}"),
            depends_on: vec![],
            non_waivable: false,
        })
        .collect::<Vec<_>>();
    high_fan_in.push(PlanRequirement {
        requirement_id: "target".into(),
        control_id: "control:target".into(),
        depends_on: (0..=aporic::MAX_REQUIREMENT_DEPENDENCIES)
            .map(|index| format!("dependency-{index}"))
            .collect(),
        non_waivable: false,
    });
    let outcome = commit(
        &store,
        request(
            1,
            "high-fan-in",
            ActorKind::Agent,
            governed_plan(high_fan_in),
        ),
    )
    .unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(
        outcome.evaluation.reason_code,
        "GOVERNED_PLAN_GRAPH_TOO_LARGE"
    );
}

#[test]
fn governed_plan_lineage_allows_only_one_successor() {
    let store = path("linear-lineage").join("events.jsonl");
    initialize(&store).unwrap();
    commit(&store, request(0, "intent", ActorKind::Human, intent())).unwrap();
    commit(
        &store,
        request(1, "plan-1", ActorKind::Agent, governed_plan(requirements())),
    )
    .unwrap();

    let successor = |plan_id: &str| {
        let mut event = governed_plan(requirements());
        if let Event::GovernedPlanRegistered {
            plan_id: event_plan_id,
            predecessor_plan_id,
            ..
        } = &mut event
        {
            *event_plan_id = plan_id.into();
            *predecessor_plan_id = Some("plan-1".into());
        }
        event
    };
    commit(
        &store,
        request(2, "plan-2", ActorKind::Agent, successor("plan-2")),
    )
    .unwrap();
    let fork = commit(
        &store,
        request(3, "plan-3", ActorKind::Agent, successor("plan-3")),
    )
    .unwrap();
    assert_eq!(fork.status, CommitStatus::Rejected);
    assert_eq!(fork.evaluation.reason_code, "PLAN_SUCCESSOR_ALREADY_EXISTS");

    commit(
        &store,
        request(
            3,
            "supersede-plan-1",
            ActorKind::Human,
            Event::GovernedPlanSuperseded {
                plan_id: "plan-1".into(),
                successor_plan_id: "plan-2".into(),
                reason: "replace the governed plan".into(),
            },
        ),
    )
    .unwrap();
    let late_successor = commit(
        &store,
        request(4, "plan-4", ActorKind::Agent, successor("plan-4")),
    )
    .unwrap();
    assert_eq!(late_successor.status, CommitStatus::Rejected);
    assert_eq!(late_successor.evaluation.reason_code, "PLAN_SUPERSEDED");
}

#[test]
fn policy_v3_rejects_unsafe_low_risk_auto_allow_combinations() {
    let valid = risk_policy(vec![local_code_profile("1")], true, false);
    valid.validate().unwrap();

    let mut grant_bypass = valid.clone();
    grant_bypass
        .tools
        .get_mut("apply_patch")
        .unwrap()
        .require_grant = true;
    assert!(grant_bypass.validate().is_err());

    let mut no_intent = valid.clone();
    no_intent
        .tools
        .get_mut("apply_patch")
        .unwrap()
        .require_intent = Some(false);
    assert!(no_intent.validate().is_err());

    let mut duplicate = valid.clone();
    duplicate
        .tools
        .get_mut("apply_patch")
        .unwrap()
        .auto_allow_low_risk_profiles
        .as_mut()
        .unwrap()
        .push(local_code_profile("1"));
    assert!(duplicate.validate().is_err());

    let mut mislabeled_v2 = valid;
    mislabeled_v2.schema_version = 2;
    assert!(mislabeled_v2.validate().is_err());

    let too_many = risk_policy(
        (0..=aporic::policy::MAX_AUTO_ALLOWED_LOW_RISK_PROFILES)
            .map(|index| LowRiskProfileRef {
                profile_id: format!("local-code-{index}"),
                profile_version: "1".into(),
            })
            .collect(),
        true,
        false,
    );
    assert!(too_many.validate().is_err());

    let mut missing_mode_v4 = risk_policy(Vec::new(), true, false);
    missing_mode_v4.schema_version = 4;
    assert!(missing_mode_v4.validate().is_err());

    let mut mislabeled_v3 = risk_policy(Vec::new(), true, false);
    mislabeled_v3.lifecycle_mode = Some(LifecycleMode::Development);
    assert!(mislabeled_v3.validate().is_err());

    let too_many_tools = PolicyDocument {
        schema_version: 4,
        lifecycle_mode: Some(LifecycleMode::Development),
        tools: (0..=aporic::policy::MAX_POLICY_TOOLS)
            .map(|index| {
                (
                    format!("tool-{index}"),
                    ToolPolicy {
                        require_plan: false,
                        require_grant: false,
                        require_intent: Some(false),
                        auto_allow_low_risk_profiles: Some(Vec::new()),
                    },
                )
            })
            .collect(),
    };
    assert!(too_many_tools.validate().is_err());
}

#[test]
fn ready_low_risk_plan_is_auto_allowed_without_bypassing_other_gates() {
    let store = path("low-risk-auto-allow").join("events.jsonl");
    initialize(&store).unwrap();
    commit(&store, request(0, "intent", ActorKind::Human, intent())).unwrap();
    let mut plan = governed_plan(requirements());
    if let Event::GovernedPlanRegistered { risk_snapshot, .. } = &mut plan {
        risk_snapshot.risk_level = RiskLevel::Low;
    }
    commit(&store, request(1, "plan", ActorKind::Agent, plan)).unwrap();

    let policy = risk_policy(vec![local_code_profile("1")], true, false);
    let action = Action {
        scope: "repo",
        session_id: "session-1",
        tool_name: "apply_patch",
        tool_use_id: Some("tool-use-1"),
        tool_input: Some(&json!({"patch": "x"})),
    };
    let pending = load(&store).unwrap();
    let evaluation = evaluate_action(pending.state(), &policy, action)
        .unwrap()
        .unwrap();
    assert_eq!(evaluation.status, GateStatus::PlanNotExecutionEligible);

    for (revision, id) in [(2, "e-design"), (3, "e-verify"), (4, "e-docs")] {
        commit(
            &store,
            request(revision, id, ActorKind::Evidence, evidence(id)),
        )
        .unwrap();
    }
    commit(
        &store,
        request(
            5,
            "resolve-design",
            ActorKind::Evidence,
            disposition(
                "resolution-design",
                "design",
                RequirementDispositionKind::Satisfied,
                vec!["e-design"],
                vec![],
            ),
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            6,
            "resolve-verify",
            ActorKind::Human,
            disposition(
                "resolution-verify",
                "verify",
                RequirementDispositionKind::Tailored,
                vec!["e-verify"],
                vec!["control:independent-check"],
            ),
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            7,
            "resolve-docs",
            ActorKind::Evidence,
            disposition(
                "resolution-docs",
                "docs",
                RequirementDispositionKind::Satisfied,
                vec!["e-docs"],
                vec![],
            ),
        ),
    )
    .unwrap();

    let loaded = load(&store).unwrap();
    let evaluation = evaluate_action(loaded.state(), &policy, action)
        .unwrap()
        .unwrap();
    assert_eq!(evaluation.status, GateStatus::Allowed);
    assert_eq!(evaluation.matching_plan_authorization_count, 0);
    assert_eq!(evaluation.matching_execution_grant_count, 0);

    let wrong_version = risk_policy(vec![local_code_profile("2")], true, false);
    assert_eq!(
        evaluate_action(loaded.state(), &wrong_version, action)
            .unwrap()
            .unwrap()
            .status,
        GateStatus::PlanAuthorizationRequired
    );

    let exact_grant = risk_policy(Vec::new(), false, true);
    exact_grant.validate().unwrap();
    assert_eq!(
        evaluate_action(loaded.state(), &exact_grant, action)
            .unwrap()
            .unwrap()
            .status,
        GateStatus::ExecutionGrantRequired
    );

    let mut maintenance = policy.clone();
    maintenance.schema_version = 4;
    maintenance.lifecycle_mode = Some(LifecycleMode::Maintenance);
    maintenance.validate().unwrap();
    assert_eq!(
        evaluate_action(loaded.state(), &maintenance, action)
            .unwrap()
            .unwrap()
            .status,
        GateStatus::ExecutionGrantRequired
    );
    commit(
        &store,
        request(
            8,
            "maintenance-grant",
            ActorKind::Human,
            Event::ExecutionGrantIssued {
                grant_id: "maintenance-grant".into(),
                plan_id: "plan-1".into(),
                session_id: "session-1".into(),
                tool_name: "apply_patch".into(),
                tool_input: json!({"patch": "x"}),
                max_uses: 1,
                authority_ref: Some("conversation:maintenance-approval".into()),
            },
        ),
    )
    .unwrap();
    let granted = load(&store).unwrap();
    assert_eq!(
        evaluate_action(granted.state(), &maintenance, action)
            .unwrap()
            .unwrap()
            .status,
        GateStatus::Allowed
    );
    let maintenance_gate = GatePolicy::from_document(maintenance.clone()).unwrap();
    let admitted = PreToolUseInput {
        session_id: "session-1".into(),
        hook_event_name: "PreToolUse".into(),
        cwd: "/repo".into(),
        turn_id: "turn-1".into(),
        tool_name: "apply_patch".into(),
        tool_use_id: "tool-use-1".into(),
        tool_input: json!({"patch": "x"}),
        model: None,
        permission_mode: None,
    };
    assert!(
        pre_tool_use_transaction(&store, &admitted, "repo", &maintenance_gate)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        load(&store).unwrap().state().execution_grants["maintenance-grant"].consumed_uses,
        1
    );
    let exhausted = PreToolUseInput {
        tool_use_id: "tool-use-2".into(),
        ..admitted
    };
    let denied = pre_tool_use_transaction(&store, &exhausted, "repo", &maintenance_gate)
        .unwrap()
        .unwrap();
    assert!(
        denied
            .hook_specific_output
            .permission_decision_reason
            .starts_with("APORIC_EXECUTION_GRANT_REQUIRED")
    );
    let mismatched_input = json!({"patch": "different"});
    assert_eq!(
        evaluate_action(
            granted.state(),
            &maintenance,
            Action {
                tool_input: Some(&mismatched_input),
                ..action
            },
        )
        .unwrap()
        .unwrap()
        .status,
        GateStatus::ExecutionGrantRequired
    );

    let mut completed = loaded.state().clone();
    completed.plans.get_mut("plan-1").unwrap().completed = true;
    assert_eq!(
        evaluate_action(&completed, &policy, action)
            .unwrap()
            .unwrap()
            .status,
        GateStatus::PlanNotExecutionEligible
    );

    let mut medium = loaded.state().clone();
    medium
        .governed_plans
        .get_mut("plan-1")
        .unwrap()
        .risk_snapshot
        .risk_level = RiskLevel::Medium;
    assert_eq!(
        evaluate_action(&medium, &policy, action)
            .unwrap()
            .unwrap()
            .status,
        GateStatus::PlanAuthorizationRequired
    );

    let mut held = loaded.state().clone();
    held.tool_holds.insert(
        "hold-1".into(),
        ToolHold {
            id: "hold-1".into(),
            tool_name: "apply_patch".into(),
            reason: "pause low-risk execution".into(),
            scope: "repo".into(),
            active: true,
        },
    );
    assert_eq!(
        evaluate_action(&held, &policy, action)
            .unwrap()
            .unwrap()
            .status,
        GateStatus::Held
    );
}

#[test]
fn invalidation_stales_only_the_dependency_closure_and_blocks_existing_authority() {
    let store = path("stale-closure").join("events.jsonl");
    initialize(&store).unwrap();
    commit(&store, request(0, "intent", ActorKind::Human, intent())).unwrap();
    commit(
        &store,
        request(1, "plan", ActorKind::Agent, governed_plan(requirements())),
    )
    .unwrap();
    for (revision, id) in [(2, "e-design"), (3, "e-verify"), (4, "e-docs")] {
        commit(
            &store,
            request(revision, id, ActorKind::Evidence, evidence(id)),
        )
        .unwrap();
    }
    let premature = commit(
        &store,
        request(
            5,
            "premature-verify",
            ActorKind::Human,
            disposition(
                "resolution-premature",
                "verify",
                RequirementDispositionKind::Tailored,
                vec!["e-verify"],
                vec!["control:independent-check"],
            ),
        ),
    )
    .unwrap();
    assert_eq!(
        premature.evaluation.reason_code,
        "REQUIREMENT_DEPENDENCY_NOT_RESOLVED"
    );
    commit(
        &store,
        request(
            5,
            "resolve-design",
            ActorKind::Evidence,
            disposition(
                "resolution-design",
                "design",
                RequirementDispositionKind::Satisfied,
                vec!["e-design"],
                vec![],
            ),
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            6,
            "resolve-verify",
            ActorKind::Human,
            disposition(
                "resolution-verify",
                "verify",
                RequirementDispositionKind::Tailored,
                vec!["e-verify"],
                vec!["control:independent-check"],
            ),
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            7,
            "resolve-docs",
            ActorKind::Evidence,
            disposition(
                "resolution-docs",
                "docs",
                RequirementDispositionKind::Satisfied,
                vec!["e-docs"],
                vec![],
            ),
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            8,
            "authorize",
            ActorKind::Human,
            Event::PlanAuthorized {
                authorization_id: "authorization-1".into(),
                plan_id: "plan-1".into(),
                session_id: "session-1".into(),
                tool_name: "apply_patch".into(),
                authority_ref: Some("conversation:approval".into()),
            },
        ),
    )
    .unwrap();

    let policy = PolicyDocument {
        schema_version: 2,
        lifecycle_mode: None,
        tools: BTreeMap::from([(
            "apply_patch".into(),
            ToolPolicy {
                require_plan: true,
                require_grant: false,
                require_intent: Some(true),
                auto_allow_low_risk_profiles: None,
            },
        )]),
    };
    let state = load(&store).unwrap();
    assert!(state.state().governed_plan_is_ready("plan-1"));
    assert_eq!(
        evaluate_action(
            state.state(),
            &policy,
            Action {
                scope: "repo",
                session_id: "session-1",
                tool_name: "apply_patch",
                tool_use_id: Some("tool-use-1"),
                tool_input: Some(&json!({"patch": "x"})),
            },
        )
        .unwrap()
        .unwrap()
        .status,
        GateStatus::Allowed
    );

    commit(
        &store,
        request(
            9,
            "invalidate-design",
            ActorKind::Evidence,
            Event::RequirementDispositionInvalidated {
                resolution_id: "resolution-design".into(),
                plan_id: "plan-1".into(),
                requirement_id: "design".into(),
                reason: "the design premise changed".into(),
                trigger_ref: "evidence:new-design".into(),
            },
        ),
    )
    .unwrap();
    let state = load(&store).unwrap();
    assert_eq!(
        state.state().governed_requirement_state("plan-1", "design"),
        Some(RequirementState::Stale)
    );
    assert_eq!(
        state.state().governed_requirement_state("plan-1", "verify"),
        Some(RequirementState::Stale)
    );
    assert_eq!(
        state.state().governed_requirement_state("plan-1", "docs"),
        Some(RequirementState::Resolved)
    );
    let evaluation = evaluate_action(
        state.state(),
        &policy,
        Action {
            scope: "repo",
            session_id: "session-1",
            tool_name: "apply_patch",
            tool_use_id: Some("tool-use-2"),
            tool_input: Some(&json!({"patch": "x"})),
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(evaluation.status, GateStatus::PlanNotExecutionEligible);
    assert_eq!(evaluation.reason_code, "PLAN_NOT_EXECUTION_ELIGIBLE");

    let mut replacement = disposition(
        "resolution-design-2",
        "design",
        RequirementDispositionKind::Satisfied,
        vec!["e-design"],
        vec![],
    );
    if let Event::RequirementDispositionRecorded {
        supersedes_resolution_id,
        ..
    } = &mut replacement
    {
        *supersedes_resolution_id = Some("resolution-design".into());
    }
    commit(
        &store,
        request(10, "replace-design", ActorKind::Evidence, replacement),
    )
    .unwrap();
    let state = load(&store).unwrap();
    assert_eq!(
        state.state().governed_requirement_state("plan-1", "design"),
        Some(RequirementState::Resolved)
    );
    assert_eq!(
        state.state().governed_requirement_state("plan-1", "verify"),
        Some(RequirementState::Stale),
        "a dependent requirement must be explicitly reevaluated"
    );
}

#[test]
fn replay_rejects_authority_preissued_for_a_pending_governed_plan() {
    let store = path("raw-replay-bypass").join("events.jsonl");
    std::fs::create_dir_all(store.parent().unwrap()).unwrap();
    let records = [
        StoredEvent {
            sequence: 1,
            request: request(0, "intent", ActorKind::Human, intent()),
        },
        StoredEvent {
            sequence: 2,
            request: request(1, "plan", ActorKind::Agent, governed_plan(requirements())),
        },
        StoredEvent {
            sequence: 3,
            request: request(
                2,
                "preissued-authority",
                ActorKind::Human,
                Event::PlanAuthorized {
                    authorization_id: "authorization-preissued".into(),
                    plan_id: "plan-1".into(),
                    session_id: "session-1".into(),
                    tool_name: "apply_patch".into(),
                    authority_ref: Some("conversation:approval".into()),
                },
            ),
        },
    ];
    let contents = records
        .iter()
        .map(|record| serde_json::to_string(record).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(&store, contents).unwrap();

    match load(&store) {
        Err(Error::Invariant(reason)) => {
            assert!(reason.contains("PLAN_NOT_EXECUTION_ELIGIBLE"), "{reason}");
        }
        other => panic!("expected governed replay invariant, got {other:?}"),
    }
}

#[test]
fn non_waivable_requirement_rejects_a_human_waiver() {
    let store = path("non-waivable").join("events.jsonl");
    initialize(&store).unwrap();
    commit(&store, request(0, "intent", ActorKind::Human, intent())).unwrap();
    commit(
        &store,
        request(1, "plan", ActorKind::Agent, governed_plan(requirements())),
    )
    .unwrap();
    commit(
        &store,
        request(
            2,
            "risk",
            ActorKind::Human,
            Event::RiskAccepted {
                risk_id: "risk-1".into(),
                subject: "skip design".into(),
                review_condition: "before release".into(),
            },
        ),
    )
    .unwrap();
    let mut waiver = disposition(
        "resolution-waiver",
        "design",
        RequirementDispositionKind::Waived,
        vec![],
        vec![],
    );
    if let Event::RequirementDispositionRecorded {
        accepted_risk_id, ..
    } = &mut waiver
    {
        *accepted_risk_id = Some("risk-1".into());
    }
    let outcome = commit(&store, request(3, "waiver", ActorKind::Human, waiver)).unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.evaluation.reason_code, "REQUIREMENT_NON_WAIVABLE");
}

#[test]
fn disposition_requires_direct_evidence_exact_authority_and_stale_supersession() {
    let store = path("disposition-contract").join("events.jsonl");
    initialize(&store).unwrap();
    commit(&store, request(0, "intent", ActorKind::Human, intent())).unwrap();
    commit(
        &store,
        request(
            1,
            "plan",
            ActorKind::Agent,
            governed_plan(vec![PlanRequirement {
                requirement_id: "verify".into(),
                control_id: "control:verify".into(),
                depends_on: vec![],
                non_waivable: false,
            }]),
        ),
    )
    .unwrap();

    let missing_evidence = commit(
        &store,
        request(
            2,
            "missing-evidence",
            ActorKind::Evidence,
            disposition(
                "resolution-missing",
                "verify",
                RequirementDispositionKind::Satisfied,
                vec![],
                vec![],
            ),
        ),
    )
    .unwrap();
    assert_eq!(
        missing_evidence.evaluation.reason_code,
        "REQUIREMENT_EVIDENCE_MISSING"
    );

    commit(
        &store,
        request(2, "evidence", ActorKind::Evidence, evidence("e-verify")),
    )
    .unwrap();
    let wrong_actor = commit(
        &store,
        request(
            3,
            "wrong-actor",
            ActorKind::Human,
            disposition(
                "resolution-wrong-actor",
                "verify",
                RequirementDispositionKind::Satisfied,
                vec!["e-verify"],
                vec![],
            ),
        ),
    )
    .unwrap();
    assert_eq!(
        wrong_actor.evaluation.reason_code,
        "REQUIREMENT_DISPOSITION_UNAUTHORIZED"
    );
    commit(
        &store,
        request(
            3,
            "resolve",
            ActorKind::Evidence,
            disposition(
                "resolution-1",
                "verify",
                RequirementDispositionKind::Satisfied,
                vec!["e-verify"],
                vec![],
            ),
        ),
    )
    .unwrap();
    commit(
        &store,
        request(
            4,
            "invalidate",
            ActorKind::Evidence,
            Event::RequirementDispositionInvalidated {
                resolution_id: "resolution-1".into(),
                plan_id: "plan-1".into(),
                requirement_id: "verify".into(),
                reason: "verification became stale".into(),
                trigger_ref: "test:changed".into(),
            },
        ),
    )
    .unwrap();

    let no_supersession = commit(
        &store,
        request(
            5,
            "no-supersession",
            ActorKind::Evidence,
            disposition(
                "resolution-2",
                "verify",
                RequirementDispositionKind::Satisfied,
                vec!["e-verify"],
                vec![],
            ),
        ),
    )
    .unwrap();
    assert_eq!(
        no_supersession.evaluation.reason_code,
        "INVALID_RESOLUTION_SUPERSESSION"
    );
    let mut replacement = disposition(
        "resolution-2",
        "verify",
        RequirementDispositionKind::Satisfied,
        vec!["e-verify"],
        vec![],
    );
    if let Event::RequirementDispositionRecorded {
        supersedes_resolution_id,
        ..
    } = &mut replacement
    {
        *supersedes_resolution_id = Some("resolution-1".into());
    }
    let replaced = commit(
        &store,
        request(5, "replace", ActorKind::Evidence, replacement),
    )
    .unwrap();
    assert_eq!(replaced.status, CommitStatus::Committed);
    assert_eq!(
        load(&store)
            .unwrap()
            .state()
            .governed_requirement_state("plan-1", "verify"),
        Some(RequirementState::Resolved)
    );
}

#[test]
fn migrates_schema_v6_to_v7_without_changing_legacy_plan_state() {
    let source = path("migration-source").join("events-v6.jsonl");
    let destination = path("migration-destination").join("events-v7.jsonl");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    let record = json!({
        "sequence": 1,
        "schema_version": 6,
        "event_id": "legacy-plan",
        "idempotency_key": "legacy-plan",
        "expected_revision": 0,
        "actor": {"kind": "human", "id": "tester", "provenance": "v6-fixture"},
        "scope": "repo",
        "event": {
            "type": "plan_registered",
            "plan_id": "legacy-plan",
            "objective": "preserve legacy behavior",
            "acceptance_checks": ["legacy replay passes"],
            "unresolved_questions": [],
            "intent_id": null
        }
    });
    let approval = json!({
        "sequence": 2,
        "schema_version": 6,
        "event_id": "approval",
        "idempotency_key": "approval",
        "expected_revision": 1,
        "actor": {"kind": "human", "id": "tester", "provenance": "v6-fixture"},
        "scope": "repo",
        "event": {
            "type": "plan_approval_recorded",
            "approval_id": "approval",
            "source_ref": "conversation:v6",
            "goal": "preserve v6 approval",
            "explicit_items": ["migrate"],
            "inferred_items": [],
            "unknown_items": [],
            "objective": "migrate approval",
            "acceptance_checks": ["approval replay passes"],
            "unresolved_questions": [],
            "session_id": "session-v6",
            "tool_name": "apply_patch",
            "authority_ref": "test approval"
        }
    });
    let source_contents = format!(
        "{}\n{}\n",
        serde_json::to_string(&record).unwrap(),
        serde_json::to_string(&approval).unwrap()
    );
    std::fs::write(&source, &source_contents).unwrap();

    let outcome = migrate_v6_to_v7(&source, &destination).unwrap();
    assert_eq!(outcome.from_schema, 6);
    assert_eq!(outcome.to_schema, 7);
    assert_eq!(std::fs::read_to_string(&source).unwrap(), source_contents);
    let migrated = load(&destination).unwrap();
    assert!(migrated.state().plans.contains_key("legacy-plan"));
    assert!(migrated.state().plans.contains_key("plan:approval"));
    assert!(migrated.state().governed_plans.is_empty());
}
