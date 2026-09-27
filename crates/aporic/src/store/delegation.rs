//! Reported host coordination, replayed directly from bounded task-stream events.
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use uuid::Uuid;

use super::{
    Error, Result, Store, append_event, canonical_workspace, duplicate_result, load_task,
    require_identifier, require_open_session, require_text, unix_millis,
};
use crate::domain::{
    DelegationChoice, DelegationDecision, DelegationDecisionOutcome, DelegationDecisionRequest,
    DelegationDimension, DelegationDisposition, DelegationReport, DelegationReportOutcome,
    DelegationReportRequest, DelegationRunOutcome, DelegationStatus, DelegationStatusRequest,
    EvidenceGrade, SessionDelegationDecision, SessionDelegationDecisionOutcome,
    SessionDelegationDecisionRequest, SessionDelegationReport, SessionDelegationReportOutcome,
    SessionDelegationReportRequest, SessionDelegationStatus, SessionDelegationStatusRequest,
    TaskStatus,
};

const DECISION_EVENT: &str = "task_delegation_assessed";
const REPORT_EVENT: &str = "task_delegation_reported";
const SESSION_DECISION_EVENT: &str = "session_delegation_assessed";
const SESSION_REPORT_EVENT: &str = "session_delegation_reported";
const MAX_EVENTS: usize = 64;

