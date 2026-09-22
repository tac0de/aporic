# Aporic

Aporic is an experimental local commitment-state kernel for coding agents. Version `0.7.0` closes the intent-to-effect loop with persistent intent lineage, stale-intent execution prevention, effect verification, and a bounded MCP capability surface.

Repository: [github.com/tac0de/aporic](https://github.com/tac0de/aporic)

Website: [tac0de.github.io/aporic](https://tac0de.github.io/aporic/)

## Status

`0.7.0` is the current evaluation release. Event schema v5, policy schema v2, project-binding schema v1, Codex projection schema v6, analyzer ABI v1, and intent-fidelity contract v1 are independent contracts. Policy schema v1 remains readable with `require_intent: false`. Existing v1, v2, v3, and v4 event stores are rejected until explicitly copied through `migrate`; installed plugins and live stores are not upgraded automatically. Authenticated actors, deterministic natural-language interpretation, broad host coverage, prebuilt release artifacts, and cross-platform plugin binaries are not promised. See [SECURITY.md](SECURITY.md) before relying on Aporic for consequential work.

## Build and verify

Source builds use the exact Rust 1.89.0 toolchain declared in `rust-toolchain.toml`, including `rustfmt`, Clippy, and the `wasm32-unknown-unknown` target. Rustup installs missing declared components on first use. The packaged plugin runs its bundled binary and structural analyzer and does not need `cargo` or `rustc` after it has been built.

Confirm that both toolchain proxies are available before building:

```console
command -v cargo
command -v rustc
rustc --version
```

Homebrew's keg-only `rustup` package does not place its proxy directory on `PATH`. Add the following line to `~/.zprofile`, then start a new shell:

```sh
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
```

```console
cargo build --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

## Current guarantees

- Events are replayed in a contiguous sequence.
- Commits check the expected revision while holding an exclusive file lock.
- Identical retries return the original result; reuse of a key with another payload is rejected.
- A malformed or non-newline-terminated record stops replay and further writes.
- A stored record with an unsupported schema version stops replay and further writes.
- Successful appends call `sync_data` before returning; this is not a power-loss durability guarantee.
- `status` takes a cooperative shared lock and fails if the requested store does not exist.
- Bounded execution grants are exact to plan, scope, session, tool, and canonical JSON tool input.
- Grant evaluation and consumption happen while one exclusive nonblocking store lock is held; a consumed `tool_use_id` is never allowed again.
- v1-to-v5, v2-to-v5, v3-to-v5, and v4-to-v5 migration are validated, non-destructive snapshot copies and refuse to replace the destination.
- Structured intent envelopes preserve caller-declared explicit, inferred, and unknown items without requiring raw prompt storage.
- Plans may bind one intent envelope; superseding it makes matching authorizations and grants ineligible at the action gate.
- `PostToolUse` records an idempotent effect receipt with bounded input and response identities instead of raw hook payloads, while retaining an explicit `unknown` outcome.
- Independent verifier reports bind a receipt to one plan acceptance check and same-scope direct evidence; their latest result participates in completion.
- One static capability registry drives four local MCP tools: capability discovery, count-only project status, read-only action explanation, and verifier-report ingestion.
- Observations cannot be backed only by agent inference, and verified/refuted claim transitions require recorded evidence.
- Plan completion requires every acceptance check's latest verification to pass, unless explicit accepted residual risks are named.
- Typed `supports`, `attacks`, `depends_on`, and `contradicts` relations connect same-scope active claims without asserting that the relation text is semantically true.
- Belief revisions compare an explicit prior status with current state and require direct evidence before verification or refutation.
- Decision reviews require a recorded decision basis plus direct outcome evidence. A basis may be linked later, but every referenced claim and evidence record must predate the decision; the decision and link sequences are retained for audit.
- Wasm analyzers receive a bounded graph snapshot, have no host imports, and are limited by module, fuel, stack, memory, input, and output bounds. Their findings cannot mutate the store or grant authority.

The lock is cooperative, not a security boundary. Actor provenance is recorded but is not cryptographically authenticated. Replay validates record shape, sequence, schema version, and state-application invariants; it does not rerun the current authorization policy over historical events. A preventive check is not atomically bound to the later tool effect. Parent-directory metadata is not synchronized, and automatic semantic classification, recovery commands, broad host coverage, and hostile same-user tamper resistance are not implemented.

## CLI

Direct event-log commands keep an explicit storage path. Global Codex hooks resolve their store from an explicit project binding and `${APORIC_DATA_HOME:-$HOME/.local/share/aporic}`.

```console
aporic init --store /path/to/events.jsonl
aporic project-init --workspace /absolute/repo --scope repo
aporic project-init --workspace /absolute/repo --scope repo --migrate-from-v1-store /path/to/events-v1.jsonl
aporic project-paths --workspace /absolute/repo
aporic status --store /path/to/events.jsonl
aporic commit --store /path/to/events.jsonl < request.json
aporic ingest-verifier-report --store /path/to/events.jsonl --scope repo < verifier-report.json
aporic doctor --store /path/to/events.jsonl --policy /path/to/policy.json
aporic explain --store /path/to/events.jsonl --scope repo --policy /path/to/policy.json < pre-tool-input.json
aporic migrate --store /path/to/events-v4.jsonl --from 4 --to /path/to/events-v5.jsonl
aporic analyze-wasm --store /path/to/events-v5.jsonl --scope repo --module /path/to/analyzer.wasm
aporic analyze-structural --store /path/to/events-v5.jsonl --scope repo
aporic mcp-serve
aporic codex-session-start --store /path/to/events.jsonl --scope repo --workspace /absolute/repo --policy /path/to/policy.json < hook-input.json
aporic codex-pre-tool-use --store /path/to/events.jsonl --scope repo --workspace /absolute/repo --policy /path/to/policy.json < hook-input.json
aporic codex-post-tool-use --store /path/to/events.jsonl --scope repo --workspace /absolute/repo < hook-input.json
aporic codex-session-end --store /path/to/events.jsonl --scope repo --workspace /absolute/repo < hook-input.json
```

`commit` accepts the contract in `schemas/commit-request.schema.json`. `ingest-verifier-report` accepts `schemas/aporic-verifier-report-v1.schema.json`. Runtime decoding uses strict Rust types; the JSON Schemas are the language-neutral external contracts.
Policy rejection is emitted as JSON with exit code `2`; runtime or storage failure uses exit code `1`.

JSON Schema validates the portable input shape. Runtime validation remains authoritative for semantic invariants and UTF-8 byte limits that JSON Schema cannot express exactly.

The Codex hook schemas intentionally allow additional host fields and nullable optional host metadata. The commit schema and Rust commit decoder remain strict.

`init` creates and syncs a new empty store and refuses to overwrite an existing path.

`project-init` creates `.aporic/config.json`, `.aporic/policy.json`, and an external store, refusing to overwrite any of them. By default the store is empty; `--migrate-from-v1-store` instead creates it as a validated, non-destructive v1-to-v5 snapshot while leaving the source untouched. Before migration, stop writers to the old store and confirm that its recorded scope matches the new binding; otherwise the snapshot can be stale or its retained authority can be out of scope. The config is published only after the destination store is valid. It is the opt-in marker used by the global adapter; the generated policy protects exact `apply_patch` calls and requires their plan to bind an active intent envelope. `project-paths` resolves the policy and event-log path for direct `status`, `commit`, `doctor`, `explain`, and `analyze-wasm` operations. The nearest binding above the hook `cwd` wins, so nested independently bound workspaces remain isolated. Set `APORIC_DATA_HOME` to an absolute path before setup and runtime to override the default user data root.

`codex-session-start` implements only the documented Codex `SessionStart` command-hook response. It projects exact-scope state as bounded, untrusted developer context on startup, resume, clear, and post-compaction `source: "compact"`. Startup uses a nonblocking store read: missing, invalid, or busy state is reported as `coverage: "unavailable"` rather than an empty successful capsule or a wait. A configured workspace must canonically match the hook `cwd`; other workspaces receive only `{ "continue": true }`. It does not authenticate human authority, read transcripts, infer scope from `cwd`, enforce tool calls, or provide prompt-injection immunity.

The packaged `UserPromptSubmit` hook applies intent-fidelity contract v1 to every user turn in a bound project. The contract asks the model to preserve explicit actors, targets, exclusions, negation, conditions, sequence, uncertainty, authorization boundaries, and exact technical strings; classify material meaning as explicit, inferred, or unknown; and clarify consequential ambiguity rather than guessing. Its context is deterministically limited to 1,200 UTF-8 bytes. The adapter validates project discovery but does not interpret, persist, hash, echo, or block the prompt. Raw hook input is limited to 1 MiB; an oversized payload skips the advisory without blocking the conversation. The advisory does not authenticate authority, grant permission, or replace `PreToolUse`.

Projection schema v6 reports exact tools retained within the byte budget, active structured intents, active epistemic claims and argument relations, recent belief revisions and decision reviews, current-session effect receipts and verifier reports, legacy action outcomes, a checkpoint claimed by this session, sampled gate status, and matching counts. For an observed store, when a large policy forces tool-name omission, aggregate status and authority counts for every omitted tool remain in `omitted_execution_summary`. An unavailable store has no trusted status to aggregate; it reports retained unknown-status tools plus protected and omitted counts. Details are removed deterministically when needed, with retained/omitted counts, `complete: false`, and a versioned omission receipt. The complete wrapped context is limited to 6,000 UTF-8 bytes. See `schemas/aporic-projection-v6.schema.json`.

`analyze-wasm` is read-only. It loads a schema-v1 graph snapshot at one revision and invokes a module that exports `memory`, `alloc(i32) -> i32`, and `analyze(i32, i32) -> i64`; the result packs the output pointer in the high 32 bits and byte length in the low 32 bits. Modules with imports are rejected. Defaults are 2,000,000 fuel and 8 MiB memory; callers may lower or raise them with `--fuel` and `--memory-bytes` only up to the host maxima of 50,000,000 fuel and 64 MiB. Module size, stack, table count and elements, instances, memories, input, and output are separately bounded. Output must be bounded JSON with the same `based_on_revision`; findings remain untrusted structural diagnostics. Build the included analyzer with `cargo build -p aporic-structural-analyzer --target wasm32-unknown-unknown --release`; the pinned toolchain supplies the target. Fresh Codex plugin packages include it at `analyzers/structural.wasm`, and the packaged `analyze-structural` command resolves and runs that module without a caller-supplied path.

`codex-pre-tool-use` interprets no command text or natural language. Policy schema v2 maps exact visible-ASCII host tool names of at most 256 bytes to `require_plan`, `require_grant`, and `require_intent`; unknown tools produce no hook decision. When `require_intent` is true, only an authorization or grant whose plan binds a current same-scope intent envelope can satisfy the gate. Schema-v1 policies remain compatible and behave as `require_intent: false`. A hold always denies before authority is considered. Legacy `plan_authorized` records remain unbounded session/tool approvals unless the selected tool requires intent. A bounded `execution_grant_issued` record references a plan and stores the exact JSON tool input plus a positive use limit. Its first admitted use appends `execution_grant_consumed` before returning allow; reuse, exhaustion, input/session/scope/tool mismatch, corruption, and lock contention deny or fail closed.

Grant consumption records preflight admission, not successful execution or filesystem effect. `PostToolUse` records an effect receipt with `fnv1a64` identities for canonical tool input and response, never their raw values, and fixes its outcome to `unknown`. Receipt idempotency uses exact scope, session, tool-use, and tool name; the first receipt wins and later payload identities are neither compared nor trusted. The identities are correlation aids, not integrity proofs or secret-safe hashes; low-entropy values can be guessed. `ingest-verifier-report` does not execute a verifier: it accepts a bounded report from a separately run verifier and requires an existing same-scope receipt, plan check, and at least one non-inference evidence record. The report is recorded as Evidence-authored, but verifier identity, provenance, evidence locators, and digests remain caller-declared and unauthenticated. `SessionEnd` publishes a transcript-independent checkpoint from recorded state, and the next distinct session can claim one open checkpoint exactly once. Exact input storage in bounded grants can still persist sensitive tool arguments; do not issue a grant containing secrets.

`PostToolUse` and `SessionEnd` input is capped at 1 MiB. Oversized input, invalid project state, missing or corrupt stores, and write contention return a nonzero hook command result instead of silently claiming that the lifecycle record was persisted. `SessionStart` reports checkpoint-claim failure through its unavailable projection.

`mcp-serve` implements a local newline-delimited JSON-RPC MCP server. Its `capabilities`, `project_status`, and `explain_action` tools are read-only. `ingest_verifier_report` is the only state-writing MCP tool and is configured to prompt for approval in the packaged plugin. MCP requests are capped at 1 MiB, workspaces must be absolute and resolve through an explicit Aporic project binding, status output contains counts rather than recorded text, and unknown methods or tools fail closed. The MCP surface cannot register or authorize plans, accept risk, record human approval, issue grants, or address an arbitrary store path.

`explain` is read-only and returns the same evaluation, selected authority identifier, and informational input identity used by the hook. `doctor` validates policy parsing and nonblocking log replay but never repairs, initializes, or migrates state.

Unresolved questions are disclosure, not an automatic veto; an authorizer may accept a plan that records them. Authorization checks authority and blocking Aporia at commit time. A later Aporia or delegation revocation does not retroactively remove existing authority. Reclaiming execution authority requires `plan_authorization_revoked`, `execution_grant_revoked`, grant exhaustion, or an active tool hold as appropriate. This non-retroactive rule keeps replay deterministic and avoids silently inferring dependencies that were never recorded.

For configured tools, missing, invalid, or busy state fails closed; unbound workspaces and unlisted tool names exit successfully without hook output. A malformed binding fails closed for every tool because its intended exact-tool policy cannot be trusted. The generated project policy initially protects only `apply_patch`, but the global plugin's all-tool matcher dispatches exact names through each project's policy so additional tools can be added without reinstalling the plugin. Shell semantics, hosted tools, special tool paths, already-running processes, plan quality, and same-user bypasses remain outside this gate. Session-bound authority is not reused after a new session starts.

## Codex plugin package

The package uses Codex's default `hooks/hooks.json` discovery and invokes one release binary from `PLUGIN_ROOT`. Install and trust this package once. Its append-only stores live under the Aporic user data root, which must resolve outside the bound repository. A lossless versioned encoding of each canonical project path prevents projects and nested paths from sharing a store accidentally. The `SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, and `SessionEnd` hooks activate only when the current directory or an ancestor has a valid `.aporic/config.json`; unbound projects are skipped.

On macOS arm64, build a fresh globally installable package into a new directory:

```console
./scripts/package-codex-plugin.sh /tmp/aporic-plugin
aporic project-init --workspace /absolute/repo --scope repo
```

The template under `packaging/codex-plugin` contains no personal filesystem path. The packager builds the locked release binary, copies static global hooks plus `.mcp.json`, and refuses to overwrite an existing output directory. Each repository owns its small binding and policy files; it does not receive another kernel binary or plugin installation. Validate the generated directory with Codex's plugin validator before installation.

Codex plugin installation and hook trust are host state, not repository state. Hook definitions are hash-trusted by Codex; changing the packaged command requires a new review and a new Codex task to pick up the package. The v0.7.0 package supports macOS arm64 only and is not a cross-platform distribution artifact. Moving or renaming a bound project changes its canonical storage path; migrate the old store deliberately rather than silently merging state. This release does not migrate a live store automatically. Rollback requires restoring both a compatible plugin and store snapshot because schema v5 is not readable by older binaries.

## Source standards

- JSON: RFC 8259
- JSON Schema: Draft 2020-12
- JSON Lines: UTF-8, one JSON value per line, newline terminated

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Release history and known limitations are tracked in [CHANGELOG.md](CHANGELOG.md), with the current verification record in [docs/v0.7.0-evaluation.md](docs/v0.7.0-evaluation.md).

## License

Licensed under either the [Apache License, Version 2.0](LICENSE-APACHE) or the [MIT License](LICENSE-MIT), at your option.
