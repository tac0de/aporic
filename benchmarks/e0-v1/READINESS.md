# E0 paired real-task benchmark: readiness audit

Date: 2026-09-27. **No E0 participant pair has run.** This is an instrument
and task-selection record, not a frozen protocol or an outcome report.
The [roadmap](../AB-ROADMAP.md) defines E0 as ordinary Codex versus the
current Aporic-assisted workflow across a two-session real implementation
task. Each stage's planning, record writing, retrieval, verification, and
operator effort must be counted. A hidden grader must score both submissions.

## Repository choice

The human first preferred Aporic alone, then asked whether Plotnodes or
`kakao-study-groupbot` would improve the comparison. A second repository
does improve external validity. Prefer **kakao-study-groupbot** for the first
external stratum: its main checkout is clean and synchronized, it has a
locked Node/TypeScript toolchain, and it has many focused behavioral tests.
Plotnodes has useful historical fixtures but its live main contains
uncommitted design work and a product-direction transition. Do not alter
either original checkout; export immutable source commits into isolated
participant workspaces without their Git object databases.

The Kakao stage case is selected for the first instrument pilot but its
paired-run protocol is **not frozen**. The other cases remain candidates.
Each needs a new hidden grader, source preflight that fails only the intended
check, public-test integrity check, leakage audit, and precise task brief
before an arm runs. The first Kakao stage case now has a
[task brief](TASK-kakao-stage.md),
[two-session protocol draft](PROTOCOL-kakao-stage.md),
[hidden check](hidden_kakao_stage.test.ts), [fresh-tree grader](grade_kakao_stage.py), and
[preflight summary](preflight-kakao-stage.json). On the pre-fix source the
hidden check failed while public focused tests and typecheck passed; on the
reference fix all three passed. This validates the instrument's direction
only. Its direct-MCP path and participant filesystem isolation remain open,
so no agent arm has run.

| Candidate | Pre-fix source | Intended acceptance | Main limitation |
| --- | --- | --- | --- |
| Kakao: stage-less direct subject quiz | parent of `eb892971` (`671cb2df`) | Math/science/Korean/English direct quiz serves a card with middle-stage default | Localized and deterministic; good first instrument case |
| Kakao: exact hub button labels | parent of `b4dbe36b` (`c0407854`) | Five product button labels map to existing commands; mentions and nonexact prose remain correct | Freeze exact labels and Unicode whitespace cases |
| Kakao: secure image URL list | parent of `254aaeeb` (`a39c774a`) | Select permitted Kakao HTTPS image from `List(...)`; reject arbitrary HTTP origins | Security-sensitive; hidden checks must include rejection cases |
| Aporic: compact recall | parent of `e7914fb` (`3aaa65a`) | Selected items and budget preserved; raw lists absent only in compact mode | Current Aporic MCP itself exposes this feature, risking treatment answer leakage |
| Aporic: versioned workflow procedures | parent of `5decc6d` (`bc9c24d`) | Versioned steps, evidence gates, restart/rework behavior | Large task; current MCP exposes its API, risking answer leakage |
| Plotnodes alternate: node-edge geometry | `e7151201` | Deterministic edge intersections for horizontal/vertical/diagonal nodes | Clean historical export required; choose only if third repository is in scope |
| Plotnodes alternate: malformed generated links | `298d1542` | Normalize bounded generated links before semantic validation | Task brief must distinguish repair from valid rejection |

Do not use the existing restore-symlink case or earlier benchmark/example
cases. Do not fill six slots with trivial tasks merely to reach a sample
size. Aporic self-hosted product-feature cases need a leakage control:
the treatment's current MCP tool definitions and responses must not reveal
the historical reference behavior being implemented. A prospective unrelated
Aporic task or more external cases may be safer.

## Instrument preflight observed