impl Store {
    pub fn assess_session_delegation(
        &self,
        request: &SessionDelegationDecisionRequest,
    ) -> Result<SessionDelegationDecisionOutcome> {
        validate_ids(&request.session_id, &request.idempotency_key)?;
        if request.parallel_paths > 32 {
            return Err(Error::Invalid("parallel_paths exceeds 32".to_owned()));
        }
        validate_choice("worker", &request.worker, request.parallel_paths >= 2)?;
        validate_choice("reviewer", &request.reviewer, request.material_change)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut previous) = duplicate_result::<SessionDelegationDecisionOutcome, _>(
            &transaction,
            &request.idempotency_key,
            SESSION_DECISION_EVENT,
            request,
        )? {
            previous.duplicate = true;
            return Ok(previous);
        }
        require_open_session(&transaction, &request.session_id)?;
        let status = read_session_status(&transaction, &request.session_id)?;
        require_session_capacity(&status, 1)?;
        let now = unix_millis()?;
        let outcome = SessionDelegationDecisionOutcome {
            decision: SessionDelegationDecision {
                decision_id: Uuid::now_v7().to_string(),
                session_id: request.session_id.clone(),
                parallel_paths: request.parallel_paths,
                material_change: request.material_change,
                worker: request.worker.clone(),
                reviewer: request.reviewer.clone(),
                evidence_grade: EvidenceGrade::Reported,
                created_at_unix_ms: now,
            },
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.session_id,
            SESSION_DECISION_EVENT,
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn report_session_delegation(
        &self,
        request: &SessionDelegationReportRequest,
    ) -> Result<SessionDelegationReportOutcome> {
        validate_ids(&request.session_id, &request.idempotency_key)?;
        require_identifier("decision_id", &request.decision_id, 128)?;
        require_identifier("host_agent_id", &request.host_agent_id, 128)?;
        require_identifier("model", &request.model, 128)?;
        require_identifier("reasoning_effort", &request.reasoning_effort, 32)?;
        bounded_text("result_summary", &request.result_summary)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut previous) = duplicate_result::<SessionDelegationReportOutcome, _>(
            &transaction,
            &request.idempotency_key,
            SESSION_REPORT_EVENT,
            request,
        )? {
            previous.duplicate = true;
            return Ok(previous);
        }
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM sessions WHERE session_id = ?1)",
            [&request.session_id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(Error::NotFound(format!("session {}", request.session_id)));
        }
        let status = read_session_status(&transaction, &request.session_id)?;
        let decision = status
            .decisions
            .iter()
            .find(|decision| decision.decision_id == request.decision_id)
            .ok_or_else(|| {
                Error::Conflict("report must reference a decision for this session".to_owned())
            })?;
        let choice = match request.dimension {
            DelegationDimension::Worker => &decision.worker,
            DelegationDimension::Reviewer => &decision.reviewer,
        };
        if choice.disposition != DelegationDisposition::Delegate {
            return Err(Error::Conflict(
                "reported dimension must have a delegate decision".to_owned(),
            ));
        }
        let prior = status.reports.iter().rev().find(|report| {
            report.decision_id == request.decision_id
                && report.dimension == request.dimension
                && report.host_agent_id == request.host_agent_id
        });
        match request.outcome {
            DelegationRunOutcome::Started => {
                require_open_session(&transaction, &request.session_id)?;
                if status
                    .decisions
                    .last()
                    .is_none_or(|latest| latest.decision_id != request.decision_id)
                    || prior.is_some()
                {
                    return Err(Error::Conflict(
                        "start requires the latest decision and a new host agent run".to_owned(),
                    ));
                }
            }
            DelegationRunOutcome::Completed | DelegationRunOutcome::Failed => {
                let started = prior
                    .filter(|report| report.outcome == DelegationRunOutcome::Started)
                    .ok_or_else(|| {
                        Error::Conflict(
                            "terminal report requires an unfinished reported start".to_owned(),
                        )
                    })?;
                if started.model != request.model
                    || started.reasoning_effort != request.reasoning_effort
                {
                    return Err(Error::Conflict(
                        "terminal report must retain the started model and reasoning effort"
                            .to_owned(),
                    ));
                }
            }
        }
        if status.reports.iter().any(|report| {
            report.host_agent_id == request.host_agent_id && report.dimension != request.dimension
        }) {
            return Err(Error::Conflict(
                "worker and reviewer must have different host agent IDs".to_owned(),
            ));
        }
        let reservation = if request.outcome == DelegationRunOutcome::Started {
            2
        } else {
            0
        };
        require_session_capacity(&status, reservation)?;
        let now = unix_millis()?;
        let outcome = SessionDelegationReportOutcome {
            report: SessionDelegationReport {
                report_id: Uuid::now_v7().to_string(),
                session_id: request.session_id.clone(),
                decision_id: request.decision_id.clone(),
                dimension: request.dimension.clone(),
                host_agent_id: request.host_agent_id.clone(),
                model: request.model.clone(),
                reasoning_effort: request.reasoning_effort.clone(),
                outcome: request.outcome.clone(),
                result_summary: request.result_summary.clone(),
                evidence_grade: EvidenceGrade::Reported,
                created_at_unix_ms: now,
            },
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.session_id,
            SESSION_REPORT_EVENT,
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn session_delegation_status(
        &self,
        request: &SessionDelegationStatusRequest,
    ) -> Result<SessionDelegationStatus> {
        require_identifier("session_id", &request.session_id, 128)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let belongs: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sessions JOIN projects USING(project_id)
             WHERE sessions.session_id = ?1 AND projects.workspace = ?2)",
            params![request.session_id, workspace],
            |row| row.get(0),
        )?;
        if !belongs {
            return Err(Error::NotFound(format!("session {}", request.session_id)));
        }
        read_session_status(&connection, &request.session_id)
    }

    pub fn assess_delegation(
        &self,
        request: &DelegationDecisionRequest,
    ) -> Result<DelegationDecisionOutcome> {
        validate_ids(&request.task_id, &request.idempotency_key)?;
        if request.parallel_paths > 32 {
            return Err(Error::Invalid("parallel_paths exceeds 32".to_owned()));
        }
        validate_choice("worker", &request.worker, request.parallel_paths >= 2)?;
        validate_choice("reviewer", &request.reviewer, request.material_change)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut previous) = duplicate_result::<DelegationDecisionOutcome, _>(
            &transaction,
            &request.idempotency_key,
            DECISION_EVENT,
            request,
        )? {
            previous.duplicate = true;
            return Ok(previous);
        }
        require_active_task(&transaction, &request.task_id)?;
        require_capacity(&transaction, &request.task_id)?;
        let now = unix_millis()?;
        let outcome = DelegationDecisionOutcome {
            decision: DelegationDecision {
                decision_id: Uuid::now_v7().to_string(),
                task_id: request.task_id.clone(),
                parallel_paths: request.parallel_paths,
                material_change: request.material_change,
                worker: request.worker.clone(),
                reviewer: request.reviewer.clone(),
                evidence_grade: EvidenceGrade::Reported,
                created_at_unix_ms: now,
            },
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.task_id,
            DECISION_EVENT,
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn report_delegation(
        &self,
        request: &DelegationReportRequest,
    ) -> Result<DelegationReportOutcome> {
        validate_ids(&request.task_id, &request.idempotency_key)?;
        require_identifier("decision_id", &request.decision_id, 128)?;
        require_identifier("host_agent_id", &request.host_agent_id, 128)?;
        require_identifier("model", &request.model, 128)?;
        require_identifier("reasoning_effort", &request.reasoning_effort, 32)?;
        bounded_text("result_summary", &request.result_summary)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut previous) = duplicate_result::<DelegationReportOutcome, _>(
            &transaction,
            &request.idempotency_key,
            REPORT_EVENT,
            request,
        )? {
            previous.duplicate = true;
            return Ok(previous);
        }
        require_active_task(&transaction, &request.task_id)?;
        require_capacity(&transaction, &request.task_id)?;
        let state = read_status(&transaction, &request.task_id)?;
        let decision = state
            .decisions
            .iter()
            .find(|decision| decision.decision_id == request.decision_id)
            .ok_or_else(|| {
                Error::Conflict("report must reference a decision for this task".to_owned())
            })?;
        let latest_decision = state.decisions.last().expect("referenced decision exists");
        let existing_agent = state.reports.iter().any(|report| {
            report.decision_id == request.decision_id
                && report.dimension == request.dimension
                && report.host_agent_id == request.host_agent_id
        });
        if latest_decision.decision_id != request.decision_id
            && (request.outcome == DelegationRunOutcome::Started || !existing_agent)
        {
            return Err(Error::Conflict(
                "an earlier decision permits only terminal reports for already reported agents"
                    .to_owned(),
            ));
        }
        let choice = match request.dimension {
            DelegationDimension::Worker => &decision.worker,
            DelegationDimension::Reviewer => &decision.reviewer,
        };
        if choice.disposition != DelegationDisposition::Delegate {
            return Err(Error::Conflict(
                "reported dimension must have a delegate decision".to_owned(),
            ));
        }
        if state.reports.iter().any(|report| {
            report.host_agent_id == request.host_agent_id && report.dimension != request.dimension
        }) {
            return Err(Error::Conflict(
                "worker and reviewer must have different host agent IDs".to_owned(),
            ));
        }
        let now = unix_millis()?;
        let outcome = DelegationReportOutcome {
            report: DelegationReport {
                report_id: Uuid::now_v7().to_string(),
                task_id: request.task_id.clone(),
                decision_id: request.decision_id.clone(),
                dimension: request.dimension.clone(),
                host_agent_id: request.host_agent_id.clone(),
                model: request.model.clone(),
                reasoning_effort: request.reasoning_effort.clone(),
                outcome: request.outcome.clone(),
                result_summary: request.result_summary.clone(),
                evidence_grade: EvidenceGrade::Reported,
                created_at_unix_ms: now,
            },
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.task_id,
            REPORT_EVENT,
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn delegation_status(&self, request: &DelegationStatusRequest) -> Result<DelegationStatus> {
        require_identifier("task_id", &request.task_id, 128)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let task = load_task(&connection, &request.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        let project_id = connection
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [&workspace],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if project_id.as_deref() != Some(task.project_id.as_str()) {
            return Err(Error::NotFound(format!("task {}", request.task_id)));
        }
        read_status(&connection, &request.task_id)
    }
}

fn validate_ids(task_id: &str, key: &str) -> Result<()> {
    require_identifier("task_id", task_id, 128)?;
    require_text("idempotency_key", key)?;
    if key.len() > 256 {
        return Err(Error::Invalid(
            "idempotency_key exceeds 256 bytes".to_owned(),
        ));
    }
    Ok(())
}

fn bounded_text(name: &str, value: &str) -> Result<()> {
    require_text(name, value)?;
    if value.len() > 2_000 {
        return Err(Error::Invalid(format!("{name} exceeds 2000 bytes")));
    }
    Ok(())
}

fn validate_choice(name: &str, choice: &DelegationChoice, required: bool) -> Result<()> {
    bounded_text(&format!("{name}.reason"), &choice.reason)?;
    if required && choice.disposition == DelegationDisposition::NotRequired {
        return Err(Error::Invalid(format!(
            "{name} requires delegation or an explicit skip reason"
        )));
    }
    Ok(())
}

fn require_active_task(connection: &Transaction<'_>, task_id: &str) -> Result<()> {
    let task = load_task(connection, task_id)?
        .ok_or_else(|| Error::NotFound(format!("task {task_id}")))?;
    if matches!(task.status, TaskStatus::Completed | TaskStatus::Cancelled) {
        return Err(Error::Conflict(
            "delegation recording requires an active task".to_owned(),
        ));
    }
    require_open_session(connection, &task.session_id)
}

fn require_capacity(connection: &Connection, task_id: &str) -> Result<()> {
    let count: usize = connection.query_row(
        "SELECT COUNT(*) FROM events WHERE stream_id = ?1 AND kind IN (?2, ?3)",
        params![task_id, DECISION_EVENT, REPORT_EVENT],
        |row| row.get(0),
    )?;
    if count >= MAX_EVENTS {
        return Err(Error::Invalid(
            "a task may record at most 64 delegation events".to_owned(),
        ));
    }
    Ok(())
}

fn require_session_capacity(status: &SessionDelegationStatus, reservation: usize) -> Result<()> {
    let count = status.decisions.len() + status.reports.len();
    let started = status
        .reports
        .iter()
        .filter(|report| report.outcome == DelegationRunOutcome::Started)
        .count();
    let terminal = status.reports.len() - started;
    let pending = started.checked_sub(terminal).ok_or_else(|| {
        Error::Conflict(
            "session delegation history has more terminal reports than starts".to_owned(),
        )
    })?;
    if count >= MAX_EVENTS || count + pending + reservation > MAX_EVENTS {
        return Err(Error::Invalid(
            "a session may record at most 64 delegation events including reserved terminal reports"
                .to_owned(),
        ));
    }
    Ok(())
}

fn read_session_status(
    connection: &Connection,
    session_id: &str,
) -> Result<SessionDelegationStatus> {
    let mut status = SessionDelegationStatus {
        session_id: session_id.to_owned(),
        decisions: Vec::new(),
        reports: Vec::new(),
        task_assessment_count: 0,
        assessment_missing: false,
        advisory_gaps: Vec::new(),
        advisory: true,
        executable: false,
    };
    let mut statement = connection.prepare(
        "SELECT kind, result_json FROM events WHERE stream_id = ?1
         AND kind IN ('session_delegation_assessed', 'session_delegation_reported')
         ORDER BY sequence ASC LIMIT 65",
    )?;
    let rows = statement.query_map([session_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (kind, result) = row?;
        if kind == SESSION_DECISION_EVENT {
            status
                .decisions
                .push(serde_json::from_str::<SessionDelegationDecisionOutcome>(&result)?.decision);
        } else {
            status
                .reports
                .push(serde_json::from_str::<SessionDelegationReportOutcome>(&result)?.report);
        }
    }
    if status.decisions.len() + status.reports.len() > MAX_EVENTS {
        return Err(Error::Conflict(
            "session delegation event history exceeds its bound".to_owned(),
        ));
    }
    status.task_assessment_count = connection.query_row(
        "SELECT COUNT(*) FROM events JOIN tasks ON tasks.task_id = events.stream_id
         WHERE tasks.session_id = ?1 AND events.kind = ?2",
        params![session_id, DECISION_EVENT],
        |row| row.get(0),
    )?;
    status.assessment_missing = status.decisions.is_empty() && status.task_assessment_count == 0;
    if let Some(decision) = status.decisions.last() {
        for (dimension, choice, label) in [
            (DelegationDimension::Worker, &decision.worker, "worker"),
            (
                DelegationDimension::Reviewer,
                &decision.reviewer,
                "reviewer",
            ),
        ] {
            if choice.disposition == DelegationDisposition::Delegate
                && !status.reports.iter().any(|report| {
                    report.decision_id == decision.decision_id && report.dimension == dimension
                })
            {
                status
                    .advisory_gaps
                    .push(format!("{label}_host_execution_not_reported"));
            }
        }
    } else if status.assessment_missing {
        status
            .advisory_gaps
            .push("delegation_assessment_missing".to_owned());
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut execution_gaps = std::collections::BTreeSet::new();
    for report in status.reports.iter().rev() {
        let reviewer = report.dimension == DelegationDimension::Reviewer;
        if !seen.insert((&report.decision_id, reviewer, &report.host_agent_id)) {
            continue;
        }
        let label = if reviewer { "reviewer" } else { "worker" };
        match report.outcome {
            DelegationRunOutcome::Failed => {
                execution_gaps.insert(format!("{label}_host_execution_reported_failed"));
            }
            DelegationRunOutcome::Started => {
                execution_gaps.insert(format!("{label}_completion_not_reported"));
            }
            DelegationRunOutcome::Completed => {}
        }
    }
    status.advisory_gaps.extend(execution_gaps);
    Ok(status)
}

pub(super) fn status_for_session(
    connection: &Connection,
    session_id: &str,
) -> Result<SessionDelegationStatus> {
    read_session_status(connection, session_id)
}

fn read_status(connection: &Connection, task_id: &str) -> Result<DelegationStatus> {
    let mut status = DelegationStatus {
        task_id: task_id.to_owned(),
        decisions: Vec::new(),
        reports: Vec::new(),
        advisory_gaps: Vec::new(),
        advisory: true,
        executable: false,
    };
    let mut statement = connection.prepare(
        "SELECT kind, result_json FROM events WHERE stream_id = ?1 AND kind IN (?2, ?3)
         ORDER BY sequence ASC LIMIT 65",
    )?;
    let rows = statement.query_map(params![task_id, DECISION_EVENT, REPORT_EVENT], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (kind, result) = row?;
        if kind == DECISION_EVENT {
            status
                .decisions
                .push(serde_json::from_str::<DelegationDecisionOutcome>(&result)?.decision);
        } else {
            status
                .reports
                .push(serde_json::from_str::<DelegationReportOutcome>(&result)?.report);
        }
    }
    if status.decisions.len() + status.reports.len() > MAX_EVENTS {
        return Err(Error::Conflict(
            "delegation event history exceeds its bound".to_owned(),
        ));
    }
    if let Some(decision) = status.decisions.last() {
        for (dimension, choice, label) in [
            (DelegationDimension::Worker, &decision.worker, "worker"),
            (
                DelegationDimension::Reviewer,
                &decision.reviewer,
                "reviewer",
            ),
        ] {
            if choice.disposition != DelegationDisposition::Delegate {
                continue;
            }
            let reported = status.reports.iter().any(|report| {
                report.decision_id == decision.decision_id && report.dimension == dimension
            });
            if !reported {
                status
                    .advisory_gaps
                    .push(format!("{label}_host_execution_not_reported"));
            }
        }
    } else {
        status
            .advisory_gaps
            .push("delegation_assessment_missing".to_owned());
    }
    // A later report from another agent or a new assessment cannot hide an
    // earlier agent's failure or unfinished review. Fold each agent's history
    // independently, retaining the latest observation for that exact run scope.
    let mut seen = std::collections::BTreeSet::new();
    let mut execution_gaps = std::collections::BTreeSet::new();
    for report in status.reports.iter().rev() {
        let reviewer = report.dimension == DelegationDimension::Reviewer;
        if !seen.insert((&report.decision_id, reviewer, &report.host_agent_id)) {
            continue;
        }
        let label = if reviewer { "reviewer" } else { "worker" };
        if report.outcome == DelegationRunOutcome::Failed {
            execution_gaps.insert(format!("{label}_host_execution_reported_failed"));
        } else if reviewer && report.outcome != DelegationRunOutcome::Completed {
            execution_gaps.insert("reviewer_completion_not_reported".to_owned());
        }
    }
    status.advisory_gaps.extend(execution_gaps);
    Ok(status)
}

pub(super) fn status_for_task(connection: &Connection, task_id: &str) -> Result<DelegationStatus> {
    read_status(connection, task_id)
}
