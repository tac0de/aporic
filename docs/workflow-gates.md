# Advisory planning stages

The task workflow makes missing inputs and stage decisions visible. It follows
the requirements, design, implementation, and verification concerns in
ISO/IEC/IEEE 29148 and 12207, while allowing iteration. It is a task-scoped
coordination contract, not a host permission gate. Aporic does not prevent a
Codex agent from editing files or calling tools outside this workflow.

## Product-initiative core

The task workflow records the work needed to deliver one bounded task. A
product initiative sits above those tasks to make the whole product delivery
contract inspectable. Schema v28 indexes initiative ownership; append-only
events hold each state revision.

An initiative has a versioned plan with these required fields:

| Field | Meaning |
| --- | --- |
| `objective` | The product change or problem the initiative intends to address. |
| `target_user` | The user or operator whose situation the change addresses. |
| `success_measure` | The observable result expected for that user or operator. |

Each plan revision requires direct file evidence for planning, design,
integration, and release. The core records presence and provenance without
choosing a domain-specific document format. Integration evidence must follow
linked task completion, and release evidence must follow integration evidence.

Each requirement belongs to one plan revision and has a stable requirement ID,
a statement, and an acceptance criterion. `aporic_initiative_task_link` connects
it to same-project tasks with that exact acceptance criterion. Each linked task
must complete its own advisory task workflow after the plan revision with a
verified proof for the criterion.
A task's completed state alone cannot establish that its requirement was satisfied.

Initiative status is computed from the current plan revision. It reports gaps
for absent required artifacts, requirements without linked tasks, linked tasks
that are incomplete, and acceptance criteria without verified proofs. A
revision requires reported user-statement evidence and clears artifact and
task-link credit for the entire plan. Historical records
remain append-only so a later status can explain why earlier credit is no
longer current.

Direct file evidence attests only that Aporic read particular bytes. It does
not establish document quality, design suitability, integration correctness,
or real-world user outcomes. Similarly, verified criterion proofs establish
only the specific mechanically verified claim they bind. The initiative core
must therefore report evidence and gaps without representing a product as
complete beyond those observed facts.

Like the task workflow, the initiative is advisory and fail-open. It constrains
only Aporic's initiative-status and completion claims; it cannot block host
edits, tool use, tests, releases, or other permissions.

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
| `verification` | The task completed with verified criterion proofs; a completion report if the latest delegation decision assigned an independent reviewer |

`aporic_workflow_advance` moves exactly one stage, using `expected_stage` to
reject stale callers. It reports concrete missing prerequisites and checks
that evidence belongs to the task's session, was registered after the current
plan, has the expected kind and grade, and is not already bound to another
workflow task. The design transition stores the delegation decision ID, so a
later assessment cannot erase its review requirement. Cancelled tasks stop;
completed tasks can only close verification.
`aporic_workflow_status` returns the current stage and next-stage gaps after
restart. Revising a plan lets a team return to discovery or requirements work.

The user-statement and reviewer records are **reported**, not host-attested.
File evidence proves Aporic read the bytes when it was registered; it does not
prove the document's quality, the user's actual approval, or implementation
fidelity. Operators should use the host conversation and independent review to
assess those claims. A skipped reviewer needs a concrete reason in
`aporic_delegation_assess`; the workflow retains that choice without claiming
an independent review occurred.

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
