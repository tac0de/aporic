use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::{
    AbandonedSession, ActiveSession, CloseDisposition, CloseOutcome, CloseRequest, ContextCapsule,
    CoordinatedTask, CriterionEvidence, DurableRecord, ExportEvent, ExportSession, Handoff,
    HubStats, OpenOutcome, OpenRequest, ProjectExport, RecallRequest, ReconcileOutcome,
    ReconcileRequest, RecordKind, RecordOutcome, RecordRequest, TaskCancelRequest,
    TaskClaimRequest, TaskCompleteRequest, TaskCreateRequest, TaskListRequest, TaskOutcome,
    TaskStatus,
};

const SCHEMA: &str = include_str!("../../../migrations/0001_initial.sql");
const MIGRATION_2: &str = include_str!("../../../migrations/0002_continuity_hardening.sql");
const MIGRATION_3: &str = include_str!("../../../migrations/0003_coordination.sql");

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid request: {0}")]
    Invalid(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Store {
    path: PathBuf,
}

impl Store {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let store = Self { path: path.into() };
        if let Some(parent) = store.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let connection = store.connection()?;
        let schema_version =
            connection.pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))?;
        match schema_version {
            0 => connection.execute_batch(SCHEMA)?,
            1 => {
                connection.execute_batch(MIGRATION_2)?;
                connection.execute_batch(MIGRATION_3)?;
            }
            2 => connection.execute_batch(MIGRATION_3)?,
            3 => {}
            version => {
                return Err(Error::Invalid(format!(
                    "database schema version {version} is newer than supported version 3"
                )));
            }
        }
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn open_session(&self, request: &OpenRequest, kernel_sha256: &str) -> Result<OpenOutcome> {
        require_text("objective", &request.objective)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(mut outcome) = duplicate_result::<OpenOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "session_opened",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }

        let project_id = find_or_create_project(&transaction, &workspace, now)?;
        let duplicate_objective = transaction
            .query_row(
                "SELECT session_id FROM sessions
                 WHERE project_id = ?1 AND status = 'open' AND abandoned = 0
                   AND lower(trim(objective)) = lower(trim(?2))
                 LIMIT 1",
                params![project_id, request.objective],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(session_id) = duplicate_objective {
            return Err(Error::Conflict(format!(
                "an active session already has this objective: {session_id}"
            )));
        }
        let context = recall_for_project(&transaction, &project_id, &workspace, 20)?;
        let session_id = Uuid::now_v7().to_string();
        transaction.execute(
            "INSERT INTO sessions
             (session_id, project_id, objective, status, opened_at_unix_ms,
              last_activity_at_unix_ms)
             VALUES (?1, ?2, ?3, 'open', ?4, ?4)",
            params![session_id, project_id, request.objective, now],
        )?;

        let outcome = OpenOutcome {
            session_id: session_id.clone(),
            project_id,
            kernel_sha256: kernel_sha256.to_owned(),
            context,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &session_id,
            "session_opened",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn recall(&self, request: &RecallRequest) -> Result<ContextCapsule> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let project_id = connection
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [&workspace],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let limit = request.limit.unwrap_or(20).clamp(1, 100);

        match project_id {
            Some(project_id) => {
                recall_for_project(&connection, &project_id, &workspace, i64::from(limit))
            }
            None => Ok(ContextCapsule {
                project_id: None,
                workspace,
                active_sessions: Vec::new(),
                recent_handoffs: Vec::new(),
                recent_records: Vec::new(),
            }),
        }
    }

    pub fn record(&self, request: &RecordRequest) -> Result<RecordOutcome> {
        require_text("session_id", &request.session_id)?;
        require_text("content", &request.content)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(mut outcome) = duplicate_result::<RecordOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "record_added",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        require_open_session(&transaction, &request.session_id)?;
        validate_record_links(&transaction, request)?;

        let record = DurableRecord {
            record_id: Uuid::now_v7().to_string(),
            session_id: request.session_id.clone(),
            kind: request.kind.clone(),
            content: request.content.clone(),
            evidence: request.evidence.clone(),
            supersedes_record_id: request.supersedes_record_id.clone(),
            verifies_effect_id: request.verifies_effect_id.clone(),
            created_at_unix_ms: now,
        };
        transaction.execute(
            "INSERT INTO records
             (record_id, session_id, kind, content, evidence, supersedes_record_id,
              verifies_effect_id, created_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.record_id,
                record.session_id,
                record.kind.as_str(),
                record.content,
                record.evidence,
                record.supersedes_record_id,
                record.verifies_effect_id,
                record.created_at_unix_ms
            ],
        )?;
        transaction.execute(
            "UPDATE sessions SET last_activity_at_unix_ms = ?1 WHERE session_id = ?2",
            params![now, request.session_id],
        )?;
        let outcome = RecordOutcome {
            record,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.session_id,
            "record_added",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn close_session(&self, request: &CloseRequest) -> Result<CloseOutcome> {
        require_text("session_id", &request.session_id)?;
        require_text("summary", &request.summary)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        if request.disposition == CloseDisposition::Handoff
            && request
                .next_action
                .as_deref()
                .is_none_or(|action| action.trim().is_empty())
        {
            return Err(Error::Invalid(
                "next_action is required for a handoff".to_owned(),
            ));
        }

        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<CloseOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "session_closed",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        require_open_session(&transaction, &request.session_id)?;
        if request.disposition == CloseDisposition::Completed {
            require_verified_effects(&transaction, &request.session_id)?;
        }
        transaction.execute(
            "UPDATE sessions
             SET status = ?1, closed_at_unix_ms = ?2, summary = ?3, next_action = ?4
             WHERE session_id = ?5 AND status = 'open'",
            params![
                request.disposition.as_str(),
                now,
                request.summary,
                request.next_action,
                request.session_id
            ],
        )?;

        let outcome = CloseOutcome {
            session_id: request.session_id.clone(),
            disposition: request.disposition.clone(),
            summary: request.summary.clone(),
            next_action: request.next_action.clone(),
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.session_id,
            "session_closed",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn reconcile(&self, request: &ReconcileRequest) -> Result<ReconcileOutcome> {
        require_text("idempotency_key", &request.idempotency_key)?;
        if !(60..=31_536_000).contains(&request.stale_after_seconds) {
            return Err(Error::Invalid(
                "stale_after_seconds must be between 60 and 31536000".to_owned(),
            ));
        }
        let workspace = canonical_workspace(&request.workspace)?;
        let now = unix_millis()?;
        let stale_millis = i64::try_from(request.stale_after_seconds)
            .map_err(|_| Error::Invalid("stale_after_seconds is too large".to_owned()))?
            .saturating_mul(1000);
        let cutoff = now.saturating_sub(stale_millis);
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(mut outcome) = duplicate_result::<ReconcileOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "sessions_reconciled",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }

        let project_id = transaction
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [&workspace],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(project_id) = project_id else {
            return Ok(ReconcileOutcome {
                project_id: None,
                abandoned_sessions: Vec::new(),
                duplicate: false,
            });
        };

        let abandoned_sessions = {
            let mut statement = transaction.prepare(
                "SELECT session_id, objective, last_activity_at_unix_ms
                 FROM sessions
                 WHERE project_id = ?1 AND status = 'open' AND abandoned = 0
                   AND last_activity_at_unix_ms < ?2
                 ORDER BY last_activity_at_unix_ms ASC",
            )?;
            statement
                .query_map(params![project_id, cutoff], |row| {
                    Ok(AbandonedSession {
                        session_id: row.get(0)?,
                        objective: row.get(1)?,
                        last_activity_at_unix_ms: row.get(2)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };

        for session in &abandoned_sessions {
            transaction.execute(
                "UPDATE sessions
                 SET abandoned = 1, closed_at_unix_ms = ?1,
                     summary = 'Automatically reconciled as stale; no completion was inferred.'
                 WHERE session_id = ?2 AND status = 'open' AND abandoned = 0",
                params![now, session.session_id],
            )?;
        }

        let outcome = ReconcileOutcome {
            project_id: Some(project_id.clone()),
            abandoned_sessions,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &project_id,
            "sessions_reconciled",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn create_task(&self, request: &TaskCreateRequest) -> Result<TaskOutcome> {
        require_text("session_id", &request.session_id)?;
        require_text("objective", &request.objective)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        require_nonempty_texts("acceptance_criteria", &request.acceptance_criteria)?;
        require_texts("write_scope", &request.write_scope)?;
        require_texts("depends_on", &request.depends_on)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(mut outcome) = duplicate_result::<TaskOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "task_created",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        require_open_session(&transaction, &request.session_id)?;
        let project_id = transaction.query_row(
            "SELECT project_id FROM sessions WHERE session_id = ?1",
            [&request.session_id],
            |row| row.get::<_, String>(0),
        )?;
        let duplicate_objective = transaction
            .query_row(
                "SELECT task_id FROM tasks
             WHERE project_id = ?1 AND status IN ('queued', 'leased')
               AND lower(trim(objective)) = lower(trim(?2))
             LIMIT 1",
                params![project_id, request.objective],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(task_id) = duplicate_objective {
            return Err(Error::Conflict(format!(
                "an active task already has this objective: {task_id}"
            )));
        }
        for dependency in &request.depends_on {
            let dependency_project = transaction
                .query_row(
                    "SELECT project_id FROM tasks WHERE task_id = ?1",
                    [dependency],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .ok_or_else(|| Error::NotFound(format!("task {dependency}")))?;
            if dependency_project != project_id {
                return Err(Error::Conflict(
                    "task dependencies must belong to the same project".to_owned(),
                ));
            }
        }

        let task_id = Uuid::now_v7().to_string();
        transaction.execute(
            "INSERT INTO tasks
             (task_id, project_id, session_id, objective, acceptance_criteria_json,
              write_scope_json, depends_on_json, status, created_at_unix_ms,
              updated_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'queued', ?8, ?8)",
            params![
                task_id,
                project_id,
                request.session_id,
                request.objective,
                serde_json::to_string(&request.acceptance_criteria)?,
                serde_json::to_string(&request.write_scope)?,
                serde_json::to_string(&request.depends_on)?,
                now,
            ],
        )?;
        let task = load_task(&transaction, &task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {task_id}")))?;
        let outcome = TaskOutcome {
            task,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &task_id,
            "task_created",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn list_tasks(&self, request: &TaskListRequest) -> Result<Vec<CoordinatedTask>> {
        let workspace = canonical_workspace(&request.workspace)?;
        let limit = request.limit.unwrap_or(50).clamp(1, 200);
        let connection = self.connection()?;
        let project_id = connection
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [&workspace],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(project_id) = project_id else {
            return Ok(Vec::new());
        };
        list_project_tasks(&connection, &project_id, i64::from(limit))
    }

    pub fn claim_task(&self, request: &TaskClaimRequest) -> Result<TaskOutcome> {
        require_text("task_id", &request.task_id)?;
        require_text("worker_id", &request.worker_id)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        if !(30..=86_400).contains(&request.lease_seconds) {
            return Err(Error::Invalid(
                "lease_seconds must be between 30 and 86400".to_owned(),
            ));
        }
        let now = unix_millis()?;
        let lease_millis = i64::try_from(request.lease_seconds)
            .map_err(|_| Error::Invalid("lease_seconds is too large".to_owned()))?
            .saturating_mul(1000);
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<TaskOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "task_claimed",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let task = load_task(&transaction, &request.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        match task.status {
            TaskStatus::Completed | TaskStatus::Cancelled => {
                return Err(Error::Conflict(format!(
                    "task {} is already {}",
                    request.task_id,
                    task.status.as_str()
                )));
            }
            TaskStatus::Leased
                if task
                    .lease_expires_at_unix_ms
                    .is_some_and(|expires| expires > now) =>
            {
                return Err(Error::Conflict(format!(
                    "task {} has an active lease",
                    request.task_id
                )));
            }
            TaskStatus::Queued | TaskStatus::Leased => {}
        }
        for dependency in &task.depends_on {
            let status = transaction
                .query_row(
                    "SELECT status FROM tasks WHERE task_id = ?1",
                    [dependency],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .ok_or_else(|| Error::NotFound(format!("task {dependency}")))?;
            if status != TaskStatus::Completed.as_str() {
                return Err(Error::Conflict(format!(
                    "dependency {dependency} is not completed"
                )));
            }
        }
        ensure_write_scope_available(&transaction, &task, now)?;
        let expires = now.saturating_add(lease_millis);
        transaction.execute(
            "UPDATE tasks
             SET status = 'leased', lease_owner = ?1, lease_expires_at_unix_ms = ?2,
                 updated_at_unix_ms = ?3
             WHERE task_id = ?4",
            params![request.worker_id, expires, now, request.task_id],
        )?;
        let task = load_task(&transaction, &request.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        let outcome = TaskOutcome {
            task,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.task_id,
            "task_claimed",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn complete_task(&self, request: &TaskCompleteRequest) -> Result<TaskOutcome> {
        require_text("task_id", &request.task_id)?;
        require_text("worker_id", &request.worker_id)?;
        require_text("outcome_summary", &request.outcome_summary)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<TaskOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "task_completed",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let task = load_task(&transaction, &request.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        if task.status != TaskStatus::Leased
            || task.lease_owner.as_deref() != Some(request.worker_id.as_str())
            || task
                .lease_expires_at_unix_ms
                .is_none_or(|expires| expires <= now)
        {
            return Err(Error::Conflict(
                "task completion requires the worker's active lease".to_owned(),
            ));
        }
        validate_criterion_evidence(&task.acceptance_criteria, &request.criterion_evidence)?;
        transaction.execute(
            "UPDATE tasks
             SET status = 'completed', outcome_summary = ?1, completion_evidence_json = ?2,
                 lease_expires_at_unix_ms = NULL, updated_at_unix_ms = ?3
             WHERE task_id = ?4",
            params![
                request.outcome_summary,
                serde_json::to_string(&request.criterion_evidence)?,
                now,
                request.task_id,
            ],
        )?;
        let task = load_task(&transaction, &request.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        let outcome = TaskOutcome {
            task,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.task_id,
            "task_completed",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn cancel_task(&self, request: &TaskCancelRequest) -> Result<TaskOutcome> {
        require_text("task_id", &request.task_id)?;
        require_text("reason", &request.reason)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<TaskOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "task_cancelled",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let task = load_task(&transaction, &request.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        if matches!(task.status, TaskStatus::Completed | TaskStatus::Cancelled) {
            return Err(Error::Conflict(format!(
                "task {} is already {}",
                request.task_id,
                task.status.as_str()
            )));
        }
        transaction.execute(
            "UPDATE tasks
             SET status = 'cancelled', outcome_summary = ?1, lease_expires_at_unix_ms = NULL,
                 updated_at_unix_ms = ?2
             WHERE task_id = ?3",
            params![request.reason, now, request.task_id],
        )?;
        let task = load_task(&transaction, &request.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        let outcome = TaskOutcome {
            task,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.task_id,
            "task_cancelled",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn event_count(&self) -> Result<u64> {
        let connection = self.connection()?;
        let count = connection.query_row("SELECT COUNT(*) FROM events", [], |row| {
            row.get::<_, u64>(0)
        })?;
        Ok(count)
    }

    pub fn stats(&self) -> Result<HubStats> {
        let connection = self.connection()?;
        Ok(HubStats {
            schema_version: connection
                .pragma_query_value(None, "user_version", |row| row.get(0))?,
            project_count: table_count(&connection, "projects", "1 = 1")?,
            open_session_count: table_count(
                &connection,
                "sessions",
                "status = 'open' AND abandoned = 0",
            )?,
            abandoned_session_count: table_count(&connection, "sessions", "abandoned = 1")?,
            record_count: table_count(&connection, "records", "1 = 1")?,
            event_count: table_count(&connection, "events", "1 = 1")?,
            queued_task_count: table_count(&connection, "tasks", "status = 'queued'")?,
            leased_task_count: table_count(&connection, "tasks", "status = 'leased'")?,
        })
    }

    pub fn export_project(&self, raw_workspace: &str) -> Result<ProjectExport> {
        let workspace = canonical_workspace(raw_workspace)?;
        let connection = self.connection()?;
        let project_id = connection
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [&workspace],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("project for workspace {workspace}")))?;

        let sessions = {
            let mut statement = connection.prepare(
                "SELECT session_id, objective, status, opened_at_unix_ms,
                        last_activity_at_unix_ms, abandoned, closed_at_unix_ms, summary,
                        next_action
                 FROM sessions WHERE project_id = ?1
                 ORDER BY opened_at_unix_ms ASC, session_id ASC",
            )?;
            statement
                .query_map([&project_id], |row| {
                    Ok(ExportSession {
                        session_id: row.get(0)?,
                        objective: row.get(1)?,
                        status: row.get(2)?,
                        opened_at_unix_ms: row.get(3)?,
                        last_activity_at_unix_ms: row.get(4)?,
                        abandoned: row.get(5)?,
                        closed_at_unix_ms: row.get(6)?,
                        summary: row.get(7)?,
                        next_action: row.get(8)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };

        let records = {
            let mut statement = connection.prepare(
                "SELECT records.record_id, records.session_id, records.kind, records.content,
                        records.evidence, records.supersedes_record_id,
                        records.verifies_effect_id, records.created_at_unix_ms
                 FROM records
                 JOIN sessions ON sessions.session_id = records.session_id
                 WHERE sessions.project_id = ?1
                 ORDER BY records.created_at_unix_ms ASC, records.record_id ASC",
            )?;
            statement
                .query_map([&project_id], |row| {
                    Ok(DurableRecord {
                        record_id: row.get(0)?,
                        session_id: row.get(1)?,
                        kind: parse_record_kind(row.get(2)?)?,
                        content: row.get(3)?,
                        evidence: row.get(4)?,
                        supersedes_record_id: row.get(5)?,
                        verifies_effect_id: row.get(6)?,
                        created_at_unix_ms: row.get(7)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };

        let tasks = list_project_tasks(&connection, &project_id, i64::MAX)?;

        let events = {
            let mut statement = connection.prepare(
                "SELECT sequence, event_id, idempotency_key, stream_id, kind, payload_json,
                        result_json, occurred_at_unix_ms
                 FROM events
                 WHERE stream_id = ?1 OR stream_id IN (
                     SELECT session_id FROM sessions WHERE project_id = ?1
                 )
                 ORDER BY sequence ASC",
            )?;
            let rows = statement.query_map([&project_id], |row| {
                Ok((
                    row.get::<_, u64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            })?;
            rows.map(|row| {
                let (sequence, event_id, idempotency_key, stream_id, kind, payload, result, at) =
                    row?;
                Ok(ExportEvent {
                    sequence,
                    event_id,
                    idempotency_key,
                    stream_id,
                    kind,
                    payload: serde_json::from_str(&payload)?,
                    result: serde_json::from_str(&result)?,
                    occurred_at_unix_ms: at,
                })
            })
            .collect::<Result<Vec<_>>>()?
        };

        Ok(ProjectExport {
            format_version: 1,
            exported_at_unix_ms: unix_millis()?,
            project_id,
            workspace,
            sessions,
            records,
            tasks,
            events,
        })
    }

    fn connection(&self) -> Result<Connection> {
        let connection = Connection::open(&self.path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        Ok(connection)
    }
}

fn canonical_workspace(raw: &str) -> Result<String> {
    require_text("workspace", raw)?;
    let path = fs::canonicalize(raw)?;
    if !path.is_dir() {
        return Err(Error::Invalid("workspace must be a directory".to_owned()));
    }
    Ok(path.to_string_lossy().into_owned())
}

fn require_text(name: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(Error::Invalid(format!("{name} must not be empty")))
    } else {
        Ok(())
    }
}

fn require_texts(name: &str, values: &[String]) -> Result<()> {
    for value in values {
        require_text(name, value)?;
    }
    let unique = values
        .iter()
        .map(|value| value.trim().to_lowercase())
        .collect::<std::collections::BTreeSet<_>>();
    if unique.len() != values.len() {
        Err(Error::Invalid(format!(
            "{name} must not contain duplicates"
        )))
    } else {
        Ok(())
    }
}

fn require_nonempty_texts(name: &str, values: &[String]) -> Result<()> {
    if values.is_empty() {
        return Err(Error::Invalid(format!("{name} must not be empty")));
    }
    require_texts(name, values)
}

fn unix_millis() -> Result<i64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| Error::Invalid(format!("system clock is before Unix epoch: {error}")))?;
    i64::try_from(duration.as_millis())
        .map_err(|_| Error::Invalid("current time exceeds the supported range".to_owned()))
}

fn find_or_create_project(
    transaction: &Transaction<'_>,
    workspace: &str,
    now: i64,
) -> Result<String> {
    if let Some(project_id) = transaction
        .query_row(
            "SELECT project_id FROM projects WHERE workspace = ?1",
            [workspace],
            |row| row.get::<_, String>(0),
        )
        .optional()?
    {
        return Ok(project_id);
    }

    let project_id = Uuid::now_v7().to_string();
    transaction.execute(
        "INSERT INTO projects (project_id, workspace, created_at_unix_ms) VALUES (?1, ?2, ?3)",
        params![project_id, workspace, now],
    )?;
    Ok(project_id)
}

fn require_open_session(transaction: &Transaction<'_>, session_id: &str) -> Result<()> {
    let state = transaction
        .query_row(
            "SELECT status, abandoned FROM sessions WHERE session_id = ?1",
            [session_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, bool>(1)?)),
        )
        .optional()?;
    match state {
        Some((_, true)) => Err(Error::Conflict(format!(
            "session {session_id} was abandoned without inferring completion"
        ))),
        Some((status, false)) if status == "open" => Ok(()),
        Some((status, false)) => Err(Error::Conflict(format!(
            "session {session_id} is already {status}"
        ))),
        None => Err(Error::NotFound(format!("session {session_id}"))),
    }
}

fn validate_record_links(transaction: &Transaction<'_>, request: &RecordRequest) -> Result<()> {
    match request.kind {
        RecordKind::Effect | RecordKind::Verification
            if request
                .evidence
                .as_deref()
                .is_none_or(|evidence| evidence.trim().is_empty()) =>
        {
            return Err(Error::Invalid(format!(
                "{} records require non-empty evidence",
                request.kind.as_str()
            )));
        }
        _ => {}
    }

    if let Some(record_id) = &request.supersedes_record_id {
        if !matches!(request.kind, RecordKind::Decision | RecordKind::Constraint) {
            return Err(Error::Invalid(
                "only decision or constraint records may supersede earlier records".to_owned(),
            ));
        }
        let target = record_identity(transaction, record_id)?;
        let Some((session_id, kind)) = target else {
            return Err(Error::NotFound(format!("record {record_id}")));
        };
        if kind != request.kind.as_str() {
            return Err(Error::Conflict(
                "a record may only supersede an earlier record of the same kind".to_owned(),
            ));
        }
        require_same_project(transaction, &request.session_id, &session_id)?;
        let already_superseded = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM records WHERE supersedes_record_id = ?1)",
            [record_id],
            |row| row.get::<_, bool>(0),
        )?;
        if already_superseded {
            return Err(Error::Conflict(format!(
                "record {record_id} is already superseded"
            )));
        }
    } else if matches!(request.kind, RecordKind::Decision | RecordKind::Constraint) {
        // A new independent decision or constraint is valid without a predecessor.
    }

    match (&request.kind, &request.verifies_effect_id) {
        (RecordKind::Verification, Some(effect_id)) => {
            let target = record_identity(transaction, effect_id)?;
            let Some((session_id, kind)) = target else {
                return Err(Error::NotFound(format!("record {effect_id}")));
            };
            if kind != RecordKind::Effect.as_str() {
                return Err(Error::Conflict(
                    "verification must reference an effect record".to_owned(),
                ));
            }
            if session_id != request.session_id {
                return Err(Error::Conflict(
                    "verification and effect must belong to the same session".to_owned(),
                ));
            }
            let already_verified = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM records WHERE verifies_effect_id = ?1)",
                [effect_id],
                |row| row.get::<_, bool>(0),
            )?;
            if already_verified {
                return Err(Error::Conflict(format!(
                    "effect {effect_id} is already verified"
                )));
            }
        }
        (RecordKind::Verification, None) => {
            return Err(Error::Invalid(
                "verification records must reference verifies_effect_id".to_owned(),
            ));
        }
        (_, Some(_)) => {
            return Err(Error::Invalid(
                "only verification records may set verifies_effect_id".to_owned(),
            ));
        }
        (_, None) => {}
    }
    Ok(())
}

fn record_identity(
    transaction: &Transaction<'_>,
    record_id: &str,
) -> Result<Option<(String, String)>> {
    Ok(transaction
        .query_row(
            "SELECT session_id, kind FROM records WHERE record_id = ?1",
            [record_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?)
}

fn require_same_project(
    transaction: &Transaction<'_>,
    left_session_id: &str,
    right_session_id: &str,
) -> Result<()> {
    let same = transaction.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM sessions left_session
            JOIN sessions right_session ON right_session.project_id = left_session.project_id
            WHERE left_session.session_id = ?1 AND right_session.session_id = ?2
         )",
        params![left_session_id, right_session_id],
        |row| row.get::<_, bool>(0),
    )?;
    if same {
        Ok(())
    } else {
        Err(Error::Conflict(
            "superseded record must belong to the same project".to_owned(),
        ))
    }
}

fn require_verified_effects(transaction: &Transaction<'_>, session_id: &str) -> Result<()> {
    let unverified = transaction.query_row(
        "SELECT COUNT(*)
         FROM records effects
         WHERE effects.session_id = ?1 AND effects.kind = 'effect'
           AND NOT EXISTS (
               SELECT 1 FROM records verifications
               WHERE verifications.verifies_effect_id = effects.record_id
                 AND verifications.kind = 'verification'
           )",
        [session_id],
        |row| row.get::<_, u64>(0),
    )?;
    if unverified == 0 {
        Ok(())
    } else {
        Err(Error::Conflict(format!(
            "session has {unverified} effect record(s) without linked verification"
        )))
    }
}

fn table_count(connection: &Connection, table: &str, predicate: &str) -> Result<u64> {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE {predicate}");
    Ok(connection.query_row(&sql, [], |row| row.get(0))?)
}

fn load_task(connection: &Connection, task_id: &str) -> Result<Option<CoordinatedTask>> {
    Ok(connection
        .query_row(
            "SELECT task_id, project_id, session_id, objective, acceptance_criteria_json,
                    write_scope_json, depends_on_json, status, lease_owner,
                    lease_expires_at_unix_ms, outcome_summary, completion_evidence_json,
                    created_at_unix_ms, updated_at_unix_ms
             FROM tasks WHERE task_id = ?1",
            [task_id],
            task_from_row,
        )
        .optional()?)
}

fn list_project_tasks(
    connection: &Connection,
    project_id: &str,
    limit: i64,
) -> Result<Vec<CoordinatedTask>> {
    let mut statement = connection.prepare(
        "SELECT task_id, project_id, session_id, objective, acceptance_criteria_json,
                write_scope_json, depends_on_json, status, lease_owner,
                lease_expires_at_unix_ms, outcome_summary, completion_evidence_json,
                created_at_unix_ms, updated_at_unix_ms
         FROM tasks WHERE project_id = ?1
         ORDER BY CASE status
                    WHEN 'leased' THEN 0
                    WHEN 'queued' THEN 1
                    WHEN 'completed' THEN 2
                    ELSE 3
                  END,
                  updated_at_unix_ms DESC
         LIMIT ?2",
    )?;
    Ok(statement
        .query_map(params![project_id, limit], task_from_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

fn task_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CoordinatedTask> {
    Ok(CoordinatedTask {
        task_id: row.get(0)?,
        project_id: row.get(1)?,
        session_id: row.get(2)?,
        objective: row.get(3)?,
        acceptance_criteria: json_column(row, 4)?,
        write_scope: json_column(row, 5)?,
        depends_on: json_column(row, 6)?,
        status: parse_task_status(row.get(7)?)?,
        lease_owner: row.get(8)?,
        lease_expires_at_unix_ms: row.get(9)?,
        outcome_summary: row.get(10)?,
        completion_evidence: json_column(row, 11)?,
        created_at_unix_ms: row.get(12)?,
        updated_at_unix_ms: row.get(13)?,
    })
}

fn json_column<T: DeserializeOwned>(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<T> {
    let json = row.get::<_, String>(index)?;
    serde_json::from_str(&json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn parse_task_status(value: String) -> rusqlite::Result<TaskStatus> {
    match value.as_str() {
        "queued" => Ok(TaskStatus::Queued),
        "leased" => Ok(TaskStatus::Leased),
        "completed" => Ok(TaskStatus::Completed),
        "cancelled" => Ok(TaskStatus::Cancelled),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn ensure_write_scope_available(
    transaction: &Transaction<'_>,
    requested: &CoordinatedTask,
    now: i64,
) -> Result<()> {
    if requested.write_scope.is_empty() {
        return Ok(());
    }
    let mut statement = transaction.prepare(
        "SELECT task_id, write_scope_json
         FROM tasks
         WHERE project_id = ?1 AND task_id != ?2 AND status = 'leased'
           AND lease_expires_at_unix_ms > ?3",
    )?;
    let active = statement.query_map(
        params![requested.project_id, requested.task_id, now],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )?;
    for row in active {
        let (task_id, scopes_json) = row?;
        let scopes: Vec<String> = serde_json::from_str(&scopes_json)?;
        if requested
            .write_scope
            .iter()
            .any(|left| scopes.iter().any(|right| write_scopes_overlap(left, right)))
        {
            return Err(Error::Conflict(format!(
                "task write scope overlaps active lease for task {task_id}"
            )));
        }
    }
    Ok(())
}

fn write_scopes_overlap(left: &str, right: &str) -> bool {
    let left = left.trim_end_matches('/');
    let right = right.trim_end_matches('/');
    left == right
        || left
            .strip_prefix(right)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || right
            .strip_prefix(left)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn validate_criterion_evidence(criteria: &[String], evidence: &[CriterionEvidence]) -> Result<()> {
    if criteria.len() != evidence.len() {
        return Err(Error::Invalid(
            "criterion_evidence must cover every acceptance criterion exactly once".to_owned(),
        ));
    }
    let expected = criteria
        .iter()
        .map(|criterion| criterion.trim())
        .collect::<std::collections::BTreeSet<_>>();
    let mut observed = std::collections::BTreeSet::new();
    for item in evidence {
        require_text("criterion", &item.criterion)?;
        require_text("criterion evidence", &item.evidence)?;
        if !observed.insert(item.criterion.trim()) {
            return Err(Error::Invalid(
                "criterion_evidence contains duplicate criteria".to_owned(),
            ));
        }
    }
    if observed != expected {
        Err(Error::Invalid(
            "criterion_evidence does not match the task acceptance criteria".to_owned(),
        ))
    } else {
        Ok(())
    }
}

fn duplicate_result<T: DeserializeOwned, P: Serialize>(
    transaction: &Transaction<'_>,
    idempotency_key: &str,
    expected_kind: &str,
    expected_payload: &P,
) -> Result<Option<T>> {
    let result = transaction
        .query_row(
            "SELECT kind, payload_json, result_json FROM events WHERE idempotency_key = ?1",
            [idempotency_key],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?;
    let Some((kind, payload_json, result_json)) = result else {
        return Ok(None);
    };
    let expected_payload_json = serde_json::to_string(expected_payload)?;
    if kind != expected_kind || payload_json != expected_payload_json {
        return Err(Error::Conflict(format!(
            "idempotency key {idempotency_key} was already used for a different request"
        )));
    }
    Ok(Some(serde_json::from_str(&result_json)?))
}

fn append_event<P: Serialize, O: Serialize>(
    transaction: &Transaction<'_>,
    idempotency_key: &str,
    stream_id: &str,
    kind: &str,
    payload: &P,
    outcome: &O,
    now: i64,
) -> Result<()> {
    transaction.execute(
        "INSERT INTO events
         (event_id, idempotency_key, stream_id, kind, payload_json, result_json, occurred_at_unix_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            Uuid::now_v7().to_string(),
            idempotency_key,
            stream_id,
            kind,
            serde_json::to_string(payload)?,
            serde_json::to_string(outcome)?,
            now
        ],
    )?;
    Ok(())
}

fn recall_for_project(
    connection: &Connection,
    project_id: &str,
    workspace: &str,
    limit: i64,
) -> Result<ContextCapsule> {
    let mut session_statement = connection.prepare(
        "SELECT session_id, objective, opened_at_unix_ms, last_activity_at_unix_ms
         FROM sessions
         WHERE project_id = ?1 AND status = 'open' AND abandoned = 0
         ORDER BY last_activity_at_unix_ms DESC
         LIMIT ?2",
    )?;
    let active_sessions = session_statement
        .query_map(params![project_id, limit], |row| {
            Ok(ActiveSession {
                session_id: row.get(0)?,
                objective: row.get(1)?,
                opened_at_unix_ms: row.get(2)?,
                last_activity_at_unix_ms: row.get(3)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut handoff_statement = connection.prepare(
        "SELECT session_id, summary, next_action, closed_at_unix_ms
         FROM sessions
         WHERE project_id = ?1 AND status = 'handoff'
         ORDER BY closed_at_unix_ms DESC
         LIMIT ?2",
    )?;
    let recent_handoffs = handoff_statement
        .query_map(params![project_id, limit], |row| {
            Ok(Handoff {
                session_id: row.get(0)?,
                summary: row.get(1)?,
                next_action: row.get(2)?,
                closed_at_unix_ms: row.get(3)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut record_statement = connection.prepare(
        "SELECT records.record_id, records.session_id, records.kind, records.content,
                records.evidence, records.supersedes_record_id, records.verifies_effect_id,
                records.created_at_unix_ms
         FROM records
         JOIN sessions ON sessions.session_id = records.session_id
         WHERE sessions.project_id = ?1
           AND NOT EXISTS (
               SELECT 1 FROM records replacements
               WHERE replacements.supersedes_record_id = records.record_id
           )
         ORDER BY CASE records.kind
                    WHEN 'decision' THEN 0
                    WHEN 'constraint' THEN 1
                    WHEN 'material_unknown' THEN 2
                    WHEN 'verification' THEN 3
                    WHEN 'effect' THEN 4
                    WHEN 'task_progress' THEN 5
                    ELSE 6
                  END,
                  records.created_at_unix_ms DESC
         LIMIT ?2",
    )?;
    let recent_records = record_statement
        .query_map(params![project_id, limit], |row| {
            let kind = parse_record_kind(row.get::<_, String>(2)?)?;
            Ok(DurableRecord {
                record_id: row.get(0)?,
                session_id: row.get(1)?,
                kind,
                content: row.get(3)?,
                evidence: row.get(4)?,
                supersedes_record_id: row.get(5)?,
                verifies_effect_id: row.get(6)?,
                created_at_unix_ms: row.get(7)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(ContextCapsule {
        project_id: Some(project_id.to_owned()),
        workspace: workspace.to_owned(),
        active_sessions,
        recent_handoffs,
        recent_records,
    })
}

fn parse_record_kind(value: String) -> rusqlite::Result<RecordKind> {
    match value.as_str() {
        "decision" => Ok(RecordKind::Decision),
        "constraint" => Ok(RecordKind::Constraint),
        "task_progress" => Ok(RecordKind::TaskProgress),
        "observation" => Ok(RecordKind::Observation),
        "effect" => Ok(RecordKind::Effect),
        "verification" => Ok(RecordKind::Verification),
        "material_unknown" => Ok(RecordKind::MaterialUnknown),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}
