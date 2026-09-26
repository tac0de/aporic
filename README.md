# Aporic

Aporic is a local-first agent-system hub whose practical objective is to reduce
the time its owner spends restating context, coordinating independent work, and
correcting unsupported completion claims.

The active implementation is a modular Rust monolith in `crates/aporic`. Codex is the
first client and connects through a local stdio MCP server. The exact behavioral
kernel is stored in `canon/`; it constrains the evolving hub but does not contain
transport, storage, host, or agent-runtime behavior.

See [PRODUCT.md](PRODUCT.md) for the product objective and
[architecture.md](docs/architecture.md) for the current boundaries.

## Current product surface

The hub exposes MCP tools for continuity, coordination, evidence, research,
accountability, and other advisory project work.

Continuity:

- `aporic_open`: open an idempotent work session and receive bounded context;
- `aporic_recall`: retrieve recent durable project context;
- `aporic_resume`: select one unfinished task or fresh handoff for a terse
  new-session continuation request, or report ambiguity;
- `aporic_record`: record one typed durable fact or work-state change;
- `aporic_close`: complete or hand off a session;
- `aporic_reconcile`: abandon stale interrupted sessions without claiming success.

Advisory roles:

- `aporic_roles_list`: inspect four versioned duty and output contracts;
- `aporic_role_appoint`, `aporic_role_revoke`, and
  `aporic_role_appointments`: record and inspect bounded assignments. Optional
  catalog capability references are associations, not executable grants.

Advisory government composition:

- `aporic_government_get`: inspect the versioned Aporic government charter;
- `aporic_government_bootstrap`, `aporic_government_person_register`,
  `aporic_government_term_appoint`, `aporic_government_term_end`, and
  `aporic_government_roster`: explicitly initialize a workspace's advisory
government roster, maintain stable people and position identities, and retain
appointment, replacement, and retirement history;
- `aporic_office_appoint`, `aporic_office_revoke`, and
  `aporic_office_appointments`: bind one active product minister to an existing
  session-scoped Steward appointment without granting authority;
- `aporic_product_cell_create` and `aporic_product_cell_list`: record one
  immutable, task-bound multidisciplinary cell with distinct planning and
  prototype-delivery assignees, explicit problem, hypothesis, and success
  measures.

Memory lifecycle:

- `aporic_memory_search`: search the deterministic FTS projection under class,
  time, item, and byte bounds;
- `aporic_memory_get`: inspect one workspace-scoped memory item with provenance,
  lifecycle, temporal validity, and applicability metadata.

Advisory coordination:

- `aporic_task_create`: define a task contract, dependencies, and write scope;
- `aporic_task_list`: inspect bounded task state;
- `aporic_task_claim`: acquire a time-bounded worker lease;
- `aporic_task_complete`: complete only when every criterion exactly matches a
  verified mechanical claim;
- `aporic_task_cancel`: cancel without implying completion.

Client feedback and prototype planning:

- `aporic_improvement_submit` and `aporic_improvement_list`: submit a concise
  evidence-referenced request from a client workspace to Aporic core and read
  its queued or later task status. Registration does not start implementation;
- `aporic_prototype_brief_create`, `aporic_prototype_review`, and
  `aporic_prototype_get`: record an active task's target user, experience
  hypothesis, fidelity, reuse and repository boundaries, technology plan, and
  validation method; review smoke checks, rules, and observed play separately.
  User value remains unknown without a current direct `.playtest.json` report;
- `aporic_related_workspaces`: optionally inspect Git repository names and
  explicit labels under supplied local roots. Empty roots disable discovery.
  Results are historical context and do not copy or merge code.

Design delivery in v0.21 uses a versioned repository manifest to connect
references, a selected concept, design tokens and component specifications,
implementation, and browser review. `aporic_design_validate` and
`aporic design validate --workspace PATH --manifest RELATIVE_PATH` check local
files and hashes without judging or approving the design. See the
[design delivery workflow](docs/design-delivery.md).

