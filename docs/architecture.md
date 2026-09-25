# Aporic hub architecture

## Boundary

The Aporic kernel and Aporic Hub have different lifecycles.

- The kernel is an exact behavioral constitution and a set of deterministic
  invariants. It has no transport, database, model, or host dependency.
- The hub is evolving software that applies those invariants to projects,
  sessions, tasks, decisions, evidence, effects, and agents.
- MCP is an adapter. It does not become the domain model and does not by itself
  form a security boundary around other host tools.

## First implementation

The first implementation is one Rust package with internal `kernel`, `domain`,
`store`, `hub`, and `mcp` modules. Package boundaries will be introduced only
when an independently versioned contract or deployment unit exists.

```text
Codex --stdio MCP--> mcp adapter --> hub services --> SQLite
                                      |
                                      +--> kernel invariants
```

Each Codex connection runs an inexpensive child process. Processes share one
SQLite database in WAL mode. Each command opens a short connection and commits
an event and its read projection in one immediate transaction.

## Durable model

The first slice stores projects, sessions, durable records, and events.
Records have an explicit epistemic or operational kind: decision, constraint,
task progress, observation, effect, verification, or material unknown.

The event log preserves accepted state transitions and idempotent results. Read
tables provide bounded context without replaying the whole log on every tool
call. Tests replay events independently and compare the result with projections.

## MCP surface

- `aporic_open`: start an idempotent session and return recent context.
- `aporic_recall`: retrieve a bounded project capsule.
- `aporic_record`: append one durable typed record.
- `aporic_close`: complete or hand off a session.

Tool failures are returned as visible tool-level errors. Protocol errors are
reserved for malformed MCP requests that cannot be routed or decoded.

## Later growth

Agent dispatch, leases, waiting, cancellation, remote transports, and controlled
effect execution are later layers. They must build on observed demand and must
not make the first continuity loop depend on them.
