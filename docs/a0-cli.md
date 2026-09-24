# A0 minimal CLI

`aporicctl` is an optional thin adapter over the clean-room Rust crates. It
reads strict JSON, invokes typed APIs, and emits compact JSON. It does not
install hooks, launch or configure a model host, discover projects globally, or
infer authority.

The `grant`, `reserve`, `effect`, `verify`, and `abandon` commands
expose the isolated authorization experiment explicitly. No automatic
integration calls them.

Build it with:

```console
cargo build -p aporic-cli --bin aporicctl
```

Every command takes an absolute connection-file path as its first argument.
Commands that mutate state read one JSON document from standard input.

## Bind a project and role package

Create an external directory, then bind a Git repository to one `role.md +
role.json` package. The connection, project registry, kernel ledger, and
handoff ledger stay outside the project; Aporic never creates project-local
`.aporic` state.

```console
mkdir -p /absolute/external/aporic-demo
aporicctl bind /absolute/external/aporic-demo/connection.json < bind.json
```

`bind.json` has this shape:

```json
{
  "schema_version": 1,
  "project_id": "demo",
  "workspace": "/absolute/project",
  "remote_name": "origin",
  "role_directory": "/absolute/aporic/roles/generalist",
  "host_policy": {
    "capabilities": ["workspace.read", "workspace.write"],
    "default_routing": "economy",
    "routing_ceiling": "deep",
    "max_tool_calls": 32,
    "max_parallel_tasks": 1,
    "may_delegate": false
  },
  "boundary_policy": {
    "actions": {
      "write_file": {
        "capability": "workspace.write",
        "delegates": false
      }
    }
  }
}
```

The role loader validates and hashes the Markdown and JSON package. It compiles
the manifest against the supplied host limits into a typed execution profile;
neither the host layer nor kernel interprets Markdown as authority.

## Commands

| Command | Standard input | Result |
| --- | --- | --- |
| `bind` | bind document | Creates the external connection and ledgers |
| `status` | none | Reports project observation, profile, and revisions |
| `day-open` | `OpenDayRequest` | Opens one continuity day |
| `handoff-project` | `ProjectHandoffRequest` | Records a bounded handoff |
| `day-close` | `CloseDayRequest` | Closes against the exact handoff digest |
| `grant` | `GrantRequest` | Records explicit experimental authority |
| `reserve` | `ReserveRequest` | Reserves one exact experimental action |
| `effect` | `EffectRequest` | Records an observed effect |
| `verify` | `VerificationRequest` | Records independent verification |
| `abandon` | `AbandonRequest` | Abandons an unused reservation |
| `route` | `RoutingSignals` | Returns a bounded generic routing tier |

The request names are strict serialized Rust types exported by `aporic-host`.
Unknown JSON fields are rejected. Kernel and handoff revisions are independent.

Exit status `0` means committed, duplicate, or successful read. Exit status
`2` means deterministic semantic rejection. Exit status `1` means invalid
input, configuration, identity, or I/O failure.

The connection file is ordinary host configuration. A0 does not authenticate
callers or turn cooperative file locks into a security boundary.
