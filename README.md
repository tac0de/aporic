# Aporic

Aporic is an experimental local commitment-state kernel for coding agents. Version `0.1.0` combines a deterministic Rust core and append-only JSONL event log with a bounded Codex projection and preventive gate.

Repository: [github.com/tac0de/aporic](https://github.com/tac0de/aporic)

Website: [tac0de.github.io/aporic](https://tac0de.github.io/aporic/)

## Status

`0.1.0` is an experimental release for evaluation and dogfooding. The event schema remains v1; the independently versioned Codex projection schema is v2. Compatibility, authenticated actors, broad tool coverage, and cross-platform plugin binaries are not yet promised. See [SECURITY.md](SECURITY.md) before relying on Aporic for consequential work.

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

The lock is cooperative, not a security boundary. Actor provenance is recorded but is not cryptographically authenticated. Replay validates record shape, sequence, schema version, and state-application invariants; it does not rerun the current authorization policy over historical events. A preventive check is not atomically bound to the later filesystem effect. Parent-directory metadata is not synchronized, and automatic semantic classification, snapshots, recovery commands, migrations, broad tool coverage, and hostile same-user tamper resistance are not implemented.

## CLI

The storage path is explicit because its final user-local location is not yet decided.

```console
aporic init --store /path/to/events.jsonl
aporic status --store /path/to/events.jsonl
aporic commit --store /path/to/events.jsonl < request.json
aporic codex-session-start --store /path/to/events.jsonl --scope repo --workspace /absolute/repo --protected-tool apply_patch --require-plan < hook-input.json
aporic codex-pre-tool-use --store /path/to/events.jsonl --scope repo --workspace /absolute/repo --protected-tool apply_patch --require-plan < hook-input.json
```

`commit` accepts the contract in `schemas/commit-request.schema.json`. Runtime decoding uses strict Rust types; the JSON Schema is the language-neutral external contract.
Policy rejection is emitted as JSON with exit code `2`; runtime or storage failure uses exit code `1`.

JSON Schema validates the portable input shape. Runtime validation remains authoritative for semantic invariants and UTF-8 byte limits that JSON Schema cannot express exactly.

The Codex hook schemas intentionally allow additional host fields and nullable optional host metadata. The commit schema and Rust commit decoder remain strict.

`init` creates and syncs a new empty store and refuses to overwrite an existing path.

`codex-session-start` implements only the documented Codex `SessionStart` command-hook response. It projects exact-scope state as bounded, untrusted developer context on startup, resume, clear, and post-compaction `source: "compact"`. Startup uses a nonblocking store read: missing, invalid, or busy state is reported as `coverage: "unavailable"` rather than an empty successful capsule or a wait. A configured workspace must canonically match the hook `cwd`; other workspaces receive only `{ "continue": true }`. It does not authenticate human authority, read transcripts, infer scope from `cwd`, enforce tool calls, or provide prompt-injection immunity.

Projection schema v2 pins the configured tool, whether a plan is required, the sampled gate status, hold and matching-authorization counts, and transition kinds blocked by open Aporia. The same deterministic gate evaluator drives `SessionStart` and `PreToolUse`; a hold takes precedence over authorization. Detailed decisions, Aporia, delegations, and risks are removed deterministically when needed, with retained/omitted counts and `complete: false`. The complete wrapped context is limited to 6,000 UTF-8 bytes. This is a byte budget, not a tokenizer-level or general Codex token optimization feature. The event-log schema remains v1; see `schemas/aporic-projection-v2.schema.json` for the projection contract.

`codex-pre-tool-use` is the first preventive gate. It interprets no command text or natural language. An active exact-scope `tool_hold_placed` event always denies the configured tool; `tool_hold_released` requires a caller-declared human or evidence actor. With `--require-plan`, the tool also requires an active `plan_authorized` record bound to the exact plan scope, Codex session ID, and tool name. Plans are immutable records with an objective, acceptance checks, and explicit unresolved questions. Authorization can come from a caller-declared human or an agent with an exact active `plan_authorize` delegation; human or evidence actors can revoke it. Open Aporia that explicitly blocks `plan_authorize` prevents authorization.

Unresolved questions are disclosure, not an automatic veto; an authorizer may accept a plan that records them. Authorization checks authority and blocking Aporia at commit time. A later Aporia or delegation revocation does not retroactively remove an existing authorization. Reclaiming execution authority requires an explicit `plan_authorization_revoked` event or an active tool hold. This non-retroactive rule keeps replay deterministic and avoids silently inferring dependencies that were never recorded.

For the configured protected tool, missing, invalid, or busy state fails closed; unrelated workspaces and tool names exit successfully without hook output. The initial plugin protects only `apply_patch`. Bash, hosted tools, special tool paths, already-running processes, plan quality, and same-user bypasses remain outside this gate. Session-bound authorization is not reused after a new session starts.

## Codex plugin package

The local package uses Codex's default `hooks/hooks.json` discovery and invokes the release binary from `PLUGIN_ROOT`. Its append-only store lives under `PLUGIN_DATA`, outside the repository. Generated hooks are pinned to an explicit canonical workspace and scope, so enabling the plugin globally does not inject state into other workspaces.

On macOS arm64, build a fresh installable package into a new directory:

```console
./scripts/package-codex-plugin.sh /tmp/aporic-plugin /absolute/repo aporic
```

The template under `packaging/codex-plugin` contains no personal filesystem path. The packager builds the locked release binary, renders the workspace and scope into both hooks, and refuses to overwrite an existing output directory. Validate the generated directory with Codex's plugin validator before installation.

Codex plugin installation and hook trust are host state, not repository state. Hook definitions are hash-trusted by Codex; changing the packaged command requires a new review and a new Codex task to pick up the package. The v0.1 package supports macOS arm64 only and is not a cross-platform distribution artifact.

## Source standards

- JSON: RFC 8259
- JSON Schema: Draft 2020-12
- JSON Lines: UTF-8, one JSON value per line, newline terminated

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Release history and known limitations are tracked in [CHANGELOG.md](CHANGELOG.md).

## License

Licensed under either the [Apache License, Version 2.0](LICENSE-APACHE) or the [MIT License](LICENSE-MIT), at your option.
