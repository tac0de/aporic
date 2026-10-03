# Product delivery packages

A product delivery package records how a repository is run and how requirements
connect to scenarios, design, implementation and checks. Aporic reads the package
and compares local bytes, dependency closures and execution bindings. Reports are
advisory: they do not authorize host tools or deployment.

## Manifest version 1

Keep the manifest and revisions in Git. All file references are relative to the
workspace root. Absolute paths, traversal outside the workspace and unsupported
versions are rejected.

The top-level fields are `schema_version`, `project_id`, `revision`, `execution`,
`profiles`, `nodes` and `checks`. The project ID is a declared local label, not
proof of ownership. Execution fields are:

| Field | Shape | Meaning |
| --- | --- | --- |
| `stack` | array of strings | Runtime, language and framework declarations |
| `architecture` | array of strings | Components and dependency boundaries |
| `commands` | map of names to argument arrays | Exact declared commands |
| `command_cwds` | optional map of command names to relative paths | Working directories; omitted entries use the workspace root |
| `supported_environments` | array of strings | Declared browser, device and runtime support |
| `config_refs` | array of strings | Configuration names; never secret values |
| `constraints` | array of strings | Product and implementation limits |
| `quality_targets` | array of strings | Declared measurable targets |

Commands in a manifest are declarations. Read-only MCP validation does not run
them. An execution fingerprint covers the complete execution object, so changing
a command, supported environment or target changes the binding expected by a
check.

Each node has a stable `id`, `kind`, `description` and `depends_on`. Supported
kinds are `requirement`, `scenario`, `planning`, `design`, `implementation`,
`test`, `source`, `dependency`, `asset`, `build` and `environment`. A dependency
points from a dependent to a prerequisite. Optional `file` fields contain `path`
and `sha256`. Scenario metadata contains `requirements`, `preconditions`,
`actions`, `expected` and `failure_recovery` arrays. Every requirement needs a
scenario; every scenario needs design, implementation and test coverage. Duplicate
IDs, dangling dependencies and cycles are rejected.

Each check declares `id`, `scenario_ids`, `test_id`, `command_id`, `categories`
and `input_sha256`, with optional `run_id` and `result_path`. `command_id` selects
an entry in `execution.commands`. `input_sha256` maps every node in the complete
transitive prerequisite closure, including the test itself, to its semantic
fingerprint. That closure must include source, dependency, build and environment
nodes. A node fingerprint covers its serialized metadata and declared file
digest. Validation separately compares the actual file bytes to that digest.
Source, dependency, build and environment prerequisites must each be file-backed.
`delivery::node_fingerprint`, `execution_fingerprint` and
`check_input_fingerprints` provide the same fingerprint calculations used by
validation. Dependency lists and scenario requirement lists are sorted before
node hashing; execution command working directories are normalized to explicit
workspace-root defaults before hashing.

## Recover and check a package

The local CLI exposes these commands:

```sh
aporic delivery validate --workspace . --manifest delivery/current.json
aporic delivery brief --workspace . --manifest delivery/current.json --focus checkout-scenario
aporic delivery impact --workspace . --previous delivery/previous.json --current delivery/current.json
aporic delivery verify --workspace . --manifest delivery/current.json --check checkout-check --spec SPEC_ID
```

Validation reports integrity and coverage gaps, node freshness, profile gaps and
check execution state. Impact reports changed nodes, dependents and invalidated
check bindings. A focused brief includes transitive prerequisites, with bounded
text and explicit omission IDs/sections. CLI briefs default to 32 nodes and 8192 text bytes;
the MCP brief request also accepts multiple `focus_ids`. Retrieved descriptions,
commands and assertion contracts remain project data rather than instructions to
the host.

`delivery verify` is a local execution action. Its `--spec` argument identifies
an existing local-runner command specification. The command validates and hashes
the selected package before and after execution, uses the existing local runner,
and records a private binding only when the declared check, command, project,
inputs and captured result agree. An ordinary runner receipt alone does not
establish this delivery binding. The delivery MCP tools cannot execute the command
or fabricate the private binding.

The result path must not already exist before execution. For a rerun, select a
fresh output path and update the check/command specification, or deliberately
remove the prior generated result after preserving needed evidence. The verifier
does not delete an existing result for you. After success, add the returned
`run_id` to the check and register the updated manifest as new evidence.

