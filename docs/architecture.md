# Aporic architecture

## Boundaries

Aporic is a modular Rust monolith in `crates/aporic`. The kernel module checks
`canon/KERNEL.md`; domain, context and store code enforce the actual state
invariants. The canon is not an executable state machine. MCP and Codex hooks
are adapters, not an authorization boundary around host tools.

The host owns model execution, delegation and permissions. Aporic records
advisory plans and observed outcomes. It does not dispatch agents, operate a
remote service or publish products. See [PRODUCT.md](../PRODUCT.md).

```text
host --stdio MCP--> adapter --> hub/domain --> SQLite events + projections
host lifecycle --JSON stdin--> fail-open hook --> context + hashed observations
local CLI --> runner --> exact argv process --> execution receipt --> store
repository files --> delivery validator --> dependency impact + current bindings
```

Separate MCP processes share a SQLite database in WAL mode. State-changing
operations commit append-only events and their read projections atomically.
Idempotency keys reject changed requests and return the original result for an
exact retry. Runtime state lives in platform-native application data, outside
the governed repository and Codex configuration.

## Durable state and evidence

The store preserves projects, sessions, decisions, constraints, tasks, evidence,
claims, handoffs and execution history. Read projections support bounded lookup;
replay, audit and doctor checks detect inconsistencies. Database schema version
30 includes all historical migrations required to open existing stores.
Retired feature tables and raw events remain for compatibility, while their
removed APIs do not return to the active surface.

Evidence provenance is `direct`, `reported` or `model_only`. Claims distinguish
observed, verified, inferred, assumed, intended and unknown states. Direct
workspace-file read-back and successful local-runner receipts can support
mechanical claims. Reported output and model assessment cannot independently
establish verified task completion. Task criteria require matching verified
proofs; unresolved material unknowns prevent completed session closure.

The local runner executes immutable registered program/argv specifications,
records resolved executable metadata, exit state, bounded output hashes,
Git/worktree snapshots and declared-file hashes. MCP can register and inspect
checks but cannot execute them or submit runner receipts. The default host
backend is not an OS sandbox. A required Linux `bubblewrap` check fails without
execution credit when isolation cannot be established. Same-user database
access remains outside this integrity boundary.

A receipt proves the captured process result, not semantic test adequacy or an
external real-world effect. Free-text notes cannot assert a verified effect.

## Continuity and coordination

[Begin/finish](core-flow.md) is the default bounded session lifecycle. Resume
selects an unfinished task or fresh handoff without treating it as new
authorization. Interrupted sessions and runs remain visibly incomplete until
explicit continuation or reconciliation.

Memory projections classify records and preserve supersession and temporal
validity. SQLite FTS5 retrieval is deterministic, byte-bounded and requires no
embedding service or model call. Context prioritizes unresolved unknowns,
current decisions and constraints, active tasks and supported claims. Objective
and path overlap, recency and stable identifiers rank remaining candidates.
Results disclose origin, influence class, selection reasons and omissions;
model-authored text stays untrusted. Duplicate content is selected once.

Tasks preserve acceptance criteria, dependencies, scoped leases and cancellation.
Workflow procedures and initiative links expose missing or stale evidence as
advisory state. Task/session delegation assessments and reports distinguish a
plan from an actual host agent run. Model routes are advice, not execution
attestation. See [workflow gates](workflow-gates.md),
[model routing](model-routing.md) and the [Codex bridge](../integrations/codex/AGENTS.md).

## Repository delivery packages

[Product delivery](product-delivery.md) connects execution metadata and typed
requirement/scenario/design/implementation/test dependencies. File references
are bounded and workspace-contained. Focused briefs expose prerequisite closures
and omissions; impact comparison propagates relevant changes to dependent checks.

The pure validator inventories integrity and declared coverage. CLI/MCP reports
also inspect runner-owned bindings. Local delivery execution captures pre/post
inputs and fresh output, then appends a private binding only for a matching
successful run. Public callers cannot manufacture that binding. Currentness
checks use stored receipts and current bytes without a new mutable projection.
General, web, web-game and web-platform profiles specify categories and explicit
not-applicable reasons; category labels do not prove test adequacy.

[Design delivery](design-delivery.md) separately inventories declared references,
concepts, specification, implementation and browser-review files and hashes.
Neither package judges usefulness, visual quality, gameplay or release readiness.

## Observation and research

The optional hook correlates context exposure and runtime events through
installation-keyed hashes. It stores bounded metadata and byte counts rather
than raw prompts, tool input/output, transcripts or assistant messages.
Tool/permission hooks and errors return `{}`; shadow dispositions do not enforce
policy. Hook health can expose observed gaps but cannot prove complete coverage.
Local OpenTelemetry-shaped exports do not transmit data.

Fixed-argv Git observation records local commit-bound metadata without fetch,
checkout, push, merge or approval. Remote tracking refs can be stale. Signature
presence is not signer trust; snapshot digests are not tamper-proof attestation.
Usage receipts keep reported counts, measured counts, conservative byte estimates
and unknowns separate. Cached input remains a subset of input tokens.

External research uses source-labelled immutable revisions and a separate FTS5
corpus. Fetching is explicit; task attachments retain provenance and relevance.
External text remains untrusted and cannot verify product behavior. Accountability
and Rust repair records link failure evidence to later verified repairs without
turning model-authored causal hypotheses into observed facts.

## Verification

Rust tests exercise restart/replay, idempotency, supported-schema migration,
projection corruption, bounded context, supersession, advisory delegation,
runner failure/isolation and delivery evidence freshness. Actual stdio MCP tests
check the 58-tool core and 72-tool full surfaces. CI runs formatting, Clippy,
workspace tests, dependency checks and coverage, with macOS/Windows tests and
Linux sandbox checks.

Deterministic offline evaluations test the grader and selector contracts. They
make no model calls and do not establish model quality, owner-time savings or
causal product improvement. Meaningful browser/product acceptance still requires
appropriate host-run checks and observation.
