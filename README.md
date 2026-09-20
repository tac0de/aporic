# Aporic

Aporic is an experimental local commitment-state kernel for coding agents. Version `0.2.0` adds an exact-action governance router: one deterministic evaluator drives multi-tool policy, Codex prevention, explanation, bounded execution grants, and action-specific context.

Repository: [github.com/tac0de/aporic](https://github.com/tac0de/aporic)

Website: [tac0de.github.io/aporic](https://tac0de.github.io/aporic/)

## Status

`0.2.0` is the current experimental release. Event schema v2, policy schema v1, and Codex projection schema v3 are independent contracts. Existing v1 stores are rejected until explicitly copied through `migrate`; installed plugins and live stores are not upgraded automatically. Authenticated actors, semantic command interpretation, broad host coverage, and cross-platform plugin binaries are not promised. See [SECURITY.md](SECURITY.md) before relying on Aporic for consequential work.

## Build and verify

Rust 1.89 or newer is required because Aporic uses the standard library's file-locking API.

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

The storage path is explicit because its final user-local location is not yet decided.

```console
aporic init --store /path/to/events.jsonl
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

`codex-session-start` implements only the documented Codex `SessionStart` command-hook response. It projects exact-scope state as bounded, untrusted developer context on startup, resume, clear, and post-compaction `source: "compact"`. Startup uses a nonblocking store read: missing, invalid, or busy state is reported as `coverage: "unavailable"` rather than an empty successful capsule or a wait. A configured workspace must canonically match the hook `cwd`; other workspaces receive only `{ "continue": true }`. It does not authenticate human authority, read transcripts, infer scope from `cwd`, enforce tool calls, or provide prompt-injection immunity.

Projection schema v3 reports exact tools retained within the byte budget, whether plan or bounded-grant authority is required, sampled gate status, and matching counts. For an observed store, when a large policy forces tool-name omission, aggregate status and authority counts for every omitted tool remain in `omitted_execution_summary`. An unavailable store has no trusted status to aggregate; it reports retained unknown-status tools plus protected and omitted counts. The same deterministic evaluator drives `SessionStart`, `PreToolUse`, and `explain`; holds take precedence. Details are removed deterministically when needed, with retained/omitted counts, `complete: false`, and, for observed state, a versioned selection rule and non-cryptographic informational omission identity. The complete wrapped context is limited to 6,000 UTF-8 bytes. This is byte bounding, not tokenizer-level or general Codex token optimization. See `schemas/aporic-projection-v3.schema.json`.

`codex-pre-tool-use` interprets no command text or natural language. Policy schema v1 maps exact visible-ASCII host tool names of at most 256 bytes to `require_plan` and `require_grant`; unknown tools produce no hook decision. A hold always denies before authority is considered. Legacy `plan_authorized` records remain unbounded session/tool approvals. A bounded `execution_grant_issued` record references a plan and stores the exact JSON tool input plus a positive use limit. Its first admitted use appends `execution_grant_consumed` before returning allow; reuse, exhaustion, input/session/scope/tool mismatch, corruption, and lock contention deny or fail closed.

Grant consumption records preflight admission, not successful execution or filesystem effect. Exact input storage can persist sensitive tool arguments in the event log; do not issue a grant containing secrets. The `fnv1a64` identities in explain/projection output are lookup aids only and never authorization evidence—authorization uses exact JSON equality.

`explain` is read-only and returns the same evaluation, selected authority identifier, and informational input identity used by the hook. `doctor` validates policy parsing and nonblocking log replay but never repairs, initializes, or migrates state.

Unresolved questions are disclosure, not an automatic veto; an authorizer may accept a plan that records them. Authorization checks authority and blocking Aporia at commit time. A later Aporia or delegation revocation does not retroactively remove existing authority. Reclaiming execution authority requires `plan_authorization_revoked`, `execution_grant_revoked`, grant exhaustion, or an active tool hold as appropriate. This non-retroactive rule keeps replay deterministic and avoids silently inferring dependencies that were never recorded.

For configured tools, missing, invalid, or busy state fails closed; unrelated workspaces and unlisted tool names exit successfully without hook output. The packaged default policy protects only `apply_patch`, but its all-tool hook matcher dispatches exact names through the policy so additional tools can be added without a matcher mismatch. Shell semantics, hosted tools, special tool paths, already-running processes, plan quality, and same-user bypasses remain outside this gate. Session-bound authority is not reused after a new session starts.

## Codex plugin package

The local package uses Codex's default `hooks/hooks.json` discovery and invokes the release binary from `PLUGIN_ROOT`. Its append-only store lives under `PLUGIN_DATA`, outside the repository. Generated hooks are pinned to an explicit canonical workspace and scope, so enabling the plugin globally does not inject state into other workspaces.

On macOS arm64, build a fresh installable package with `config/policy.json` into a new directory:

```console
./scripts/package-codex-plugin.sh /tmp/aporic-plugin /absolute/repo aporic
```

The template under `packaging/codex-plugin` contains no personal filesystem path. The packager builds the locked release binary, copies the strict JSON policy, renders the workspace and scope into both hooks, and refuses to overwrite an existing output directory. Validate the generated directory with Codex's plugin validator before installation.

Codex plugin installation and hook trust are host state, not repository state. Hook definitions are hash-trusted by Codex; changing the packaged command requires a new review and a new Codex task to pick up the package. The v0.2 package supports macOS arm64 only and is not a cross-platform distribution artifact. Migration creates a snapshot; before switching a live plugin to the new path, stop writes and confirm the source revision still equals the reported migrated revision. Rollback means restoring the old binary and untouched v1 store; a v2 store containing grant-consumption history cannot be downgraded by changing only the binary.

## Source standards

- JSON: RFC 8259
- JSON Schema: Draft 2020-12
- JSON Lines: UTF-8, one JSON value per line, newline terminated

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Release history and known limitations are tracked in [CHANGELOG.md](CHANGELOG.md).

## License

Licensed under either the [Apache License, Version 2.0](LICENSE-APACHE) or the [MIT License](LICENSE-MIT), at your option.
