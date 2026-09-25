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

## Non-goals for the first vertical slice

- intercepting or authorizing host tool calls;
- storing complete conversations or every tool result;
- launching or supervising additional agents;
- remote service operation, publication, or deployment;
- a graphical interface;
- compatibility with the current experimental Rust APIs.

## Current stage

The product is in the thin-prototype stage. The prototype question is:

> Can a local MCP hub restore and preserve the minimum useful work context
> across Codex tasks with less overhead than manually restating it?

The first slice is complete only when an actual stdio MCP client can open a
session, recall project context, record a durable item, close the session, and
recover the same state after a process restart.

## Constraints and decisions

- Rust owns product logic.
- The first implementation is a modular monolith, not a service mesh.
- Runtime state lives outside both the governed project and `~/.codex`.
- `~/.codex` will eventually contain only the Codex-owned configuration and a
  minimal Aporic bootstrap instruction.
- The first integration is advisory and fail-open. MCP failure must be visible
  but must not silently narrow host permissions.
- Important state changes are append-only events with transactionally updated
  read projections.
- Raw conversation history is not durable product memory.

## Material unknowns

- How reliably Codex invokes the hub for substantive tasks without adding
  friction to trivial work.
- Which context-ranking strategy minimizes both omission and stale-context
  noise.
- Which agent runtime should back later delegation and scheduling.
- Whether the observed efficiency gain justifies an always-running local
  service after the stdio prototype.

## Deferred operational work

Privacy controls, retention, export, backup, remote authentication, long-running
agent scheduling, and controlled effect execution are intentionally deferred.
They must be resolved before their corresponding capabilities are introduced.

## Prototype evidence

Observed on 2026-09-25:

- the exact kernel candidate in the repository matches the installed candidate
  by SHA-256;
- a real stdio MCP child process exposes all four tools and preserves a record
  across a server restart;
- exact retries are idempotent and conflicting reuse of a key is rejected;
- concurrent writers retain all tested sessions through SQLite WAL;
- the complete workspace test and clippy suites pass;
- an already initialized release database opens below the resolution of the
  local `time` measurement, while first-time database creation was about 0.57
  seconds and remains an optimization target.

Not yet observed:

- Codex loading the repository MCP bridge in a fresh real task;
- reduced restatement or coordination time during representative daily work;
- agent dispatch or parallel worker operation.

The protocol slice is implemented, but the product hypothesis remains open
until the Codex bridge is installed and exercised on real work.
