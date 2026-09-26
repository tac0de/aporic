# Core Aporic paired pilots

These pilots compare concrete task outcomes with and without one Aporic core
capability at a time. Aporic remains advisory; host model, tools, permissions,
and task acceptance criteria are held equal. The baseline omission of Aporic
is the explicit experimental contrast requested by the user.

The first three capabilities are cross-session continuity, bounded context
selection, and evidence-based completion. Parallel coordination is deferred
until these lower-cost comparisons have been run. Existing deterministic
offline tests establish mechanism behavior, not agent outcome gains.

Each pair uses separate fresh agent contexts and isolated workspaces. Freeze
the task brief, source state, hidden checks, and rubric before starting the
agents. Do not let either agent see the other artifact or hidden checks.
Evaluate final code with the same hidden checks; blind human or agent review
is used for criteria that cannot be scored mechanically. Report retries,
clarification requests, and overhead alongside correctness. One pair is a
pilot, not an average treatment-effect estimate.

For time, capture tool-observed UTC start/end timestamps where possible.
Record host token use only if per-run telemetry is actually available; do not
infer token counts from text length or account-level usage. Missing telemetry
is a stated limitation rather than a zero.
