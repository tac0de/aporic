# Changelog

All notable changes are documented here. Aporic is experimental; its contracts may change between minor releases.

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
