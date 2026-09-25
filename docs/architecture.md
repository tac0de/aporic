# Aporic hub architecture

## Boundary

The Aporic kernel and Aporic Hub have different lifecycles.

- The kernel is an exact behavioral constitution and a set of deterministic
  invariants. It has no transport, database, model, or host dependency.
- The hub is evolving software that applies those invariants to projects,
  sessions, tasks, decisions, typed evidence, epistemic claims, and agents.
- MCP is an adapter. It does not become the domain model and does not by itself
  form a security boundary around other host tools.

## Implementation

The implementation is one Rust package with internal `kernel`, `domain`,
`context`, `government`, `store`, `hub`, `runner`, `git`, `git_process`,
`bounded`, `recovery`, `hook`, and `mcp` modules. Package
boundaries will be introduced only when an independently versioned contract or
deployment unit exists.

```text
Codex --stdio MCP--> mcp adapter --> hub services --> SQLite
                                      |
                                      +--> kernel invariants

Codex lifecycle --JSON stdin--> fail-open hook --> bounded context + exposure receipt
                                             \--> hashed runtime event + projections

local CLI --> runner --> exact argv process --> hashed receipt --> SQLite

local Git --fixed read-only argv--> git observer --> governance snapshot --> SQLite

host/local counts --> provenance gate --> token usage receipt --> efficiency report

Git snapshot + typed evidence --> public argument graph --> provisional decision
                                      \--> open aporia + staleness report

human intent --> versioned government charter --> office appointment
                                                \--> task-bound product cell
                                                     \--> prototype evidence

independent Inspector appointment ------------------> assurance report
```

Each Codex connection runs an inexpensive child process. Processes share one
SQLite database in WAL mode. Each command opens a short connection and commits
an event and its read projection in one immediate transaction.

## Durable model

The hub stores projects, sessions, durable records, advisory tasks, typed
evidence artifacts, epistemic claims, and events. Legacy records retain their
explicit operational kind. New effect and verification assertions go through an
epistemic gate:

```text
source -> evidence grade -> claim status -> criterion proof -> completion
           direct            verified       exact match
           reported          inferred       cannot complete
           model_only        inferred       cannot complete
```

Workspace files read and hashed by Aporic are direct. The local runner also
creates direct execution receipts for immutable, pre-registered argv-based
command specifications. It stores executable resolution, termination and exit
status, stdout/stderr hashes and lengths, Git/worktree snapshots, and hashes for
declared artifacts. Raw command output is not durable state. Successful receipts
create one canonical verified claim; other terminal states do not.

The MCP boundary can register a specification and read run state, but cannot
execute it or submit a receipt. The local CLI owns execution. A host-profile run
is an integrity boundary inside the application, not an OS sandbox: a process
running as the same user can still alter the database or workspace. A required
Linux profile enters a `bubblewrap` namespace/mount boundary and must attest
backend readiness before its result can become verified. A successful receipt
still proves only the recorded process result and enforcement evidence, not test
quality or remote effects.

The event log preserves accepted state transitions and idempotent results. Read
tables provide bounded context without replaying the whole log on every tool
call. Tests replay events independently and compare the result with projections.

Schema v7 adds a rebuildable `memory_items` projection, explicit relation edges,
and a synchronized local FTS5 index. Active retrieval applies lifecycle and
valid-time filters before deterministic class/rank/ID ordering. The source rows
remain authoritative; the projection adds no permissions. Exposure receipts
store selected IDs, byte counts, the policy digest, and installation-keyed HMACs
for host session/turn correlation—never prompt or transcript contents.

Schema v8 adds append-only `runtime_events` and a trigger-maintained
`capability_observations` projection. Runtime rows keep bounded host/tool
metadata, inferred capability and outcome classes, latency, byte counts, and
installation-keyed HMACs for correlation and payload equality. They do not keep
raw prompt, input, output, transcript, or assistant content. Tool events link to
the latest exposure with identical project/session/turn HMACs. This is an
observability relation, not evidence that recalled memory caused an outcome.

Schema v9 adds append-only `git_snapshots`. Each row binds observed local Git
metadata and deterministic governance findings to a SHA-256 digest. Changed
paths are retained, but patches, blobs, commit messages, remote URLs, and signing
keys are not. Snapshot digests detect accidental projection corruption; because
the database remains writable by the same user, they are not a tamper-proof
security boundary.

Schema v10 adds append-only `token_usage_receipts`. Each receipt keeps the
counting source, input/output/reasoning counts, cached-input subset, context
bytes, outcome status, verification reference, and a canonical digest. A
verified outcome must resolve to matching Aporic-direct state in the same
workspace. Reports aggregate measured counts separately from conservative byte
upper bounds and unknowns. They expose measured tokens per verified success
only when a measured verified denominator exists, and mark the overall report
incomplete when estimates, unknown provenance, or unverified outcomes remain.

