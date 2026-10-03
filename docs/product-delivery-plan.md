# Product delivery v1: PRD and technical contract

## Product requirement

Aporic must let a host recover a project's execution method, trace a requirement
through scenarios/design/implementation/checks, identify stale verification after
changes, and select applicable web/game/platform checks. The human authorizes
stages 1–5. Release/operations automation and live product pilots (stage 6) are
outside this implementation. All outputs are advisory and fail-open.

The users are a project owner and host agents working across sessions. Success
means reproducible recovery and deterministic detection of missing, changed or
unbound evidence. This implementation does not claim improved developer time,
game enjoyment, design quality, or test adequacy without separate evaluation.

## Required deliverables and acceptance

1. A versioned repository-owned project execution manifest with stack,
   architecture, commands, supported environments, configuration references,
   constraints and quality targets. Commands are declarations, never executed by
   MCP; configuration references contain names, never credential values.
2. Typed stable-ID nodes and dependency edges for requirement, scenario,
   planning, design, implementation, test, source, dependency, asset, build and
   environment. Scenario nodes carry requirement IDs, preconditions, actions,
   expected outcomes and failure/recovery states. Every requirement needs a
   scenario; every scenario needs design, implementation and test coverage.
3. Read-only validation, bounded focused briefs and old/new impact comparison.
   Reject duplicates, dangling edges, cycles, unsupported versions and escaping
   paths. Hash files with bounded reads. Preserve unaffected check credit only
   when its complete input closure and execution bindings remain current.
4. Bind check records to existing local-runner run IDs and receipt artifacts,
   not submitted receipts. Match project, success, command and input snapshots;
   detect absent/foreign/failed runs and changed output. A successful process
   proves execution of that check, not the semantic adequacy of its assertions.
   No caller-created JSON is independently verified evidence.
5. Versioned general/web/web_game/web_platform profiles. List required check
   categories and explicit not-applicable reasons. Game: state/input/save/time/
   audio/performance; platform: permissions/integrity/idempotency/concurrency/
   integration failure. Test negative omission and stale cases for each profile.

## Technical contract

Implement Rust modules `delivery` and `delivery_profiles`; reuse existing store,
runner, evidence, initiative and task protocols. No database migration or host
permission changes. Repository manifests and revision files are preserved in
Git; registering a manifest uses existing immutable workspace-file evidence
and append-only evidence events. Existing initiative full revision resets stay
unchanged. Dependency-aware impact is a separate local package calculation.

Public request types: `DeliveryValidateRequest { workspace, manifest }`,
`DeliveryImpactRequest { workspace, previous, current }`, and
`DeliveryBriefRequest { workspace, manifest, focus_ids, max_nodes, max_bytes }`.
Validation returns integrity/coverage gaps, node freshness, profile gaps and
check execution states with an explicit assurance boundary. Focused briefs
include transitive prerequisites, disclose omissions, and never promote text
to instructions. Impact compares node semantic fingerprints, execution spec,
profile selection and check bindings; changes propagate to dependents. An
unknown mapping invalidates all relevant checks rather than assuming no effect.

Manifest v1 fields: `schema_version`, `project_id` (local declared label),
`revision`, `execution` (stack/architecture/commands/supported_environments/
config_refs/constraints/quality_targets), `profiles`, `nodes`, `checks`.
Node fields: `id`, `kind`, `description`, `depends_on`, optional `file`
(`path`, `sha256`), optional `scenario` (requirements/preconditions/actions/
expected/failure_recovery). Check fields: `id`, `command_id`, `scenario_ids`, `test_id`,
`categories`, `input_sha256` (map of complete transitive test prerequisites and
the test node to their semantic fingerprints), optional `run_id`, optional
`result_path`. Profile fields: `id`, `version`, `not_applicable` (category/reason).
Semantic node fingerprints hash canonical serialized node metadata and declared
file digests. The current file bytes must separately match those digests.
Execution `command_cwds` optionally specifies each command's relative working
directory; omitted entries mean the workspace root. Required source, dependency,
build and environment inputs must be file-backed. Check output must not exist
before local delivery verification: use a fresh output path for every run.

The pure Rust `delivery::validate` is an integrity/coverage inventory and never
sets execution credit. CLI/MCP use `delivery_receipts::validate`, which also
checks runner-only bindings and reports unbound executions as unresolved gaps.

Runner output at `result_path` is JSON with `check_id`, `input_sha256`,
`execution_sha256` (canonical manifest execution metadata fingerprint), and
`passed` boolean. The result must be an artifact captured by the cited runner,
with matching current digest. The complete check prerequisite closure must
include source/dependency/build/environment nodes. Missing coverage is a gap.
Profile categories are reported mappings, not proof that tests examine them.

## Validation and review

Use Rust integration tests for valid webgame/platform packages, partial
delivery, graph corruption, file staleness, selective impact, output replacement,
foreign/failed/missing runner receipts, restart and immutable evidence replay.
Exercise CLI and actual stdio MCP. Run workspace tests, format, Clippy and diff
checks; independently review assurance and invalidation semantics. Commit and
integrate locally into main. Do not push or deploy as part of this request.
