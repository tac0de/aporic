# Contributing

Aporic is an evolving local-first Rust hub. Read `AGENTS.md`, `PRODUCT.md`, and
`docs/architecture.md` before making architectural changes.

Keep product logic in Rust and Bash limited to thin packaging or integration
layers. Preserve unrelated work and keep changes reversible. During development,
automatic integrations must remain advisory and fail-open.

Before submitting a change, run:

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Changes to migrations must cover both new databases and upgrades from every
schema version still accepted by the release binary.
