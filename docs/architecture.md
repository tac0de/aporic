# Aporic hub architecture

## Boundary

The Aporic kernel and Aporic Hub have different lifecycles.

- The kernel is an exact behavioral constitution and a set of deterministic
  invariants. It has no transport, database, model, or host dependency.
- The hub is evolving software that applies those invariants to projects,
  sessions, tasks, decisions, typed evidence, epistemic claims, and agents.
- MCP is an adapter. It does not become the domain model and does not by itself
  form a security boundary around other host tools.

## Implementation

The implementation is one Rust package with internal `kernel`, `domain`,
`store`, `hub`, `runner`, and `mcp` modules. Package boundaries will be introduced only
when an independently versioned contract or deployment unit exists.

```text
Codex --stdio MCP--> mcp adapter --> hub services --> SQLite
                                      |
                                      +--> kernel invariants

local CLI --> runner --> exact argv process --> hashed receipt --> SQLite
```

Each Codex connection runs an inexpensive child process. Processes share one
SQLite database in WAL mode. Each command opens a short connection and commits
an event and its read projection in one immediate transaction.

## Durable model

The hub stores projects, sessions, durable records, advisory tasks, typed
evidence artifacts, epistemic claims, and events. Legacy records retain their
explicit operational kind. New effect and verification assertions go through an
epistemic gate:

```text
source -> evidence grade -> claim status -> criterion proof -> completion
           direct            verified       exact match
           reported          inferred       cannot complete
           model_only        inferred       cannot complete
```

Workspace files read and hashed by Aporic are direct. The local runner also
creates direct execution receipts for immutable, pre-registered argv-based
command specifications. It stores executable resolution, termination and exit
status, stdout/stderr hashes and lengths, Git/worktree snapshots, and hashes for
declared artifacts. Raw command output is not durable state. Successful receipts
create one canonical verified claim; other terminal states do not.

The MCP boundary can register a specification and read run state, but cannot
execute it or submit a receipt. The local CLI owns execution. This is an
integrity boundary inside the application, not an OS sandbox: a process running
as the same user can still alter the database or workspace. A successful receipt
also proves only the recorded process result, not test quality or remote effects.

The event log preserves accepted state transitions and idempotent results. Read
tables provide bounded context without replaying the whole log on every tool
call. Tests replay events independently and compare the result with projections.

## MCP surface

- `aporic_open`: start an idempotent session and return recent context.
- `aporic_recall`: retrieve a bounded project capsule.
- `aporic_record`: append one durable typed record.
- `aporic_evidence_add`, `aporic_claim_assert`, and `aporic_dissent_assess`:
  enforce the evidence, certainty, unknown, and counterargument gates.
- `aporic_model_route`: return an advisory Astra/Sol/Terra route from typed task
  signals.
- `aporic_check_register`, `aporic_run_list`, and `aporic_run_get`: register
  immutable checks and inspect verifiable execution history without exposing an
  MCP execution capability.
- `aporic_close`: complete or hand off a session.
- `aporic_reconcile`: mark inactive sessions abandoned without converting them
  into completed work.
- `aporic_task_create`, `aporic_task_list`, `aporic_task_claim`,
  `aporic_task_complete`, and `aporic_task_cancel`: maintain advisory task
  contracts, dependency gates, non-overlapping write leases, and
  criterion-by-criterion verified proofs.

Recall excludes records superseded by a newer decision or constraint and
prioritizes active decisions, constraints, and material unknowns. Unresolved
material unknowns block completion. Historical effect/verification links remain
readable, but new writes use the typed gate.

Tool failures are returned as visible tool-level errors. Protocol errors are
reserved for malformed MCP requests that cannot be routed or decoded.

## Automated evaluation

`frontier_failures` is a deterministic simulation suite for five recurrent
model failures: context loss after restart, stale decision reuse, unsupported
completion, duplicate work, and orphaned sessions. It is a regression contract,
not evidence that every natural-language model behavior is solved.

`coordination_failures` injects dependency violations, overlapping write scopes,
unsupported completion, duplicate task objectives, and expired leases.
`long_horizon` repeats decision revision, restart, lease, and completion cycles
to detect accumulating stale state.

`epistemic_gate` injects unsupported certainty, unresolved unknowns, irrelevant
dissent, and model-routing boundary cases.

`verifiable_execution` simulates success, non-zero exit, timeout, missing
artifacts, path traversal, duplicate registration, concurrent execution, retry,
task proof binding, interrupted-run reconciliation, export, and event replay.

## Later growth

Actual agent dispatch, event-driven waiting, remote transports, sandboxing, and
remote-effect verification are later layers. The current task and lease records
are advisory coordination state only and do not make the continuity loop depend
on a worker runtime.
