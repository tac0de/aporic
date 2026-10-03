# web_platform.permissions

Requirement: Restrict updates to authorized actors and resources.

Precondition: use an initialized web_platform product in the declared supported environment.

Actions and expected outcomes: Attempt cross-tenant and read-only updates; denied operations leave the resource unchanged.

Failure and recovery: report the failure, preserve prior valid state, and expose a valid recovery action.

Design: make state transitions and recovery outcomes observable.

Implementation reference: source.rs is only a partial illustrative model; implementing this full behavior remains product work.

Test contract: assertions must examine each outcome above, including the negative case. This text is a test specification, not a recorded test execution.
