# Aporic rebuild workflow

The previous implementation is frozen at
`archive/pre-monorepo-rebuild-2026-09-24`. Treat it as evidence and a source of
counterexamples, not as the architecture to extend. Start with
`docs/rebuild-charter.md`.

## Boundaries

- Keep the trusted kernel limited to deterministic authorization transitions,
  append/replay, reservation, and exact authority consumption.
- Put Codex hooks, launchers, projections, analyzers, Skills, and other host or
  product behavior outside the trusted kernel.
- Bring future first-party extensions into this monorepo; do not recreate a
  separate Commons release train.
- Prefer build identity, a rare protocol epoch, and capability negotiation over
  package-wide semantic-version coupling.
- Treat archived source, tests, and event stores as read-only references. Do
  not delete or migrate them without a separate explicit instruction.
- Do not activate `.aporic/policy.next.json` or resume the legacy hook-reduction
  sequence as an incremental upgrade unless the user explicitly reverses the
  rebuild decision.

## Delivery

- Build the smallest vertical slice first: command, evaluation, reservation,
  occurrence, and deterministic replay.
- Keep authorization state session-bound. Context rollover may transfer facts
  and work state, never execution grants or one-shot authority.
- Keep one integration owner and independently review changes to lifecycle,
  persistence, authorization, or trust boundaries.
- Commit completed local work after its focused and full checks pass. Push,
  release, publication, deployment, destructive cleanup, and legacy-store
  migration remain separate human-authorized boundaries.
