# Data and search

Inspect the current SQLite schema, migration chain, transaction boundaries, event history, and read projections. Persist source events or immutable revisions before deriving an index. Keep historical rows available while excluding stale revisions from current search. Use deterministic ordering and explicit result bounds so replay and retrieval remain stable.

When changing schema, specify upgrade from the previous supported version, fresh-database behavior, export and doctor implications, and whether projections can be rebuilt. Avoid an in-place rewrite of append-only evidence or a migration that silently changes old meaning. For FTS work, preserve source labels, timestamps, content hashes, and the separation between external text and Aporic's internal decision memory.

Verify the affected path with a migrated database, a fresh database, restart/readback, and a failure or corruption case when material. Check concurrent writers if the change alters uniqueness, idempotency, or transaction order. Search relevance is an empirical product question; a correct FTS query alone does not prove usefulness.
