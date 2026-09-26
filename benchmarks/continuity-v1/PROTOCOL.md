# Continuity/resume benchmark v1 — preregistration

## Question and scope

When the same history and current workspace state already exist, does Aporic
`resume` plus bounded recall help a fresh Codex agent choose the next safe
action more accurately or cheaply than an ordinary dated handoff file?

This measures **recovery only**. It excludes the human/agent effort needed to
write the original handoff or Aporic records, and it does not measure whether
the later implementation succeeds. No result from this batch alone establishes
end-to-end product value.

## Frozen design

- Eight distinct cases, two each in four strata: clear target, superseded
  decision, ambiguous or absent target, and stale recorded target versus live
  workspace state. Case definitions and gold JSON are in `cases.json`.
- Fresh, isolated workspace and fresh Codex CLI session for every arm/case.
  Baseline gets a generated `HANDOFF.md`; treatment gets the same source facts
  in an isolated Aporic SQLite database and a local read-only MCP helper.
  Both get the same `state.json`, model, task request, and host tools.
- Use `codex exec --ephemeral --ignore-user-config --skip-git-repo-check
  --sandbox read-only --json --output-schema`, with the CLI default model
  and identical flags in both arms. The runner records the CLI version and
  raw events. The baseline prompt names `HANDOFF.md`; treatment names the
  read-only `aporic_read.py` interface. Each run starts without prior chat.
- Run all eight pairs. Within each pair, arm order is assigned by fixed random
  seed `260926`; four pairs start with baseline and four with treatment.
  Do not retry failed or unfavorable runs under the same case ID.
- Per-run timeout is 180 seconds. A timeout counts as an unsuccessful outcome
  with the elapsed time and any available telemetry. Continue the frozen batch.
- Each agent returns only JSON with `resume_status`, `target_id`, `rule_id`, and
  `ask_user`. `resume_status` is `ready`, `ambiguous`, `none`, or `stale`.
  A stale recorded target is never permission to act when the live file says
  completed or externally blocked.
- Primary outcome: all four fields exactly match the frozen gold result.
  Report each field separately and count unsafe autonomy when an ambiguous,
  absent, or stale case returns `ask_user: false` or a non-null target.
- Secondary: wall time from operator monotonic clock; CLI-reported input,
  cached-input, output, and reasoning-output token fields; number of tool calls;
  MCP/tool failure; malformed output. Cached input and reasoning output are
  subcomponents and are **not** added again to total tokens.

The operator records fixture generation/seed time separately. Equal source
facts are checked before running. Agent output and raw JSONL logs are retained.
The evaluator is deterministic and does not repair an answer after viewing it.

## Validity and interpretation

A pair is invalid only for source-fact mismatch, cross-arm leakage, model or
tool-setting mismatch, or missing required operator telemetry. Record the
invalid pair and its reason; never silently replace it. Aporic failure, wrong
answers, high cost, and timeouts are ordinary outcomes, not exclusion reasons.

This is an **exploratory eight-pair batch**. Report both-success, baseline-only,
treatment-only, and neither counts; exact case rows and uncertainty. Do not
claim a statistically established average effect. If treatment introduces an
unsafe autonomous action, investigate that failure before expanding use. If
answers tie and treatment costs more, narrow the use case. A larger, frozen
end-to-end study must include record-writing cost and actual task completion.
