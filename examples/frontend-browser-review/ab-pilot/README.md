# First paired frontend pilot

This is one controlled task comparing a normal Codex run with an Aporic-guided
run. Both started from clean commit `5dc8fbd` in separate worktrees, received
the same feature brief, used `gpt-6-sol` at medium effort, and produced a
standalone review-board page. The baseline deliberately omitted Aporic under
the user's request to measure its presence versus absence. The Aporic arm used
an isolated test database and the `frontend@v2` standard procedure. Neither
artifact was repaired after blinded scoring.

## Blinded result and reveal

The independent reviewer received only anonymous code copies and the
[precommitted rubric](../pilot-rubric.md). Its [full report](evidence/blind-review.md)
was finalized before revealing the mapping:

| Condition | Blind label | Acceptance gates | Quality score | Agent-reported wall time |
| --- | --- | ---: | ---: | ---: |
| [Without Aporic](baseline/) | Y | 8/8 | 7/8 | 6 min 53 sec |
| [With Aporic](aporic/) | X | 8/8 | 7/8 | 7 min 54 sec |

The agents' observed times put the Aporic run 61 seconds longer in this one
pair, about 15% of the baseline duration. These times were reported by the
agents, not independently captured from host logs. Host token and tool-call
counts were unavailable. The
reviewer's evidence-quality score describes its own equal audit coverage, not
the agents' evidence production. This pair shows **no measured quality gain**
on the frozen rubric. It is too small to estimate an average effect or to
attribute the time difference to Aporic alone.

The reviewer found a completed-review dialog defect only in X: a disabled
“Mark reviewed” button remains visible when reopening an already reviewed
proposal. Both variants have a visible/accessible proposal action-name
mismatch. These findings are preserved in the artifacts and detailed report.

## Treatment fidelity and evidence

The isolated Aporic [workflow status](evidence/aporic-workflow-status.json)
confirms a `frontend@v2` standard plan, 12 recorded completed steps, and the
verification stage. The step receipts point to workspace files; browser check
results inside those files are author-reported, while the separate blind audit
directly reran the browser checks. The Aporic task was **not** marked complete:
`task_acceptance_proofs_incomplete` remained because no verified mechanical
claim was attached. This process result is separate from the blind browser
acceptance score. The local experiment database is
`/tmp/aporic-review-board-pilot.sqlite3`; it is outside Git and may not persist.

The reviewer used the same [Playwright CLI audit script](evidence/audit.js) for
both artifacts. Structured observations are [X](evidence/X-audit.json) and
[Y](evidence/Y-audit.json); command logs and desktop/mobile screenshots are in
[`evidence/`](evidence/). Browser review found no horizontal overflow, console
errors, or failed requests for either variant at 1440×900 and 390×844. The
reviewer's screenshots and code copies were made before treatment labels were
revealed.

## Interpretation

This pilot validates that a same-task, blinded comparison is feasible. It does
not show that Aporic improves frontend outcomes. A decision about keeping or
expanding `frontend.browser_review` needs repeated tasks with fresh contexts,
predeclared thresholds, and reliable token/time telemetry. A small follow-up
handoff task would test the separate continuity claim; it was not run here.
