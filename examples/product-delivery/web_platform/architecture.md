# web_platform architecture

Rust owns state transitions and invariants. A browser adapter translates user events and renders observable states. The adapter and actual browser runs are outside this declarative fixture.
