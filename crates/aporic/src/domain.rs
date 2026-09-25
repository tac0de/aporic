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
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveSession {
    pub session_id: String,
    pub objective: String,
    pub opened_at_unix_ms: i64,
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
