use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use hmac::{Hmac, Mac};
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, params,
};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    bounded::{
        MAX_EVIDENCE_FILE_BYTES, MAX_RECEIPT_ARTIFACT_BYTES, MAX_RECEIPT_ARTIFACT_TOTAL_BYTES,
        MAX_RECEIPT_ARTIFACTS, sha256_file_bounded,
    },
    domain::{
        AbandonedSession, AccountabilityAudit, AccountabilityCase, AccountabilityListRequest,
        AccountabilityNotice, AccountabilityOpenRequest, AccountabilityOutcome,
        AccountabilityPlanRequest, AccountabilityReport, AccountabilityResolveRequest,
        AccountabilityStatus, ActiveSession, ActualTaskOutcome, AdvisoryDisposition, AdvisoryMode,
        AdvisoryRoleKind, AdvisoryRoleReport, AdvisoryRoleReportRequest, AdvisorySourceKind,
        CapabilityCatalogState, CapabilityClass, CapabilityEffectClass, CapabilityGetRequest,
        CapabilityManifest, CapabilityMaturity, CapabilityObservation, CapabilityOutcome,
        CapabilityProviderKind, CapabilityRegisterRequest, CapabilityReport,
        CapabilitySearchRequest, CapabilitySummary, ClaimOutcome, ClaimRequest, ClaimStatus,
        CloseDisposition, CloseOutcome, CloseRequest, CommandSpec, CommandSpecOutcome,
        CommandSpecRequest, Consequence, ContextCapsule, CoordinatedTask, CriterionProof,
        Deliberation, DeliberationAudit, DeliberationCreateRequest, DeliberationDecision,
        DeliberationDecisionRequest, DeliberationEdge, DeliberationEdgeKind,
        DeliberationGetRequest, DeliberationGraph, DeliberationListRequest, DeliberationNode,
        DeliberationNodeAddRequest, DeliberationNodeKind, DeliberationOutcome, DeliberationSummary,
        DissentAssessment, DissentRequest, DurableRecord, EpistemicClaim, EvidenceArtifact,
        EvidenceGrade, EvidenceKind, EvidenceOutcome, EvidenceRequest, ExecutionFinish,
        ExecutionGetRequest, ExecutionListRequest, ExecutionOutcome, ExecutionReceipt,
        ExecutionReplayAudit, ExecutionRun, ExecutionStart, ExecutionStatus, ExperimentCampaign,
        ExperimentComparison, ExperimentCreateRequest, ExperimentCriterion,
        ExperimentCriterionKind, ExperimentDecision, ExperimentDecisionKind,
        ExperimentDecisionRequest, ExperimentDiversityAxis, ExperimentGetRequest,
        ExperimentListRequest, ExperimentMeasurement, ExperimentMeasurementAddRequest,
        ExperimentOutcome, ExperimentPortfolio, ExperimentSummary, ExperimentVariant,
        ExperimentVariantAddRequest, ExportEvent, ExportSession, GitSnapshot, GitSnapshotAudit,
        GitSnapshotDraft, GitSnapshotGetRequest, GitSnapshotListRequest, GovernmentAudit,
        GovernmentWorkspaceRequest, Handoff, HookHealthReport, HubStats, InfluenceClass,
        MemoryClass, MemoryEdge, MemoryExposure, MemoryGetRequest, MemoryItem, MemoryLifecycle,
        MemoryProjectionAudit, MemorySearchRequest, MemorySearchResult, OfficeAppointment,
        OfficeAppointmentCreateRequest, OfficeAppointmentOutcome, OfficeAppointmentRevokeRequest,
        OpenOutcome, OpenRequest, OrchestrationAudit, OrchestrationOutcome, OrchestrationRun,
        OrchestrationRunCreateRequest, OrchestrationRunGetRequest, OrchestrationRunListRequest,
        OrchestrationRunStatus, OrchestrationRunSummary, OrchestrationRunView, OriginChannel,
        PredictedTaskOutcome, ProductCell, ProductCellCreateRequest, ProductCellMember,
        ProductCellOutcome, ProjectExport, RecallRequest, ReceiptArtifact, ReconcileOutcome,
        ReconcileRequest, RecordKind, RecordOutcome, RecordRequest, ResumeBrief, ResumeCandidate,
        ResumeRequest, ResumeStatus, RoleAppointment, RoleAppointmentAudit,
        RoleAppointmentCreateRequest, RoleAppointmentListRequest, RoleAppointmentOutcome,
        RoleAppointmentRevokeRequest, RoleCapabilityRef, RuntimeEvent, RuntimeEventKind,
        RuntimeObservation, RuntimeOutcomeStatus, RuntimeProjectionAudit, RuntimeTraceGetRequest,
        RuntimeTraceListRequest, RuntimeWorkspaceRequest, SandboxEnforcement,
        SecureCapabilityAudit, SecurityArtifactImport, SecurityAssessment,
        SecurityAssessmentGetRequest, SecurityAssessmentListRequest, SecurityAssessmentOutcome,
        SecurityCoverage, ShadowDisposition, ShadowEvaluation, ShadowEvaluationRequest,
        TaskCancelRequest, TaskClaimRequest, TaskCompleteRequest, TaskCreateRequest,
        TaskListRequest, TaskOutcome, TaskStatus, TokenCountSource, TokenEfficiencyReport,
        TokenEfficiencyReportRequest, TokenUsageAudit, TokenUsageListRequest, TokenUsageOutcome,
        TokenUsageReceipt, TokenUsageRecordRequest, UsageOutcome, workspace_file_claim,
    },
};

