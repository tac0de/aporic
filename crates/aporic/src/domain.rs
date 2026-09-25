use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecordKind {
    Decision,
    Constraint,
    TaskProgress,
    Observation,
    Effect,
    Verification,
    MaterialUnknown,
}

impl RecordKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Decision => "decision",
            Self::Constraint => "constraint",
            Self::TaskProgress => "task_progress",
            Self::Observation => "observation",
            Self::Effect => "effect",
            Self::Verification => "verification",
            Self::MaterialUnknown => "material_unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CloseDisposition {
    Completed,
    Handoff,
}

impl CloseDisposition {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Handoff => "handoff",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OpenRequest {
    pub workspace: String,
    pub objective: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecallRequest {
    pub workspace: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordRequest {
    pub session_id: String,
    pub kind: RecordKind,
    pub content: String,
    pub evidence: Option<String>,
    #[serde(default)]
    pub supersedes_record_id: Option<String>,
    #[serde(default)]
    pub verifies_effect_id: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReconcileRequest {
    pub workspace: String,
    pub stale_after_seconds: u64,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CloseRequest {
    pub session_id: String,
    pub disposition: CloseDisposition,
    pub summary: String,
    pub next_action: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableRecord {
    pub record_id: String,
    pub session_id: String,
    pub kind: RecordKind,
    pub content: String,
    pub evidence: Option<String>,
    pub supersedes_record_id: Option<String>,
    pub verifies_effect_id: Option<String>,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveSession {
    pub session_id: String,
    pub objective: String,
    pub opened_at_unix_ms: i64,
    pub last_activity_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbandonedSession {
    pub session_id: String,
    pub objective: String,
    pub last_activity_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Handoff {
    pub session_id: String,
    pub summary: String,
    pub next_action: String,
    pub closed_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextCapsule {
    pub project_id: Option<String>,
    pub workspace: String,
    pub active_sessions: Vec<ActiveSession>,
    pub recent_handoffs: Vec<Handoff>,
    pub recent_records: Vec<DurableRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenOutcome {
    pub session_id: String,
    pub project_id: String,
    pub kernel_sha256: String,
    pub context: ContextCapsule,
    pub duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordOutcome {
    pub record: DurableRecord,
    pub duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseOutcome {
    pub session_id: String,
    pub disposition: CloseDisposition,
    pub summary: String,
    pub next_action: Option<String>,
    pub duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconcileOutcome {
    pub project_id: Option<String>,
    pub abandoned_sessions: Vec<AbandonedSession>,
    pub duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HubStats {
    pub schema_version: u32,
    pub project_count: u64,
    pub open_session_count: u64,
    pub abandoned_session_count: u64,
    pub record_count: u64,
    pub event_count: u64,
    pub queued_task_count: u64,
    pub leased_task_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Leased,
    Completed,
    Cancelled,
}

impl TaskStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Leased => "leased",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordinatedTask {
    pub task_id: String,
    pub project_id: String,
    pub session_id: String,
    pub objective: String,
    pub acceptance_criteria: Vec<String>,
    pub write_scope: Vec<String>,
    pub depends_on: Vec<String>,
    pub status: TaskStatus,
    pub lease_owner: Option<String>,
    pub lease_expires_at_unix_ms: Option<i64>,
    pub outcome_summary: Option<String>,
    pub completion_evidence: Vec<CriterionEvidence>,
    pub created_at_unix_ms: i64,
    pub updated_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CriterionEvidence {
    pub criterion: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskCreateRequest {
    pub session_id: String,
    pub objective: String,
    pub acceptance_criteria: Vec<String>,
    #[serde(default)]
    pub write_scope: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskListRequest {
    pub workspace: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskClaimRequest {
    pub task_id: String,
    pub worker_id: String,
    pub lease_seconds: u64,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskCompleteRequest {
    pub task_id: String,
    pub worker_id: String,
    pub outcome_summary: String,
    pub criterion_evidence: Vec<CriterionEvidence>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskCancelRequest {
    pub task_id: String,
    pub reason: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskOutcome {
    pub task: CoordinatedTask,
    pub duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportSession {
    pub session_id: String,
    pub objective: String,
    pub status: String,
    pub opened_at_unix_ms: i64,
    pub last_activity_at_unix_ms: i64,
    pub abandoned: bool,
    pub closed_at_unix_ms: Option<i64>,
    pub summary: Option<String>,
    pub next_action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportEvent {
    pub sequence: u64,
    pub event_id: String,
    pub idempotency_key: String,
    pub stream_id: String,
    pub kind: String,
    pub payload: serde_json::Value,
    pub result: serde_json::Value,
    pub occurred_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectExport {
    pub format_version: u32,
    pub exported_at_unix_ms: i64,
    pub project_id: String,
    pub workspace: String,
    pub sessions: Vec<ExportSession>,
    pub records: Vec<DurableRecord>,
    pub tasks: Vec<CoordinatedTask>,
    pub events: Vec<ExportEvent>,
}
