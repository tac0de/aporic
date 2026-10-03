# web_game.input

Requirement: Resolve repeated, ordered and lost-focus input predictably.

Precondition: use an initialized web_game product in the declared supported environment.

Actions and expected outcomes: Hold and repeat movement keys, combine inputs and lose focus; no stuck input survives the focus transition.

Failure and recovery: report the failure, preserve prior valid state, and expose a valid recovery action.

Design: make state transitions and recovery outcomes observable.

Implementation reference: source.rs is only a partial illustrative model; implementing this full behavior remains product work.

Test contract: assertions must examine each outcome above, including the negative case. This text is a test specification, not a recorded test execution.
