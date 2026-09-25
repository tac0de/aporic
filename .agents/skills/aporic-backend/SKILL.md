---
name: aporic-backend
description: Implement and review Aporic backend work across Rust service APIs, SQLite data and search, local execution and external integrations, and operational reliability. Use for substantive backend changes, not browser UI design.
---

# Aporic backend practice

Use this skill for backend implementation in Aporic. Read the current [architecture](../../../docs/architecture.md), the affected code, and tests before choosing a design. The kernel owns exact deterministic invariants; Hub and Store own evolving application behavior; MCP and CLI are adapters. Keep product logic in Rust and Bash as a thin integration layer. Preserve the current advisory, fail-open integration boundary unless the human explicitly changes it.

## Frame the change

Identify the user-visible or operator-visible problem, the call path, the state or effect being changed, and one observable acceptance check. Follow the path from adapter through Hub, Store, and any kernel rule. State whether the change affects a public contract, persisted data, local execution, external network activity, or operational recovery. Keep unrelated work intact and make migrations and recovery behavior explicit before editing persistent state.

Read only the references relevant to the change:

- [API and service contracts](references/api-and-service.md) for MCP, CLI, Hub methods, request validation, and compatibility.
- [Data and search](references/data-and-search.md) for SQLite schema, append-only records, projections, FTS, and migrations.
- [Execution and integrations](references/execution-and-integrations.md) for local runner behavior, external sources, and trust boundaries.
- [Reliability and performance](references/reliability-and-performance.md) for measurement, failure handling, diagnostics, backup, and rollout checks.

For changes spanning these areas, trace the complete path rather than treating each layer as an isolated patch. Favor a narrow vertical slice with a concrete acceptance check over a broad abstraction built without a caller.

## Implement and verify

Keep input bounds, idempotency, transaction scope, error meaning, and provenance visible in types and tests. Test meaningful state transitions or failure paths when changing a contract or durable state; include restart, replay, or concurrency checks when the risk calls for them. Inspect generated schemas and a real stdio or CLI path when adapter behavior changes. Measure a workload before claiming a performance improvement.

Run appropriate mechanical checks: `cargo fmt --all -- --check`, affected tests, and `cargo clippy -p aporic --all-targets --locked -- -D warnings`; run the full suite for cross-cutting changes. Record what was observed through Aporic's typed evidence or durable records when the work warrants continuity. Do not turn a model assessment, passing test, local Git snapshot, or remote API response into a broader claim than it supports.

Report the changed contract, data and migration effects, verification results, and any remaining operational limit. A completed backend change follows the repository's commit, main integration, push, and branch-cleanup convention unless the human directs otherwise.
