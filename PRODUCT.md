# Aporic product record

## Objective

Aporic exists to maximize one person's working efficiency. It should let
agents recover relevant context, coordinate independent work, preserve durable
decisions, and verify consequential results without requiring the human to
restate or manually orchestrate the work.

The product is a local-first agent-system hub. Codex is its first client and
connects through MCP. The behavioral kernel constrains the hub, but the kernel
is not the whole product.

## Intended outcome

A substantive Codex task can open one Aporic session, recover a bounded capsule
of relevant project state, record only durable changes, and leave a verified
handoff that a later task can resume.

Success is evidenced by less repeated explanation, fewer coordination prompts,
faster recovery across tasks, useful parallel work, and less rework. A feature
fails when the time or attention required to operate it exceeds the work it
saves.

## Current non-goals

- intercepting or authorizing host tool calls;
- storing complete conversations or every tool result;
- launching or supervising additional agents;
- remote service operation, publication, or deployment;
- a graphical interface.

## Current stage

The product is in v1 stabilization after the secure-capability and
prototype-portfolio stage. The product question is:

> Can one local MCP surface catalog many custom capabilities and compare diverse
> prototypes against common evidence without turning plugin declarations,
> security scans, or model preferences into execution authority?

The first protocol slice is complete: an actual stdio MCP client can open a
session, recall project context, record a durable item, close the session, and
recover the same state after a process restart. Current work tests whether the
hub can structurally reduce recurring frontier-model failures without requiring
manual evaluation.

## Constraints and decisions

- Rust owns product logic.
- The implementation is a modular monolith, not a service mesh.
- Runtime state lives outside both the governed project and `~/.codex`.
- `~/.codex` will eventually contain only the Codex-owned configuration and a
  minimal Aporic bootstrap instruction.
- The Codex integration is advisory and fail-open. MCP failure must be visible
  but must not silently narrow host permissions.
- Important state changes are append-only events with transactionally updated
  read projections.
- Raw conversation history is not durable product memory.
- Evidence provenance is typed as direct, reported, or model-only. A proposition
  mechanically derived from Aporic's workspace-file read-back or local runner
  receipt can become observed or verified.
- Models may register immutable command specifications and inspect outcomes, but
  cannot execute them through MCP or submit receipts. Only the local Rust runner
  can turn a successful exact-argv execution into the canonical verified claim.
- Free text, command reports, external-source reports, user statements, and
  model assessments cannot independently establish verified completion.
- Material unknowns block session completion until a direct observed or
  verified claim explicitly supersedes them.
- A newer decision or constraint can explicitly supersede one older record;
  stale records remain in history but are omitted from active recall.
- Coordination is advisory: task contracts, dependencies, write scopes, leases,
  cancellations, and criterion proofs are recorded, but Aporic does not launch
  workers or grant host authority.
- Government composition is advisory: workspace-scoped people, positions, and
  appointment terms preserve organizational continuity, but never dispatch an
  agent, invoke a model, approve work, or grant authority. The current human
  instruction remains the authority source.
- Hermes orchestration is an Aporic-native deterministic envelope, not an
  external agent framework. Steward- and Worker-style roles can submit only
  bounded advisory reports under a fixed `propose` ceiling; Aporic makes no
  model/API call and dispatches no agent.
- Blind-shadow reports must be sealed before task termination and remain absent
  from normal reads, exports, and event payloads until their prediction is
  compared with the independently recorded outcome. A role report can never
  complete the task or become its proof.
- Model routing is advisory and outside the kernel: Terra handles bounded work,
  Sol is the default for complex implementation, and Astra handles frontier or
  high-consequence ambiguous work.
- Recalled memory has an explicit origin and influence class. Model-authored
  free text remains untrusted data and cannot promote itself into authority or
  verified fact.
- A rebuildable projection classifies memory as episodic, semantic, procedural,
  gotcha, or unknown and preserves temporal validity plus supersession edges.
- Retrieval is local FTS5 with deterministic limits; v0.7 has no embeddings,
  vector database, learned memory manager, or model/API call.
