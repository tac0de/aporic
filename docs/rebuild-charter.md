# Aporic rebuild charter

## Current development posture

As of 2026-09-24, Aporic is not an operating service. Its integrations are
therefore advisory and fail-open. They may preserve continuity, observations,
and append-only history, but they must not deny a tool call, require a grant,
add an approval prompt, or narrow permissions supplied by the host.

Governance may be introduced only after an operating service exists, one
explicit and observable control at a time. Each control needs a concrete
threat or operational failure, an owner, an escape hatch, and evidence that it
improves the service without obstructing development. The authorization work
below is retained as design evidence, not as the active integration contract.

## Purpose

Aporic currently explores durable continuity and deterministic records for
coding-agent work. It is not a runtime permission boundary, a general agent
framework, a natural-language interpreter, or a workflow catalog.

## Candidate trusted core for a future operating service

If authorization is reintroduced, the candidate trusted computing base owns
only:

- append-only records, locking, idempotency, and deterministic replay;
- exact scope, session, action, and input authority;
- evaluation, reservation, occurrence, verification, and abandonment state;
- fail-closed handling of corrupt, incomplete, unsupported, or contended state.

Natural-language interpretation, risk inference, plan quality, analyzers,
host integrations, UI projections, and extension workflows stay outside this
boundary. Extensions may submit commands and evidence but cannot grant
authority, mutate the ledger directly, or override replay.

## Court governance

Aporic adopts a Joseon-inspired court as its durable human-facing governance
model. The human who owns and directs Aporic is the **King of Aporic**. The
King sets product purpose, appoints priorities, accepts consequential tradeoffs,
and makes the final decision when counsel presents materially different paths.

Agents serve as court officials rather than substitute sovereigns. They must
interpret the King's explicit intent faithfully, investigate ordinary unknowns,
offer candid evidence and dissent, preserve constraints, and execute only
within the authority actually granted. They must not turn deference into
flattery, conceal risk, or invent a royal command from ambiguity.

The court model borrows institutional separation from Joseon:

- **의정부** frames plans, alternatives, and coordinated execution;
- **승정원** preserves exact intent, decisions, and continuity between days;
- **사헌부·사간원** challenge unsupported claims, surface risk, and record
  principled dissent;
- **호조** accounts for bounded time, tools, compute, and other resources.

These names are a core organizational and interaction convention, not kernel
authority or an authentication mechanism. The deterministic kernel continues
to recognize only explicit actors, scopes, grants, reservations, and recorded
effects. No title bypasses system or host policy, law, safety boundaries,
third-party rights, or the need for explicit authorization.

## Repository model

The new implementation is one monorepo with conceptual `kernel`, `ledger`,
`protocol`, `hosts`, `extensions`, `sdk`, and `tests` boundaries. Begin with
module boundaries and split packages only when independent contracts require
it. A Git commit identifies a coherent build. Compatibility is established by
a rare protocol epoch plus declared capabilities and event-kind revisions.

## Superseded authorization slice

The original first milestone accepted one command, evaluated it, reserved exact
authority, recorded whether the effect occurred, and replayed to identical
state. That work remains useful as an isolated experiment, but it is no longer
connected to automatic host authorization during development.

## Context rollover

Long work should not depend on an expensive end-of-window model summary. The
system continuously projects a small handoff capsule containing objective,
constraints, accepted decisions, repository state, completed checks, open
questions, and the next executable action.

At a host-provided or conservatively estimated context threshold, a host
integration may prepare a successor task. The successor must acknowledge the capsule
and workspace handshake before the predecessor retires. Only facts and work
state cross this boundary; session authority, execution grants, and one-shot
consumption rights never transfer.

## Reversal conditions

Reconsider the clean rebuild if the first vertical slice cannot reproduce the
archived kernel's essential invariants with materially less coupling, or if the
host cannot expose or safely approximate the lifecycle signals required for
rollover. Preserve the archive until those questions are resolved.
