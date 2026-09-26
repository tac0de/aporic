use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use uuid::Uuid;

use super::{
    Error, Result, Store, append_event, canonical_workspace, duplicate_result, load_task,
    require_text, unix_millis,
};
use crate::domain::{
    PromptComparison, PromptComparisonRequest, PromptCriterionAssessment, PromptCriterionRating,
    PromptTrial, PromptTrialOutcome, PromptTrialRequest, PromptVariantSummary,
};

impl Store {
    pub fn record_prompt_trial(&self, request: &PromptTrialRequest) -> Result<PromptTrialOutcome> {
        for (name, value) in [
            ("brief_receipt_id", &request.brief_receipt_id),
            ("response_sha256", &request.response_sha256),
            ("idempotency_key", &request.idempotency_key),
        ] {
            require_text(name, value)?;
        }
        if request.response_sha256.len() != 64
            || !request
                .response_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(Error::Invalid(
                "response_sha256 must be a lowercase SHA-256 hex digest".to_owned(),
            ));
        }
        if let Some(model) = &request.reported_model
            && (model.is_empty()
                || model.len() > 128
                || !model.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'/')
                }))
        {
            return Err(Error::Invalid("reported_model is invalid".to_owned()));
        }
        let workspace = canonical_workspace(&request.workspace)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<PromptTrialOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "prompt_trial_recorded",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let receipt = transaction
            .query_row(
                "SELECT project_id, task_id, template_id FROM task_brief_receipts
                 WHERE receipt_id = ?1",
                [&request.brief_receipt_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| Error::NotFound("task brief receipt".to_owned()))?;
        let project_id: Option<String> = transaction
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [&workspace],
                |row| row.get(0),
            )
            .optional()?;
        if project_id.as_deref() != Some(receipt.0.as_str()) {
            return Err(Error::NotFound("task brief receipt".to_owned()));
        }
        let task = load_task(&transaction, &receipt.1)?
            .ok_or_else(|| Error::NotFound(format!("task {}", receipt.1)))?;
        validate_assessments(&transaction, &task, &request.assessments)?;
        let response_file_verified = if let Some(evidence_id) = &request.response_evidence_id {
            let evidence = transaction
                .query_row(
                    "SELECT s.project_id, e.kind, e.grade, e.content_sha256
                     FROM evidence_artifacts e JOIN sessions s ON s.session_id = e.session_id
                     WHERE e.evidence_id = ?1",
                    [evidence_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                        ))
                    },
                )
                .optional()?
                .ok_or_else(|| Error::NotFound(format!("evidence {evidence_id}")))?;
            if evidence.0 != receipt.0
                || evidence.1 != "workspace_file"
                || evidence.2 != "direct"
                || evidence.3 != request.response_sha256
            {
                return Err(Error::Conflict(
                    "response evidence must be a matching direct workspace file".to_owned(),
                ));
            }
            true
        } else {
            false
        };
        let trial = PromptTrial {
            trial_id: Uuid::now_v7().to_string(),
            task_id: task.task_id,
            brief_receipt_id: request.brief_receipt_id.clone(),
            template_id: receipt.2,
            response_sha256: request.response_sha256.clone(),
            response_evidence_id: request.response_evidence_id.clone(),
            response_file_verified,
            reported_model: request.reported_model.clone(),
            assessments: request.assessments.clone(),
            created_at_unix_ms: now,
            rating_provenance: "host_reported".to_owned(),
        };
        transaction.execute(
            "INSERT INTO prompt_trials
             (trial_id, project_id, task_id, brief_receipt_id, idempotency_key,
              template_id, response_sha256, response_evidence_id, response_file_verified,
              reported_model, assessments_json, created_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                trial.trial_id,
                receipt.0,
                trial.task_id,
                trial.brief_receipt_id,
                request.idempotency_key,
                trial.template_id,
                trial.response_sha256,
                trial.response_evidence_id,
                trial.response_file_verified,
                trial.reported_model,
                serde_json::to_string(&trial.assessments)?,
                now,
            ],
        )?;
        let outcome = PromptTrialOutcome {
            trial,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &outcome.trial.task_id,
            "prompt_trial_recorded",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn compare_prompt_trials(
        &self,
        request: &PromptComparisonRequest,
    ) -> Result<PromptComparison> {
        require_text("task_id", &request.task_id)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let task = load_task(&connection, &request.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        let project_id: Option<String> = connection
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [&workspace],
                |row| row.get(0),
            )
            .optional()?;
        if project_id.as_deref() != Some(task.project_id.as_str()) {
            return Err(Error::NotFound(format!("task {}", request.task_id)));
        }
        let mut variants = BTreeMap::<(String, u32, String), PromptVariantSummary>::new();
        let mut receipt_statement = connection.prepare(
            "SELECT template_id, template_version, template_sha256, COUNT(*)
             FROM task_brief_receipts WHERE task_id = ?1
             GROUP BY template_id, template_version, template_sha256
             ORDER BY template_id, template_version, template_sha256",
        )?;
        let receipts = receipt_statement.query_map([&request.task_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, u32>(3)?,
            ))
        })?;
        for receipt in receipts {
            let (template_id, template_version, template_sha256, brief_count) = receipt?;
            variants.insert(
                (
                    template_id.clone(),
                    template_version,
                    template_sha256.clone(),
                ),
                PromptVariantSummary {
                    template_id,
                    template_version,
                    template_sha256,
                    brief_count,
                    trial_count: 0,
                    reported_met: 0,
                    reported_unmet: 0,
                    reported_unknown: 0,
                    associated_verified_claims: 0,
                    response_files_verified: 0,
                },
            );
        }
        let mut trial_statement = connection.prepare(
            "SELECT t.trial_id, t.task_id, t.brief_receipt_id, t.template_id, t.response_sha256,
                    t.response_evidence_id, t.response_file_verified, t.reported_model,
                    t.assessments_json, t.created_at_unix_ms,
                    b.template_id, b.template_version, b.template_sha256
             FROM prompt_trials t JOIN task_brief_receipts b ON b.receipt_id = t.brief_receipt_id
             WHERE t.task_id = ?1 ORDER BY t.created_at_unix_ms, t.trial_id",
        )?;
        let trials = trial_statement.query_map([&request.task_id], |row| {
            Ok((
                trial_from_row(row)?,
                row.get::<_, String>(10)?,
                row.get::<_, u32>(11)?,
                row.get::<_, String>(12)?,
            ))
        })?;
        for trial in trials {
            let (trial, template_id, template_version, template_sha256) = trial?;
            if trial.template_id != template_id {
                return Err(Error::Invalid(
                    "trial template does not match its receipt".to_owned(),
                ));
            }
            let Some(summary) = variants.get_mut(&(template_id, template_version, template_sha256))
            else {
                return Err(Error::Invalid(
                    "trial refers to an unknown template".to_owned(),
                ));
            };
            summary.trial_count = summary.trial_count.saturating_add(1);
            summary.response_files_verified += u32::from(trial.response_file_verified);
            for assessment in trial.assessments {
                match assessment.rating {
                    PromptCriterionRating::Met => summary.reported_met += 1,
                    PromptCriterionRating::Unmet => summary.reported_unmet += 1,
                    PromptCriterionRating::Unknown => summary.reported_unknown += 1,
                }
                if assessment.verified_claim_id.is_some() {
                    summary.associated_verified_claims += 1;
                }
            }
        }
        Ok(PromptComparison {
            task_id: request.task_id.clone(),
            variants: variants.into_values().collect(),
            rating_provenance: "host_reported".to_owned(),
            attribution_verified: false,
            advisory: true,
        })
    }

    pub fn list_prompt_trials(&self, raw_workspace: &str) -> Result<Vec<PromptTrial>> {
        let workspace = canonical_workspace(raw_workspace)?;
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT t.trial_id, t.task_id, t.brief_receipt_id, t.template_id,
                    t.response_sha256, t.response_evidence_id, t.response_file_verified,
                    t.reported_model, t.assessments_json, t.created_at_unix_ms
             FROM prompt_trials t JOIN projects p ON p.project_id = t.project_id
             WHERE p.workspace = ?1 ORDER BY t.created_at_unix_ms, t.trial_id",
        )?;
        let rows = statement.query_map([workspace], trial_from_row)?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
}

