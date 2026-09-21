# Contributing

Aporic is an experimental pre-1.0 project. Small, evidence-backed changes are preferred over broad framework expansion.

## Before changing behavior

- State the concrete failure or governance gap.
- Separate recorded evidence from inference and preference.
- Preserve explicit unknowns instead of filling them silently.
- Keep policy transitions deterministic and language-neutral at the JSON boundary.
- Do not broaden a guarantee beyond what a focused test reproduces.

## Local checks

Source builds use the exact Rust version, components, and Wasm target declared in `rust-toolchain.toml`. Homebrew `rustup` users may need to add `/opt/homebrew/opt/rustup/bin` to `PATH`; see the README setup note. Check the active toolchain before running the suite:

```console
command -v cargo
command -v rustc
rustc --version
rustup target list --installed
```

```console
cargo fmt --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build -p aporic-structural-analyzer --target wasm32-unknown-unknown --release --locked
```

Changes to event contracts must update the Rust types, JSON Schema, regression tests, README, and changelog together.

## Pull requests

Describe the problem, the smallest selected approach, relevant alternatives, focused verification, and residual risk. Do not include generated binaries, personal plugin data, local event stores, credentials, or absolute user paths.

Unless explicitly stated otherwise, contributions intentionally submitted for inclusion are licensed under the project's `MIT OR Apache-2.0` terms.
