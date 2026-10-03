# web_platform.integrity

Requirement: Preserve resource invariants across failed writes.

Precondition: use an initialized web_platform product in the declared supported environment.

Actions and expected outcomes: Submit invalid values and interrupt a multi-field update; reject invalid state and retain a complete prior version.

Failure and recovery: report the failure, preserve prior valid state, and expose a valid recovery action.

Design: make state transitions and recovery outcomes observable.

Implementation reference: source.rs is only a partial illustrative model; implementing this full behavior remains product work.

Test contract: assertions must examine each outcome above, including the negative case. This text is a test specification, not a recorded test execution.
