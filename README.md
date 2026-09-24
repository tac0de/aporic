# Aporic

Aporic is a clean-room Rust workspace exploring durable continuity and
deterministic records for coding-agent work.

The current implementation is developmental. It does not install host hooks,
ship a plugin, deny tools, add approval prompts, or control model selection.
Governance may be introduced only after an operating service exists, one
explicit and observable control at a time. See the
[rebuild charter](docs/rebuild-charter.md).

## Shape

Aporic is organized around typed Rust logic and versioned content packages:

- `crates/aporic-kernel`: append-only exact-action record experiment
- `crates/aporic-boundary`: isolated role/boundary admission experiment
- `crates/aporic-projects`: immutable external project bindings
- `crates/aporic-roles`: validates `role.md + role.json` packages
- `crates/aporic-routing`: deterministic routing recommendations
- `crates/aporic-handoff`: session/day continuity records
- `crates/aporic-host`: typed composition boundary
- `crates/aporic-cli`: thin JSON-to-Rust command adapter
- `roles/`: human-readable instructions paired with machine-readable manifests
- `schemas/`: active JSON contracts

Markdown remains human/model-facing content. JSON carries bounded identifiers,
versions, capabilities, and limits. The role loader validates and hashes both
before producing a typed profile; the deterministic kernel does not interpret
Markdown.

Authorization APIs remain available only for explicit experiments. Nothing in
this repository automatically connects them to a model host or tool runtime.

## Build and verify

The exact Rust toolchain is declared in `rust-toolchain.toml`.

```console
cargo build --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Build the optional local CLI:

```console
cargo build -p aporic-cli --bin aporicctl
```

Its connection and command formats are documented in
[A0 minimal CLI](docs/a0-cli.md). Building does not install, publish, or connect
Aporic to another application.

## Historical material

The pre-rebuild implementation is not part of this workspace. Historical
snapshots under `archive/` and the Git archive ref described in
[Legacy implementation archive](docs/archive.md) are evidence only and are not
built, tested, installed, or treated as current design.

Licensed under MIT or Apache-2.0.
