use aporic::eval::{
    EvalTrial, StructuredAnswer, TrialProvenance, frontier_scenarios, grade, routing_eligible,
    simulate, simulate_context_selection,
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
