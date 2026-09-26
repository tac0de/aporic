# Aporic versus baseline: paired evaluation protocol

## Question

Does using Aporic's `frontend@v2` procedure improve the quality of a completed
frontend task enough to justify its time and token cost?

The existing page and browser observations are **a feasibility example**, not
an A/B result. They contain one implementation, no independently executed
baseline, and no blinded comparison. Neither a completed Aporic task nor a
passed browser check establishes an Aporic benefit.

## Arms

| Arm | Available process | Required output |
| --- | --- | --- |
| A: baseline | Normal Codex repository workflow; no Aporic task brief, procedure, or evidence prompts for this trial | Working code, tests, and a short delivery note |
| B: Aporic | The same Codex host and tools, plus Aporic `frontend@v2` task planning, step records, and evidence review | The same working code, tests, and delivery note; Aporic records are additional |

Aporic stays advisory and fail-open. The trial changes task context and review
structure, not tool permissions. The baseline's omission of Aporic is the
explicit experimental contrast requested for this comparison. Record any
unavoidable Aporic exposure in the baseline as protocol contamination.

## Task and assignment

Use the same written feature request and acceptance rubric in both arms. For
this pilot, start both from clean commit `5dc8fbd`, before the example was
added, in separate checkouts and separate fresh agent contexts. Do not reuse
the already completed example as one arm. Hide the other
run's artifacts and feedback until both runs finish. Keep model version,
reasoning effort, tool access, environment, and time limit equal. Randomize
which arm runs first; repeat with the order reversed across later tasks.

The first task can use this review-board scenario: implement the proposal
review flow, responsive layout, keyboard dialog behavior, and reduced-motion
support. Deliver the same self-contained task brief to both fresh agents;
do not point them at this directory, its screenshots, or its observations.
Those artifacts leak a worked solution.

One paired task is a **pilot** for measuring the process, not evidence of a
general effect. Repeat across several independently specified frontend tasks
and fresh runs before drawing a product conclusion.

## Predeclared measures

| Measure | How to collect it |
| --- | --- |
| Primary: accepted task | A reviewer blinded to arm checks the same acceptance rubric and runs the same Playwright CLI scenarios. Record pass/fail and blocking defects. |
| Review quality | Blinded reviewer scores visual hierarchy, responsive behavior, keyboard access, and evidence completeness on a fixed rubric written before runs. |
| Effort | Record wall-clock time, model input/output tokens when available, tool calls, and number of corrective iterations. Missing telemetry stays missing. |
| Continuity | Give a second, fresh agent the finished handoff and a small follow-up change; score whether it identifies constraints and completes the change without re-discovery. |

Save raw commands, screenshots, scores, and reviewer notes for both arms under
separate run IDs. Strip arm labels from code and screenshots before blinded
review where feasible. Record any unblinding. Compare paired differences,
including failures and overhead; do not substitute procedure completion for
task quality. Report each task's result and uncertainty, not just an average.
Use the frozen [pilot rubric](pilot-rubric.md) for the first paired task.

## Decision rule

For the pilot, the outcome is whether the trial can be run fairly and measured
reliably. It cannot establish that Aporic is better. For repeated tasks,
prefer B only if accepted-task rate or review quality improves without an
unacceptable rise in time or tokens. Set those thresholds before collecting
the repeated-run results. If B adds record-keeping but no detectable outcome
gain, simplify or remove the procedure.

## Current state

Protocol specified. No baseline run, paired result, or causal estimate exists
yet. The existing local Aporic replay is process evidence only.
