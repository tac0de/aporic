# Bounded context-selection pilot

Two fresh `gpt-6-sol` medium agents started from settlement fixture commit
`afa545cfbec349d22ea5b5c6e9cf63d7c8408336`. Both implemented
`merchant_totals` from the same six dated historical facts. The baseline
searched ordinary local notes; the treatment used isolated Aporic `open`,
`recall`, and `memory_search`. Aporic supersession links hid two obsolete
rules from active recall, while both arms could access the current facts.
Neither agent requested user clarification.

| Condition | Hidden checks | Current record IDs named | Agent-reported UTC elapsed |
| --- | ---: | --- | ---: |
| [Without Aporic](runs/baseline/src/lib.rs) | [7/7](runs/baseline/hidden-result.txt) | SET-003, SET-004, SET-005 | 83 seconds |
| [With Aporic](runs/treatment/src/lib.rs) | [7/7](runs/treatment/hidden-result.txt) | SET-003, SET-004, SET-005 | 75 seconds |

Both scored 9/9 under the [frozen rubric](rubric.md): seven hidden checks,
the public smoke test, and correct provenance IDs. The source was copied into
separate evaluator directories for the same [hidden suite](evaluator/hidden_tests.rs);
the submissions themselves were not repaired before scoring. The treatment
was eight seconds faster by agent-observed clock times. There is no accuracy
gain in this pair and no basis for an average speed claim. Per-run token and
tool-call counts were unavailable.

The [canonical history](fixture/canonical/history.jsonl), baseline notes,
neutral treatment import, initial Rust source, answer key, and local operator
scripts are preserved for inspection. The operator scripts contain paths from
this run; the isolated Aporic SQLite database remains outside Git.
