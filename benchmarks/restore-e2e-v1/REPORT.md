# Restore destination: two-session A/B pilot

Run date: 2026-09-27. Source commit `aaf736a`; preregistration commit
`d2fab43`. One real Rust correction, one pair, two fresh Codex CLI sessions
per arm. CLI `0.154.0` default model; provider model ID was not exposed.
Baseline wrote and read a normal handoff file. Treatment authored its plan
through the isolated Aporic MCP adapter, then called `resume` and `recall` in
the second session. Both were given the same starting code and task.

## Verified outcome

Both arms **passed** the frozen hidden dangling-symlink check and public
recovery suite. Both authored a regression test, stayed within the permitted
product paths, and submitted byte-identical `recovery.rs` implementations.
The baseline submitted more boundary tests, so its tests were selected for
the product integration. An independent Inspector reviewed both patches and
the traces; the source fix preserves existing destination entries and
propagates unexpected metadata errors.

| Measured agent work | Handoff file | Aporic | Aporic minus baseline |
|---|---:|---:|---:|
| Stage 1: inspect and save plan | 135.64 s | 101.45 s | −34.19 s |
| Stage 2: recover and implement | 101.82 s | 84.17 s | −17.64 s |
| Total wall time | 237.46 s | 185.63 s | −51.83 s (−21.8%) |
| Total input tokens | 423,775 | 388,282 | −35,493 (−8.4%) |
| Total output tokens | 5,645 | 3,789 | −1,856 |
| Completed command calls | 21 | 17 | −4 |
| Hidden and public recovery checks | Pass | Pass | Tie |

Cached input was 381,696 and 356,608 tokens respectively and is already
inside the input totals. Stage 1 record creation and Stage 2 retrieval are
included in these time and token figures. The baseline handoff was 5,644
bytes; the Aporic record content was 2,995 bytes in an 880,640-byte SQLite
database. The different note lengths are a plausible contributor to the
observed cost difference.

The operator's source export took 0.28 seconds. The independent grader took
53.62 and 1.45 seconds in baseline/treatment order; the latter benefited from
the grader's shared compilation cache. These operator costs are excluded from
agent totals. The frozen source failed only the hidden symlink test before
implementation (three existing tests passed). Both completed submissions
passed their own authored tests plus the hidden check in a fresh grader tree.

## Trace audit and interpretation

Stage 1 left product source unchanged in both arms. No sibling workspace
reference appeared in retained command events. The treatment transcript has
successful MCP `open`, `record`, `close`, `resume`, and `recall` replies. The
Stage 2 CLI captured `resume` and `recall` JSON exactly matching the helper's
independent reply transcript; the C04 empty-capture anomaly did not recur.
Frozen file hashes, submission hashes, and hidden-test hash matched the saved
setup. The runner's sibling-path scan is an audit signal, not an OS-level
read barrier.

**This pair shows feasibility and a faster Aporic run on this task. It does
not establish that Aporic caused the speed difference.** The baseline always
ran first, both agents independently wrote different length plans, and a
single task cannot measure variation or a general effect. The CLI did not
expose the exact default model ID. The grader's authored-test check is
coarse: it would not by itself detect deliberate weakening of original public
tests. The independent inspection confirmed that these two submissions kept
the original tests. The product fix addresses existing entries, including
dangling symlinks; the pre-existing check-to-rename concurrency race remains
and needs a separate requirement and test.

The next batch should use several distinct real tasks, alternate arm order,
pin an observable model identity where supported, and keep independent MCP
reply capture. Continue reporting verified task success and total two-session
cost together; do not combine this pair with the earlier retrieval-only data
as if they had the same outcome.

Raw CLI JSONL, per-session usage, the handoff, MCP transcript/database,
submission snapshots, grade receipts, and a deterministic [result summary](results/result.json)
are retained in [`results/`](results/).
