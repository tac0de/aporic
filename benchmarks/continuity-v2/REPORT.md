# Aporic continuity benchmark v2 — exploratory result

Run date: 2026-09-26. Preregistered protocol commit: `efbd96e` (generated
Python cache removed in `08670a8`; no case, gold answer, prompt, or runner
change). `codex-cli 0.154.0`, CLI default model, fresh session per run,
balanced first-arm order. The provider model identifier was not emitted by
the CLI, so exact model identity cannot be independently verified.

## Result

The primary four-field exact answer was **8/8 for the dated handoff baseline
and 8/8 for Aporic**. Paired counts: both correct 8, baseline only 0, Aporic
only 0, neither 0. No arm returned an unsafe autonomous target in the four
ambiguous, absent, or stale cases. This batch found no quality improvement.

| Case | Condition | Baseline exact | Aporic exact | Baseline sec | Aporic sec | Baseline input tokens | Aporic input tokens |
|---|---|---:|---:|---:|---:|---:|---:|
| C01 | Clear task | 1 | 1 | 9.69 | 17.37 | 28,707 | 45,059 |
| C02 | Clear handoff | 1 | 1 | 9.25 | 12.08 | 28,706 | 44,744 |
| C03 | Superseded decision | 1 | 1 | 14.84 | 19.68 | 28,733 | 60,649 |
| C04 | Superseded decision | 1 | 1* | 11.75 | 13.35 | 28,731 | 44,776 |
| C05 | Ambiguous tasks | 1 | 1 | 8.84 | 12.77 | 28,660 | 45,045 |
| C06 | No unfinished work | 1 | 1 | 9.49 | 8.43 | 28,692 | 28,830 |
| C07 | Completed live state | 1 | 1 | 9.78 | 11.42 | 28,716 | 29,052 |
| C08 | Externally blocked | 1 | 1 | 10.69 | 14.93 | 28,694 | 31,883 |

Across eight runs per arm, baseline took 84.34 seconds and 229,639 input
tokens; Aporic took 110.03 seconds and 330,038 input tokens. Aporic took
25.70 more seconds (+30.5%) and 100,399 more input tokens (+43.7%). Median
run time was 9.73 versus 13.06 seconds. Output tokens were 874 versus
1,106; completed command calls were 8 versus 24. Cached input tokens are
included within reported input tokens and are not added to these totals.
The measured setup script took 0.39 seconds for eight fixtures; human
record-writing and implementation work were not measured.

`*` C04 Aporic returned the exact answer, but its `aporic_read.py recall`
command produced empty stdout with exit code 0. The raw trace does not show
where the model obtained `FE-R3`. Keep the pre-registered exact score, but
do not treat this case as evidence of a reliable rule retrieval. The original
run was not retried.

## Validity audit and limits

The post-run audit verified that the frozen manifest, protocol, runner, and
schema hashes match the metadata captured before the batch. It inspected all
eight treatment databases: decision text, supersedes links, task objectives,
handoffs, and SQLite integrity matched the frozen case facts. The baseline
notes and both arms' live `state.json` were checked by the runner before any
run. The audit found one tool-output anomaly, C04 above. The post-run audit
and evaluator hardening were added after the batch; their source is committed
separately and their hash is in `results/audit.json`.

This compares **Aporic API assisted recovery** with a curated dated handoff
file, given prewritten history. In six cases `aporic_resume` already returned
the expected status and candidate, so this is not a test of independent LLM
inference. Aporic also supplied structured metadata and a generic acceptance
criterion absent from the baseline note. Eight synthetic cases selected by
us are too few and too unlike full implementation work to support a product
wide or statistical claim. The apparent cost difference applies to this CLI
and these short recovery tasks, not to all use cases or pricing.

Version 1 is excluded: the read-only CLI sandbox prevented its treatment
helper from opening SQLite. It was stopped after one full pair and one
baseline run; its partial logs remain under `/private/tmp/aporic-continuity-v1`.
Version 2 used a new workspace, unchanged case facts and gold, and a
preflight-tested workspace-write sandbox.

## Next comparison

Measure a full real task across prior work, record creation, session restart,
actual implementation, and independent verification. Separate a deterministic
test of `aporic_resume` correctness from an agent comparison using equal raw
chronology in both arms. Add repeated runs on frozen cases and include the
cost of writing records. Resolve the C04 empty-output path before relying on
retrieval-dependent answers.

Raw JSONL events, CLI usage, per-run result JSON, setup metadata, and the
audit are retained in [`results/`](results/).
