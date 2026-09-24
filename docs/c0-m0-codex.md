# C0 Codex adapter, M0-lite routing, and M1 model control

## Boundary

The Codex adapter is a host integration, not part of the authorization kernel.
It consumes the documented Codex hook JSON and invokes the same H0 and D0 APIs
as `aporicctl`.

- `SessionStart` opens or resumes one Aporic day and injects a bounded handoff.
- `PreToolUse` requires and consumes one exact grant before a local tool runs.
- `PostToolUse` records an `unknown` effect; it never infers success from output.
- `PreCompact` projects a bounded, transcript-independent handoff.
- `SessionEnd` refreshes, seals, and closes that handoff.

Hosted tools and specialized paths that do not emit local tool hooks remain
outside this adapter. A successful Aporic pre-tool check emits no allow decision,
so it does not bypass Codex's own permission flow.

## Routing

M0-lite selects `economy`, `balanced`, or `deep` from explicit structured
signals. The default generalist route is `economy`. Multi-step work or a first
retry raises it to `balanced`; material uncertainty, failed verification,
repeated failure, or an explicit burn-budget signal raises it to `deep`. The
effective role ceiling always wins.

Every reservation stores the chosen tier and reason codes in the append-only
kernel record. The `route` command can be used by another host before it creates
a model request:

```console
aporicctl route /absolute/state/connection.json < routing-signals.json
```

Codex hooks report the recommendation as context but do not claim to change the
active Codex model or reasoning effort because the hook output contract does not
provide that control.

## Model control

M1 keeps host application outside the deterministic authorization kernel. An
external connection maps each routing tier to an explicit model and reasoning
effort. `model-plan` exposes the exact result without side effects.

`codex-launch` applies that result at the supported boundary: before a new
Codex process starts. It supplies explicit CLI overrides, pins the process to
the bound workspace, rejects conflicting caller overrides, and writes separate
append-only `planned` and `finished` audit records. Required control fails
closed; it never falls back to a cheaper or stronger target silently.

The Rust capability interface also produces the documented Agents API session
update request for hosts that own such a session. It does not send network
requests or handle credentials. The Codex lifecycle hook remains advisory
because it cannot mutate the active model. Launch enforcement is not immutable:
a human can deliberately change the model later through the Codex UI or `/model`.

## Package and register

Create a fresh local plugin directory:

```console
./scripts/package-aporic-codex.sh /absolute/output/aporic-codex
```

Choose one external Aporic data root and register each bound connection into its
catalog:

```console
export APORIC_DATA_HOME=/absolute/external/aporic-data
mkdir -p "$APORIC_DATA_HOME"
printf '%s\n' '{"connection_file":"/absolute/project-state/connection.json"}' |
  /absolute/output/aporic-codex/bin/aporicctl catalog-register \
  "$APORIC_DATA_HOME/catalog"
```

Install the generated directory through a local Codex plugin marketplace. The
hook definition must then be reviewed and trusted in Codex. The packaged hook
uses `APORIC_DATA_HOME/catalog` when that variable is present and otherwise uses
its private `PLUGIN_DATA/catalog` directory.

The catalog and all ledgers remain outside governed repositories. The closest
registered workspace wins for nested projects.