Schema v11 adds append-only `deliberations`, `deliberation_nodes`,
`deliberation_edges`, and `deliberation_decisions`. The root question is bound
to an existing clean, committed v0.9 Git snapshot and its HEAD commit/tree;
dirty and unborn snapshots are rejected. Nodes use a closed
public vocabulary, and edges make support, attack, undercut, dependency,
falsification, and revision explicit. Material challenges require direct typed
evidence or an observed/verified claim. A material objection, counterexample,
or falsifier stays open until a later undercut or revision explicitly targets
it. A material unknown rejects narrative revision and closes only through a
directly evidenced undercut. Decisions store the open-material count at
decision time and remain provisional. Reads compare the bound commit/tree with the newest observed Git
snapshot and report staleness without mutating history. Canonical digests make
projection corruption visible but do not create a same-user security boundary.
Decision writes reject stale graphs, including a newest dirty snapshot, so old
repository premises cannot silently produce a fresh-looking decision.
List reads return summaries rather than graph bodies. Detail and mutation
responses cap nodes, edges, and decisions at fixed limits while reporting total
counts, truncation, and continuation sequences, preventing old argument history
from becoming an unbounded or inaccessible context payload.

Schema v12 adds immutable `capability_manifests` with append-only catalog-state
events, plus commit-bound experiment campaigns, criteria, variants,
measurements, and decisions. Manifests are bounded catalog data and always
report `executable: false`; the MCP surface has no generic invocation tool.
Hard-gate qualification requires direct evidence or an observed/verified claim,
while Pareto comparison cannot override a failed gate. Imported
`security_assessments` bind locally hashed Codex Security manifest, findings,
and coverage artifacts to one clean Git snapshot. They prove observed bytes and
normalized coverage/counts, never safety or approval.

v0.13 deliberately leaves the database at schema v12 because stabilization adds
no tables. Runtime boundaries now share streaming file hashing, bounded Git
subprocess capture, lower-case portable-ASCII advisory write scopes, and fresh-destination
restore. Backup retention remains an explicit CLI operation over a strict
filename pattern.

Schema v13 adds capability maturity, versioned capability-manifest digests,
verification sandbox profiles, and receipt enforcement evidence. Existing
capabilities receive the minimum maturity implied by their effect class and keep
their v1 digest contract; new manifests use the v2 digest. Existing checks retain
the `host` profile. New checks can require Linux `bubblewrap`, network denial,
and read-only or read-write workspace access. Required execution fails closed on
unsupported platforms or backend/setup failure and cannot issue a verified claim
without the in-sandbox readiness marker.

Schema v14 adds immutable `orchestration_runs`, one-to-one
`advisory_role_reports`, and one-to-one `shadow_evaluations`. Each Hermes run
binds an advisory role to an active task, clean Git commit/tree, context-policy
digest, and fixed resource budget. The only capability ceiling is `propose`;
there is no provider invocation or agent-dispatch path. Blind-shadow report
content is sealed before the task outcome and omitted from reads, exports, and
event payloads until evaluation. Evaluation uses the task projection and its
existing Aporic-direct criterion proofs, records Git staleness, and never turns
model output into completion evidence.

Schema v15 adds bounded `role_appointments` for four versioned advisory duty
contracts. Each appointment is scoped to a session or active task, preserves
optional model and existing capability-catalog references, and has an immutable
creation digest with a separately recorded revocation. Worker and Inspector
assignees are separated for the same task. An appointment does not invoke the
assignee, load plugin code, or grant host access. Schema v16 lets new Hermes
runs cite an appointment matching their session, task, role, and model hint;
legacy runs retain their original digest. `aporic_resume` is a read-only
selector over unfinished tasks, open sessions, and fresh handoffs; it reports
ambiguity and does not execute the selected next action.

Schema v17 adds immutable `office_appointments` and `product_cells`. The first
versioned office is `product.experiment`, headed by one active session-scoped
Steward appointment. A task may bind one product cell with an explicit problem,
hypothesis, bounded success measures, and two to eight discipline assignments.
Planning and prototype delivery are mandatory and must use distinct active
Worker assignees for the same task. The minister cannot be a cell member or an
Inspector; Worker/Inspector separation continues to apply. These records are
advisory organization and accountability data, not agent dispatch, execution,
approval, deployment, or host authority.

Schema v18 adds source-scoped external research documents and append-only
revisions with a separate FTS5 projection. Official API sync is explicit and
local; MCP exposes only workspace-scoped search and current-document reads.
An optional product cell supplies query context. Search results carry source
URLs, fetch times, hashes, and an untrusted-content notice. The corpus never
enters verified claims or authorization state.

