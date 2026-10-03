# Aporic

Aporic is a local-first Rust hub for project continuity, advisory coordination,
and verification evidence. Codex connects through a local stdio MCP server.
It preserves decisions and work state across sessions, separates reported claims
from observed results, and tracks when changes invalidate declared check evidence.

Product delivery packages connect repository execution methods, requirements,
scenarios, design, implementation and version-bound local checks. General, web,
web-game and web-platform profiles describe applicable verification categories.
See the [delivery guide](docs/product-delivery.md) and
[example packages](examples/product-delivery/README.md).

Aporic does not launch agents, authorize host tools, judge product quality or
publish websites. Integrations remain advisory and fail-open. Its effect on
real development time and quality remains to be established through actual use.

## Build and run

```console
cargo build --release --locked
./target/release/aporic identity
./target/release/aporic release validate --checkout .
./target/release/aporic mcp serve --stdio
```

The default MCP core profile exposes 58 tools. Use `--profile full` for all 72,
including optional diagnostics; core rejects calls to full-only tools.
Replacing a binary does not update an already-running MCP process. Reconnect
that process and inspect its runtime identity; see [core workflow](docs/core-flow.md).

State lives in platform-native application data outside the governed repository
and `~/.codex`. Set `APORIC_DATABASE` to an explicit path for an isolated store.
The [Codex integration templates](integrations/codex/) are not automatically
installed and do not change global configuration.

## Work across sessions

1. `aporic_begin` opens a session, records the host's work-shape assessment and
   returns bounded historical context.
2. Perform the authorized work. When host agents run, record actual starts and
   outcomes through task or session delegation reports.
3. `aporic_finish` saves bounded durable notes and completes or hands off the
   session. For a continuation request, first use `aporic_resume` and check its
   candidate against current intent and repository state.

Historical context is data, not authority. Store durable changes rather than
raw conversation or tool output. The host owns spawning, model choice and
permissions. Existing `open`, `record` and `close` clients remain supported.

## Product capabilities

| Area | Current responsibility | Guide |
| --- | --- | --- |
| Continuity and memory | Append-only records, explicit supersession, deterministic bounded FTS5 recall, resumable handoffs | [Core workflow](docs/core-flow.md) |
| Tasks and workflows | Contracts, dependencies, write leases, versioned procedures, criterion-bound completion, initiative traceability | [Workflow gates](docs/workflow-gates.md) |
| Product delivery | Execution manifests, dependency graphs, focused briefs, change impact, check profiles and local-runner bindings | [Delivery packages](docs/product-delivery.md) |
| Design delivery | File-backed references, concepts, specifications and browser-review inventory | [Design delivery](docs/design-delivery.md) |
| Model advice | Typed worker/reviewer route recommendations; no model invocation | [Model routing](docs/model-routing.md) |
| Diagnostics and research | Provenance-labelled research, repair records, privacy-minimized hooks, local Git observations and usage receipts | [Architecture](docs/architecture.md) |

The optional Codex hook stores bounded metadata, byte counts and keyed hashes,
not prompts, tool payloads or transcripts. Tool and permission hooks return
`{}` and fail open. Observed coverage and shadow classifications never become
host enforcement.

## Verification boundaries

MCP can register immutable command specifications and inspect receipts. Only
the local CLI runner executes a check and issues its execution claim:

```console
./target/release/aporic verify --spec SPEC_ID
./target/release/aporic executions reconcile --stale-after 300
./target/release/aporic delivery validate --workspace . --manifest delivery/current.json
./target/release/aporic delivery verify --workspace . --manifest delivery/current.json --check CHECK_ID --spec SPEC_ID
```

The runner captures exact command execution, output hashes and declared
artifacts. Delivery verification additionally binds pre/post declared input
bytes to the check result. Successful execution does not prove assertion
adequacy, gameplay, visual quality, undeclared-input coverage or remote effects.
A declaration or model-authored report cannot establish verified completion.

The default host execution backend is not an OS security sandbox. Checks that
require the Linux `bubblewrap` backend cannot receive execution credit if that
required backend fails or is unavailable. This check-level requirement does
not narrow the host's own tool permissions.

## Maintenance and development

```console
./target/release/aporic doctor
./target/release/aporic export --workspace /absolute/project/path
./target/release/aporic backup --to /absolute/aporic-backup.sqlite3
./target/release/aporic restore --dry-run /absolute/aporic-backup.sqlite3
./target/release/aporic restore --from /absolute/aporic-backup.sqlite3 --to /absolute/restored.sqlite3
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

The Rust regression suites cover state transitions, replay, migration,
provenance, bounded retrieval, runner failures and delivery freshness. Offline
`eval` commands exercise deterministic fixtures without model or network calls;
their results are not model-quality or productivity benchmarks.

Historical migrations and raw event export preserve existing databases,
including records from retired features. They do not restore those features to
the current API. Read [PRODUCT.md](PRODUCT.md),
[architecture](docs/architecture.md) and [CONTRIBUTING.md](CONTRIBUTING.md)
for product boundaries and development requirements.

Licensed under MIT or Apache-2.0.
