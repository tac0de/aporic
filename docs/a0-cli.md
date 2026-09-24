# A0 minimal CLI

`aporicctl` is the thin adapter over the clean-room Aporic crates. Most commands
only translate strict JSON into the H0 and D0 APIs. The explicit `codex-launch`
host command is the sole exception: it starts a new Codex process with the M1
model and reasoning overrides selected before launch. It does not call a model
itself, install hooks, or infer authority.

Build it with:

```console
cargo build -p aporic-cli --bin aporicctl
```

Every command takes an absolute connection-file path as its first argument.
Commands other than `codex-launch` take no additional arguments. Commands that
mutate state read one JSON document from standard input and emit one compact
JSON document to standard output.

## Bind a project

Create an external directory first, then bind a Git repository. The connection
file, project registry, authorization ledger, handoff ledger, and model-control
audit are created in that directory. `bind` refuses a directory inside the
governed repository and never creates `.aporic` state in it.

```console
mkdir -p /absolute/external/aporic-demo
aporicctl bind /absolute/external/aporic-demo/connection.json < bind.json
```

`bind.json` has this strict shape:

```json
{
  "schema_version": 1,
  "project_id": "demo",
  "workspace": "/absolute/project",
  "remote_name": "origin",
  "role_directory": "/absolute/aporic/roles/generalist",
  "host_policy": {
    "capabilities": ["workspace.read", "workspace.write"],
    "default_routing": "balanced",
    "routing_ceiling": "balanced",
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
  },
  "model_control": {
    "schema_version": 1,
    "enforcement": "required",
    "codex_executable": "/absolute/path/to/codex",
    "tiers": {
      "economy": {"model": "model-for-economy", "reasoning_effort": "low"},
      "balanced": {"model": "model-for-balanced", "reasoning_effort": "medium"},
      "deep": {"model": "model-for-deep", "reasoning_effort": "high"}
    }
  }
}
```

Model identifiers are host and account dependent. Resolve the Codex executable
to an absolute path and choose identifiers that the target host actually makes
available; Aporic does not guess or silently substitute either value.

## Commands

| Command | Standard-input document | Result |
| --- | --- | --- |
| `status` | none | Project observation, profile, and both revisions |
| `day-open` | `OpenDayRequest` | Opens one session/day |
| `handoff-project` | `ProjectHandoffRequest` | Returns the sealed handoff digest |
| `day-close` | `CloseDayRequest` | Closes against that exact digest |
| `grant` | `GrantRequest` | Records exact authority |
| `reserve` | `ReserveRequest` | Consumes it into one reservation |
| `effect` | `EffectRequest` | Records the observed effect |
| `verify` | `VerificationRequest` | Records independent verification |
| `abandon` | `AbandonRequest` | Abandons an unused reservation |
| `route` | `RoutingSignals` | Returns the bounded M0 recommendation |
| `model-plan` | `RoutingSignals` | Returns the exact M1 application plan without executing it |
| `catalog-register` | connection-file reference | Registers a project for C0 discovery |
| `codex-hook` | Codex hook JSON | Runs the C0 lifecycle adapter |
| `codex-launch` | none; takes an absolute routing-signals file as its next argument | Launches a new Codex session with required M1 overrides |

Launch syntax is:

```console
aporicctl codex-launch /absolute/state/connection.json \
  /absolute/state/routing-signals.json -- [CODEX_ARGUMENTS...]
```

The launcher rejects caller-provided model, profile, and config overrides,
passes explicit `--model` and `model_reasoning_effort` values before the caller
arguments, fixes the Codex working directory to the bound project, and records
both the plan and process outcome in `model-control.jsonl`. A required mapping
fails closed when its application capability is unavailable. This is
launch-bound enforcement: a human can still use Codex's own `/model` command
later, and Codex may reject a model or effort unavailable to that account.

The request names above are the strict serialized Rust types exported by
`aporic-host`. Unknown JSON fields are rejected. Revisions for the kernel and
handoff logs are independent; use `status` to read both.

Exit status `0` means committed, duplicate, or successful read. Exit status `2`
means a deterministic semantic rejection whose reason is in `reason_code`.
Exit status `1` means invalid input, configuration, project identity, or I/O
failure. JSON errors are written to standard error.

The connection file is host configuration and should be writable only by its
operator. A0 does not authenticate callers or make the cooperative file lock a
security boundary.

See [C0 Codex adapter, M0 routing, and M1 model control](c0-m0-codex.md) for
automatic lifecycle integration and plugin packaging.
