# general.traceability

Requirement: Trace every product requirement through a reproducible scenario.

Precondition: use an initialized web_game product in the declared supported environment.

Actions and expected outcomes: Resolve the requirement, scenario, design, implementation and test links; a missing link reports incomplete delivery.

Failure and recovery: report the failure, preserve prior valid state, and expose a valid recovery action.

Design: make state transitions and recovery outcomes observable.

Implementation reference: source.rs is only a partial illustrative model; implementing this full behavior remains product work.

Test contract: assertions must examine each outcome above, including the negative case. This text is a test specification, not a recorded test execution.
