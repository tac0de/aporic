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
`aporic mcp serve --stdio` exposes the 42-tool core profile. Use
`aporic mcp serve --stdio --profile full` to expose all 56 current tools,
including optional diagnostics. Core rejects direct calls to full-only tools.
Tools marked as full-only below require `--profile full`.

Continuity:

- `aporic_open`: open an idempotent work session and receive bounded context;
- `aporic_recall`: retrieve recent durable project context;
- `aporic_resume`: select one unfinished task or fresh handoff for a terse
  new-session continuation request, or report ambiguity;
- `aporic_record`: record one typed durable fact or work-state change;
- `aporic_close`: complete or hand off a session;
- `aporic_reconcile`: abandon stale interrupted sessions without claiming success.

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
- `aporic_workflow_plan`, `aporic_workflow_advance`, and
  `aporic_workflow_status`: record iterative planning and evidence-backed
  advisory stage transitions, with explicit unresolved material inputs. See
  the [workflow gates](docs/workflow-gates.md).
- `aporic_workflow_steps` and `aporic_workflow_step_record`: inspect and record
  versioned general or UI procedure checks, concrete skips, and rework within
  those advisory stages.

The generic product-initiative core keeps a versioned plan, requirements with
stable IDs, links to the tasks that deliver them, and direct file evidence for
planning, design, integration, and release. `aporic_initiative_plan`,
`aporic_initiative_artifact_record`, `aporic_initiative_task_link`, and
`aporic_initiative_status` expose its advisory contract. Status computes missing
links, stale task credit, and verified acceptance-criterion proofs; a revision
clears prior links and artifacts. See the [workflow gates](docs/workflow-gates.md).

Design delivery in v0.21 uses a versioned repository manifest to connect
references, a selected concept, design tokens and component specifications,
implementation, and browser review. `aporic_design_validate` and
`aporic design validate --workspace PATH --manifest RELATIVE_PATH` check local
files and hashes without judging or approving the design. See the
[design delivery workflow](docs/design-delivery.md).

`aporic_related_workspaces` (full-only) can inspect Git repository names and explicit labels
under supplied local roots. Results are historical context only and never copy
or merge code.

Accountability and repair (full-only):

- `aporic_accountability_open`: record an evidence-labelled task failure as an
  advisory case; any assignee attribution remains reported, not established;
- `aporic_accountability_plan`: preserve a model-authored root-cause hypothesis
  and prevention change while linking a later repair task; revisions remain in
  the event log;
- `aporic_accountability_resolve`: mark a case repaired only when the linked
  task has completed with verified criterion proofs;
- `aporic_accountability_list`: inspect bounded open and repaired cases,
  evidence grades, and the outstanding repair count.

Rust repair pilot (core):

- `aporic_rust_repair_open`: link a matching rustc JSON diagnostic and direct
  reproduction file to an active task in the same workspace. A local runner
  `rustc` failure receipt must match the diagnostic stderr hash, source path,
  and input hash captured by the runner before execution.
  The compiler version and edition are recorded as reported context.
- `aporic_rust_repair_learn`: save a conditional rule and counterexample only
  when the task's verified completion proof is backed by a successful
  post-intake local-runner `cargo test` receipt.
- `aporic_rust_repair_search`: retrieve bounded lessons for the same workspace,
  diagnostic code, compiler version, and edition. Lessons remain advisory;
  a passing test does not establish a general Rust rule or prove causation.

Epistemic gate:

- `aporic_evidence_add`: classify provenance as direct, reported, or model-only;
- `aporic_claim_assert`: keep observed, verified, inferred, assumed, intended,
  and unknown claims distinct;
- `aporic_dissent_assess` (full-only): surface only consequential, actionable dissent with
  direct evidence.

Model routing (full-only):

- `aporic_model_route`: recommend Astra, Sol, or Luna from typed task signals.

Verifiable execution history:

- `aporic_check_register`: register an immutable argv-based local check without
  executing it;
- `aporic_run_list`: list bounded run lifecycle state;
- `aporic_run_get`: inspect one receipt, its hashes, declared artifacts, and any
  mechanically issued claim.

Runtime observation (full-only):

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

Token efficiency (full-only):

- `aporic_token_usage_record`: append a provenance-labelled usage receipt;
- `aporic_token_usage_list`: inspect bounded, append-only usage history;
- `aporic_token_efficiency_report`: separate measured counts, conservative
  estimates, cached input, and verified outcomes.

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

