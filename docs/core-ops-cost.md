# Core continuity: operating cost evidence

Snapshot: 2026-09-27, repository `a3c00cb`. This note evaluates the work needed to maintain and use durable continuity, rather than treating the number of tool calls as the outcome. Aporic is advisory; its records do not authorize host actions.

## What the existing paired work shows

| Workload | Accepted result | Observed cost | Limit |
| --- | --- | --- | --- |
| [Continuity v2](../benchmarks/continuity-v2/REPORT.md), eight synthetic retrieval pairs | Both arms exact in 8/8 | Aporic 110.03 vs 84.34 seconds (+30.5%); 330,038 vs 229,639 input tokens (+43.7%); 24 vs 8 command calls | Prewritten history; no record-authoring effort. C04 had an unexplained empty tool capture. |
| [Restore end-to-end v1](../benchmarks/restore-e2e-v1/REPORT.md), one two-session Rust pair | Both passed, byte-identical product implementation | Aporic 185.63 vs 237.46 seconds (−21.8%); 388,282 vs 423,775 input tokens (−8.4%) | Baseline ran first; independently written handoffs differed in length; one pair cannot establish an average effect. |
| [Three core paired pilots](../examples/core-ab/README.md) | Both arms passed each hidden rubric | Aporic +18 seconds for continuity, −8 for context, +131 for evidence completion | One pair per mechanism; most token telemetry unavailable. The evidence path exposed a confusing manual claim interaction. |
| [Frontend paired pilot](../examples/frontend-browser-review/ab-pilot/README.md) | Both passed 8/8 gates and scored 7/8 blind quality | Agent-reported Aporic time +61 seconds | One pair; author tokens and tool counts unavailable. The Aporic task remained incomplete in its own workflow state despite blind browser acceptance. |

These results support feasibility and show that overhead can be material. They do not establish a net benefit or penalty across real tasks. The [A/B roadmap](../benchmarks/AB-ROADMAP.md) correctly makes accepted work and owner correction/restatement the outcomes, with time and tokens treated as costs.

## Small local protocol probe

After `cargo build --quiet`, I ran the local `target/debug/aporic` binary as one MCP stdio server per repetition with a fresh temporary SQLite DB and workspace. Each of three repetitions did `initialize`, then `open → record → close(handoff) → resume → recall` sequentially with one short observation and handoff. Timing covers the local JSON-RPC request/response round trip after server initialization; it excludes process startup and model activity. Response bytes are UTF-8 sizes of the returned MCP line and its inner Aporic JSON text. The [probe script](../benchmarks/core-ops-v1/probe.py) and [raw local result](../benchmarks/core-ops-v1/result.txt) retain the command and binary digest.

| Call | Median local round trip | MCP response line | Inner JSON text |
| --- | ---: | ---: | ---: |
| `open` | 8.21 ms | 1,691 B | 1,503 B |
| `record` | 4.80 ms | 525 B | 395 B |
| `close` | 7.53 ms | 312 B | 200 B |
| `resume` | 4.24 ms | 867 B | 721 B |
| `recall` | 4.50 ms | 2,130 B | 1,880 B |
| Five-call sequence | 26.58–29.44 ms observed | 5,525 B | 4,699 B |

All 15 replies reported `ok=true`. These are three local samples, not a latency distribution or production service benchmark. Byte sizes are invariant here because each fresh fixture had the same short content. They are **response transport bytes**, not provider input tokens. The probe did not measure tool-schema exposure, request bytes, host routing, model deliberation, repeated context, review/record-writing time, or the cost of selecting relevant facts. It also did not include task/workflow, delegation, evidence, or verification calls. The current [MCP server instructions](../crates/aporic/src/mcp.rs) request a delegation assessment after `open` for substantive work, so this five-call probe understates the current instructed path by at least one tool call.

## Live task-record friction

In this workspace, `aporic_resume` initially returned six unfinished task contracts. Five had corresponding implementation commits already in `main` (v0.23, v0.24, native frontend workflow, v0.25, and v0.26), but none had Aporic criterion proofs. They were cancelled with commit-specific reasons, **not** marked verified-complete; the E0 groupbot instrument task remains queued. A fresh `aporic_resume` now returns that one E0 task. The [resume selector](../crates/aporic/src/store.rs) prioritizes queued or leased tasks and cannot infer from a Git commit that their broad acceptance criteria were satisfied. The manual comparison and cancellation are real owner/agent upkeep absent from the small protocol probe. Future task contracts should use mechanically provable criteria when verified completion is needed; exploratory work can use sessions and handoffs without creating a task that is difficult to close truthfully under the exact proof gate.

## Provisional scope decision

Keep the continuity and evidence path as the unit of evaluation: bounded session context, durable records and handoffs, claim provenance, and replayable storage. Do not make capability catalogs or prototype portfolios prerequisites for that path. Their presence in the one Rust crate is source organization and historical compatibility, not evidence that they improve owner outcomes. Freeze new default-workflow requirements from those layers while the core cost test is unresolved. Remove or archive a layer only after identifying its live callers, stored data, and a migration or read-only history path; tool count alone is not the decision criterion.

## Cost model and next test

The direct local server cost of this short core sequence is small. The consequential burden is elsewhere: deciding what merits a durable record, writing accurate provenance and handoff text, reading and checking old state against current Git/user intent, correcting stale records, and carrying extra tool results through model context. Those steps can repay their cost only if they reduce rediscovery, owner restatement, mistakes, or correction on later work. The synthetic retrieval batch shows measurable model-side overhead even when record creation is omitted; the Rust pair shows that a full two-session task can still finish faster. Neither identifies which part of the structure causes the difference.

Next, run a **frozen two-task instrument pilot** before a confirmatory batch: two distinct real two-session changes, each with ordinary dated notes and Aporic arms, alternate arm order, and give both arms the same historical facts. Capture setup and record-authoring minutes; host input/output/cached tokens and elapsed time; MCP payload bytes/latency; owner-proxy clarification, restatement, and correction minutes; and blind acceptance plus an independent stale-context audit. Include every required Aporic call and ordinary-note upkeep. Retain raw per-arm traces and the frozen rubric locally. If telemetry and grading work, expand under the [roadmap](../benchmarks/AB-ROADMAP.md) to at least six distinct tasks per family with predeclared quality and acceptable-overhead thresholds. Keep the one-pair pilots out of that estimate.
