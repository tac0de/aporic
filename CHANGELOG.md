# Changelog

All notable changes are documented here. Aporic is experimental; its contracts may change between minor releases.

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
