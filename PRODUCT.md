# Aporic product record

## Objective

Aporic exists to maximize one person's working efficiency. It should let
agents recover relevant context, coordinate independent work, preserve durable
decisions, and verify consequential results without requiring the human to
restate or manually orchestrate the work.

The product is a local-first agent-system hub. Codex is its first client and
connects through MCP. The behavioral kernel constrains the hub, but the kernel
is not the whole product.

## Intended outcome

A substantive Codex task can open one Aporic session, recover a bounded capsule
of relevant project state, record only durable changes, and leave a verified
handoff that a later task can resume.

Success is evidenced by less repeated explanation, fewer coordination prompts,
faster recovery across tasks, useful parallel work, and less rework. A feature
fails when the time or attention required to operate it exceeds the work it
saves.

## Current non-goals

- intercepting or authorizing host tool calls;
- storing complete conversations or every tool result;
- launching or supervising additional agents;
- remote service operation, publication, or deployment;
- a graphical interface.

## Current stage

The product is in the simulated-coordination stage. The product question is:

> Can a local MCP hub restore and preserve the minimum useful work context
> across Codex tasks with less overhead than manually restating it?

The first protocol slice is complete: an actual stdio MCP client can open a
session, recall project context, record a durable item, close the session, and
recover the same state after a process restart. Current work tests whether the
hub can structurally reduce recurring frontier-model failures without requiring
manual evaluation.

## Constraints and decisions

- Rust owns product logic.
- The implementation is a modular monolith, not a service mesh.
- Runtime state lives outside both the governed project and `~/.codex`.
- `~/.codex` will eventually contain only the Codex-owned configuration and a
  minimal Aporic bootstrap instruction.
- The Codex integration is advisory and fail-open. MCP failure must be visible
  but must not silently narrow host permissions.
- Important state changes are append-only events with transactionally updated
  read projections.
- Raw conversation history is not durable product memory.
- Evidence provenance is typed as direct, reported, or model-only. Only a
  proposition mechanically derived from Aporic's own workspace-file read-back
  can currently become observed or verified.
- Free text, command reports, external-source reports, user statements, and
  model assessments cannot independently establish verified completion.
- Material unknowns block session completion until a direct observed or
  verified claim explicitly supersedes them.
- A newer decision or constraint can explicitly supersede one older record;
  stale records remain in history but are omitted from active recall.
- Coordination is advisory: task contracts, dependencies, write scopes, leases,
  cancellations, and criterion proofs are recorded, but Aporic does not launch
  workers or grant host authority.
- Model routing is advisory and outside the kernel: Terra handles bounded work,
  Sol is the default for complex implementation, and Astra handles frontier or
  high-consequence ambiguous work.

## Material unknowns

- How reliably Codex invokes the hub for substantive tasks without adding
  friction to trivial work.
- Which context-ranking strategy minimizes both omission and stale-context
  noise.
- Which agent runtime should back later delegation and scheduling.
- Whether the observed efficiency gain justifies an always-running local
  service beyond the current stdio deployment.

## Deferred operational work

Privacy controls, retention, import/restore, backup, remote authentication,
long-running agent scheduling, and controlled effect execution are intentionally
deferred. They must be resolved before their corresponding capabilities are
introduced.

## Current evidence

Observed on 2026-09-25:

- the exact kernel candidate in the repository matches the installed candidate
  by SHA-256;
- a real stdio MCP child process exposes all fourteen tools and preserves a record
  across a server restart;
- exact retries are idempotent and conflicting reuse of a key is rejected;
- concurrent writers retain all tested sessions through SQLite WAL;
- all active workspace tests and the clippy suite pass;
- a deterministic frontier-failure suite improves the enforced-property score
  from the v0.1 baseline of 1/5 to 5/5 for restart memory, stale-decision
  suppression, verified completion, duplicate-work rejection, and stale-session
  reconciliation;
- a deterministic coordination suite scores 6/6 for dependency gating,
  overlapping-write prevention, criterion evidence, duplicate rejection, and
  expired-lease recovery plus cancellation without implied completion;
- a long-horizon workload preserves one active decision across 30 revisions and
  six process restarts, completes 24 leased tasks, and rejects 77 injected stale,
  duplicate, or unsupported state transitions.
- the epistemic-gate simulation rejects reported evidence as verification,
  blocks completion on a material unknown, resolves it with a direct file
  read-back, suppresses low-materiality dissent, and exercises all three model
  routes.

Not yet established:

- generalized performance on model-generated adversarial scenarios beyond the
  deterministic and long-horizon regression suites;
- reduced restatement or coordination cost across a long synthetic workload;
- actual agent dispatch or parallel worker execution.
- trusted receipts for shell commands, test runners, external APIs, or remote
  effects; these remain reported rather than direct evidence.

The protocol slice and its live Codex bridge are operating. The broader product
hypothesis remains open until automated long-horizon simulations show that the
stored context reduces errors without creating stale-context or coordination
overhead.
