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

The hub exposes seventeen MCP tools in five groups.

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
- `aporic_task_complete`: complete only when every criterion exactly matches a
  verified mechanical claim;
- `aporic_task_cancel`: cancel without implying completion.

Epistemic gate:

- `aporic_evidence_add`: classify provenance as direct, reported, or model-only;
- `aporic_claim_assert`: keep observed, verified, inferred, assumed, intended,
  and unknown claims distinct;
- `aporic_dissent_assess`: surface only consequential, actionable dissent with
  direct evidence.

Model routing:

- `aporic_model_route`: recommend Astra, Sol, or Terra from typed task signals.

Verifiable execution history:

- `aporic_check_register`: register an immutable argv-based local check without
  executing it;
- `aporic_run_list`: list bounded run lifecycle state;
- `aporic_run_get`: inspect one receipt, its hashes, declared artifacts, and any
  mechanically issued claim.

MCP cannot execute a registered check or submit a receipt. A human or local
automation invokes `aporic verify --spec SPEC_ID`; the Rust runner executes the
exact registered program and argv without a shell command string, captures only
output hashes and byte counts, records Git/worktree snapshots and declared-file
hashes, then issues a verified claim only for the expected exit code and complete
artifact set. Failed, timed-out, interrupted, or incomplete runs cannot issue
that claim. Output is hashed as a bounded-memory stream. On Unix, timeout cleanup
targets the normal child process group. This is best-effort lifecycle cleanup,
not a security sandbox: a hostile process that creates a new session or uses
external effects is outside this guarantee.

Coordination records do not launch agents, grant host authority, or block tools.
They make parallel-work conflicts and unsupported completion claims visible at
the hub boundary.

Decision and constraint records may explicitly supersede older records. New
effect and verification records use the typed evidence/claim path: free text,
reported command output, and model assessment cannot become `verified`.
Aporic-direct verification means that Aporic itself read and hashed a file inside
the session workspace, or that its local runner generated an execution receipt
for a pre-registered command specification. A receipt proves the observed
process result and declared artifacts, not that a test is semantically adequate
or that an external real-world effect occurred. Material unknowns block
completion until an observed or verified claim supersedes them.

The router is advisory and outside the behavioral kernel. It does not dispatch a
model, grant authority, or turn a model review into evidence. See
[model routing](docs/model-routing.md).

## Offline evaluation

Aporic does not call the OpenAI API or any other model endpoint. Its v0.5
evaluation core is deterministic and offline. It checks structured outcomes for
false completion, unsupported certainty, preservation of unknowns, needless
dissent, missing necessary dissent, and failed-tool overclaiming. Built-in
actors test the grader itself; their scores are explicitly ineligible for model
routing. Imported model names and results remain `reported` unless a host can
attest their provenance, so a self-declared Astra, Sol, or Terra result cannot
promote a routing rule.

Run the offline grader simulation:

```console
cargo run -p aporic -- eval simulate --actor calibrated
cargo run -p aporic -- eval simulate --actor overclaiming
cargo run -p aporic -- eval simulate --actor contrarian
```

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

Execute a previously registered check, or reconcile a run left behind by an
interrupted runner:

```console
cargo run -p aporic -- verify --spec SPEC_ID
cargo run -p aporic -- executions reconcile --stale-after 300
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
cargo test -p aporic --test epistemic_gate -- --nocapture
cargo test -p aporic --test verifiable_execution -- --nocapture
cargo test -p aporic --test offline_eval -- --nocapture
```

The Codex bridge template is under `integrations/codex/`. Nothing in the build
installs or changes global Codex configuration.

Licensed under MIT or Apache-2.0.
