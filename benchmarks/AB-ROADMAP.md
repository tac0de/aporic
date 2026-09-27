# Aporic A/B evaluation roadmap

Status: planning inventory, 2026-09-27. This is a test plan, not evidence that
Aporic improves agent outcomes. Aporic remains advisory and fail-open. The
product question is whether it reduces owner restatement, coordination, and
correction effort enough to repay its setup and runtime cost.

## What is already known

- Three [core paired pilots](../examples/core-ab/README.md) each used one pair.
  Both arms passed their checks. Aporic was 18 seconds slower for continuity,
  8 seconds faster for bounded context, and 131 seconds slower for evidence
  completion. None establishes an average effect.
- [Continuity v2](continuity-v2/REPORT.md) used eight synthetic pairs. Both
  arms were exact in 8/8; Aporic used 30.5% more time and 43.7% more input
  tokens. The C04 empty CLI output remains an unproved, isolated capture
  anomaly.
- [Restore end-to-end v1](restore-e2e-v1/REPORT.md) used one real two-stage
  Rust task. Both passed the independent grader and produced byte-identical
  product code; Aporic used 21.8% less time and 8.4% fewer input tokens. One
  ordered pair cannot establish causation.
- The [frontend paired pilot](../examples/frontend-browser-review/ab-pilot/README.md)
  passed 8/8 acceptance gates in both arms with equal 7/8 blind quality
  scores. Agent-reported Aporic time was 61 seconds longer. It is feasibility
  evidence, not a repeated effect estimate.
- The new `aporic_recall compact=true` option has a local smaller-response
  check, but no retained task-level A/B result.

## Common protocol for outcome experiments

1. Freeze a task brief, source commit, supplied history, acceptance rubric,
   hidden checks, time limit, and analysis script before either arm runs. Use
   real work from more than one repository and task type. Never reuse an
   already solved example as the treatment task.
2. Compare ordinary Codex work without Aporic against Codex with Aporic.
   Keep the host, model, reasoning effort, non-Aporic tools, permissions, and
   user requirements the same.
   Both arms receive the same underlying historical facts. The baseline may
   use a normal dated handoff or notes; count time spent creating them. Do not
   give either arm a worked solution or the other's artifacts. Freeze the
   treatment's required Aporic calls and decision points per family; optional
   extra Aporic use is recorded as a protocol deviation.
   For C1-C3 mechanism tests, freeze fact content, provenance/recency labels,
   representation budget, and permitted search/navigation in both arms.
   Separately run pragmatic E0 with each workflow's natural representation
   and full authoring cost.
3. Use fresh agent sessions and isolated checkouts/databases. Counterbalance
   arm order across tasks; record the actual provider model identity where the
   host exposes it, otherwise mark it unknown. Log contamination, retries,
   tool failures, and missing telemetry. Count all record-writing, lookup,
   verification, coordination, and operator setup that the workflow needs.
4. Score outputs with the same hidden mechanical tests and an arm-blinded
   reviewer for criteria that need judgment. Keep acceptance/defect results
   separate from wall time, input/output/cached tokens, tool calls, corrective
   turns, and owner interventions. Record unsupported completion, unsafe
   action, and stale-context mistakes as distinct failure classes.
5. Report each paired result and uncertainty. A one-pair pilot only checks
   feasibility. For a directional product decision, use at least six distinct
   tasks per family across at least two task types, with repeated runs where
   model variance is material. Set quality and acceptable-overhead thresholds
   before a confirmatory batch; do not choose them after seeing results.
6. Run deterministic mechanism, replay, migration, and safety tests as
   separate gates. A/B success never replaces those checks. A structural
   invariant passing does not establish user value.

The first confirmatory primary outcome is **blindly accepted task without a
blocking defect**. Track owner correction/restatement effort as a second
product outcome. Time and measured tokens are costs, not proxies for quality.
Treat a new unsafe action or fabricated verification as a stop-and-investigate
event before expanding the treatment. Do not combine heterogeneous families
into one headline win rate.

For owner effort, use the same scripted owner proxy in both arms: identical
opportunities to answer clarification requests and deliver corrections during
the task and one fixed follow-up window. Count owner messages, words of
repeated project facts, requested manual coordination actions, and active
minutes spent on corrections; log the proxy script and all deviations. A real
owner's time is reported separately as an observational measure, not mixed
with the paired proxy result. Blinded graders classify whether each request
was necessary under the frozen brief.

## Candidate comparisons

Priority means execution order, not a claim of expected benefit. Each row
needs its own frozen cases and grader before running.