- `codex-cli 0.154.0` accepted `--model gpt-5.5` with
  `-c 'model_reasoning_effort="medium"'` in an isolated, ephemeral one-line
  run. It emitted input, cached-input, and output token usage. Its JSONL did
  not independently attest the provider-side model identity.
- The same CLI rejected `gpt-6-sol` for this ChatGPT account. Do not select
  it merely because the Codex desktop host lists it.
- A current release Aporic binary built offline. In one fresh CLI workspace,
  a direct `aporic_recall` MCP call was visible but denied because its
  approval policy was `never`. Separate `--approve-for-me` and one-shot
  configuration probes did not expose a callable Aporic tool. CLI MCP server
  registration alone did not establish a working route. No participant pair
  can use this as a repeatable direct MCP path yet.
- The older benchmark's command helper calls the real Aporic stdio MCP
  server and captures replies, but it excludes Codex's direct MCP schema and
  approval cost. If used, label the experiment **Aporic core via helper**,
  not full installed-plugin E0. Preserve the failed direct-MCP probes as
  instrument evidence, not product-outcome failures.
- A targeted macOS `sandbox-exec` smoke, outside the nested host sandbox,
  denied reads of the original Groupbot checkout and hidden test while
  allowing the participant's own source export. This has not established
  complete allowlist isolation or a working Codex CLI launch under that
  profile; participant access control remains a gate.

## Gates before a frozen paired run

1. Establish one host-approved, repeatable Aporic MCP route in isolated CLI
   sessions, or explicitly choose the narrower helper-mediated question.
   Verify open/record/close and resume/recall in two fresh sessions, with
   unique database/session IDs and matching tool reply/event transcripts.
2. Select cases whose source exports contain no later commits, benchmark
   solutions, hidden tests, or current documentation that reveals the fix.
   Freeze source archives, exact prompts, allowed paths, public and hidden
   checks, original-test preservation checks, and hashes before a run.
3. Freeze the literal CLI model/config, same non-Aporic tools and permissions,
   stage timeouts, arm order, error/invalidity rules, and all required Aporic
   calls. Counterbalance order across the eventual batch; no retries or
   substitutions after results.
4. Implement a versioned owner proxy if measuring restatement/correction.
   It must offer the same scripted clarification opportunities and a fixed
   follow-up window to both arms, log requests/answers, and have a declared
   time-cost lookup. Until then, report only blinded rubric-estimated
   remediation burden, not measured owner time.
5. Grade anonymous submitted product files in fresh, separate trees with
   frozen original public tests plus independent hidden tests. Keep arm
   mapping, history artifacts, timing, and tool traces from graders until
   scores are final. Define blocking defects and adjudication in advance.
6. Report quality and cost separately: paired accepted-task counts, false
   completion, unsafe action, stale context; complete two-stage wall time,
   model tokens, cached subset, calls, Aporic/notes bytes, setup and grader
   cost. An unsafe or fabricated-verification event in either arm triggers
   investigation. Ordinary wrong answers and timeouts remain outcomes.

An independent fixture review originally found four freeze gaps. The draft
protocol now specifies both session messages and the handoff/submission
rules. The grader now audits the complete submitted tree, requires the added
regression test to pass on the submission and fail on the original product
file, and preserves original public files. The hidden check now covers all
three explicit school stages as well as missing and general profiles. A
same-host npm cache archive is frozen by digest, but it is still held in
temporary local storage rather than a portable published artifact. The
direct-MCP and participant filesystem-isolation gates remain open. The blind
Inspector procedure is specified but has not run. These are
instrument gaps, not participant failures.

The next step for the **instrument pilot** is to resolve the Aporic MCP access
gate and these freeze gaps. The Kakao stage-less quiz case is selected, and
its independent hidden grader fails on the pre-fix source while passing the
reference fix. Freeze the pair's protocol before agents run.
Then decide whether to expand to the remaining cases. A six-task,
two-repository interpretation remains conditional on valid fixtures and
balanced instrumented runs.
