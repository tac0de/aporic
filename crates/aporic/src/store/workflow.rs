//! Advisory stage transitions for one task. Events are the durable source of truth.
use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use uuid::Uuid;

use super::{
    Error, Result, Store, append_event, canonical_workspace, duplicate_result, load_task,
    require_text, unix_millis,
};
use crate::domain::{
    CoordinatedTask, DelegationDisposition, DelegationRunOutcome, EvidenceGrade, EvidenceKind,
    TaskStatus, WorkflowAdvanceRequest, WorkflowOutcome, WorkflowPlan, WorkflowPlanRequest,
    WorkflowProcedureDepth, WorkflowProcedureProfile, WorkflowStage, WorkflowStatus,
    WorkflowStatusRequest, WorkflowStepDefinition, WorkflowStepDisposition,
    WorkflowStepRecordRequest, WorkflowStepStatus, WorkflowSteps, WorkflowStepsRequest,
    WorkflowTransition,
};

const PLAN_EVENT: &str = "task_workflow_planned";
const ADVANCE_EVENT: &str = "task_workflow_advanced";
const STEP_EVENT: &str = "task_workflow_step_recorded";
const MAX_EVENTS: i64 = 128;
const PROCEDURE_TEMPLATE_VERSION: u32 = 1;

impl Store {
    pub fn plan_workflow(&self, request: &WorkflowPlanRequest) -> Result<WorkflowOutcome> {
        require_text("task_id", &request.task_id)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        if request.procedure_profile.is_some() != request.procedure_depth.is_some() {
            return Err(Error::Invalid(
                "procedure_profile and procedure_depth must be set together".into(),
            ));
        }
        let (procedure_profile, procedure_depth) =
            match (request.procedure_profile, request.procedure_depth) {
                (None, None) => (
                    Some(WorkflowProcedureProfile::General),
                    Some(if request.material_change {
                        WorkflowProcedureDepth::Standard
                    } else {
                        WorkflowProcedureDepth::Light
                    }),
                ),
                pair => pair,
            };
        for (name, value) in [
            ("objective", &request.objective),
            ("target_user", &request.target_user),
            ("constraints", &request.constraints),
            ("success_measure", &request.success_measure),
        ] {
            if value.len() > 2_000 {
                return Err(Error::Invalid(format!("{name} exceeds 2000 bytes")));
            }
        }
        if request.material_unknowns.len() > 16
            || request
                .material_unknowns
                .iter()
                .any(|value| value.trim().is_empty() || value.len() > 500)
        {
            return Err(Error::Invalid(
                "material_unknowns requires at most 16 nonempty entries of 500 bytes".into(),
            ));
        }
        if request.unknown_resolutions.len() > 16 {
            return Err(Error::Invalid(
                "at most 16 unknown resolutions are allowed".into(),
            ));
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<WorkflowOutcome, _>(
            &transaction,
            &request.idempotency_key,
            PLAN_EVENT,
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let task = active_task(&transaction, &request.task_id)?;
        require_capacity(&transaction, &request.task_id)?;
        let previous = read_status(&transaction, &task)?;
        let old_unknowns = previous
            .plan
            .as_ref()
            .map(|plan| plan.material_unknowns.as_slice())
            .unwrap_or(&[]);
        for unknown in old_unknowns {
            if request.material_unknowns.contains(unknown) {
                continue;
            }
            let resolutions = request
                .unknown_resolutions
                .iter()
                .filter(|resolution| &resolution.unknown == unknown)
                .collect::<Vec<_>>();
            if resolutions.len() != 1 {
                return Err(Error::Conflict(format!(
                    "removed material unknown needs one evidenced resolution: {unknown}"
                )));
            }
            check_resolution(
                &transaction,
                &task,
                &resolutions[0].evidence_id,
                previous.plan.as_ref().unwrap().created_at_unix_ms,
            )?;
        }
        if request.unknown_resolutions.iter().any(|resolution| {
            !old_unknowns.contains(&resolution.unknown)
                || request.material_unknowns.contains(&resolution.unknown)
        }) {
            return Err(Error::Invalid(
                "unknown_resolutions must name only removed prior unknowns".into(),
            ));
        }
        let scope_changed = previous.plan.as_ref().is_some_and(|plan| {
            (plan.requires_user_decision && !request.requires_user_decision)
                || (plan.material_change && !request.material_change)
                || (plan.procedure_profile == Some(WorkflowProcedureProfile::Ui)
                    && procedure_profile != Some(WorkflowProcedureProfile::Ui))
                || (plan.procedure_profile.is_some() && procedure_profile.is_none())
                || (plan.procedure_depth.is_some()
                    && (procedure_depth.is_none() || procedure_depth < plan.procedure_depth))
        });
        if scope_changed {
            let evidence_id = request.scope_change_evidence_id.as_deref().ok_or_else(|| {
                Error::Conflict(
                    "relaxing a decision or review requirement needs reported user evidence".into(),
                )
            })?;
            check_evidence(
                &transaction,
                &task,
                evidence_id,
                EvidenceKind::UserStatement,
                EvidenceGrade::Reported,
                previous.plan.as_ref().unwrap().created_at_unix_ms,
            )?;
        } else if request.scope_change_evidence_id.is_some() {
            return Err(Error::Invalid(
                "scope_change_evidence_id is only for relaxed requirements".into(),
            ));
        }
        let now = unix_millis()?;
        let mut status = WorkflowStatus {
            task_id: request.task_id.clone(),
            stage: WorkflowStage::Intake,
            plan: Some(WorkflowPlan {
                revision_id: Uuid::now_v7().to_string(),
                created_at_unix_ms: now,
                objective: request.objective.clone(),
                target_user: request.target_user.clone(),
                constraints: request.constraints.clone(),
                success_measure: request.success_measure.clone(),
                material_unknowns: request.material_unknowns.clone(),
                unknown_resolutions: request.unknown_resolutions.clone(),
                scope_change_evidence_id: request.scope_change_evidence_id.clone(),
                requires_user_decision: request.requires_user_decision,
                material_change: request.material_change,
                procedure_profile,
                procedure_depth,
                procedure_template_version: procedure_profile.map(|_| PROCEDURE_TEMPLATE_VERSION),
            }),
            transitions: Vec::new(),
            step_statuses: Vec::new(),
            missing_for_next_stage: Vec::new(),
            advisory: true,
            executable: false,
        };
        status.missing_for_next_stage = missing(&transaction, &task, &status)?;
        let outcome = WorkflowOutcome {
            status,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.task_id,
            PLAN_EVENT,
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn advance_workflow(&self, request: &WorkflowAdvanceRequest) -> Result<WorkflowOutcome> {
        require_text("task_id", &request.task_id)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        if request.artifact_evidence_ids.len() > 8 {
            return Err(Error::Invalid(
                "at most 8 artifact evidence IDs are allowed".into(),
            ));
        }
        let unique = request
            .artifact_evidence_ids
            .iter()
            .collect::<BTreeSet<_>>();
        if unique.len() != request.artifact_evidence_ids.len() {
            return Err(Error::Invalid("duplicate artifact evidence IDs".into()));
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<WorkflowOutcome, _>(
            &transaction,
            &request.idempotency_key,
            ADVANCE_EVENT,
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let task = load_task(&transaction, &request.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        let mut status = read_status(&transaction, &task)?;
        if task.status == TaskStatus::Cancelled {
            return Err(Error::Conflict(
                "cancelled tasks cannot advance workflow".into(),
            ));
        }
        if task.status == TaskStatus::Completed {
            if status.stage != WorkflowStage::Verification {
                return Err(Error::Conflict(
                    "completed tasks may only close verification".into(),
                ));
            }
        } else {
            super::require_open_session(&transaction, &task.session_id)?;
        }
        if status.stage != request.expected_stage {
            return Err(Error::Conflict(format!(
                "workflow is at {:?}, not {:?}",
                status.stage, request.expected_stage
            )));
        }
        if status.plan.is_none() {
            return Err(Error::Conflict("workflow plan is missing".into()));
        }
        let gaps = missing(&transaction, &task, &status)?;
        if !gaps.is_empty() {
            return Err(Error::Conflict(format!(
                "stage prerequisites missing: {}",
                gaps.join(", ")
            )));
        }
        let next = match status.stage {
            WorkflowStage::Intake => WorkflowStage::Planning,
            WorkflowStage::Planning => WorkflowStage::Design,
            WorkflowStage::Design => WorkflowStage::Implementation,
            WorkflowStage::Implementation => WorkflowStage::Verification,
            WorkflowStage::Verification => WorkflowStage::Completed,
            WorkflowStage::Completed => {
                return Err(Error::Conflict("workflow already completed".into()));
            }
        };
        if matches!(
            status.stage,
            WorkflowStage::Planning | WorkflowStage::Design | WorkflowStage::Implementation
        ) {
            if request.artifact_evidence_ids.is_empty() {
                return Err(Error::Conflict(
                    "a direct workspace-file artifact is required for this transition".into(),
                ));
            }
            for evidence_id in &request.artifact_evidence_ids {
                check_evidence(
                    &transaction,
                    &task,
                    evidence_id,
                    EvidenceKind::WorkspaceFile,
                    EvidenceGrade::Direct,
                    status.plan.as_ref().unwrap().created_at_unix_ms,
                )?;
            }
        } else if !request.artifact_evidence_ids.is_empty() {
            return Err(Error::Invalid(
                "artifact evidence is not used for this transition".into(),
            ));
        }
        let needs_user_choice = status.stage == WorkflowStage::Design
            && status
                .plan
                .as_ref()
                .is_some_and(|plan| plan.requires_user_decision);
        if needs_user_choice {
            let evidence_id = request
                .user_decision_evidence_id
                .as_deref()
                .ok_or_else(|| {
                    Error::Conflict("reported user decision evidence is required".into())
                })?;
            check_evidence(
                &transaction,
                &task,
                evidence_id,
                EvidenceKind::UserStatement,
                EvidenceGrade::Reported,
                status.plan.as_ref().unwrap().created_at_unix_ms,
            )?;
        } else if request.user_decision_evidence_id.is_some() {
            return Err(Error::Invalid(
                "user decision evidence is not used for this transition".into(),
            ));
        }
        require_capacity(&transaction, &request.task_id)?;
        let delegation_decision_id = if status.stage == WorkflowStage::Design {
            Some(
                super::delegation::status_for_task(&transaction, &task.task_id)?
                    .decisions
                    .last()
                    .expect("design prerequisite checked")
                    .decision_id
                    .clone(),
            )
        } else {
            None
        };
        status.stage = next;
        status.transitions.push(WorkflowTransition {
            stage: next,
            artifact_evidence_ids: request.artifact_evidence_ids.clone(),
            user_decision_evidence_id: request.user_decision_evidence_id.clone(),
            delegation_decision_id,
        });
        status.missing_for_next_stage = missing(&transaction, &task, &status)?;
        let outcome = WorkflowOutcome {
            status,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.task_id,
            ADVANCE_EVENT,
            request,
            &outcome,
            unix_millis()?,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn workflow_status(&self, request: &WorkflowStatusRequest) -> Result<WorkflowStatus> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let task = load_task(&connection, &request.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        let actual: Option<String> = connection
            .query_row(
                "SELECT workspace FROM projects WHERE project_id = ?1",
                [&task.project_id],
                |row| row.get(0),
            )
            .optional()?;
        if actual.as_deref() != Some(workspace.as_str()) {
            return Err(Error::NotFound(format!("task {}", request.task_id)));
        }
        read_status(&connection, &task)
    }

    pub fn workflow_steps(&self, request: &WorkflowStepsRequest) -> Result<WorkflowSteps> {
        let status = self.workflow_status(&WorkflowStatusRequest {
            workspace: request.workspace.clone(),
            task_id: request.task_id.clone(),
        })?;
        let definitions = status
            .plan
            .as_ref()
            .map_or_else(Vec::new, definitions_for_plan);
        Ok(WorkflowSteps {
            task_id: request.task_id.clone(),
            template_version: status
                .plan
                .as_ref()
                .and_then(|plan| plan.procedure_template_version),
            definitions,
            status,
        })
    }

    pub fn record_workflow_step(
        &self,
        request: &WorkflowStepRecordRequest,
    ) -> Result<WorkflowOutcome> {
        require_text("task_id", &request.task_id)?;
        require_text("step_id", &request.step_id)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        if request.evidence_ids.len() > 8
            || request.evidence_ids.iter().collect::<BTreeSet<_>>().len()
                != request.evidence_ids.len()
        {
            return Err(Error::Invalid(
                "step evidence IDs must be unique and at most 8".into(),
            ));
        }
        if request
            .reason
            .as_ref()
            .is_some_and(|reason| reason.len() > 500)
        {
            return Err(Error::Invalid("step reason exceeds 500 bytes".into()));
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<WorkflowOutcome, _>(
            &transaction,
            &request.idempotency_key,
            STEP_EVENT,
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let task = active_task(&transaction, &request.task_id)?;
        let mut status = read_status(&transaction, &task)?;
        let plan = status
            .plan
            .as_ref()
            .ok_or_else(|| Error::Conflict("workflow plan is missing".into()))?;
        let definition = definitions_for_plan(plan)
            .into_iter()
            .find(|step| step.step_id == request.step_id)
            .ok_or_else(|| {
                Error::Invalid(format!(
                    "unknown step for current procedure: {}",
                    request.step_id
                ))
            })?;
        let target_index = stage_index(definition.stage);
        let current_index = stage_index(status.stage);
        if request.disposition == WorkflowStepDisposition::ReworkRequired {
            if target_index > current_index || status.stage == WorkflowStage::Completed {
                return Err(Error::Conflict(
                    "rework target must be the current or an earlier active stage".into(),
                ));
            }
        } else if definition.stage != status.stage {
            return Err(Error::Conflict("step is not in the current stage".into()));
        }
        match request.disposition {
            WorkflowStepDisposition::Completed => {
                if request.evidence_ids.is_empty() || request.reason.is_some() {
                    return Err(Error::Invalid(
                        "completed step needs direct file evidence and no reason".into(),
                    ));
                }
            }
            WorkflowStepDisposition::Skipped => {
                if !definition.skippable
                    || !request.evidence_ids.is_empty()
                    || request
                        .reason
                        .as_deref()
                        .is_none_or(|reason| reason.trim().is_empty())
                {
                    return Err(Error::Invalid(
                        "step is not skippable, or skip needs a reason and no evidence".into(),
                    ));
                }
            }
            WorkflowStepDisposition::ReworkRequired => {
                if request
                    .reason
                    .as_deref()
                    .is_none_or(|reason| reason.trim().is_empty())
                {
                    return Err(Error::Invalid("rework needs a concrete reason".into()));
                }
            }
        }
        for evidence_id in &request.evidence_ids {
            check_evidence(
                &transaction,
                &task,
                evidence_id,
                EvidenceKind::WorkspaceFile,
                EvidenceGrade::Direct,
                plan.created_at_unix_ms,
            )?;
            if request.disposition == WorkflowStepDisposition::Completed {
                require_evidence_after_latest_rework(&transaction, &task, evidence_id)?;
            }
        }
        require_capacity(&transaction, &request.task_id)?;
        if request.disposition == WorkflowStepDisposition::ReworkRequired {
            status.stage = definition.stage;
            status
                .transitions
                .retain(|transition| stage_index(transition.stage) <= target_index);
            status
                .step_statuses
                .retain(|step| stage_index(step.stage) < target_index);
        } else {
            status
                .step_statuses
                .retain(|step| step.step_id != request.step_id);
        }
        status.step_statuses.push(WorkflowStepStatus {
            step_id: request.step_id.clone(),
            stage: definition.stage,
            disposition: request.disposition,
            evidence_ids: request.evidence_ids.clone(),
            reason: request.reason.clone(),
        });
        status.missing_for_next_stage = missing(&transaction, &task, &status)?;
        let outcome = WorkflowOutcome {
            status,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.task_id,
            STEP_EVENT,
            request,
            &outcome,
            unix_millis()?,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }
}

fn active_task(connection: &Transaction<'_>, task_id: &str) -> Result<CoordinatedTask> {
    let task = load_task(connection, task_id)?
        .ok_or_else(|| Error::NotFound(format!("task {task_id}")))?;
    if matches!(task.status, TaskStatus::Completed | TaskStatus::Cancelled) {
        return Err(Error::Conflict(
            "workflow planning requires an active task".into(),
        ));
    }
    super::require_open_session(connection, &task.session_id)?;
    Ok(task)
}

fn require_capacity(connection: &Connection, task_id: &str) -> Result<()> {
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM events WHERE stream_id = ?1 AND kind IN (?2, ?3, ?4)",
        params![task_id, PLAN_EVENT, ADVANCE_EVENT, STEP_EVENT],
        |row| row.get(0),
    )?;
    if count >= MAX_EVENTS {
        return Err(Error::Invalid("workflow event limit reached".into()));
    }
    Ok(())
}

fn read_status(connection: &Connection, task: &CoordinatedTask) -> Result<WorkflowStatus> {
    let result: Option<String> = connection.query_row(
        "SELECT result_json FROM events WHERE stream_id = ?1 AND kind IN (?2, ?3, ?4) ORDER BY sequence DESC LIMIT 1",
        params![task.task_id, PLAN_EVENT, ADVANCE_EVENT, STEP_EVENT], |row| row.get(0),
    ).optional()?;
    let mut status = if let Some(result) = result {
        serde_json::from_str::<WorkflowOutcome>(&result)?.status
    } else {
        WorkflowStatus {
            task_id: task.task_id.clone(),
            stage: WorkflowStage::Intake,
            plan: None,
            transitions: Vec::new(),
            step_statuses: Vec::new(),
            missing_for_next_stage: Vec::new(),
            advisory: true,
            executable: false,
        }
    };
    status.missing_for_next_stage = missing(connection, task, &status)?;
    Ok(status)
}

fn missing(
    connection: &Connection,
    task: &CoordinatedTask,
    status: &WorkflowStatus,
) -> Result<Vec<String>> {
    let mut gaps = Vec::new();
    let Some(plan) = &status.plan else {
        return Ok(vec!["workflow_plan_missing".into()]);
    };
    if plan.procedure_profile.is_some() && plan.procedure_template_version != Some(1) {
        gaps.push("procedure_template_version_unsupported".into());
    }
    for definition in definitions_for_plan(plan)
        .into_iter()
        .filter(|definition| definition.stage == status.stage)
    {
        if !status.step_statuses.iter().any(|step| {
            step.step_id == definition.step_id
                && matches!(
                    step.disposition,
                    WorkflowStepDisposition::Completed | WorkflowStepDisposition::Skipped
                )
        }) {
            gaps.push(format!("procedure_step_missing:{}", definition.step_id));
        }
    }
    match status.stage {
        WorkflowStage::Intake => {
            for (name, value) in [
                ("objective", &plan.objective),
                ("target_user", &plan.target_user),
                ("constraints", &plan.constraints),
                ("success_measure", &plan.success_measure),
            ] {
                if value.trim().is_empty() {
                    gaps.push(format!("{name}_missing"));
                }
            }
            if !plan.material_unknowns.is_empty() {
                gaps.push("material_unknowns_unresolved".into());
            }
        }
        WorkflowStage::Design => {
            let delegation = super::delegation::status_for_task(connection, &task.task_id)?;
            if delegation
                .decisions
                .last()
                .is_none_or(|decision| decision.created_at_unix_ms < plan.created_at_unix_ms)
            {
                gaps.push("delegation_assessment_missing_for_plan".into());
            } else if delegation
                .decisions
                .last()
                .is_some_and(|decision| decision.material_change != plan.material_change)
            {
                gaps.push("delegation_materiality_differs_from_plan".into());
            } else if plan.material_change
                && delegation.decisions.last().is_some_and(|decision| {
                    decision.reviewer.disposition == DelegationDisposition::NotRequired
                })
            {
                gaps.push("material_change_reviewer_decision_missing".into());
            }
        }
        WorkflowStage::Verification => {
            if task.status != TaskStatus::Completed {
                gaps.push("task_acceptance_proofs_incomplete".into());
            }
            let delegation = super::delegation::status_for_task(connection, &task.task_id)?;
            let bound = status
                .transitions
                .iter()
                .find(|transition| transition.stage == WorkflowStage::Implementation)
                .and_then(|transition| transition.delegation_decision_id.as_deref());
            if let Some(decision) = bound.and_then(|id| {
                delegation
                    .decisions
                    .iter()
                    .find(|decision| decision.decision_id == id)
            }) {
                if decision.reviewer.disposition == DelegationDisposition::Delegate
                    && !delegation.reports.iter().any(|report| {
                        report.decision_id == decision.decision_id
                            && report.outcome == DelegationRunOutcome::Completed
                            && report.dimension == crate::domain::DelegationDimension::Reviewer
                    })
                {
                    gaps.push("reviewer_completion_not_reported".into());
                }
            } else {
                gaps.push("design_delegation_decision_missing".into());
            }
        }
        WorkflowStage::Planning | WorkflowStage::Implementation | WorkflowStage::Completed => {}
    }
    Ok(gaps)
}

fn check_evidence(
    connection: &Connection,
    task: &CoordinatedTask,
    id: &str,
    kind: EvidenceKind,
    grade: EvidenceGrade,
    not_before_unix_ms: i64,
) -> Result<()> {
    require_text("evidence_id", id)?;
    let found: Option<(String, String, String, i64)> = connection
        .query_row(
            "SELECT e.session_id, e.kind, e.grade, e.created_at_unix_ms FROM evidence_artifacts e
         JOIN sessions s ON s.session_id = e.session_id WHERE e.evidence_id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;
    if !matches!(found, Some((session, actual_kind, actual_grade, created))
        if session == task.session_id && actual_kind == kind.as_str()
            && actual_grade == grade.as_str() && created >= not_before_unix_ms)
    {
        return Err(Error::Conflict(format!(
            "evidence {id} must be {:?}/{:?} in the task session after its plan",
            kind, grade
        )));
    }
    ensure_not_bound_elsewhere(connection, task, id)?;
    Ok(())
}

fn check_resolution(
    connection: &Connection,
    task: &CoordinatedTask,
    id: &str,
    not_before_unix_ms: i64,
) -> Result<()> {
    require_text("evidence_id", id)?;
    let found: Option<(String, String, String, i64)> = connection
        .query_row(
            "SELECT e.session_id, e.kind, e.grade, e.created_at_unix_ms FROM evidence_artifacts e
         JOIN sessions s ON s.session_id = e.session_id WHERE e.evidence_id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;
    if !matches!(found, Some((session, kind, grade, created)) if session == task.session_id
        && created >= not_before_unix_ms
        && ((kind == EvidenceKind::WorkspaceFile.as_str() && grade == EvidenceGrade::Direct.as_str())
            || (kind == EvidenceKind::UserStatement.as_str() && grade == EvidenceGrade::Reported.as_str())))
    {
        return Err(Error::Conflict(format!(
            "resolution {id} needs task-session direct file or reported user statement evidence"
        )));
    }
    ensure_not_bound_elsewhere(connection, task, id)?;
    Ok(())
}

fn ensure_not_bound_elsewhere(
    connection: &Connection,
    task: &CoordinatedTask,
    id: &str,
) -> Result<()> {
    let used: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM events e
         JOIN tasks t ON t.task_id = e.stream_id
         JOIN json_tree(e.result_json) value
         WHERE t.project_id = ?1 AND e.stream_id != ?2
           AND e.kind IN (?3, ?4, ?5) AND value.type = 'text' AND value.value = ?6)",
        params![
            task.project_id,
            task.task_id,
            PLAN_EVENT,
            ADVANCE_EVENT,
            STEP_EVENT,
            id
        ],
        |row| row.get(0),
    )?;
    if used {
        return Err(Error::Conflict(format!(
            "evidence {id} is already bound to another workflow task"
        )));
    }
    Ok(())
}

fn require_evidence_after_latest_rework(
    connection: &Connection,
    task: &CoordinatedTask,
    evidence_id: &str,
) -> Result<()> {
    let latest_rework: Option<i64> = connection.query_row(
        "SELECT MAX(sequence) FROM events WHERE stream_id = ?1 AND kind = ?2
         AND json_extract(payload_json, '$.disposition') = 'rework_required'",
        params![task.task_id, STEP_EVENT],
        |row| row.get(0),
    )?;
    let Some(latest_rework) = latest_rework else {
        return Ok(());
    };
    let evidence_sequence: Option<i64> = connection
        .query_row(
            "SELECT sequence FROM events WHERE stream_id = ?1 AND kind = 'evidence_added'
         AND json_extract(result_json, '$.evidence.evidence_id') = ?2 LIMIT 1",
            params![task.session_id, evidence_id],
            |row| row.get(0),
        )
        .optional()?;
    if evidence_sequence.is_none_or(|sequence| sequence <= latest_rework) {
        return Err(Error::Conflict(format!(
            "evidence {evidence_id} must be registered after the latest workflow rework"
        )));
    }
    Ok(())
}

struct StepSpec {
    id: &'static str,
    stage: WorkflowStage,
    description: &'static str,
    minimum_depth: WorkflowProcedureDepth,
    ui_only: bool,
    skippable: bool,
}

const STEP_SPECS: &[StepSpec] = &[
    StepSpec {
        id: "problem_and_outcome",
        stage: WorkflowStage::Intake,
        description: "Describe the user problem and observable outcome",
        minimum_depth: WorkflowProcedureDepth::Light,
        ui_only: false,
        skippable: false,
    },
    StepSpec {
        id: "baseline_and_constraints",
        stage: WorkflowStage::Planning,
        description: "Inspect current behavior and constraints",
        minimum_depth: WorkflowProcedureDepth::Light,
        ui_only: false,
        skippable: false,
    },
    StepSpec {
        id: "options_and_risks",
        stage: WorkflowStage::Planning,
        description: "Compare approaches and identify risks",
        minimum_depth: WorkflowProcedureDepth::Standard,
        ui_only: false,
        skippable: true,
    },
    StepSpec {
        id: "verification_strategy",
        stage: WorkflowStage::Planning,
        description: "Define checks for the desired outcome",
        minimum_depth: WorkflowProcedureDepth::Standard,
        ui_only: false,
        skippable: true,
    },
    StepSpec {
        id: "delivery_contract",
        stage: WorkflowStage::Design,
        description: "Specify behavior, interfaces, and handoff details",
        minimum_depth: WorkflowProcedureDepth::Light,
        ui_only: false,
        skippable: false,
    },
    StepSpec {
        id: "prototype_feedback",
        stage: WorkflowStage::Design,
        description: "Evaluate a small prototype and revise the design",
        minimum_depth: WorkflowProcedureDepth::High,
        ui_only: false,
        skippable: true,
    },
    StepSpec {
        id: "concept_comparison",
        stage: WorkflowStage::Design,
        description: "Compare distinct visual concepts when direction is uncertain",
        minimum_depth: WorkflowProcedureDepth::Standard,
        ui_only: true,
        skippable: true,
    },
    StepSpec {
        id: "interaction_spec",
        stage: WorkflowStage::Design,
        description: "Specify interaction states, responsive behavior, and assets",
        minimum_depth: WorkflowProcedureDepth::Standard,
        ui_only: true,
        skippable: true,
    },
    StepSpec {
        id: "vertical_slice",
        stage: WorkflowStage::Implementation,
        description: "Implement a usable end-to-end slice",
        minimum_depth: WorkflowProcedureDepth::Light,
        ui_only: false,
        skippable: false,
    },
    StepSpec {
        id: "scenario_walkthrough",
        stage: WorkflowStage::Implementation,
        description: "Walk through the primary user scenario and revise defects",
        minimum_depth: WorkflowProcedureDepth::Standard,
        ui_only: false,
        skippable: true,
    },
    StepSpec {
        id: "rendered_browser_review",
        stage: WorkflowStage::Implementation,
        description: "Inspect the rendered UI and its interactions",
        minimum_depth: WorkflowProcedureDepth::Standard,
        ui_only: true,
        skippable: false,
    },
    StepSpec {
        id: "mechanical_checks",
        stage: WorkflowStage::Verification,
        description: "Run relevant mechanical checks and record results",
        minimum_depth: WorkflowProcedureDepth::Light,
        ui_only: false,
        skippable: false,
    },
    StepSpec {
        id: "quality_review",
        stage: WorkflowStage::Verification,
        description: "Review usability or operational quality and triage findings",
        minimum_depth: WorkflowProcedureDepth::Standard,
        ui_only: false,
        skippable: true,
    },
    StepSpec {
        id: "residual_risks",
        stage: WorkflowStage::Verification,
        description: "Document remaining risks and follow-up work",
        minimum_depth: WorkflowProcedureDepth::High,
        ui_only: false,
        skippable: true,
    },
    StepSpec {
        id: "responsive_accessibility_review",
        stage: WorkflowStage::Verification,
        description: "Review narrow layouts, keyboard behavior, and accessibility",
        minimum_depth: WorkflowProcedureDepth::High,
        ui_only: true,
        skippable: false,
    },
];

fn definitions_for_plan(plan: &WorkflowPlan) -> Vec<WorkflowStepDefinition> {
    // Keep each stored version's catalog even when new plans use a later version.
    if plan.procedure_template_version != Some(1) {
        return Vec::new();
    }
    let (Some(profile), Some(depth)) = (plan.procedure_profile, plan.procedure_depth) else {
        return Vec::new();
    };
    STEP_SPECS
        .iter()
        .filter(|step| {
            depth >= step.minimum_depth
                && (!step.ui_only || profile == WorkflowProcedureProfile::Ui)
        })
        .map(|step| WorkflowStepDefinition {
            step_id: step.id.into(),
            stage: step.stage,
            description: step.description.into(),
            skippable: step.skippable,
        })
        .collect()
}

fn stage_index(stage: WorkflowStage) -> u8 {
    match stage {
        WorkflowStage::Intake => 0,
        WorkflowStage::Planning => 1,
        WorkflowStage::Design => 2,
        WorkflowStage::Implementation => 3,
        WorkflowStage::Verification => 4,
        WorkflowStage::Completed => 5,
    }
}
