use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::{
    AbandonedSession, ActiveSession, ClaimOutcome, ClaimRequest, ClaimStatus, CloseDisposition,
    CloseOutcome, CloseRequest, CommandSpec, CommandSpecOutcome, CommandSpecRequest, Consequence,
    ContextCapsule, CoordinatedTask, CriterionProof, DissentAssessment, DissentRequest,
    DurableRecord, EpistemicClaim, EvidenceArtifact, EvidenceGrade, EvidenceKind, EvidenceOutcome,
    EvidenceRequest, ExecutionFinish, ExecutionGetRequest, ExecutionListRequest, ExecutionOutcome,
    ExecutionReceipt, ExecutionReplayAudit, ExecutionRun, ExecutionStart, ExecutionStatus,
    ExportEvent, ExportSession, Handoff, HubStats, OpenOutcome, OpenRequest, ProjectExport,
    RecallRequest, ReceiptArtifact, ReconcileOutcome, ReconcileRequest, RecordKind, RecordOutcome,
    RecordRequest, TaskCancelRequest, TaskClaimRequest, TaskCompleteRequest, TaskCreateRequest,
    TaskListRequest, TaskOutcome, TaskStatus, workspace_file_claim,
};

const SCHEMA: &str = include_str!("../../../migrations/0001_initial.sql");
const MIGRATION_2: &str = include_str!("../../../migrations/0002_continuity_hardening.sql");
const MIGRATION_3: &str = include_str!("../../../migrations/0003_coordination.sql");
const MIGRATION_4: &str = include_str!("../../../migrations/0004_epistemic_gate.sql");
const MIGRATION_5: &str = include_str!("../../../migrations/0005_verifiable_execution.sql");

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
                connection.execute_batch(MIGRATION_4)?;
                connection.execute_batch(MIGRATION_5)?;
            }
            2 => {
                connection.execute_batch(MIGRATION_3)?;
                connection.execute_batch(MIGRATION_4)?;
                connection.execute_batch(MIGRATION_5)?;
            }
            3 => {
                connection.execute_batch(MIGRATION_4)?;
                connection.execute_batch(MIGRATION_5)?;
            }
            4 => connection.execute_batch(MIGRATION_5)?,
            5 => {}
            version => {
                return Err(Error::Invalid(format!(
                    "database schema version {version} is newer than supported version 5"
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

    pub fn add_evidence(&self, request: &EvidenceRequest) -> Result<EvidenceOutcome> {
        require_text("session_id", &request.session_id)?;
        require_text("locator", &request.locator)?;
        require_text("summary", &request.summary)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<EvidenceOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "evidence_added",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        require_open_session(&transaction, &request.session_id)?;
        let (grade, digest) = validate_evidence(&transaction, request)?;
        let evidence = EvidenceArtifact {
            evidence_id: Uuid::now_v7().to_string(),
            session_id: request.session_id.clone(),
            kind: request.kind.clone(),
            grade,
            locator: request.locator.clone(),
            summary: request.summary.clone(),
            content_sha256: digest,
            created_at_unix_ms: now,
        };
        transaction.execute(
            "INSERT INTO evidence_artifacts
             (evidence_id, session_id, kind, grade, locator, summary, content_sha256, created_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![evidence.evidence_id, evidence.session_id, evidence.kind.as_str(),
                evidence.grade.as_str(), evidence.locator, evidence.summary,
                evidence.content_sha256, evidence.created_at_unix_ms],
        )?;
        let outcome = EvidenceOutcome {
            evidence,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.session_id,
            "evidence_added",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn assert_claim(&self, request: &ClaimRequest) -> Result<ClaimOutcome> {
        require_text("session_id", &request.session_id)?;
        require_text("statement", &request.statement)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        require_texts("evidence_ids", &request.evidence_ids)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<ClaimOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "claim_asserted",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        require_open_session(&transaction, &request.session_id)?;
        validate_claim(&transaction, request)?;
        let claim = EpistemicClaim {
            claim_id: Uuid::now_v7().to_string(),
            session_id: request.session_id.clone(),
            status: request.status.clone(),
            statement: request.statement.clone(),
            material: request.material,
            evidence_ids: request.evidence_ids.clone(),
            receipt_ids: Vec::new(),
            supersedes_claim_id: request.supersedes_claim_id.clone(),
            created_at_unix_ms: now,
        };
        transaction.execute(
            "INSERT INTO claims
             (claim_id, session_id, status, statement, material, supersedes_claim_id, created_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![claim.claim_id, claim.session_id, claim.status.as_str(), claim.statement,
                claim.material, claim.supersedes_claim_id, claim.created_at_unix_ms],
        )?;
        for evidence_id in &claim.evidence_ids {
            transaction.execute(
                "INSERT INTO claim_evidence (claim_id, evidence_id) VALUES (?1, ?2)",
                params![claim.claim_id, evidence_id],
            )?;
        }
        let outcome = ClaimOutcome {
            claim,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.session_id,
            "claim_asserted",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn assess_dissent(&self, request: &DissentRequest) -> Result<DissentAssessment> {
        require_text("session_id", &request.session_id)?;
        require_text("target_claim_id", &request.target_claim_id)?;
        require_text("actionable_change", &request.actionable_change)?;
        require_texts("evidence_ids", &request.evidence_ids)?;
        let connection = self.connection()?;
        let target_session = connection
            .query_row(
                "SELECT session_id FROM claims WHERE claim_id = ?1",
                [&request.target_claim_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("claim {}", request.target_claim_id)))?;
        require_same_project_connection(&connection, &request.session_id, &target_session)?;
        let consequential = matches!(
            request.consequence,
            Consequence::High | Consequence::Critical
        );
        let direct = evidence_grades(&connection, &request.session_id, &request.evidence_ids)?
            .contains(&EvidenceGrade::Direct);
        let mut reasons = Vec::new();
        if !consequential {
            reasons.push("consequence_below_high".to_owned());
        }
        if !direct {
            reasons.push("no_direct_evidence".to_owned());
        }
        if reasons.is_empty() {
            reasons.push("material_and_directly_evidenced".to_owned());
        }
        Ok(DissentAssessment {
            surface: consequential && direct,
            reasons,
        })
    }

    pub fn register_command_spec(
        &self,
        request: &CommandSpecRequest,
    ) -> Result<CommandSpecOutcome> {
        require_text("session_id", &request.session_id)?;
        require_text("program", &request.program)?;
        require_text("workspace_relative_cwd", &request.workspace_relative_cwd)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        for argument in &request.args {
            if argument.contains('\0') {
                return Err(Error::Invalid("args must not contain NUL bytes".to_owned()));
            }
        }
        require_texts("artifact_paths", &request.artifact_paths)?;
        if !(1..=3_600).contains(&request.timeout_seconds) {
            return Err(Error::Invalid(
                "timeout_seconds must be between 1 and 3600".to_owned(),
            ));
        }
        validate_relative_path("workspace_relative_cwd", &request.workspace_relative_cwd)?;
        for path in &request.artifact_paths {
            validate_relative_path("artifact path", path)?;
        }
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<CommandSpecOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "command_spec_registered",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        require_open_session(&transaction, &request.session_id)?;
        let workspace = workspace_for_session(&transaction, &request.session_id)?;
        let cwd = fs::canonicalize(Path::new(&workspace).join(&request.workspace_relative_cwd))?;
        if !cwd.is_dir() || !cwd.starts_with(Path::new(&workspace)) {
            return Err(Error::Invalid(
                "workspace_relative_cwd must resolve to a directory inside the workspace"
                    .to_owned(),
            ));
        }
        let canonical = serde_json::json!({
            "program": request.program,
            "args": request.args,
            "workspace_relative_cwd": request.workspace_relative_cwd,
            "expected_exit_code": request.expected_exit_code,
            "timeout_seconds": request.timeout_seconds,
            "artifact_paths": request.artifact_paths,
        });
        let canonical_sha256 = format!("{:x}", Sha256::digest(serde_json::to_vec(&canonical)?));
        let spec = CommandSpec {
            spec_id: Uuid::now_v7().to_string(),
            session_id: request.session_id.clone(),
            workspace,
            program: request.program.clone(),
            args: request.args.clone(),
            workspace_relative_cwd: request.workspace_relative_cwd.clone(),
            expected_exit_code: request.expected_exit_code,
            timeout_seconds: request.timeout_seconds,
            artifact_paths: request.artifact_paths.clone(),
            success_claim: command_success_claim(&canonical_sha256, request.expected_exit_code),
            canonical_sha256,
            created_at_unix_ms: now,
        };
        transaction.execute(
            "INSERT INTO verification_specs
             (spec_id, session_id, program, args_json, workspace_relative_cwd,
              expected_exit_code, timeout_seconds, artifact_paths_json,
              canonical_sha256, created_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                spec.spec_id,
                spec.session_id,
                spec.program,
                serde_json::to_string(&spec.args)?,
                spec.workspace_relative_cwd,
                spec.expected_exit_code,
                spec.timeout_seconds,
                serde_json::to_string(&spec.artifact_paths)?,
                spec.canonical_sha256,
                spec.created_at_unix_ms,
            ],
        )?;
        let outcome = CommandSpecOutcome {
            spec,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.session_id,
            "command_spec_registered",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub(crate) fn start_execution(&self, spec_id: &str) -> Result<ExecutionStart> {
        require_text("spec_id", spec_id)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let spec = load_command_spec(&transaction, spec_id)?
            .ok_or_else(|| Error::NotFound(format!("verification spec {spec_id}")))?;
        require_open_session(&transaction, &spec.session_id)?;
        let running = transaction
            .query_row(
                "SELECT run_id FROM execution_runs WHERE spec_id = ?1 AND status = 'running'",
                [spec_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(run_id) = running {
            return Err(Error::Conflict(format!(
                "verification spec {spec_id} already has running execution {run_id}"
            )));
        }
        let prior = transaction
            .query_row(
                "SELECT run_id, attempt FROM execution_runs WHERE spec_id = ?1
             ORDER BY attempt DESC LIMIT 1",
                [spec_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?)),
            )
            .optional()?;
        let run = ExecutionRun {
            run_id: Uuid::now_v7().to_string(),
            spec_id: spec_id.to_owned(),
            session_id: spec.session_id.clone(),
            status: ExecutionStatus::Running,
            attempt: prior.as_ref().map_or(1, |(_, attempt)| attempt + 1),
            retry_of_run_id: prior.map(|(run_id, _)| run_id),
            started_at_unix_ms: now,
            finished_at_unix_ms: None,
        };
        transaction.execute(
            "INSERT INTO execution_runs
             (run_id, spec_id, session_id, status, attempt, retry_of_run_id,
              started_at_unix_ms)
             VALUES (?1, ?2, ?3, 'running', ?4, ?5, ?6)",
            params![
                run.run_id,
                run.spec_id,
                run.session_id,
                run.attempt,
                run.retry_of_run_id,
                run.started_at_unix_ms
            ],
        )?;
        let outcome = ExecutionStart { spec, run };
        append_event(
            &transaction,
            &format!("execution-start:{}", outcome.run.run_id),
            &outcome.run.run_id,
            "execution_run_started",
            &spec_id,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub(crate) fn finish_execution(&self, finish: &ExecutionFinish) -> Result<ExecutionOutcome> {
        if !matches!(
            finish.status,
            ExecutionStatus::Succeeded | ExecutionStatus::Failed | ExecutionStatus::TimedOut
        ) {
            return Err(Error::Invalid(
                "runner may only finish as succeeded, failed, or timed_out".to_owned(),
            ));
        }
        require_text("run_id", &finish.run_id)?;
        require_text("termination", &finish.termination)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut run = load_execution_run(&transaction, &finish.run_id)?
            .ok_or_else(|| Error::NotFound(format!("execution run {}", finish.run_id)))?;
        if run.status != ExecutionStatus::Running {
            return Err(Error::Conflict(format!(
                "execution run {} is already {}",
                run.run_id,
                run.status.as_str()
            )));
        }
        let spec = load_command_spec(&transaction, &run.spec_id)?
            .ok_or_else(|| Error::NotFound(format!("verification spec {}", run.spec_id)))?;
        if finish.status == ExecutionStatus::Succeeded
            && (finish.exit_code != Some(spec.expected_exit_code) || finish.termination != "exited")
        {
            return Err(Error::Invalid(
                "succeeded execution must exit normally with the expected exit code".to_owned(),
            ));
        }
        if finish.status == ExecutionStatus::Succeeded
            && (finish.artifacts.len() != spec.artifact_paths.len()
                || !spec.artifact_paths.iter().all(|expected| {
                    finish
                        .artifacts
                        .iter()
                        .any(|actual| &actual.workspace_relative_path == expected)
                }))
        {
            return Err(Error::Invalid(
                "receipt artifacts must exactly cover the command specification".to_owned(),
            ));
        }
        let receipt = ExecutionReceipt {
            receipt_id: Uuid::now_v7().to_string(),
            run_id: run.run_id.clone(),
            command_spec_sha256: spec.canonical_sha256.clone(),
            resolved_executable: finish.resolved_executable.clone(),
            exit_code: finish.exit_code,
            termination: finish.termination.clone(),
            stdout_sha256: finish.stdout_sha256.clone(),
            stdout_bytes: finish.stdout_bytes,
            stderr_sha256: finish.stderr_sha256.clone(),
            stderr_bytes: finish.stderr_bytes,
            git_head_before: finish.git_head_before.clone(),
            git_head_after: finish.git_head_after.clone(),
            worktree_state_before_sha256: finish.worktree_state_before_sha256.clone(),
            worktree_state_after_sha256: finish.worktree_state_after_sha256.clone(),
            created_at_unix_ms: now,
        };
        transaction.execute(
            "INSERT INTO execution_receipts
             (receipt_id, run_id, command_spec_sha256, resolved_executable, exit_code,
              termination, stdout_sha256, stdout_bytes, stderr_sha256, stderr_bytes,
              git_head_before, git_head_after, worktree_state_before_sha256,
              worktree_state_after_sha256, created_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                receipt.receipt_id,
                receipt.run_id,
                receipt.command_spec_sha256,
                receipt.resolved_executable,
                receipt.exit_code,
                receipt.termination,
                receipt.stdout_sha256,
                receipt.stdout_bytes,
                receipt.stderr_sha256,
                receipt.stderr_bytes,
                receipt.git_head_before,
                receipt.git_head_after,
                receipt.worktree_state_before_sha256,
                receipt.worktree_state_after_sha256,
                receipt.created_at_unix_ms
            ],
        )?;
        let mut artifacts = Vec::new();
        for item in &finish.artifacts {
            let artifact = ReceiptArtifact {
                artifact_id: Uuid::now_v7().to_string(),
                receipt_id: receipt.receipt_id.clone(),
                workspace_relative_path: item.workspace_relative_path.clone(),
                sha256: item.sha256.clone(),
                byte_length: item.byte_length,
            };
            transaction.execute(
                "INSERT INTO receipt_artifacts
                 (artifact_id, receipt_id, workspace_relative_path, sha256, byte_length)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    artifact.artifact_id,
                    artifact.receipt_id,
                    artifact.workspace_relative_path,
                    artifact.sha256,
                    artifact.byte_length
                ],
            )?;
            artifacts.push(artifact);
        }
        run.status = finish.status.clone();
        run.finished_at_unix_ms = Some(now);
        transaction.execute(
            "UPDATE execution_runs SET status = ?1, finished_at_unix_ms = ?2 WHERE run_id = ?3",
            params![run.status.as_str(), now, run.run_id],
        )?;
        let verified_claim_id = if run.status == ExecutionStatus::Succeeded
            && finish.exit_code == Some(spec.expected_exit_code)
        {
            let claim_id = Uuid::now_v7().to_string();
            transaction.execute(
                "INSERT INTO claims
                 (claim_id, session_id, status, statement, material, created_at_unix_ms)
                 VALUES (?1, ?2, 'verified', ?3, 1, ?4)",
                params![claim_id, spec.session_id, spec.success_claim, now],
            )?;
            transaction.execute(
                "INSERT INTO claim_receipts (claim_id, receipt_id) VALUES (?1, ?2)",
                params![claim_id, receipt.receipt_id],
            )?;
            Some(claim_id)
        } else {
            None
        };
        let outcome = ExecutionOutcome {
            run,
            receipt: Some(receipt),
            artifacts,
            verified_claim_id,
        };
        append_event(
            &transaction,
            &format!("execution-finish:{}", finish.run_id),
            &finish.run_id,
            "execution_run_finished",
            finish,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn list_executions(&self, request: &ExecutionListRequest) -> Result<Vec<ExecutionRun>> {
        let workspace = canonical_workspace(&request.workspace)?;
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
        load_project_runs(
            &connection,
            &project_id,
            i64::from(request.limit.unwrap_or(20).clamp(1, 100)),
        )
    }

    pub fn get_execution(&self, request: &ExecutionGetRequest) -> Result<ExecutionOutcome> {
        require_text("run_id", &request.run_id)?;
        let connection = self.connection()?;
        load_execution_outcome(&connection, &request.run_id)?
            .ok_or_else(|| Error::NotFound(format!("execution run {}", request.run_id)))
    }

    pub fn audit_execution_replay(&self) -> Result<ExecutionReplayAudit> {
        let connection = self.connection()?;
        let mut replayed = std::collections::BTreeMap::new();
        let mut statement = connection.prepare(
            "SELECT kind, result_json FROM events
             WHERE kind IN ('execution_run_started', 'execution_run_finished',
                            'execution_run_interrupted')
             ORDER BY sequence ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (kind, json) = row?;
            let run = match kind.as_str() {
                "execution_run_started" => serde_json::from_str::<ExecutionStart>(&json)?.run,
                "execution_run_finished" => serde_json::from_str::<ExecutionOutcome>(&json)?.run,
                "execution_run_interrupted" => serde_json::from_str::<ExecutionRun>(&json)?,
                _ => unreachable!(),
            };
            replayed.insert(run.run_id.clone(), run);
        }
        let mut projected = std::collections::BTreeMap::new();
        let mut statement = connection.prepare(
            "SELECT run_id, spec_id, session_id, status, attempt, retry_of_run_id,
                    started_at_unix_ms, finished_at_unix_ms
             FROM execution_runs ORDER BY run_id ASC",
        )?;
        for run in statement.query_map([], execution_run_from_row)? {
            let run = run?;
            projected.insert(run.run_id.clone(), run);
        }
        let mut mismatches = Vec::new();
        for run_id in replayed.keys().chain(projected.keys()) {
            if replayed.get(run_id) != projected.get(run_id) && !mismatches.contains(run_id) {
                mismatches.push(run_id.clone());
            }
        }
        Ok(ExecutionReplayAudit {
            replayed_run_count: replayed.len() as u64,
            projection_run_count: projected.len() as u64,
            mismatches,
        })
    }

    pub fn reconcile_executions(&self, stale_after_seconds: u64) -> Result<u64> {
        if !(1..=31_536_000).contains(&stale_after_seconds) {
            return Err(Error::Invalid(
                "stale_after_seconds must be between 1 and 31536000".to_owned(),
            ));
        }
        let now = unix_millis()?;
        let cutoff = now.saturating_sub(
            i64::try_from(stale_after_seconds)
                .map_err(|_| Error::Invalid("stale_after_seconds is too large".to_owned()))?
                .saturating_mul(1000),
        );
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let stale = {
            let mut statement = transaction.prepare(
                "SELECT run_id FROM execution_runs
                 WHERE status = 'running' AND started_at_unix_ms < ?1
                 ORDER BY started_at_unix_ms ASC",
            )?;
            statement
                .query_map([cutoff], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        for run_id in &stale {
            transaction.execute(
                "UPDATE execution_runs SET status = 'interrupted', finished_at_unix_ms = ?1
                 WHERE run_id = ?2 AND status = 'running'",
                params![now, run_id],
            )?;
            let run = load_execution_run(&transaction, run_id)?
                .ok_or_else(|| Error::NotFound(format!("execution run {run_id}")))?;
            append_event(
                &transaction,
                &format!("execution-interrupted:{run_id}"),
                run_id,
                "execution_run_interrupted",
                &serde_json::json!({"stale_after_seconds": stale_after_seconds}),
                &run,
                now,
            )?;
        }
        transaction.commit()?;
        Ok(stale.len() as u64)
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
            require_no_material_unknowns(&transaction, &request.session_id)?;
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
        validate_criterion_proofs(&transaction, &task, &request.criterion_proofs)?;
        transaction.execute(
            "UPDATE tasks
             SET status = 'completed', outcome_summary = ?1, completion_proofs_json = ?2,
                 lease_expires_at_unix_ms = NULL, updated_at_unix_ms = ?3
             WHERE task_id = ?4",
            params![
                request.outcome_summary,
                serde_json::to_string(&request.criterion_proofs)?,
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
            evidence_count: table_count(&connection, "evidence_artifacts", "1 = 1")?,
            claim_count: table_count(&connection, "claims", "1 = 1")?,
            unresolved_material_unknown_count: table_count(
                &connection,
                "claims",
                "status = 'unknown' AND material = 1 AND NOT EXISTS (
                    SELECT 1 FROM claims resolutions
                    WHERE resolutions.supersedes_claim_id = claims.claim_id
                )",
            )?,
            running_execution_count: table_count(
                &connection,
                "execution_runs",
                "status = 'running'",
            )?,
            interrupted_execution_count: table_count(
                &connection,
                "execution_runs",
                "status = 'interrupted'",
            )?,
            execution_receipt_count: table_count(&connection, "execution_receipts", "1 = 1")?,
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

        let evidence = load_project_evidence(&connection, &project_id)?;
        let claims = load_project_claims(&connection, &project_id)?;
        let command_specs = load_project_specs(&connection, &project_id)?;
        let execution_runs = load_project_runs(&connection, &project_id, i64::MAX)?;
        let execution_receipts = load_project_receipts(&connection, &project_id)?;
        let receipt_artifacts = load_project_receipt_artifacts(&connection, &project_id)?;

        let events = {
            let mut statement = connection.prepare(
                "SELECT sequence, event_id, idempotency_key, stream_id, kind, payload_json,
                        result_json, occurred_at_unix_ms
                 FROM events
                 WHERE stream_id = ?1 OR stream_id IN (
                     SELECT session_id FROM sessions WHERE project_id = ?1
                 ) OR stream_id IN (
                     SELECT task_id FROM tasks WHERE project_id = ?1
                 ) OR stream_id IN (
                     SELECT execution_runs.run_id FROM execution_runs
                     JOIN sessions ON sessions.session_id = execution_runs.session_id
                     WHERE sessions.project_id = ?1
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
            format_version: 3,
            exported_at_unix_ms: unix_millis()?,
            project_id,
            workspace,
            sessions,
            records,
            tasks,
            evidence,
            claims,
            command_specs,
            execution_runs,
            execution_receipts,
            receipt_artifacts,
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

fn validate_relative_path(name: &str, raw: &str) -> Result<()> {
    use std::path::Component;

    require_text(name, raw)?;
    let path = Path::new(raw);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(Error::Invalid(format!(
            "{name} must be a workspace-relative path without parent traversal"
        )));
    }
    Ok(())
}

fn workspace_for_session(connection: &Connection, session_id: &str) -> Result<String> {
    connection
        .query_row(
            "SELECT projects.workspace FROM sessions
             JOIN projects ON projects.project_id = sessions.project_id
             WHERE sessions.session_id = ?1",
            [session_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("session {session_id}")))
}

fn command_success_claim(canonical_sha256: &str, expected_exit_code: i32) -> String {
    format!("command_exit:{canonical_sha256}:code={expected_exit_code}")
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

fn validate_evidence(
    transaction: &Transaction<'_>,
    request: &EvidenceRequest,
) -> Result<(EvidenceGrade, String)> {
    match request.kind {
        EvidenceKind::WorkspaceFile => {
            let workspace = transaction.query_row(
                "SELECT projects.workspace FROM sessions
                 JOIN projects ON projects.project_id = sessions.project_id
                 WHERE sessions.session_id = ?1",
                [&request.session_id],
                |row| row.get::<_, String>(0),
            )?;
            let path = fs::canonicalize(&request.locator)?;
            if !path.is_file() || !path.starts_with(Path::new(&workspace)) {
                return Err(Error::Invalid(
                    "workspace_file must be an existing file inside the session workspace"
                        .to_owned(),
                ));
            }
            let digest = format!("{:x}", Sha256::digest(fs::read(path)?));
            if request
                .content_sha256
                .as_deref()
                .is_some_and(|expected| expected != digest)
            {
                return Err(Error::Conflict(
                    "workspace_file content_sha256 does not match Aporic's read-back".to_owned(),
                ));
            }
            Ok((EvidenceGrade::Direct, digest))
        }
        EvidenceKind::ModelAssessment => Ok((
            EvidenceGrade::ModelOnly,
            require_sha256(request.content_sha256.as_deref())?,
        )),
        EvidenceKind::CommandResult
        | EvidenceKind::ExternalSource
        | EvidenceKind::UserStatement => Ok((
            EvidenceGrade::Reported,
            require_sha256(request.content_sha256.as_deref())?,
        )),
    }
}

fn require_sha256(value: Option<&str>) -> Result<String> {
    let value = value
        .ok_or_else(|| Error::Invalid("non-file evidence requires content_sha256".to_owned()))?;
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::Invalid(
            "content_sha256 must be 64 hexadecimal characters".to_owned(),
        ));
    }
    Ok(value.to_ascii_lowercase())
}

fn validate_claim(transaction: &Transaction<'_>, request: &ClaimRequest) -> Result<()> {
    let grades = evidence_grades(transaction, &request.session_id, &request.evidence_ids)?;
    let has_direct = grades.contains(&EvidenceGrade::Direct);
    match request.status {
        ClaimStatus::Observed | ClaimStatus::Verified if !has_direct => {
            return Err(Error::Conflict(
                "observed and verified claims require Aporic-direct evidence".to_owned(),
            ));
        }
        ClaimStatus::Inferred if grades.is_empty() => {
            return Err(Error::Invalid(
                "inferred claims require typed evidence".to_owned(),
            ));
        }
        ClaimStatus::Assumed | ClaimStatus::Intended | ClaimStatus::Unknown
            if !grades.is_empty() =>
        {
            return Err(Error::Invalid(
                "assumed, intended, and unknown claims must not present evidence as support"
                    .to_owned(),
            ));
        }
        _ => {}
    }
    if matches!(
        request.status,
        ClaimStatus::Observed | ClaimStatus::Verified
    ) {
        let mut mechanically_matches = false;
        for evidence_id in &request.evidence_ids {
            let evidence = transaction.query_row(
                "SELECT grade, kind, locator, content_sha256 FROM evidence_artifacts
                 WHERE evidence_id = ?1",
                [evidence_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )?;
            if evidence.0 == "direct"
                && evidence.1 == "workspace_file"
                && request.statement == workspace_file_claim(&evidence.2, &evidence.3)
            {
                mechanically_matches = true;
            }
        }
        if !mechanically_matches {
            return Err(Error::Conflict(
                "observed and verified statements must exactly encode a directly observed file digest"
                    .to_owned(),
            ));
        }
    }
    if let Some(target_id) = &request.supersedes_claim_id {
        let target = transaction
            .query_row(
                "SELECT session_id, status FROM claims WHERE claim_id = ?1",
                [target_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("claim {target_id}")))?;
        require_same_project(transaction, &request.session_id, &target.0)?;
        if target.1 == "unknown"
            && !matches!(
                request.status,
                ClaimStatus::Observed | ClaimStatus::Verified
            )
        {
            return Err(Error::Conflict(
                "an unknown may only be resolved by an observed or verified claim".to_owned(),
            ));
        }
    }
    Ok(())
}

fn evidence_grades(
    connection: &Connection,
    session_id: &str,
    evidence_ids: &[String],
) -> Result<Vec<EvidenceGrade>> {
    let mut grades = Vec::with_capacity(evidence_ids.len());
    for evidence_id in evidence_ids {
        let item = connection
            .query_row(
                "SELECT evidence_artifacts.grade, evidence_artifacts.session_id
             FROM evidence_artifacts WHERE evidence_id = ?1",
                [evidence_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("evidence {evidence_id}")))?;
        require_same_project_connection(connection, session_id, &item.1)?;
        grades.push(parse_evidence_grade_value(&item.0)?);
    }
    Ok(grades)
}

fn require_same_project_connection(
    connection: &Connection,
    left_session_id: &str,
    right_session_id: &str,
) -> Result<()> {
    let same = connection.query_row(
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
            "referenced evidence or claim belongs to another project".to_owned(),
        ))
    }
}

fn validate_record_links(transaction: &Transaction<'_>, request: &RecordRequest) -> Result<()> {
    if matches!(request.kind, RecordKind::Effect | RecordKind::Verification) {
        return Err(Error::Invalid(
            "new effect and verification records are disabled; use typed evidence and claims"
                .to_owned(),
        ));
    }
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

fn require_no_material_unknowns(transaction: &Transaction<'_>, session_id: &str) -> Result<()> {
    let typed = transaction.query_row(
        "SELECT COUNT(*) FROM claims unknowns
         WHERE unknowns.session_id = ?1 AND unknowns.status = 'unknown' AND unknowns.material = 1
           AND NOT EXISTS (SELECT 1 FROM claims resolutions
                           WHERE resolutions.supersedes_claim_id = unknowns.claim_id)",
        [session_id],
        |row| row.get::<_, u64>(0),
    )?;
    let legacy = transaction.query_row(
        "SELECT COUNT(*) FROM records WHERE session_id = ?1 AND kind = 'material_unknown'",
        [session_id],
        |row| row.get::<_, u64>(0),
    )?;
    let unresolved = typed + legacy;
    if unresolved == 0 {
        Ok(())
    } else {
        Err(Error::Conflict(format!(
            "session has {unresolved} unresolved material unknown claim(s)"
        )))
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
                    completion_proofs_json, created_at_unix_ms, updated_at_unix_ms
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
                completion_proofs_json, created_at_unix_ms, updated_at_unix_ms
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
        completion_proofs: json_column(row, 12)?,
        created_at_unix_ms: row.get(13)?,
        updated_at_unix_ms: row.get(14)?,
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

fn parse_evidence_kind_value(value: &str) -> rusqlite::Result<EvidenceKind> {
    match value {
        "workspace_file" => Ok(EvidenceKind::WorkspaceFile),
        "command_result" => Ok(EvidenceKind::CommandResult),
        "external_source" => Ok(EvidenceKind::ExternalSource),
        "user_statement" => Ok(EvidenceKind::UserStatement),
        "model_assessment" => Ok(EvidenceKind::ModelAssessment),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_evidence_grade_value(value: &str) -> rusqlite::Result<EvidenceGrade> {
    match value {
        "direct" => Ok(EvidenceGrade::Direct),
        "reported" => Ok(EvidenceGrade::Reported),
        "model_only" => Ok(EvidenceGrade::ModelOnly),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_claim_status_value(value: &str) -> rusqlite::Result<ClaimStatus> {
    match value {
        "observed" => Ok(ClaimStatus::Observed),
        "verified" => Ok(ClaimStatus::Verified),
        "inferred" => Ok(ClaimStatus::Inferred),
        "assumed" => Ok(ClaimStatus::Assumed),
        "intended" => Ok(ClaimStatus::Intended),
        "unknown" => Ok(ClaimStatus::Unknown),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_execution_status(value: String) -> rusqlite::Result<ExecutionStatus> {
    match value.as_str() {
        "running" => Ok(ExecutionStatus::Running),
        "succeeded" => Ok(ExecutionStatus::Succeeded),
        "failed" => Ok(ExecutionStatus::Failed),
        "timed_out" => Ok(ExecutionStatus::TimedOut),
        "interrupted" => Ok(ExecutionStatus::Interrupted),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn command_spec_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CommandSpec> {
    let canonical_sha256 = row.get::<_, String>(9)?;
    let expected_exit_code = row.get::<_, i32>(6)?;
    Ok(CommandSpec {
        spec_id: row.get(0)?,
        session_id: row.get(1)?,
        workspace: row.get(2)?,
        program: row.get(3)?,
        args: json_column(row, 4)?,
        workspace_relative_cwd: row.get(5)?,
        expected_exit_code,
        timeout_seconds: row.get(7)?,
        artifact_paths: json_column(row, 8)?,
        success_claim: command_success_claim(&canonical_sha256, expected_exit_code),
        canonical_sha256,
        created_at_unix_ms: row.get(10)?,
    })
}

fn load_command_spec(connection: &Connection, spec_id: &str) -> Result<Option<CommandSpec>> {
    Ok(connection
        .query_row(
            "SELECT verification_specs.spec_id, verification_specs.session_id,
                    projects.workspace, verification_specs.program,
                    verification_specs.args_json, verification_specs.workspace_relative_cwd,
                    verification_specs.expected_exit_code, verification_specs.timeout_seconds,
                    verification_specs.artifact_paths_json, verification_specs.canonical_sha256,
                    verification_specs.created_at_unix_ms
             FROM verification_specs
             JOIN sessions ON sessions.session_id = verification_specs.session_id
             JOIN projects ON projects.project_id = sessions.project_id
             WHERE verification_specs.spec_id = ?1",
            [spec_id],
            command_spec_from_row,
        )
        .optional()?)
}

fn load_project_specs(connection: &Connection, project_id: &str) -> Result<Vec<CommandSpec>> {
    let mut statement = connection.prepare(
        "SELECT verification_specs.spec_id, verification_specs.session_id,
                projects.workspace, verification_specs.program,
                verification_specs.args_json, verification_specs.workspace_relative_cwd,
                verification_specs.expected_exit_code, verification_specs.timeout_seconds,
                verification_specs.artifact_paths_json, verification_specs.canonical_sha256,
                verification_specs.created_at_unix_ms
         FROM verification_specs
         JOIN sessions ON sessions.session_id = verification_specs.session_id
         JOIN projects ON projects.project_id = sessions.project_id
         WHERE sessions.project_id = ?1
         ORDER BY verification_specs.created_at_unix_ms ASC, verification_specs.spec_id ASC",
    )?;
    Ok(statement
        .query_map([project_id], command_spec_from_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

fn execution_run_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExecutionRun> {
    Ok(ExecutionRun {
        run_id: row.get(0)?,
        spec_id: row.get(1)?,
        session_id: row.get(2)?,
        status: parse_execution_status(row.get(3)?)?,
        attempt: row.get(4)?,
        retry_of_run_id: row.get(5)?,
        started_at_unix_ms: row.get(6)?,
        finished_at_unix_ms: row.get(7)?,
    })
}

fn load_execution_run(connection: &Connection, run_id: &str) -> Result<Option<ExecutionRun>> {
    Ok(connection
        .query_row(
            "SELECT run_id, spec_id, session_id, status, attempt, retry_of_run_id,
                    started_at_unix_ms, finished_at_unix_ms
             FROM execution_runs WHERE run_id = ?1",
            [run_id],
            execution_run_from_row,
        )
        .optional()?)
}

fn load_project_runs(
    connection: &Connection,
    project_id: &str,
    limit: i64,
) -> Result<Vec<ExecutionRun>> {
    let mut statement = connection.prepare(
        "SELECT execution_runs.run_id, execution_runs.spec_id, execution_runs.session_id,
                execution_runs.status, execution_runs.attempt, execution_runs.retry_of_run_id,
                execution_runs.started_at_unix_ms, execution_runs.finished_at_unix_ms
         FROM execution_runs
         JOIN sessions ON sessions.session_id = execution_runs.session_id
         WHERE sessions.project_id = ?1
         ORDER BY execution_runs.started_at_unix_ms DESC, execution_runs.run_id DESC
         LIMIT ?2",
    )?;
    Ok(statement
        .query_map(params![project_id, limit], execution_run_from_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

fn receipt_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExecutionReceipt> {
    Ok(ExecutionReceipt {
        receipt_id: row.get(0)?,
        run_id: row.get(1)?,
        command_spec_sha256: row.get(2)?,
        resolved_executable: row.get(3)?,
        exit_code: row.get(4)?,
        termination: row.get(5)?,
        stdout_sha256: row.get(6)?,
        stdout_bytes: row.get(7)?,
        stderr_sha256: row.get(8)?,
        stderr_bytes: row.get(9)?,
        git_head_before: row.get(10)?,
        git_head_after: row.get(11)?,
        worktree_state_before_sha256: row.get(12)?,
        worktree_state_after_sha256: row.get(13)?,
        created_at_unix_ms: row.get(14)?,
    })
}

const RECEIPT_COLUMNS: &str = "execution_receipts.receipt_id, execution_receipts.run_id,
    execution_receipts.command_spec_sha256, execution_receipts.resolved_executable,
    execution_receipts.exit_code, execution_receipts.termination,
    execution_receipts.stdout_sha256, execution_receipts.stdout_bytes,
    execution_receipts.stderr_sha256, execution_receipts.stderr_bytes,
    execution_receipts.git_head_before, execution_receipts.git_head_after,
    execution_receipts.worktree_state_before_sha256,
    execution_receipts.worktree_state_after_sha256,
    execution_receipts.created_at_unix_ms";

fn load_receipt(connection: &Connection, run_id: &str) -> Result<Option<ExecutionReceipt>> {
    let sql = format!("SELECT {RECEIPT_COLUMNS} FROM execution_receipts WHERE run_id = ?1");
    Ok(connection
        .query_row(&sql, [run_id], receipt_from_row)
        .optional()?)
}

fn load_receipt_artifacts(
    connection: &Connection,
    receipt_id: &str,
) -> Result<Vec<ReceiptArtifact>> {
    let mut statement = connection.prepare(
        "SELECT artifact_id, receipt_id, workspace_relative_path, sha256, byte_length
         FROM receipt_artifacts WHERE receipt_id = ?1 ORDER BY workspace_relative_path ASC",
    )?;
    Ok(statement
        .query_map([receipt_id], |row| {
            Ok(ReceiptArtifact {
                artifact_id: row.get(0)?,
                receipt_id: row.get(1)?,
                workspace_relative_path: row.get(2)?,
                sha256: row.get(3)?,
                byte_length: row.get(4)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

fn load_execution_outcome(
    connection: &Connection,
    run_id: &str,
) -> Result<Option<ExecutionOutcome>> {
    let Some(run) = load_execution_run(connection, run_id)? else {
        return Ok(None);
    };
    let receipt = load_receipt(connection, run_id)?;
    let artifacts = match &receipt {
        Some(receipt) => load_receipt_artifacts(connection, &receipt.receipt_id)?,
        None => Vec::new(),
    };
    let verified_claim_id = match &receipt {
        Some(receipt) => connection
            .query_row(
                "SELECT claim_id FROM claim_receipts WHERE receipt_id = ?1 LIMIT 1",
                [&receipt.receipt_id],
                |row| row.get(0),
            )
            .optional()?,
        None => None,
    };
    Ok(Some(ExecutionOutcome {
        run,
        receipt,
        artifacts,
        verified_claim_id,
    }))
}

fn load_project_receipts(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<ExecutionReceipt>> {
    let sql = format!(
        "SELECT {RECEIPT_COLUMNS} FROM execution_receipts
         JOIN execution_runs ON execution_runs.run_id = execution_receipts.run_id
         JOIN sessions ON sessions.session_id = execution_runs.session_id
         WHERE sessions.project_id = ?1
         ORDER BY execution_receipts.created_at_unix_ms ASC, execution_receipts.receipt_id ASC"
    );
    let mut statement = connection.prepare(&sql)?;
    Ok(statement
        .query_map([project_id], receipt_from_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

fn load_project_receipt_artifacts(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<ReceiptArtifact>> {
    let mut statement = connection.prepare(
        "SELECT receipt_artifacts.artifact_id, receipt_artifacts.receipt_id,
                receipt_artifacts.workspace_relative_path, receipt_artifacts.sha256,
                receipt_artifacts.byte_length
         FROM receipt_artifacts
         JOIN execution_receipts ON execution_receipts.receipt_id = receipt_artifacts.receipt_id
         JOIN execution_runs ON execution_runs.run_id = execution_receipts.run_id
         JOIN sessions ON sessions.session_id = execution_runs.session_id
         WHERE sessions.project_id = ?1
         ORDER BY receipt_artifacts.artifact_id ASC",
    )?;
    Ok(statement
        .query_map([project_id], |row| {
            Ok(ReceiptArtifact {
                artifact_id: row.get(0)?,
                receipt_id: row.get(1)?,
                workspace_relative_path: row.get(2)?,
                sha256: row.get(3)?,
                byte_length: row.get(4)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

fn load_project_evidence(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<EvidenceArtifact>> {
    let mut statement = connection.prepare(
        "SELECT evidence_id, evidence_artifacts.session_id, evidence_artifacts.kind,
                evidence_artifacts.grade, evidence_artifacts.locator, evidence_artifacts.summary,
                evidence_artifacts.content_sha256, evidence_artifacts.created_at_unix_ms
         FROM evidence_artifacts
         JOIN sessions ON sessions.session_id = evidence_artifacts.session_id
         WHERE sessions.project_id = ?1
         ORDER BY evidence_artifacts.created_at_unix_ms ASC, evidence_id ASC",
    )?;
    Ok(statement
        .query_map([project_id], |row| {
            Ok(EvidenceArtifact {
                evidence_id: row.get(0)?,
                session_id: row.get(1)?,
                kind: parse_evidence_kind_value(&row.get::<_, String>(2)?)?,
                grade: parse_evidence_grade_value(&row.get::<_, String>(3)?)?,
                locator: row.get(4)?,
                summary: row.get(5)?,
                content_sha256: row.get(6)?,
                created_at_unix_ms: row.get(7)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

fn load_project_claims(connection: &Connection, project_id: &str) -> Result<Vec<EpistemicClaim>> {
    let mut statement = connection.prepare(
        "SELECT claims.claim_id, claims.session_id, claims.status, claims.statement,
                claims.material, claims.supersedes_claim_id, claims.created_at_unix_ms
         FROM claims JOIN sessions ON sessions.session_id = claims.session_id
         WHERE sessions.project_id = ?1
         ORDER BY claims.created_at_unix_ms ASC, claims.claim_id ASC",
    )?;
    let rows = statement.query_map([project_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, bool>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, i64>(6)?,
        ))
    })?;
    let mut claims = Vec::new();
    for row in rows {
        let (claim_id, session_id, status, statement, material, supersedes_claim_id, created_at) =
            row?;
        let evidence_ids = {
            let mut evidence = connection.prepare(
                "SELECT evidence_id FROM claim_evidence WHERE claim_id = ?1 ORDER BY evidence_id ASC",
            )?;
            evidence
                .query_map([&claim_id], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let receipt_ids = {
            let mut receipts = connection.prepare(
                "SELECT receipt_id FROM claim_receipts WHERE claim_id = ?1 ORDER BY receipt_id ASC",
            )?;
            receipts
                .query_map([&claim_id], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        claims.push(EpistemicClaim {
            claim_id,
            session_id,
            status: parse_claim_status_value(&status)?,
            statement,
            material,
            evidence_ids,
            receipt_ids,
            supersedes_claim_id,
            created_at_unix_ms: created_at,
        });
    }
    Ok(claims)
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

fn validate_criterion_proofs(
    connection: &Connection,
    task: &CoordinatedTask,
    proofs: &[CriterionProof],
) -> Result<()> {
    if task.acceptance_criteria.len() != proofs.len() {
        return Err(Error::Invalid(
            "criterion_proofs must cover every acceptance criterion exactly once".to_owned(),
        ));
    }
    let expected = task
        .acceptance_criteria
        .iter()
        .map(|criterion| criterion.trim())
        .collect::<std::collections::BTreeSet<_>>();
    let mut observed = std::collections::BTreeSet::new();
    for proof in proofs {
        require_text("criterion", &proof.criterion)?;
        require_text("verified_claim_id", &proof.verified_claim_id)?;
        if !observed.insert(proof.criterion.trim()) {
            return Err(Error::Invalid(
                "criterion_proofs contains duplicate criteria".to_owned(),
            ));
        }
        let claim = connection
            .query_row(
                "SELECT claims.status, sessions.project_id, claims.statement
             FROM claims JOIN sessions ON sessions.session_id = claims.session_id
             WHERE claims.claim_id = ?1",
                [&proof.verified_claim_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("claim {}", proof.verified_claim_id)))?;
        if claim.0 != "verified" || claim.1 != task.project_id || claim.2 != proof.criterion {
            return Err(Error::Conflict(
                "each criterion proof must exactly match a verified mechanical claim in the task project".to_owned(),
            ));
        }
    }
    if observed != expected {
        Err(Error::Invalid(
            "criterion_proofs do not match the task acceptance criteria".to_owned(),
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