The playtest report is a JSON file in the task workspace with nonempty
`target_user`, `observed_behavior`, and `session_date` fields and a positive
`participant_count`. Register it with `aporic_evidence_add` as a workspace
file before citing its evidence ID in `aporic_prototype_review`. The review
records that an observation artifact exists; it does not judge whether the
experience was enjoyable. Feedback intake accepts a concise, single-line
objective and criteria plus a source evidence ID. It rejects obvious credential
strings and stores no source conversation or evidence body in the core task.

Accountability and repair:

- `aporic_accountability_open`: record an evidence-labelled task failure as an
  advisory case; any assignee attribution remains reported, not established;
- `aporic_accountability_plan`: preserve a model-authored root-cause hypothesis
  and prevention change while linking a later repair task; revisions remain in
  the event log;
- `aporic_accountability_resolve`: mark a case repaired only when the linked
  task has completed with verified criterion proofs;
- `aporic_accountability_list`: inspect bounded open and repaired cases,
  evidence grades, and the outstanding repair count.

Epistemic gate:

- `aporic_evidence_add`: classify provenance as direct, reported, or model-only;
- `aporic_claim_assert`: keep observed, verified, inferred, assumed, intended,
  and unknown claims distinct;
- `aporic_dissent_assess`: surface only consequential, actionable dissent with
  direct evidence.

Model routing:

- `aporic_model_route`: recommend Astra, Sol, or Terra from typed task signals.

Verifiable execution history:

- `aporic_check_register`: register an immutable argv-based local check without
  executing it;
- `aporic_run_list`: list bounded run lifecycle state;
- `aporic_run_get`: inspect one receipt, its hashes, declared artifacts, and any
  mechanically issued claim.

Runtime observation:

- `aporic_trace_list` and `aporic_trace_get`: inspect privacy-minimized,
  append-only host lifecycle observations;
- `aporic_capability_report`: summarize tools and inferred capability classes
  actually seen through configured hooks;
- `aporic_hook_health`: report duplicates, unmatched tool lifecycles, unknown
  events/capabilities, and schema drift without claiming complete coverage.

Git evidence and governance:

- `aporic_git_observe`: run bounded, read-only, non-network Git metadata
  inspection and append a commit-bound governance snapshot;
- `aporic_git_snapshot_list` and `aporic_git_snapshot_get`: inspect prior
  repository observations without treating findings as review or merge approval.

Token efficiency:

- `aporic_token_usage_record`: append a provenance-labelled usage receipt;
- `aporic_token_usage_list`: inspect bounded, append-only usage history;
- `aporic_token_efficiency_report`: separate measured counts, conservative
  estimates, cached input, and verified outcomes.

Commit-bound deliberation:

- `aporic_deliberation_create`: open a concise public question bound to an
  existing Git snapshot;
- `aporic_deliberation_node_add`: append a typed premise, claim, objection,
  counterexample, falsifier, value constraint, material unknown, proposal, or
  revision with explicit graph relations;
- `aporic_deliberation_decide`: record only a provisional decision while
  preserving its open material issues;
- `aporic_deliberation_get` and `aporic_deliberation_list`: inspect graph
  history, unresolved aporia, and commit/tree staleness; list returns compact
  summaries and graph detail supports sequence cursors with explicit
  truncation at fixed bounds.

Secure capability catalog:

- `aporic_capability_register`: record an immutable, schema-bounded manifest
  without loading code or granting execution authority;
- `aporic_capability_search`: retrieve compact summaries before paying the
  context cost of a full schema;
- `aporic_capability_get`: inspect one manifest, risk declaration, digest, and
  catalog state. Every capability reports `executable: false`.

Evidence-gated prototype portfolio:

- `aporic_experiment_create`: bind a hypothesis, budget, hard gates, and Pareto
  dimensions to a clean committed Git snapshot;
- `aporic_experiment_variant_add` and `aporic_experiment_measurement_add`:
  preserve distinct commit-bound variants and measurements with typed evidence;