- v0.18 keeps external GitHub and Stack Overflow research in a separate,
  source-labelled FTS5 corpus. Explicit CLI sync retains immutable revisions;
  agent retrieval is bounded and read-only. External text remains untrusted
  and cannot establish verified product outcomes.
- v0.25 lets a host explicitly fetch bounded official-source material for an
  existing task, then attach selected revisions as citations. Bounded Reddit
  and LinkedIn observations can be attached as host-reported source data;
  Aporic does not claim direct API access or independent verification.
- Context selection is deterministic, byte-bounded, and auditable through a
  policy digest and per-item reason codes.
- The optional Codex hook reads lifecycle events without calling a model API or
  persisting prompts, transcripts, or assistant messages. Session and turn IDs
  are recorded only as installation-keyed HMACs in exposure receipts.
  Hook failure is advisory and fail-open.
- Runtime events are append-only observations. Tool inputs and outputs are
  represented only by installation-keyed HMACs and byte counts; tool metadata
  is bounded. Matching session/turn HMACs connect tool outcomes to memory
  exposure receipts.
- Capability classes and shadow dispositions are deterministic inferences, not
  host permissions. `would_ask` and `would_deny` never affect execution, and
  hook health explicitly reports gaps and unknowns rather than claiming full
  coverage.
- The runtime trace can be exported locally in a content-free,
  OpenTelemetry-shaped format. v0.8 includes no network exporter.
- Git observations use fixed-argv, read-only local commands to create append-only
  snapshots bound to HEAD/tree/base metadata and a content digest. They never
  fetch, checkout, commit, push, open a PR, or merge.
- Locally cached tracking refs are explicitly stale-capable:
  `remote_state_fresh` remains false without an attested remote observation.
  Commit signature presence is not signer trust, a successful receipt is not
  test adequacy, and `approval_proven` remains false.
- v0.9 governance findings are deterministic advisory signals for dirty,
  detached, divergent, unresolved-base, sensitive-path, missing-receipt, and
  truncated-inventory states. They do not become an authorization kernel.
- v0.10 token counts retain their provenance. Host-reported and local-tokenizer
  counts, conservative UTF-8 byte upper bounds, and unknown counts are never
  merged into a false-precision total.
- Cached input tokens remain input tokens. Cost or latency savings from caching
  are not presented as context-token reduction.
- Usage outcomes become verified only by binding to a matching direct Aporic
  claim or execution state in the same workspace.
- Exact duplicate context content is removed only after safety-priority sorting;
  budget reports disclose duplicate, oversized, and item-limit omissions.
- v0.11 deliberations contain only concise public statements and typed
  relations; there is no field for private chain-of-thought or raw transcripts.
- Every deliberation is bound to an existing clean, committed local Git
  snapshot and becomes stale when the latest observed HEAD commit/tree differs
  or the newest snapshot is dirty.
- A new decision cannot be recorded on a stale graph; current Git evidence must
  seed a new deliberation instead of silently reusing old premises.
- Material objections, counterexamples, falsifiers, and unknowns require direct
  evidence or an observed/verified claim. Non-material dissent remains visible
  but cannot block a provisional decision.
- Decisions remain provisional, disclose the count of open material issues,
  and never establish approval, permission, merge safety, or tool authority.
- Deliberation listings return summaries, while graph detail uses fixed
  node/edge/decision bounds and reports total counts, sequence cursors, and
  truncation explicitly.

## Material unknowns

- How reliably Codex invokes the hub for substantive tasks without adding
  friction to trivial work.
- How much deterministic context selection reduces omission and restatement in
  model-driven workloads, beyond the current structural simulations.
- Whether host environments expose complete and trustworthy token counts for
  all Astra, Sol, and Terra work, including hidden reasoning and cached input.
- Whether public argument graphs improve real project decisions enough to
  justify their recording overhead beyond deterministic adversarial tests.
