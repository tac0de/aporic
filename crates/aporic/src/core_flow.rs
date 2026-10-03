//! Bounded, advisory lifecycle shortcuts. Legacy APIs and evidence gates remain intact.
use crate::domain::{
    CloseDisposition, CloseOutcome, ContextCapsule, DelegationChoice, RecordKind,
    SessionDelegationDecision,
};
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Reserve 1 KiB for ordinary JSON-RPC framing (request IDs remain host supplied).
pub const MAX_RESPONSE_BYTES: usize = 15_360;
pub const MAX_NOTES: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkShape {
    pub parallel_paths: u8,
    pub material_change: bool,
    pub worker: DelegationChoice,
    pub reviewer: DelegationChoice,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BeginRequest {
    pub workspace: String,
    pub objective: String,
    pub idempotency_key: String,
    /// Host assessment only; never dispatches agents or changes permissions.
    pub work_shape: WorkShape,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeginOutcome {
    pub session_id: String,
    pub project_id: String,
    pub context: ContextCapsule,
    pub delegation: SessionDelegationDecision,
    pub runtime: crate::runtime::RuntimeIdentity,
    pub open_repair_count: u64,
    pub repair_notice: String,
    pub recovery: Vec<RecoverySession>,
    pub omitted_recovery_sessions: u64,
    pub response_limit_bytes: usize,
    pub response_omitted_items: u32,
    pub duplicate: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverySession {
    pub session_id: String,
    pub status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FinishNote {
    pub kind: RecordKind,
    pub content: String,
    pub evidence: Option<String>,
    #[serde(default)]
    pub supersedes_record_id: Option<String>,
    #[serde(default)]
    pub verifies_effect_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FinishRequest {
    pub session_id: String,
    pub disposition: CloseDisposition,
    pub summary: String,
    pub next_action: Option<String>,
    #[serde(default)]
    pub notes: Vec<FinishNote>,
    pub idempotency_key: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishOutcome {
    pub close: CloseOutcome,
    pub record_ids: Vec<String>,
    pub duplicate: bool,
}

// Measure the actual text-content result wrapper and JSON escaping used by MCP.
pub fn response_bytes(value: &impl Serialize) -> Result<usize, serde_json::Error> {
    let text = serde_json::to_string(&serde_json::json!({"ok": true, "result": value}))?;
    Ok(serde_json::to_vec(
        &serde_json::json!({"content":[{"type":"text","text":text}],"isError":false}),
    )?
    .len())
}