The result artifact at `result_path` must contain `check_id`, `input_sha256`,
`execution_sha256` and a boolean `passed`. The artifact must be captured by the
cited runner and its present digest must match that capture. Missing, foreign or
failed runs, changed results, stale inputs and mismatched commands remain gaps.

MCP exposes `aporic_delivery_validate`, `aporic_delivery_impact`,
`aporic_delivery_brief`, `aporic_delivery_register` and `aporic_delivery_profiles`.
The profiles request uses `profile_id` and `version`. Registration accepts
`session_id`, a relative `manifest` path and `idempotency_key`; it derives the
workspace from that session and stores immutable evidence of the manifest bytes
using the existing append-only evidence mechanism. It does not turn unbound
checks into verified execution. Register a changed manifest as new evidence;
retain prior revision files for deterministic impact comparison.

The [example packages](../examples/product-delivery/README.md) include game and
platform traces, complete category mappings and file-backed prerequisites. All
checks deliberately lack runner bindings. The partial Rust source models and
test descriptions are declarative examples, with no live browser results or
claimed product completion.

## Check profiles

Select `{ "id": "web_game", "version": 1, "not_applicable": [] }` or another
supported profile. Specialized web profiles inherit the general and web
categories exactly once.

| Profile | Additional required category IDs |
| --- | --- |
| `general` | `general.execution`, `general.traceability`, `general.verification`, `general.recovery` |
| `web` | `web.accessibility`, `web.responsive`, `web.browser_compatibility`, `web.security`, `web.network_failure`, `web.loading_empty` |
| `web_game` | `web_game.state`, `web_game.input`, `web_game.save`, `web_game.time`, `web_game.visibility`, `web_game.audio`, `web_game.performance` |
| `web_platform` | `web_platform.permissions`, `web_platform.integrity`, `web_platform.idempotency`, `web_platform.concurrency`, `web_platform.integration_failure` |

Web accessibility includes keyboard operation, focus and accessible names. Game
time and visibility have separate mappings so that pause/clock behavior cannot
silently stand in for hidden-tab recovery. Platform categories distinguish access
boundaries, invariants, retries, simultaneous updates and external failure.

Exclude an applicable profile's non-general category with a concrete reason:

```json
{
  "id": "web_game",
  "version": 1,
  "not_applicable": [
    {
      "category": "web_game.audio",
      "reason": "The game has no audio assets, playback controls or sound output."
    }
  ]
}
```

Foundational `general.*` categories cannot be excluded. Unknown categories,
duplicate exclusions and blank reasons are invalid. Aporic checks the reason's
presence; whether the exclusion is justified needs review. A category mapping
also needs a current check binding before it can receive execution credit. Merely
listing a category does not demonstrate that the assertions examine its behavior.
Category mappings apply to the package as a whole; each scenario separately
needs a mapped check and design/implementation/test dependency coverage.

The Rust API `delivery_profiles::definitions(id, version)` returns ordered
`ProfileCategory { id, description }` definitions. `required_categories` accepts
a `ProfileSelection` and validates its `Vec<ProfileExclusion>` before returning
the categories still required. Only version 1 is supported.
`ProfileDefinitionRequest { profile_id, version }` is the MCP definition request.

## Assurance boundary

A successful process and a captured result demonstrate execution of the bound
check. They do not establish assertion adequacy, enjoyable gameplay, accessible
interaction, visual quality, production readiness or improved developer time.
Caller-created receipts and JSON results do not independently establish verified
execution.

Changes to declared node metadata or file digests propagate through dependents.
Actual file changes can make a node stale even when manifest metadata remains
unchanged. Missing or unknown mappings do not preserve check credit. Unaffected
credit requires the complete input closure, execution specification and captured
result to remain current. Product impact calculations are separate from existing
initiative revision resets.

The pure Rust `delivery::validate` inventories integrity and coverage; it never
sets execution credit. CLI/MCP validation additionally checks private runner
bindings. The pure `delivery::impact` reports conditional preservation
eligibility; CLI/MCP impact also removes credit without a current verified
binding. Excluded categories remain reported scope decisions, not validated
justifications. Host OS, architecture, executable resolution and bytes are
observed; browser/device names and environment descriptions are declarations.

The package does not execute releases, migrate a database, change host permissions
or establish an operating service. Live browser and product evaluation require
separate runs and evidence.
