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
Relaxing a previously required user decision or material-change review, or
changing an existing plan's procedure profile, requires a reported
user-statement evidence ID in `scope_change_evidence_id`.

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

## Nested procedures

A procedure is a versioned checklist inside the advisory workflow stages. New
plans may choose `procedure_profile` as `general` or `frontend`, and
`procedure_depth` as `light`, `standard`, or `high`. The `frontend` profile
includes the applicable general steps plus its first native module,
`frontend.browser_review`. Existing `ui@v1` plans remain readable and
recordable with their original steps. Changing one to `frontend@v2` is a new
planning revision requiring the reported user evidence above. Existing plans with
no profile remain legacy and
do not acquire procedure requirements retroactively. A new plan that omits
both fields receives `general` at `standard` depth for a material change and
`general` at `light` depth otherwise, so an old client cannot bypass the new
procedure. A plan must supply both fields when it chooses either one.

`aporic_workflow_steps` lists the definitions and the current status for a
task. An absent record means a required step is unresolved; it is not a
separate state. `aporic_workflow_status.missing_for_next_stage` reports it as
`procedure_step_missing:<step_id>`.

### `general@v1` steps

| Depth | Stage | Step ID | Skippable |
| --- | --- | --- | --- |
| light | `intake` | `problem_and_outcome` | No |
| light | `planning` | `baseline_and_constraints` | No |
| light | `design` | `delivery_contract` | No |
| light | `implementation` | `vertical_slice` | No |
| light | `verification` | `mechanical_checks` | No |
| standard | `planning` | `options_and_risks` | Yes |
| standard | `planning` | `verification_strategy` | Yes |
| standard | `implementation` | `scenario_walkthrough` | Yes |
| standard | `verification` | `quality_review` | Yes |
| high | `design` | `prototype_feedback` | Yes |
| high | `verification` | `residual_risks` | Yes |

`standard` adds its rows to `light`; `high` adds its rows to `standard`.

### `ui@v1` additions

This profile remains available for historical compatibility; new frontend work
uses `frontend@v2`.

| Depth | Stage | Step ID | Skippable |
| --- | --- | --- | --- |
| standard | `design` | `concept_comparison` | Yes |
| standard | `design` | `interaction_spec` | Yes |
| standard | `implementation` | `rendered_browser_review` | No |
| high | `verification` | `responsive_accessibility_review` | No |

### `frontend@v2` first module

`frontend.browser_review` is the first small domain module. It adds the
following steps to the applicable `general@v1` steps. MCP step definitions
identify these additions with `module_id: "frontend.browser_review"`; common
steps have no module ID. A later frontend module must use a new template
version so stored tasks keep the procedure they originally selected.

| Depth | Stage | Step ID | Skippable |
| --- | --- | --- | --- |
| standard | `planning` | `browser_scenarios` | No |
| light | `implementation` | `rendered_browser_review` | No |
| standard | `verification` | `responsive_accessibility_review` | No |

The browser steps ask for scenario, rendered interaction, viewport, keyboard,
and accessibility evidence. A workspace file attests recorded bytes only;
Aporic does not run a browser or judge visual quality from the receipt.

### Recording and rework

`aporic_workflow_step_record` accepts one of three resolution states:

| State | Record requirements | Effect |
| --- | --- | --- |
| `completed` | One to eight direct workspace-file evidence IDs. | Resolves the step for the current plan. |
| `skipped` | A concrete reason and no evidence IDs; available only for a skippable step. | Resolves the step for the current plan. |
| `rework_required` | A reason and optionally direct workspace-file evidence. | Reopens the step's stage, clears resolution status for that stage and every later step, and truncates later workflow transitions. |

The older records stay in the append-only task history. A new completion or
skip must therefore be recorded after rework before the task can claim the
affected stage again.

Procedure evidence follows the task's existing provenance rules. A direct
workspace-file receipt proves only that Aporic read the registered bytes. It
does not prove quality, user approval, or an unobserved host action.

These procedures are advisory and fail-open. They make missing checks visible
and constrain only Aporic's procedure and stage-completion claims. They do not
block edits, tests, browser use, delegation, tool calls, Git operations, or any
other host permission. A missing Aporic service or record is a continuity gap
to report, never a reason to represent host work as denied or undone.