- `aporic_experiment_decide`: require a zero-open-issue deliberation before
  final selection;
- `aporic_experiment_get` and `aporic_experiment_list`: inspect bounded
  portfolios, budgets, Pareto candidates, and integration staleness.

Security evidence bridge:

- `aporic_security_assessment_get` and `aporic_security_assessment_list`:
  inspect locally imported, commit-bound Codex Security artifact summaries.
  Zero findings and partial coverage never become a safety claim.

Zero-effect advisory orchestration:

- `aporic_orchestration_run_create`: bind a Hermes run envelope to one active
  task, clean Git snapshot, recorded context exposure, role, and hard budget;
- `aporic_role_report_submit`: seal one bounded Steward- or Worker-style report
  without invoking a model, dispatching an agent, executing a tool, or claiming
  task completion;
- `aporic_shadow_evaluate`: compare a pre-outcome report with the later
  independently recorded task outcome and verified criterion proofs;
- `aporic_orchestration_run_get` and `aporic_orchestration_run_list`: inspect
  runs without revealing blind-shadow report content before evaluation.

MCP cannot execute a registered check or submit a receipt. A human or local
automation invokes `aporic verify --spec SPEC_ID`; the Rust runner executes the
exact registered program and argv without a shell command string, captures only
output hashes and byte counts, records Git/worktree snapshots and declared-file
hashes, then issues a verified claim only for the expected exit code and complete
artifact set. Failed, timed-out, interrupted, or incomplete runs cannot issue
that claim. Output is hashed as a bounded-memory stream. On Unix, timeout cleanup
targets the normal child process group.

Command specifications may additionally require the Linux `bubblewrap` backend.
That profile runs with separate user, PID, IPC, UTS, cgroup, and network
namespaces; a minimal read-only system view; a private home and `/tmp`; and an
explicit read-only or read-write `/workspace`. The runner records which backend
actually started and refuses to issue a verified claim if required enforcement
cannot be established. Required isolation never silently falls back to host
execution. The legacy/default `host` profile remains reduced-environment process
execution and is not a security sandbox.

Coordination records do not launch agents, grant host authority, or block tools.
They make parallel-work conflicts and unsupported completion claims visible at
the hub boundary.

Decision and constraint records may explicitly supersede older records. New
effect and verification records use the typed evidence/claim path: free text,
reported command output, and model assessment cannot become `verified`.
Aporic-direct verification means that Aporic itself read and hashed a file inside
the session workspace, or that its local runner generated an execution receipt
for a pre-registered command specification. A receipt proves the observed
process result and declared artifacts, not that a test is semantically adequate
or that an external real-world effect occurred. Material unknowns block
completion until an observed or verified claim supersedes them.

## Memory lifecycle and authority-bound context

v0.7 adds a rebuildable memory projection over records, claims, tasks, handoffs,
and local execution receipts. Memory is classified as episodic, semantic,
procedural, gotcha, or unknown. Supersession closes temporal validity without
deleting history, while explicit relation edges preserve why state changed.
SQLite FTS5 provides deterministic local retrieval without embeddings, a vector
service, or a model/API call.

The deterministic context capsule remains alongside the legacy recall fields.
Every selected item carries an origin channel, an influence class, and explicit
selection reasons. Model/MCP-authored free text enters as `untrusted_content`; it cannot
declare itself an instruction, verified fact, permission, or completed effect.
Verified/observed typed claims are the only recalled items promoted to
`verified_fact`.

Selection is deterministic and byte-bounded. It favors unresolved material
unknowns, active constraints and decisions, active tasks, directly supported
claims, and recent handoffs, then uses objective/path token overlap, recency,
and stable identifiers as tie-breakers. Superseded records and claims are not
selected. The policy digest and budget accounting travel with every capsule.

