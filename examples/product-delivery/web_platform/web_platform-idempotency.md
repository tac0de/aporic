# web_platform.idempotency

Requirement: Apply a repeated operation exactly once.

Precondition: use an initialized web_platform product in the declared supported environment.

Actions and expected outcomes: Retry the same operation key after a timeout; return the original result and do not duplicate the side effect.

Failure and recovery: report the failure, preserve prior valid state, and expose a valid recovery action.

Design: make state transitions and recovery outcomes observable.

Implementation reference: source.rs is only a partial illustrative model; implementing this full behavior remains product work.

Test contract: assertions must examine each outcome above, including the negative case. This text is a test specification, not a recorded test execution.
