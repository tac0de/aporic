# Contributing

Aporic is being rebuilt from first principles. Read `AGENTS.md` and
`docs/rebuild-charter.md` before making architectural changes.

Keep product logic in Rust and Bash limited to thin packaging or integration
layers. Preserve unrelated work and keep changes reversible. During development,
automatic integrations must remain advisory and fail-open.

Before submitting a change, run:

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Validate the active JSON contracts and package manifests with
`python3 -m json.tool` when they change.
