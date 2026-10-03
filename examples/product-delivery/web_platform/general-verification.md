# general.verification

Requirement: Preserve only current, execution-bound check credit.

Precondition: use an initialized web_platform product in the declared supported environment.

Actions and expected outcomes: Compare full prerequisite fingerprints and command specification; altered source or replaced result invalidates the old check.

Failure and recovery: report the failure, preserve prior valid state, and expose a valid recovery action.

Design: make state transitions and recovery outcomes observable.

Implementation reference: source.rs is only a partial illustrative model; implementing this full behavior remains product work.

Test contract: assertions must examine each outcome above, including the negative case. This text is a test specification, not a recorded test execution.
