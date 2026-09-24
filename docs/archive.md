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

Legacy project-local `.aporic` state is not preserved in this repository. The
rebuilt system keeps its connection and ledger state outside governed
workspaces and does not read the old project-local format.