Schema v19 adds task-bound accountability cases as advisory repair obligations.
The opening record is digest-bound and source evidence retains its grade; any
assignee attribution remains reported. Plan revisions preserve model-authored
causal hypotheses and prevention proposals in the append-only event stream.
The latest plan links a later task in the same workspace. Resolution requires
that task's existing verified criterion proofs, while the original failure and
all plan revisions remain readable. Open cases are summarized on `aporic_open`
and do not narrow host permissions or change authorization decisions.

## MCP surface

- `aporic_open`: start an idempotent session and return recent context.
- `aporic_recall`: retrieve a bounded project capsule.
- `aporic_resume`: identify one unambiguous unfinished task or fresh handoff
  for a new session, without treating prior text as authority.
- `aporic_roles_list`, `aporic_role_appoint`, `aporic_role_revoke`, and
  `aporic_role_appointments`: inspect duty contracts and record advisory
  assignments with optional catalog capability references.
- `aporic_government_get`, `aporic_office_appoint`, `aporic_office_revoke`,
  `aporic_office_appointments`, `aporic_product_cell_create`, and
  `aporic_product_cell_list`: inspect the government charter and record the
  Product Experiment Ministry's accountable, multidisciplinary advisory cells.
- `aporic_memory_search` and `aporic_memory_get`: inspect deterministic,
  workspace-scoped memory without granting write or execution authority.
- `aporic_record`: append one durable typed record.
- `aporic_evidence_add`, `aporic_claim_assert`, and `aporic_dissent_assess`:
  enforce the evidence, certainty, unknown, and counterargument gates.
- `aporic_model_route`: return an advisory Astra/Sol/Terra route from typed task
  signals.
- `aporic_check_register`, `aporic_run_list`, and `aporic_run_get`: register
  immutable checks and inspect verifiable execution history without exposing an
  MCP execution capability.
- `aporic_close`: complete or hand off a session.
- `aporic_reconcile`: mark inactive sessions abandoned without converting them
  into completed work.
- `aporic_task_create`, `aporic_task_list`, `aporic_task_claim`,
  `aporic_task_complete`, and `aporic_task_cancel`: maintain advisory task
  contracts, dependency gates, non-overlapping write leases, and
  criterion-by-criterion verified proofs.
- `aporic_accountability_open`, `aporic_accountability_plan`,
  `aporic_accountability_resolve`, and `aporic_accountability_list`: preserve
  evidence-labelled failure reports and repair obligations without changing
  host permissions or treating model reflection as verification.
- `aporic_trace_list`, `aporic_trace_get`, `aporic_capability_report`, and
  `aporic_hook_health`: inspect runtime observations, inferred capability
  projections, and observable hook gaps without exposing a control surface.
- `aporic_git_observe`, `aporic_git_snapshot_list`, and
  `aporic_git_snapshot_get`: capture and inspect local commit-bound Git evidence
  without fetch, checkout, commit, push, PR, review, or merge capabilities.
- `aporic_token_usage_record`, `aporic_token_usage_list`, and
  `aporic_token_efficiency_report`: append and inspect provenance-labelled usage
  without calling a model API or converting estimates into exact counts.
- `aporic_deliberation_create`, `aporic_deliberation_node_add`,
  `aporic_deliberation_decide`, `aporic_deliberation_get`, and
  `aporic_deliberation_list`: maintain and inspect public commit-bound argument
  graphs without storing hidden reasoning, granting approval, or gating tools.
- `aporic_capability_register`, `aporic_capability_search`, and
  `aporic_capability_get`: maintain a progressively disclosed catalog without
  loading provider code or granting execution authority.
- `aporic_experiment_create`, `aporic_experiment_variant_add`,
  `aporic_experiment_measurement_add`, `aporic_experiment_decide`,
  `aporic_experiment_get`, and `aporic_experiment_list`: maintain an advisory,
  evidence-gated prototype tournament bound to immutable Git evidence.
- `aporic_security_assessment_get` and `aporic_security_assessment_list`:
  inspect imported security-artifact summaries; scanner execution remains a
  host-owned operation outside MCP.
- `aporic_orchestration_run_create`, `aporic_role_report_submit`,
  `aporic_shadow_evaluate`, `aporic_orchestration_run_get`, and
  `aporic_orchestration_run_list`: maintain commit/context/task-bound advisory
  shadow runs while keeping blind reports sealed until deterministic outcome
  evaluation. They do not invoke models, dispatch agents, execute tools, or
  grant authority.

Recall excludes records and claims superseded by newer state. Its selector
combines unresolved material unknowns, active constraints and decisions, active
tasks, verified/observed claims, handoffs, and other recent records under a
fixed item and UTF-8 content-byte budget. Objective and focus-path token overlap
only rank items within the higher-level safety priority; recency and stable IDs
make ties deterministic. Each item exposes its origin, influence class, and
selection reasons. Historical effect/verification links remain readable, but
new writes use the typed gate.

