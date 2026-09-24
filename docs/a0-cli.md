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

Create an external directory first, then connect a Git repository. The
connection file, project registry, authorization ledger, handoff ledger,
model-control audit, and improvement ledger are created in that directory.
`project-connect` also registers the connection in the global hook catalog.
It refuses state inside the governed repository and never creates `.aporic`
state in it.

```console
mkdir -p /absolute/external/aporic-demo
aporicctl project-connect /absolute/external/aporic-demo/connection.json \
  < project-connect.json
```

`project-connect.json` has this strict shape. `binding` alone is also accepted
by the lower-level `bind` command.

```json
{
  "catalog": "/absolute/aporic-data/catalog",
  "binding": {
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
    },
    "model_control": {
      "schema_version": 1,
      "enforcement": "required",
      "codex_executable": "/absolute/path/to/codex",
      "tiers": {
        "economy": {"model": "gpt-6-luna", "reasoning_effort": "low"},
        "balanced": {"model": "gpt-6-sol", "reasoning_effort": "medium"},
        "deep": {"model": "gpt-6-astra", "reasoning_effort": "high"}
      }
    }
  }
}
```

This is Aporic's current recommended mapping. Model identifiers remain host and
account dependent: resolve the Codex executable to an absolute path and verify
that the target host makes each identifier available. Aporic does not guess or
silently substitute either value.

## Commands

| Command | Standard-input document | Result |
| --- | --- | --- |
| `project-connect` | catalog path plus `BindInput` | Creates all external state and registers the project |
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
| `improvement-request` | bounded request, evidence, and expected evolution revision | Queues an Aporic change without modifying the active runtime |
| `improvement-implemented` | request ID, pushed Aporic commit, and checks | Verifies the commit is on a fetched remote branch and records implementation |
| `improvement-reconnect` | request ID, catalog, build manifest, and expected revision | Verifies and re-registers the project with the current adapter/profile hashes |
| `improvement-list` | none | Replays the improvement workflow |
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

## Improvement loop

Project agents record an improvement only after preserving the current task's
result. The request identifies one component (`kernel`, `role`, `adapter`, or
`integration`) and one change (`add`, `modify`, `merge`, or `remove`). It has no
authority to alter the active runtime.

After the change is implemented in the Aporic repository,
`improvement-implemented` accepts only a full commit reachable from a fetched
remote branch. Package that clean, pushed checkout with
`package-aporic-codex.sh`; the package contains `aporic-build.json`, binding its
commit to the adapter hash. Install the fresh adapter, start a fresh task, and
run `improvement-reconnect` with that manifest; it verifies the running binary
before recording the role-profile hash and exact source-project binding. Only
then is the request complete.

See [C0 Codex adapter, M0 routing, and M1 model control](c0-m0-codex.md) for
automatic lifecycle integration and plugin packaging.
