# Advisory planning stages

The task workflow makes missing inputs and stage decisions visible. It follows
the requirements, design, implementation, and verification concerns in
ISO/IEC/IEEE 29148 and 12207, while allowing iteration. It is a task-scoped
coordination contract, not a host permission gate. Aporic does not prevent a
Codex agent from editing files or calling tools outside this workflow.

## Stages

`aporic_workflow_plan` records a full planning revision for an existing active
task. The brief has an objective, target user, constraints, observable success
measure, material unknowns, and flags for user decision and material change.
Blank brief fields and unresolved material unknowns appear in
`aporic_workflow_status.missing_for_next_stage`. A removed material unknown
requires an exact `unknown_resolutions` entry with evidence from the task's
session: a
direct workspace file or a reported user statement. Changing the plan resets
the stage to `intake` and discards prior transition credit; its older events
remain in the append-only task history.
Relaxing a previously required user decision or material-change review also
requires a reported user-statement evidence ID in `scope_change_evidence_id`.

| Advance from | Required before the transition |
| --- | --- |
| `intake` | All four brief fields filled and no material unknowns open |
| `planning` | A direct workspace-file planning artifact |
| `design` | A direct workspace-file design artifact; a delegation assessment made for the current plan; a reported user-statement evidence item if the plan requires a user decision |
| `implementation` | A direct workspace-file implementation artifact |
| `verification` | The task completed with verified criterion proofs; a completion report if the latest delegation decision assigned an Inspector |

`aporic_workflow_advance` moves exactly one stage, using `expected_stage` to
reject stale callers. It reports concrete missing prerequisites and checks
that evidence belongs to the task's session, was registered after the current
plan, has the expected kind and grade, and is not already bound to another
workflow task. The design transition stores the delegation decision ID, so a
later assessment cannot erase its Inspector requirement. Cancelled tasks stop;
completed tasks can only close verification.
`aporic_workflow_status` returns the current stage and next-stage gaps after
restart. Revising a plan lets a team return to discovery or requirements work.

The user-statement and Inspector records are **reported**, not host-attested.
File evidence proves Aporic read the bytes when it was registered; it does not
prove the document's quality, the user's actual approval, or implementation
fidelity. Operators should use the host conversation and independent review to
assess those claims. A skipped Inspector needs a concrete reason in
`aporic_delegation_assess`; the workflow retains that choice without claiming
an Inspector ran.

For substantial tasks, create the task, record its plan, resolve material
unknowns, advance each stage, and read gaps before claiming the next stage is
finished. The existing `aporic_task_claim` and host tools remain fail-open and
advisory; this workflow governs Aporic's stage claims only.