| ID | Priority | Intervention and baseline | Primary outcome / distinct cost |
| --- | --- | --- | --- |
| E0 | P0 | Full Aporic workflow versus ordinary Codex over a two-session implementation, including writing the history in both arms | Accepted follow-up and owner restatement/correction; all setup, model, and review cost |
| C1 | P0 | `resume` plus handoff versus dated handoff note after a long gap or several intervening tasks | Correct safe target and accepted follow-up; rediscovery time and clarification requests |
| C2 | P0 | Bounded recall, memory search, and supersession versus equally dated raw notes containing old and new decisions, including malicious stored instructions | Current-rule application, untrusted-text authority errors, and unintended writes/external actions; lookup tokens and calls |
| C3 | P0 | Procedural/gotcha memory from a previous failed attempt versus an ordinary failure note | Avoided repeat failure and accepted repair; record creation and retrieval cost |
| V1 | P0 | Typed evidence, registered local-runner check receipt, and exact criterion proof versus equivalent ordinary command execution and completion report | False-complete rate under seeded failed/missing/stale/wrong-scope checks, plus accepted task; registration/execution overhead and test adequacy separately |
| W1 | P0 | Advisory task contracts, leases, scopes, and delegation records versus ordinary host coordination on a genuinely parallel task | Record-level conflict detection, host compliance, final Git collisions, and accepted integration separately; total agent time and coordination turns |
| W2 | P1 | `general` workflow stages/steps versus ordinary task planning on multi-step backend work | First-pass acceptance and missed criteria; planning and rework time |
| F1 | P1 | `frontend` procedure versus normal frontend workflow on new, distinct UI tasks | Blind visual, responsive, keyboard, and functional acceptance using Playwright CLI; procedure cost |
| R1 | P1 | Task-scoped cited research retrieval versus ordinary host research using the same available sources | Correct source-backed decision and unsupported-claim rate; retrieval/review time |
| D1 | P1 | Evidence-linked deliberation plus commit-bound Git state versus ordinary design notes and Git inspection | Decision quality after new contrary evidence and stale-premise/reversal rate; graph upkeep |
| T1 | P1 | `compact=true` or smaller bounded context versus legacy Aporic recall at the same selection facts | Noninferior accepted task and retained required constraints; provider tokens, tool bytes, latency |
| T2 | P1 | Evidence-first task brief versus baseline brief and ordinary instructions, with templates frozen | Accepted task and correction turns; brief generation and trial bookkeeping |
| O1 | P2 | Aporic-guided role/Inspector/coordination reporting versus the same host-run agents with ordinary assignment notes | Review defects found before integration and net parallel-work benefit; reporting overhead |
| A1 | P2 | Accountability case and linked repair plan versus ordinary bug postmortem/handoff | Recurrence on a later analogous task and verified repair; case upkeep |
| P1 | P2 | Evidence-gated prototype portfolio and frontend/product-cell review versus ordinary variant selection | Blind user/task success of selected prototype per budget; variant and playtest cost |
| K1 | P2 | Capability catalog search and bounded schema disclosure versus ordinary tool documentation | Correct capability choice without false authority claim; discovery tokens/time |
| H1 | P2 | Optional hook-based context exposure/trace guidance versus explicit manual recall only | Missed relevant context and task acceptance; hook gaps, payload privacy, latency |
| M1 | Conditional | Advisory model-route recommendation versus a fixed host-selected model on matched tasks | Accepted tasks per measured cost, stratified by task risk; actual model identity must be observable |

E0 is the product-level A/B. C1 through H1 are ablations or targeted
comparisons to explain an E0 result; do not add all interventions at once and
then credit one module. V1 and W1 deserve early testing because prior pilots
showed meaningful overhead or product-critical coordination risk. T1 follows
the compact-recall implementation but remains an untested task-level claim.
In C2, seed equivalent malicious historical text in both arms and check that
neither treats it as current authority, skips a required user decision, or
performs an unrequested write/external action. In W1, Aporic detects or records
conflicts but does not block host tools; give both arms the same host
coordination instructions and score observed agent behavior separately from
record-level detection. In V1, a valid receipt proves only the declared check
ran, so blind review must also assess whether that check was adequate.

## Mechanisms that should not be sold as standalone A/B wins yet

- Backup/restore, event replay, idempotency, schema migration, path bounds,
  fail-open hooks, and required sandbox enforcement need adversarial mechanical
  tests. A paired agent run is not a substitute for correctness or security
  evidence.
- Runtime trace, token receipts, Git snapshots, security artifact import, and
  blind-shadow evaluation are primarily measurement/evidence mechanisms. First
  validate their coverage and calibration against known ground truth; test a
  user-facing intervention only when their output changes an actual decision.
- Government roster, office appointments, and advisory orchestration do not
  dispatch agents or grant authority. Evaluate them within W1/O1 or P1 only
  after a concrete host-run workflow uses the records.
- The external research provider needs a separately frozen source corpus or
  contemporaneous source snapshot. Otherwise source changes confound R1.

## Execution sequence

1. **Instrument and freeze:** reusable paired runner, setup-cost capture,
   model-identity/telemetry audit, blind grader, contamination checks, and
   preregistered thresholds. Use C1/C2/V1 cases as instrument checks.
2. **Core confirmation:** E0 across distinct real implementation tasks,
   counterbalanced in order; then C1, C2, C3, V1, and W1 to identify which
   mechanism helps or hurts. Keep prior pilots out of the confirmatory totals.
3. **Focused modules:** W2, F1, R1, D1, T1, and T2. Revise or remove modules
   that only add calls/records without improving accepted outcomes or owner
   effort.
4. **Conditional breadth:** O1, A1, P1, K1, H1, and M1 only when a real
   workflow, source corpus, users, and reliable instrumentation exist.

For every family, publish the frozen protocol and raw arm results before
writing a cross-family interpretation. Preserve negative results. A feature
can be useful in one workload and wasteful in another; the output of this
roadmap is a usage boundary, not a single universal verdict.
