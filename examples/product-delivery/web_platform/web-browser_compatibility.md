# web.browser_compatibility

Requirement: Support declared browser capabilities.

Precondition: use an initialized web_platform product in the declared supported environment.

Actions and expected outcomes: Exercise supported browsers and unavailable optional capabilities; either the operation works or a clear fallback is shown.

Failure and recovery: report the failure, preserve prior valid state, and expose a valid recovery action.

Design: make state transitions and recovery outcomes observable.

Implementation reference: source.rs is only a partial illustrative model; implementing this full behavior remains product work.

Test contract: assertions must examine each outcome above, including the negative case. This text is a test specification, not a recorded test execution.
