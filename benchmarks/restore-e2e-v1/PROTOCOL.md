# Restore destination end-to-end pilot — preregistration

## Question

For one real Aporic Rust correction interrupted between planning and editing,
does Aporic-authored continuity improve verified completion or total measured
agent cost over a conventional handoff file?

This is a **single paired feasibility pilot**, not an estimate of average
product benefit. The independent unit is one task, even though it has two
agent sessions per arm. Do not pool it with the synthetic continuity batch.

## Task and frozen acceptance

Source commit: `aaf736aff7894594f12eec05ceac243b36deb92b`.
`recovery::restore_to` claims a fresh destination but uses `Path::exists`,
which follows symlinks. A dangling symlink at the destination is therefore
silently replaced on Unix. The task is to preserve any pre-existing
destination entry, including a dangling symlink, without regressing valid
restore or backup validation. Source modifications are limited to
`crates/aporic/src/recovery.rs` and relevant `crates/aporic/tests/recovery.rs`.

Primary success: hidden Unix dangling-symlink test plus the public recovery
integration suite pass; the original link and its missing target are unchanged,
and no failed-restore temporary file is left. Existing regular-file conflict
and normal validated restore remain correct. Require an authored recovery test
change mentioning symlink restoration and no unrelated product file changes.
Mechanical grade runs outside both arm workspaces from frozen
`hidden_recovery.rs` and `grade.py`.
Preflight on the frozen source produced three passing public recovery tests
and one failing hidden symlink test, establishing that the grader detects the
initial defect.

## Paired procedure

1. Export the same source commit into isolated baseline/treatment directories.
   Replace the root AGENTS.md in both with an identical neutral trial rule so
   neither arm is told to use Aporic by project defaults. Remove prior
   benchmark examples from both to avoid answer leakage. Same code, task,
   sandbox, CLI default model, timeout and tools.
2. Stage 1: fresh Codex CLI session reads the task and code, makes a bounded
   plan, and stops before editing product code. Baseline writes `HANDOFF.md`;
   treatment writes the plan through the supplied Aporic MCP helper, which
   opens, records, and closes a real isolated session. No operator-authored
   answer is seeded. Record all Stage 1 time and tokens.
3. Stage 2: a new CLI session in each arm reads its respective handoff and
   implements the same task. The Aporic helper exposes `resume` and `recall`
   from the isolated DB. Record all Stage 2 time and tokens. The helper also
   writes a separate transcript of MCP replies, allowing comparison with CLI
   `aggregated_output` after the C04 instrumentation anomaly.
4. Run baseline first, treatment second, fixed before results. This order is
   a limitation; one pair cannot separate arm effect from order or stochastic
   model variation. Do not retry either run under this case ID.

Each CLI call uses `codex exec --ephemeral --ignore-user-config
--skip-git-repo-check --sandbox workspace-write --json` with a fresh session.
Stage timeouts are 180 and 600 seconds respectively. Any failure, timeout,
missing handoff, or malformed output is retained as an outcome. Source-fact
mismatch, cross-arm leakage, or missing operator telemetry invalidates the
pair and stops interpretation. Agent errors are not exclusion reasons.

The setup metadata freezes source and file hashes before any agent session.
The grader reconstructs its own checkout from the source commit and overlays
only the allowed submitted recovery files; a separate hidden test remains
present even when an authored test is weak. The runner retains command traces
and scans them for sibling workspace references. This is an audit signal, not
an OS-level guarantee against cross-arm reads; any observed cross-arm access
invalidates the pair.

## Metrics and interpretation

Primary: independently graded task success in each arm. Secondary: Stage 1,
Stage 2 and combined monotonic-clock wall time; actual CLI input, cached
input, output and reasoning-output tokens; tool calls; modified paths;
history artifact size; Aporic MCP command status, reply bytes and CLI event
capture parity. Cached input and reasoning output are subcomponents, not
added again to total tokens. Fixture export and operator grading cost are
reported separately, not counted as agent productivity.

If both pass, compare full two-session cost and inspect whether each actually
used its saved plan. If only one passes, inspect the code and trace before
attributing the difference to history. A single pair cannot establish a
statistical effect or justify default Aporic use.
