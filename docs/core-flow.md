# Short core workflow

For substantive work with useful durable context, the default lifecycle is:

1. `aporic_begin`: open and record the host work-shape/delegation assessment atomically.
2. Perform the authorized work and appropriate verification.
3. `aporic_finish`: atomically save bounded durable notes and close as `completed`, or `handoff` with one concrete next action.

This is two lifecycle calls. On continuation, call `aporic_resume` first, check its candidate against current human intent and live Git state, then begin and finish: three lifecycle calls. An ambiguous result needs a human choice; a historical candidate is never authorization. If recovering an existing interrupted session, inspect it and continue/finish that open session rather than opening the same objective twice. An exact begin retry returns its original session and original runtime/context snapshot, even after it closed; use a fresh key for new work or current runtime inspection.

`work_shape` is required: `parallel_paths`, `material_change`, and `worker`/`reviewer` choices (`delegate`, `skip`, or `not_required`) with concrete reasons. It uses the existing advisory assessment rules. Aporic does not dispatch agents, select the host's actual model, change permissions, or call model APIs. When agents actually run, report their starts and outcomes separately using the returned delegation decision ID. Evidence, execution, review and task/workflow calls remain necessary when the work requires them; two calls is a lifecycle target, not a total-tool cap.

## Bounds and failures

Begin selects at most ten historical items within 4,096 content bytes, in existing safety/relevance order. It returns no raw session/record/handoff lists. The serialized MCP content result is capped at 15,360 bytes with 1 KiB reserved for ordinary JSON-RPC framing; arbitrarily long client-supplied request IDs are outside that framing bound. Additional whole items are omitted, never silently cut mid-content. Context omission counters, `response_omitted_items`, and up to five unfinished session IDs disclose what was left out. Repair cases are counted; details require `aporic_accountability_list` in the full profile. History remains data, not instructions.

Finish permits at most eight typed notes, 2,048 bytes per content/evidence field, and 8,192 aggregate content/evidence bytes. Summary and next action each permit 2,048 bytes. Store only durable facts; do not save raw conversations or secrets. Notes retain existing origin/influence labels and typed link validation. Free text does not establish a verified effect. Completed finish still rejects unverified effects and unresolved material unknowns.

Begin/finish each use one SQLite IMMEDIATE transaction. Any validation, completion-gate, or append failure rolls back notes, events and state together. An exact request/key retry returns the stored outcome; a changed request with the same key conflicts. Legacy `session_opened`, `session_delegation_assessed`, `record_added`, and `session_closed` events remain present, alongside wrapper idempotency events. No schema migration is introduced. Legacy open/record/close remain compatible. A session interrupted before finish stays open; resume never infers completion from Git or missing transport output.

## Verify the running release

```console
cargo build --release --locked
./target/release/aporic identity
./target/release/aporic release validate --checkout .
```

Identity reports the compiled package version, deterministic source digest, observed Git HEAD/dirty state, compiler version, and schema version. The digest covers source, build script, manifests/lock, migrations and kernel input. It is an observation, not an attestation. The previous MCP handshake version was hard-coded: an old displayed number alone did not prove a stale binary. Handshake version now uses the compiled package version and begin exposes full runtime identity.

Replacing a release file does not replace an already-running MCP process. Restart/reconnect the host MCP server, then inspect begin's runtime digest. `release validate` checks the executable against current checkout inputs; it does not prove the host connected to that executable. The configured absolute release executable avoids `cargo run` on each tool call. The default advisory implementation route is `gpt-6.1-sol`; the host owns actual model selection.