fn validate_assessments(
    connection: &Connection,
    task: &crate::domain::CoordinatedTask,
    assessments: &[PromptCriterionAssessment],
) -> Result<()> {
    if assessments.is_empty() || assessments.len() != task.acceptance_criteria.len() {
        return Err(Error::Invalid(
            "assessments must cover every task acceptance criterion exactly once".to_owned(),
        ));
    }
    if serde_json::to_vec(assessments)?.len() > 32_768 {
        return Err(Error::Invalid("assessments exceed 32768 bytes".to_owned()));
    }
    let mut seen = BTreeSet::new();
    for assessment in assessments {
        let index = usize::try_from(assessment.criterion_index)
            .map_err(|_| Error::Invalid("criterion index is out of range".to_owned()))?;
        if task.acceptance_criteria.get(index) != Some(&assessment.criterion) {
            return Err(Error::Invalid(
                "assessment does not match task criterion index".to_owned(),
            ));
        }
        if !seen.insert(assessment.criterion_index) {
            return Err(Error::Invalid(
                "duplicate assessed criterion index".to_owned(),
            ));
        }
        if let Some(claim_id) = &assessment.verified_claim_id {
            if assessment.rating != PromptCriterionRating::Met {
                return Err(Error::Invalid(
                    "a verified claim can only accompany a met rating".to_owned(),
                ));
            }
            let claim = connection
                .query_row(
                    "SELECT c.status, s.project_id, c.statement
                     FROM claims c JOIN sessions s ON s.session_id = c.session_id
                     WHERE c.claim_id = ?1",
                    [claim_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()?
                .ok_or_else(|| Error::NotFound(format!("claim {claim_id}")))?;
            if claim.0 != "verified"
                || claim.1 != task.project_id
                || claim.2 != assessment.criterion
            {
                return Err(Error::Conflict(
                    "verified claim must exactly match the task criterion and workspace".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn trial_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PromptTrial> {
    let assessments_json: String = row.get(8)?;
    let assessments = serde_json::from_str(&assessments_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(PromptTrial {
        trial_id: row.get(0)?,
        task_id: row.get(1)?,
        brief_receipt_id: row.get(2)?,
        template_id: row.get(3)?,
        response_sha256: row.get(4)?,
        response_evidence_id: row.get(5)?,
        response_file_verified: row.get(6)?,
        reported_model: row.get(7)?,
        assessments,
        created_at_unix_ms: row.get(9)?,
        rating_provenance: "host_reported".to_owned(),
    })
}
