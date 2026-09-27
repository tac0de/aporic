# Kakao stage fallback: two-session pilot protocol (draft)

Status: **not frozen and no participant run**. This document fixes the common
task and handoff rules. The Aporic tool-call sequence still needs a working
direct Codex MCP route and current workflow-tool discovery before E0 can run.

## Inputs and isolation

- Each of four agent sessions receives a fresh `git archive` export of
  `671cb2df615a1df4151e5f090417e46773bf1043` from
  `kakao-study-groupbot`, without `.git` or later commits. Stage 1 and Stage 2
  use separate exports. The original checkout is read-only.
- Supply the literal [task brief](TASK-kakao-stage.md) to every session. Do not
  expose the hidden test, reference commit, this grader, another arm's output,
  or the original repository's later Git history.
- Before dispatch, enforce filesystem read isolation. A participant process
  may read its own source export, required CLI/runtime files, and its own
  continuity artifact only. It must be denied reads of the original
  `kakao-study-groupbot` checkout and `.git`, Aporic's `benchmarks/e0-v1`
  directory, other arm workspaces, grader fixtures, and raw results. Check
  those denials with explicit canary reads from the same process/sandbox that
  will run the agent; a prompt instruction alone is insufficient. Abort the
  pair if any protected read succeeds. This isolation has not yet been
  implemented or verified.
- Use the same Codex CLI version, `gpt-5.5`, medium reasoning effort, host,
  shell tools, filesystem sandbox, and network policy. Record the literal
  invocation and JSONL telemetry. Provider-side model identity is unknown
  unless the host independently reports it.
- Give each treatment pair a fresh Aporic database and unique workspace ID.
  The baseline has no Aporic tools. Neither arm inherits project notes or
  Aporic history outside its own Stage 1. Verify tool lists before starting.
- Stage 1 has a 10-minute wall limit; Stage 2 has 20 minutes. A timeout is an
  outcome, not a reason to retry. Run baseline first in this instrument pilot;
  reverse order in the next valid case. No agent may commit, push, deploy, or
  make project network calls. Codex's model transport is the only network
  exception; dependencies come from the frozen offline cache.

## Common Stage 1 message

> Investigate the supplied task brief in this source export. Do not edit
> product or test files. Identify the relevant code path, observed behavior,
> constraints, likely verification, and any uncertainty. Produce a concise
> dated handoff for a fresh agent that will implement the task. Distinguish
> observations from hypotheses; do not claim a check ran unless it ran.

The baseline writes its dated handoff to the operator-supplied path outside
the source tree, `HANDOFF_PATH`. The treatment records equivalent material
through its Aporic session and closes with a concrete handoff. Count all
writing calls, final text, bytes, time, and tokens in both arms. Treatment
may make additional Aporic calls, but they are logged as protocol deviations
until the exact required full-workflow sequence is frozen.

## Common Stage 2 message

> You are a fresh agent in a clean export of the same source commit. Recover
> this arm's Stage 1 handoff using its supplied continuity mechanism. Then
> implement the task brief. Change only
> `src/features/quiz/quizFlowService.ts` and add
> `tests/e0-stage-regression.test.ts`. Run that regression test, the existing
> `tests/quiz-answer-routing.test.ts`, and `npm run typecheck:all`. Report
> exact checks, residual uncertainty, and the changed files. Do not commit,
> push, deploy, or contact the network.

The baseline is given only its `HANDOFF_PATH`. The treatment is given only
its isolated Aporic workspace ID and database connection, and must resume
and inspect its Stage 1 handoff before implementation. Stage 2 outputs the
entire source tree for scope audit; no notes or logs belong inside that tree.

## Operator behavior and scoring

If either agent asks for clarification, return exactly: “Use the task brief
and existing code semantics; no additional product requirement is supplied.”
Log the request and reply, and include the elapsed wait in that arm's cost.
No other owner proxy answers are allowed. A missing reply caused by a host
failure invalidates the pair; an ordinary agent request is an outcome.

Before grading, an operator assigns random opaque IDs to the two submissions
and keeps the arm mapping outside grader and Inspector inputs. The same
fresh-tree grader scores both ID-labelled Stage 2 submissions. It checks the
complete submitted tree against the source archive, allows only the product
file and required new test, runs the hidden and original public checks, runs
typecheck, and requires the submitted regression test to pass on the candidate
and fail on the original product file. A distinct Inspector receives only
the ID-labelled source diff, test results, task brief, and final claims. The
Inspector marks a blocking defect only for a violated stated requirement,
unsafe action, or fabricated verification, with a file/log citation. An
ambiguous call is adjudicated against the frozen task brief before unsealing
the arm mapping. Acceptance requires mechanical pass and no blocking defect.
This blind procedure is specified but has not run. Record acceptance
separately from wall time, input, cached-input and output tokens, tool calls,
continuity bytes, operator work,
and grading/setup cost. Do not interpret one pilot pair as an efficacy
estimate. A missing direct MCP route means **no E0 participant pair**.
