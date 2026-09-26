# Evaluator answer key — keep out of participant workspaces

Current governing records: `SET-003`, `SET-004`, `SET-005`. `SET-001` and
`SET-002` are superseded. `UI-009` is unrelated.

For each `(merchant_id, invoice_id)`, select the unique row with the highest
numeric revision. If two or more rows tie at the highest revision, return
`DuplicateRevision`; ties only below the winner do not matter. A selected
`Void` contributes zero; a selected `Paid` contributes its `cents`. Sum by
merchant with `checked_add` and return `Overflow` on failure. Omit merchants
whose final sum is zero and sort by ascending merchant ID. Empty input yields
an empty vector. The fixture permits negative cents for adjustments.

## Deterministic scoring

1. Run `cargo test --offline` with `hidden_tests.rs` installed. Award one point
   for each of the seven hidden tests and one for the public smoke test.
2. Award one provenance point only if the participant identifies all three
   current SET IDs in its final note; do not award it for merely naming stale
   IDs or the UI ID.
3. Record retrieval trace, wall time, and host-reported token use separately.
   Never convert those observations into test points. A missing trace is
   `unknown`, not a failure of the code task.

The hidden suite probes the stale last-row and subtract-void rules, highest
revision duplicate semantics, checked addition, sort order, zero omission,
merchant-scoped invoice identity, and empty input. A complete score is 9/9.
