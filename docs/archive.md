# Legacy implementation archive

The pre-rebuild Aporic implementation is preserved by the local Git ref
`archive/pre-monorepo-rebuild-2026-09-24`. The snapshot includes the event-log
kernel, policy and project schemas, Codex hook adapter, MCP surface, structural
analyzer, packaging, documentation, and tests present when the rebuild was
chosen.

This archive is reference material, not the active architecture. In particular,
the staged policy-v4 activation and incremental hook-reduction sequence are not
pending work for the new implementation. Their requirements may be reconsidered
against the rebuild charter rather than copied mechanically.

The existing project event store and `.aporic/policy.next.json` are preserved
unchanged. They are not automatically compatible with the rebuilt kernel and
must not be deleted, activated, or migrated without a separately reviewed
transition.

The companion Aporic Commons repository is preserved at the same-named local
archive ref. Its source is an input to future monorepo extensions, not a second
release train.
