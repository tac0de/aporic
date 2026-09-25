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
`context`, `store`, `hub`, `runner`, `hook`, and `mcp` modules. Package
boundaries will be introduced only when an independently versioned contract or
deployment unit exists.

```text
Codex --stdio MCP--> mcp adapter --> hub services --> SQLite
                                      |
                                      +--> kernel invariants

Codex lifecycle --JSON stdin--> fail-open hook --> bounded context + exposure receipt
                                             \--> hashed runtime event + projections

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

Schema v7 adds a rebuildable `memory_items` projection, explicit relation edges,
and a synchronized local FTS5 index. Active retrieval applies lifecycle and
valid-time filters before deterministic class/rank/ID ordering. The source rows
remain authoritative; the projection adds no permissions. Exposure receipts
store selected IDs, byte counts, the policy digest, and installation-keyed HMACs
for host session/turn correlation—never prompt or transcript contents.

Schema v8 adds append-only `runtime_events` and a trigger-maintained
`capability_observations` projection. Runtime rows keep bounded host/tool
metadata, inferred capability and outcome classes, latency, byte counts, and
installation-keyed HMACs for correlation and payload equality. They do not keep
raw prompt, input, output, transcript, or assistant content. Tool events link to
the latest exposure with identical project/session/turn HMACs. This is an
observability relation, not evidence that recalled memory caused an outcome.

## MCP surface

- `aporic_open`: start an idempotent session and return recent context.
- `aporic_recall`: retrieve a bounded project capsule.
- `aporic_memory_search` and `aporic_memory_get`: inspect deterministic,
  workspace-scoped memory without granting write or execution authority.
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
- `aporic_trace_list`, `aporic_trace_get`, `aporic_capability_report`, and
  `aporic_hook_health`: inspect runtime observations, inferred capability
  projections, and observable hook gaps without exposing a control surface.

Recall excludes records and claims superseded by newer state. Its selector
combines unresolved material unknowns, active constraints and decisions, active
tasks, verified/observed claims, handoffs, and other recent records under a
fixed item and UTF-8 content-byte budget. Objective and focus-path token overlap
only rank items within the higher-level safety priority; recency and stable IDs
make ties deterministic. Each item exposes its origin, influence class, and
selection reasons. Historical effect/verification links remain readable, but
new writes use the typed gate.

The Codex hook adapter appends no raw hook payload. Opening the local store may
still initialize or migrate its schema. For session and prompt events, it uses a
submitted prompt only as an in-memory relevance query, emits model-visible text
inside JSON data records under a data-not-instructions header, and records a
privacy-preserving exposure receipt. For all recognized lifecycle events it
records privacy-minimized runtime metadata and hashes. Tool and permission hooks
always return an empty JSON object. Errors also return `{}`. The adapter does not
block tools, request permissions, invoke models, or modify Codex configuration.

Shadow disposition is a deterministic counterfactual classification. Even
`would_deny` is a recorded observation and has no enforcement path. Hook health
compares pre/terminal tool pairs, duplicate fingerprints, and unknown event,
capability, or schema values. It can reveal evidence of incomplete observation;
it cannot prove completeness, so `coverage_proven` is always false. The CLI can emit local OpenTelemetry-shaped JSON
with 128-bit trace IDs and 64-bit span IDs, but does not transmit it.

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

`context_runtime` simulates migration, deterministic budget enforcement,
supersession, unknown/task retention, memory-injection labeling, fail-open hook
behavior, and non-persistence of sensitive lifecycle fields.

`memory_lifecycle` exercises FTS retrieval, explicit temporal supersession,
untrusted-by-default text, bounded Hook output, HMAC-only exposure correlation,
and projection/FTS consistency. `eval memory` adds an offline fixed-trace check
for stale exclusion, gotcha retention, unknown preservation, determinism, and
zero poison authority escalation.

`runtime_trace` exercises v7 migration, HMAC correlation, memory-exposure
association, payload non-retention, capability projection, shadow-policy
non-enforcement, gap/duplicate/schema detection, trace lookup, and content-free
export. `eval runtime` adds a fixed offline adversarial trace and asserts no
blocking, raw payload retention, network export, or model/API invocation.

## Later growth

Actual agent dispatch, event-driven waiting, remote transports, sandboxing, and
remote-effect verification are later layers. The current task and lease records
are advisory coordination state only and do not make the continuity loop depend
on a worker runtime.