const SCHEMA: &str = include_str!("../../../migrations/0001_initial.sql");
const MIGRATION_2: &str = include_str!("../../../migrations/0002_continuity_hardening.sql");
const MIGRATION_3: &str = include_str!("../../../migrations/0003_coordination.sql");
const MIGRATION_4: &str = include_str!("../../../migrations/0004_epistemic_gate.sql");
const MIGRATION_5: &str = include_str!("../../../migrations/0005_verifiable_execution.sql");
const MIGRATION_6: &str = include_str!("../../../migrations/0006_authority_bound_context.sql");
const MIGRATION_7: &str = include_str!("../../../migrations/0007_memory_lifecycle.sql");
const MIGRATION_8: &str = include_str!("../../../migrations/0008_runtime_trace.sql");
const MIGRATION_9: &str = include_str!("../../../migrations/0009_git_governance.sql");
const MIGRATION_10: &str = include_str!("../../../migrations/0010_token_efficiency.sql");
const MIGRATION_11: &str = include_str!("../../../migrations/0011_commit_bound_deliberation.sql");
const MIGRATION_12: &str = include_str!("../../../migrations/0012_secure_capability_fabric.sql");
const MIGRATION_13: &str = include_str!("../../../migrations/0013_execution_governance.sql");
const MIGRATION_14: &str = include_str!("../../../migrations/0014_advisory_orchestration.sql");
const MIGRATION_15: &str = include_str!("../../../migrations/0015_roles.sql");
const MIGRATION_16: &str = include_str!("../../../migrations/0016_role_run_link.sql");
const MIGRATION_17: &str = include_str!("../../../migrations/0017_product_government.sql");
const MIGRATION_18: &str = include_str!("../../../migrations/0018_external_research.sql");
const MIGRATION_19: &str = include_str!("../../../migrations/0019_accountability.sql");
const SCHEMA_VERSION: u32 = 19;
const MIGRATIONS: &[(u32, &str)] = &[
    (2, MIGRATION_2),
    (3, MIGRATION_3),
    (4, MIGRATION_4),
    (5, MIGRATION_5),
    (6, MIGRATION_6),
    (7, MIGRATION_7),
    (8, MIGRATION_8),
    (9, MIGRATION_9),
    (10, MIGRATION_10),
    (11, MIGRATION_11),
    (12, MIGRATION_12),
    (13, MIGRATION_13),
    (14, MIGRATION_14),
    (15, MIGRATION_15),
    (16, MIGRATION_16),
    (17, MIGRATION_17),
    (18, MIGRATION_18),
    (19, MIGRATION_19),
];

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
        let mut schema_version =
            connection.pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))?;
        if schema_version > SCHEMA_VERSION {
            return Err(Error::Invalid(format!(
                "database schema version {schema_version} is newer than supported version {SCHEMA_VERSION}"
            )));
        }
        if schema_version == 0 {
            connection.execute_batch(SCHEMA)?;
            schema_version =
                connection.pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))?;
        }
        for (target_version, migration) in MIGRATIONS {
            if *target_version > schema_version {
                connection.execute_batch(migration)?;
            }
        }
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn backup_to(&self, target: &Path) -> Result<()> {
        if target.exists() {
            return Err(Error::Conflict(format!(
                "backup target {} already exists",
                target.display()
            )));
        }
        let parent = target.parent().ok_or_else(|| {
            Error::Invalid("backup target must have a parent directory".to_owned())
        })?;
        fs::create_dir_all(parent)?;
        let connection = self.connection()?;
        connection.execute("VACUUM INTO ?1", [target.to_string_lossy().as_ref()])?;
        let validation = Self::validate_backup(target)?;
        if validation != SCHEMA_VERSION {
            return Err(Error::Conflict(format!(
                "backup schema version {validation} does not match {SCHEMA_VERSION}"
            )));
        }
        Ok(())
    }

    pub fn validate_backup(path: &Path) -> Result<u32> {
        if !path.is_file() {
            return Err(Error::Invalid(
                "backup path must be an existing file".to_owned(),
            ));
        }
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        let integrity: String =
            connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if integrity != "ok" {
            return Err(Error::Conflict(format!(
                "backup integrity check failed: {integrity}"
            )));
        }
        let version = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        let core_table_count: u32 = connection.query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'table' AND name IN ('projects', 'sessions', 'records', 'events')",
            [],
            |row| row.get(0),
        )?;
        if version == 0 || core_table_count != 4 {
            return Err(Error::Invalid(
                "backup is not a recognized Aporic database".to_owned(),
            ));
        }
        if version > SCHEMA_VERSION {
            return Err(Error::Invalid(format!(
                "backup schema version {version} is newer than supported version {SCHEMA_VERSION}"
            )));
        }
        Ok(version)
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
        let context = recall_for_project(
            &transaction,
            &project_id,
            &workspace,
            20,
            Some(&request.objective),
            &[],
            16_384,
        )?;
        let open_repair_count = transaction.query_row(
            "SELECT COUNT(*) FROM accountability_cases
             WHERE project_id = ?1 AND status = 'open'",
            [&project_id],
            |row| row.get::<_, u64>(0),
        )?;
        let open_repair_obligations = transaction
            .prepare(
                "SELECT case_id, source_task_id, observed_behavior, repair_task_id
             FROM accountability_cases WHERE project_id = ?1 AND status = 'open'
             ORDER BY created_at_unix_ms DESC, case_id DESC LIMIT 5",
            )?
            .query_map([&project_id], |row| {
                Ok(AccountabilityNotice {
                    case_id: row.get(0)?,
                    source_task_id: row.get(1)?,
                    observed_behavior: row.get(2)?,
                    repair_task_id: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
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
            open_repair_count,
            open_repair_obligations,
            accountability_notice: accountability_authority_notice(),
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
        let max_bytes = request.max_bytes.unwrap_or(16_384).clamp(1_024, 65_536);

        match project_id {
            Some(project_id) => recall_for_project(
                &connection,
                &project_id,
                &workspace,
                i64::from(limit),
                request.objective.as_deref(),
                &request.focus_paths,
                max_bytes,
            ),
            None => {
                let (selected_items, budget) = crate::context::select(
                    Vec::new(),
                    request.objective.as_deref(),
                    &request.focus_paths,
                    limit,
                    max_bytes,
                );
                Ok(ContextCapsule {
                    project_id: None,
                    workspace,
                    active_sessions: Vec::new(),
                    recent_handoffs: Vec::new(),
                    recent_records: Vec::new(),
                    selected_items,
                    budget,
                    policy_sha256: crate::context::policy_sha256(),
                    authority_notice: crate::context::AUTHORITY_NOTICE.to_owned(),
                })
            }
        }
    }

    pub fn resume(&self, request: &ResumeRequest) -> Result<ResumeBrief> {
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
            return Ok(ResumeBrief {
                status: ResumeStatus::None,
                candidates: Vec::new(),
                selected: None,
                reason: "No recorded project work exists for this workspace.".to_owned(),
                authority_notice: resume_authority_notice(),
            });
        };

        let mut statement = connection.prepare(
            "SELECT task_id, objective, updated_at_unix_ms FROM tasks
             WHERE project_id = ?1 AND status IN ('queued', 'leased')
             ORDER BY updated_at_unix_ms DESC, task_id DESC LIMIT 6",
        )?;
        let tasks = statement.query_map([&project_id], |row| {
            Ok(ResumeCandidate {
                source: "active_task".to_owned(),
                source_id: row.get(0)?,
                objective: row.get(1)?,
                next_action: "Inspect the task's acceptance criteria and current repository state, then continue the unfinished work.".to_owned(),
                updated_at_unix_ms: row.get(2)?,
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        if !tasks.is_empty() {
            let selected = (tasks.len() == 1).then(|| tasks[0].clone());
            return Ok(ResumeBrief {
                status: if selected.is_some() {
                    ResumeStatus::Ready
                } else {
                    ResumeStatus::Ambiguous
                },
                candidates: tasks,
                selected,
                reason: "Unfinished advisory tasks take precedence over historical handoffs."
                    .to_owned(),
                authority_notice: resume_authority_notice(),
            });
        }

        let mut open_statement = connection.prepare(
            "SELECT session_id, objective, last_activity_at_unix_ms FROM sessions
             WHERE project_id = ?1 AND status = 'open' AND abandoned = 0
             ORDER BY last_activity_at_unix_ms DESC, session_id DESC LIMIT 6",
        )?;
        let open_sessions = open_statement.query_map([&project_id], |row| {
            Ok(ResumeCandidate {
                source: "open_session".to_owned(),
                source_id: row.get(0)?,
                objective: row.get(1)?,
                next_action: "Inspect the interrupted session's recent records and live repository state before continuing.".to_owned(),
                updated_at_unix_ms: row.get(2)?,
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        if !open_sessions.is_empty() {
            let selected = (open_sessions.len() == 1).then(|| open_sessions[0].clone());
            return Ok(ResumeBrief {
                status: if selected.is_some() {
                    ResumeStatus::Ready
                } else {
                    ResumeStatus::Ambiguous
                },
                candidates: open_sessions,
                selected,
                reason: "Open sessions with no unfinished task may represent interrupted work."
                    .to_owned(),
                authority_notice: resume_authority_notice(),
            });
        }

        let latest_completion: Option<i64> = connection.query_row(
            "SELECT MAX(closed_at_unix_ms) FROM sessions
             WHERE project_id = ?1 AND status = 'completed'",
            [&project_id],
            |row| row.get(0),
        )?;
        let handoff = connection
            .query_row(
                "SELECT session_id, summary, next_action, closed_at_unix_ms FROM sessions
             WHERE project_id = ?1 AND status = 'handoff'
             ORDER BY closed_at_unix_ms DESC, session_id DESC LIMIT 1",
                [&project_id],
                |row| {
                    Ok(ResumeCandidate {
                        source: "handoff".to_owned(),
                        source_id: row.get(0)?,
                        objective: row.get(1)?,
                        next_action: row.get(2)?,
                        updated_at_unix_ms: row.get(3)?,
                    })
                },
            )
            .optional()?;
        let selected = handoff.filter(|item| {
            latest_completion.is_none_or(|completed| item.updated_at_unix_ms > completed)
        });
        Ok(ResumeBrief {
            status: if selected.is_some() { ResumeStatus::Ready } else { ResumeStatus::None },
            candidates: selected.clone().into_iter().collect(),
            selected,
            reason: "Only a handoff newer than the latest completed session is eligible when no active task exists.".to_owned(),
            authority_notice: resume_authority_notice(),
        })
    }

    pub fn create_role_appointment(
        &self,
        request: &RoleAppointmentCreateRequest,
    ) -> Result<RoleAppointmentOutcome> {
        require_text("session_id", &request.session_id)?;
        require_identifier("assignee_id", &request.assignee_id, 128)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        if let Some(model) = &request.model_hint {
            require_identifier("model_hint", model, 128)?;
        }
        if request.capability_refs.len() > 16 {
            return Err(Error::Invalid(
                "an appointment may reference at most 16 capabilities".to_owned(),
            ));
        }
        let role = crate::roles::get(&request.role_id, request.role_version)
            .ok_or_else(|| Error::Invalid("unknown role or role version".to_owned()))?;
        if role.task_required != request.task_id.is_some() {
            return Err(Error::Invalid(
                "this role requires a different task scope".to_owned(),
            ));
        }
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<RoleAppointmentOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "role_appointment_created",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        require_open_session(&transaction, &request.session_id)?;
        let project_id: String = transaction.query_row(
            "SELECT project_id FROM sessions WHERE session_id = ?1",
            [&request.session_id],
            |row| row.get(0),
        )?;
        if request.role_id == "oversight.inspector" {
            let office_conflict: u64 = transaction.query_row(
                "SELECT COUNT(*) FROM office_appointments
                 WHERE session_id = ?1 AND assignee_id = ?2 AND revoked_at_unix_ms IS NULL",
                params![request.session_id, request.assignee_id],
                |row| row.get(0),
            )?;
            if office_conflict > 0 {
                return Err(Error::Conflict(
                    "an office head and inspector must have different assignees".to_owned(),
                ));
            }
        }
        let mut unique_refs = std::collections::BTreeSet::new();
        for reference in &request.capability_refs {
            require_identifier("capability_id", &reference.capability_id, 128)?;
            require_identifier("capability_version", &reference.version, 64)?;
            if !unique_refs.insert((&reference.capability_id, &reference.version)) {
                return Err(Error::Invalid("duplicate capability reference".to_owned()));
            }
            load_capability_manifest(
                &transaction,
                &project_id,
                &reference.capability_id,
                &reference.version,
            )?;
        }
        if let Some(task_id) = &request.task_id {
            let task = load_task(&transaction, task_id)?
                .ok_or_else(|| Error::NotFound(format!("task {task_id}")))?;
            if task.project_id != project_id
                || task.session_id != request.session_id
                || matches!(task.status, TaskStatus::Completed | TaskStatus::Cancelled)
            {
                return Err(Error::Conflict(
                    "appointment requires an active task in the same project".to_owned(),
                ));
            }
            let opposed = if request.role_id == "delivery.worker" {
                "oversight.inspector"
            } else {
                "delivery.worker"
            };
            if request.role_id == "delivery.worker" || request.role_id == "oversight.inspector" {
                let conflict: u64 = transaction.query_row(
                    "SELECT COUNT(*) FROM role_appointments WHERE task_id = ?1 AND role_id = ?2
                       AND assignee_id = ?3 AND revoked_at_unix_ms IS NULL",
                    params![task_id, opposed, request.assignee_id],
                    |row| row.get(0),
                )?;
                if conflict > 0 {
                    return Err(Error::Conflict(
                        "worker and inspector must have different assignees for a task".to_owned(),
                    ));
                }
            }
        }
        if request.role_id == "executive.prime_minister" {
            let count: u64 = transaction.query_row(
                "SELECT COUNT(*) FROM role_appointments WHERE session_id = ?1
                 AND role_id = 'executive.prime_minister' AND revoked_at_unix_ms IS NULL",
                [&request.session_id],
                |row| row.get(0),
            )?;
            if count > 0 {
                return Err(Error::Conflict(
                    "a session already has an active prime minister appointment".to_owned(),
                ));
            }
        }
        let appointment_id = Uuid::now_v7().to_string();
        let digest = role_appointment_digest(&appointment_id, request)?;
        transaction.execute(
            "INSERT INTO role_appointments(appointment_id, project_id, session_id, task_id,
             role_id, role_version, assignee_id, model_hint, capability_refs_json,
             appointment_sha256, created_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                appointment_id,
                project_id,
                request.session_id,
                request.task_id,
                request.role_id,
                request.role_version,
                request.assignee_id,
                request.model_hint,
                serde_json::to_string(&request.capability_refs)?,
                digest,
                now
            ],
        )?;
        let appointment = load_role_appointment(&transaction, &appointment_id)?;
        let outcome = RoleAppointmentOutcome {
            appointment,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &appointment_id,
            "role_appointment_created",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn revoke_role_appointment(
        &self,
        request: &RoleAppointmentRevokeRequest,
    ) -> Result<RoleAppointmentOutcome> {
        require_text("appointment_id", &request.appointment_id)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<RoleAppointmentOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "role_appointment_revoked",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let appointment = load_role_appointment(&transaction, &request.appointment_id)?;
        if appointment.revoked_at_unix_ms.is_some() {
            return Err(Error::Conflict("appointment is already revoked".to_owned()));
        }
        transaction.execute(
            "UPDATE role_appointments SET revoked_at_unix_ms = ?1 WHERE appointment_id = ?2",
            params![now, request.appointment_id],
        )?;
        let outcome = RoleAppointmentOutcome {
            appointment: load_role_appointment(&transaction, &request.appointment_id)?,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.appointment_id,
            "role_appointment_revoked",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn list_role_appointments(
        &self,
        request: &RoleAppointmentListRequest,
    ) -> Result<Vec<RoleAppointment>> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT appointment_id FROM role_appointments
             WHERE project_id = (SELECT project_id FROM projects WHERE workspace = ?1)
             ORDER BY created_at_unix_ms DESC, appointment_id DESC LIMIT ?2",
        )?;
        let ids = statement
            .query_map(
                params![
                    workspace,
                    i64::from(request.limit.unwrap_or(50).clamp(1, 200))
                ],
                |row| row.get::<_, String>(0),
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ids.iter()
            .map(|id| load_role_appointment(&connection, id))
            .collect()
    }

    pub fn audit_role_appointments(&self) -> Result<RoleAppointmentAudit> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare("SELECT appointment_id FROM role_appointments ORDER BY sequence ASC")?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let invalid_count = ids
            .iter()
            .filter(|id| load_role_appointment(&connection, id).is_err())
            .count() as u64;
        Ok(RoleAppointmentAudit {
            appointment_count: ids.len() as u64,
            invalid_count,
            consistent: invalid_count == 0,
        })
    }

    pub fn create_office_appointment(
        &self,
        request: &OfficeAppointmentCreateRequest,
    ) -> Result<OfficeAppointmentOutcome> {
        require_text("session_id", &request.session_id)?;
        require_text("role_appointment_id", &request.role_appointment_id)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let office = crate::government::office(&request.office_id, request.office_version)
            .ok_or_else(|| Error::Invalid("unknown office or office version".to_owned()))?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<OfficeAppointmentOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "office_appointment_created",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        require_open_session(&transaction, &request.session_id)?;
        let project_id: String = transaction.query_row(
            "SELECT project_id FROM sessions WHERE session_id = ?1",
            [&request.session_id],
            |row| row.get(0),
        )?;
        let role = load_role_appointment(&transaction, &request.role_appointment_id)?;
        if role.session_id != request.session_id
            || role.task_id.is_some()
            || role.role_id != office.head_role_id
            || role.revoked_at_unix_ms.is_some()
        {
            return Err(Error::Conflict(
                "office head requires an active matching session-scoped role appointment"
                    .to_owned(),
            ));
        }
        let inspector_conflict: u64 = transaction.query_row(
            "SELECT COUNT(*) FROM role_appointments
             WHERE session_id = ?1 AND role_id = 'oversight.inspector'
               AND assignee_id = ?2 AND revoked_at_unix_ms IS NULL",
            params![request.session_id, role.assignee_id],
            |row| row.get(0),
        )?;
        if inspector_conflict > 0 {
            return Err(Error::Conflict(
                "an office head and inspector must have different assignees".to_owned(),
            ));
        }
        let active_head: u64 = transaction.query_row(
            "SELECT COUNT(*) FROM office_appointments
             WHERE session_id = ?1 AND office_id = ?2 AND revoked_at_unix_ms IS NULL",
            params![request.session_id, request.office_id],
            |row| row.get(0),
        )?;
        if active_head > 0 {
            return Err(Error::Conflict(
                "the office already has an active head in this session".to_owned(),
            ));
        }
        let office_appointment_id = Uuid::now_v7().to_string();
        let digest = office_appointment_digest(&office_appointment_id, request, &role.assignee_id)?;
        transaction.execute(
            "INSERT INTO office_appointments(
                office_appointment_id, project_id, session_id, office_id, office_version,
                role_appointment_id, assignee_id, appointment_sha256, created_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                office_appointment_id,
                project_id,
                request.session_id,
                request.office_id,
                request.office_version,
                request.role_appointment_id,
                role.assignee_id,
                digest,
                now,
            ],
        )?;
        let outcome = OfficeAppointmentOutcome {
            appointment: load_office_appointment(&transaction, &office_appointment_id)?,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &office_appointment_id,
            "office_appointment_created",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn revoke_office_appointment(
        &self,
        request: &OfficeAppointmentRevokeRequest,
    ) -> Result<OfficeAppointmentOutcome> {
        require_text("office_appointment_id", &request.office_appointment_id)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<OfficeAppointmentOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "office_appointment_revoked",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let appointment = load_office_appointment(&transaction, &request.office_appointment_id)?;
        if appointment.revoked_at_unix_ms.is_some() {
            return Err(Error::Conflict(
                "office appointment is already revoked".to_owned(),
            ));
        }
        transaction.execute(
            "UPDATE office_appointments SET revoked_at_unix_ms = ?1
             WHERE office_appointment_id = ?2",
            params![now, request.office_appointment_id],
        )?;
        let outcome = OfficeAppointmentOutcome {
            appointment: load_office_appointment(&transaction, &request.office_appointment_id)?,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.office_appointment_id,
            "office_appointment_revoked",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn list_office_appointments(
        &self,
        request: &GovernmentWorkspaceRequest,
    ) -> Result<Vec<OfficeAppointment>> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT office_appointment_id FROM office_appointments
             WHERE project_id = (SELECT project_id FROM projects WHERE workspace = ?1)
             ORDER BY created_at_unix_ms DESC, office_appointment_id DESC LIMIT ?2",
        )?;
        let ids = statement
            .query_map(
                params![
                    workspace,
                    i64::from(request.limit.unwrap_or(50).clamp(1, 200))
                ],
                |row| row.get::<_, String>(0),
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ids.iter()
            .map(|id| load_office_appointment(&connection, id))
            .collect()
    }

    pub fn create_product_cell(
        &self,
        request: &ProductCellCreateRequest,
    ) -> Result<ProductCellOutcome> {
        require_text("session_id", &request.session_id)?;
        require_text("task_id", &request.task_id)?;
        require_text("office_appointment_id", &request.office_appointment_id)?;
        require_bounded_public_text("title", &request.title, 256)?;
        require_bounded_public_text("problem_statement", &request.problem_statement, 4096)?;
        require_bounded_public_text("hypothesis", &request.hypothesis, 4096)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        if request.success_measures.is_empty() || request.success_measures.len() > 8 {
            return Err(Error::Invalid(
                "a product cell requires between 1 and 8 success measures".to_owned(),
            ));
        }
        require_texts("success_measures", &request.success_measures)?;
        for measure in &request.success_measures {
            require_bounded_public_text("success_measure", measure, 1024)?;
        }
        if !(2..=8).contains(&request.members.len()) {
            return Err(Error::Invalid(
                "a product cell requires between 2 and 8 members".to_owned(),
            ));
        }
        let duties = request
            .members
            .iter()
            .map(|member| member.duty.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        if !duties.contains("product_planning") || !duties.contains("prototype_delivery") {
            return Err(Error::Invalid(
                "a product cell requires product planning and prototype delivery disciplines"
                    .to_owned(),
            ));
        }
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<ProductCellOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "product_cell_created",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        require_open_session(&transaction, &request.session_id)?;
        let project_id: String = transaction.query_row(
            "SELECT project_id FROM sessions WHERE session_id = ?1",
            [&request.session_id],
            |row| row.get(0),
        )?;
        let task = load_task(&transaction, &request.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        if task.project_id != project_id
            || task.session_id != request.session_id
            || matches!(task.status, TaskStatus::Completed | TaskStatus::Cancelled)
        {
            return Err(Error::Conflict(
                "a product cell requires an active task in the same session".to_owned(),
            ));
        }
        let office = load_office_appointment(&transaction, &request.office_appointment_id)?;
        if office.session_id != request.session_id
            || office.office_id != crate::government::PRODUCT_EXPERIMENT_OFFICE_ID
            || office.revoked_at_unix_ms.is_some()
        {
            return Err(Error::Conflict(
                "a product cell requires the active product experiment minister for its session"
                    .to_owned(),
            ));
        }
        let mut resolved_members = Vec::with_capacity(request.members.len());
        let mut role_ids = std::collections::BTreeSet::new();
        let mut assignees = std::collections::BTreeSet::new();
        for member in &request.members {
            if !role_ids.insert(&member.role_appointment_id) {
                return Err(Error::Invalid(
                    "duplicate product cell role appointment".to_owned(),
                ));
            }
            let role = load_role_appointment(&transaction, &member.role_appointment_id)?;
            if role.session_id != request.session_id
                || role.task_id.as_deref() != Some(request.task_id.as_str())
                || role.role_id != "delivery.worker"
                || role.revoked_at_unix_ms.is_some()
            {
                return Err(Error::Conflict(
                    "product cell members require active delivery-worker appointments for the same task"
                        .to_owned(),
                ));
            }
            if role.assignee_id == office.assignee_id {
                return Err(Error::Conflict(
                    "the product minister and product cell members must have different assignees"
                        .to_owned(),
                ));
            }
            assignees.insert(role.assignee_id.clone());
            resolved_members.push(ProductCellMember {
                role_appointment_id: role.appointment_id,
                assignee_id: role.assignee_id,
                duty: member.duty.clone(),
            });
        }
        if assignees.len() < 2 {
            return Err(Error::Conflict(
                "a multidisciplinary product cell requires at least two distinct assignees"
                    .to_owned(),
            ));
        }
        let cell_id = Uuid::now_v7().to_string();
        let digest = product_cell_digest(&cell_id, request, &resolved_members)?;
        transaction.execute(
            "INSERT INTO product_cells(
                cell_id, project_id, session_id, task_id, office_appointment_id, title,
                problem_statement, hypothesis, success_measures_json, members_json,
                cell_sha256, created_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                cell_id,
                project_id,
                request.session_id,
                request.task_id,
                request.office_appointment_id,
                request.title,
                request.problem_statement,
                request.hypothesis,
                serde_json::to_string(&request.success_measures)?,
                serde_json::to_string(&resolved_members)?,
                digest,
                now,
            ],
        )?;
        let outcome = ProductCellOutcome {
            cell: load_product_cell(&transaction, &cell_id)?,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &cell_id,
            "product_cell_created",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn list_product_cells(
        &self,
        request: &GovernmentWorkspaceRequest,
    ) -> Result<Vec<ProductCell>> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT cell_id FROM product_cells
             WHERE project_id = (SELECT project_id FROM projects WHERE workspace = ?1)
             ORDER BY created_at_unix_ms DESC, cell_id DESC LIMIT ?2",
        )?;
        let ids = statement
            .query_map(
                params![
                    workspace,
                    i64::from(request.limit.unwrap_or(50).clamp(1, 200))
                ],
                |row| row.get::<_, String>(0),
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ids.iter()
            .map(|id| load_product_cell(&connection, id))
            .collect()
    }

    pub fn audit_government(&self) -> Result<GovernmentAudit> {
        let connection = self.connection()?;
        let office_ids = connection
            .prepare("SELECT office_appointment_id FROM office_appointments ORDER BY sequence")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let cell_ids = connection
            .prepare("SELECT cell_id FROM product_cells ORDER BY sequence")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let invalid_count = office_ids
            .iter()
            .filter(|id| load_office_appointment(&connection, id).is_err())
            .count() as u64
            + cell_ids
                .iter()
                .filter(|id| load_product_cell(&connection, id).is_err())
                .count() as u64;
        Ok(GovernmentAudit {
            office_appointment_count: office_ids.len() as u64,
            product_cell_count: cell_ids.len() as u64,
            invalid_count,
            consistent: invalid_count == 0,
        })
    }

    pub fn open_accountability_case(
        &self,
        request: &AccountabilityOpenRequest,
    ) -> Result<AccountabilityOutcome> {
        require_text("session_id", &request.session_id)?;
        require_text("source_task_id", &request.source_task_id)?;
        require_text("evidence_id", &request.evidence_id)?;
        require_bounded_public_text("expected_behavior", &request.expected_behavior, 2048)?;
        require_bounded_public_text("observed_behavior", &request.observed_behavior, 2048)?;
        require_bounded_public_text("impact", &request.impact, 2048)?;
        if let Some(assignee) = &request.reported_assignee_id {
            require_identifier("reported_assignee_id", assignee, 128)?;
        }
        require_text("idempotency_key", &request.idempotency_key)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<AccountabilityOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "accountability_case_opened",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        require_open_session(&transaction, &request.session_id)?;
        let project_id: String = transaction.query_row(
            "SELECT project_id FROM sessions WHERE session_id = ?1",
            [&request.session_id],
            |row| row.get(0),
        )?;
        let source_task = load_task(&transaction, &request.source_task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.source_task_id)))?;
        if source_task.project_id != project_id {
            return Err(Error::Conflict(
                "source task belongs to another workspace".to_owned(),
            ));
        }
        let evidence_project = transaction
            .query_row(
                "SELECT sessions.project_id FROM evidence_artifacts AS evidence
             JOIN sessions ON sessions.session_id = evidence.session_id
             WHERE evidence.evidence_id = ?1",
                [&request.evidence_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("evidence {}", request.evidence_id)))?;
        if evidence_project != project_id {
            return Err(Error::Conflict(
                "evidence belongs to another workspace".to_owned(),
            ));
        }
        let existing = transaction
            .query_row(
                "SELECT case_id FROM accountability_cases
             WHERE source_task_id = ?1 AND evidence_id = ?2 AND observed_behavior = ?3
               AND status = 'open' LIMIT 1",
                params![
                    request.source_task_id,
                    request.evidence_id,
                    request.observed_behavior
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(case_id) = existing {
            return Err(Error::Conflict(format!(
                "open case already records this finding: {case_id}"
            )));
        }
        let case_id = Uuid::now_v7().to_string();
        let case_sha256 = accountability_case_digest(&case_id, &project_id, request)?;
        transaction.execute(
            "INSERT INTO accountability_cases
             (case_id, project_id, session_id, source_task_id, evidence_id,
              reported_assignee_id, expected_behavior, observed_behavior, impact,
              case_sha256, status, created_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'open', ?11)",
            params![
                case_id,
                project_id,
                request.session_id,
                request.source_task_id,
                request.evidence_id,
                request.reported_assignee_id,
                request.expected_behavior,
                request.observed_behavior,
                request.impact,
                case_sha256,
                now
            ],
        )?;
        let outcome = AccountabilityOutcome {
            case: load_accountability_case(&transaction, &case_id)?,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &case_id,
            "accountability_case_opened",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn plan_accountability_repair(
        &self,
        request: &AccountabilityPlanRequest,
    ) -> Result<AccountabilityOutcome> {
        require_text("case_id", &request.case_id)?;
        require_text("repair_task_id", &request.repair_task_id)?;
        require_bounded_public_text(
            "root_cause_hypothesis",
            &request.root_cause_hypothesis,
            4096,
        )?;
        require_bounded_public_text("prevention_change", &request.prevention_change, 4096)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<AccountabilityOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "accountability_repair_planned",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let case = load_accountability_case(&transaction, &request.case_id)?;
        if case.status != AccountabilityStatus::Open {
            return Err(Error::Conflict(
                "repaired case cannot be replanned".to_owned(),
            ));
        }
        let repair_task = load_task(&transaction, &request.repair_task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.repair_task_id)))?;
        if repair_task.project_id != case.project_id
            || repair_task.task_id == case.source_task_id
            || repair_task.created_at_unix_ms < case.created_at_unix_ms
            || matches!(
                repair_task.status,
                TaskStatus::Cancelled | TaskStatus::Completed
            )
        {
            return Err(Error::Conflict(
                "repair task must be a later, unfinished task in the same workspace".to_owned(),
            ));
        }
        transaction.execute(
            "UPDATE accountability_cases
             SET repair_task_id = ?1, root_cause_hypothesis = ?2, prevention_change = ?3,
                 plan_revision = plan_revision + 1 WHERE case_id = ?4",
            params![
                request.repair_task_id,
                request.root_cause_hypothesis,
                request.prevention_change,
                request.case_id
            ],
        )?;
        let outcome = AccountabilityOutcome {
            case: load_accountability_case(&transaction, &request.case_id)?,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.case_id,
            "accountability_repair_planned",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn resolve_accountability_case(
        &self,
        request: &AccountabilityResolveRequest,
    ) -> Result<AccountabilityOutcome> {
        require_text("case_id", &request.case_id)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<AccountabilityOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "accountability_case_repaired",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let case = load_accountability_case(&transaction, &request.case_id)?;
        if case.status != AccountabilityStatus::Open {
            return Err(Error::Conflict("case is already repaired".to_owned()));
        }
        let repair_task_id = case.repair_task_id.as_ref().ok_or_else(|| {
            Error::Conflict("case has no linked repair task and reflection".to_owned())
        })?;
        let repair_task = load_task(&transaction, repair_task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {repair_task_id}")))?;
        if repair_task.status != TaskStatus::Completed || repair_task.completion_proofs.is_empty() {
            return Err(Error::Conflict(
                "repair case requires a completed task with verified criterion proofs".to_owned(),
            ));
        }
        transaction.execute(
            "UPDATE accountability_cases SET status = 'repaired', repaired_at_unix_ms = ?1
             WHERE case_id = ?2",
            params![now, request.case_id],
        )?;
        let outcome = AccountabilityOutcome {
            case: load_accountability_case(&transaction, &request.case_id)?,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.case_id,
            "accountability_case_repaired",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn list_accountability_cases(
        &self,
        request: &AccountabilityListRequest,
    ) -> Result<AccountabilityReport> {
        let workspace = canonical_workspace(&request.workspace)?;
        if let Some(assignee) = &request.reported_assignee_id {
            require_identifier("reported_assignee_id", assignee, 128)?;
        }
        let connection = self.connection()?;
        let project_id = connection
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [&workspace],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(project_id) = project_id else {
            return Ok(AccountabilityReport {
                open_count: 0,
                repaired_count: 0,
                cases: Vec::new(),
                advisory: true,
                authority_notice: accountability_authority_notice(),
            });
        };
        let (open_count, repaired_count): (u64, u64) = connection.query_row(
            "SELECT SUM(CASE WHEN status = 'open' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN status = 'repaired' THEN 1 ELSE 0 END)
             FROM accountability_cases WHERE project_id = ?1
               AND (?2 IS NULL OR reported_assignee_id = ?2)",
            params![project_id, request.reported_assignee_id],
            |row| {
                Ok((
                    row.get::<_, Option<u64>>(0)?.unwrap_or(0),
                    row.get::<_, Option<u64>>(1)?.unwrap_or(0),
                ))
            },
        )?;
        let mut statement = connection.prepare(
            "SELECT case_id FROM accountability_cases WHERE project_id = ?1
               AND (?2 IS NULL OR reported_assignee_id = ?2)
             ORDER BY CASE status WHEN 'open' THEN 0 ELSE 1 END,
                      created_at_unix_ms DESC, case_id DESC LIMIT ?3",
        )?;
        let ids = statement
            .query_map(
                params![
                    project_id,
                    request.reported_assignee_id,
                    i64::from(request.limit.unwrap_or(50).clamp(1, 200))
                ],
                |row| row.get::<_, String>(0),
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let cases = ids
            .iter()
            .map(|id| load_accountability_case(&connection, id))
            .collect::<Result<Vec<_>>>()?;
        Ok(AccountabilityReport {
            open_count,
            repaired_count,
            cases,
            advisory: true,
            authority_notice: accountability_authority_notice(),
        })
    }

    pub fn audit_accountability(&self) -> Result<AccountabilityAudit> {
        let connection = self.connection()?;
        let ids = connection
            .prepare("SELECT case_id FROM accountability_cases ORDER BY sequence")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let invalid_count = ids
            .iter()
            .filter(|id| {
                load_accountability_case(&connection, id)
                    .and_then(|case| validate_accountability_history(&connection, &case))
                    .is_err()
            })
            .count() as u64;
        Ok(AccountabilityAudit {
            case_count: ids.len() as u64,
            invalid_count,
            consistent: invalid_count == 0,
        })
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
            origin_channel: OriginChannel::McpAgent,
            influence_class: InfluenceClass::UntrustedContent,
            created_at_unix_ms: now,
        };
        transaction.execute(
            "INSERT INTO records
             (record_id, session_id, kind, content, evidence, supersedes_record_id,
              verifies_effect_id, origin_channel, influence_class, created_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                record.record_id,
                record.session_id,
                record.kind.as_str(),
                record.content,
                record.evidence,
                record.supersedes_record_id,
                record.verifies_effect_id,
                record.origin_channel.as_str(),
                record.influence_class.as_str(),
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
        if request.artifact_paths.len() > MAX_RECEIPT_ARTIFACTS {
            return Err(Error::Invalid(format!(
                "artifact_paths exceeds the {MAX_RECEIPT_ARTIFACTS}-item limit"
            )));
        }
        if !(1..=3_600).contains(&request.timeout_seconds) {
            return Err(Error::Invalid(
                "timeout_seconds must be between 1 and 3600".to_owned(),
            ));
        }
        crate::sandbox::validate_profile(
            &request.sandbox_profile.enforcement,
            &request.sandbox_profile.workspace_access,
            &request.sandbox_profile.network_access,
        )
        .map_err(|message| Error::Invalid(message.to_owned()))?;
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
            "sandbox_profile": request.sandbox_profile,
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
            sandbox_profile: request.sandbox_profile.clone(),
            success_claim: command_success_claim(&canonical_sha256, request.expected_exit_code),
            canonical_sha256,
            created_at_unix_ms: now,
        };
        transaction.execute(
            "INSERT INTO verification_specs
             (spec_id, session_id, program, args_json, workspace_relative_cwd,
              expected_exit_code, timeout_seconds, artifact_paths_json,
              canonical_sha256, created_at_unix_ms, sandbox_profile_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
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
                serde_json::to_string(&spec.sandbox_profile)?,
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
        if spec.artifact_paths.len() > MAX_RECEIPT_ARTIFACTS {
            return Err(Error::Invalid(format!(
                "verification spec exceeds the artifact count limit of {MAX_RECEIPT_ARTIFACTS}"
            )));
        }
        crate::sandbox::validate_profile(
            &spec.sandbox_profile.enforcement,
            &spec.sandbox_profile.workspace_access,
            &spec.sandbox_profile.network_access,
        )
        .map_err(|message| Error::Invalid(message.to_owned()))?;
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
        require_text("sandbox_backend", &finish.sandbox_backend)?;
        if finish.status == ExecutionStatus::Succeeded
            && spec.sandbox_profile.enforcement == SandboxEnforcement::Required
            && !finish.sandbox_enforced
        {
            return Err(Error::Invalid(
                "required sandbox execution cannot succeed without enforcement evidence".to_owned(),
            ));
        }
        if finish.artifacts.len() > MAX_RECEIPT_ARTIFACTS
            || finish
                .artifacts
                .iter()
                .any(|artifact| artifact.byte_length > MAX_RECEIPT_ARTIFACT_BYTES)
            || finish.artifacts.iter().fold(0_u64, |total, artifact| {
                total.saturating_add(artifact.byte_length)
            }) > MAX_RECEIPT_ARTIFACT_TOTAL_BYTES
        {
            return Err(Error::Invalid(
                "receipt artifacts exceed the configured count or byte limits".to_owned(),
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
            sandbox_backend: finish.sandbox_backend.clone(),
            sandbox_enforced: finish.sandbox_enforced,
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
              termination, sandbox_backend, sandbox_enforced,
              stdout_sha256, stdout_bytes, stderr_sha256, stderr_bytes,
              git_head_before, git_head_after, worktree_state_before_sha256,
              worktree_state_after_sha256, created_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                receipt.receipt_id,
                receipt.run_id,
                receipt.command_spec_sha256,
                receipt.resolved_executable,
                receipt.exit_code,
                receipt.termination,
                receipt.sandbox_backend,
                receipt.sandbox_enforced,
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
        let write_scope = normalize_write_scopes(&request.write_scope)?;
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
                serde_json::to_string(&write_scope)?,
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

    pub fn memory_search(&self, request: &MemorySearchRequest) -> Result<MemorySearchResult> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let project_id = connection
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [&workspace],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let terms = memory_query_terms(&request.query);
        let limit = request.limit.unwrap_or(12).clamp(1, 50);
        let max_bytes = request.max_bytes.unwrap_or(8_192).clamp(256, 65_536);
        let Some(project_id) = project_id else {
            return Ok(MemorySearchResult {
                project_id: None,
                workspace,
                query_terms: terms,
                items: Vec::new(),
                budget: crate::domain::ContextBudget {
                    max_items: limit,
                    max_content_bytes: max_bytes,
                    used_content_bytes: 0,
                    omitted_items: 0,
                    candidate_items: 0,
                    deduplicated_items: 0,
                    oversized_items: 0,
                    item_limit_items: 0,
                    conservative_input_token_upper_bound: 0,
                    token_estimate_source: "conservative_utf8_byte_upper_bound".to_owned(),
                },
                warnings: vec!["project_not_found".to_owned()],
                policy_sha256: crate::context::policy_sha256(),
            });
        };
        let as_of = request.as_of_unix_ms.unwrap_or(unix_millis()?);
        let mut candidates = if terms.is_empty() {
            load_current_memory(&connection, &project_id, as_of)?
        } else {
            load_matching_memory(&connection, &project_id, as_of, &terms)?
        };
        if !request.classes.is_empty() {
            candidates.retain(|item| request.classes.contains(&item.memory_class));
        }
        let candidate_count = candidates.len();
        let mut items = Vec::new();
        let mut used = 0usize;
        let mut seen_content = std::collections::BTreeSet::new();
        let mut deduplicated = 0usize;
        let mut oversized = 0usize;
        let mut item_limited = 0usize;
        for mut item in candidates {
            if items.len() >= limit as usize {
                item_limited += 1;
                continue;
            }
            let content_sha256 = format!("{:x}", Sha256::digest(item.content.as_bytes()));
            if !seen_content.insert(content_sha256) {
                deduplicated += 1;
                continue;
            }
            let bytes = item.content.len();
            if used + bytes > max_bytes as usize {
                oversized += 1;
                continue;
            }
            item.selection_reasons.push(if terms.is_empty() {
                "current_memory".to_owned()
            } else {
                "fts_match".to_owned()
            });
            item.selection_reasons
                .push(format!("class:{}", item.memory_class.as_str()));
            if item.influence_class == InfluenceClass::VerifiedFact {
                item.selection_reasons.push("verified_fact".to_owned());
            }
            used += bytes;
            items.push(item);
        }
        let selected = items.len();
        let mut warnings = Vec::new();
        if items.is_empty() {
            warnings.push("no_matching_memory".to_owned());
        }
        if items
            .iter()
            .any(|item| item.memory_class == MemoryClass::Unknown)
        {
            warnings.push("contains_unresolved_unknown".to_owned());
        }
        Ok(MemorySearchResult {
            project_id: Some(project_id),
            workspace,
            query_terms: terms,
            items,
            budget: crate::domain::ContextBudget {
                max_items: limit,
                max_content_bytes: max_bytes,
                used_content_bytes: u32::try_from(used).unwrap_or(u32::MAX),
                omitted_items: u32::try_from(candidate_count.saturating_sub(selected))
                    .unwrap_or(u32::MAX),
                candidate_items: u32::try_from(candidate_count).unwrap_or(u32::MAX),
                deduplicated_items: u32::try_from(deduplicated).unwrap_or(u32::MAX),
                oversized_items: u32::try_from(oversized).unwrap_or(u32::MAX),
                item_limit_items: u32::try_from(item_limited).unwrap_or(u32::MAX),
                conservative_input_token_upper_bound: u32::try_from(used).unwrap_or(u32::MAX),
                token_estimate_source: "conservative_utf8_byte_upper_bound".to_owned(),
            },
            warnings,
            policy_sha256: crate::context::policy_sha256(),
        })
    }

    pub fn memory_get(&self, request: &MemoryGetRequest) -> Result<MemoryItem> {
        require_text("memory_id", &request.memory_id)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT memory_items.memory_id, memory_items.source_kind,
                        memory_items.source_id, memory_items.memory_class,
                        memory_items.content, memory_items.origin_channel,
                        memory_items.influence_class, memory_items.source_status,
                        memory_items.lifecycle_state, memory_items.valid_from_unix_ms,
                        memory_items.valid_until_unix_ms, memory_items.applicability_json,
                        memory_items.created_at_unix_ms, memory_items.updated_at_unix_ms
                 FROM memory_items JOIN projects USING(project_id)
                 WHERE projects.workspace = ?1 AND memory_items.memory_id = ?2",
                params![workspace, request.memory_id],
                memory_item_from_row,
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("memory {}", request.memory_id)))
    }

    pub fn audit_memory_projection(&self) -> Result<MemoryProjectionAudit> {
        let connection = self.connection()?;
        let expected_source_count = connection.query_row(
            "SELECT
                (SELECT count(*) FROM records) +
                (SELECT count(*) FROM claims) +
                (SELECT count(*) FROM tasks) +
                (SELECT count(*) FROM sessions
                   WHERE status = 'handoff' AND summary IS NOT NULL AND next_action IS NOT NULL) +
                (SELECT count(*) FROM execution_receipts)",
            [],
            |row| row.get::<_, u64>(0),
        )?;
        let item_count = table_count(&connection, "memory_items", "1 = 1")?;
        let active_count = table_count(&connection, "memory_items", "lifecycle_state = 'active'")?;
        let superseded_count = table_count(
            &connection,
            "memory_items",
            "lifecycle_state = 'superseded'",
        )?;
        let untrusted_count = table_count(
            &connection,
            "memory_items",
            "influence_class = 'untrusted_content'",
        )?;
        let fts_count = table_count(&connection, "memory_fts", "1 = 1")?;
        let source_consistent = expected_source_count == item_count;
        Ok(MemoryProjectionAudit {
            expected_source_count,
            item_count,
            active_count,
            superseded_count,
            untrusted_count,
            fts_count,
            source_consistent,
            consistent: source_consistent && item_count == fts_count,
        })
    }

    pub(crate) fn record_memory_exposure(
        &self,
        workspace: &str,
        event_kind: &str,
        host_session_id: Option<&str>,
        host_turn_id: Option<&str>,
        memory_ids: &[String],
        content_bytes: u32,
    ) -> Result<Option<String>> {
        let workspace = canonical_workspace(workspace)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let project_id = transaction
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [&workspace],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(project_id) = project_id else {
            return Ok(None);
        };
        let secret = installation_secret(&transaction, now)?;
        let exposure_id = Uuid::now_v7().to_string();
        transaction.execute(
            "INSERT INTO memory_exposures(
                exposure_id, project_id, event_kind, host_session_hmac, host_turn_hmac,
                policy_sha256, memory_ids_json, content_bytes, created_at_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                exposure_id,
                project_id,
                event_kind,
                hmac_text(&secret, host_session_id)?,
                hmac_text(&secret, host_turn_id)?,
                crate::context::policy_sha256(),
                serde_json::to_string(memory_ids)?,
                content_bytes,
                now,
            ],
        )?;
        transaction.commit()?;
        Ok(Some(exposure_id))
    }

    pub(crate) fn record_runtime_observation(
        &self,
        observation: &RuntimeObservation,
    ) -> Result<Option<RuntimeEvent>> {
        let workspace = canonical_workspace(&observation.workspace)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let project_id = transaction
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [&workspace],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(project_id) = project_id else {
            return Ok(None);
        };
        let secret = installation_secret(&transaction, now)?;
        let host_session_hmac = hmac_text(&secret, observation.host_session_id.as_deref())?;
        let host_turn_hmac = hmac_text(&secret, observation.host_turn_id.as_deref())?;
        let host_tool_call_hmac = hmac_text(&secret, observation.host_tool_call_id.as_deref())?;
        let (input_hmac, input_bytes) = hmac_json(&secret, observation.input.as_ref())?;
        let (output_hmac, output_bytes) = hmac_json(&secret, observation.output.as_ref())?;
        let exposure_id = if observation.exposure_id.is_some() {
            observation.exposure_id.clone()
        } else if host_session_hmac.is_some() || host_turn_hmac.is_some() {
            transaction
                .query_row(
                    "SELECT exposure_id FROM memory_exposures
                     WHERE project_id = ?1
                       AND host_session_hmac IS ?2
                       AND host_turn_hmac IS ?3
                     ORDER BY created_at_unix_ms DESC LIMIT 1",
                    params![project_id, host_session_hmac, host_turn_hmac],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
        } else {
            None
        };
        let duplicate_of_event_id = transaction
            .query_row(
                "SELECT event_id FROM runtime_events
                 WHERE project_id = ?1 AND event_kind = ?2
                   AND host_session_hmac IS ?3 AND host_turn_hmac IS ?4
                   AND host_tool_call_hmac IS ?5 AND input_hmac IS ?6 AND output_hmac IS ?7
                 ORDER BY sequence ASC LIMIT 1",
                params![
                    project_id,
                    observation.event_kind.as_str(),
                    host_session_hmac,
                    host_turn_hmac,
                    host_tool_call_hmac,
                    input_hmac,
                    output_hmac,
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let event_id = Uuid::now_v7().to_string();
        transaction.execute(
            "INSERT INTO runtime_events(
                event_id, project_id, exposure_id, host_provider, event_kind,
                host_session_hmac, host_turn_hmac, host_tool_call_hmac, tool_name,
                capability_class, outcome_status, input_hmac, input_bytes,
                output_hmac, output_bytes, latency_ms, hook_schema_version,
                shadow_disposition, shadow_reasons_json, duplicate_of_event_id,
                received_at_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                       ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                event_id,
                project_id,
                exposure_id,
                observation.host_provider,
                observation.event_kind.as_str(),
                host_session_hmac,
                host_turn_hmac,
                host_tool_call_hmac,
                observation.tool_name,
                observation.capability_class.as_str(),
                observation.outcome_status.as_str(),
                input_hmac,
                input_bytes,
                output_hmac,
                output_bytes,
                observation.latency_ms,
                observation.hook_schema_version,
                observation.shadow_disposition.as_str(),
                serde_json::to_string(&observation.shadow_reasons)?,
                duplicate_of_event_id,
                now,
            ],
        )?;
        let sequence = u64::try_from(transaction.last_insert_rowid())
            .map_err(|_| Error::Invalid("runtime event sequence overflow".to_owned()))?;
        let event = RuntimeEvent {
            sequence,
            event_id,
            exposure_id,
            host_provider: observation.host_provider.clone(),
            event_kind: observation.event_kind.clone(),
            host_session_hmac,
            host_turn_hmac,
            host_tool_call_hmac,
            tool_name: observation.tool_name.clone(),
            capability_class: observation.capability_class.clone(),
            outcome_status: observation.outcome_status.clone(),
            input_hmac,
            input_bytes,
            output_hmac,
            output_bytes,
            latency_ms: observation.latency_ms,
            hook_schema_version: observation.hook_schema_version.clone(),
            shadow_disposition: observation.shadow_disposition.clone(),
            shadow_reasons: observation.shadow_reasons.clone(),
            duplicate_of_event_id,
            received_at_unix_ms: now,
        };
        transaction.commit()?;
        Ok(Some(event))
    }

    pub fn list_runtime_events(
        &self,
        request: &RuntimeTraceListRequest,
    ) -> Result<Vec<RuntimeEvent>> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let project_id = project_id_for_workspace(&connection, &workspace)?;
        let Some(project_id) = project_id else {
            return Ok(Vec::new());
        };
        let limit = request.limit.unwrap_or(100).clamp(1, 500) as usize;
        let mut statement = connection.prepare(&format!(
            "{} FROM runtime_events WHERE project_id = ?1 ORDER BY sequence DESC LIMIT 2000",
            runtime_event_select()
        ))?;
        let mut events = statement
            .query_map([project_id], runtime_event_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if !request.event_kinds.is_empty() {
            events.retain(|event| request.event_kinds.contains(&event.event_kind));
        }
        events.truncate(limit);
        Ok(events)
    }

    pub fn get_runtime_event(&self, request: &RuntimeTraceGetRequest) -> Result<RuntimeEvent> {
        require_text("event_id", &request.event_id)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        connection
            .query_row(
                &format!(
                    "{} FROM runtime_events JOIN projects USING(project_id)
                     WHERE projects.workspace = ?1 AND runtime_events.event_id = ?2",
                    runtime_event_select()
                ),
                params![workspace, request.event_id],
                runtime_event_from_row,
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("runtime event {}", request.event_id)))
    }

    pub fn capability_report(&self, request: &RuntimeWorkspaceRequest) -> Result<CapabilityReport> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let project_id = project_id_for_workspace(&connection, &workspace)?;
        let Some(project_id) = project_id else {
            return Ok(CapabilityReport {
                project_id: None,
                workspace,
                observations: Vec::new(),
                authority_notice: runtime_authority_notice().to_owned(),
            });
        };
        let observations = load_capability_observations(&connection, &project_id)?;
        Ok(CapabilityReport {
            project_id: Some(project_id),
            workspace,
            observations,
            authority_notice: runtime_authority_notice().to_owned(),
        })
    }

    pub fn hook_health(&self, request: &RuntimeWorkspaceRequest) -> Result<HookHealthReport> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let project_id = project_id_for_workspace(&connection, &workspace)?;
        let Some(project_id) = project_id else {
            return Ok(HookHealthReport {
                project_id: None,
                workspace,
                event_count: 0,
                unmatched_pre_tool_count: 0,
                terminal_without_pre_count: 0,
                duplicate_count: 0,
                unknown_event_count: 0,
                unknown_capability_count: 0,
                last_event_at_unix_ms: None,
                no_detected_gaps: false,
                coverage_proven: false,
                warnings: vec!["project_not_found".to_owned()],
            });
        };
        let event_count = count_runtime(&connection, &project_id, "1 = 1")?;
        let duplicate_count = count_runtime(
            &connection,
            &project_id,
            "duplicate_of_event_id IS NOT NULL",
        )?;
        let unknown_event_count =
            count_runtime(&connection, &project_id, "event_kind = 'unknown'")?;
        let unknown_capability_count = count_runtime(
            &connection,
            &project_id,
            "tool_name IS NOT NULL AND capability_class = 'unknown'",
        )?;
        let unmatched_pre_tool_count = connection.query_row(
            "SELECT count(*) FROM runtime_events pre
             WHERE pre.project_id = ?1 AND pre.event_kind = 'pre_tool'
               AND (pre.host_tool_call_hmac IS NULL OR NOT EXISTS (
                    SELECT 1 FROM runtime_events terminal
                    WHERE terminal.project_id = pre.project_id
                      AND terminal.host_tool_call_hmac = pre.host_tool_call_hmac
                      AND terminal.event_kind IN ('post_tool', 'tool_failure')
                      AND terminal.sequence > pre.sequence
               ))",
            [&project_id],
            |row| row.get::<_, u64>(0),
        )?;
        let terminal_without_pre_count = connection.query_row(
            "SELECT count(*) FROM runtime_events terminal
             WHERE terminal.project_id = ?1
               AND terminal.event_kind IN ('post_tool', 'tool_failure')
               AND (terminal.host_tool_call_hmac IS NULL OR NOT EXISTS (
                    SELECT 1 FROM runtime_events pre
                    WHERE pre.project_id = terminal.project_id
                      AND pre.host_tool_call_hmac = terminal.host_tool_call_hmac
                      AND pre.event_kind = 'pre_tool' AND pre.sequence < terminal.sequence
               ))",
            [&project_id],
            |row| row.get::<_, u64>(0),
        )?;
        let last_event_at_unix_ms = connection.query_row(
            "SELECT max(received_at_unix_ms) FROM runtime_events WHERE project_id = ?1",
            [&project_id],
            |row| row.get::<_, Option<i64>>(0),
        )?;
        let mut warnings = Vec::new();
        if unmatched_pre_tool_count > 0 {
            warnings.push("unmatched_pre_tool".to_owned());
        }
        if terminal_without_pre_count > 0 {
            warnings.push("terminal_without_pre_tool".to_owned());
        }
        if duplicate_count > 0 {
            warnings.push("duplicate_events".to_owned());
        }
        if unknown_event_count > 0 || unknown_capability_count > 0 {
            warnings.push("unknown_host_schema_or_capability".to_owned());
        }
        Ok(HookHealthReport {
            project_id: Some(project_id),
            workspace,
            event_count,
            unmatched_pre_tool_count,
            terminal_without_pre_count,
            duplicate_count,
            unknown_event_count,
            unknown_capability_count,
            last_event_at_unix_ms,
            no_detected_gaps: warnings.is_empty(),
            coverage_proven: false,
            warnings,
        })
    }

    pub fn audit_runtime_projection(&self) -> Result<RuntimeProjectionAudit> {
        let connection = self.connection()?;
        let event_tool_group_count = connection.query_row(
            "SELECT count(*) FROM (
                SELECT project_id, host_provider, tool_name, capability_class
                FROM runtime_events WHERE tool_name IS NOT NULL
                GROUP BY project_id, host_provider, tool_name, capability_class
             )",
            [],
            |row| row.get::<_, u64>(0),
        )?;
        let capability_projection_count =
            table_count(&connection, "capability_observations", "1 = 1")?;
        Ok(RuntimeProjectionAudit {
            event_tool_group_count,
            capability_projection_count,
            consistent: event_tool_group_count == capability_projection_count,
        })
    }

    pub fn export_runtime_otel(
        &self,
        request: &RuntimeWorkspaceRequest,
    ) -> Result<serde_json::Value> {
        let events = self.list_runtime_events(&RuntimeTraceListRequest {
            workspace: request.workspace.clone(),
            limit: Some(500),
            event_kinds: Vec::new(),
        })?;
        let spans = events
            .into_iter()
            .rev()
            .map(|event| {
                let trace_source = event
                    .host_session_hmac
                    .as_deref()
                    .unwrap_or(&event.event_id);
                serde_json::json!({
                    "name": format!("gen_ai.{}", event.event_kind.as_str()),
                    "span_id": otel_hex_id(&event.event_id, 16),
                    "trace_id": otel_hex_id(trace_source, 32),
                    "parent_span_id": event.host_turn_hmac.as_deref().map(|id| otel_hex_id(id, 16)),
                    "start_time_unix_ms": event.received_at_unix_ms,
                    "attributes": {
                        "gen_ai.operation.name": match event.event_kind {
                            RuntimeEventKind::PreTool | RuntimeEventKind::PostTool | RuntimeEventKind::ToolFailure => "execute_tool",
                            _ => "invoke_agent"
                        },
                        "gen_ai.tool.name": event.tool_name,
                        "aporic.capability.class": event.capability_class.as_str(),
                        "aporic.outcome.status": event.outcome_status.as_str(),
                        "aporic.shadow.disposition": event.shadow_disposition.as_str(),
                        "aporic.payload.content_recorded": false,
                        "aporic.input.bytes": event.input_bytes,
                        "aporic.output.bytes": event.output_bytes
                    }
                })
            })
            .collect::<Vec<_>>();
        Ok(serde_json::json!({
            "format": "aporic-otel-json-v1",
            "network_exported": false,
            "content_recorded": false,
            "spans": spans
        }))
    }

    pub(crate) fn record_git_snapshot(&self, draft: &GitSnapshotDraft) -> Result<GitSnapshot> {
        let workspace = canonical_workspace(&draft.workspace)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let project_id = find_or_create_project(&transaction, &workspace, now)?;
        let successful_receipt_bound = match draft.head_commit.as_deref() {
            Some(head) => transaction.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM execution_receipts receipts
                    JOIN execution_runs runs ON runs.run_id = receipts.run_id
                    JOIN sessions ON sessions.session_id = runs.session_id
                    WHERE sessions.project_id = ?1 AND runs.status = 'succeeded'
                      AND receipts.git_head_after = ?2
                 )",
                params![project_id, head],
                |row| row.get::<_, bool>(0),
            )?,
            None => false,
        };
        let findings = crate::git::assess_governance(draft, successful_receipt_bound);
        let snapshot_id = Uuid::now_v7().to_string();
        let mut snapshot = GitSnapshot {
            sequence: 0,
            snapshot_id,
            repository_root: draft.repository_root.clone(),
            head_commit: draft.head_commit.clone(),
            head_tree: draft.head_tree.clone(),
            branch: draft.branch.clone(),
            detached: draft.detached,
            upstream_ref: draft.upstream_ref.clone(),
            base_ref: draft.base_ref.clone(),
            base_commit: draft.base_commit.clone(),
            merge_base: draft.merge_base.clone(),
            ahead_count: draft.ahead_count,
            behind_count: draft.behind_count,
            remote_state_fresh: false,
            dirty: draft.dirty,
            staged_count: draft.staged_count,
            unstaged_count: draft.unstaged_count,
            untracked_count: draft.untracked_count,
            local_branch_count: draft.local_branch_count,
            head_parent_count: draft.head_parent_count,
            head_has_signature: draft.head_has_signature,
            successful_receipt_bound,
            paths_truncated: draft.paths_truncated,
            changed_paths: draft.changed_paths.clone(),
            worktrees: draft.worktrees.clone(),
            remotes: draft.remotes.clone(),
            findings,
            policy_version: crate::git::GIT_POLICY_VERSION,
            snapshot_sha256: String::new(),
            captured_at_unix_ms: now,
            approval_proven: false,
            authority_notice: git_authority_notice().to_owned(),
        };
        snapshot.snapshot_sha256 = git_snapshot_digest(&snapshot)?;
        transaction.execute(
            "INSERT INTO git_snapshots(
                snapshot_id, project_id, repository_root, head_commit, head_tree, branch,
                detached, upstream_ref, base_ref, base_commit, merge_base, ahead_count,
                behind_count, remote_state_fresh, dirty, staged_count, unstaged_count,
                untracked_count, local_branch_count, head_parent_count, head_has_signature,
                successful_receipt_bound, paths_truncated, changed_paths_json, worktrees_json,
                remotes_json, findings_json, policy_version, snapshot_sha256,
                captured_at_unix_ms
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 0,
                ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26,
                ?27, ?28, ?29
             )",
            params![
                snapshot.snapshot_id,
                project_id,
                snapshot.repository_root,
                snapshot.head_commit,
                snapshot.head_tree,
                snapshot.branch,
                snapshot.detached,
                snapshot.upstream_ref,
                snapshot.base_ref,
                snapshot.base_commit,
                snapshot.merge_base,
                snapshot.ahead_count,
                snapshot.behind_count,
                snapshot.dirty,
                snapshot.staged_count,
                snapshot.unstaged_count,
                snapshot.untracked_count,
                snapshot.local_branch_count,
                snapshot.head_parent_count,
                snapshot.head_has_signature,
                snapshot.successful_receipt_bound,
                snapshot.paths_truncated,
                serde_json::to_string(&snapshot.changed_paths)?,
                serde_json::to_string(&snapshot.worktrees)?,
                serde_json::to_string(&snapshot.remotes)?,
                serde_json::to_string(&snapshot.findings)?,
                snapshot.policy_version,
                snapshot.snapshot_sha256,
                snapshot.captured_at_unix_ms,
            ],
        )?;
        snapshot.sequence = u64::try_from(transaction.last_insert_rowid())
            .map_err(|_| Error::Invalid("Git snapshot sequence overflow".to_owned()))?;
        transaction.commit()?;
        Ok(snapshot)
    }

    pub fn list_git_snapshots(&self, request: &GitSnapshotListRequest) -> Result<Vec<GitSnapshot>> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let Some(project_id) = project_id_for_workspace(&connection, &workspace)? else {
            return Ok(Vec::new());
        };
        let limit = request.limit.unwrap_or(50).clamp(1, 200);
        let mut statement = connection.prepare(&format!(
            "{} FROM git_snapshots WHERE project_id = ?1 ORDER BY sequence DESC LIMIT ?2",
            git_snapshot_select()
        ))?;
        Ok(statement
            .query_map(params![project_id, limit], git_snapshot_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn get_git_snapshot(&self, request: &GitSnapshotGetRequest) -> Result<GitSnapshot> {
        require_text("snapshot_id", &request.snapshot_id)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        connection
            .query_row(
                &format!(
                    "{} FROM git_snapshots JOIN projects USING(project_id)
                     WHERE projects.workspace = ?1 AND git_snapshots.snapshot_id = ?2",
                    git_snapshot_select()
                ),
                params![workspace, request.snapshot_id],
                git_snapshot_from_row,
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("Git snapshot {}", request.snapshot_id)))
    }

    pub fn audit_git_snapshots(&self) -> Result<GitSnapshotAudit> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(&format!(
            "{} FROM git_snapshots ORDER BY sequence ASC",
            git_snapshot_select()
        ))?;
        let rows = statement.query_map([], git_snapshot_from_row)?;
        let mut snapshot_count = 0;
        let mut digest_mismatch_count = 0;
        let mut invalid_json_count = 0;
        for row in rows {
            snapshot_count += 1;
            match row {
                Ok(snapshot) => {
                    if git_snapshot_digest(&snapshot)? != snapshot.snapshot_sha256 {
                        digest_mismatch_count += 1;
                    }
                }
                Err(_) => invalid_json_count += 1,
            }
        }
        Ok(GitSnapshotAudit {
            snapshot_count,
            digest_mismatch_count,
            invalid_json_count,
            consistent: digest_mismatch_count == 0 && invalid_json_count == 0,
        })
    }

    pub fn record_token_usage(
        &self,
        request: &TokenUsageRecordRequest,
    ) -> Result<TokenUsageOutcome> {
        require_text("scope_kind", &request.scope_kind)?;
        require_text("scope_id", &request.scope_id)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        if request.scope_kind.len() > 64 || request.scope_id.len() > 512 {
            return Err(Error::Invalid(
                "usage scope_kind or scope_id exceeds its bounded length".to_owned(),
            ));
        }
        let counts = [
            request.input_tokens,
            request.output_tokens,
            request.cached_input_tokens,
            request.reasoning_tokens,
            request.context_bytes,
        ];
        if counts
            .iter()
            .flatten()
            .any(|value| *value > i64::MAX as u64)
        {
            return Err(Error::Invalid(
                "usage count exceeds SQLite range".to_owned(),
            ));
        }
        if request.cached_input_tokens.unwrap_or(0) > request.input_tokens.unwrap_or(0) {
            return Err(Error::Invalid(
                "cached_input_tokens cannot exceed input_tokens".to_owned(),
            ));
        }
        if matches!(
            request.outcome,
            UsageOutcome::VerifiedSuccess | UsageOutcome::VerifiedFailure
        ) && request
            .verification_ref
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(Error::Invalid(
                "verified usage outcomes require verification_ref".to_owned(),
            ));
        }
        match request.source_kind {
            TokenCountSource::HostReported | TokenCountSource::LocalTokenizer => {
                if request.input_tokens.is_none()
                    && request.output_tokens.is_none()
                    && request.reasoning_tokens.is_none()
                {
                    return Err(Error::Invalid(
                        "measured usage requires at least one token count".to_owned(),
                    ));
                }
            }
            TokenCountSource::ConservativeByteUpperBound => {
                if request.context_bytes.is_none() || request.input_tokens.is_some() {
                    return Err(Error::Invalid(
                        "byte upper-bound usage requires context_bytes and must not claim input_tokens"
                            .to_owned(),
                    ));
                }
            }
            TokenCountSource::Unknown => {
                if counts[..4].iter().any(Option::is_some) {
                    return Err(Error::Invalid(
                        "unknown token provenance cannot carry token counts".to_owned(),
                    ));
                }
            }
        }

        let workspace = canonical_workspace(&request.workspace)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<TokenUsageOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "token_usage_recorded",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let project_id = find_or_create_project(&transaction, &workspace, now)?;
        if let Some(session_id) = request.session_id.as_deref() {
            let owner = transaction
                .query_row(
                    "SELECT project_id FROM sessions WHERE session_id = ?1",
                    [session_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .ok_or_else(|| Error::NotFound(format!("session {session_id}")))?;
            if owner != project_id {
                return Err(Error::Conflict(
                    "usage session belongs to another workspace".to_owned(),
                ));
            }
        }
        if let Some(reference) = request.verification_ref.as_deref() {
            let reference_valid = match request.outcome {
                UsageOutcome::VerifiedSuccess => transaction.query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM execution_runs runs
                        JOIN sessions ON sessions.session_id = runs.session_id
                        LEFT JOIN execution_receipts receipts ON receipts.run_id = runs.run_id
                        WHERE sessions.project_id = ?1 AND runs.status = 'succeeded'
                          AND (runs.run_id = ?2 OR receipts.receipt_id = ?2)
                        UNION ALL
                        SELECT 1 FROM claims
                        JOIN sessions ON sessions.session_id = claims.session_id
                        WHERE sessions.project_id = ?1 AND claims.status = 'verified'
                          AND claims.claim_id = ?2
                    )",
                    params![project_id, reference],
                    |row| row.get::<_, bool>(0),
                )?,
                UsageOutcome::VerifiedFailure => transaction.query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM execution_runs runs
                        JOIN sessions ON sessions.session_id = runs.session_id
                        WHERE sessions.project_id = ?1
                          AND runs.status IN ('failed', 'timed_out', 'interrupted')
                          AND runs.run_id = ?2
                    )",
                    params![project_id, reference],
                    |row| row.get::<_, bool>(0),
                )?,
                UsageOutcome::Unverified => true,
            };
            if !reference_valid {
                return Err(Error::Conflict(
                    "verification_ref is not a matching Aporic-direct outcome in this workspace"
                        .to_owned(),
                ));
            }
        }
        let mut receipt = TokenUsageReceipt {
            sequence: 0,
            receipt_id: Uuid::now_v7().to_string(),
            session_id: request.session_id.clone(),
            scope_kind: request.scope_kind.trim().to_owned(),
            scope_id: request.scope_id.trim().to_owned(),
            model: request
                .model
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_owned),
            source_kind: request.source_kind.clone(),
            input_tokens: request.input_tokens,
            output_tokens: request.output_tokens,
            cached_input_tokens: request.cached_input_tokens,
            reasoning_tokens: request.reasoning_tokens,
            context_bytes: request.context_bytes,
            outcome: request.outcome.clone(),
            verification_ref: request.verification_ref.clone(),
            receipt_sha256: String::new(),
            recorded_at_unix_ms: now,
        };
        receipt.receipt_sha256 = token_usage_digest(&receipt)?;
        transaction.execute(
            "INSERT INTO token_usage_receipts(
                receipt_id, project_id, session_id, scope_kind, scope_id, model, source_kind,
                input_tokens, output_tokens, cached_input_tokens, reasoning_tokens,
                context_bytes, outcome, verification_ref, receipt_sha256, recorded_at_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                receipt.receipt_id,
                project_id,
                receipt.session_id,
                receipt.scope_kind,
                receipt.scope_id,
                receipt.model,
                receipt.source_kind.as_str(),
                receipt.input_tokens,
                receipt.output_tokens,
                receipt.cached_input_tokens,
                receipt.reasoning_tokens,
                receipt.context_bytes,
                receipt.outcome.as_str(),
                receipt.verification_ref,
                receipt.receipt_sha256,
                receipt.recorded_at_unix_ms,
            ],
        )?;
        receipt.sequence = u64::try_from(transaction.last_insert_rowid())
            .map_err(|_| Error::Invalid("usage receipt sequence overflow".to_owned()))?;
        let outcome = TokenUsageOutcome {
            receipt,
            duplicate: false,
        };
        let stream_id = request.session_id.as_deref().unwrap_or(&project_id);
        append_event(
            &transaction,
            &request.idempotency_key,
            stream_id,
            "token_usage_recorded",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn list_token_usage(
        &self,
        request: &TokenUsageListRequest,
    ) -> Result<Vec<TokenUsageReceipt>> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let Some(project_id) = project_id_for_workspace(&connection, &workspace)? else {
            return Ok(Vec::new());
        };
        let limit = request.limit.unwrap_or(100).clamp(1, 500);
        let mut statement = connection.prepare(&format!(
            "{} FROM token_usage_receipts WHERE project_id = ?1 ORDER BY sequence DESC LIMIT ?2",
            token_usage_select()
        ))?;
        Ok(statement
            .query_map(params![project_id, limit], token_usage_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn token_efficiency_report(
        &self,
        request: &TokenEfficiencyReportRequest,
    ) -> Result<TokenEfficiencyReport> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let Some(project_id) = project_id_for_workspace(&connection, &workspace)? else {
            return Ok(empty_token_efficiency_report());
        };
        let mut statement = connection.prepare(&format!(
            "{} FROM token_usage_receipts WHERE project_id = ?1 ORDER BY sequence ASC",
            token_usage_select()
        ))?;
        let receipts = statement
            .query_map([project_id], token_usage_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(build_token_efficiency_report(&receipts))
    }

    pub fn audit_token_usage(&self) -> Result<TokenUsageAudit> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(&format!(
            "{} FROM token_usage_receipts ORDER BY sequence ASC",
            token_usage_select()
        ))?;
        let receipts = statement
            .query_map([], token_usage_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let digest_mismatch_count = receipts
            .iter()
            .filter(|receipt| {
                token_usage_digest(receipt)
                    .map(|digest| digest != receipt.receipt_sha256)
                    .unwrap_or(true)
            })
            .count() as u64;
        Ok(TokenUsageAudit {
            receipt_count: receipts.len() as u64,
            digest_mismatch_count,
            consistent: digest_mismatch_count == 0,
        })
    }

    pub fn register_capability(
        &self,
        request: &CapabilityRegisterRequest,
    ) -> Result<CapabilityOutcome> {
        require_text("session_id", &request.session_id)?;
        require_identifier("capability_id", &request.capability_id, 128)?;
        require_identifier("version", &request.version, 64)?;
        require_bounded_public_text("title", &request.title, 256)?;
        require_bounded_public_text("description", &request.description, 2_048)?;
        require_bounded_public_text("evidence_contract", &request.evidence_contract, 2_048)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        validate_capability_schema("input_schema", &request.input_schema, true)?;
        validate_capability_schema("output_schema", &request.output_schema, false)?;
        let maturity = request
            .maturity
            .clone()
            .unwrap_or_else(|| minimum_capability_maturity(&request.effect_class));
        validate_capability_maturity(&request.effect_class, &maturity, request.idempotent)?;
        if request
            .implementation_sha256
            .as_deref()
            .is_some_and(|value| !is_sha256(value))
        {
            return Err(Error::Invalid(
                "implementation_sha256 must be 64 lowercase hexadecimal characters".to_owned(),
            ));
        }
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<CapabilityOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "capability_registered",
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
        let input_schema_json = serde_json::to_string(&request.input_schema)?;
        let output_schema_json = serde_json::to_string(&request.output_schema)?;
        let manifest_sha256 = digest_json(&serde_json::json!({
            "capability_id": request.capability_id.trim(),
            "version": request.version.trim(),
            "provider_kind": request.provider_kind.as_str(),
            "title": request.title.trim(),
            "description": request.description.trim(),
            "effect_class": request.effect_class.as_str(),
            "maturity": maturity.as_str(),
            "reads_private_data": request.reads_private_data,
            "sees_untrusted_content": request.sees_untrusted_content,
            "uses_network": request.uses_network,
            "requires_credentials": request.requires_credentials,
            "idempotent": request.idempotent,
            "reversible": request.reversible,
            "input_schema": request.input_schema,
            "output_schema": request.output_schema,
            "evidence_contract": request.evidence_contract.trim(),
            "implementation_sha256": request.implementation_sha256,
        }))?;
        transaction.execute(
            "INSERT INTO capability_manifests(
                capability_id, version, project_id, provider_kind, title, description,
                effect_class, maturity, reads_private_data, sees_untrusted_content, uses_network,
                requires_credentials, idempotent, reversible, input_schema_json,
                output_schema_json, evidence_contract, implementation_sha256,
                manifest_sha256, created_at_unix_ms, manifest_digest_version
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                       ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, 2)",
            params![
                request.capability_id.trim(),
                request.version.trim(),
                project_id,
                request.provider_kind.as_str(),
                request.title.trim(),
                request.description.trim(),
                request.effect_class.as_str(),
                maturity.as_str(),
                request.reads_private_data,
                request.sees_untrusted_content,
                request.uses_network,
                request.requires_credentials,
                request.idempotent,
                request.reversible,
                input_schema_json,
                output_schema_json,
                request.evidence_contract.trim(),
                request.implementation_sha256,
                manifest_sha256,
                now,
            ],
        )?;
        let event_id = Uuid::now_v7().to_string();
        let event_sha256 = digest_json(&serde_json::json!({
            "capability_id": request.capability_id.trim(),
            "version": request.version.trim(),
            "state": "registered",
            "reason": "manifest accepted as untrusted catalog data",
            "created_at_unix_ms": now,
        }))?;
        transaction.execute(
            "INSERT INTO capability_state_events(
                state_event_id, project_id, capability_id, version, state, reason,
                event_sha256, created_at_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, 'registered', ?5, ?6, ?7)",
            params![
                event_id,
                project_id,
                request.capability_id.trim(),
                request.version.trim(),
                "manifest accepted as untrusted catalog data",
                event_sha256,
                now
            ],
        )?;
        let capability = load_capability_manifest(
            &transaction,
            &project_id,
            request.capability_id.trim(),
            request.version.trim(),
        )?;
        let outcome = CapabilityOutcome {
            capability,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.session_id,
            "capability_registered",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn get_capability(&self, request: &CapabilityGetRequest) -> Result<CapabilityManifest> {
        require_identifier("capability_id", &request.capability_id, 128)?;
        require_identifier("version", &request.version, 64)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let project_id = project_id_for_workspace(&connection, &workspace)?
            .ok_or_else(|| Error::NotFound(format!("project for workspace {workspace}")))?;
        load_capability_manifest(
            &connection,
            &project_id,
            &request.capability_id,
            &request.version,
        )
    }

    pub fn search_capabilities(
        &self,
        request: &CapabilitySearchRequest,
    ) -> Result<Vec<CapabilitySummary>> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let Some(project_id) = project_id_for_workspace(&connection, &workspace)? else {
            return Ok(Vec::new());
        };
        let query = request.query.as_deref().unwrap_or("").trim().to_lowercase();
        let limit = request.limit.unwrap_or(20).clamp(1, 100) as usize;
        let mut statement = connection.prepare(
            "SELECT capability_id, version, title, effect_class, maturity, manifest_sha256
             FROM capability_manifests WHERE project_id = ?1
             ORDER BY sequence DESC",
        )?;
        let rows = statement.query_map([&project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;
        let mut summaries = Vec::new();
        for row in rows {
            let (capability_id, version, title, effect_class, maturity, manifest_sha256) = row?;
            if !query.is_empty()
                && !capability_id.to_lowercase().contains(&query)
                && !title.to_lowercase().contains(&query)
            {
                continue;
            }
            let state =
                latest_capability_state(&connection, &project_id, &capability_id, &version)?;
            if !request.include_unavailable
                && !matches!(
                    state,
                    CapabilityCatalogState::Registered | CapabilityCatalogState::Available
                )
            {
                continue;
            }
            summaries.push(CapabilitySummary {
                capability_id,
                version,
                title,
                effect_class: parse_capability_effect_class(effect_class)?,
                maturity: parse_capability_maturity(maturity)?,
                state,
                manifest_sha256,
                executable: false,
            });
            if summaries.len() == limit {
                break;
            }
        }
        Ok(summaries)
    }

    pub fn audit_secure_capabilities(&self) -> Result<SecureCapabilityAudit> {
        let connection = self.connection()?;
        let capabilities = {
            let mut statement = connection.prepare(
                "SELECT project_id, capability_id, version
                 FROM capability_manifests ORDER BY sequence ASC",
            )?;
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let state_events = {
            let mut statement = connection.prepare(
                "SELECT capability_id, version, state, reason, event_sha256,
                        created_at_unix_ms
                 FROM capability_state_events ORDER BY sequence ASC",
            )?;
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let campaigns = {
            let mut statement = connection.prepare(
                "SELECT project_id, campaign_id
                 FROM experiment_campaigns ORDER BY sequence ASC",
            )?;
            statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let assessments = {
            let mut statement = connection.prepare(
                "SELECT project_id, assessment_id
                 FROM security_assessments ORDER BY sequence ASC",
            )?;
            statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };

        let mut integrity_failure_count = 0_u64;
        for (project_id, capability_id, version) in &capabilities {
            if load_capability_manifest(&connection, project_id, capability_id, version).is_err() {
                integrity_failure_count += 1;
            }
        }
        for (capability_id, version, state, reason, expected, created_at_unix_ms) in &state_events {
            let observed = digest_json(&serde_json::json!({
                "capability_id": capability_id,
                "version": version,
                "state": state,
                "reason": reason,
                "created_at_unix_ms": created_at_unix_ms,
            }))?;
            if observed != *expected {
                integrity_failure_count += 1;
            }
        }
        for (project_id, campaign_id) in &campaigns {
            if load_experiment_portfolio(&connection, project_id, campaign_id, 500).is_err() {
                integrity_failure_count += 1;
            }
        }
        for (project_id, assessment_id) in &assessments {
            if load_security_assessment(&connection, project_id, assessment_id).is_err() {
                integrity_failure_count += 1;
            }
        }

        let count = |table: &str| -> Result<u64> {
            let allowed = [
                "experiment_criteria",
                "experiment_variants",
                "experiment_measurements",
                "experiment_decisions",
            ];
            if !allowed.contains(&table) {
                return Err(Error::Invalid("unsupported audit table".to_owned()));
            }
            Ok(
                connection.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })?,
            )
        };
        Ok(SecureCapabilityAudit {
            capability_manifest_count: capabilities.len() as u64,
            capability_state_event_count: state_events.len() as u64,
            experiment_campaign_count: campaigns.len() as u64,
            experiment_criterion_count: count("experiment_criteria")?,
            experiment_variant_count: count("experiment_variants")?,
            experiment_measurement_count: count("experiment_measurements")?,
            experiment_decision_count: count("experiment_decisions")?,
            security_assessment_count: assessments.len() as u64,
            integrity_failure_count,
            consistent: integrity_failure_count == 0,
        })
    }

    pub fn create_experiment(
        &self,
        request: &ExperimentCreateRequest,
    ) -> Result<ExperimentOutcome> {
        require_bounded_public_text("title", &request.title, 256)?;
        require_bounded_public_text("problem", &request.problem, 4_096)?;
        require_bounded_public_text("target_user", &request.target_user, 1_024)?;
        require_bounded_public_text("desired_outcome", &request.desired_outcome, 2_048)?;
        require_bounded_public_text("hypothesis", &request.hypothesis, 2_048)?;
        require_text("git_snapshot_id", &request.git_snapshot_id)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        if !(2..=16).contains(&request.max_variants) {
            return Err(Error::Invalid(
                "max_variants must be between 2 and 16".to_owned(),
            ));
        }
        if request.criteria.is_empty() || request.criteria.len() > 32 {
            return Err(Error::Invalid(
                "an experiment requires 1 to 32 criteria".to_owned(),
            ));
        }
        let mut criterion_names = std::collections::BTreeSet::new();
        for criterion in &request.criteria {
            require_bounded_public_text("criterion name", &criterion.name, 128)?;
            require_identifier("criterion unit", &criterion.unit, 32)?;
            if !criterion_names.insert(criterion.name.trim().to_lowercase()) {
                return Err(Error::Invalid("criterion names must be unique".to_owned()));
            }
            validate_criterion(criterion)?;
        }
        let workspace = canonical_workspace(&request.workspace)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<ExperimentOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "experiment_created",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let project_id = find_or_create_project(&transaction, &workspace, now)?;
        let snapshot = load_clean_project_snapshot(
            &transaction,
            &project_id,
            &request.git_snapshot_id,
            "experiment",
        )?;
        let campaign_id = Uuid::now_v7().to_string();
        let mut campaign = ExperimentCampaign {
            sequence: 0,
            campaign_id: campaign_id.clone(),
            title: request.title.trim().to_owned(),
            problem: request.problem.trim().to_owned(),
            target_user: request.target_user.trim().to_owned(),
            desired_outcome: request.desired_outcome.trim().to_owned(),
            hypothesis: request.hypothesis.trim().to_owned(),
            git_snapshot_id: request.git_snapshot_id.clone(),
            bound_base_commit: snapshot.head_commit.expect("clean snapshot has commit"),
            bound_base_tree: snapshot.head_tree.expect("clean snapshot has tree"),
            max_variants: request.max_variants,
            max_token_budget: request.max_token_budget,
            campaign_sha256: String::new(),
            created_at_unix_ms: now,
        };
        campaign.campaign_sha256 = experiment_campaign_digest(&campaign)?;
        transaction.execute(
            "INSERT INTO experiment_campaigns(
                campaign_id, project_id, title, problem, target_user, desired_outcome,
                hypothesis, git_snapshot_id, bound_base_commit, bound_base_tree,
                max_variants, max_token_budget, campaign_sha256, created_at_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                campaign.campaign_id,
                project_id,
                campaign.title,
                campaign.problem,
                campaign.target_user,
                campaign.desired_outcome,
                campaign.hypothesis,
                campaign.git_snapshot_id,
                campaign.bound_base_commit,
                campaign.bound_base_tree,
                campaign.max_variants,
                campaign.max_token_budget,
                campaign.campaign_sha256,
                campaign.created_at_unix_ms
            ],
        )?;
        campaign.sequence = u64::try_from(transaction.last_insert_rowid())
            .map_err(|_| Error::Invalid("campaign sequence overflow".to_owned()))?;
        for input in &request.criteria {
            let criterion_id = Uuid::now_v7().to_string();
            let threshold = normalized_threshold(input);
            let digest = digest_json(&serde_json::json!({
                "campaign_id": campaign_id,
                "contract_revision": 1,
                "name": input.name.trim(),
                "kind": input.kind.as_str(),
                "comparison": input.comparison.as_str(),
                "threshold": threshold,
                "unit": input.unit.trim(),
            }))?;
            transaction.execute(
                "INSERT INTO experiment_criteria(
                    criterion_id, campaign_id, contract_revision, name, kind,
                    comparison, threshold, unit, criterion_sha256, created_at_unix_ms
                 ) VALUES (?1, ?2, 1, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    criterion_id,
                    campaign_id,
                    input.name.trim(),
                    input.kind.as_str(),
                    input.comparison.as_str(),
                    threshold,
                    input.unit.trim(),
                    digest,
                    now
                ],
            )?;
        }
        let portfolio = load_experiment_portfolio(&transaction, &project_id, &campaign_id, 500)?;
        let outcome = ExperimentOutcome {
            portfolio,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &campaign_id,
            "experiment_created",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn add_experiment_variant(
        &self,
        request: &ExperimentVariantAddRequest,
    ) -> Result<ExperimentOutcome> {
        require_text("campaign_id", &request.campaign_id)?;
        require_bounded_public_text("name", &request.name, 256)?;
        require_bounded_public_text("approach", &request.approach, 4_096)?;
        require_texts("parent_variant_ids", &request.parent_variant_ids)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        if request.parent_variant_ids.len() > 8 {
            return Err(Error::Invalid(
                "a variant may have at most 8 parents".to_owned(),
            ));
        }
        let workspace = canonical_workspace(&request.workspace)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<ExperimentOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "experiment_variant_added",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let project_id = project_id_for_workspace(&transaction, &workspace)?
            .ok_or_else(|| Error::NotFound(format!("project for workspace {workspace}")))?;
        let campaign = load_experiment_campaign(&transaction, &project_id, &request.campaign_id)?;
        let existing_count = transaction.query_row(
            "SELECT COUNT(*) FROM experiment_variants WHERE campaign_id = ?1",
            [&request.campaign_id],
            |row| row.get::<_, u64>(0),
        )?;
        if existing_count >= u64::from(campaign.max_variants) {
            return Err(Error::Conflict(
                "experiment variant budget is exhausted".to_owned(),
            ));
        }
        let snapshot = load_clean_project_snapshot(
            &transaction,
            &project_id,
            &request.git_snapshot_id,
            "variant",
        )?;
        for parent in &request.parent_variant_ids {
            require_variant_owner(&transaction, &request.campaign_id, parent)?;
        }
        let approach_sha256 = format!("{:x}", Sha256::digest(request.approach.trim().as_bytes()));
        if transaction
            .query_row(
                "SELECT 1 FROM experiment_variants WHERE campaign_id = ?1 AND approach_sha256 = ?2",
                params![request.campaign_id, approach_sha256],
                |_| Ok(()),
            )
            .optional()?
            .is_some()
        {
            return Err(Error::Conflict(
                "an exact approach duplicate already exists".to_owned(),
            ));
        }
        let variant_id = Uuid::now_v7().to_string();
        let parents_json = serde_json::to_string(&request.parent_variant_ids)?;
        let mut variant = ExperimentVariant {
            sequence: 0,
            variant_id: variant_id.clone(),
            name: request.name.trim().to_owned(),
            diversity_axis: request.diversity_axis.clone(),
            approach: request.approach.trim().to_owned(),
            approach_sha256,
            git_snapshot_id: request.git_snapshot_id.clone(),
            head_commit: snapshot.head_commit.expect("clean snapshot has commit"),
            head_tree: snapshot.head_tree.expect("clean snapshot has tree"),
            parent_variant_ids: request.parent_variant_ids.clone(),
            variant_sha256: String::new(),
            created_at_unix_ms: now,
        };
        variant.variant_sha256 = experiment_variant_digest(&request.campaign_id, &variant)?;
        transaction.execute(
            "INSERT INTO experiment_variants(
                variant_id, campaign_id, name, diversity_axis, approach, approach_sha256,
                git_snapshot_id, head_commit, head_tree, parent_variant_ids_json,
                variant_sha256, created_at_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                variant.variant_id,
                request.campaign_id,
                variant.name,
                variant.diversity_axis.as_str(),
                variant.approach,
                variant.approach_sha256,
                variant.git_snapshot_id,
                variant.head_commit,
                variant.head_tree,
                parents_json,
                variant.variant_sha256,
                variant.created_at_unix_ms
            ],
        )?;
        let portfolio =
            load_experiment_portfolio(&transaction, &project_id, &request.campaign_id, 500)?;
        let outcome = ExperimentOutcome {
            portfolio,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.campaign_id,
            "experiment_variant_added",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn add_experiment_measurement(
        &self,
        request: &ExperimentMeasurementAddRequest,
    ) -> Result<ExperimentOutcome> {
        require_text("campaign_id", &request.campaign_id)?;
        require_text("variant_id", &request.variant_id)?;
        require_text("criterion_id", &request.criterion_id)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<ExperimentOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "experiment_measurement_added",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let project_id = project_id_for_workspace(&transaction, &workspace)?
            .ok_or_else(|| Error::NotFound(format!("project for workspace {workspace}")))?;
        load_experiment_campaign(&transaction, &project_id, &request.campaign_id)?;
        require_variant_owner(&transaction, &request.campaign_id, &request.variant_id)?;
        let criterion_campaign = transaction
            .query_row(
                "SELECT campaign_id FROM experiment_criteria WHERE criterion_id = ?1",
                [&request.criterion_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("criterion {}", request.criterion_id)))?;
        if criterion_campaign != request.campaign_id {
            return Err(Error::Conflict(
                "criterion belongs to another campaign".to_owned(),
            ));
        }
        validate_deliberation_evidence(
            &transaction,
            &project_id,
            request.evidence_id.as_deref(),
            request.claim_id.as_deref(),
        )?;
        let measurement_id = Uuid::now_v7().to_string();
        let digest = digest_json(&serde_json::json!({
            "campaign_id": request.campaign_id,
            "variant_id": request.variant_id,
            "criterion_id": request.criterion_id,
            "value": request.value,
            "evidence_id": request.evidence_id,
            "claim_id": request.claim_id,
            "created_at_unix_ms": now,
        }))?;
        transaction.execute(
            "INSERT INTO experiment_measurements(
                measurement_id, campaign_id, variant_id, criterion_id, value,
                evidence_id, claim_id, measurement_sha256, created_at_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                measurement_id,
                request.campaign_id,
                request.variant_id,
                request.criterion_id,
                request.value,
                request.evidence_id,
                request.claim_id,
                digest,
                now
            ],
        )?;
        let portfolio =
            load_experiment_portfolio(&transaction, &project_id, &request.campaign_id, 500)?;
        let outcome = ExperimentOutcome {
            portfolio,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.campaign_id,
            "experiment_measurement_added",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn decide_experiment(
        &self,
        request: &ExperimentDecisionRequest,
    ) -> Result<ExperimentOutcome> {
        require_text("campaign_id", &request.campaign_id)?;
        require_bounded_public_text("summary", &request.summary, 4_096)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<ExperimentOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "experiment_decided",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let project_id = project_id_for_workspace(&transaction, &workspace)?
            .ok_or_else(|| Error::NotFound(format!("project for workspace {workspace}")))?;
        load_experiment_campaign(&transaction, &project_id, &request.campaign_id)?;
        if request.kind != ExperimentDecisionKind::Abandon && request.variant_id.is_none() {
            return Err(Error::Invalid(
                "this decision kind requires variant_id".to_owned(),
            ));
        }
        if let Some(variant_id) = request.variant_id.as_deref() {
            require_variant_owner(&transaction, &request.campaign_id, variant_id)?;
        }
        let deliberation_pair = match (
            request.deliberation_id.as_deref(),
            request.deliberation_decision_id.as_deref(),
        ) {
            (None, None) => false,
            (Some(deliberation_id), Some(decision_id)) => {
                require_deliberation_owner(&transaction, &project_id, deliberation_id)?;
                let open: u64 = transaction
                    .query_row(
                        "SELECT open_material_issues FROM deliberation_decisions
                     WHERE deliberation_id = ?1 AND decision_id = ?2",
                        params![deliberation_id, decision_id],
                        |row| row.get(0),
                    )
                    .optional()?
                    .ok_or_else(|| {
                        Error::NotFound(format!("deliberation decision {decision_id}"))
                    })?;
                open == 0
            }
            _ => {
                return Err(Error::Invalid(
                    "deliberation_id and deliberation_decision_id must be supplied together"
                        .to_owned(),
                ));
            }
        };
        let before =
            load_experiment_portfolio(&transaction, &project_id, &request.campaign_id, 500)?;
        let variant_qualified = request.variant_id.as_ref().is_some_and(|id| {
            !before.hard_gate_failed_variant_ids.contains(id)
                && !before.evidence_incomplete_variant_ids.contains(id)
                && before
                    .variants
                    .iter()
                    .any(|variant| &variant.variant_id == id)
        });
        let qualified = match request.kind {
            ExperimentDecisionKind::Select => variant_qualified && deliberation_pair,
            ExperimentDecisionKind::Advance | ExperimentDecisionKind::Synthesize => {
                variant_qualified
            }
            ExperimentDecisionKind::Eliminate | ExperimentDecisionKind::Abandon => true,
        };
        if matches!(request.kind, ExperimentDecisionKind::Select) && !qualified {
            return Err(Error::Conflict(
                "selection requires all hard gates to pass with direct evidence and a deliberation decision with no open material issues"
                    .to_owned(),
            ));
        }
        let decision_id = Uuid::now_v7().to_string();
        let digest = digest_json(&serde_json::json!({
            "campaign_id": request.campaign_id,
            "kind": request.kind.as_str(),
            "variant_id": request.variant_id,
            "summary": request.summary.trim(),
            "deliberation_id": request.deliberation_id,
            "deliberation_decision_id": request.deliberation_decision_id,
            "qualified": qualified,
            "created_at_unix_ms": now,
        }))?;
        transaction.execute(
            "INSERT INTO experiment_decisions(
                decision_id, campaign_id, kind, variant_id, summary, deliberation_id,
                deliberation_decision_id, qualified, decision_sha256, created_at_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                decision_id,
                request.campaign_id,
                request.kind.as_str(),
                request.variant_id,
                request.summary.trim(),
                request.deliberation_id,
                request.deliberation_decision_id,
                qualified,
                digest,
                now
            ],
        )?;
        let portfolio =
            load_experiment_portfolio(&transaction, &project_id, &request.campaign_id, 500)?;
        let outcome = ExperimentOutcome {
            portfolio,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.campaign_id,
            "experiment_decided",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn get_experiment(&self, request: &ExperimentGetRequest) -> Result<ExperimentPortfolio> {
        require_text("campaign_id", &request.campaign_id)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let project_id = project_id_for_workspace(&connection, &workspace)?
            .ok_or_else(|| Error::NotFound(format!("project for workspace {workspace}")))?;
        load_experiment_portfolio(
            &connection,
            &project_id,
            &request.campaign_id,
            request.limit.unwrap_or(200).clamp(1, 500) as usize,
        )
    }

    pub fn list_experiments(
        &self,
        request: &ExperimentListRequest,
    ) -> Result<Vec<ExperimentSummary>> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let Some(project_id) = project_id_for_workspace(&connection, &workspace)? else {
            return Ok(Vec::new());
        };
        let limit = request.limit.unwrap_or(20).clamp(1, 100);
        let mut statement = connection.prepare(
            "SELECT campaign_id, title, created_at_unix_ms,
                (SELECT COUNT(*) FROM experiment_variants v WHERE v.campaign_id = c.campaign_id),
                (SELECT COUNT(*) FROM experiment_decisions d WHERE d.campaign_id = c.campaign_id),
                max_variants
             FROM experiment_campaigns c WHERE project_id = ?1
             ORDER BY sequence DESC LIMIT ?2",
        )?;
        Ok(statement
            .query_map(params![project_id, limit], |row| {
                let variants = row.get::<_, u64>(3)?;
                let max_variants = row.get::<_, u64>(5)?;
                Ok(ExperimentSummary {
                    campaign_id: row.get(0)?,
                    title: row.get(1)?,
                    created_at_unix_ms: row.get(2)?,
                    variant_count: variants,
                    decision_count: row.get(4)?,
                    budget_exhausted: variants >= max_variants,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub(crate) fn import_security_assessment(
        &self,
        request: &SecurityArtifactImport,
    ) -> Result<SecurityAssessmentOutcome> {
        require_text("session_id", &request.session_id)?;
        require_identifier(
            "provider_capability_id",
            &request.provider_capability_id,
            128,
        )?;
        require_identifier("provider_version", &request.provider_version, 64)?;
        require_identifier("source_scan_id", &request.source_scan_id, 128)?;
        require_text("git_snapshot_id", &request.git_snapshot_id)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        for digest in [
            &request.manifest_sha256,
            &request.findings_sha256,
            &request.coverage_sha256,
        ] {
            if !is_sha256(digest) {
                return Err(Error::Invalid(
                    "security artifact digests must be lowercase SHA-256".to_owned(),
                ));
            }
        }
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<SecurityAssessmentOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "security_assessment_imported",
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
        let snapshot = load_clean_project_snapshot(
            &transaction,
            &project_id,
            &request.git_snapshot_id,
            "security assessment",
        )?;
        let manifest_evidence_id = insert_imported_security_evidence(
            &transaction,
            &request.session_id,
            &request.manifest_locator,
            "Codex Security scan manifest bytes observed by Aporic",
            &request.manifest_sha256,
            now,
        )?;
        let findings_evidence_id = insert_imported_security_evidence(
            &transaction,
            &request.session_id,
            &request.findings_locator,
            "Codex Security findings bytes observed by Aporic",
            &request.findings_sha256,
            now,
        )?;
        let coverage_evidence_id = insert_imported_security_evidence(
            &transaction,
            &request.session_id,
            &request.coverage_locator,
            "Codex Security coverage bytes observed by Aporic",
            &request.coverage_sha256,
            now,
        )?;
        let assessment_id = Uuid::now_v7().to_string();
        let target_commit = snapshot.head_commit.expect("clean snapshot has commit");
        let target_tree = snapshot.head_tree.expect("clean snapshot has tree");
        let assessment_sha256 = digest_json(&serde_json::json!({
            "assessment_id": assessment_id,
            "provider_capability_id": request.provider_capability_id,
            "provider_version": request.provider_version,
            "source_scan_id": request.source_scan_id,
            "git_snapshot_id": request.git_snapshot_id,
            "target_commit": target_commit,
            "target_tree": target_tree,
            "coverage": request.coverage.as_str(),
            "reportable_critical": request.reportable_critical,
            "reportable_high": request.reportable_high,
            "reportable_medium": request.reportable_medium,
            "reportable_low": request.reportable_low,
            "manifest_evidence_id": manifest_evidence_id,
            "findings_evidence_id": findings_evidence_id,
            "coverage_evidence_id": coverage_evidence_id,
            "created_at_unix_ms": now,
        }))?;
        transaction.execute(
            "INSERT INTO security_assessments(
                assessment_id, project_id, provider_capability_id, provider_version,
                source_scan_id, git_snapshot_id, target_commit, target_tree, coverage,
                reportable_critical, reportable_high, reportable_medium, reportable_low,
                manifest_evidence_id, findings_evidence_id, coverage_evidence_id,
                assessment_sha256, created_at_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                       ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                assessment_id,
                project_id,
                request.provider_capability_id,
                request.provider_version,
                request.source_scan_id,
                request.git_snapshot_id,
                target_commit,
                target_tree,
                request.coverage.as_str(),
                request.reportable_critical,
                request.reportable_high,
                request.reportable_medium,
                request.reportable_low,
                manifest_evidence_id,
                findings_evidence_id,
                coverage_evidence_id,
                assessment_sha256,
                now
            ],
        )?;
        let assessment = load_security_assessment(&transaction, &project_id, &assessment_id)?;
        let outcome = SecurityAssessmentOutcome {
            assessment,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &assessment_id,
            "security_assessment_imported",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn get_security_assessment(
        &self,
        request: &SecurityAssessmentGetRequest,
    ) -> Result<SecurityAssessment> {
        require_text("assessment_id", &request.assessment_id)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let project_id = project_id_for_workspace(&connection, &workspace)?
            .ok_or_else(|| Error::NotFound(format!("project for workspace {workspace}")))?;
        load_security_assessment(&connection, &project_id, &request.assessment_id)
    }

    pub fn list_security_assessments(
        &self,
        request: &SecurityAssessmentListRequest,
    ) -> Result<Vec<SecurityAssessment>> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let Some(project_id) = project_id_for_workspace(&connection, &workspace)? else {
            return Ok(Vec::new());
        };
        let limit = request.limit.unwrap_or(20).clamp(1, 100);
        let mut statement = connection.prepare(
            "SELECT assessment_id FROM security_assessments WHERE project_id = ?1
             ORDER BY sequence DESC LIMIT ?2",
        )?;
        let ids = statement
            .query_map(params![project_id, limit], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ids.iter()
            .map(|id| load_security_assessment(&connection, &project_id, id))
            .collect()
    }

    pub fn create_deliberation(
        &self,
        request: &DeliberationCreateRequest,
    ) -> Result<DeliberationOutcome> {
        require_bounded_public_text("title", &request.title, 256)?;
        require_bounded_public_text("question", &request.question, 4_096)?;
        require_text("git_snapshot_id", &request.git_snapshot_id)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<DeliberationOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "deliberation_created",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let project_id = find_or_create_project(&transaction, &workspace, now)?;
        let snapshot = transaction
            .query_row(
                &format!(
                    "{} FROM git_snapshots WHERE project_id = ?1 AND snapshot_id = ?2",
                    git_snapshot_select()
                ),
                params![project_id, request.git_snapshot_id],
                git_snapshot_from_row,
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("Git snapshot {}", request.git_snapshot_id)))?;
        if git_snapshot_digest(&snapshot)? != snapshot.snapshot_sha256 {
            return Err(Error::Conflict(
                "cannot bind deliberation to a Git snapshot with a digest mismatch".to_owned(),
            ));
        }
        if snapshot.dirty || snapshot.head_commit.is_none() || snapshot.head_tree.is_none() {
            return Err(Error::Invalid(
                "deliberation requires a clean committed Git snapshot".to_owned(),
            ));
        }
        let mut deliberation = Deliberation {
            sequence: 0,
            deliberation_id: Uuid::now_v7().to_string(),
            title: request.title.trim().to_owned(),
            question: request.question.trim().to_owned(),
            git_snapshot_id: request.git_snapshot_id.clone(),
            bound_head_commit: snapshot.head_commit,
            bound_head_tree: snapshot.head_tree,
            deliberation_sha256: String::new(),
            created_at_unix_ms: now,
        };
        deliberation.deliberation_sha256 = deliberation_digest(&deliberation)?;
        transaction.execute(
            "INSERT INTO deliberations(
                deliberation_id, project_id, title, question, git_snapshot_id,
                bound_head_commit, bound_head_tree, deliberation_sha256, created_at_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                deliberation.deliberation_id,
                project_id,
                deliberation.title,
                deliberation.question,
                deliberation.git_snapshot_id,
                deliberation.bound_head_commit,
                deliberation.bound_head_tree,
                deliberation.deliberation_sha256,
                deliberation.created_at_unix_ms,
            ],
        )?;
        deliberation.sequence = u64::try_from(transaction.last_insert_rowid())
            .map_err(|_| Error::Invalid("deliberation sequence overflow".to_owned()))?;
        let mut root = DeliberationNode {
            sequence: 0,
            node_id: Uuid::now_v7().to_string(),
            deliberation_id: deliberation.deliberation_id.clone(),
            kind: DeliberationNodeKind::Question,
            statement: deliberation.question.clone(),
            material: true,
            evidence_id: None,
            claim_id: None,
            node_sha256: String::new(),
            created_at_unix_ms: now,
        };
        root.node_sha256 = deliberation_node_digest(&deliberation.deliberation_id, &root)?;
        insert_deliberation_node(&transaction, &deliberation.deliberation_id, &root)?;
        root.sequence = u64::try_from(transaction.last_insert_rowid())
            .map_err(|_| Error::Invalid("deliberation node sequence overflow".to_owned()))?;
        let graph =
            load_deliberation_graph(&transaction, &project_id, &deliberation.deliberation_id)?;
        let outcome = DeliberationOutcome {
            graph,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &deliberation.deliberation_id,
            "deliberation_created",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn add_deliberation_node(
        &self,
        request: &DeliberationNodeAddRequest,
    ) -> Result<DeliberationOutcome> {
        require_text("deliberation_id", &request.deliberation_id)?;
        require_bounded_public_text("statement", &request.statement, 4_096)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        if request.kind == DeliberationNodeKind::Question {
            return Err(Error::Invalid(
                "the root question is created with the deliberation".to_owned(),
            ));
        }
        if request.relations.len() > 32 {
            return Err(Error::Invalid(
                "a node may declare at most 32 relations".to_owned(),
            ));
        }
        let workspace = canonical_workspace(&request.workspace)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<DeliberationOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "deliberation_node_added",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let project_id = project_id_for_workspace(&transaction, &workspace)?
            .ok_or_else(|| Error::NotFound(format!("project for workspace {workspace}")))?;
        require_deliberation_owner(&transaction, &project_id, &request.deliberation_id)?;
        let direct_evidence = validate_deliberation_evidence(
            &transaction,
            &project_id,
            request.evidence_id.as_deref(),
            request.claim_id.as_deref(),
        )?;
        if request.material && is_challenge_kind(&request.kind) && !direct_evidence {
            return Err(Error::Invalid(
                "material objections, counterexamples, falsifiers, and unknowns require direct evidence or an observed/verified claim"
                    .to_owned(),
            ));
        }
        for relation in &request.relations {
            require_text("target_node_id", &relation.target_node_id)?;
            let target = transaction
                .query_row(
                    "SELECT kind, material FROM deliberation_nodes
                 WHERE deliberation_id = ?1 AND node_id = ?2",
                    params![request.deliberation_id, relation.target_node_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, bool>(1)?)),
                )
                .optional()?
                .ok_or_else(|| {
                    Error::Conflict(format!(
                        "relation target {} is not in deliberation {}",
                        relation.target_node_id, request.deliberation_id
                    ))
                })?;
            if relation.kind == DeliberationEdgeKind::Revision
                && request.kind != DeliberationNodeKind::Revision
            {
                return Err(Error::Invalid(
                    "revision relations must originate from revision nodes".to_owned(),
                ));
            }
            if target.0 == "material_unknown" && target.1 {
                if relation.kind == DeliberationEdgeKind::Revision {
                    return Err(Error::Invalid(
                        "a material unknown cannot be closed by narrative revision; use a directly evidenced undercut"
                            .to_owned(),
                    ));
                }
                if relation.kind == DeliberationEdgeKind::Undercut && !direct_evidence {
                    return Err(Error::Invalid(
                        "undercutting a material unknown requires direct evidence or an observed/verified claim"
                            .to_owned(),
                    ));
                }
            }
        }
        let mut node = DeliberationNode {
            sequence: 0,
            node_id: Uuid::now_v7().to_string(),
            deliberation_id: request.deliberation_id.clone(),
            kind: request.kind.clone(),
            statement: request.statement.trim().to_owned(),
            material: request.material,
            evidence_id: request.evidence_id.clone(),
            claim_id: request.claim_id.clone(),
            node_sha256: String::new(),
            created_at_unix_ms: now,
        };
        node.node_sha256 = deliberation_node_digest(&request.deliberation_id, &node)?;
        insert_deliberation_node(&transaction, &request.deliberation_id, &node)?;
        node.sequence = u64::try_from(transaction.last_insert_rowid())
            .map_err(|_| Error::Invalid("deliberation node sequence overflow".to_owned()))?;
        for relation in &request.relations {
            let mut edge = DeliberationEdge {
                sequence: 0,
                edge_id: Uuid::now_v7().to_string(),
                deliberation_id: request.deliberation_id.clone(),
                source_node_id: node.node_id.clone(),
                target_node_id: relation.target_node_id.clone(),
                kind: relation.kind.clone(),
                edge_sha256: String::new(),
                created_at_unix_ms: now,
            };
            edge.edge_sha256 = deliberation_edge_digest(&request.deliberation_id, &edge)?;
            transaction.execute(
                "INSERT INTO deliberation_edges(
                    edge_id, deliberation_id, source_node_id, target_node_id,
                    kind, edge_sha256, created_at_unix_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    edge.edge_id,
                    request.deliberation_id,
                    edge.source_node_id,
                    edge.target_node_id,
                    edge.kind.as_str(),
                    edge.edge_sha256,
                    edge.created_at_unix_ms,
                ],
            )?;
        }
        let graph = load_deliberation_graph(&transaction, &project_id, &request.deliberation_id)?;
        let outcome = DeliberationOutcome {
            graph,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.deliberation_id,
            "deliberation_node_added",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn record_deliberation_decision(
        &self,
        request: &DeliberationDecisionRequest,
    ) -> Result<DeliberationOutcome> {
        require_text("deliberation_id", &request.deliberation_id)?;
        require_text("proposal_node_id", &request.proposal_node_id)?;
        require_bounded_public_text("summary", &request.summary, 4_096)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<DeliberationOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "deliberation_decision_recorded",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let project_id = project_id_for_workspace(&transaction, &workspace)?
            .ok_or_else(|| Error::NotFound(format!("project for workspace {workspace}")))?;
        require_deliberation_owner(&transaction, &project_id, &request.deliberation_id)?;
        let proposal_kind = transaction
            .query_row(
                "SELECT kind FROM deliberation_nodes
                 WHERE deliberation_id = ?1 AND node_id = ?2",
                params![request.deliberation_id, request.proposal_node_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| {
                Error::NotFound(format!("proposal node {}", request.proposal_node_id))
            })?;
        if !matches!(proposal_kind.as_str(), "proposal" | "revision") {
            return Err(Error::Invalid(
                "a provisional decision must reference a proposal or revision node".to_owned(),
            ));
        }
        let graph_before =
            load_deliberation_graph(&transaction, &project_id, &request.deliberation_id)?;
        if graph_before.stale {
            return Err(Error::Conflict(
                "cannot record a decision on a stale deliberation; create a graph bound to the current Git snapshot"
                    .to_owned(),
            ));
        }
        let mut decision = DeliberationDecision {
            sequence: 0,
            decision_id: Uuid::now_v7().to_string(),
            deliberation_id: request.deliberation_id.clone(),
            proposal_node_id: request.proposal_node_id.clone(),
            summary: request.summary.trim().to_owned(),
            open_material_issues: graph_before.total_open_material_issue_count,
            decision_sha256: String::new(),
            created_at_unix_ms: now,
        };
        decision.decision_sha256 =
            deliberation_decision_digest(&request.deliberation_id, &decision)?;
        transaction.execute(
            "INSERT INTO deliberation_decisions(
                decision_id, deliberation_id, proposal_node_id, summary,
                open_material_issues, decision_sha256, created_at_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                decision.decision_id,
                request.deliberation_id,
                decision.proposal_node_id,
                decision.summary,
                decision.open_material_issues,
                decision.decision_sha256,
                decision.created_at_unix_ms,
            ],
        )?;
        let graph = load_deliberation_graph(&transaction, &project_id, &request.deliberation_id)?;
        let outcome = DeliberationOutcome {
            graph,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.deliberation_id,
            "deliberation_decision_recorded",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn get_deliberation(&self, request: &DeliberationGetRequest) -> Result<DeliberationGraph> {
        require_text("deliberation_id", &request.deliberation_id)?;
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let project_id = project_id_for_workspace(&connection, &workspace)?
            .ok_or_else(|| Error::NotFound(format!("project for workspace {workspace}")))?;
        load_deliberation_graph_page(
            &connection,
            &project_id,
            &request.deliberation_id,
            request.node_after_sequence.unwrap_or(0),
            request.edge_after_sequence.unwrap_or(0),
            request.decision_after_sequence.unwrap_or(0),
            request.limit.unwrap_or(200).clamp(1, 500) as usize,
        )
    }

    pub fn list_deliberations(
        &self,
        request: &DeliberationListRequest,
    ) -> Result<Vec<DeliberationSummary>> {
        let workspace = canonical_workspace(&request.workspace)?;
        let connection = self.connection()?;
        let Some(project_id) = project_id_for_workspace(&connection, &workspace)? else {
            return Ok(Vec::new());
        };
        let limit = request.limit.unwrap_or(20).clamp(1, 100);
        let mut statement = connection.prepare(
            "SELECT deliberation_id FROM deliberations
             WHERE project_id = ?1 ORDER BY sequence DESC LIMIT ?2",
        )?;
        let ids = statement
            .query_map(params![project_id, limit], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ids.iter()
            .map(|id| {
                let graph = load_deliberation_graph(&connection, &project_id, id)?;
                Ok(DeliberationSummary {
                    deliberation_id: graph.deliberation.deliberation_id,
                    title: graph.deliberation.title,
                    git_snapshot_id: graph.deliberation.git_snapshot_id,
                    current_git_snapshot_id: graph.current_git_snapshot_id,
                    stale: graph.stale,
                    node_count: graph.total_node_count,
                    edge_count: graph.total_edge_count,
                    decision_count: graph.total_decision_count,
                    open_material_issue_count: graph.total_open_material_issue_count,
                    created_at_unix_ms: graph.deliberation.created_at_unix_ms,
                    provisional_only: true,
                    approval_proven: false,
                })
            })
            .collect()
    }

    pub fn audit_deliberations(&self) -> Result<DeliberationAudit> {
        let connection = self.connection()?;
        let deliberations = load_all_deliberations(&connection)?;
        let nodes = load_all_deliberation_nodes(&connection)?;
        let edges = load_all_deliberation_edges(&connection)?;
        let decisions = load_all_deliberation_decisions(&connection)?;
        let mut digest_mismatch_count = 0;
        for deliberation in &deliberations {
            if deliberation_digest(deliberation)? != deliberation.deliberation_sha256 {
                digest_mismatch_count += 1;
            }
        }
        for (deliberation_id, node) in &nodes {
            if deliberation_node_digest(deliberation_id, node)? != node.node_sha256 {
                digest_mismatch_count += 1;
            }
        }
        for (deliberation_id, edge) in &edges {
            if deliberation_edge_digest(deliberation_id, edge)? != edge.edge_sha256 {
                digest_mismatch_count += 1;
            }
        }
        for (deliberation_id, decision) in &decisions {
            if deliberation_decision_digest(deliberation_id, decision)? != decision.decision_sha256
            {
                digest_mismatch_count += 1;
            }
        }
        let cross_graph_edge_count = connection.query_row(
            "SELECT COUNT(*) FROM deliberation_edges edges
             JOIN deliberation_nodes sources ON sources.node_id = edges.source_node_id
             JOIN deliberation_nodes targets ON targets.node_id = edges.target_node_id
             WHERE sources.deliberation_id != edges.deliberation_id
                OR targets.deliberation_id != edges.deliberation_id",
            [],
            |row| row.get::<_, u64>(0),
        )?;
        Ok(DeliberationAudit {
            deliberation_count: deliberations.len() as u64,
            node_count: nodes.len() as u64,
            edge_count: edges.len() as u64,
            decision_count: decisions.len() as u64,
            digest_mismatch_count,
            cross_graph_edge_count,
            consistent: digest_mismatch_count == 0 && cross_graph_edge_count == 0,
        })
    }

    pub fn create_orchestration_run(
        &self,
        request: &OrchestrationRunCreateRequest,
    ) -> Result<OrchestrationOutcome> {
        require_text("session_id", &request.session_id)?;
        require_text("task_id", &request.task_id)?;
        require_text("git_snapshot_id", &request.git_snapshot_id)?;
        require_text("context_exposure_id", &request.context_exposure_id)?;
        require_identifier("role_id", &request.role_id, 128)?;
        require_bounded_public_text("objective", &request.objective, 4_096)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        if let Some(model) = &request.model {
            require_identifier("model", model, 128)?;
        }
        if let Some(effort) = &request.reasoning_effort {
            require_identifier("reasoning_effort", effort, 32)?;
        }
        if !(1..=10_000_000).contains(&request.max_input_tokens)
            || !(1..=1_000_000).contains(&request.max_output_tokens)
            || !(1..=86_400).contains(&request.max_duration_seconds)
        {
            return Err(Error::Invalid(
                "orchestration budgets exceed the supported bounded range".to_owned(),
            ));
        }
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<OrchestrationOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "orchestration_run_created",
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
        let task = load_task(&transaction, &request.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", request.task_id)))?;
        let task_project = transaction.query_row(
            "SELECT project_id FROM tasks WHERE task_id = ?1",
            [&request.task_id],
            |row| row.get::<_, String>(0),
        )?;
        if task_project != project_id || task.session_id != request.session_id {
            return Err(Error::Conflict(
                "orchestration task and session must belong to the same project session".to_owned(),
            ));
        }
        if matches!(task.status, TaskStatus::Completed | TaskStatus::Cancelled) {
            return Err(Error::Conflict(
                "an orchestration run must bind an active advisory task".to_owned(),
            ));
        }
        if let Some(appointment_id) = &request.role_appointment_id {
            let appointment = load_role_appointment(&transaction, appointment_id)?;
            let expected_role = match request.role_kind {
                AdvisoryRoleKind::Steward => "portfolio.steward",
                AdvisoryRoleKind::Worker => "delivery.worker",
            };
            if appointment.session_id != request.session_id
                || appointment.role_id != expected_role
                || appointment.revoked_at_unix_ms.is_some()
                || appointment
                    .task_id
                    .as_deref()
                    .is_some_and(|id| id != request.task_id)
                || appointment
                    .model_hint
                    .as_deref()
                    .is_some_and(|model| request.model.as_deref() != Some(model))
            {
                return Err(Error::Conflict("orchestration role appointment does not match the active session, task, and role".to_owned()));
            }
        }
        let snapshot = load_clean_project_snapshot(
            &transaction,
            &project_id,
            &request.git_snapshot_id,
            "orchestration run",
        )?;
        let context_policy_sha256 = transaction
            .query_row(
                "SELECT policy_sha256 FROM memory_exposures
                 WHERE project_id = ?1 AND exposure_id = ?2",
                params![project_id, request.context_exposure_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| {
                Error::NotFound(format!("context exposure {}", request.context_exposure_id))
            })?;
        let run_id = Uuid::now_v7().to_string();
        let bound_head_commit = snapshot.head_commit.expect("clean snapshot has commit");
        let bound_head_tree = snapshot.head_tree.expect("clean snapshot has tree");
        let run_sha256 = orchestration_run_digest(
            &run_id,
            request,
            &bound_head_commit,
            &bound_head_tree,
            &context_policy_sha256,
        )?;
        transaction.execute(
            "INSERT INTO orchestration_runs(
                run_id, project_id, session_id, task_id, git_snapshot_id,
                context_exposure_id, mode, role_kind, role_id, objective, model,
                reasoning_effort, max_input_tokens, max_output_tokens,
                max_duration_seconds, capability_ceiling, status, bound_head_commit,
                bound_head_tree, context_policy_sha256, run_sha256, created_at_unix_ms,
                role_appointment_id
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                       ?13, ?14, ?15, 'propose', 'awaiting_report', ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                run_id,
                project_id,
                request.session_id,
                request.task_id,
                request.git_snapshot_id,
                request.context_exposure_id,
                request.mode.as_str(),
                request.role_kind.as_str(),
                request.role_id.trim(),
                request.objective.trim(),
                request.model.as_deref(),
                request.reasoning_effort.as_deref(),
                request.max_input_tokens,
                request.max_output_tokens,
                request.max_duration_seconds,
                bound_head_commit,
                bound_head_tree,
                context_policy_sha256,
                run_sha256,
                now,
                request.role_appointment_id,
            ],
        )?;
        let view = load_orchestration_view(&transaction, &project_id, &run_id)?;
        let outcome = OrchestrationOutcome {
            view,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &run_id,
            "orchestration_run_created",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn submit_advisory_role_report(
        &self,
        request: &AdvisoryRoleReportRequest,
    ) -> Result<OrchestrationOutcome> {
        require_text("run_id", &request.run_id)?;
        require_bounded_public_text("summary", &request.summary, 4_096)?;
        require_bounded_public_text("public_rationale", &request.public_rationale, 8_192)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        if request.claims_task_complete {
            return Err(Error::Conflict(
                "advisory role reports cannot claim task completion".to_owned(),
            ));
        }
        if let Some(action) = &request.recommended_next_action {
            require_bounded_public_text("recommended_next_action", action, 2_048)?;
        }
        if request.uncertainties.len() > 32 || request.addressed_criteria.len() > 64 {
            return Err(Error::Invalid(
                "role reports allow at most 32 uncertainties and 64 addressed criteria".to_owned(),
            ));
        }
        for uncertainty in &request.uncertainties {
            require_bounded_public_text("uncertainty", uncertainty, 1_024)?;
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let sealed_payload = advisory_report_event_payload(request)?;
        if let Some(mut outcome) = duplicate_result::<OrchestrationOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "advisory_role_report_submitted",
            &sealed_payload,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let (project_id, run) = load_orchestration_run_any(&transaction, &request.run_id)?;
        if run.status != OrchestrationRunStatus::AwaitingReport {
            return Err(Error::Conflict(
                "an orchestration run accepts exactly one role report".to_owned(),
            ));
        }
        let task = load_task(&transaction, &run.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", run.task_id)))?;
        if matches!(task.status, TaskStatus::Completed | TaskStatus::Cancelled) {
            return Err(Error::Conflict(
                "a shadow report must be sealed before the bound task reaches an outcome"
                    .to_owned(),
            ));
        }
        if run.role_kind == AdvisoryRoleKind::Steward
            && request.disposition == AdvisoryDisposition::ProposeWork
            && request.recommended_next_action.is_none()
        {
            return Err(Error::Invalid(
                "a steward proposing work must provide a recommended next action".to_owned(),
            ));
        }
        if request.disposition == AdvisoryDisposition::Abstain
            && request.recommended_next_action.is_some()
        {
            return Err(Error::Invalid(
                "an abstaining report must not recommend an action".to_owned(),
            ));
        }
        validate_report_budget(&run, request)?;
        let task_criteria = task
            .acceptance_criteria
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        let mut addressed = std::collections::BTreeSet::new();
        for criterion in &request.addressed_criteria {
            require_bounded_public_text("addressed criterion", criterion, 2_048)?;
            if !task_criteria.contains(criterion) {
                return Err(Error::Conflict(
                    "addressed criteria must exactly match the bound task acceptance criteria"
                        .to_owned(),
                ));
            }
            if !addressed.insert(criterion) {
                return Err(Error::Invalid(
                    "addressed criteria must be unique".to_owned(),
                ));
            }
        }
        let report_id = Uuid::now_v7().to_string();
        let report_sha256 = advisory_report_digest(&report_id, request)?;
        transaction.execute(
            "INSERT INTO advisory_role_reports(
                report_id, run_id, source_kind, disposition, predicted_task_outcome,
                summary, public_rationale, recommended_next_action, uncertainties_json,
                addressed_criteria_json, reported_input_tokens, reported_output_tokens,
                reported_duration_ms, report_sha256, created_at_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                report_id,
                request.run_id,
                request.source_kind.as_str(),
                request.disposition.as_str(),
                request.predicted_task_outcome.as_str(),
                request.summary.trim(),
                request.public_rationale.trim(),
                request.recommended_next_action.as_deref(),
                serde_json::to_string(&request.uncertainties)?,
                serde_json::to_string(&request.addressed_criteria)?,
                request.reported_input_tokens,
                request.reported_output_tokens,
                request.reported_duration_ms,
                report_sha256,
                unix_millis()?,
            ],
        )?;
        let now = unix_millis()?;
        transaction.execute(
            "UPDATE orchestration_runs SET status = 'sealed', sealed_at_unix_ms = ?1
             WHERE run_id = ?2",
            params![now, request.run_id],
        )?;
        let view = load_orchestration_view(&transaction, &project_id, &request.run_id)?;
        let outcome = OrchestrationOutcome {
            view,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.run_id,
            "advisory_role_report_submitted",
            &sealed_payload,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn evaluate_shadow_run(
        &self,
        request: &ShadowEvaluationRequest,
    ) -> Result<OrchestrationOutcome> {
        require_text("run_id", &request.run_id)?;
        require_text("idempotency_key", &request.idempotency_key)?;
        let now = unix_millis()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(mut outcome) = duplicate_result::<OrchestrationOutcome, _>(
            &transaction,
            &request.idempotency_key,
            "shadow_run_evaluated",
            request,
        )? {
            outcome.duplicate = true;
            return Ok(outcome);
        }
        let (project_id, run) = load_orchestration_run_any(&transaction, &request.run_id)?;
        if run.status != OrchestrationRunStatus::Sealed {
            return Err(Error::Conflict(
                "shadow evaluation requires one sealed role report".to_owned(),
            ));
        }
        let report = load_advisory_report(&transaction, &run.run_id)?
            .ok_or_else(|| Error::Conflict("sealed run is missing its role report".to_owned()))?;
        let task = load_task(&transaction, &run.task_id)?
            .ok_or_else(|| Error::NotFound(format!("task {}", run.task_id)))?;
        let actual_task_outcome = match task.status {
            TaskStatus::Completed => ActualTaskOutcome::Completed,
            TaskStatus::Cancelled => ActualTaskOutcome::Cancelled,
            TaskStatus::Queued | TaskStatus::Leased => {
                return Err(Error::Conflict(
                    "shadow evaluation requires the bound task to be completed or cancelled"
                        .to_owned(),
                ));
            }
        };
        let prediction_match = match report.predicted_task_outcome {
            PredictedTaskOutcome::Completion => {
                Some(actual_task_outcome == ActualTaskOutcome::Completed)
            }
            PredictedTaskOutcome::NonCompletion => {
                Some(actual_task_outcome == ActualTaskOutcome::Cancelled)
            }
            PredictedTaskOutcome::Uncertain => None,
        };
        let git_stale = orchestration_git_stale(&transaction, &project_id, &run)?;
        let criterion_count = task.acceptance_criteria.len() as u64;
        let addressed_count = report.addressed_criteria.len() as u64;
        let verified_count = task.completion_proofs.len() as u64;
        let evidence_eligible = actual_task_outcome == ActualTaskOutcome::Completed
            && verified_count == criterion_count
            && criterion_count > 0
            && !git_stale;
        let evaluation_id = Uuid::now_v7().to_string();
        let evaluation_sha256 = shadow_evaluation_digest(
            &evaluation_id,
            &run.run_id,
            &actual_task_outcome,
            prediction_match,
            criterion_count,
            addressed_count,
            verified_count,
            git_stale,
            evidence_eligible,
            now,
        )?;
        transaction.execute(
            "INSERT INTO shadow_evaluations(
                evaluation_id, run_id, actual_task_outcome, prediction_match,
                acceptance_criterion_count, addressed_criterion_count,
                verified_criterion_count, git_stale, evidence_eligible,
                evaluation_sha256, created_at_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                evaluation_id,
                run.run_id,
                actual_task_outcome.as_str(),
                prediction_match,
                criterion_count,
                addressed_count,
                verified_count,
                git_stale,
                evidence_eligible,
                evaluation_sha256,
                now,
            ],
        )?;
        transaction.execute(
            "UPDATE orchestration_runs SET status = 'evaluated', evaluated_at_unix_ms = ?1
             WHERE run_id = ?2",
            params![now, run.run_id],
        )?;
        let view = load_orchestration_view(&transaction, &project_id, &run.run_id)?;
        let outcome = OrchestrationOutcome {
            view,
            duplicate: false,
        };
        append_event(
            &transaction,
            &request.idempotency_key,
            &request.run_id,
            "shadow_run_evaluated",
            request,
            &outcome,
            now,
        )?;
        transaction.commit()?;
        Ok(outcome)
    }

    pub fn get_orchestration_run(
        &self,
        request: &OrchestrationRunGetRequest,
    ) -> Result<OrchestrationRunView> {
        let workspace = canonical_workspace(&request.workspace)?;
        require_text("run_id", &request.run_id)?;
        let connection = self.connection()?;
        let project_id = connection
            .query_row(
                "SELECT project_id FROM projects WHERE workspace = ?1",
                [&workspace],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("workspace {workspace}")))?;
        load_orchestration_view(&connection, &project_id, &request.run_id)
    }

    pub fn list_orchestration_runs(
        &self,
        request: &OrchestrationRunListRequest,
    ) -> Result<Vec<OrchestrationRunSummary>> {
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
        let mut statement = connection.prepare(
            "SELECT run_id, task_id, mode, role_kind, role_id, status,
                    CASE WHEN mode = 'visible_advisory' OR status = 'evaluated' THEN 1 ELSE 0 END,
                    (SELECT evidence_eligible FROM shadow_evaluations
                     WHERE shadow_evaluations.run_id = orchestration_runs.run_id),
                    created_at_unix_ms
             FROM orchestration_runs WHERE project_id = ?1
             ORDER BY sequence DESC LIMIT ?2",
        )?;
        Ok(statement
            .query_map(params![project_id, limit], |row| {
                Ok(OrchestrationRunSummary {
                    run_id: row.get(0)?,
                    task_id: row.get(1)?,
                    mode: parse_advisory_mode(row.get(2)?)?,
                    role_kind: parse_advisory_role_kind(row.get(3)?)?,
                    role_id: row.get(4)?,
                    status: parse_orchestration_status(row.get(5)?)?,
                    report_revealed: row.get(6)?,
                    evidence_eligible: row.get(7)?,
                    created_at_unix_ms: row.get(8)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn audit_orchestration(&self) -> Result<OrchestrationAudit> {
        let connection = self.connection()?;
        let runs = load_all_orchestration_runs(&connection)?;
        let reports = load_all_advisory_reports(&connection)?;
        let evaluations = load_all_shadow_evaluations(&connection)?;
        let mut digest_mismatch_count = 0_u64;
        for run in &runs {
            if orchestration_run_digest_from_value(run)? != run.run_sha256 {
                digest_mismatch_count += 1;
            }
        }
        for report in &reports {
            if advisory_report_digest_from_value(report)? != report.report_sha256 {
                digest_mismatch_count += 1;
            }
        }
        for evaluation in &evaluations {
            if shadow_evaluation_digest_from_value(evaluation)? != evaluation.evaluation_sha256 {
                digest_mismatch_count += 1;
            }
        }
        let invalid_reference_count = connection.query_row(
            "SELECT COUNT(*) FROM orchestration_runs runs
             LEFT JOIN sessions ON sessions.session_id = runs.session_id
             LEFT JOIN tasks ON tasks.task_id = runs.task_id
             LEFT JOIN git_snapshots ON git_snapshots.snapshot_id = runs.git_snapshot_id
             LEFT JOIN memory_exposures ON memory_exposures.exposure_id = runs.context_exposure_id
             LEFT JOIN role_appointments ON role_appointments.appointment_id = runs.role_appointment_id
             WHERE sessions.project_id IS NULL OR tasks.project_id IS NULL
                OR git_snapshots.project_id IS NULL OR memory_exposures.project_id IS NULL
                OR sessions.project_id != runs.project_id OR tasks.project_id != runs.project_id
                OR git_snapshots.project_id != runs.project_id
                OR memory_exposures.project_id != runs.project_id
                OR (runs.role_appointment_id IS NOT NULL AND
                    (role_appointments.project_id IS NULL
                     OR role_appointments.project_id != runs.project_id
                     OR role_appointments.session_id != runs.session_id
                     OR (role_appointments.task_id IS NOT NULL AND role_appointments.task_id != runs.task_id)))",
            [],
            |row| row.get::<_, u64>(0),
        )?;
        Ok(OrchestrationAudit {
            run_count: runs.len() as u64,
            report_count: reports.len() as u64,
            evaluation_count: evaluations.len() as u64,
            digest_mismatch_count,
            invalid_reference_count,
            consistent: digest_mismatch_count == 0 && invalid_reference_count == 0,
        })
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
            git_snapshot_count: table_count(&connection, "git_snapshots", "1 = 1")?,
            token_usage_receipt_count: table_count(&connection, "token_usage_receipts", "1 = 1")?,
            deliberation_count: table_count(&connection, "deliberations", "1 = 1")?,
            deliberation_decision_count: table_count(
                &connection,
                "deliberation_decisions",
                "1 = 1",
            )?,
            capability_manifest_count: table_count(&connection, "capability_manifests", "1 = 1")?,
            experiment_campaign_count: table_count(&connection, "experiment_campaigns", "1 = 1")?,
            security_assessment_count: table_count(&connection, "security_assessments", "1 = 1")?,
            orchestration_run_count: table_count(&connection, "orchestration_runs", "1 = 1")?,
            shadow_evaluation_count: table_count(&connection, "shadow_evaluations", "1 = 1")?,
            open_repair_case_count: table_count(
                &connection,
                "accountability_cases",
                "status = 'open'",
            )?,
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
                        records.verifies_effect_id, records.origin_channel,
                        records.influence_class, records.created_at_unix_ms
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
                        origin_channel: parse_origin_channel(row.get(7)?)?,
                        influence_class: parse_influence_class(row.get(8)?)?,
                        created_at_unix_ms: row.get(9)?,
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
        let memory_items = {
            let sql = format!(
                "{} FROM memory_items WHERE project_id = ?1
                 ORDER BY created_at_unix_ms ASC, memory_id ASC",
                memory_select()
            );
            let mut statement = connection.prepare(&sql)?;
            statement
                .query_map([&project_id], memory_item_from_row)?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let memory_edges = {
            let mut statement = connection.prepare(
                "SELECT edge_id, from_memory_id, to_memory_id, relation, created_at_unix_ms
                 FROM memory_edges WHERE project_id = ?1
                 ORDER BY created_at_unix_ms ASC, edge_id ASC",
            )?;
            statement
                .query_map([&project_id], |row| {
                    Ok(MemoryEdge {
                        edge_id: row.get(0)?,
                        from_memory_id: row.get(1)?,
                        to_memory_id: row.get(2)?,
                        relation: row.get(3)?,
                        created_at_unix_ms: row.get(4)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let memory_exposures = {
            let mut statement = connection.prepare(
                "SELECT exposure_id, event_kind, host_session_hmac, host_turn_hmac,
                        policy_sha256, memory_ids_json, content_bytes, created_at_unix_ms
                 FROM memory_exposures WHERE project_id = ?1
                 ORDER BY created_at_unix_ms ASC, exposure_id ASC",
            )?;
            let rows = statement.query_map([&project_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, u32>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            })?;
            rows.map(|row| {
                let (id, kind, session, turn, policy, ids, bytes, created) = row?;
                Ok(MemoryExposure {
                    exposure_id: id,
                    event_kind: kind,
                    host_session_hmac: session,
                    host_turn_hmac: turn,
                    policy_sha256: policy,
                    memory_ids: serde_json::from_str(&ids)?,
                    content_bytes: bytes,
                    created_at_unix_ms: created,
                })
            })
            .collect::<Result<Vec<_>>>()?
        };
        let runtime_events = {
            let mut statement = connection.prepare(&format!(
                "{} FROM runtime_events WHERE project_id = ?1 ORDER BY sequence ASC",
                runtime_event_select()
            ))?;
            statement
                .query_map([&project_id], runtime_event_from_row)?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let capability_observations = load_capability_observations(&connection, &project_id)?;
        let git_snapshots = {
            let mut statement = connection.prepare(&format!(
                "{} FROM git_snapshots WHERE project_id = ?1 ORDER BY sequence ASC",
                git_snapshot_select()
            ))?;
            statement
                .query_map([&project_id], git_snapshot_from_row)?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let token_usage_receipts = {
            let mut statement = connection.prepare(&format!(
                "{} FROM token_usage_receipts WHERE project_id = ?1 ORDER BY sequence ASC",
                token_usage_select()
            ))?;
            statement
                .query_map([&project_id], token_usage_from_row)?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let deliberations = load_project_deliberations(&connection, &project_id)?;
        let deliberation_nodes = load_project_deliberation_nodes(&connection, &project_id)?;
        let deliberation_edges = load_project_deliberation_edges(&connection, &project_id)?;
        let deliberation_decisions = load_project_deliberation_decisions(&connection, &project_id)?;
        let capability_manifests = {
            let mut statement = connection.prepare(
                "SELECT capability_id, version FROM capability_manifests
                 WHERE project_id = ?1 ORDER BY sequence ASC",
            )?;
            let keys = statement
                .query_map([&project_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            keys.iter()
                .map(|(id, version)| {
                    load_capability_manifest(&connection, &project_id, id, version)
                })
                .collect::<Result<Vec<_>>>()?
        };
        let experiments = {
            let mut statement = connection.prepare(
                "SELECT campaign_id FROM experiment_campaigns
                 WHERE project_id = ?1 ORDER BY sequence ASC",
            )?;
            let ids = statement
                .query_map([&project_id], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            ids.iter()
                .map(|id| load_experiment_portfolio(&connection, &project_id, id, 500))
                .collect::<Result<Vec<_>>>()?
        };
        let security_assessments = {
            let mut statement = connection.prepare(
                "SELECT assessment_id FROM security_assessments
                 WHERE project_id = ?1 ORDER BY sequence ASC",
            )?;
            let ids = statement
                .query_map([&project_id], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            ids.iter()
                .map(|id| load_security_assessment(&connection, &project_id, id))
                .collect::<Result<Vec<_>>>()?
        };
        let orchestration_runs = {
            let mut statement = connection.prepare(&format!(
                "{} FROM orchestration_runs WHERE project_id = ?1 ORDER BY sequence ASC",
                orchestration_run_select()
            ))?;
            statement
                .query_map([&project_id], orchestration_run_from_row)?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let sealed_advisory_report_count = connection.query_row(
            "SELECT COUNT(*) FROM advisory_role_reports AS reports
             JOIN orchestration_runs AS runs ON runs.run_id = reports.run_id
             WHERE runs.project_id = ?1 AND runs.mode = 'blind_shadow'
                   AND runs.status != 'evaluated'",
            [&project_id],
            |row| row.get::<_, u64>(0),
        )?;
        let advisory_role_reports = {
            let mut statement = connection.prepare(&format!(
                "{} FROM advisory_role_reports AS reports
                 WHERE reports.run_id IN (
                     SELECT run_id FROM orchestration_runs
                     WHERE project_id = ?1
                       AND (mode = 'visible_advisory' OR status = 'evaluated')
                 )
                 ORDER BY reports.sequence ASC",
                advisory_report_select()
            ))?;
            statement
                .query_map([&project_id], advisory_report_from_row)?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let shadow_evaluations = {
            let mut statement = connection.prepare(&format!(
                "{} FROM shadow_evaluations
                 WHERE run_id IN (SELECT run_id FROM orchestration_runs WHERE project_id = ?1)
                 ORDER BY sequence ASC",
                shadow_evaluation_select()
            ))?;
            statement
                .query_map([&project_id], shadow_evaluation_from_row)?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let role_appointments = {
            let mut statement = connection.prepare(
                "SELECT appointment_id FROM role_appointments WHERE project_id = ?1 ORDER BY sequence ASC",
            )?;
            let ids = statement
                .query_map([&project_id], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            ids.iter()
                .map(|id| load_role_appointment(&connection, id))
                .collect::<Result<Vec<_>>>()?
        };
        let office_appointments = {
            let mut statement = connection.prepare(
                "SELECT office_appointment_id FROM office_appointments
                 WHERE project_id = ?1 ORDER BY sequence ASC",
            )?;
            let ids = statement
                .query_map([&project_id], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            ids.iter()
                .map(|id| load_office_appointment(&connection, id))
                .collect::<Result<Vec<_>>>()?
        };
        let product_cells = {
            let mut statement = connection.prepare(
                "SELECT cell_id FROM product_cells WHERE project_id = ?1 ORDER BY sequence ASC",
            )?;
            let ids = statement
                .query_map([&project_id], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            ids.iter()
                .map(|id| load_product_cell(&connection, id))
                .collect::<Result<Vec<_>>>()?
        };
        let accountability_cases = {
            let mut statement = connection.prepare(
                "SELECT case_id FROM accountability_cases WHERE project_id = ?1
                 ORDER BY sequence ASC",
            )?;
            let ids = statement
                .query_map([&project_id], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            ids.iter()
                .map(|id| load_accountability_case(&connection, id))
                .collect::<Result<Vec<_>>>()?
        };

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
                 ) OR stream_id IN (
                     SELECT deliberation_id FROM deliberations WHERE project_id = ?1
                 ) OR stream_id IN (
                     SELECT campaign_id FROM experiment_campaigns WHERE project_id = ?1
                 ) OR stream_id IN (
                     SELECT assessment_id FROM security_assessments WHERE project_id = ?1
                 ) OR stream_id IN (
                     SELECT run_id FROM orchestration_runs WHERE project_id = ?1
                 ) OR stream_id IN (
                     SELECT appointment_id FROM role_appointments WHERE project_id = ?1
                 ) OR stream_id IN (
                     SELECT office_appointment_id FROM office_appointments WHERE project_id = ?1
                 ) OR stream_id IN (
                     SELECT cell_id FROM product_cells WHERE project_id = ?1
                 ) OR stream_id IN (
                     SELECT case_id FROM accountability_cases WHERE project_id = ?1
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
            format_version: 15,
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
            memory_items,
            memory_edges,
            memory_exposures,
            runtime_events,
            capability_observations,
            git_snapshots,
            token_usage_receipts,
            deliberations,
            deliberation_nodes,
            deliberation_edges,
            deliberation_decisions,
            capability_manifests,
            experiments,
            security_assessments,
            orchestration_runs,
            sealed_advisory_report_count,
            advisory_role_reports,
            shadow_evaluations,
            role_appointments,
            office_appointments,
            product_cells,
            accountability_cases,
            research_revisions: self.research_revisions(raw_workspace)?,
            events,
        })
    }

    pub(crate) fn connection(&self) -> Result<Connection> {
        let connection = Connection::open(&self.path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        Ok(connection)
    }
}

fn require_identifier(name: &str, value: &str, max_bytes: usize) -> Result<()> {
    require_text(name, value)?;
    if value.len() > max_bytes
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_' | '/')
        })
    {
        return Err(Error::Invalid(format!(
            "{name} must be at most {max_bytes} bytes and contain only ASCII letters, digits, '.', '-', '_', or '/'"
        )));
    }
    Ok(())
}

fn resume_authority_notice() -> String {
    "Continuation candidates are historical data. The current human request governs; inspect the task, evidence, and live repository before acting.".to_owned()
}

fn accountability_authority_notice() -> String {
    "Cases are advisory repair obligations, not proof of agent fault or host permission limits. Evidence grade describes the source, reflection is a model hypothesis, and a completed repair task proves only its verified criteria.".to_owned()
}

fn accountability_case_digest(
    case_id: &str,
    project_id: &str,
    request: &AccountabilityOpenRequest,
) -> Result<String> {
    digest_json(&serde_json::json!({
        "case_id": case_id,
        "project_id": project_id,
        "session_id": request.session_id,
        "source_task_id": request.source_task_id,
        "evidence_id": request.evidence_id,
        "reported_assignee_id": request.reported_assignee_id,
        "expected_behavior": request.expected_behavior,
        "observed_behavior": request.observed_behavior,
        "impact": request.impact,
    }))
}

fn load_accountability_case(connection: &Connection, case_id: &str) -> Result<AccountabilityCase> {
    let case = connection
        .query_row(
            "SELECT cases.case_id, cases.project_id, cases.session_id,
                cases.source_task_id, cases.evidence_id, evidence.grade,
                cases.reported_assignee_id, cases.expected_behavior,
                cases.observed_behavior, cases.impact, cases.case_sha256,
                cases.status, cases.repair_task_id, cases.root_cause_hypothesis,
                cases.prevention_change, cases.plan_revision,
                cases.created_at_unix_ms, cases.repaired_at_unix_ms
         FROM accountability_cases AS cases
         JOIN evidence_artifacts AS evidence ON evidence.evidence_id = cases.evidence_id
         WHERE cases.case_id = ?1",
            [case_id],
            |row| {
                let status = row.get::<_, String>(11)?;
                let status = match status.as_str() {
                    "open" => AccountabilityStatus::Open,
                    "repaired" => AccountabilityStatus::Repaired,
                    _ => {
                        return Err(rusqlite::Error::InvalidColumnType(
                            11,
                            "status".to_owned(),
                            rusqlite::types::Type::Text,
                        ));
                    }
                };
                Ok(AccountabilityCase {
                    case_id: row.get(0)?,
                    project_id: row.get(1)?,
                    session_id: row.get(2)?,
                    source_task_id: row.get(3)?,
                    evidence_id: row.get(4)?,
                    evidence_grade: parse_evidence_grade_value(&row.get::<_, String>(5)?)?,
                    reported_assignee_id: row.get(6)?,
                    expected_behavior: row.get(7)?,
                    observed_behavior: row.get(8)?,
                    impact: row.get(9)?,
                    case_sha256: row.get(10)?,
                    status,
                    repair_task_id: row.get(12)?,
                    root_cause_hypothesis: row.get(13)?,
                    prevention_change: row.get(14)?,
                    plan_revision: row.get(15)?,
                    created_at_unix_ms: row.get(16)?,
                    repaired_at_unix_ms: row.get(17)?,
                    advisory: true,
                })
            },
        )
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("accountability case {case_id}")))?;
    let expected_digest = accountability_case_digest(
        &case.case_id,
        &case.project_id,
        &AccountabilityOpenRequest {
            session_id: case.session_id.clone(),
            source_task_id: case.source_task_id.clone(),
            evidence_id: case.evidence_id.clone(),
            reported_assignee_id: case.reported_assignee_id.clone(),
            expected_behavior: case.expected_behavior.clone(),
            observed_behavior: case.observed_behavior.clone(),
            impact: case.impact.clone(),
            idempotency_key: String::new(),
        },
    )?;
    if expected_digest != case.case_sha256 {
        return Err(Error::Conflict(
            "accountability case digest mismatch".to_owned(),
        ));
    }
    let session_project: String = connection.query_row(
        "SELECT project_id FROM sessions WHERE session_id = ?1",
        [&case.session_id],
        |row| row.get(0),
    )?;
    let evidence_project: String = connection.query_row(
        "SELECT sessions.project_id FROM evidence_artifacts AS evidence
         JOIN sessions ON sessions.session_id = evidence.session_id
         WHERE evidence.evidence_id = ?1",
        [&case.evidence_id],
        |row| row.get(0),
    )?;
    let source_task = load_task(connection, &case.source_task_id)?
        .ok_or_else(|| Error::NotFound(format!("task {}", case.source_task_id)))?;
    if session_project != case.project_id
        || evidence_project != case.project_id
        || source_task.project_id != case.project_id
    {
        return Err(Error::Conflict(
            "accountability case workspace binding mismatch".to_owned(),
        ));
    }
    match (
        &case.repair_task_id,
        &case.root_cause_hypothesis,
        &case.prevention_change,
        case.plan_revision,
    ) {
        (None, None, None, 0) => {}
        (Some(repair_id), Some(_), Some(_), revision) if revision > 0 => {
            let repair = load_task(connection, repair_id)?
                .ok_or_else(|| Error::NotFound(format!("task {repair_id}")))?;
            if repair.project_id != case.project_id
                || repair.task_id == case.source_task_id
                || repair.created_at_unix_ms < case.created_at_unix_ms
            {
                return Err(Error::Conflict(
                    "accountability repair task binding mismatch".to_owned(),
                ));
            }
            if case.status == AccountabilityStatus::Repaired
                && (repair.status != TaskStatus::Completed || repair.completion_proofs.is_empty())
            {
                return Err(Error::Conflict(
                    "repaired case lacks verified repair task".to_owned(),
                ));
            }
            if case.status == AccountabilityStatus::Repaired {
                validate_criterion_proofs(connection, &repair, &repair.completion_proofs)?;
            }
        }
        _ => {
            return Err(Error::Conflict(
                "accountability plan state mismatch".to_owned(),
            ));
        }
    }
    if (case.status == AccountabilityStatus::Open) != case.repaired_at_unix_ms.is_none() {
        return Err(Error::Conflict(
            "accountability repair status mismatch".to_owned(),
        ));
    }
    Ok(case)
}

fn validate_accountability_history(
    connection: &Connection,
    case: &AccountabilityCase,
) -> Result<()> {
    let mut statement = connection.prepare(
        "SELECT kind, payload_json, result_json FROM events
         WHERE stream_id = ?1 AND kind IN (
             'accountability_case_opened', 'accountability_repair_planned',
             'accountability_case_repaired')
         ORDER BY sequence ASC",
    )?;
    let events = statement
        .query_map([&case.case_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut opened = 0_u32;
    let mut plans = 0_u32;
    let mut repaired = 0_u32;
    let mut latest_plan: Option<AccountabilityPlanRequest> = None;
    let mut last_event_case: Option<AccountabilityCase> = None;
    for (kind, payload, result) in events {
        let outcome: AccountabilityOutcome = serde_json::from_str(&result)?;
        if outcome.case.case_id != case.case_id || outcome.duplicate {
            return Err(Error::Conflict(
                "accountability event result mismatch".to_owned(),
            ));
        }
        match kind.as_str() {
            "accountability_case_opened" if opened == 0 && plans == 0 && repaired == 0 => {
                opened += 1;
                let request: AccountabilityOpenRequest = serde_json::from_str(&payload)?;
                if request.session_id != case.session_id
                    || request.source_task_id != case.source_task_id
                    || request.evidence_id != case.evidence_id
                    || request.reported_assignee_id != case.reported_assignee_id
                    || request.expected_behavior != case.expected_behavior
                    || request.observed_behavior != case.observed_behavior
                    || request.impact != case.impact
                    || outcome.case.plan_revision != 0
                    || outcome.case.status != AccountabilityStatus::Open
                {
                    return Err(Error::Conflict(
                        "accountability opening event mismatch".to_owned(),
                    ));
                }
            }
            "accountability_repair_planned" if opened == 1 && repaired == 0 => {
                plans += 1;
                let request: AccountabilityPlanRequest = serde_json::from_str(&payload)?;
                if request.case_id != case.case_id
                    || outcome.case.plan_revision != plans
                    || outcome.case.repair_task_id.as_deref()
                        != Some(request.repair_task_id.as_str())
                    || outcome.case.root_cause_hypothesis.as_deref()
                        != Some(request.root_cause_hypothesis.as_str())
                    || outcome.case.prevention_change.as_deref()
                        != Some(request.prevention_change.as_str())
                {
                    return Err(Error::Conflict(
                        "accountability plan event mismatch".to_owned(),
                    ));
                }
                latest_plan = Some(request);
            }
            "accountability_case_repaired" if opened == 1 && plans > 0 && repaired == 0 => {
                repaired += 1;
                let request: AccountabilityResolveRequest = serde_json::from_str(&payload)?;
                if request.case_id != case.case_id
                    || outcome.case.status != AccountabilityStatus::Repaired
                    || outcome.case.plan_revision != plans
                {
                    return Err(Error::Conflict(
                        "accountability repair event mismatch".to_owned(),
                    ));
                }
            }
            _ => {
                return Err(Error::Conflict(
                    "accountability event order mismatch".to_owned(),
                ));
            }
        }
        last_event_case = Some(outcome.case);
    }
    if opened != 1
        || plans != case.plan_revision
        || repaired != u32::from(case.status == AccountabilityStatus::Repaired)
    {
        return Err(Error::Conflict(
            "accountability event count mismatch".to_owned(),
        ));
    }
    if let Some(plan) = latest_plan
        && (case.repair_task_id.as_deref() != Some(plan.repair_task_id.as_str())
            || case.root_cause_hypothesis.as_deref() != Some(plan.root_cause_hypothesis.as_str())
            || case.prevention_change.as_deref() != Some(plan.prevention_change.as_str()))
    {
        return Err(Error::Conflict(
            "accountability plan projection mismatch".to_owned(),
        ));
    }
    if last_event_case.as_ref() != Some(case) {
        return Err(Error::Conflict(
            "accountability latest event projection mismatch".to_owned(),
        ));
    }
    Ok(())
}

fn role_appointment_digest(id: &str, request: &RoleAppointmentCreateRequest) -> Result<String> {
    digest_json(&serde_json::json!({
        "appointment_id": id,
        "session_id": request.session_id,
        "task_id": request.task_id,
        "role_id": request.role_id,
        "role_version": request.role_version,
        "assignee_id": request.assignee_id,
        "model_hint": request.model_hint,
        "capability_refs": request.capability_refs,
    }))
}

fn load_role_appointment(connection: &Connection, id: &str) -> Result<RoleAppointment> {
    let appointment = connection
        .query_row(
            "SELECT appointment_id, session_id, task_id, role_id, role_version, assignee_id,
         model_hint, capability_refs_json, appointment_sha256, created_at_unix_ms, revoked_at_unix_ms
         FROM role_appointments WHERE appointment_id = ?1",
            [id],
            |row| {
                Ok(RoleAppointment {
                    appointment_id: row.get(0)?,
                    session_id: row.get(1)?,
                    task_id: row.get(2)?,
                    role_id: row.get(3)?,
                    role_version: row.get(4)?,
                    assignee_id: row.get(5)?,
                    model_hint: row.get(6)?,
                    capability_refs: serde_json::from_str::<Vec<RoleCapabilityRef>>(&row.get::<_, String>(7)?)
                        .map_err(|error| rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(error)))?,
                    appointment_sha256: row.get(8)?,
                    created_at_unix_ms: row.get(9)?,
                    revoked_at_unix_ms: row.get(10)?,
                    advisory: true,
                    grants_authority: false,
                })
            },
        )
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("appointment {id}")))?;
    let expected = digest_json(&serde_json::json!({
        "appointment_id": appointment.appointment_id,
        "session_id": appointment.session_id,
        "task_id": appointment.task_id,
        "role_id": appointment.role_id,
        "role_version": appointment.role_version,
        "assignee_id": appointment.assignee_id,
        "model_hint": appointment.model_hint,
        "capability_refs": appointment.capability_refs,
    }))?;
    if expected != appointment.appointment_sha256 {
        return Err(Error::Conflict(
            "role appointment digest mismatch".to_owned(),
        ));
    }
    Ok(appointment)
}

fn office_appointment_digest(
    id: &str,
    request: &OfficeAppointmentCreateRequest,
    assignee_id: &str,
) -> Result<String> {
    digest_json(&serde_json::json!({
        "office_appointment_id": id,
        "session_id": request.session_id,
        "office_id": request.office_id,
        "office_version": request.office_version,
        "role_appointment_id": request.role_appointment_id,
        "assignee_id": assignee_id,
    }))
}

fn load_office_appointment(connection: &Connection, id: &str) -> Result<OfficeAppointment> {
    let appointment = connection
        .query_row(
            "SELECT office_appointment_id, session_id, office_id, office_version,
                    role_appointment_id, assignee_id, appointment_sha256,
                    created_at_unix_ms, revoked_at_unix_ms
             FROM office_appointments WHERE office_appointment_id = ?1",
            [id],
            |row| {
                Ok(OfficeAppointment {
                    office_appointment_id: row.get(0)?,
                    session_id: row.get(1)?,
                    office_id: row.get(2)?,
                    office_version: row.get(3)?,
                    role_appointment_id: row.get(4)?,
                    assignee_id: row.get(5)?,
                    appointment_sha256: row.get(6)?,
                    created_at_unix_ms: row.get(7)?,
                    revoked_at_unix_ms: row.get(8)?,
                    advisory: true,
                    grants_authority: false,
                })
            },
        )
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("office appointment {id}")))?;
    if crate::government::office(&appointment.office_id, appointment.office_version).is_none() {
        return Err(Error::Conflict(
            "office appointment references an unknown office definition".to_owned(),
        ));
    }
    let role = load_role_appointment(connection, &appointment.role_appointment_id)?;
    let office = crate::government::office(&appointment.office_id, appointment.office_version)
        .expect("office definition checked above");
    if role.session_id != appointment.session_id
        || role.task_id.is_some()
        || role.role_id != office.head_role_id
        || role.assignee_id != appointment.assignee_id
    {
        return Err(Error::Conflict(
            "office appointment role binding mismatch".to_owned(),
        ));
    }
    let expected = digest_json(&serde_json::json!({
        "office_appointment_id": appointment.office_appointment_id,
        "session_id": appointment.session_id,
        "office_id": appointment.office_id,
        "office_version": appointment.office_version,
        "role_appointment_id": appointment.role_appointment_id,
        "assignee_id": appointment.assignee_id,
    }))?;
    if expected != appointment.appointment_sha256 {
        return Err(Error::Conflict(
            "office appointment digest mismatch".to_owned(),
        ));
    }
    Ok(appointment)
}

fn product_cell_digest(
    id: &str,
    request: &ProductCellCreateRequest,
    members: &[ProductCellMember],
) -> Result<String> {
    digest_json(&serde_json::json!({
        "cell_id": id,
        "session_id": request.session_id,
        "task_id": request.task_id,
        "office_appointment_id": request.office_appointment_id,
        "title": request.title,
        "problem_statement": request.problem_statement,
        "hypothesis": request.hypothesis,
        "success_measures": request.success_measures,
        "members": members,
    }))
}

fn load_product_cell(connection: &Connection, id: &str) -> Result<ProductCell> {
    let mut cell = connection
        .query_row(
            "SELECT cell_id, session_id, task_id, office_appointment_id, title,
                    problem_statement, hypothesis, success_measures_json, members_json,
                    cell_sha256, created_at_unix_ms
             FROM product_cells WHERE cell_id = ?1",
            [id],
            |row| {
                let success_measures = serde_json::from_str::<Vec<String>>(
                    &row.get::<_, String>(7)?,
                )
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        7,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
                let members =
                    serde_json::from_str::<Vec<ProductCellMember>>(&row.get::<_, String>(8)?)
                        .map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                8,
                                rusqlite::types::Type::Text,
                                Box::new(error),
                            )
                        })?;
                Ok(ProductCell {
                    cell_id: row.get(0)?,
                    session_id: row.get(1)?,
                    task_id: row.get(2)?,
                    office_appointment_id: row.get(3)?,
                    title: row.get(4)?,
                    problem_statement: row.get(5)?,
                    hypothesis: row.get(6)?,
                    success_measures,
                    members,
                    cell_sha256: row.get(9)?,
                    created_at_unix_ms: row.get(10)?,
                    active: false,
                    advisory: true,
                    grants_authority: false,
                })
            },
        )
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("product cell {id}")))?;
    let expected = digest_json(&serde_json::json!({
        "cell_id": cell.cell_id,
        "session_id": cell.session_id,
        "task_id": cell.task_id,
        "office_appointment_id": cell.office_appointment_id,
        "title": cell.title,
        "problem_statement": cell.problem_statement,
        "hypothesis": cell.hypothesis,
        "success_measures": cell.success_measures,
        "members": cell.members,
    }))?;
    if expected != cell.cell_sha256 {
        return Err(Error::Conflict("product cell digest mismatch".to_owned()));
    }
    let office = load_office_appointment(connection, &cell.office_appointment_id)?;
    if office.session_id != cell.session_id
        || office.office_id != crate::government::PRODUCT_EXPERIMENT_OFFICE_ID
    {
        return Err(Error::Conflict(
            "product cell office binding mismatch".to_owned(),
        ));
    }
    let duties = cell
        .members
        .iter()
        .map(|member| member.duty.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let assignees = cell
        .members
        .iter()
        .map(|member| member.assignee_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if cell.members.len() < 2
        || !duties.contains("product_planning")
        || !duties.contains("prototype_delivery")
        || assignees.len() < 2
        || assignees.contains(office.assignee_id.as_str())
    {
        return Err(Error::Conflict(
            "product cell multidisciplinary composition mismatch".to_owned(),
        ));
    }
    let mut composition_active = office.revoked_at_unix_ms.is_none();
    for member in &cell.members {
        let role = load_role_appointment(connection, &member.role_appointment_id)?;
        if role.session_id != cell.session_id
            || role.task_id.as_deref() != Some(cell.task_id.as_str())
            || role.role_id != "delivery.worker"
            || role.assignee_id != member.assignee_id
        {
            return Err(Error::Conflict(
                "product cell member role binding mismatch".to_owned(),
            ));
        }
        composition_active &= role.revoked_at_unix_ms.is_none();
    }
    let task = load_task(connection, &cell.task_id)?
        .ok_or_else(|| Error::NotFound(format!("task {}", cell.task_id)))?;
    cell.active =
        composition_active && !matches!(task.status, TaskStatus::Completed | TaskStatus::Cancelled);
    Ok(cell)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_capability_schema(
    name: &str,
    schema: &serde_json::Value,
    require_object_root: bool,
) -> Result<()> {
    let encoded = serde_json::to_vec(schema)?;
    if encoded.len() > 65_536 {
        return Err(Error::Invalid(format!("{name} exceeds 65536 bytes")));
    }
    let object = schema
        .as_object()
        .ok_or_else(|| Error::Invalid(format!("{name} must be a JSON object")))?;
    if require_object_root
        && object.get("type").and_then(serde_json::Value::as_str) != Some("object")
    {
        return Err(Error::Invalid(format!("{name} root type must be object")));
    }
    if json_depth(schema) > 32 {
        return Err(Error::Invalid(format!(
            "{name} exceeds the maximum nesting depth"
        )));
    }
    if contains_secret_material(schema) {
        return Err(Error::Invalid(format!(
            "{name} contains a credential-like field; manifests may describe credentials but never contain them"
        )));
    }
    Ok(())
}

fn json_depth(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Array(values) => 1 + values.iter().map(json_depth).max().unwrap_or(0),
        serde_json::Value::Object(values) => 1 + values.values().map(json_depth).max().unwrap_or(0),
        _ => 1,
    }
}

fn contains_secret_material(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(values) => values.iter().any(|(key, value)| {
            let key = key.to_ascii_lowercase();
            matches!(
                key.as_str(),
                "password" | "secret" | "token" | "authorization" | "api_key" | "apikey"
            ) && !value.is_null()
                || contains_secret_material(value)
        }),
        serde_json::Value::Array(values) => values.iter().any(contains_secret_material),
        _ => false,
    }
}

fn parse_capability_provider_kind(value: String) -> rusqlite::Result<CapabilityProviderKind> {
    match value.as_str() {
        "built_in" => Ok(CapabilityProviderKind::BuiltIn),
        "external_artifact" => Ok(CapabilityProviderKind::ExternalArtifact),
        "workspace_manifest" => Ok(CapabilityProviderKind::WorkspaceManifest),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_capability_effect_class(value: String) -> rusqlite::Result<CapabilityEffectClass> {
    match value.as_str() {
        "observe" => Ok(CapabilityEffectClass::Observe),
        "record_local" => Ok(CapabilityEffectClass::RecordLocal),
        "verify_local" => Ok(CapabilityEffectClass::VerifyLocal),
        "external_effect" => Ok(CapabilityEffectClass::ExternalEffect),
        "privileged" => Ok(CapabilityEffectClass::Privileged),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_capability_maturity(value: String) -> rusqlite::Result<CapabilityMaturity> {
    match value.as_str() {
        "observe" => Ok(CapabilityMaturity::Observe),
        "propose" => Ok(CapabilityMaturity::Propose),
        "sandboxed_execute" => Ok(CapabilityMaturity::SandboxedExecute),
        "connected_effect" => Ok(CapabilityMaturity::ConnectedEffect),
        "persistent_routine" => Ok(CapabilityMaturity::PersistentRoutine),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn validate_capability_maturity(
    effect: &CapabilityEffectClass,
    maturity: &CapabilityMaturity,
    idempotent: bool,
) -> Result<()> {
    let minimum = minimum_capability_maturity(effect);
    if maturity.rank() < minimum.rank() {
        return Err(Error::Invalid(format!(
            "capability maturity {} is below the minimum {} for effect class {}",
            maturity.as_str(),
            minimum.as_str(),
            effect.as_str()
        )));
    }
    if *maturity == CapabilityMaturity::PersistentRoutine && !idempotent {
        return Err(Error::Invalid(
            "persistent_routine capabilities must declare idempotent=true".to_owned(),
        ));
    }
    Ok(())
}

fn minimum_capability_maturity(effect: &CapabilityEffectClass) -> CapabilityMaturity {
    match effect {
        CapabilityEffectClass::Observe => CapabilityMaturity::Observe,
        CapabilityEffectClass::RecordLocal => CapabilityMaturity::Propose,
        CapabilityEffectClass::VerifyLocal => CapabilityMaturity::SandboxedExecute,
        CapabilityEffectClass::ExternalEffect | CapabilityEffectClass::Privileged => {
            CapabilityMaturity::ConnectedEffect
        }
    }
}

fn parse_capability_state(value: String) -> rusqlite::Result<CapabilityCatalogState> {
    match value.as_str() {
        "registered" => Ok(CapabilityCatalogState::Registered),
        "available" => Ok(CapabilityCatalogState::Available),
        "disabled" => Ok(CapabilityCatalogState::Disabled),
        "deprecated" => Ok(CapabilityCatalogState::Deprecated),
        "revoked" => Ok(CapabilityCatalogState::Revoked),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn latest_capability_state(
    connection: &Connection,
    project_id: &str,
    capability_id: &str,
    version: &str,
) -> Result<CapabilityCatalogState> {
    let state = connection.query_row(
        "SELECT state FROM capability_state_events
         WHERE project_id = ?1 AND capability_id = ?2 AND version = ?3
         ORDER BY sequence DESC LIMIT 1",
        params![project_id, capability_id, version],
        |row| row.get::<_, String>(0),
    )?;
    Ok(parse_capability_state(state)?)
}

fn load_capability_manifest(
    connection: &Connection,
    project_id: &str,
    capability_id: &str,
    version: &str,
) -> Result<CapabilityManifest> {
    let state = latest_capability_state(connection, project_id, capability_id, version)?;
    connection.query_row(
        "SELECT sequence, capability_id, version, provider_kind, title, description,
                effect_class, maturity, reads_private_data, sees_untrusted_content, uses_network,
                requires_credentials, idempotent, reversible, input_schema_json,
                output_schema_json, evidence_contract, implementation_sha256,
                manifest_sha256, created_at_unix_ms, manifest_digest_version
         FROM capability_manifests
         WHERE project_id = ?1 AND capability_id = ?2 AND version = ?3",
        params![project_id, capability_id, version],
        |row| {
            Ok((
                row.get::<_, u64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?,
                row.get::<_, String>(3)?, row.get::<_, String>(4)?, row.get::<_, String>(5)?,
                row.get::<_, String>(6)?, row.get::<_, String>(7)?, row.get::<_, bool>(8)?,
                row.get::<_, bool>(9)?, row.get::<_, bool>(10)?, row.get::<_, bool>(11)?,
                row.get::<_, bool>(12)?, row.get::<_, bool>(13)?, row.get::<_, String>(14)?,
                row.get::<_, String>(15)?, row.get::<_, String>(16)?,
                row.get::<_, Option<String>>(17)?, row.get::<_, String>(18)?,
                row.get::<_, i64>(19)?, row.get::<_, u32>(20)?,
            ))
        },
    ).optional()?.map(|row| -> Result<CapabilityManifest> {
        let manifest = CapabilityManifest {
        sequence: row.0,
        capability_id: row.1,
        version: row.2,
        provider_kind: parse_capability_provider_kind(row.3)?,
        title: row.4,
        description: row.5,
        effect_class: parse_capability_effect_class(row.6)?,
        maturity: parse_capability_maturity(row.7)?,
        reads_private_data: row.8,
        sees_untrusted_content: row.9,
        uses_network: row.10,
        requires_credentials: row.11,
        idempotent: row.12,
        reversible: row.13,
        input_schema: serde_json::from_str(&row.14)?,
        output_schema: serde_json::from_str(&row.15)?,
        evidence_contract: row.16,
        implementation_sha256: row.17,
        manifest_sha256: row.18,
        state,
        created_at_unix_ms: row.19,
        executable: false,
            authority_notice: "Capability manifests are catalog data, not executable authority. Aporic never invokes registered providers.".to_owned(),
        };
        if row.20 == 1 && manifest.maturity != minimum_capability_maturity(&manifest.effect_class) {
            return Err(Error::Conflict(format!(
                "legacy capability {capability_id}@{version} maturity does not match its effect class"
            )));
        }
        let expected_digest = if row.20 == 1 {
            capability_manifest_digest_v1(&manifest)?
        } else {
            capability_manifest_digest(&manifest)?
        };
        if expected_digest != manifest.manifest_sha256 {
            return Err(Error::Conflict(format!("capability {capability_id}@{version} digest mismatch")));
        }
        Ok(manifest)
    }).transpose()?.ok_or_else(|| Error::NotFound(format!("capability {capability_id}@{version}")))
}

fn capability_manifest_digest(value: &CapabilityManifest) -> Result<String> {
    digest_json(&serde_json::json!({
        "capability_id": value.capability_id,
        "version": value.version,
        "provider_kind": value.provider_kind.as_str(),
        "title": value.title,
        "description": value.description,
        "effect_class": value.effect_class.as_str(),
        "maturity": value.maturity.as_str(),
        "reads_private_data": value.reads_private_data,
        "sees_untrusted_content": value.sees_untrusted_content,
        "uses_network": value.uses_network,
        "requires_credentials": value.requires_credentials,
        "idempotent": value.idempotent,
        "reversible": value.reversible,
        "input_schema": value.input_schema,
        "output_schema": value.output_schema,
        "evidence_contract": value.evidence_contract,
        "implementation_sha256": value.implementation_sha256,
    }))
}

fn capability_manifest_digest_v1(value: &CapabilityManifest) -> Result<String> {
    digest_json(&serde_json::json!({
        "capability_id": value.capability_id,
        "version": value.version,
        "provider_kind": value.provider_kind.as_str(),
        "title": value.title,
        "description": value.description,
        "effect_class": value.effect_class.as_str(),
        "reads_private_data": value.reads_private_data,
        "sees_untrusted_content": value.sees_untrusted_content,
        "uses_network": value.uses_network,
        "requires_credentials": value.requires_credentials,
        "idempotent": value.idempotent,
        "reversible": value.reversible,
        "input_schema": value.input_schema,
        "output_schema": value.output_schema,
        "evidence_contract": value.evidence_contract,
        "implementation_sha256": value.implementation_sha256,
    }))
}

fn validate_criterion(input: &crate::domain::ExperimentCriterionInput) -> Result<()> {
    match (&input.kind, &input.comparison) {
        (ExperimentCriterionKind::HardGate, ExperimentComparison::MustPass) => {
            if input.threshold.is_some_and(|value| value != 1) {
                return Err(Error::Invalid(
                    "must_pass threshold, when supplied, must equal 1".to_owned(),
                ));
            }
        }
        (
            ExperimentCriterionKind::HardGate,
            ExperimentComparison::Gte | ExperimentComparison::Lte,
        ) => {
            if input.threshold.is_none() {
                return Err(Error::Invalid(
                    "gte/lte hard gates require a threshold".to_owned(),
                ));
            }
        }
        (
            ExperimentCriterionKind::ParetoDimension,
            ExperimentComparison::Minimize | ExperimentComparison::Maximize,
        ) => {
            if input.threshold.is_some() {
                return Err(Error::Invalid(
                    "Pareto dimensions do not accept thresholds".to_owned(),
                ));
            }
        }
        _ => {
            return Err(Error::Invalid(
                "hard gates use must_pass/gte/lte; Pareto dimensions use minimize/maximize"
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

fn normalized_threshold(input: &crate::domain::ExperimentCriterionInput) -> Option<i64> {
    if input.comparison == ExperimentComparison::MustPass {
        Some(1)
    } else {
        input.threshold
    }
}

fn load_clean_project_snapshot(
    connection: &Connection,
    project_id: &str,
    snapshot_id: &str,
    purpose: &str,
) -> Result<GitSnapshot> {
    let snapshot = connection
        .query_row(
            &format!(
                "{} FROM git_snapshots WHERE project_id = ?1 AND snapshot_id = ?2",
                git_snapshot_select()
            ),
            params![project_id, snapshot_id],
            git_snapshot_from_row,
        )
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("Git snapshot {snapshot_id}")))?;
    if git_snapshot_digest(&snapshot)? != snapshot.snapshot_sha256 {
        return Err(Error::Conflict(format!(
            "cannot bind {purpose} to a Git snapshot with a digest mismatch"
        )));
    }
    if snapshot.dirty || snapshot.head_commit.is_none() || snapshot.head_tree.is_none() {
        return Err(Error::Invalid(format!(
            "{purpose} requires a clean committed Git snapshot"
        )));
    }
    Ok(snapshot)
}

fn orchestration_run_digest(
    run_id: &str,
    request: &OrchestrationRunCreateRequest,
    bound_head_commit: &str,
    bound_head_tree: &str,
    context_policy_sha256: &str,
) -> Result<String> {
    let mut value = serde_json::json!({
        "run_id": run_id,
        "session_id": request.session_id,
        "task_id": request.task_id,
        "git_snapshot_id": request.git_snapshot_id,
        "context_exposure_id": request.context_exposure_id,
        "mode": request.mode.as_str(),
        "role_kind": request.role_kind.as_str(),
        "role_id": request.role_id.trim(),
        "objective": request.objective.trim(),
        "model": request.model,
        "reasoning_effort": request.reasoning_effort,
        "max_input_tokens": request.max_input_tokens,
        "max_output_tokens": request.max_output_tokens,
        "max_duration_seconds": request.max_duration_seconds,
        "capability_ceiling": "propose",
        "bound_head_commit": bound_head_commit,
        "bound_head_tree": bound_head_tree,
        "context_policy_sha256": context_policy_sha256,
    });
    if let Some(appointment_id) = &request.role_appointment_id {
        value["role_appointment_id"] = serde_json::json!(appointment_id);
    }
    digest_json(&value)
}

fn orchestration_run_digest_from_value(run: &OrchestrationRun) -> Result<String> {
    let mut value = serde_json::json!({
        "run_id": run.run_id,
        "session_id": run.session_id,
        "task_id": run.task_id,
        "git_snapshot_id": run.git_snapshot_id,
        "context_exposure_id": run.context_exposure_id,
        "mode": run.mode.as_str(),
        "role_kind": run.role_kind.as_str(),
        "role_id": run.role_id,
        "objective": run.objective,
        "model": run.model,
        "reasoning_effort": run.reasoning_effort,
        "max_input_tokens": run.max_input_tokens,
        "max_output_tokens": run.max_output_tokens,
        "max_duration_seconds": run.max_duration_seconds,
        "capability_ceiling": "propose",
        "bound_head_commit": run.bound_head_commit,
        "bound_head_tree": run.bound_head_tree,
        "context_policy_sha256": run.context_policy_sha256,
    });
    if let Some(appointment_id) = &run.role_appointment_id {
        value["role_appointment_id"] = serde_json::json!(appointment_id);
    }
    digest_json(&value)
}

fn advisory_report_digest(report_id: &str, request: &AdvisoryRoleReportRequest) -> Result<String> {
    digest_json(&serde_json::json!({
        "report_id": report_id,
        "run_id": request.run_id,
        "source_kind": request.source_kind.as_str(),
        "disposition": request.disposition.as_str(),
        "predicted_task_outcome": request.predicted_task_outcome.as_str(),
        "summary": request.summary.trim(),
        "public_rationale": request.public_rationale.trim(),
        "recommended_next_action": request.recommended_next_action,
        "uncertainties": request.uncertainties,
        "addressed_criteria": request.addressed_criteria,
        "reported_input_tokens": request.reported_input_tokens,
        "reported_output_tokens": request.reported_output_tokens,
        "reported_duration_ms": request.reported_duration_ms,
    }))
}

fn advisory_report_event_payload(request: &AdvisoryRoleReportRequest) -> Result<serde_json::Value> {
    let request_sha256 = digest_json(&serde_json::json!({
        "run_id": request.run_id,
        "source_kind": request.source_kind.as_str(),
        "disposition": request.disposition.as_str(),
        "predicted_task_outcome": request.predicted_task_outcome.as_str(),
        "summary": request.summary.trim(),
        "public_rationale": request.public_rationale.trim(),
        "recommended_next_action": request.recommended_next_action,
        "uncertainties": request.uncertainties,
        "addressed_criteria": request.addressed_criteria,
        "reported_input_tokens": request.reported_input_tokens,
        "reported_output_tokens": request.reported_output_tokens,
        "reported_duration_ms": request.reported_duration_ms,
        "claims_task_complete": request.claims_task_complete,
    }))?;
    Ok(serde_json::json!({
        "run_id": request.run_id,
        "sealed_report_request_sha256": request_sha256,
    }))
}

fn advisory_report_digest_from_value(report: &AdvisoryRoleReport) -> Result<String> {
    digest_json(&serde_json::json!({
        "report_id": report.report_id,
        "run_id": report.run_id,
        "source_kind": report.source_kind.as_str(),
        "disposition": report.disposition.as_str(),
        "predicted_task_outcome": report.predicted_task_outcome.as_str(),
        "summary": report.summary,
        "public_rationale": report.public_rationale,
        "recommended_next_action": report.recommended_next_action,
        "uncertainties": report.uncertainties,
        "addressed_criteria": report.addressed_criteria,
        "reported_input_tokens": report.reported_input_tokens,
        "reported_output_tokens": report.reported_output_tokens,
        "reported_duration_ms": report.reported_duration_ms,
    }))
}

#[allow(clippy::too_many_arguments)]
fn shadow_evaluation_digest(
    evaluation_id: &str,
    run_id: &str,
    actual_task_outcome: &ActualTaskOutcome,
    prediction_match: Option<bool>,
    acceptance_criterion_count: u64,
    addressed_criterion_count: u64,
    verified_criterion_count: u64,
    git_stale: bool,
    evidence_eligible: bool,
    created_at_unix_ms: i64,
) -> Result<String> {
    digest_json(&serde_json::json!({
        "evaluation_id": evaluation_id,
        "run_id": run_id,
        "actual_task_outcome": actual_task_outcome.as_str(),
        "prediction_match": prediction_match,
        "acceptance_criterion_count": acceptance_criterion_count,
        "addressed_criterion_count": addressed_criterion_count,
        "verified_criterion_count": verified_criterion_count,
        "git_stale": git_stale,
        "evidence_eligible": evidence_eligible,
        "created_at_unix_ms": created_at_unix_ms,
    }))
}

fn shadow_evaluation_digest_from_value(value: &ShadowEvaluation) -> Result<String> {
    shadow_evaluation_digest(
        &value.evaluation_id,
        &value.run_id,
        &value.actual_task_outcome,
        value.prediction_match,
        value.acceptance_criterion_count,
        value.addressed_criterion_count,
        value.verified_criterion_count,
        value.git_stale,
        value.evidence_eligible,
        value.created_at_unix_ms,
    )
}

fn validate_report_budget(
    run: &OrchestrationRun,
    request: &AdvisoryRoleReportRequest,
) -> Result<()> {
    if request
        .reported_input_tokens
        .is_some_and(|value| value > run.max_input_tokens)
        || request
            .reported_output_tokens
            .is_some_and(|value| value > run.max_output_tokens)
        || request
            .reported_duration_ms
            .is_some_and(|value| value > run.max_duration_seconds.saturating_mul(1_000))
    {
        return Err(Error::Conflict(
            "reported role usage exceeds the immutable run budget".to_owned(),
        ));
    }
    Ok(())
}

fn orchestration_git_stale(
    connection: &Connection,
    project_id: &str,
    run: &OrchestrationRun,
) -> Result<bool> {
    let current = connection
        .query_row(
            "SELECT head_commit, head_tree, dirty FROM git_snapshots
             WHERE project_id = ?1 ORDER BY sequence DESC LIMIT 1",
            [project_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            },
        )
        .optional()?;
    Ok(current.is_none_or(|(commit, tree, dirty)| {
        dirty
            || commit.as_deref() != Some(run.bound_head_commit.as_str())
            || tree.as_deref() != Some(run.bound_head_tree.as_str())
    }))
}

fn orchestration_run_select() -> &'static str {
    "SELECT sequence, run_id, session_id, task_id, git_snapshot_id,
            context_exposure_id, mode, role_kind, role_id, objective, model,
            reasoning_effort, max_input_tokens, max_output_tokens,
            max_duration_seconds, status, bound_head_commit, bound_head_tree,
            context_policy_sha256, run_sha256, created_at_unix_ms,
            sealed_at_unix_ms, evaluated_at_unix_ms, role_appointment_id"
}

fn orchestration_run_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<OrchestrationRun> {
    Ok(OrchestrationRun {
        sequence: row.get(0)?,
        run_id: row.get(1)?,
        session_id: row.get(2)?,
        task_id: row.get(3)?,
        git_snapshot_id: row.get(4)?,
        context_exposure_id: row.get(5)?,
        mode: parse_advisory_mode(row.get(6)?)?,
        role_kind: parse_advisory_role_kind(row.get(7)?)?,
        role_id: row.get(8)?,
        objective: row.get(9)?,
        model: row.get(10)?,
        reasoning_effort: row.get(11)?,
        max_input_tokens: row.get(12)?,
        max_output_tokens: row.get(13)?,
        max_duration_seconds: row.get(14)?,
        capability_ceiling: CapabilityMaturity::Propose,
        status: parse_orchestration_status(row.get(15)?)?,
        bound_head_commit: row.get(16)?,
        bound_head_tree: row.get(17)?,
        context_policy_sha256: row.get(18)?,
        run_sha256: row.get(19)?,
        created_at_unix_ms: row.get(20)?,
        sealed_at_unix_ms: row.get(21)?,
        evaluated_at_unix_ms: row.get(22)?,
        role_appointment_id: row.get(23)?,
        advisory: true,
        executable: false,
    })
}

fn load_orchestration_run_any(
    connection: &Connection,
    run_id: &str,
) -> Result<(String, OrchestrationRun)> {
    connection
        .query_row(
            &format!(
                "SELECT project_id, {} FROM orchestration_runs WHERE run_id = ?1",
                orchestration_run_select().trim_start_matches("SELECT ")
            ),
            [run_id],
            |row| {
                let project_id = row.get::<_, String>(0)?;
                let run = OrchestrationRun {
                    sequence: row.get(1)?,
                    run_id: row.get(2)?,
                    session_id: row.get(3)?,
                    task_id: row.get(4)?,
                    git_snapshot_id: row.get(5)?,
                    context_exposure_id: row.get(6)?,
                    mode: parse_advisory_mode(row.get(7)?)?,
                    role_kind: parse_advisory_role_kind(row.get(8)?)?,
                    role_id: row.get(9)?,
                    objective: row.get(10)?,
                    model: row.get(11)?,
                    reasoning_effort: row.get(12)?,
                    max_input_tokens: row.get(13)?,
                    max_output_tokens: row.get(14)?,
                    max_duration_seconds: row.get(15)?,
                    capability_ceiling: CapabilityMaturity::Propose,
                    status: parse_orchestration_status(row.get(16)?)?,
                    bound_head_commit: row.get(17)?,
                    bound_head_tree: row.get(18)?,
                    context_policy_sha256: row.get(19)?,
                    run_sha256: row.get(20)?,
                    created_at_unix_ms: row.get(21)?,
                    sealed_at_unix_ms: row.get(22)?,
                    evaluated_at_unix_ms: row.get(23)?,
                    role_appointment_id: row.get(24)?,
                    advisory: true,
                    executable: false,
                };
                Ok((project_id, run))
            },
        )
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("orchestration run {run_id}")))
}

fn advisory_report_select() -> &'static str {
    "SELECT sequence, report_id, run_id, source_kind, disposition,
            predicted_task_outcome, summary, public_rationale,
            recommended_next_action, uncertainties_json, addressed_criteria_json,
            reported_input_tokens, reported_output_tokens, reported_duration_ms,
            report_sha256, created_at_unix_ms"
}

fn advisory_report_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AdvisoryRoleReport> {
    let uncertainties = row.get::<_, String>(9)?;
    let criteria = row.get::<_, String>(10)?;
    Ok(AdvisoryRoleReport {
        sequence: row.get(0)?,
        report_id: row.get(1)?,
        run_id: row.get(2)?,
        source_kind: parse_advisory_source_kind(row.get(3)?)?,
        disposition: parse_advisory_disposition(row.get(4)?)?,
        predicted_task_outcome: parse_predicted_task_outcome(row.get(5)?)?,
        summary: row.get(6)?,
        public_rationale: row.get(7)?,
        recommended_next_action: row.get(8)?,
        uncertainties: serde_json::from_str(&uncertainties).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                uncertainties.len(),
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        addressed_criteria: serde_json::from_str(&criteria).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                criteria.len(),
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        reported_input_tokens: row.get(11)?,
        reported_output_tokens: row.get(12)?,
        reported_duration_ms: row.get(13)?,
        report_sha256: row.get(14)?,
        created_at_unix_ms: row.get(15)?,
        evidence_grade: EvidenceGrade::ModelOnly,
    })
}

fn load_advisory_report(
    connection: &Connection,
    run_id: &str,
) -> Result<Option<AdvisoryRoleReport>> {
    Ok(connection
        .query_row(
            &format!(
                "{} FROM advisory_role_reports WHERE run_id = ?1",
                advisory_report_select()
            ),
            [run_id],
            advisory_report_from_row,
        )
        .optional()?)
}

fn shadow_evaluation_select() -> &'static str {
    "SELECT sequence, evaluation_id, run_id, actual_task_outcome,
            prediction_match, acceptance_criterion_count, addressed_criterion_count,
            verified_criterion_count, git_stale, evidence_eligible,
            evaluation_sha256, created_at_unix_ms"
}

fn shadow_evaluation_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ShadowEvaluation> {
    Ok(ShadowEvaluation {
        sequence: row.get(0)?,
        evaluation_id: row.get(1)?,
        run_id: row.get(2)?,
        actual_task_outcome: parse_actual_task_outcome(row.get(3)?)?,
        prediction_match: row.get(4)?,
        acceptance_criterion_count: row.get(5)?,
        addressed_criterion_count: row.get(6)?,
        verified_criterion_count: row.get(7)?,
        git_stale: row.get(8)?,
        evidence_eligible: row.get(9)?,
        evaluation_sha256: row.get(10)?,
        created_at_unix_ms: row.get(11)?,
    })
}

fn load_shadow_evaluation(
    connection: &Connection,
    run_id: &str,
) -> Result<Option<ShadowEvaluation>> {
    Ok(connection
        .query_row(
            &format!(
                "{} FROM shadow_evaluations WHERE run_id = ?1",
                shadow_evaluation_select()
            ),
            [run_id],
            shadow_evaluation_from_row,
        )
        .optional()?)
}

fn load_orchestration_view(
    connection: &Connection,
    project_id: &str,
    run_id: &str,
) -> Result<OrchestrationRunView> {
    let run = connection
        .query_row(
            &format!(
                "{} FROM orchestration_runs WHERE project_id = ?1 AND run_id = ?2",
                orchestration_run_select()
            ),
            params![project_id, run_id],
            orchestration_run_from_row,
        )
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("orchestration run {run_id}")))?;
    if orchestration_run_digest_from_value(&run)? != run.run_sha256 {
        return Err(Error::Conflict(
            "orchestration run digest mismatch".to_owned(),
        ));
    }
    let stored_report = load_advisory_report(connection, run_id)?;
    if let Some(report) = &stored_report
        && advisory_report_digest_from_value(report)? != report.report_sha256
    {
        return Err(Error::Conflict(
            "advisory report digest mismatch".to_owned(),
        ));
    }
    let evaluation = load_shadow_evaluation(connection, run_id)?;
    if let Some(value) = &evaluation
        && shadow_evaluation_digest_from_value(value)? != value.evaluation_sha256
    {
        return Err(Error::Conflict(
            "shadow evaluation digest mismatch".to_owned(),
        ));
    }
    let report_revealed = run.mode == AdvisoryMode::VisibleAdvisory
        || run.status == OrchestrationRunStatus::Evaluated;
    Ok(OrchestrationRunView {
        run,
        report: report_revealed.then_some(stored_report).flatten(),
        report_revealed,
        evaluation,
        authority_notice: "Hermes coordination and role reports are advisory data. They do not invoke models, execute tools, grant permission, prove utility, or complete tasks."
            .to_owned(),
    })
}

fn load_all_orchestration_runs(connection: &Connection) -> Result<Vec<OrchestrationRun>> {
    let mut statement = connection.prepare(&format!(
        "{} FROM orchestration_runs ORDER BY sequence ASC",
        orchestration_run_select()
    ))?;
    Ok(statement
        .query_map([], orchestration_run_from_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

fn load_all_advisory_reports(connection: &Connection) -> Result<Vec<AdvisoryRoleReport>> {
    let mut statement = connection.prepare(&format!(
        "{} FROM advisory_role_reports ORDER BY sequence ASC",
        advisory_report_select()
    ))?;
    Ok(statement
        .query_map([], advisory_report_from_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

fn load_all_shadow_evaluations(connection: &Connection) -> Result<Vec<ShadowEvaluation>> {
    let mut statement = connection.prepare(&format!(
        "{} FROM shadow_evaluations ORDER BY sequence ASC",
        shadow_evaluation_select()
    ))?;
    Ok(statement
        .query_map([], shadow_evaluation_from_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

fn experiment_campaign_digest(value: &ExperimentCampaign) -> Result<String> {
    digest_json(&serde_json::json!({
        "campaign_id": value.campaign_id, "title": value.title, "problem": value.problem,
        "target_user": value.target_user, "desired_outcome": value.desired_outcome,
        "hypothesis": value.hypothesis, "git_snapshot_id": value.git_snapshot_id,
        "bound_base_commit": value.bound_base_commit, "bound_base_tree": value.bound_base_tree,
        "max_variants": value.max_variants, "max_token_budget": value.max_token_budget,
        "created_at_unix_ms": value.created_at_unix_ms,
    }))
}

fn experiment_variant_digest(campaign_id: &str, value: &ExperimentVariant) -> Result<String> {
    digest_json(&serde_json::json!({
        "campaign_id": campaign_id, "variant_id": value.variant_id, "name": value.name,
        "diversity_axis": value.diversity_axis.as_str(), "approach": value.approach,
        "approach_sha256": value.approach_sha256, "git_snapshot_id": value.git_snapshot_id,
        "head_commit": value.head_commit, "head_tree": value.head_tree,
        "parent_variant_ids": value.parent_variant_ids,
        "created_at_unix_ms": value.created_at_unix_ms,
    }))
}

fn load_experiment_campaign(
    connection: &Connection,
    project_id: &str,
    campaign_id: &str,
) -> Result<ExperimentCampaign> {
    connection
        .query_row(
            "SELECT sequence, campaign_id, title, problem, target_user, desired_outcome,
                hypothesis, git_snapshot_id, bound_base_commit, bound_base_tree,
                max_variants, max_token_budget, campaign_sha256, created_at_unix_ms
         FROM experiment_campaigns WHERE project_id = ?1 AND campaign_id = ?2",
            params![project_id, campaign_id],
            |row| {
                Ok(ExperimentCampaign {
                    sequence: row.get(0)?,
                    campaign_id: row.get(1)?,
                    title: row.get(2)?,
                    problem: row.get(3)?,
                    target_user: row.get(4)?,
                    desired_outcome: row.get(5)?,
                    hypothesis: row.get(6)?,
                    git_snapshot_id: row.get(7)?,
                    bound_base_commit: row.get(8)?,
                    bound_base_tree: row.get(9)?,
                    max_variants: row.get(10)?,
                    max_token_budget: row.get(11)?,
                    campaign_sha256: row.get(12)?,
                    created_at_unix_ms: row.get(13)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("experiment campaign {campaign_id}")))
}

fn require_variant_owner(
    connection: &Connection,
    campaign_id: &str,
    variant_id: &str,
) -> Result<()> {
    let belongs = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM experiment_variants WHERE campaign_id = ?1 AND variant_id = ?2)",
        params![campaign_id, variant_id], |row| row.get::<_, bool>(0),
    )?;
    if belongs {
        Ok(())
    } else {
        Err(Error::NotFound(format!("variant {variant_id}")))
    }
}

fn parse_experiment_criterion_kind(value: String) -> rusqlite::Result<ExperimentCriterionKind> {
    match value.as_str() {
        "hard_gate" => Ok(ExperimentCriterionKind::HardGate),
        "pareto_dimension" => Ok(ExperimentCriterionKind::ParetoDimension),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_experiment_comparison(value: String) -> rusqlite::Result<ExperimentComparison> {
    match value.as_str() {
        "must_pass" => Ok(ExperimentComparison::MustPass),
        "minimize" => Ok(ExperimentComparison::Minimize),
        "maximize" => Ok(ExperimentComparison::Maximize),
        "gte" => Ok(ExperimentComparison::Gte),
        "lte" => Ok(ExperimentComparison::Lte),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_experiment_diversity_axis(value: String) -> rusqlite::Result<ExperimentDiversityAxis> {
    match value.as_str() {
        "product_assumption" => Ok(ExperimentDiversityAxis::ProductAssumption),
        "ux" => Ok(ExperimentDiversityAxis::Ux),
        "architecture" => Ok(ExperimentDiversityAxis::Architecture),
        "data_model" => Ok(ExperimentDiversityAxis::DataModel),
        "automation" => Ok(ExperimentDiversityAxis::Automation),
        "cost_safety" => Ok(ExperimentDiversityAxis::CostSafety),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_experiment_decision_kind(value: String) -> rusqlite::Result<ExperimentDecisionKind> {
    match value.as_str() {
        "advance" => Ok(ExperimentDecisionKind::Advance),
        "eliminate" => Ok(ExperimentDecisionKind::Eliminate),
        "synthesize" => Ok(ExperimentDecisionKind::Synthesize),
        "abandon" => Ok(ExperimentDecisionKind::Abandon),
        "select" => Ok(ExperimentDecisionKind::Select),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn load_experiment_portfolio(
    connection: &Connection,
    project_id: &str,
    campaign_id: &str,
    limit: usize,
) -> Result<ExperimentPortfolio> {
    let campaign = load_experiment_campaign(connection, project_id, campaign_id)?;
    if experiment_campaign_digest(&campaign)? != campaign.campaign_sha256 {
        return Err(Error::Conflict(
            "experiment campaign digest mismatch".to_owned(),
        ));
    }
    let criteria = {
        let mut statement = connection.prepare(
            "SELECT sequence, criterion_id, contract_revision, name, kind, comparison,
                    threshold, unit, criterion_sha256
             FROM experiment_criteria WHERE campaign_id = ?1 ORDER BY sequence ASC LIMIT ?2",
        )?;
        statement
            .query_map(params![campaign_id, limit as u64], |row| {
                Ok(ExperimentCriterion {
                    sequence: row.get(0)?,
                    criterion_id: row.get(1)?,
                    contract_revision: row.get(2)?,
                    name: row.get(3)?,
                    kind: parse_experiment_criterion_kind(row.get(4)?)?,
                    comparison: parse_experiment_comparison(row.get(5)?)?,
                    threshold: row.get(6)?,
                    unit: row.get(7)?,
                    criterion_sha256: row.get(8)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?
    };
    for criterion in &criteria {
        let observed = digest_json(&serde_json::json!({
            "campaign_id": campaign_id,
            "contract_revision": criterion.contract_revision,
            "name": criterion.name,
            "kind": criterion.kind.as_str(),
            "comparison": criterion.comparison.as_str(),
            "threshold": criterion.threshold,
            "unit": criterion.unit,
        }))?;
        if observed != criterion.criterion_sha256 {
            return Err(Error::Conflict(format!(
                "criterion {} digest mismatch",
                criterion.criterion_id
            )));
        }
    }
    let variants = {
        let mut statement = connection.prepare(
            "SELECT sequence, variant_id, name, diversity_axis, approach, approach_sha256,
                    git_snapshot_id, head_commit, head_tree, parent_variant_ids_json,
                    variant_sha256, created_at_unix_ms
             FROM experiment_variants WHERE campaign_id = ?1 ORDER BY sequence ASC LIMIT ?2",
        )?;
        let rows = statement.query_map(params![campaign_id, limit as u64], |row| {
            Ok((
                row.get::<_, u64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, i64>(11)?,
            ))
        })?;
        rows.map(|row| {
            let row = row?;
            Ok(ExperimentVariant {
                sequence: row.0,
                variant_id: row.1,
                name: row.2,
                diversity_axis: parse_experiment_diversity_axis(row.3)?,
                approach: row.4,
                approach_sha256: row.5,
                git_snapshot_id: row.6,
                head_commit: row.7,
                head_tree: row.8,
                parent_variant_ids: serde_json::from_str(&row.9)?,
                variant_sha256: row.10,
                created_at_unix_ms: row.11,
            })
        })
        .collect::<Result<Vec<_>>>()?
    };
    for variant in &variants {
        if experiment_variant_digest(campaign_id, variant)? != variant.variant_sha256 {
            return Err(Error::Conflict(format!(
                "variant {} digest mismatch",
                variant.variant_id
            )));
        }
    }
    let measurements = {
        let mut statement = connection.prepare(
            "SELECT sequence, measurement_id, variant_id, criterion_id, value,
                    evidence_id, claim_id, measurement_sha256, created_at_unix_ms
             FROM experiment_measurements WHERE campaign_id = ?1 ORDER BY sequence ASC LIMIT ?2",
        )?;
        let rows = statement.query_map(params![campaign_id, limit as u64], |row| {
            Ok((
                row.get::<_, u64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, i64>(8)?,
            ))
        })?;
        rows.map(|row| {
            let row = row?;
            let qualified = validate_deliberation_evidence(
                connection,
                project_id,
                row.5.as_deref(),
                row.6.as_deref(),
            )?;
            Ok(ExperimentMeasurement {
                sequence: row.0,
                measurement_id: row.1,
                variant_id: row.2,
                criterion_id: row.3,
                value: row.4,
                evidence_id: row.5,
                claim_id: row.6,
                evidence_qualified: qualified,
                measurement_sha256: row.7,
                created_at_unix_ms: row.8,
            })
        })
        .collect::<Result<Vec<_>>>()?
    };
    for measurement in &measurements {
        let observed = digest_json(&serde_json::json!({
            "campaign_id": campaign_id,
            "variant_id": measurement.variant_id,
            "criterion_id": measurement.criterion_id,
            "value": measurement.value,
            "evidence_id": measurement.evidence_id,
            "claim_id": measurement.claim_id,
            "created_at_unix_ms": measurement.created_at_unix_ms,
        }))?;
        if observed != measurement.measurement_sha256 {
            return Err(Error::Conflict(format!(
                "measurement {} digest mismatch",
                measurement.measurement_id
            )));
        }
    }
    let decisions = {
        let mut statement = connection.prepare(
            "SELECT sequence, decision_id, kind, variant_id, summary, deliberation_id,
                    deliberation_decision_id, qualified, decision_sha256, created_at_unix_ms
             FROM experiment_decisions WHERE campaign_id = ?1 ORDER BY sequence ASC LIMIT ?2",
        )?;
        statement
            .query_map(params![campaign_id, limit as u64], |row| {
                Ok(ExperimentDecision {
                    sequence: row.get(0)?,
                    decision_id: row.get(1)?,
                    kind: parse_experiment_decision_kind(row.get(2)?)?,
                    variant_id: row.get(3)?,
                    summary: row.get(4)?,
                    deliberation_id: row.get(5)?,
                    deliberation_decision_id: row.get(6)?,
                    qualified: row.get(7)?,
                    decision_sha256: row.get(8)?,
                    created_at_unix_ms: row.get(9)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?
    };
    for decision in &decisions {
        let observed = digest_json(&serde_json::json!({
            "campaign_id": campaign_id,
            "kind": decision.kind.as_str(),
            "variant_id": decision.variant_id,
            "summary": decision.summary,
            "deliberation_id": decision.deliberation_id,
            "deliberation_decision_id": decision.deliberation_decision_id,
            "qualified": decision.qualified,
            "created_at_unix_ms": decision.created_at_unix_ms,
        }))?;
        if observed != decision.decision_sha256 {
            return Err(Error::Conflict(format!(
                "experiment decision {} digest mismatch",
                decision.decision_id
            )));
        }
    }
    let (hard_gate_failed_variant_ids, evidence_incomplete_variant_ids, pareto_variant_ids) =
        classify_experiment_variants(&criteria, &variants, &measurements);
    let current = connection
        .query_row(
            "SELECT snapshot_id, head_commit, head_tree FROM git_snapshots
         WHERE project_id = ?1 ORDER BY sequence DESC LIMIT 1",
            [project_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()?;
    let integration_stale = current.as_ref().is_none_or(|(_, commit, tree)| {
        commit.as_deref() != Some(campaign.bound_base_commit.as_str())
            || tree.as_deref() != Some(campaign.bound_base_tree.as_str())
    });
    Ok(ExperimentPortfolio {
        budget_exhausted: variants.len() >= campaign.max_variants as usize,
        campaign, criteria, variants, measurements, decisions, pareto_variant_ids,
        hard_gate_failed_variant_ids, evidence_incomplete_variant_ids,
        current_git_snapshot_id: current.map(|value| value.0), integration_stale,
        executable: false, approval_proven: false,
        authority_notice: "Experiment results are advisory evidence. Aporic does not build variants, invoke providers, approve changes, or mutate Git.".to_owned(),
    })
}

fn classify_experiment_variants(
    criteria: &[ExperimentCriterion],
    variants: &[ExperimentVariant],
    measurements: &[ExperimentMeasurement],
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut failed = Vec::new();
    let mut incomplete = Vec::new();
    let mut qualified = Vec::new();
    for variant in variants {
        let mut missing = false;
        let mut hard_failed = false;
        for criterion in criteria
            .iter()
            .filter(|value| value.kind == ExperimentCriterionKind::HardGate)
        {
            let measurement = measurements.iter().find(|value| {
                value.variant_id == variant.variant_id
                    && value.criterion_id == criterion.criterion_id
            });
            let Some(measurement) = measurement else {
                missing = true;
                continue;
            };
            if !measurement.evidence_qualified {
                missing = true;
                continue;
            }
            let passes = match criterion.comparison {
                ExperimentComparison::MustPass => measurement.value == 1,
                ExperimentComparison::Gte => criterion
                    .threshold
                    .is_some_and(|value| measurement.value >= value),
                ExperimentComparison::Lte => criterion
                    .threshold
                    .is_some_and(|value| measurement.value <= value),
                _ => false,
            };
            hard_failed |= !passes;
        }
        if hard_failed {
            failed.push(variant.variant_id.clone());
        } else if missing {
            incomplete.push(variant.variant_id.clone());
        } else {
            qualified.push(variant.variant_id.clone());
        }
    }
    let pareto_criteria = criteria
        .iter()
        .filter(|value| value.kind == ExperimentCriterionKind::ParetoDimension)
        .collect::<Vec<_>>();
    let complete = qualified
        .into_iter()
        .filter(|variant_id| {
            pareto_criteria.iter().all(|criterion| {
                measurements.iter().any(|value| {
                    value.variant_id == *variant_id
                        && value.criterion_id == criterion.criterion_id
                        && value.evidence_qualified
                })
            })
        })
        .collect::<Vec<_>>();
    let pareto = complete
        .iter()
        .filter(|candidate| {
            !complete.iter().any(|other| {
                if candidate == &other {
                    return false;
                }
                let mut no_worse = true;
                let mut strictly_better = false;
                for criterion in &pareto_criteria {
                    let candidate_value = measurements
                        .iter()
                        .find(|value| {
                            value.variant_id == **candidate
                                && value.criterion_id == criterion.criterion_id
                        })
                        .map(|value| value.value)
                        .unwrap_or_default();
                    let other_value = measurements
                        .iter()
                        .find(|value| {
                            value.variant_id == **other
                                && value.criterion_id == criterion.criterion_id
                        })
                        .map(|value| value.value)
                        .unwrap_or_default();
                    match criterion.comparison {
                        ExperimentComparison::Minimize => {
                            no_worse &= other_value <= candidate_value;
                            strictly_better |= other_value < candidate_value;
                        }
                        ExperimentComparison::Maximize => {
                            no_worse &= other_value >= candidate_value;
                            strictly_better |= other_value > candidate_value;
                        }
                        _ => {}
                    }
                }
                no_worse && strictly_better
            })
        })
        .cloned()
        .collect();
    (failed, incomplete, pareto)
}

fn insert_imported_security_evidence(
    connection: &Connection,
    session_id: &str,
    locator: &str,
    summary: &str,
    content_sha256: &str,
    now: i64,
) -> Result<String> {
    require_bounded_public_text("security artifact locator", locator, 4_096)?;
    let evidence_id = Uuid::now_v7().to_string();
    connection.execute(
        "INSERT INTO evidence_artifacts(
            evidence_id, session_id, kind, grade, locator, summary,
            content_sha256, created_at_unix_ms
         ) VALUES (?1, ?2, 'external_source', 'direct', ?3, ?4, ?5, ?6)",
        params![
            evidence_id,
            session_id,
            locator,
            summary,
            content_sha256,
            now
        ],
    )?;
    Ok(evidence_id)
}

fn parse_security_coverage(value: String) -> rusqlite::Result<SecurityCoverage> {
    match value.as_str() {
        "complete" => Ok(SecurityCoverage::Complete),
        "partial" => Ok(SecurityCoverage::Partial),
        "unknown" => Ok(SecurityCoverage::Unknown),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn load_security_assessment(
    connection: &Connection,
    project_id: &str,
    assessment_id: &str,
) -> Result<SecurityAssessment> {
    let assessment = connection.query_row(
        "SELECT sequence, assessment_id, provider_capability_id, provider_version,
                source_scan_id, git_snapshot_id, target_commit, target_tree, coverage,
                reportable_critical, reportable_high, reportable_medium, reportable_low,
                manifest_evidence_id, findings_evidence_id, coverage_evidence_id,
                assessment_sha256, created_at_unix_ms
         FROM security_assessments WHERE project_id = ?1 AND assessment_id = ?2",
        params![project_id, assessment_id],
        |row| Ok(SecurityAssessment {
            sequence: row.get(0)?, assessment_id: row.get(1)?,
            provider_capability_id: row.get(2)?, provider_version: row.get(3)?,
            source_scan_id: row.get(4)?, git_snapshot_id: row.get(5)?,
            target_commit: row.get(6)?, target_tree: row.get(7)?,
            coverage: parse_security_coverage(row.get(8)?)?,
            reportable_critical: row.get(9)?, reportable_high: row.get(10)?,
            reportable_medium: row.get(11)?, reportable_low: row.get(12)?,
            manifest_evidence_id: row.get(13)?, findings_evidence_id: row.get(14)?,
            coverage_evidence_id: row.get(15)?, assessment_sha256: row.get(16)?,
            created_at_unix_ms: row.get(17)?, safety_proven: false,
            authority_notice: "Imported security artifacts prove only the observed bytes, declared coverage, and normalized counts. Zero findings never proves safety or approval.".to_owned(),
        }),
    ).optional()?.ok_or_else(|| Error::NotFound(format!("security assessment {assessment_id}")))?;
    let observed = digest_json(&serde_json::json!({
        "assessment_id": assessment.assessment_id,
        "provider_capability_id": assessment.provider_capability_id,
        "provider_version": assessment.provider_version,
        "source_scan_id": assessment.source_scan_id,
        "git_snapshot_id": assessment.git_snapshot_id,
        "target_commit": assessment.target_commit,
        "target_tree": assessment.target_tree,
        "coverage": assessment.coverage.as_str(),
        "reportable_critical": assessment.reportable_critical,
        "reportable_high": assessment.reportable_high,
        "reportable_medium": assessment.reportable_medium,
        "reportable_low": assessment.reportable_low,
        "manifest_evidence_id": assessment.manifest_evidence_id,
        "findings_evidence_id": assessment.findings_evidence_id,
        "coverage_evidence_id": assessment.coverage_evidence_id,
        "created_at_unix_ms": assessment.created_at_unix_ms,
    }))?;
    if observed != assessment.assessment_sha256 {
        return Err(Error::Conflict(format!(
            "security assessment {assessment_id} digest mismatch"
        )));
    }
    Ok(assessment)
}

fn require_bounded_public_text(name: &str, value: &str, max_bytes: usize) -> Result<()> {
    require_text(name, value)?;
    if value.len() > max_bytes {
        return Err(Error::Invalid(format!(
            "{name} exceeds the {max_bytes}-byte public-record limit"
        )));
    }
    if value.contains('\0') {
        return Err(Error::Invalid(format!("{name} contains a NUL byte")));
    }
    Ok(())
}

fn is_challenge_kind(kind: &DeliberationNodeKind) -> bool {
    matches!(
        kind,
        DeliberationNodeKind::Objection
            | DeliberationNodeKind::Counterexample
            | DeliberationNodeKind::Falsifier
            | DeliberationNodeKind::MaterialUnknown
    )
}

fn require_deliberation_owner(
    connection: &Connection,
    project_id: &str,
    deliberation_id: &str,
) -> Result<()> {
    let belongs = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM deliberations
         WHERE project_id = ?1 AND deliberation_id = ?2)",
        params![project_id, deliberation_id],
        |row| row.get::<_, bool>(0),
    )?;
    if !belongs {
        return Err(Error::NotFound(format!("deliberation {deliberation_id}")));
    }
    Ok(())
}

fn validate_deliberation_evidence(
    connection: &Connection,
    project_id: &str,
    evidence_id: Option<&str>,
    claim_id: Option<&str>,
) -> Result<bool> {
    let mut direct = false;
    if let Some(evidence_id) = evidence_id {
        require_text("evidence_id", evidence_id)?;
        let grade = connection
            .query_row(
                "SELECT evidence_artifacts.grade FROM evidence_artifacts
                 JOIN sessions ON sessions.session_id = evidence_artifacts.session_id
                 WHERE sessions.project_id = ?1 AND evidence_artifacts.evidence_id = ?2",
                params![project_id, evidence_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("evidence {evidence_id}")))?;
        direct |= grade == "direct";
    }
    if let Some(claim_id) = claim_id {
        require_text("claim_id", claim_id)?;
        let status = connection
            .query_row(
                "SELECT claims.status FROM claims
                 JOIN sessions ON sessions.session_id = claims.session_id
                 WHERE sessions.project_id = ?1 AND claims.claim_id = ?2",
                params![project_id, claim_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("claim {claim_id}")))?;
        direct |= matches!(status.as_str(), "observed" | "verified");
    }
    Ok(direct)
}

fn insert_deliberation_node(
    connection: &Connection,
    deliberation_id: &str,
    node: &DeliberationNode,
) -> Result<()> {
    connection.execute(
        "INSERT INTO deliberation_nodes(
            node_id, deliberation_id, kind, statement, material,
            evidence_id, claim_id, node_sha256, created_at_unix_ms
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            node.node_id,
            deliberation_id,
            node.kind.as_str(),
            node.statement,
            node.material,
            node.evidence_id,
            node.claim_id,
            node.node_sha256,
            node.created_at_unix_ms,
        ],
    )?;
    Ok(())
}

fn load_deliberation_graph(
    connection: &Connection,
    project_id: &str,
    deliberation_id: &str,
) -> Result<DeliberationGraph> {
    load_deliberation_graph_page(connection, project_id, deliberation_id, 0, 0, 0, 200)
}

fn load_deliberation_graph_page(
    connection: &Connection,
    project_id: &str,
    deliberation_id: &str,
    node_after_sequence: u64,
    edge_after_sequence: u64,
    decision_after_sequence: u64,
    limit: usize,
) -> Result<DeliberationGraph> {
    let deliberation = connection
        .query_row(
            &format!(
                "{} FROM deliberations WHERE project_id = ?1 AND deliberation_id = ?2",
                deliberation_select()
            ),
            params![project_id, deliberation_id],
            deliberation_from_row,
        )
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("deliberation {deliberation_id}")))?;
    let mut nodes = {
        let mut statement = connection.prepare(&format!(
            "{} FROM deliberation_nodes WHERE deliberation_id = ?1 ORDER BY sequence ASC",
            deliberation_node_select()
        ))?;
        statement
            .query_map([deliberation_id], deliberation_node_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?
    };
    let mut edges = {
        let mut statement = connection.prepare(&format!(
            "{} FROM deliberation_edges WHERE deliberation_id = ?1 ORDER BY sequence ASC",
            deliberation_edge_select()
        ))?;
        statement
            .query_map([deliberation_id], deliberation_edge_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?
    };
    let mut decisions = {
        let mut statement = connection.prepare(&format!(
            "{} FROM deliberation_decisions WHERE deliberation_id = ?1 ORDER BY sequence ASC",
            deliberation_decision_select()
        ))?;
        statement
            .query_map([deliberation_id], deliberation_decision_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?
    };
    let current = connection
        .query_row(
            "SELECT snapshot_id, head_commit, head_tree, dirty FROM git_snapshots
             WHERE project_id = ?1 ORDER BY sequence DESC LIMIT 1",
            [project_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, bool>(3)?,
                ))
            },
        )
        .optional()?;
    let stale = current.as_ref().is_some_and(|(_, head, tree, dirty)| {
        *dirty || *head != deliberation.bound_head_commit || *tree != deliberation.bound_head_tree
    });
    let mut open_material_node_ids = nodes
        .iter()
        .filter(|node| node.material && is_challenge_kind(&node.kind))
        .filter(|node| {
            !edges.iter().any(|edge| {
                if edge.target_node_id != node.node_id {
                    return false;
                }
                if node.kind == DeliberationNodeKind::MaterialUnknown {
                    edge.kind == DeliberationEdgeKind::Undercut
                        && nodes.iter().any(|source| {
                            source.node_id == edge.source_node_id
                                && (source.evidence_id.is_some() || source.claim_id.is_some())
                        })
                } else {
                    matches!(
                        edge.kind,
                        DeliberationEdgeKind::Undercut | DeliberationEdgeKind::Revision
                    )
                }
            })
        })
        .map(|node| node.node_id.clone())
        .collect::<Vec<_>>();
    let total_open_material_issue_count = open_material_node_ids.len() as u64;
    let open_material_issues_truncated = open_material_node_ids.len() > 200;
    open_material_node_ids.truncate(200);
    let total_node_count = nodes.len() as u64;
    let total_edge_count = edges.len() as u64;
    let total_decision_count = decisions.len() as u64;
    nodes.retain(|value| value.sequence > node_after_sequence);
    edges.retain(|value| value.sequence > edge_after_sequence);
    decisions.retain(|value| value.sequence > decision_after_sequence);
    let next_node_after_sequence = (nodes.len() > limit).then(|| nodes[limit - 1].sequence);
    let next_edge_after_sequence = (edges.len() > limit).then(|| edges[limit - 1].sequence);
    let next_decision_after_sequence =
        (decisions.len() > limit).then(|| decisions[limit - 1].sequence);
    let truncated = next_node_after_sequence.is_some()
        || next_edge_after_sequence.is_some()
        || next_decision_after_sequence.is_some();
    nodes.truncate(limit);
    edges.truncate(limit);
    decisions.truncate(limit);
    Ok(DeliberationGraph {
        deliberation,
        nodes,
        edges,
        decisions,
        total_node_count,
        total_edge_count,
        total_decision_count,
        truncated,
        next_node_after_sequence,
        next_edge_after_sequence,
        next_decision_after_sequence,
        current_git_snapshot_id: current.map(|(id, _, _, _)| id),
        stale,
        open_material_node_ids,
        total_open_material_issue_count,
        open_material_issues_truncated,
        provisional_only: true,
        approval_proven: false,
        authority_notice: "Deliberation records public arguments and provisional decisions. It stores no hidden chain-of-thought, proves no approval, grants no permission, and never gates host tools."
            .to_owned(),
    })
}

fn deliberation_select() -> &'static str {
    "SELECT deliberations.sequence, deliberations.deliberation_id, deliberations.title,
            deliberations.question, deliberations.git_snapshot_id,
            deliberations.bound_head_commit, deliberations.bound_head_tree,
            deliberations.deliberation_sha256, deliberations.created_at_unix_ms"
}

fn deliberation_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Deliberation> {
    Ok(Deliberation {
        sequence: row.get(0)?,
        deliberation_id: row.get(1)?,
        title: row.get(2)?,
        question: row.get(3)?,
        git_snapshot_id: row.get(4)?,
        bound_head_commit: row.get(5)?,
        bound_head_tree: row.get(6)?,
        deliberation_sha256: row.get(7)?,
        created_at_unix_ms: row.get(8)?,
    })
}

fn deliberation_node_select() -> &'static str {
    "SELECT deliberation_nodes.sequence, deliberation_nodes.node_id,
            deliberation_nodes.deliberation_id, deliberation_nodes.kind,
            deliberation_nodes.statement, deliberation_nodes.material,
            deliberation_nodes.evidence_id, deliberation_nodes.claim_id,
            deliberation_nodes.node_sha256, deliberation_nodes.created_at_unix_ms"
}

fn deliberation_node_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DeliberationNode> {
    Ok(DeliberationNode {
        sequence: row.get(0)?,
        node_id: row.get(1)?,
        deliberation_id: row.get(2)?,
        kind: parse_deliberation_node_kind(row.get(3)?)?,
        statement: row.get(4)?,
        material: row.get(5)?,
        evidence_id: row.get(6)?,
        claim_id: row.get(7)?,
        node_sha256: row.get(8)?,
        created_at_unix_ms: row.get(9)?,
    })
}

fn deliberation_edge_select() -> &'static str {
    "SELECT deliberation_edges.sequence, deliberation_edges.edge_id,
            deliberation_edges.deliberation_id, deliberation_edges.source_node_id,
            deliberation_edges.target_node_id, deliberation_edges.kind,
            deliberation_edges.edge_sha256, deliberation_edges.created_at_unix_ms"
}

fn deliberation_edge_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DeliberationEdge> {
    Ok(DeliberationEdge {
        sequence: row.get(0)?,
        edge_id: row.get(1)?,
        deliberation_id: row.get(2)?,
        source_node_id: row.get(3)?,
        target_node_id: row.get(4)?,
        kind: parse_deliberation_edge_kind(row.get(5)?)?,
        edge_sha256: row.get(6)?,
        created_at_unix_ms: row.get(7)?,
    })
}

fn deliberation_decision_select() -> &'static str {
    "SELECT deliberation_decisions.sequence, deliberation_decisions.decision_id,
            deliberation_decisions.deliberation_id, deliberation_decisions.proposal_node_id,
            deliberation_decisions.summary, deliberation_decisions.open_material_issues,
            deliberation_decisions.decision_sha256,
            deliberation_decisions.created_at_unix_ms"
}

fn deliberation_decision_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<DeliberationDecision> {
    Ok(DeliberationDecision {
        sequence: row.get(0)?,
        decision_id: row.get(1)?,
        deliberation_id: row.get(2)?,
        proposal_node_id: row.get(3)?,
        summary: row.get(4)?,
        open_material_issues: row.get(5)?,
        decision_sha256: row.get(6)?,
        created_at_unix_ms: row.get(7)?,
    })
}

fn load_project_deliberations(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<Deliberation>> {
    let mut statement = connection.prepare(&format!(
        "{} FROM deliberations WHERE project_id = ?1 ORDER BY sequence ASC",
        deliberation_select()
    ))?;
    Ok(statement
        .query_map([project_id], deliberation_from_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

fn load_project_deliberation_nodes(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<DeliberationNode>> {
    let mut statement = connection.prepare(&format!(
        "{} FROM deliberation_nodes JOIN deliberations USING(deliberation_id)
         WHERE deliberations.project_id = ?1 ORDER BY deliberation_nodes.sequence ASC",
        deliberation_node_select()
    ))?;
    Ok(statement
        .query_map([project_id], deliberation_node_from_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

fn load_project_deliberation_edges(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<DeliberationEdge>> {
    let mut statement = connection.prepare(&format!(
        "{} FROM deliberation_edges JOIN deliberations USING(deliberation_id)
         WHERE deliberations.project_id = ?1 ORDER BY deliberation_edges.sequence ASC",
        deliberation_edge_select()
    ))?;
    Ok(statement
        .query_map([project_id], deliberation_edge_from_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

fn load_project_deliberation_decisions(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<DeliberationDecision>> {
    let mut statement = connection.prepare(&format!(
        "{} FROM deliberation_decisions JOIN deliberations USING(deliberation_id)
         WHERE deliberations.project_id = ?1 ORDER BY deliberation_decisions.sequence ASC",
        deliberation_decision_select()
    ))?;
    Ok(statement
        .query_map([project_id], deliberation_decision_from_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

fn load_all_deliberations(connection: &Connection) -> Result<Vec<Deliberation>> {
    let mut statement = connection.prepare(&format!(
        "{} FROM deliberations ORDER BY sequence ASC",
        deliberation_select()
    ))?;
    Ok(statement
        .query_map([], deliberation_from_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

fn load_all_deliberation_nodes(connection: &Connection) -> Result<Vec<(String, DeliberationNode)>> {
    load_all_with_parent(
        connection,
        deliberation_node_select(),
        "deliberation_nodes",
        deliberation_node_from_row,
    )
}

fn load_all_deliberation_edges(connection: &Connection) -> Result<Vec<(String, DeliberationEdge)>> {
    load_all_with_parent(
        connection,
        deliberation_edge_select(),
        "deliberation_edges",
        deliberation_edge_from_row,
    )
}

fn load_all_deliberation_decisions(
    connection: &Connection,
) -> Result<Vec<(String, DeliberationDecision)>> {
    load_all_with_parent(
        connection,
        deliberation_decision_select(),
        "deliberation_decisions",
        deliberation_decision_from_row,
    )
}

fn load_all_with_parent<T, F>(
    connection: &Connection,
    select: &str,
    table: &str,
    mapper: F,
) -> Result<Vec<(String, T)>>
where
    T: Serialize,
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    let mut statement =
        connection.prepare(&format!("{select} FROM {table} ORDER BY sequence ASC"))?;
    let values = statement
        .query_map([], mapper)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    // Every child domain record carries its deliberation id. Serialize once to
    // keep this generic helper private and bounded to doctor-time auditing.
    values
        .into_iter()
        .map(|value| {
            let json = serde_json::to_value(&value)?;
            let parent = json["deliberation_id"]
                .as_str()
                .ok_or_else(|| {
                    Error::Invalid("missing deliberation_id in audit record".to_owned())
                })?
                .to_owned();
            Ok((parent, value))
        })
        .collect()
}

fn deliberation_digest(value: &Deliberation) -> Result<String> {
    digest_json(&serde_json::json!({
        "deliberation_id": value.deliberation_id,
        "title": value.title,
        "question": value.question,
        "git_snapshot_id": value.git_snapshot_id,
        "bound_head_commit": value.bound_head_commit,
        "bound_head_tree": value.bound_head_tree,
        "created_at_unix_ms": value.created_at_unix_ms,
    }))
}

fn deliberation_node_digest(deliberation_id: &str, value: &DeliberationNode) -> Result<String> {
    digest_json(&serde_json::json!({
        "node_id": value.node_id,
        "deliberation_id": deliberation_id,
        "kind": value.kind.as_str(),
        "statement": value.statement,
        "material": value.material,
        "evidence_id": value.evidence_id,
        "claim_id": value.claim_id,
        "created_at_unix_ms": value.created_at_unix_ms,
    }))
}

fn deliberation_edge_digest(deliberation_id: &str, value: &DeliberationEdge) -> Result<String> {
    digest_json(&serde_json::json!({
        "edge_id": value.edge_id,
        "deliberation_id": deliberation_id,
        "source_node_id": value.source_node_id,
        "target_node_id": value.target_node_id,
        "kind": value.kind.as_str(),
        "created_at_unix_ms": value.created_at_unix_ms,
    }))
}

fn deliberation_decision_digest(
    deliberation_id: &str,
    value: &DeliberationDecision,
) -> Result<String> {
    digest_json(&serde_json::json!({
        "decision_id": value.decision_id,
        "deliberation_id": deliberation_id,
        "proposal_node_id": value.proposal_node_id,
        "summary": value.summary,
        "open_material_issues": value.open_material_issues,
        "created_at_unix_ms": value.created_at_unix_ms,
    }))
}

fn digest_json(value: &serde_json::Value) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}

fn invalid_enum(value: String, kind: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        format!("invalid {kind} {value}").into(),
    )
}

fn parse_advisory_mode(value: String) -> rusqlite::Result<AdvisoryMode> {
    match value.as_str() {
        "blind_shadow" => Ok(AdvisoryMode::BlindShadow),
        "visible_advisory" => Ok(AdvisoryMode::VisibleAdvisory),
        _ => Err(invalid_enum(value, "advisory mode")),
    }
}

fn parse_advisory_role_kind(value: String) -> rusqlite::Result<AdvisoryRoleKind> {
    match value.as_str() {
        "steward" => Ok(AdvisoryRoleKind::Steward),
        "worker" => Ok(AdvisoryRoleKind::Worker),
        _ => Err(invalid_enum(value, "advisory role kind")),
    }
}

fn parse_orchestration_status(value: String) -> rusqlite::Result<OrchestrationRunStatus> {
    match value.as_str() {
        "awaiting_report" => Ok(OrchestrationRunStatus::AwaitingReport),
        "sealed" => Ok(OrchestrationRunStatus::Sealed),
        "evaluated" => Ok(OrchestrationRunStatus::Evaluated),
        _ => Err(invalid_enum(value, "orchestration run status")),
    }
}

fn parse_advisory_disposition(value: String) -> rusqlite::Result<AdvisoryDisposition> {
    match value.as_str() {
        "abstain" => Ok(AdvisoryDisposition::Abstain),
        "recommend" => Ok(AdvisoryDisposition::Recommend),
        "flag_risk" => Ok(AdvisoryDisposition::FlagRisk),
        "propose_work" => Ok(AdvisoryDisposition::ProposeWork),
        _ => Err(invalid_enum(value, "advisory disposition")),
    }
}

fn parse_predicted_task_outcome(value: String) -> rusqlite::Result<PredictedTaskOutcome> {
    match value.as_str() {
        "completion" => Ok(PredictedTaskOutcome::Completion),
        "non_completion" => Ok(PredictedTaskOutcome::NonCompletion),
        "uncertain" => Ok(PredictedTaskOutcome::Uncertain),
        _ => Err(invalid_enum(value, "predicted task outcome")),
    }
}

fn parse_advisory_source_kind(value: String) -> rusqlite::Result<AdvisorySourceKind> {
    match value.as_str() {
        "host_reported" => Ok(AdvisorySourceKind::HostReported),
        "deterministic_simulator" => Ok(AdvisorySourceKind::DeterministicSimulator),
        _ => Err(invalid_enum(value, "advisory source kind")),
    }
}

fn parse_actual_task_outcome(value: String) -> rusqlite::Result<ActualTaskOutcome> {
    match value.as_str() {
        "completed" => Ok(ActualTaskOutcome::Completed),
        "cancelled" => Ok(ActualTaskOutcome::Cancelled),
        _ => Err(invalid_enum(value, "actual task outcome")),
    }
}

fn parse_deliberation_node_kind(value: String) -> rusqlite::Result<DeliberationNodeKind> {
    match value.as_str() {
        "question" => Ok(DeliberationNodeKind::Question),
        "premise" => Ok(DeliberationNodeKind::Premise),
        "claim" => Ok(DeliberationNodeKind::Claim),
        "objection" => Ok(DeliberationNodeKind::Objection),
        "counterexample" => Ok(DeliberationNodeKind::Counterexample),
        "falsifier" => Ok(DeliberationNodeKind::Falsifier),
        "value_constraint" => Ok(DeliberationNodeKind::ValueConstraint),
        "material_unknown" => Ok(DeliberationNodeKind::MaterialUnknown),
        "proposal" => Ok(DeliberationNodeKind::Proposal),
        "revision" => Ok(DeliberationNodeKind::Revision),
        _ => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            format!("invalid deliberation node kind {value}").into(),
        )),
    }
}

fn parse_deliberation_edge_kind(value: String) -> rusqlite::Result<DeliberationEdgeKind> {
    match value.as_str() {
        "support" => Ok(DeliberationEdgeKind::Support),
        "attack" => Ok(DeliberationEdgeKind::Attack),
        "undercut" => Ok(DeliberationEdgeKind::Undercut),
        "dependency" => Ok(DeliberationEdgeKind::Dependency),
        "falsification" => Ok(DeliberationEdgeKind::Falsification),
        "revision" => Ok(DeliberationEdgeKind::Revision),
        _ => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            format!("invalid deliberation edge kind {value}").into(),
        )),
    }
}

pub(crate) fn canonical_workspace(raw: &str) -> Result<String> {
    require_text("workspace", raw)?;
    let path = fs::canonicalize(raw)?;
    if !path.is_dir() {
        return Err(Error::Invalid("workspace must be a directory".to_owned()));
    }
    Ok(path.to_string_lossy().into_owned())
}

fn project_id_for_workspace(connection: &Connection, workspace: &str) -> Result<Option<String>> {
    Ok(connection
        .query_row(
            "SELECT project_id FROM projects WHERE workspace = ?1",
            [workspace],
            |row| row.get::<_, String>(0),
        )
        .optional()?)
}

fn installation_secret(connection: &Connection, now: i64) -> Result<String> {
    if let Some(secret) = connection
        .query_row(
            "SELECT value FROM installation_secrets WHERE name = 'exposure_hmac_v1'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?
    {
        return Ok(secret);
    }
    let candidate = Uuid::now_v7().to_string();
    connection.execute(
        "INSERT OR IGNORE INTO installation_secrets(name, value, created_at_unix_ms)
         VALUES ('exposure_hmac_v1', ?1, ?2)",
        params![candidate, now],
    )?;
    Ok(connection.query_row(
        "SELECT value FROM installation_secrets WHERE name = 'exposure_hmac_v1'",
        [],
        |row| row.get::<_, String>(0),
    )?)
}

fn hmac_bytes(secret: &str, bytes: &[u8]) -> Result<String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| Error::Invalid("invalid HMAC key".to_owned()))?;
    mac.update(bytes);
    Ok(format!("{:x}", mac.finalize().into_bytes()))
}

fn hmac_text(secret: &str, value: Option<&str>) -> Result<Option<String>> {
    value
        .map(|value| hmac_bytes(secret, value.as_bytes()))
        .transpose()
}

fn hmac_json(secret: &str, value: Option<&serde_json::Value>) -> Result<(Option<String>, u64)> {
    let Some(value) = value else {
        return Ok((None, 0));
    };
    let bytes = serde_json::to_vec(value)?;
    Ok((
        Some(hmac_bytes(secret, &bytes)?),
        u64::try_from(bytes.len()).unwrap_or(u64::MAX),
    ))
}

fn otel_hex_id(value: &str, len: usize) -> String {
    let digest = format!("{:x}", Sha256::digest(value.as_bytes()));
    digest[..len.min(digest.len())].to_owned()
}

fn runtime_authority_notice() -> &'static str {
    "Observed capabilities and shadow decisions are telemetry, not permissions, grants, denials, or proof that every host action was observed."
}

fn git_authority_notice() -> &'static str {
    "Git observations and governance findings are local, advisory evidence. They do not prove remote freshness, signer trust, review approval, code safety, or permission to merge."
}

fn git_snapshot_select() -> &'static str {
    "SELECT git_snapshots.sequence, git_snapshots.snapshot_id,
            git_snapshots.repository_root, git_snapshots.head_commit,
            git_snapshots.head_tree, git_snapshots.branch, git_snapshots.detached,
            git_snapshots.upstream_ref, git_snapshots.base_ref,
            git_snapshots.base_commit, git_snapshots.merge_base,
            git_snapshots.ahead_count, git_snapshots.behind_count,
            git_snapshots.remote_state_fresh, git_snapshots.dirty,
            git_snapshots.staged_count, git_snapshots.unstaged_count,
            git_snapshots.untracked_count, git_snapshots.local_branch_count,
            git_snapshots.head_parent_count, git_snapshots.head_has_signature,
            git_snapshots.successful_receipt_bound, git_snapshots.paths_truncated,
            git_snapshots.changed_paths_json, git_snapshots.worktrees_json,
            git_snapshots.remotes_json, git_snapshots.findings_json,
            git_snapshots.policy_version, git_snapshots.snapshot_sha256,
            git_snapshots.captured_at_unix_ms"
}

fn git_snapshot_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GitSnapshot> {
    Ok(GitSnapshot {
        sequence: row.get(0)?,
        snapshot_id: row.get(1)?,
        repository_root: row.get(2)?,
        head_commit: row.get(3)?,
        head_tree: row.get(4)?,
        branch: row.get(5)?,
        detached: row.get(6)?,
        upstream_ref: row.get(7)?,
        base_ref: row.get(8)?,
        base_commit: row.get(9)?,
        merge_base: row.get(10)?,
        ahead_count: row.get(11)?,
        behind_count: row.get(12)?,
        remote_state_fresh: row.get(13)?,
        dirty: row.get(14)?,
        staged_count: row.get(15)?,
        unstaged_count: row.get(16)?,
        untracked_count: row.get(17)?,
        local_branch_count: row.get(18)?,
        head_parent_count: row.get(19)?,
        head_has_signature: row.get(20)?,
        successful_receipt_bound: row.get(21)?,
        paths_truncated: row.get(22)?,
        changed_paths: json_column(row, 23)?,
        worktrees: json_column(row, 24)?,
        remotes: json_column(row, 25)?,
        findings: json_column(row, 26)?,
        policy_version: row.get(27)?,
        snapshot_sha256: row.get(28)?,
        captured_at_unix_ms: row.get(29)?,
        approval_proven: false,
        authority_notice: git_authority_notice().to_owned(),
    })
}

fn git_snapshot_digest(snapshot: &GitSnapshot) -> Result<String> {
    let value = serde_json::json!({
        "snapshot_id": snapshot.snapshot_id,
        "repository_root": snapshot.repository_root,
        "head_commit": snapshot.head_commit,
        "head_tree": snapshot.head_tree,
        "branch": snapshot.branch,
        "detached": snapshot.detached,
        "upstream_ref": snapshot.upstream_ref,
        "base_ref": snapshot.base_ref,
        "base_commit": snapshot.base_commit,
        "merge_base": snapshot.merge_base,
        "ahead_count": snapshot.ahead_count,
        "behind_count": snapshot.behind_count,
        "remote_state_fresh": snapshot.remote_state_fresh,
        "dirty": snapshot.dirty,
        "staged_count": snapshot.staged_count,
        "unstaged_count": snapshot.unstaged_count,
        "untracked_count": snapshot.untracked_count,
        "local_branch_count": snapshot.local_branch_count,
        "head_parent_count": snapshot.head_parent_count,
        "head_has_signature": snapshot.head_has_signature,
        "successful_receipt_bound": snapshot.successful_receipt_bound,
        "paths_truncated": snapshot.paths_truncated,
        "changed_paths": snapshot.changed_paths,
        "worktrees": snapshot.worktrees,
        "remotes": snapshot.remotes,
        "findings": snapshot.findings,
        "policy_version": snapshot.policy_version,
        "captured_at_unix_ms": snapshot.captured_at_unix_ms,
    });
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(&value)?)))
}

fn token_usage_select() -> &'static str {
    "SELECT token_usage_receipts.sequence, token_usage_receipts.receipt_id,
            token_usage_receipts.session_id, token_usage_receipts.scope_kind,
            token_usage_receipts.scope_id, token_usage_receipts.model,
            token_usage_receipts.source_kind, token_usage_receipts.input_tokens,
            token_usage_receipts.output_tokens, token_usage_receipts.cached_input_tokens,
            token_usage_receipts.reasoning_tokens, token_usage_receipts.context_bytes,
            token_usage_receipts.outcome, token_usage_receipts.verification_ref,
            token_usage_receipts.receipt_sha256, token_usage_receipts.recorded_at_unix_ms"
}

fn token_usage_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TokenUsageReceipt> {
    Ok(TokenUsageReceipt {
        sequence: row.get(0)?,
        receipt_id: row.get(1)?,
        session_id: row.get(2)?,
        scope_kind: row.get(3)?,
        scope_id: row.get(4)?,
        model: row.get(5)?,
        source_kind: parse_token_count_source(row.get(6)?)?,
        input_tokens: row.get(7)?,
        output_tokens: row.get(8)?,
        cached_input_tokens: row.get(9)?,
        reasoning_tokens: row.get(10)?,
        context_bytes: row.get(11)?,
        outcome: parse_usage_outcome(row.get(12)?)?,
        verification_ref: row.get(13)?,
        receipt_sha256: row.get(14)?,
        recorded_at_unix_ms: row.get(15)?,
    })
}

fn token_usage_digest(receipt: &TokenUsageReceipt) -> Result<String> {
    let value = serde_json::json!({
        "receipt_id": receipt.receipt_id,
        "session_id": receipt.session_id,
        "scope_kind": receipt.scope_kind,
        "scope_id": receipt.scope_id,
        "model": receipt.model,
        "source_kind": receipt.source_kind,
        "input_tokens": receipt.input_tokens,
        "output_tokens": receipt.output_tokens,
        "cached_input_tokens": receipt.cached_input_tokens,
        "reasoning_tokens": receipt.reasoning_tokens,
        "context_bytes": receipt.context_bytes,
        "outcome": receipt.outcome,
        "verification_ref": receipt.verification_ref,
        "recorded_at_unix_ms": receipt.recorded_at_unix_ms,
    });
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(&value)?)))
}

fn empty_token_efficiency_report() -> TokenEfficiencyReport {
    TokenEfficiencyReport {
        receipt_count: 0,
        host_reported_receipts: 0,
        local_tokenizer_receipts: 0,
        estimated_receipts: 0,
        unknown_receipts: 0,
        measured_input_tokens: 0,
        measured_output_tokens: 0,
        measured_reasoning_tokens: 0,
        cached_input_tokens: 0,
        estimated_input_token_upper_bound: 0,
        verified_successes: 0,
        verified_failures: 0,
        measured_verified_successes: 0,
        measured_tokens_per_verified_success: None,
        measurement_complete: false,
        warnings: vec!["no_usage_receipts".to_owned()],
    }
}

fn build_token_efficiency_report(receipts: &[TokenUsageReceipt]) -> TokenEfficiencyReport {
    if receipts.is_empty() {
        return empty_token_efficiency_report();
    }
    let mut report = TokenEfficiencyReport {
        receipt_count: receipts.len() as u64,
        host_reported_receipts: 0,
        local_tokenizer_receipts: 0,
        estimated_receipts: 0,
        unknown_receipts: 0,
        measured_input_tokens: 0,
        measured_output_tokens: 0,
        measured_reasoning_tokens: 0,
        cached_input_tokens: 0,
        estimated_input_token_upper_bound: 0,
        verified_successes: 0,
        verified_failures: 0,
        measured_verified_successes: 0,
        measured_tokens_per_verified_success: None,
        measurement_complete: false,
        warnings: Vec::new(),
    };
    let mut unverified = 0u64;
    for receipt in receipts {
        let measured = matches!(
            receipt.source_kind,
            TokenCountSource::HostReported | TokenCountSource::LocalTokenizer
        );
        match receipt.source_kind {
            TokenCountSource::HostReported => report.host_reported_receipts += 1,
            TokenCountSource::LocalTokenizer => report.local_tokenizer_receipts += 1,
            TokenCountSource::ConservativeByteUpperBound => {
                report.estimated_receipts += 1;
                report.estimated_input_token_upper_bound = report
                    .estimated_input_token_upper_bound
                    .saturating_add(receipt.context_bytes.unwrap_or(0));
            }
            TokenCountSource::Unknown => report.unknown_receipts += 1,
        }
        if measured {
            report.measured_input_tokens = report
                .measured_input_tokens
                .saturating_add(receipt.input_tokens.unwrap_or(0));
            report.measured_output_tokens = report
                .measured_output_tokens
                .saturating_add(receipt.output_tokens.unwrap_or(0));
            report.measured_reasoning_tokens = report
                .measured_reasoning_tokens
                .saturating_add(receipt.reasoning_tokens.unwrap_or(0));
            report.cached_input_tokens = report
                .cached_input_tokens
                .saturating_add(receipt.cached_input_tokens.unwrap_or(0));
        }
        match receipt.outcome {
            UsageOutcome::VerifiedSuccess => {
                report.verified_successes += 1;
                if measured {
                    report.measured_verified_successes += 1;
                }
            }
            UsageOutcome::VerifiedFailure => report.verified_failures += 1,
            UsageOutcome::Unverified => unverified += 1,
        }
    }
    let measured_total = report
        .measured_input_tokens
        .saturating_add(report.measured_output_tokens)
        .saturating_add(report.measured_reasoning_tokens);
    if report.measured_verified_successes > 0 {
        report.measured_tokens_per_verified_success =
            Some(measured_total as f64 / report.measured_verified_successes as f64);
    }
    if report.estimated_receipts > 0 {
        report
            .warnings
            .push("estimated_tokens_are_not_provider_counts".to_owned());
    }
    if report.unknown_receipts > 0 {
        report
            .warnings
            .push("unknown_token_provenance_present".to_owned());
    }
    if unverified > 0 {
        report
            .warnings
            .push("unverified_outcomes_present".to_owned());
    }
    if report.verified_successes != report.measured_verified_successes {
        report
            .warnings
            .push("some_verified_successes_lack_measured_tokens".to_owned());
    }
    report.measurement_complete = report.unknown_receipts == 0
        && report.estimated_receipts == 0
        && unverified == 0
        && report.verified_successes == report.measured_verified_successes;
    report
}

fn runtime_event_select() -> &'static str {
    "SELECT runtime_events.sequence, runtime_events.event_id, runtime_events.exposure_id,
            runtime_events.host_provider, runtime_events.event_kind,
            runtime_events.host_session_hmac, runtime_events.host_turn_hmac,
            runtime_events.host_tool_call_hmac, runtime_events.tool_name,
            runtime_events.capability_class, runtime_events.outcome_status,
            runtime_events.input_hmac, runtime_events.input_bytes,
            runtime_events.output_hmac, runtime_events.output_bytes,
            runtime_events.latency_ms, runtime_events.hook_schema_version,
            runtime_events.shadow_disposition, runtime_events.shadow_reasons_json,
            runtime_events.duplicate_of_event_id, runtime_events.received_at_unix_ms"
}

fn runtime_event_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RuntimeEvent> {
    let reasons = row.get::<_, String>(18)?;
    Ok(RuntimeEvent {
        sequence: row.get(0)?,
        event_id: row.get(1)?,
        exposure_id: row.get(2)?,
        host_provider: row.get(3)?,
        event_kind: parse_runtime_event_kind(row.get(4)?)?,
        host_session_hmac: row.get(5)?,
        host_turn_hmac: row.get(6)?,
        host_tool_call_hmac: row.get(7)?,
        tool_name: row.get(8)?,
        capability_class: parse_capability_class(row.get(9)?)?,
        outcome_status: parse_runtime_outcome(row.get(10)?)?,
        input_hmac: row.get(11)?,
        input_bytes: row.get(12)?,
        output_hmac: row.get(13)?,
        output_bytes: row.get(14)?,
        latency_ms: row.get(15)?,
        hook_schema_version: row.get(16)?,
        shadow_disposition: parse_shadow_disposition(row.get(17)?)?,
        shadow_reasons: serde_json::from_str(&reasons).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                reasons.len(),
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        duplicate_of_event_id: row.get(19)?,
        received_at_unix_ms: row.get(20)?,
    })
}

fn load_capability_observations(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<CapabilityObservation>> {
    let mut statement = connection.prepare(
        "SELECT observation_id, host_provider, tool_name, capability_class,
                first_seen_at_unix_ms, last_seen_at_unix_ms, event_count,
                succeeded_count, failed_count
         FROM capability_observations WHERE project_id = ?1
         ORDER BY event_count DESC, host_provider ASC, tool_name ASC, capability_class ASC",
    )?;
    Ok(statement
        .query_map([project_id], |row| {
            Ok(CapabilityObservation {
                observation_id: row.get(0)?,
                host_provider: row.get(1)?,
                tool_name: row.get(2)?,
                capability_class: parse_capability_class(row.get(3)?)?,
                first_seen_at_unix_ms: row.get(4)?,
                last_seen_at_unix_ms: row.get(5)?,
                event_count: row.get(6)?,
                succeeded_count: row.get(7)?,
                failed_count: row.get(8)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

fn count_runtime(connection: &Connection, project_id: &str, predicate: &str) -> Result<u64> {
    Ok(connection.query_row(
        &format!("SELECT count(*) FROM runtime_events WHERE project_id = ?1 AND {predicate}"),
        [project_id],
        |row| row.get(0),
    )?)
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
            let (digest, _) =
                sha256_file_bounded(&path, MAX_EVIDENCE_FILE_BYTES, "workspace evidence file")?;
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

    if request.verifies_effect_id.is_some() {
        return Err(Error::Invalid(
            "new verification records are disabled; verifies_effect_id is not accepted".to_owned(),
        ));
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
        sandbox_profile: json_column(row, 11)?,
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
                    verification_specs.created_at_unix_ms,
                    verification_specs.sandbox_profile_json
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
                verification_specs.created_at_unix_ms,
                verification_specs.sandbox_profile_json
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
        sandbox_backend: row.get(6)?,
        sandbox_enforced: row.get(7)?,
        stdout_sha256: row.get(8)?,
        stdout_bytes: row.get(9)?,
        stderr_sha256: row.get(10)?,
        stderr_bytes: row.get(11)?,
        git_head_before: row.get(12)?,
        git_head_after: row.get(13)?,
        worktree_state_before_sha256: row.get(14)?,
        worktree_state_after_sha256: row.get(15)?,
        created_at_unix_ms: row.get(16)?,
    })
}

const RECEIPT_COLUMNS: &str = "execution_receipts.receipt_id, execution_receipts.run_id,
    execution_receipts.command_spec_sha256, execution_receipts.resolved_executable,
    execution_receipts.exit_code, execution_receipts.termination,
    execution_receipts.sandbox_backend, execution_receipts.sandbox_enforced,
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
            .try_fold(false, |overlap, left| {
                if overlap {
                    return Ok(true);
                }
                scopes.iter().try_fold(false, |inner_overlap, right| {
                    if inner_overlap {
                        Ok(true)
                    } else {
                        write_scopes_overlap(left, right)
                    }
                })
            })?
        {
            return Err(Error::Conflict(format!(
                "task write scope overlaps active lease for task {task_id}"
            )));
        }
    }
    Ok(())
}

fn normalize_write_scopes(scopes: &[String]) -> Result<Vec<String>> {
    let mut normalized = Vec::with_capacity(scopes.len());
    for scope in scopes {
        let scope = normalize_write_scope(scope)?;
        if normalized.contains(&scope) {
            return Err(Error::Invalid(format!(
                "write_scope contains duplicate normalized path {scope}"
            )));
        }
        normalized.push(scope);
    }
    Ok(normalized)
}

fn normalize_write_scope(raw: &str) -> Result<String> {
    require_text("write_scope", raw)?;
    if raw.starts_with('/')
        || raw.starts_with('\\')
        || raw.contains('\\')
        || raw.as_bytes().get(1) == Some(&b':')
    {
        return Err(Error::Invalid(
            "write_scope must use workspace-relative forward-slash paths".to_owned(),
        ));
    }
    let mut components = Vec::new();
    for component in raw.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                return Err(Error::Invalid(
                    "write_scope must not contain parent traversal".to_owned(),
                ));
            }
            value => {
                if !value.is_ascii()
                    || value.ends_with(['.', ' '])
                    || value
                        .bytes()
                        .any(|byte| byte.is_ascii_control() || b"<>:\"|?*~".contains(&byte))
                {
                    return Err(Error::Invalid(
                        "write_scope components must use portable ASCII names without trailing dots or spaces"
                            .to_owned(),
                    ));
                }
                let value = value.to_ascii_lowercase();
                let stem = value.split('.').next().unwrap_or_default();
                let reserved = matches!(stem, "con" | "prn" | "aux" | "nul")
                    || stem.strip_prefix("com").is_some_and(|suffix| {
                        matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
                    })
                    || stem.strip_prefix("lpt").is_some_and(|suffix| {
                        matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
                    });
                if reserved {
                    return Err(Error::Invalid(format!(
                        "write_scope contains a reserved portable component {value}"
                    )));
                }
                components.push(value);
            }
        }
    }
    Ok(if components.is_empty() {
        ".".to_owned()
    } else {
        components.join("/")
    })
}

fn write_scopes_overlap(left: &str, right: &str) -> Result<bool> {
    let left = normalize_write_scope(left)?;
    let right = normalize_write_scope(right)?;
    if left == "." || right == "." {
        return Ok(true);
    }
    Ok(left == right
        || left
            .strip_prefix(&right)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || right
            .strip_prefix(&left)
            .is_some_and(|suffix| suffix.starts_with('/')))
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
    objective: Option<&str>,
    focus_paths: &[String],
    max_bytes: u32,
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
                records.origin_channel, records.influence_class, records.created_at_unix_ms
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
    let candidate_record_limit = limit.max(200);
    let candidate_records = record_statement
        .query_map(params![project_id, candidate_record_limit], |row| {
            let kind = parse_record_kind(row.get::<_, String>(2)?)?;
            Ok(DurableRecord {
                record_id: row.get(0)?,
                session_id: row.get(1)?,
                kind,
                content: row.get(3)?,
                evidence: row.get(4)?,
                supersedes_record_id: row.get(5)?,
                verifies_effect_id: row.get(6)?,
                origin_channel: parse_origin_channel(row.get(7)?)?,
                influence_class: parse_influence_class(row.get(8)?)?,
                created_at_unix_ms: row.get(9)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let recent_records = candidate_records
        .iter()
        .take(usize::try_from(limit).unwrap_or(100))
        .cloned()
        .collect::<Vec<_>>();

    let mut candidates = candidate_records
        .iter()
        .map(|record| {
            let (priority, reason) = match record.kind {
                RecordKind::MaterialUnknown => (0, "unresolved_material_unknown"),
                RecordKind::Constraint => (1, "active_constraint"),
                RecordKind::Decision => (2, "active_decision"),
                RecordKind::Verification => (5, "verification_record"),
                _ => (7, "recent_record"),
            };
            crate::context::Candidate {
                item_type: "record".to_owned(),
                item_id: record.record_id.clone(),
                origin_channel: record.origin_channel.clone(),
                influence_class: record.influence_class.clone(),
                status: Some(record.kind.as_str().to_owned()),
                content: record.content.clone(),
                reason: reason.to_owned(),
                priority,
                created_at_unix_ms: record.created_at_unix_ms,
            }
        })
        .collect::<Vec<_>>();

    for task in list_project_tasks(connection, project_id, 200)?
        .into_iter()
        .filter(|task| matches!(task.status, TaskStatus::Queued | TaskStatus::Leased))
    {
        candidates.push(crate::context::Candidate {
            item_type: "task".to_owned(),
            item_id: task.task_id,
            origin_channel: OriginChannel::McpAgent,
            influence_class: InfluenceClass::HistoricalContext,
            status: Some(task.status.as_str().to_owned()),
            content: task.objective,
            reason: "active_task".to_owned(),
            priority: 3,
            created_at_unix_ms: task.updated_at_unix_ms,
        });
    }

    let claims = load_project_claims(connection, project_id)?;
    let superseded_claims = claims
        .iter()
        .filter_map(|claim| claim.supersedes_claim_id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    for claim in claims.into_iter().filter(|claim| {
        !superseded_claims.contains(&claim.claim_id)
            && ((claim.status == ClaimStatus::Unknown && claim.material)
                || matches!(claim.status, ClaimStatus::Observed | ClaimStatus::Verified))
    }) {
        let unknown = claim.status == ClaimStatus::Unknown;
        let origin_channel = if claim.receipt_ids.is_empty() {
            OriginChannel::McpAgent
        } else {
            OriginChannel::LocalRunner
        };
        candidates.push(crate::context::Candidate {
            item_type: "claim".to_owned(),
            item_id: claim.claim_id,
            origin_channel,
            influence_class: if unknown {
                InfluenceClass::HistoricalContext
            } else {
                InfluenceClass::VerifiedFact
            },
            status: Some(claim.status.as_str().to_owned()),
            content: claim.statement,
            reason: if unknown {
                "unresolved_material_unknown"
            } else {
                "directly_supported_claim"
            }
            .to_owned(),
            priority: if unknown { 0 } else { 4 },
            created_at_unix_ms: claim.created_at_unix_ms,
        });
    }

    for handoff in &recent_handoffs {
        candidates.push(crate::context::Candidate {
            item_type: "handoff".to_owned(),
            item_id: handoff.session_id.clone(),
            origin_channel: OriginChannel::McpAgent,
            influence_class: InfluenceClass::HistoricalContext,
            status: Some("handoff".to_owned()),
            content: format!("{} Next action: {}", handoff.summary, handoff.next_action),
            reason: "recent_handoff".to_owned(),
            priority: 6,
            created_at_unix_ms: handoff.closed_at_unix_ms,
        });
    }

    let max_items = u32::try_from(limit).unwrap_or(100).clamp(1, 100);
    let (selected_items, budget) =
        crate::context::select(candidates, objective, focus_paths, max_items, max_bytes);

    Ok(ContextCapsule {
        project_id: Some(project_id.to_owned()),
        workspace: workspace.to_owned(),
        active_sessions,
        recent_handoffs,
        recent_records,
        selected_items,
        budget,
        policy_sha256: crate::context::policy_sha256(),
        authority_notice: crate::context::AUTHORITY_NOTICE.to_owned(),
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

fn memory_query_terms(query: &str) -> Vec<String> {
    let mut terms = query
        .split(|character: char| {
            !(character.is_alphanumeric() || character == '_' || character == '-')
        })
        .map(str::to_lowercase)
        .filter(|term| term.chars().count() >= 2)
        .collect::<Vec<_>>();
    terms.sort();
    terms.dedup();
    terms.truncate(16);
    terms
}

fn memory_select() -> &'static str {
    "SELECT memory_items.memory_id, memory_items.source_kind,
            memory_items.source_id, memory_items.memory_class,
            memory_items.content, memory_items.origin_channel,
            memory_items.influence_class, memory_items.source_status,
            memory_items.lifecycle_state, memory_items.valid_from_unix_ms,
            memory_items.valid_until_unix_ms, memory_items.applicability_json,
            memory_items.created_at_unix_ms, memory_items.updated_at_unix_ms"
}

fn load_current_memory(
    connection: &Connection,
    project_id: &str,
    as_of: i64,
) -> Result<Vec<MemoryItem>> {
    let sql = format!(
        "{} FROM memory_items
         WHERE project_id = ?1 AND lifecycle_state IN ('active', 'superseded')
           AND valid_from_unix_ms <= ?2
           AND (valid_until_unix_ms IS NULL OR valid_until_unix_ms > ?2)
         ORDER BY CASE memory_class
                    WHEN 'gotcha' THEN 0 WHEN 'unknown' THEN 1 WHEN 'semantic' THEN 2
                    WHEN 'procedural' THEN 3 ELSE 4 END,
                  updated_at_unix_ms DESC, memory_id ASC LIMIT 500",
        memory_select()
    );
    let mut statement = connection.prepare(&sql)?;
    Ok(statement
        .query_map(params![project_id, as_of], memory_item_from_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

fn load_matching_memory(
    connection: &Connection,
    project_id: &str,
    as_of: i64,
    terms: &[String],
) -> Result<Vec<MemoryItem>> {
    let expression = terms
        .iter()
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" OR ");
    let sql = format!(
        "{} FROM memory_fts
         JOIN memory_items ON memory_items.memory_id = memory_fts.memory_id
         WHERE memory_fts MATCH ?1 AND memory_items.project_id = ?2
           AND memory_items.lifecycle_state IN ('active', 'superseded')
           AND memory_items.valid_from_unix_ms <= ?3
           AND (memory_items.valid_until_unix_ms IS NULL OR memory_items.valid_until_unix_ms > ?3)
         ORDER BY bm25(memory_fts),
                  CASE memory_items.memory_class
                    WHEN 'gotcha' THEN 0 WHEN 'unknown' THEN 1 WHEN 'semantic' THEN 2
                    WHEN 'procedural' THEN 3 ELSE 4 END,
                  memory_items.updated_at_unix_ms DESC, memory_items.memory_id ASC LIMIT 500",
        memory_select()
    );
    let mut statement = connection.prepare(&sql)?;
    Ok(statement
        .query_map(params![expression, project_id, as_of], memory_item_from_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

fn memory_item_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryItem> {
    let applicability_json = row.get::<_, String>(11)?;
    Ok(MemoryItem {
        memory_id: row.get(0)?,
        source_kind: row.get(1)?,
        source_id: row.get(2)?,
        memory_class: parse_memory_class(row.get(3)?)?,
        content: row.get(4)?,
        origin_channel: parse_origin_channel(row.get(5)?)?,
        influence_class: parse_influence_class(row.get(6)?)?,
        source_status: row.get(7)?,
        lifecycle_state: parse_memory_lifecycle(row.get(8)?)?,
        valid_from_unix_ms: row.get(9)?,
        valid_until_unix_ms: row.get(10)?,
        applicability: serde_json::from_str(&applicability_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                applicability_json.len(),
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        selection_reasons: Vec::new(),
        created_at_unix_ms: row.get(12)?,
        updated_at_unix_ms: row.get(13)?,
    })
}

fn parse_memory_class(value: String) -> rusqlite::Result<MemoryClass> {
    match value.as_str() {
        "episodic" => Ok(MemoryClass::Episodic),
        "semantic" => Ok(MemoryClass::Semantic),
        "procedural" => Ok(MemoryClass::Procedural),
        "gotcha" => Ok(MemoryClass::Gotcha),
        "unknown" => Ok(MemoryClass::Unknown),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_memory_lifecycle(value: String) -> rusqlite::Result<MemoryLifecycle> {
    match value.as_str() {
        "active" => Ok(MemoryLifecycle::Active),
        "superseded" => Ok(MemoryLifecycle::Superseded),
        "quarantined" => Ok(MemoryLifecycle::Quarantined),
        "tombstoned" => Ok(MemoryLifecycle::Tombstoned),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_runtime_event_kind(value: String) -> rusqlite::Result<RuntimeEventKind> {
    match value.as_str() {
        "session_start" => Ok(RuntimeEventKind::SessionStart),
        "user_prompt" => Ok(RuntimeEventKind::UserPrompt),
        "pre_tool" => Ok(RuntimeEventKind::PreTool),
        "post_tool" => Ok(RuntimeEventKind::PostTool),
        "tool_failure" => Ok(RuntimeEventKind::ToolFailure),
        "permission_request" => Ok(RuntimeEventKind::PermissionRequest),
        "stop" => Ok(RuntimeEventKind::Stop),
        "session_end" => Ok(RuntimeEventKind::SessionEnd),
        "unknown" => Ok(RuntimeEventKind::Unknown),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_capability_class(value: String) -> rusqlite::Result<CapabilityClass> {
    match value.as_str() {
        "read" => Ok(CapabilityClass::Read),
        "write" => Ok(CapabilityClass::Write),
        "execute" => Ok(CapabilityClass::Execute),
        "network" => Ok(CapabilityClass::Network),
        "external_mutation" => Ok(CapabilityClass::ExternalMutation),
        "delegation" => Ok(CapabilityClass::Delegation),
        "unknown" => Ok(CapabilityClass::Unknown),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_runtime_outcome(value: String) -> rusqlite::Result<RuntimeOutcomeStatus> {
    match value.as_str() {
        "proposed" => Ok(RuntimeOutcomeStatus::Proposed),
        "succeeded" => Ok(RuntimeOutcomeStatus::Succeeded),
        "failed" => Ok(RuntimeOutcomeStatus::Failed),
        "unknown" => Ok(RuntimeOutcomeStatus::Unknown),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_shadow_disposition(value: String) -> rusqlite::Result<ShadowDisposition> {
    match value.as_str() {
        "observe" => Ok(ShadowDisposition::Observe),
        "warn" => Ok(ShadowDisposition::Warn),
        "would_ask" => Ok(ShadowDisposition::WouldAsk),
        "would_deny" => Ok(ShadowDisposition::WouldDeny),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_origin_channel(value: String) -> rusqlite::Result<OriginChannel> {
    match value.as_str() {
        "legacy" => Ok(OriginChannel::Legacy),
        "mcp_agent" => Ok(OriginChannel::McpAgent),
        "local_runner" => Ok(OriginChannel::LocalRunner),
        "codex_hook" => Ok(OriginChannel::CodexHook),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_influence_class(value: String) -> rusqlite::Result<InfluenceClass> {
    match value.as_str() {
        "verified_fact" => Ok(InfluenceClass::VerifiedFact),
        "historical_context" => Ok(InfluenceClass::HistoricalContext),
        "untrusted_content" => Ok(InfluenceClass::UntrustedContent),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_token_count_source(value: String) -> rusqlite::Result<TokenCountSource> {
    match value.as_str() {
        "host_reported" => Ok(TokenCountSource::HostReported),
        "local_tokenizer" => Ok(TokenCountSource::LocalTokenizer),
        "conservative_byte_upper_bound" => Ok(TokenCountSource::ConservativeByteUpperBound),
        "unknown" => Ok(TokenCountSource::Unknown),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_usage_outcome(value: String) -> rusqlite::Result<UsageOutcome> {
    match value.as_str() {
        "unverified" => Ok(UsageOutcome::Unverified),
        "verified_success" => Ok(UsageOutcome::VerifiedSuccess),
        "verified_failure" => Ok(UsageOutcome::VerifiedFailure),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}
