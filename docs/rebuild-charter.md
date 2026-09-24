# Aporic rebuild charter

## Purpose

Aporic is a small deterministic authorization kernel for coding-agent actions.
It records enough durable state to replay authority decisions and effects. It is
not a general agent framework, a natural-language interpreter, or a workflow
catalog.

## Trusted core

The trusted computing base owns only:

- append-only records, locking, idempotency, and deterministic replay;
- exact scope, session, action, and input authority;
- evaluation, reservation, occurrence, verification, and abandonment state;
- fail-closed handling of corrupt, incomplete, unsupported, or contended state.

Natural-language interpretation, risk inference, plan quality, analyzers,
host-specific hooks, UI projections, and extension workflows stay outside this
boundary. Extensions may submit commands and evidence but cannot grant
authority, mutate the ledger directly, or override replay.

## Repository model

The new implementation is one monorepo with conceptual `kernel`, `ledger`,
`protocol`, `hosts`, `extensions`, `sdk`, and `tests` boundaries. Begin with
module boundaries and split packages only when independent contracts require
it. A Git commit identifies a coherent build. Compatibility is established by
a rare protocol epoch plus declared capabilities and event-kind revisions, not
by matching product or plugin release numbers.

## First vertical slice

The first milestone accepts one command, evaluates it, reserves exact authority,
records whether the effect occurred, and replays to the identical state. It must
cover stale revisions, duplicate retries, contention, corrupt records, abandoned
reservations, and attempted one-shot reuse before adding policy DSLs, plan
graphs, Wasm analysis, or multiple host hooks.

## Context rollover

Long work should not depend on an expensive end-of-window model summary. The
system continuously projects a small handoff capsule containing objective,
constraints, accepted decisions, repository state, completed checks, open
questions, and the next executable action.

At a host-provided or conservatively estimated context threshold, the host
adapter prepares a successor task. The successor must acknowledge the capsule
and workspace handshake before the predecessor retires. Only facts and work
state cross this boundary; session authority, execution grants, and one-shot
consumption rights never transfer.

## Reversal conditions

Reconsider the clean rebuild if the first vertical slice cannot reproduce the
archived kernel's essential invariants with materially less coupling, or if the
host cannot expose or safely approximate the lifecycle signals required for
rollover. Preserve the archive until those questions are resolved.