After ranking, identical content is selected once and lower-priority duplicates
are omitted. Context budgets disclose candidate, duplicate, oversized, and
item-limit counts. The conservative input-token upper bound equals selected
UTF-8 bytes; its source label prevents consumers from mistaking it for a
provider tokenizer count.

The Codex hook adapter appends no raw hook payload. Opening the local store may
still initialize or migrate its schema. For session and prompt events, it uses a
submitted prompt only as an in-memory relevance query, emits model-visible text
inside JSON data records under a data-not-instructions header, and records a
privacy-preserving exposure receipt. For all recognized lifecycle events it
records privacy-minimized runtime metadata and hashes. Tool and permission hooks
always return an empty JSON object. Errors also return `{}`. The adapter does not
block tools, request permissions, invoke models, or modify Codex configuration.

Shadow disposition is a deterministic counterfactual classification. Even
`would_deny` is a recorded observation and has no enforcement path. Hook health
compares pre/terminal tool pairs, duplicate fingerprints, and unknown event,
capability, or schema values. It can reveal evidence of incomplete observation;
it cannot prove completeness, so `coverage_proven` is always false. The CLI can emit local OpenTelemetry-shaped JSON
with 128-bit trace IDs and 64-bit span IDs, but does not transmit it.

Tool failures are returned as visible tool-level errors. Protocol errors are
reserved for malformed MCP requests that cannot be routed or decoded.

## Automated evaluation

`frontier_failures` is a deterministic simulation suite for five recurrent
model failures: context loss after restart, stale decision reuse, unsupported
completion, duplicate work, and orphaned sessions. It is a regression contract,
not evidence that every natural-language model behavior is solved.

`coordination_failures` injects dependency violations, overlapping write scopes,
unsupported completion, duplicate task objectives, and expired leases.
`long_horizon` repeats decision revision, restart, lease, and completion cycles
to detect accumulating stale state.

`epistemic_gate` injects unsupported certainty, unresolved unknowns, irrelevant
dissent, and model-routing boundary cases.

`verifiable_execution` simulates success, non-zero exit, timeout, missing
artifacts, path traversal, duplicate registration, concurrent execution, retry,
task proof binding, interrupted-run reconciliation, export, and event replay.

`context_runtime` simulates migration, deterministic budget enforcement,
supersession, unknown/task retention, memory-injection labeling, fail-open hook
behavior, and non-persistence of sensitive lifecycle fields.

`memory_lifecycle` exercises FTS retrieval, explicit temporal supersession,
untrusted-by-default text, bounded Hook output, HMAC-only exposure correlation,
and projection/FTS consistency. `eval memory` adds an offline fixed-trace check
for stale exclusion, gotcha retention, unknown preservation, determinism, and
zero poison authority escalation.

`runtime_trace` exercises v7 migration, HMAC correlation, memory-exposure
association, payload non-retention, capability projection, shadow-policy
non-enforcement, gap/duplicate/schema detection, trace lookup, and content-free
export. `eval runtime` adds a fixed offline adversarial trace and asserts no
blocking, raw payload retention, network export, or model/API invocation.

`git_governance` exercises v8 migration, immutable observation, ref validation,
sensitive-path detection, append-only lookup, and digest consistency against
real temporary repositories. `eval git` fixes six adversarial governance states
and asserts zero Git mutation, approval, network, or model/API calls.

`token_efficiency` exercises v9 migration, provenance validation, idempotent
append-only receipts, direct verification binding, cache/input separation,
digest corruption detection, export, deterministic context deduplication, and
unknown retention. `eval tokens` compares a duplicate-bearing byte baseline
with the bounded selector and explicitly emits no exact token claim from byte
estimates.

`commit_bound_deliberation` exercises v10 migration, idempotent graph creation,
direct-evidence requirements for material dissent, non-blocking irrelevant
objections, explicit revision, provisional decisions with retained aporia,
Git-state staleness, export, and post-write digest corruption detection. `eval
deliberation` fixes adversarial policy cases and asserts zero approvals, hidden
reasoning fields, network calls, or model/API calls.

`secure_capabilities` exercises v11 migration, manifest bounds,
non-executability, direct-evidence hard gates, exact-clone and budget rejection,
Pareto selection, Codex Security artifact import, partial-coverage honesty, and
backup integrity. `eval capabilities`, `eval experiments`, and `eval
security-import` remain fixed offline contracts with zero provider, network,
scanner, or model calls.

## Later growth

Actual agent dispatch, event-driven waiting, remote transports, remote-effect
verification, stronger cgroup/seccomp/VM isolation, PR automation,
protected-ref enforcement, and merge queues are later layers. The current task,
lease, Git snapshot, and finding records are advisory state only and do not make
the continuity loop depend on a worker runtime or grant repository authority.