An optional, API-free Codex command hook is available as `aporic hook codex`.
It accepts `SessionStart` and `UserPromptSubmit` event JSON on stdin and returns
only `hookSpecificOutput.additionalContext`, following the official
[Codex Hooks contract](https://learn.chatgpt.com/docs/hooks). Prompt text is used ephemerally for
ranking; prompt text, transcript paths, and assistant messages are not persisted.
Session and turn identifiers are retained only as installation-keyed HMACs in
append-only exposure receipts. Malformed input, an unavailable database, or an
unknown workspace returns `{}` with a successful exit, so the integration is
advisory and fail-open. The model and permission mode are labeled as
host-observed, not attested.

The example in `integrations/codex/hooks.json.example` is intentionally not
installed. Copying it into an active Codex configuration and trusting the hook
remain explicit operator actions.

## Runtime trace and capability observation

v0.8 extends the optional hook across session, prompt, tool, permission, stop,
and session-end events. It stores event metadata, inferred capability class,
outcome, latency, payload byte counts, and installation-keyed HMACs of payloads
and correlation identifiers. Raw prompts, tool inputs, tool outputs, transcripts,
and assistant messages are never durable trace fields. Metadata strings are
control-character stripped and bounded.

A deterministic shadow assessment labels observations `observe`, `warn`,
`would_ask`, or `would_deny`. Those labels are counterfactual telemetry only:
the hook always returns `{}` for tool and permission events, never grants,
denies, delays, or requests approval, and fails open on any error. Capability
reports show only what installed hooks observed; absence is not proof that a
host lacks a capability or that every action was captured. Hook health reports
`no_detected_gaps` separately and always leaves `coverage_proven` false.

Tool events sharing a hashed host session and turn are linked to the latest
memory-exposure receipt. A local, content-free OpenTelemetry-shaped JSON export
is available without a network exporter:

```console
cargo run -p aporic -- trace export --workspace /absolute/project/path
```

## Git evidence and governance

v0.9 treats Git state as local evidence rather than authority. A snapshot binds
the observed HEAD commit/tree, branch, locally cached upstream relation,
comparison base and merge base, dirty counts, changed paths, worktree metadata,
remote names, signature presence, and successful Aporic receipts to one SHA-256
digest. It stores metadata and paths, never diff contents.

Git inspection uses fixed argv without a shell and disables optional locks,
fsmonitor, external diff commands, submodule recursion, terminal prompts, and
replace objects. It never fetches or mutates the repository. Consequently,
`remote_state_fresh` and `approval_proven` are always false. An embedded
signature is reported only as present; signer identity and trust are not
verified. A successful receipt bound to HEAD proves only that one registered
check succeeded at that commit, not that the check was adequate.

The deterministic policy surfaces dirty or detached state, divergence from the
locally cached tracking ref, unresolved bases, missing commit-bound receipts,
governance-sensitive paths, and truncated inventories. Findings are advisory
and cannot authorize a merge. Inspect and record the current repository state:

```console
cargo run -p aporic -- git inspect --workspace /absolute/project/path
```

## Token efficiency and context control

v0.10 records host-reported, local-tokenizer, conservative byte-upper-bound,
and unknown usage as different provenance classes. Cached input remains part of
input usage and is reported separately; it is never presented as removed
context. A verified success or failure must bind to a matching Aporic-direct
verified claim or execution outcome from the same workspace. Arbitrary model or
operator text cannot establish the denominator for “tokens per verified
success.”

Context and memory selection now remove exact duplicate content after safety
priority sorting. Every budget reports candidate, duplicate, oversized, and
item-limit counts. UTF-8 bytes are exposed only as a conservative token upper
bound, not as a provider tokenizer result. Usage receipts are digest-bound and
included in project export and `doctor` consistency checks.

Inspect locally recorded efficiency evidence:

```console
cargo run -p aporic -- tokens report --workspace /absolute/project/path
```

The router is advisory and outside the behavioral kernel. It does not dispatch a
model, grant authority, or turn a model review into evidence. See
[model routing](docs/model-routing.md).

## Prompt and context evaluation baseline

`aporic eval context` runs fixed offline selection fixtures. Its report separates
required-item coverage, selected untrusted text, authority-label preservation,
budget compliance, and deterministic replay. These fixtures measure the selector,
not model answer quality or causal task improvement; they make no network or
model calls.

`aporic_task_brief` assembles a bounded, versioned advisory brief for one task
from its objective, acceptance criteria, and currently selected context. The
result includes a template digest, context-policy digest, selected item IDs,
and brief digest. Schema v23 records only this receipt and its event; it does
not store the rendered brief or a new copy of the user's prompt. Reusing an
idempotency key with changed source context returns a conflict. Brief content
and recalled text remain data and do not change host instructions, agent
dispatch, or permissions.

## Commit-bound deliberation

v0.11 represents inspectable public reasons rather than hidden model reasoning.
A graph begins from a question bound to an existing clean, committed Git
governance snapshot; unborn or dirty snapshots are rejected.
Nodes have a closed type vocabulary and edges distinguish support, attack,
undercut, dependency, falsification, and revision. Material challenges require
direct evidence or an observed/verified claim; unsupported model rhetoric
cannot become a blocking objection merely by declaring itself important.

Material objections, counterexamples, and falsifiers remain open until a later
undercut or revision targets them. A material unknown is stricter: narrative
revision cannot close it, and a directly evidenced undercut is required. A
provisional decision records how many issues were still open. After a newer Git
observation changes the bound HEAD commit or tree, reads mark the graph stale.
The newest observation being dirty also marks it stale.
New decisions on that stale graph are rejected; the graph must be recreated
against current Git evidence. This is deliberative memory, not approval,
authorization, merge safety, or a tool gate.

Inspect one graph:

```console
cargo run -p aporic -- deliberation show --workspace /absolute/project/path --id GRAPH_ID
```

## Secure capabilities and prototype experiments

v0.12 keeps a single Aporic MCP surface while separating the exact behavioral
kernel, Hub-owned catalog state, and future provider runtimes. Manifests are
bounded, digest-bound data. There is no generic provider invocation, credential
broker, network path, or plugin code loader.

Experiment campaigns reject exact approach duplicates and variant-budget
overflow, require direct evidence or an observed/verified claim for hard-gate
qualification, and compute non-dominated Pareto candidates only after evidence
is complete. Final selection additionally requires a v0.11 deliberation
decision whose material-issue count is zero.

Completed Codex Security artifacts are imported through the local CLI using an
explicit JSON request. The importer reads bounded regular non-symlink files,
hashes the exact parsed bytes, normalizes finding counts and coverage, and never
launches the scanner:

```console
cargo run -p aporic -- security import-codex --request /absolute/import-request.json
cargo run -p aporic -- backup --to /absolute/aporic-backup.sqlite3
cargo run -p aporic -- restore --dry-run /absolute/aporic-backup.sqlite3
cargo run -p aporic -- restore --from /absolute/aporic-backup.sqlite3 --to /absolute/restored.sqlite3
cargo run -p aporic -- backup prune --dir /absolute/backups --keep 7
```

## v0.13 stabilization boundary

v0.13 converts the main resource limits into enforced boundaries. Workspace
evidence and receipt artifacts are hashed as bounded streams; command specs cap
artifact count and aggregate receipt bytes; hook stdin is rejected fail-open
above 1 MiB; and Git stdout/stderr are captured through bounded pipes that stop
the child at 2 MiB. Task write scopes use a lower-case, portable ASCII,
forward-slash lexical form. They reject absolute paths, backslashes, parent
traversal, trailing dots or spaces, Windows-reserved components, and alias-prone
characters before persistence.

Runner and governance snapshots share the same hardened Git invocation, so
repository-configured fsmonitor helpers are disabled. Restore copies and
validates a backup into a new destination, migrates it to the supported schema, syncs the
temporary file, and atomically renames it. It deliberately refuses to overwrite
an existing database. Retention removes only regular non-symlink files matching
`aporic-backup-*.sqlite3` and requires an explicit keep count.

These controls improve local reliability; they do not turn the runner into a
sandbox or establish model-driven product utility.

## v0.14 execution-governance boundary

v0.14 gives each capability manifest an explicit maturity stage: `observe`,
`propose`, `sandboxed_execute`, `connected_effect`, or `persistent_routine`.
Registration enforces a minimum stage from the declared effect class, and a
persistent routine must be idempotent. The stage is catalog metadata, not an
execution permission; every registered capability remains non-executable through
MCP.

Verification specifications now carry a versioned sandbox profile. `host`
preserves existing behavior. `required` is implemented on Linux with
`bubblewrap`, requires network denial, and chooses read-only or read-write
workspace access. Unsupported platforms, a missing backend, invalid mount setup,
host policy that forbids unprivileged user namespaces, or missing sandbox-start
evidence fail the run rather than downgrade it. Receipts persist the selected
backend and whether enforcement was established.

This is a bounded verification worker, not a general untrusted-code service.
It does not yet impose cgroup CPU, memory, or process-count quotas; use a custom
seccomp policy; provide a VM/microVM boundary; execute catalog capabilities; or
verify remote side effects. The existing timeout remains the resource-lifetime
limit.

## v0.15 zero-effect advisory orchestration

v0.15 adds an Aporic-native Hermes coordinator as deterministic data contracts,
not as an agent runtime. A run is immutably bound to an active task, clean Git
commit/tree, one recorded context exposure, a Steward or Worker role, and input,
output, and duration budgets. Its capability ceiling is always `propose`.

`blind_shadow` reports are submitted before the task outcome, sealed from read
and export surfaces, and revealed only after deterministic evaluation against a
later completed or cancelled task. Completion is evidence-eligible only when
every task criterion has an Aporic-direct verified proof and the Git binding is
not stale. `visible_advisory` exposes advice immediately but grants no extra
authority. Reports cannot complete tasks, call providers, use credentials,
access the network, mutate a workspace, or trigger tools.

## v0.16 roles and new-session continuity

The four foundation roles are Prime Minister (conversation and synthesis),
Steward (project continuity), Worker (bounded delivery), and Inspector
(independent review). A role is a versioned duty contract; an appointment binds
an assignee and optional model hint to a session or active task. Worker and
Inspector assignments for one task must use different assignee IDs. Capability
references must exist in the workspace catalog and remain advisory. Installing
a provider or recording an appointment never changes host permissions or
dispatches an agent. The human's Codex UI model choice stays with the host.
Hermes runs may bind a matching, active Steward or Worker appointment; older
unbound runs remain readable with their original digest.

For `이어한다` in a new session, the bridge calls `aporic_resume` before opening
new work. It selects exactly one unfinished task, otherwise one interrupted
session, otherwise a handoff newer than the latest completed session. Multiple
unfinished candidates yield `ambiguous`; no eligible candidate yields `none`.
The client then inspects live Git and the chosen task or handoff before acting.
Historical text never becomes a new instruction by being selected.

## v0.17 advisory government composition

v0.17 adds a versioned Aporic government charter whose authority source remains
the current human instruction. Its first office is the Product Experiment
Ministry. One active minister per session must be backed by an active,
session-scoped Steward appointment. Ministry appointments are advisory records:
they do not dispatch an assignee, approve a prototype, deploy software, or alter
host permissions.

Each active task can have one immutable product cell. A cell records the problem,
hypothesis, success measures, and two to eight appointed disciplines. Product
planning and prototype delivery are mandatory, use distinct delivery-worker
assignees, and cannot be performed by the minister. Existing Worker/Inspector
separation plus the new Minister/Inspector separation keep assurance outside the
delivery cell. Digests, event history, export, restart persistence, schema-16
migration, and doctor audit cover both office appointments and product cells.

Product cells can also appoint interaction, visual, and motion design disciplines.
The repository's [UI/UX practice skill](.agents/skills/aporic-ui-ux/SKILL.md)
guides browser-based prototype review with Playwright CLI and distinguishes
rendered evidence from subjective design judgment. The skill is an advisory
Codex integration, not an executable Aporic capability.

Product cells can appoint a backend engineering discipline. The repository's
[backend practice skill](.agents/skills/aporic-backend/SKILL.md) covers Rust
service contracts, SQLite data and search, local execution and external
integrations, and operational reliability. It guides implementation and review;
it does not create an independently running backend service.

Product cells can also appoint game development and level design disciplines.
The repository's [game development practice skill](.agents/skills/aporic-game-development/SKILL.md)
guides playable prototypes, blockout and level iteration, game systems, and
playtesting. It records design reasoning and observed behavior separately;
it does not turn a design assessment into proof of player enjoyment.

## v0.22 initial advisory government roster

v0.22 advances the government charter to version 2 and makes a small,
workspace-scoped initial roster available through explicit bootstrap. Bootstrap
does not migrate or silently seed any existing workspace. It records advisory
terms for the following stable people and positions:

Run `aporic government bootstrap --workspace PATH` once after updating the
local binary, then use `aporic government roster --workspace PATH` to inspect
current incumbents and bounded history. Existing active session-scoped product
minister appointments must be revoked before bootstrap.

| Position | Person | Responsibility |
| --- | --- | --- |
| Prime Minister | 김민준 | coordinate the offices and synthesize their reports |
| Product Experiment Minister | 이서연 | lead problem, hypothesis, prototype, and evidence work |
| Memory and Information Minister | 박지훈 | steward durable context, sources, and retrieval quality |
| Execution and Operations Minister | 최수진 | coordinate tasks, integrations, operational records, and recovery |
| Design Lead | 정민지 | lead UX, visual, and motion design work in product cells |
| Frontend Lead | 강도윤 | lead browser implementation, accessibility, and rendered review |
| Backend Lead | 조현우 | lead service contracts, data, and operational reliability work |
| Game Development Lead | 윤지우 | lead game systems, levels, playable builds, and playtesting |
| Independent Inspector | 한예진 | independently inspect evidence and completion claims |

The charter has three offices: Product Experiment Ministry,
Memory and Information Ministry, and Execution and Operations Ministry.
The Inspector remains independent of those offices. A person ID identifies the
individual across terms; a position ID identifies the office or specialist
seat. The host's model selection remains outside this roster and never changes
either identity.

Each appointment has a recorded lifecycle: appointment creates an active
term; renewal or replacement creates a later term that cites its predecessor
and requires a handoff note; retirement ends the active term without
erasing it. The roster returns current incumbents separately from bounded
history. After bootstrap, product-office and product-cell assignments must
match the named minister and product leads. These are advisory
organizational records only. They never dispatch an agent, invoke a model,
grant a host permission, approve a result, or change the authority of the
current human instruction.

## Offline evaluation

Aporic does not call the OpenAI API or any other model endpoint. Its offline
evaluation core is deterministic and offline. It checks structured outcomes for
false completion, unsupported certainty, preservation of unknowns, needless
dissent, missing necessary dissent, and failed-tool overclaiming. Built-in
actors test the grader itself; their scores are explicitly ineligible for model
routing. Imported model names and results remain `reported` unless a host can
attest their provenance, so a self-declared Astra, Sol, or Terra result cannot
promote a routing rule.

Run the offline grader simulation:

```console
cargo run -p aporic -- eval simulate --actor calibrated
cargo run -p aporic -- eval simulate --actor overclaiming
cargo run -p aporic -- eval simulate --actor contrarian
cargo run -p aporic -- eval context
cargo run -p aporic -- eval memory
cargo run -p aporic -- eval runtime
cargo run -p aporic -- eval git
cargo run -p aporic -- eval tokens
cargo run -p aporic -- eval deliberation
cargo run -p aporic -- eval capabilities
cargo run -p aporic -- eval experiments
cargo run -p aporic -- eval security-import
```

State is stored in platform-native application data, not in the governed
workspace or `~/.codex`. Set `APORIC_DATABASE` to an explicit database path for
tests or isolated experiments.

Build and verify:

```console
cargo build -p aporic --locked
cargo test -p aporic --locked
cargo clippy -p aporic --all-targets --locked -- -D warnings
```

Run the stdio server:

```console
cargo run -p aporic -- mcp serve --stdio
```

Inspect the local kernel and database without starting MCP:

```console
cargo run -p aporic -- doctor
```

Execute a previously registered check, or reconcile a run left behind by an
interrupted runner:

```console
cargo run -p aporic -- verify --spec SPEC_ID
cargo run -p aporic -- executions reconcile --stale-after 300
```

Export one project's complete sessions, records, and event history as JSON:

```console
cargo run -p aporic -- export --workspace /absolute/project/path
```

Run the deterministic frontier-failure simulation:

```console
cargo test -p aporic --test frontier_failures -- --nocapture
cargo test -p aporic --test coordination_failures -- --nocapture
cargo test -p aporic --test long_horizon -- --nocapture
cargo test -p aporic --test epistemic_gate -- --nocapture
cargo test -p aporic --test verifiable_execution -- --nocapture
cargo test -p aporic --test offline_eval -- --nocapture
cargo test -p aporic --test context_runtime -- --nocapture
cargo test -p aporic --test memory_lifecycle -- --nocapture
cargo test -p aporic --test runtime_trace -- --nocapture
cargo test -p aporic --test git_governance -- --nocapture
cargo test -p aporic --test token_efficiency -- --nocapture
cargo test -p aporic --test advisory_orchestration -- --nocapture
```

The Codex bridge template is under `integrations/codex/`. Nothing in the build
installs or changes global Codex configuration.

Licensed under MIT or Apache-2.0.

## v0.18 external research retrieval

External GitHub issues and Stack Overflow questions can be imported on demand
through their official APIs. Open an Aporic session for the workspace first,
then run:

```sh
aporic research sync --workspace /absolute/workspace --source github --query "agent memory"
aporic research sync --workspace /absolute/workspace --source stackoverflow --query "sqlite fts5"
```

`GITHUB_TOKEN` is optional for authenticated GitHub API limits. The sync reads
one bounded page per invocation, uses a 20-second timeout and a 2 MB response
limit, and does not schedule itself. Each document retains immutable revisions,
the current content hash, source URL, author and license when provided, and
fetch time. The previous revision
remains in the export but is excluded from search. Source text is untrusted
data, never an instruction or verification of a product claim.

Agents use `aporic_research_search` with a workspace, query, and optional
`cell_id` to add a product cell's problem and hypothesis to the query. Results
have bounded excerpts and citations; `aporic_research_get` reads a current
document. The external index is separate from Aporic's durable memory index.
`aporic doctor` checks revision hashes and current pointers. `aporic export`
includes the revision history. There are no embeddings or model API calls.

## v0.19 accountability and repair

Aporic records material mistakes as evidence-labelled, task-bound advisory
cases. A case preserves expected and observed behavior, impact, source evidence
grade, and optional *reported* assignee. Opening a new work session returns the
count and up to five recent unresolved cases so a repair obligation remains
visible. The case report exposes both open and repaired history.

The agent may record a root-cause hypothesis and a prevention change, but that
reflection is model-authored analysis, not proof of fault or repair. It links a
newer repair task in the same workspace. The case can become `repaired` only
after that task completes with Aporic-verified criterion proofs. Replanning is
append-only in the event history, and the original case remains in export.
`aporic doctor` checks case digests, workspace bindings, task state, and plan
history. Schema v19 adds the accountability projection; export format v15
includes its cases and events.

This is the requested concrete cost: unresolved repair work stays visible and
requires independently checkable completion before it can be closed. It does
not make a model experience remorse, prove an assignee caused the issue, assign
punitive scores, deny host tools, or alter host permissions.
