use aporic::eval::{
    EvalTrial, StructuredAnswer, TrialProvenance, frontier_scenarios, grade, routing_eligible,
    simulate, simulate_capability_fabric, simulate_context_selection,
    simulate_experiment_portfolio, simulate_git_governance, simulate_memory_lifecycle,
    simulate_runtime_trace, simulate_security_import,
};

#[test]
fn calibrated_simulator_passes_without_becoming_model_evidence() {
    let report = simulate("calibrated").unwrap();
    assert_eq!(report.failed, 0);
    assert_eq!(report.passed as usize, frontier_scenarios().len());
    assert_eq!(report.network_or_model_calls, 0);
    assert!(!report.routing_eligible);
}

#[test]
fn git_governance_simulation_detects_risks_without_mutation_or_approval() {
    let report = simulate_git_governance();
    assert_eq!(report.risks_detected, report.risks_expected);
    assert_eq!(report.stale_remote_states_marked_unproven, 1);
    assert_eq!(report.approvals_issued, 0);
    assert_eq!(report.git_mutations, 0);
    assert!(report.deterministic);
    assert!(!report.routing_eligible);
    assert_eq!(report.network_or_model_calls, 0);
}

#[test]
fn runtime_trace_simulation_detects_gaps_without_blocking_or_payload_capture() {
    let report = simulate_runtime_trace();
    assert_eq!(
        report.lifecycle_gaps_detected,
        report.lifecycle_gaps_expected
    );
    assert_eq!(
        report.duplicate_events_detected,
        report.duplicate_events_expected
    );
    assert_eq!(report.unknown_schemas_preserved, 1);
    assert_eq!(report.destructive_patterns_shadowed, 1);
    assert_eq!(report.blocking_hook_outputs, 0);
    assert_eq!(report.raw_payloads_persisted, 0);
    assert!(report.deterministic);
    assert!(!report.routing_eligible);
    assert_eq!(report.network_or_model_calls, 0);
}

#[test]
fn memory_lifecycle_simulation_rejects_stale_and_poisoned_authority() {
    let report = simulate_memory_lifecycle();
    assert_eq!(report.stale_memories_selected, 0);
    assert_eq!(report.gotchas_retained, report.gotchas_expected);
    assert_eq!(report.unresolved_unknowns_preserved, 1);
    assert_eq!(report.poison_authority_escalations, 0);
    assert!(report.deterministic);
    assert!(!report.routing_eligible);
    assert_eq!(report.network_or_model_calls, 0);
}

#[test]
fn authority_bound_context_beats_recency_without_claiming_model_evidence() {
    let report = simulate_context_selection();
    assert_eq!(
        report.selected_relevant_items,
        report.expected_relevant_items
    );
    assert!(report.selected_relevant_items > report.naive_recency_relevant_items);
    assert_eq!(report.selected_poison_items, 0);
    assert_eq!(report.naive_recency_poison_items, 1);
    assert_eq!(report.authority_escalations, 0);
    assert!(report.deterministic);
    assert!(!report.routing_eligible);
    assert_eq!(report.network_or_model_calls, 0);
}

#[test]
fn frontier_failure_actors_are_detected() {
    let overclaiming = simulate("overclaiming").unwrap();
    assert!(overclaiming.failed >= 4);
    let contrarian = simulate("contrarian").unwrap();
    assert!(contrarian.failed >= 3);
}

#[test]
fn reported_model_identity_cannot_promote_a_route() {
    let scenario = &frontier_scenarios()[0];
    let trial = EvalTrial {
        scenario_id: scenario.id.to_owned(),
        reported_model: "gpt-6-astra".to_owned(),
        provenance: TrialProvenance::Reported,
        answer: StructuredAnswer {
            completed: scenario.expected_completed,
            preserves_unknown: scenario.expected_unknown,
            surfaces_dissent: scenario.expected_dissent,
        },
    };
    assert!(grade(scenario, &trial).passed);
    assert!(!routing_eligible(&[trial]));
}

#[test]
fn only_nonempty_host_attested_batches_are_routing_eligible() {
    assert!(!routing_eligible(&[]));
    let scenario = &frontier_scenarios()[0];
    assert!(routing_eligible(&[EvalTrial {
        scenario_id: scenario.id.to_owned(),
        reported_model: "host-attested-model".to_owned(),
        provenance: TrialProvenance::HostAttested,
        answer: StructuredAnswer {
            completed: false,
            preserves_unknown: true,
            surfaces_dissent: false,
        },
    }]));
}

#[test]
fn an_answer_cannot_be_graded_against_a_different_scenario() {
    let scenario = &frontier_scenarios()[0];
    let result = grade(
        scenario,
        &EvalTrial {
            scenario_id: "different-scenario".to_owned(),
            reported_model: "reported-model".to_owned(),
            provenance: TrialProvenance::Reported,
            answer: StructuredAnswer {
                completed: scenario.expected_completed,
                preserves_unknown: scenario.expected_unknown,
                surfaces_dissent: scenario.expected_dissent,
            },
        },
    );
    assert!(!result.scenario_correct);
    assert!(!result.passed);
}

#[test]
fn secure_capability_simulations_preserve_advisory_boundaries() {
    let capabilities = simulate_capability_fabric();
    assert_eq!(capabilities.executable_capabilities, 0);
    assert_eq!(capabilities.generic_invocation_surfaces, 0);
    assert_eq!(capabilities.secret_bearing_manifests_rejected, 1);
    assert!(capabilities.deterministic);

    let experiments = simulate_experiment_portfolio();
    assert_eq!(experiments.hard_gate_failures_preserved, 1);
    assert_eq!(experiments.model_only_hard_gate_promotions, 0);
    assert_eq!(experiments.moving_goalposts_rewritten, 0);
    assert!(experiments.deterministic);

    let security = simulate_security_import();
    assert_eq!(security.partial_coverage_safety_claims, 0);
    assert_eq!(security.zero_finding_safety_claims, 0);
    assert_eq!(security.scanner_invocations, 0);
    assert!(security.deterministic);
    assert_eq!(
        capabilities.network_or_model_calls
            + experiments.network_or_model_calls
            + security.network_or_model_calls,
        0
    );
}