- Which agent runtime should back later delegation and scheduling.
- Whether the observed efficiency gain justifies an always-running local
  service beyond the current stdio deployment.
- Whether Linux namespace isolation remains reliable across the deployment
  environments that would host verification workers; current evidence is the
  supported CI image and deterministic adversarial fixtures.

## Deferred operational work

Destructive in-place restore, remote authentication,
long-running agent scheduling, provider code loading, credential brokering,
general-purpose capability execution, remote-effect verification, and
multi-tenant worker isolation are intentionally deferred. v0.13 adds validated
restore to a new destination, narrowly matched
backup retention, bounded streaming file and Git observation, canonical task
write scopes, and supply-chain/coverage release gates. Restore still refuses to
overwrite an existing database. v0.14 adds capability maturity metadata and an
opt-in, fail-closed Linux `bubblewrap` profile for local verification commands;
neither feature grants provider authority or makes catalog entries executable.
v0.15 adds only zero-effect advisory orchestration and deterministic shadow
evaluation; provider invocation, scheduling, and recommendation execution remain
deferred.

v0.22 introduces the first explicit advisory government roster. A workspace may
bootstrap the version-2 charter with three offices: Product Experiment, Memory
and Information, and Execution and Operations. It begins with a Prime Minister,
three ministers, four product-specialist leads, and an independent Inspector.
People retain stable IDs across terms, positions retain separate stable IDs, and
the host's model selection remains outside roster records. Appointments,
renewals, replacements, handoffs, and retirements are preserved as history so
later work can identify the responsible position at the time. Each term keeps
the position definition used at appointment. Once bootstrapped, product
ministry appointments and product-cell members must match active named
incumbents. Bootstrap is an explicit action; opening or migrating a workspace
never seeds personnel.

## Current evidence

Observed on 2026-09-25:

- the exact versioned kernel candidate in the repository matches the installed candidate
  by SHA-256;
- a real stdio MCP child process exposes all fifty tools and preserves a record
  across a server restart;
- exact retries are idempotent and conflicting reuse of a key is rejected;
- concurrent writers retain all tested sessions through SQLite WAL;
- all active workspace tests and the clippy suite pass;
- a deterministic frontier-failure suite improves the enforced-property score
  from the v0.1 baseline of 1/5 to 5/5 for restart memory, stale-decision
  suppression, verified completion, duplicate-work rejection, and stale-session
  reconciliation;
- a deterministic coordination suite scores 6/6 for dependency gating,
  overlapping-write prevention, criterion evidence, duplicate rejection, and
  expired-lease recovery plus cancellation without implied completion;
- a long-horizon workload preserves one active decision across 30 revisions and
  six process restarts, completes 24 leased tasks, and rejects 77 injected stale,
  duplicate, or unsupported state transitions.
- the epistemic-gate simulation rejects reported evidence as verification,
  blocks completion on a material unknown, resolves it with a direct file
  read-back, suppresses low-materiality dissent, and exercises all three model
  routes.
- the verifiable-execution simulation covers successful and failed checks,
  timeout, missing artifacts, path traversal, concurrent-run rejection, retry,
  interrupted-run reconciliation, task-proof binding, export, and event replay.
- the authority-bound-context simulation migrates v5 records with
  non-authoritative defaults, produces byte-bounded deterministic capsules,
  suppresses superseded memory, preserves unresolved unknowns and active tasks,
  labels injected instructions as untrusted data, and proves that hook prompt,
  transcript, and assistant-message fields are not persisted.
- the memory-lifecycle suite exercises deterministic FTS retrieval, time-bounded
  supersession, untrusted-by-default model text, projection/FTS consistency,
  HMAC-only exposure receipts, gotcha retention, unknown preservation, and zero
  poison authority escalations without network or model calls.
- the runtime-trace suite migrates v7 state, correlates prompt exposure with
  tool lifecycles, retains no raw prompt/tool payloads or host identifiers,
  summarizes observed capabilities, detects duplicates and lifecycle gaps,
  validates read-only trace lookup, and exports standard-length trace/span IDs.
