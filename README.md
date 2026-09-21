# Aporic

Aporic is an experimental local commitment-state kernel for coding agents. Version `0.4.0` adds a bounded, language-neutral intent-fidelity advisory at each Codex user turn while retaining exact-action governance.

Repository: [github.com/tac0de/aporic](https://github.com/tac0de/aporic)

Website: [tac0de.github.io/aporic](https://tac0de.github.io/aporic/)

## Status

`0.4.0` is the current development release. Event schema v2, policy schema v1, project-binding schema v1, Codex projection schema v3, and intent-fidelity contract v1 are independent contracts. Existing v1 stores are rejected until explicitly copied through `migrate`; installed plugins and live stores are not upgraded automatically. Authenticated actors, deterministic natural-language interpretation, broad host coverage, prebuilt release artifacts, and cross-platform plugin binaries are not promised. See [SECURITY.md](SECURITY.md) before relying on Aporic for consequential work.

## Build and verify

Rust 1.89 or newer is required for source builds because Aporic uses the standard library's file-locking API. The packaged plugin runs its bundled binary and does not need `cargo` or `rustc` after it has been built.

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
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
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
- v1-to-v2 migration is a validated, non-destructive snapshot copy and refuses to replace its destination.

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
aporic doctor --store /path/to/events.jsonl --policy /path/to/policy.json
aporic explain --store /path/to/events.jsonl --scope repo --policy /path/to/policy.json < pre-tool-input.json
aporic migrate --store /path/to/events-v1.jsonl --from 1 --to /path/to/events-v2.jsonl
aporic codex-session-start --store /path/to/events.jsonl --scope repo --workspace /absolute/repo --policy /path/to/policy.json < hook-input.json
aporic codex-pre-tool-use --store /path/to/events.jsonl --scope repo --workspace /absolute/repo --policy /path/to/policy.json < hook-input.json
```

`commit` accepts the contract in `schemas/commit-request.schema.json`. Runtime decoding uses strict Rust types; the JSON Schema is the language-neutral external contract.
Policy rejection is emitted as JSON with exit code `2`; runtime or storage failure uses exit code `1`.

JSON Schema validates the portable input shape. Runtime validation remains authoritative for semantic invariants and UTF-8 byte limits that JSON Schema cannot express exactly.

The Codex hook schemas intentionally allow additional host fields and nullable optional host metadata. The commit schema and Rust commit decoder remain strict.

`init` creates and syncs a new empty store and refuses to overwrite an existing path.

`project-init` creates `.aporic/config.json`, `.aporic/policy.json`, and an external store, refusing to overwrite any of them. By default the store is empty; `--migrate-from-v1-store` instead creates it as a validated, non-destructive v1-to-v2 snapshot while leaving the source untouched. Before migration, stop writers to the old store and confirm that its recorded scope matches the new binding; otherwise the snapshot can be stale or its retained authority can be out of scope. The config is published only after the destination store is valid. It is the opt-in marker used by the global adapter; the policy starts by protecting exact `apply_patch` calls that require a registered plan. `project-paths` resolves the policy and event-log path for direct `status`, `commit`, `doctor`, and `explain` operations. The nearest binding above the hook `cwd` wins, so nested independently bound workspaces remain isolated. Set `APORIC_DATA_HOME` to an absolute path before setup and runtime to override the default user data root.

`codex-session-start` implements only the documented Codex `SessionStart` command-hook response. It projects exact-scope state as bounded, untrusted developer context on startup, resume, clear, and post-compaction `source: "compact"`. Startup uses a nonblocking store read: missing, invalid, or busy state is reported as `coverage: "unavailable"` rather than an empty successful capsule or a wait. A configured workspace must canonically match the hook `cwd`; other workspaces receive only `{ "continue": true }`. It does not authenticate human authority, read transcripts, infer scope from `cwd`, enforce tool calls, or provide prompt-injection immunity.

The packaged `UserPromptSubmit` hook applies intent-fidelity contract v1 to every user turn in a bound project. The contract asks the model to preserve explicit actors, targets, exclusions, negation, conditions, sequence, uncertainty, authorization boundaries, and exact technical strings; classify material meaning as explicit, inferred, or unknown; and clarify consequential ambiguity rather than guessing. Its context is deterministically limited to 1,200 UTF-8 bytes. The adapter validates project discovery but does not interpret, persist, hash, echo, or block the prompt. Raw hook input is limited to 1 MiB; an oversized payload skips the advisory without blocking the conversation. The advisory does not authenticate authority, grant permission, or replace `PreToolUse`.

Projection schema v3 reports exact tools retained within the byte budget, whether plan or bounded-grant authority is required, sampled gate status, and matching counts. For an observed store, when a large policy forces tool-name omission, aggregate status and authority counts for every omitted tool remain in `omitted_execution_summary`. An unavailable store has no trusted status to aggregate; it reports retained unknown-status tools plus protected and omitted counts. The same deterministic evaluator drives `SessionStart`, `PreToolUse`, and `explain`; holds take precedence. Details are removed deterministically when needed, with retained/omitted counts, `complete: false`, and, for observed state, a versioned selection rule and non-cryptographic informational omission identity. The complete wrapped context is limited to 6,000 UTF-8 bytes. This is byte bounding, not tokenizer-level or general Codex token optimization. See `schemas/aporic-projection-v3.schema.json`.

`codex-pre-tool-use` interprets no command text or natural language. Policy schema v1 maps exact visible-ASCII host tool names of at most 256 bytes to `require_plan` and `require_grant`; unknown tools produce no hook decision. A hold always denies before authority is considered. Legacy `plan_authorized` records remain unbounded session/tool approvals. A bounded `execution_grant_issued` record references a plan and stores the exact JSON tool input plus a positive use limit. Its first admitted use appends `execution_grant_consumed` before returning allow; reuse, exhaustion, input/session/scope/tool mismatch, corruption, and lock contention deny or fail closed.

Grant consumption records preflight admission, not successful execution or filesystem effect. Exact input storage can persist sensitive tool arguments in the event log; do not issue a grant containing secrets. The `fnv1a64` identities in explain/projection output are lookup aids only and never authorization evidence—authorization uses exact JSON equality.

`explain` is read-only and returns the same evaluation, selected authority identifier, and informational input identity used by the hook. `doctor` validates policy parsing and nonblocking log replay but never repairs, initializes, or migrates state.

Unresolved questions are disclosure, not an automatic veto; an authorizer may accept a plan that records them. Authorization checks authority and blocking Aporia at commit time. A later Aporia or delegation revocation does not retroactively remove existing authority. Reclaiming execution authority requires `plan_authorization_revoked`, `execution_grant_revoked`, grant exhaustion, or an active tool hold as appropriate. This non-retroactive rule keeps replay deterministic and avoids silently inferring dependencies that were never recorded.

For configured tools, missing, invalid, or busy state fails closed; unbound workspaces and unlisted tool names exit successfully without hook output. A malformed binding fails closed for every tool because its intended exact-tool policy cannot be trusted. The generated project policy initially protects only `apply_patch`, but the global plugin's all-tool matcher dispatches exact names through each project's policy so additional tools can be added without reinstalling the plugin. Shell semantics, hosted tools, special tool paths, already-running processes, plan quality, and same-user bypasses remain outside this gate. Session-bound authority is not reused after a new session starts.

## Codex plugin package

The package uses Codex's default `hooks/hooks.json` discovery and invokes one release binary from `PLUGIN_ROOT`. Install and trust this package once. Its append-only stores live under the Aporic user data root, which must resolve outside the bound repository. A lossless versioned encoding of each canonical project path prevents projects and nested paths from sharing a store accidentally. The `SessionStart`, `UserPromptSubmit`, and `PreToolUse` hooks activate only when the current directory or an ancestor has a valid `.aporic/config.json`; unbound projects are skipped.

On macOS arm64, build a fresh globally installable package into a new directory:

```console
./scripts/package-codex-plugin.sh /tmp/aporic-plugin
aporic project-init --workspace /absolute/repo --scope repo
```

The template under `packaging/codex-plugin` contains no personal filesystem path. The packager builds the locked release binary, copies static global hooks, and refuses to overwrite an existing output directory. Each repository owns its small binding and policy files; it does not receive another kernel binary or plugin installation. Validate the generated directory with Codex's plugin validator before installation.

Codex plugin installation and hook trust are host state, not repository state. Hook definitions are hash-trusted by Codex; changing the packaged command requires a new review and a new Codex task to pick up the package. The v0.4 package supports macOS arm64 only and is not a cross-platform distribution artifact. Moving or renaming a bound project changes its canonical storage path; migrate the old store deliberately rather than silently merging state. This release does not migrate the event store. Rollback means restoring the v0.3.1 plugin package; the v2 store remains compatible. The global adapter does not discover or migrate old plugin stores automatically.

## Source standards

- JSON: RFC 8259
- JSON Schema: Draft 2020-12
- JSON Lines: UTF-8, one JSON value per line, newline terminated

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Release history and known limitations are tracked in [CHANGELOG.md](CHANGELOG.md), with the current verification record in [docs/v0.4.0-evaluation.md](docs/v0.4.0-evaluation.md).

## License

Licensed under either the [Apache License, Version 2.0](LICENSE-APACHE) or the [MIT License](LICENSE-MIT), at your option.