Historical records from retired prompt-trial, deliberation, capability-catalog,
experiment, security-summary, improvement, and prototype features remain
exportable as raw append-only events in project export format v23. They are not
part of the current MCP or CLI surface.

```console
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

## Verification execution boundary
Verification specifications now carry a versioned sandbox profile. `host`
preserves existing behavior. `required` is implemented on Linux with
`bubblewrap`, requires network denial, and chooses read-only or read-write
workspace access. Unsupported platforms, a missing backend, invalid mount setup,
host policy that forbids unprivileged user namespaces, or missing sandbox-start
evidence fail the run rather than downgrade it. Receipts persist the selected
backend and whether enforcement was established.

This is a bounded verification worker, not a general untrusted-code service.
It does not yet impose cgroup CPU, memory, or process-count quotas; use a custom
seccomp policy; provide a VM/microVM boundary; or verify remote side effects.
The existing timeout remains the resource-lifetime limit.

## Offline evaluation

Aporic does not call the OpenAI API or any other model endpoint. Its offline
evaluation core is deterministic and offline. It checks structured outcomes for
false completion, unsupported certainty, preservation of unknowns, needless
dissent, missing necessary dissent, and failed-tool overclaiming. Built-in
actors test the grader itself; their scores are explicitly ineligible for model
routing. Imported model names and results remain `reported` unless a host can
attest their provenance, so a self-declared Astra, Sol, or Luna result cannot
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

Agents use `aporic_research_search` with a workspace and query. Results have
bounded excerpts and citations; `aporic_research_get` reads a current document.
The external index is separate from Aporic's durable memory index.
`aporic doctor` checks revision hashes and current pointers. `aporic export`
includes the revision history. There are no embeddings or model API calls.

## v0.27 delegation entry contract

`aporic_open` now exposes an advisory `session_delegation` status for every
workspace session. Substantive work can record a delegation and review decision with
`aporic_session_delegation_assess` without creating a task solely to discuss
delegation. It records independent path count, material change, and a delegate
or concrete skip decision. `aporic_session_delegation_report` records actual
host-reported starts and terminal outcomes; status distinguishes missing
assessment, a decision, and reported execution. An existing task-scoped
assessment also satisfies the session's assessment diagnostic.

Aporic never launches a subagent, chooses the Codex UI model, or grants host
tools. Its core makes a missed decision visible. The Codex host still applies
its own instructions and permissions to any dispatch, and a reported start is
not an independent attestation. The Codex bridge template in
`integrations/codex/AGENTS.md` describes the host handoff.

## v0.26 core hardening

The continuity core prioritizes unresolved material unknowns, constraints, and
active decisions before lower-priority history, including when recall has more
records than its candidate limit. A new material unknown must name a stable
`subject_key`; a resolving claim must name the same subject. Direct file claims
also bind that key to the canonical file observed when evidence was added and
recheck the locator and digest when the claim is asserted. Older unkeyed claims
retain their historical behavior.

Task completion rejects a verified claim after it has been superseded. For a
direct file claim, it checks the current canonical file identity and bytes
against the recorded observation again at completion. A successful command receipt proves the registered command
ran with the recorded result; it does not prove that a test adequately covers a
natural-language requirement. Task criteria still require exact mechanical
claim statements.

`aporic doctor` separates core health from extension health while its existing
`ok` field still reports overall health. The memory audit
compares reconstructed fields and search-index content, not only row counts.
The event audit states which historical rows it can check and reports legacy or
unsupported coverage gaps; it is not a claim that every old database can be
fully replayed. Schema v26 and project export format v22 preserve the new
subject keys.

## v0.25 task-scoped research

`aporic_research_fetch` takes an existing `workspace` and `task_id`, plus a
`source` (`github` or `stackoverflow`) and query. The host calls it when the
task needs current outside evidence. It validates the task before making one
bounded official API request, then returns sync counts and matches from the
workspace corpus for the same source and query. Those matches may include
earlier imports. It does not schedule itself or decide when research is needed.

`aporic_task_research_attach` links a selected immutable `revision_id` to the
task with an idempotency key and a short `relevance_note`. A later sync cannot
change that citation. `aporic_task_research_list` returns links with URL,
content hash, observed time, and provenance.

For Reddit and LinkedIn, the host may instead attach a URL, title, and at most
1,500 bytes of excerpt through `host_observation`. These are marked
`host_reported`: Aporic does not fetch, authenticate, or independently verify
them. Use host-owned access only when available and permitted. There is no
Reddit or LinkedIn API adapter. All external text remains untrusted task data.
Schema v25 adds append-only task research items; project export format v21 and
`aporic doctor` include them.

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
