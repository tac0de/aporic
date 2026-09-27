use rusqlite::{OptionalExtension, TransactionBehavior, params};
use uuid::Uuid;

use super::{
    Error, Result, Store, append_event, canonical_workspace, duplicate_result,
    require_open_session, require_text, unix_millis,
};
use crate::domain::{
    EvidenceGrade, EvidenceKind, Intake, IntakeCreateRequest, IntakeGetRequest, IntakeOutcome,
    TaskStatus,
};

const EVENT: &str = "intake_created";

impl Store {
    pub fn create_intake(&self, request: &IntakeCreateRequest) -> Result<IntakeOutcome> {
        for (name, value) in [
            ("session_id", &request.session_id),
            ("source_project", &request.source_project),
            (
                "reproduction_evidence_id",
                &request.reproduction_evidence_id,
            ),
            ("task_id", &request.task_id),
            ("idempotency_key", &request.idempotency_key),
        ] {
            require_text(name, value)?;
        }
        if request.source_project.len() > 512 {
            return Err(Error::Invalid("source_project exceeds 512 bytes".into()));
        }
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) =
            duplicate_result::<IntakeOutcome, _>(&tx, &request.idempotency_key, EVENT, request)?
        {
            outcome.intake = load_intake(&tx, &outcome.intake.intake_id)?
                .ok_or_else(|| Error::NotFound(format!("intake {}", outcome.intake.intake_id)))?;
            outcome.duplicate = true;
            return Ok(outcome);
        }
        require_open_session(&tx, &request.session_id)?;
        let project_id: String = tx.query_row(
            "SELECT project_id FROM sessions WHERE session_id = ?1",
            [&request.session_id],
            |row| row.get(0),
        )?;
        let task_project: String = tx
            .query_row(
                "SELECT project_id FROM tasks WHERE task_id = ?1",
                [&request.task_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        if task_project != project_id {
            return Err(Error::Conflict(
                "task belongs to another Aporic project".into(),
            ));
        }
        let evidence: (String, String) = tx.query_row(
            "SELECT s.project_id, e.kind FROM evidence_artifacts e JOIN sessions s ON s.session_id = e.session_id WHERE e.evidence_id = ?1",
            [&request.reproduction_evidence_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()?.ok_or_else(|| Error::NotFound(format!("evidence {}", request.reproduction_evidence_id)))?;
        if evidence.0 != project_id {
            return Err(Error::Conflict(
                "evidence belongs to another Aporic project".into(),
            ));
        }
        if !matches!(
            evidence.1.as_str(),
            "workspace_file" | "command_result" | "external_source"
        ) {
            return Err(Error::Invalid(
                "reproduction evidence must be a file, command result, or external source".into(),
            ));
        }
        let intake_id = Uuid::now_v7().to_string();
        let now = unix_millis()?;
        tx.execute("INSERT INTO intakes (intake_id, project_id, source_project, reproduction_evidence_id, task_id, created_at_unix_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![intake_id, project_id, request.source_project.trim(), request.reproduction_evidence_id, request.task_id, now])?;
        let outcome = IntakeOutcome {
            intake: load_intake(&tx, &intake_id)?
                .ok_or_else(|| Error::NotFound(format!("intake {intake_id}")))?,
            duplicate: false,
        };
        append_event(
            &tx,
            &request.idempotency_key,
            &intake_id,
            EVENT,
            request,
            &outcome,
            now,
        )?;
        tx.commit()?;
        Ok(outcome)
    }

    pub fn get_intake(&self, request: &IntakeGetRequest) -> Result<Intake> {
        require_text("intake_id", &request.intake_id)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let project_id: String = connection
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [&workspace],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| Error::NotFound("project".into()))?;
        let intake = load_intake(&connection, &request.intake_id)?
            .ok_or_else(|| Error::NotFound(format!("intake {}", request.intake_id)))?;
        let owner: String = connection.query_row(
            "SELECT project_id FROM intakes WHERE intake_id = ?1",
            [&request.intake_id],
            |row| row.get(0),
        )?;
        if owner != project_id {
            return Err(Error::NotFound(format!("intake {}", request.intake_id)));
        }
        Ok(intake)
    }
}

fn load_intake(connection: &rusqlite::Connection, id: &str) -> Result<Option<Intake>> {
    let row = connection.query_row(
        "SELECT i.intake_id, i.source_project, i.reproduction_evidence_id, e.kind, e.grade, e.locator, e.summary, e.content_sha256, i.task_id, t.status, i.created_at_unix_ms FROM intakes i JOIN evidence_artifacts e ON e.evidence_id = i.reproduction_evidence_id JOIN tasks t ON t.task_id = i.task_id WHERE i.intake_id = ?1",
        [id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?, row.get::<_, String>(5)?, row.get::<_, String>(6)?, row.get::<_, String>(7)?, row.get::<_, String>(8)?, row.get::<_, String>(9)?, row.get::<_, i64>(10)?)))
        .optional()?;
    row.map(
        |(
            intake_id,
            source_project,
            reproduction_evidence_id,
            kind,
            grade,
            evidence_locator,
            evidence_summary,
            evidence_sha256,
            task_id,
            status,
            created_at_unix_ms,
        )| {
            let evidence_kind = match kind.as_str() {
                "workspace_file" => EvidenceKind::WorkspaceFile,
                "command_result" => EvidenceKind::CommandResult,
                "external_source" => EvidenceKind::ExternalSource,
                _ => {
                    return Err(Error::Invalid(format!(
                        "unknown reproduction evidence kind: {kind}"
                    )));
                }
            };
            let evidence_grade = match grade.as_str() {
                "direct" => EvidenceGrade::Direct,
                "reported" => EvidenceGrade::Reported,
                "model_only" => EvidenceGrade::ModelOnly,
                _ => return Err(Error::Invalid(format!("unknown evidence grade: {grade}"))),
            };
            let task_status = match status.as_str() {
                "queued" => TaskStatus::Queued,
                "leased" => TaskStatus::Leased,
                "completed" => TaskStatus::Completed,
                "cancelled" => TaskStatus::Cancelled,
                _ => return Err(Error::Invalid(format!("unknown task status: {status}"))),
            };
            Ok(Intake {
                intake_id,
                source_project,
                reproduction_evidence_id,
                evidence_kind,
                evidence_grade,
                evidence_locator,
                evidence_summary,
                evidence_sha256,
                task_id,
                task_status,
                created_at_unix_ms,
            })
        },
    )
    .transpose()
}

pub(super) fn export_intakes(
    connection: &rusqlite::Connection,
    project_id: &str,
) -> Result<Vec<Intake>> {
    let mut stmt = connection.prepare(
        "SELECT intake_id FROM intakes WHERE project_id = ?1 ORDER BY created_at_unix_ms, intake_id",
    )?;
    let ids = stmt
        .query_map([project_id], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ids.iter()
        .map(|id| {
            load_intake(connection, id)?.ok_or_else(|| Error::NotFound(format!("intake {id}")))
        })
        .collect()
}
