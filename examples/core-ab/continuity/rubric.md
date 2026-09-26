# Continuity pilot rubric

Freeze before either participant starts. Both arms begin at the same Git commit
with identical `shipping.py` and visible tests. The baseline receives ordinary
`HANDOFF.md` notes; treatment receives the same decisions in an isolated
Aporic database through `aporic_resume`/`aporic_open`/`aporic_recall`.

The shared user request is to finish the interrupted shipping revision, add
focused checks, and report verification. The evaluator keeps hidden tests
outside both participant workspaces.

## Primary outcome

The unchanged hidden test suite checks:

1. KR rate 400 cents, FR/DE/ES rate 700, other countries 900 below threshold.
2. The current free-shipping threshold is 6,000 cents; the superseded 5,000
   threshold must not be applied.
3. Loyalty subtracts 200 cents from shipping, floors at zero, and does not
   change free shipping.
4. Negative subtotals raise `ValueError`.

`accepted_task` requires every hidden test to pass. Record each test result,
visible test result, and final source hash. Do not edit artifacts before scoring.

## Process measures

Record whether the agent found the unfinished target, whether it applied the
superseding decision, any request for the human to repeat context, tool-observed
start/end time, and per-run tokens only when the host exposes them. The baseline
handoff and Aporic records contain the same substantive facts, including an
irrelevant future color note. A single pair tests feasibility and concrete
failure modes; it does not establish a reliable average time or quality effect.
