# Changelog

All notable changes are documented here. Aporic is experimental; its contracts may change between minor releases.

## 0.5.0 - 2026-09-21

### Added

- Event schema v3 for evidence, epistemic claims, plan bases, action outcomes, acceptance-check verification, plan completion, and continuation checkpoints.
- `PostToolUse` occurrence recording and transcript-independent `SessionEnd` checkpoint publication.
- Projection schema v4 with active claims, current-session outcomes, and a single claimed continuation checkpoint.
- Explicit non-destructive v1-to-v3 and v2-to-v3 migration paths.

### Changed

- Completion now requires passing verification for every acceptance check or named accepted residual risks.
- The plugin package and crate report `0.5.0`.

### Known limitations

- `PostToolUse` records `unknown`, not success, because tool responses are not a uniform attestation format.
- Checkpoints derive only from Aporic recorded state and do not read Codex transcripts.
- Actors and evidence locators remain caller-declared rather than cryptographically authenticated.

## 0.4.0 - 2026-09-21

### Added

- A bounded `UserPromptSubmit` adapter for language-neutral intent-fidelity guidance in explicitly bound projects.
- Intent-fidelity contract v1 covering explicit, inferred, and unknown meaning; actor, target, exclusion, negation, condition, sequence, uncertainty, authorization, and exact-string preservation.
- A strict but host-extensible `UserPromptSubmit` input schema and focused opt-in, prompt-non-disclosure, invalid-binding, and packaging tests.

### Changed

- The global plugin now combines turn-scoped advisory guidance with the existing `SessionStart` projection and independent `PreToolUse` gate.
- Crate and Codex plugin package versions now report `0.4.0`; event, policy, project-binding, and projection schema versions are unchanged.

### Known limitations

- The Rust adapter does not interpret natural language or verify that the model followed the advisory.
- Intent is not persisted or linked to plans in this release, and inferred meaning never constitutes authority.
- A changed hook definition requires host trust review and a new Codex task after installation.

## 0.3.1 - 2026-09-20

### Changed

- Source-build documentation now distinguishes build-time Rust requirements from the packaged plugin runtime.
- The macOS arm64 packager now fails before creating output when `cargo` or `rustc` is unavailable and provides an actionable Homebrew `rustup` PATH diagnostic when detected.
- Crate and Codex plugin package versions now report `0.3.1`; event, policy, project-binding, and projection schema versions are unchanged.

### Known limitations

- Public prebuilt artifacts are not provided; creating a plugin package still requires a local Rust 1.89-or-newer toolchain.
- The packaged host remains macOS arm64 only.

## 0.3.0 - 2026-09-20

### Added

- One globally installed Codex adapter that discovers explicit `.aporic/config.json` project bindings.
- `project-init` for non-overwriting project binding, exact-tool policy creation, and optional non-destructive v1-store migration.
- Canonical workspace-path isolation for project stores under a configurable user data root.

### Changed

- Plugin packages are workspace-independent and no longer embed a repository path, scope, or policy.
- `project-init` initializes the empty store for a new binding; hooks never recreate missing state, and protected pre-tool checks fail closed when state cannot be verified.

### Known limitations

- Moving or renaming a repository changes its resolved store path and requires explicit state migration.
- Binding and policy files are controlled by the same operating-system user and are not a hostile-user security boundary.

## 0.2.0 - 2026-09-20

### Added

- Strict JSON multi-tool policy schema with exact-name dispatch.
- Shared action evaluator for startup projection, preventive checks, and read-only `explain` output.
- Bounded execution grants with exact JSON input binding and atomic preflight consumption.
- Structured read-only `doctor` diagnostics.
- Explicit non-destructive v1-to-v2 event-log migration.
- Projection schema v3 with multi-tool status and deterministic omission receipt.

### Changed

- Event schema is now v2; old stores require explicit migration.
- The packaged hook observes all tool names and lets the exact policy decide whether a tool is protected.

### Known limitations

- Grant consumption records admission, not successful tool execution.
- Exact bound input is persisted and may contain sensitive values.
- FNV identities are informational and are never authorization evidence.
- TOML policy support is deferred; v0.2 uses strict JSON without adding a parser dependency.

## 0.1.0 - 2026-09-20

### Added

- Shared deterministic Codex gate evaluation for startup projection and preventive tool checks.
- Projection schema v2 with execution status, blocker kinds, exact UTF-8 byte budgeting, and retained/omitted counts.
- Nonblocking SessionStart reads with explicit busy-state reporting.
- Reproducible macOS arm64 Codex plugin template and packager.

### Changed

- `codex-session-start` now requires the same `--protected-tool` and optional `--require-plan` policy inputs as `codex-pre-tool-use`.
- Aporic now reports version `0.1.0`; the append-only event schema remains v1.

### Known limitations

- Projection budgeting is deterministic byte bounding, not tokenizer-aware optimization.
- The plugin artifact is currently limited to macOS arm64 and one configured tool/workspace/scope.
- Existing plan authorization remains valid when a later Aporia blocks new authorizations; revocation or a tool hold is required to reclaim execution authority.

## 0.0.1 - 2026-09-20

### Added

- Deterministic append-only JSONL commitment-state kernel.
- Explicit Aporia, delegation, decision, accepted-risk, tool-hold, plan, and plan-authorization lifecycles.
- Revision checks, cooperative file locking, idempotent retries, corruption detection, and synchronized appends.
- Codex `SessionStart` context projection and opt-in `PreToolUse` prevention for `apply_patch`.
- Exact scope, session, and tool binding for plan authorization.
- Strict JSON contracts and focused regression tests.
- Fail-closed rejection of unsupported stored schema versions and cross-scope delegation revocation.

### Known limitations

- Actor identity and provenance are caller-declared, not authenticated.
- The preventive adapter protects only the configured tool; the initial local plugin targets `apply_patch`.
- Plan quality and the semantic relationship between a plan and a patch are not evaluated.
- Same-user tampering, shell bypasses, and hook-process failure are not security boundaries.
- A preventive check is not atomically bound to the later filesystem effect.
- Data synchronization is not a power-loss guarantee; repair, migration, and snapshot mechanisms are absent.
- Projected session context is untrusted and does not provide prompt-injection immunity.
