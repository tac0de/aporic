# Aporic

Aporic is a local-first agent-system hub whose practical objective is to reduce
the time its owner spends restating context, coordinating independent work, and
correcting unsupported completion claims.

The active implementation is a modular Rust monolith in `crates/aporic`. Codex is the
first client and connects through a local stdio MCP server. The exact behavioral
kernel is stored in `canon/`; it constrains the evolving hub but does not contain
transport, storage, host, or agent-runtime behavior.

See [PRODUCT.md](PRODUCT.md) for the product objective and
[architecture.md](docs/architecture.md) for the current boundaries.

## Current product surface

The hub exposes ten MCP tools in two groups.

Continuity:

- `aporic_open`: open an idempotent work session and receive bounded context;
- `aporic_recall`: retrieve recent durable project context;
- `aporic_record`: record one typed durable fact or work-state change;
- `aporic_close`: complete or hand off a session;
- `aporic_reconcile`: abandon stale interrupted sessions without claiming success.

Advisory coordination:

- `aporic_task_create`: define a task contract, dependencies, and write scope;
- `aporic_task_list`: inspect bounded task state;
- `aporic_task_claim`: acquire a time-bounded worker lease;
- `aporic_task_complete`: complete only with evidence for every criterion;
- `aporic_task_cancel`: cancel without implying completion.

Coordination records do not launch agents, grant host authority, or block tools.
They make parallel-work conflicts and unsupported completion claims visible at
the hub boundary.

Decision and constraint records may explicitly supersede older records. Effect
records require evidence, and a session with effects cannot be completed until
each effect has a linked verification record. These structural checks target
common agent failures: stale context, duplicate work, and unsupported completion
claims.

State is stored in platform-native application data, not in the governed
workspace or `~/.codex`. Set `APORIC_DATABASE` to an explicit database path for
tests or isolated experiments.

Build and verify:

```console
cargo build -p aporic --locked
cargo test -p aporic --locked
cargo clippy -p aporic --all-targets --locked -- -D warnings
```

Run the stdio server:

```console
cargo run -p aporic -- mcp serve --stdio
```

Inspect the local kernel and database without starting MCP:

```console
cargo run -p aporic -- doctor
```

Export one project's complete sessions, records, and event history as JSON:

```console
cargo run -p aporic -- export --workspace /absolute/project/path
```

Run the deterministic frontier-failure simulation:

```console
cargo test -p aporic --test frontier_failures -- --nocapture
cargo test -p aporic --test coordination_failures -- --nocapture
cargo test -p aporic --test long_horizon -- --nocapture
```

The Codex bridge template is under `integrations/codex/`. Nothing in the build
installs or changes global Codex configuration.

Licensed under MIT or Apache-2.0.