- the fixed runtime simulation covers lifecycle gaps, duplicate delivery,
  unknown schemas, and destructive shadow classification while asserting zero
  blocks, raw payload retention, network exports, and model calls.
- the Git-governance suite migrates v8 state, proves observation leaves HEAD and
  the working tree unchanged, rejects option-shaped refs, records digest-bound
  append-only snapshots, detects governance-sensitive dirty changes, and never
  claims remote freshness or approval.
- the fixed Git simulation detects six risky states with zero Git mutations,
  approvals, network calls, or model/API calls.
- the token-efficiency suite migrates v9 state, rejects unknown-provenance token
  claims and invented verification references, keeps cached input separate,
  detects digest corruption, and exports append-only usage receipts.
- the fixed token simulation reduces its duplicate-bearing context fixture from
  206 to 107 UTF-8 bytes while retaining all three essential items and the
  unresolved unknown; it makes zero exact token claims, network calls, or model
  calls.
- the commit-bound-deliberation suite migrates v10 state, rejects unsupported
  material dissent and narrative closure of unknowns, preserves non-blocking
  low-materiality objections, requires direct evidence to undercut a material
  unknown, marks changed commit/tree decisions stale, detects digest
  corruption, and exports the complete graph.
- the fixed deliberation simulation preserves all three material challenges,
  rejects unsupported materiality, detects stale state, and emits zero
  approvals, hidden-reasoning fields, network calls, or model calls.
- the secure-capability suite migrates schema v11 to v12, rejects embedded
  credential material and exact prototype clones, exposes no generic invocation
  surface, and keeps every registered capability non-executable;
- the experiment suite requires direct hard-gate evidence, prevents a faster but
  unsafe candidate from winning, computes Pareto candidates, enforces variant
  budgets, and rejects final selection without a zero-open-issue deliberation;
- the Codex Security bridge hashes three bounded local artifacts, preserves
  partial coverage and zero findings without claiming safety, and validates a
  SQLite backup through a read-only integrity check.
- the v0.13 adversarial boundary suite rejects oversized hook input, evidence,
  receipt artifacts, and artifact lists; normalizes write scopes; disables Git
  fsmonitor in runner snapshots; and restores a validated backup into a fresh
  supported-schema database without overwriting an existing target.
- the v0.14 execution-governance suite rejects invalid host-profile restriction
  claims, rejects insufficient capability maturity and non-idempotent persistent
  routines, and requires unsupported platforms to fail closed. Linux CI also
  exercises denial of an out-of-workspace secret, read-only workspace writes,
  and network connection attempts inside the real `bubblewrap` backend.
- the v0.15 orchestration suite keeps blind reports out of reads, exports, and
  event payloads before evaluation; rejects false completion, budget overrun,
  post-outcome advice, and digest tampering; and scores predictions only against
  later task state backed by existing direct criterion proofs.

Not yet established:

- generalized performance on model-generated adversarial scenarios beyond the
  deterministic and long-horizon regression suites;
- reduced restatement or coordination cost across a model-driven synthetic
  workload;
- a real reduction in provider-reported tokens per verified success; current
  byte simulation is structural evidence only;
- better decisions or reduced rework in model-driven deliberation; the current
  graph and simulation establish structural guarantees only;
- actual agent dispatch or parallel worker execution;
- hostile multi-tenant isolation, cgroup resource quotas, custom seccomp policy,
  and VM/microVM containment.
- real-world hook coverage and outcome accuracy across host versions; the
  current report detects observable gaps but cannot prove unobserved actions.
- remote branch-protection/ruleset freshness, signer trust, independent human
  approval, and semantic test adequacy; local Git metadata cannot prove them.
- semantic adequacy of a successful test, OS-level isolation from same-user
  processes, child-process-tree containment, and trusted receipts for external
  APIs or remote effects.

The protocol slice and its live Codex bridge are operating. The broader product
hypothesis remains open until automated long-horizon simulations show that the
stored context reduces errors without creating stale-context or coordination
overhead.
