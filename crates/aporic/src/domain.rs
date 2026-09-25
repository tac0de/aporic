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
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub focus_paths: Vec<String>,
    #[serde(default)]
    pub max_bytes: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginChannel {
    Legacy,
    McpAgent,
    LocalRunner,
    CodexHook,
}

impl OriginChannel {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Legacy => "legacy",
            Self::McpAgent => "mcp_agent",
            Self::LocalRunner => "local_runner",
            Self::CodexHook => "codex_hook",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InfluenceClass {
    VerifiedFact,
    HistoricalContext,
    UntrustedContent,
}

impl InfluenceClass {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::VerifiedFact => "verified_fact",
            Self::HistoricalContext => "historical_context",
            Self::UntrustedContent => "untrusted_content",
        }
    }
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
    pub origin_channel: OriginChannel,
    pub influence_class: InfluenceClass,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResumeRequest {
    pub workspace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResumeStatus {
    Ready,
    Ambiguous,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeCandidate {
    pub source: String,
    pub source_id: String,
    pub objective: String,
    pub next_action: String,
    pub updated_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeBrief {
    pub status: ResumeStatus,
    pub candidates: Vec<ResumeCandidate>,
    pub selected: Option<ResumeCandidate>,
    pub reason: String,
    pub authority_notice: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleDefinition {
    pub role_id: String,
    pub version: u32,
    pub title: String,
    pub responsibility: String,
    pub output_contract: String,
    pub task_required: bool,
    pub advisory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RoleAppointmentCreateRequest {
    pub session_id: String,
    #[serde(default)]
    pub task_id: Option<String>,
    pub role_id: String,
    pub role_version: u32,
    pub assignee_id: String,
    #[serde(default)]
    pub model_hint: Option<String>,
    #[serde(default)]
    pub capability_refs: Vec<RoleCapabilityRef>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RoleCapabilityRef {
    pub capability_id: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RoleAppointmentRevokeRequest {
    pub appointment_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RoleAppointmentListRequest {
    pub workspace: String,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleAppointment {
    pub appointment_id: String,
    pub session_id: String,
    pub task_id: Option<String>,
    pub role_id: String,
    pub role_version: u32,
    pub assignee_id: String,
    pub model_hint: Option<String>,
    pub capability_refs: Vec<RoleCapabilityRef>,
    pub appointment_sha256: String,
    pub created_at_unix_ms: i64,
    pub revoked_at_unix_ms: Option<i64>,
    pub advisory: bool,
    pub grants_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleAppointmentOutcome {
    pub appointment: RoleAppointment,
    pub duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleAppointmentAudit {
    pub appointment_count: u64,
    pub invalid_count: u64,
    pub consistent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernmentDefinition {
    pub government_id: String,
    pub version: u32,
    pub title: String,
    pub authority_source: String,
    pub executive_role_id: String,
    pub offices: Vec<OfficeDefinition>,
    pub advisory: bool,
    pub grants_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficeDefinition {
    pub office_id: String,
    pub version: u32,
    pub title: String,
    pub head_title: String,
    pub responsibility: String,
    pub output_contract: String,
    pub head_role_id: String,
    pub cell_based: bool,
    pub advisory: bool,
    pub grants_authority: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OfficeAppointmentCreateRequest {
    pub session_id: String,
    pub office_id: String,
    pub office_version: u32,
    pub role_appointment_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OfficeAppointmentRevokeRequest {
    pub office_appointment_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GovernmentWorkspaceRequest {
    pub workspace: String,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficeAppointment {
    pub office_appointment_id: String,
    pub session_id: String,
    pub office_id: String,
    pub office_version: u32,
    pub role_appointment_id: String,
    pub assignee_id: String,
    pub appointment_sha256: String,
    pub created_at_unix_ms: i64,
    pub revoked_at_unix_ms: Option<i64>,
    pub advisory: bool,
    pub grants_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficeAppointmentOutcome {
    pub appointment: OfficeAppointment,
    pub duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProductCellDuty {
    ProductPlanning,
    PrototypeDelivery,
    UserResearch,
    TechnicalFeasibility,
    InteractionDesign,
    VisualDesign,
    MotionDesign,
}

impl ProductCellDuty {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::ProductPlanning => "product_planning",
            Self::PrototypeDelivery => "prototype_delivery",
            Self::UserResearch => "user_research",
            Self::TechnicalFeasibility => "technical_feasibility",
            Self::InteractionDesign => "interaction_design",
            Self::VisualDesign => "visual_design",
            Self::MotionDesign => "motion_design",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProductCellMemberRequest {
    pub role_appointment_id: String,
    pub duty: ProductCellDuty,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProductCellCreateRequest {
    pub session_id: String,
    pub task_id: String,
    pub office_appointment_id: String,
    pub title: String,
    pub problem_statement: String,
    pub hypothesis: String,
    pub success_measures: Vec<String>,
    pub members: Vec<ProductCellMemberRequest>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductCellMember {
    pub role_appointment_id: String,
    pub assignee_id: String,
    pub duty: ProductCellDuty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductCell {
    pub cell_id: String,
    pub session_id: String,
    pub task_id: String,
    pub office_appointment_id: String,
    pub title: String,
    pub problem_statement: String,
    pub hypothesis: String,
    pub success_measures: Vec<String>,
    pub members: Vec<ProductCellMember>,
    pub cell_sha256: String,
    pub created_at_unix_ms: i64,
    pub active: bool,
    pub advisory: bool,
    pub grants_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductCellOutcome {
    pub cell: ProductCell,
    pub duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernmentAudit {
    pub office_appointment_count: u64,
    pub product_cell_count: u64,
    pub invalid_count: u64,
    pub consistent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextCapsule {
    pub project_id: Option<String>,
    pub workspace: String,
    pub active_sessions: Vec<ActiveSession>,
    pub recent_handoffs: Vec<Handoff>,
    pub recent_records: Vec<DurableRecord>,
    pub selected_items: Vec<ContextItem>,
    pub budget: ContextBudget,
    pub policy_sha256: String,
    pub authority_notice: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextItem {
    pub item_type: String,
    pub item_id: String,
    pub origin_channel: OriginChannel,
    pub influence_class: InfluenceClass,
    pub status: Option<String>,
    pub content: String,
    pub selection_reasons: Vec<String>,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextBudget {
    pub max_items: u32,
    pub max_content_bytes: u32,
    pub used_content_bytes: u32,
    pub omitted_items: u32,
    pub candidate_items: u32,
    pub deduplicated_items: u32,
    pub oversized_items: u32,
    pub item_limit_items: u32,
    pub conservative_input_token_upper_bound: u32,
    pub token_estimate_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TokenCountSource {
    HostReported,
    LocalTokenizer,
    ConservativeByteUpperBound,
    Unknown,
}

impl TokenCountSource {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::HostReported => "host_reported",
            Self::LocalTokenizer => "local_tokenizer",
            Self::ConservativeByteUpperBound => "conservative_byte_upper_bound",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UsageOutcome {
    Unverified,
    VerifiedSuccess,
    VerifiedFailure,
}

impl UsageOutcome {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Unverified => "unverified",
            Self::VerifiedSuccess => "verified_success",
            Self::VerifiedFailure => "verified_failure",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TokenUsageRecordRequest {
    pub workspace: String,
    #[serde(default)]
    pub session_id: Option<String>,
    pub scope_kind: String,
    pub scope_id: String,
    #[serde(default)]
    pub model: Option<String>,
    pub source_kind: TokenCountSource,
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub cached_input_tokens: Option<u64>,
    #[serde(default)]
    pub reasoning_tokens: Option<u64>,
    #[serde(default)]
    pub context_bytes: Option<u64>,
    pub outcome: UsageOutcome,
    #[serde(default)]
    pub verification_ref: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TokenUsageListRequest {
    pub workspace: String,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TokenEfficiencyReportRequest {
    pub workspace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsageReceipt {
    pub sequence: u64,
    pub receipt_id: String,
    pub session_id: Option<String>,
    pub scope_kind: String,
    pub scope_id: String,
    pub model: Option<String>,
    pub source_kind: TokenCountSource,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
    pub context_bytes: Option<u64>,
    pub outcome: UsageOutcome,
    pub verification_ref: Option<String>,
    pub receipt_sha256: String,
    pub recorded_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenUsageOutcome {
    pub receipt: TokenUsageReceipt,
    pub duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenEfficiencyReport {
    pub receipt_count: u64,
    pub host_reported_receipts: u64,
    pub local_tokenizer_receipts: u64,
    pub estimated_receipts: u64,
    pub unknown_receipts: u64,
    pub measured_input_tokens: u64,
    pub measured_output_tokens: u64,
    pub measured_reasoning_tokens: u64,
    pub cached_input_tokens: u64,
    pub estimated_input_token_upper_bound: u64,
    pub verified_successes: u64,
    pub verified_failures: u64,
    pub measured_verified_successes: u64,
    pub measured_tokens_per_verified_success: Option<f64>,
    pub measurement_complete: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsageAudit {
    pub receipt_count: u64,
    pub digest_mismatch_count: u64,
    pub consistent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MemoryClass {
    Episodic,
    Semantic,
    Procedural,
    Gotcha,
    Unknown,
}

impl MemoryClass {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Episodic => "episodic",
            Self::Semantic => "semantic",
            Self::Procedural => "procedural",
            Self::Gotcha => "gotcha",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryLifecycle {
    Active,
    Superseded,
    Quarantined,
    Tombstoned,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MemorySearchRequest {
    pub workspace: String,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub classes: Vec<MemoryClass>,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub max_bytes: Option<u32>,
    #[serde(default)]
    pub as_of_unix_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MemoryGetRequest {
    pub workspace: String,
    pub memory_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryItem {
    pub memory_id: String,
    pub source_kind: String,
    pub source_id: String,
    pub memory_class: MemoryClass,
    pub content: String,
    pub origin_channel: OriginChannel,
    pub influence_class: InfluenceClass,
    pub source_status: Option<String>,
    pub lifecycle_state: MemoryLifecycle,
    pub valid_from_unix_ms: i64,
    pub valid_until_unix_ms: Option<i64>,
    pub applicability: serde_json::Value,
    pub selection_reasons: Vec<String>,
    pub created_at_unix_ms: i64,
    pub updated_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemorySearchResult {
    pub project_id: Option<String>,
    pub workspace: String,
    pub query_terms: Vec<String>,
    pub items: Vec<MemoryItem>,
    pub budget: ContextBudget,
    pub warnings: Vec<String>,
    pub policy_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProjectionAudit {
    pub expected_source_count: u64,
    pub item_count: u64,
    pub active_count: u64,
    pub superseded_count: u64,
    pub untrusted_count: u64,
    pub fts_count: u64,
    pub source_consistent: bool,
    pub consistent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryEdge {
    pub edge_id: String,
    pub from_memory_id: String,
    pub to_memory_id: String,
    pub relation: String,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryExposure {
    pub exposure_id: String,
    pub event_kind: String,
    pub host_session_hmac: Option<String>,
    pub host_turn_hmac: Option<String>,
    pub policy_sha256: String,
    pub memory_ids: Vec<String>,
    pub content_bytes: u32,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEventKind {
    SessionStart,
    UserPrompt,
    PreTool,
    PostTool,
    ToolFailure,
    PermissionRequest,
    Stop,
    SessionEnd,
    Unknown,
}

impl RuntimeEventKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::SessionStart => "session_start",
            Self::UserPrompt => "user_prompt",
            Self::PreTool => "pre_tool",
            Self::PostTool => "post_tool",
            Self::ToolFailure => "tool_failure",
            Self::PermissionRequest => "permission_request",
            Self::Stop => "stop",
            Self::SessionEnd => "session_end",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityClass {
    Read,
    Write,
    Execute,
    Network,
    ExternalMutation,
    Delegation,
    Unknown,
}

impl CapabilityClass {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Execute => "execute",
            Self::Network => "network",
            Self::ExternalMutation => "external_mutation",
            Self::Delegation => "delegation",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeOutcomeStatus {
    Proposed,
    Succeeded,
    Failed,
    Unknown,
}

impl RuntimeOutcomeStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShadowDisposition {
    Observe,
    Warn,
    WouldAsk,
    WouldDeny,
}

impl ShadowDisposition {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Warn => "warn",
            Self::WouldAsk => "would_ask",
            Self::WouldDeny => "would_deny",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeObservation {
    pub workspace: String,
    pub host_provider: String,
    pub event_kind: RuntimeEventKind,
    pub host_session_id: Option<String>,
    pub host_turn_id: Option<String>,
    pub host_tool_call_id: Option<String>,
    pub tool_name: Option<String>,
    pub capability_class: CapabilityClass,
    pub outcome_status: RuntimeOutcomeStatus,
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub latency_ms: Option<u64>,
    pub hook_schema_version: Option<String>,
    pub shadow_disposition: ShadowDisposition,
    pub shadow_reasons: Vec<String>,
    pub exposure_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTraceListRequest {
    pub workspace: String,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub event_kinds: Vec<RuntimeEventKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTraceGetRequest {
    pub workspace: String,
    pub event_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeWorkspaceRequest {
    pub workspace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeEvent {
    pub sequence: u64,
    pub event_id: String,
    pub exposure_id: Option<String>,
    pub host_provider: String,
    pub event_kind: RuntimeEventKind,
    pub host_session_hmac: Option<String>,
    pub host_turn_hmac: Option<String>,
    pub host_tool_call_hmac: Option<String>,
    pub tool_name: Option<String>,
    pub capability_class: CapabilityClass,
    pub outcome_status: RuntimeOutcomeStatus,
    pub input_hmac: Option<String>,
    pub input_bytes: u64,
    pub output_hmac: Option<String>,
    pub output_bytes: u64,
    pub latency_ms: Option<u64>,
    pub hook_schema_version: Option<String>,
    pub shadow_disposition: ShadowDisposition,
    pub shadow_reasons: Vec<String>,
    pub duplicate_of_event_id: Option<String>,
    pub received_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityObservation {
    pub observation_id: String,
    pub host_provider: String,
    pub tool_name: String,
    pub capability_class: CapabilityClass,
    pub first_seen_at_unix_ms: i64,
    pub last_seen_at_unix_ms: i64,
    pub event_count: u64,
    pub succeeded_count: u64,
    pub failed_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityReport {
    pub project_id: Option<String>,
    pub workspace: String,
    pub observations: Vec<CapabilityObservation>,
    pub authority_notice: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookHealthReport {
    pub project_id: Option<String>,
    pub workspace: String,
    pub event_count: u64,
    pub unmatched_pre_tool_count: u64,
    pub terminal_without_pre_count: u64,
    pub duplicate_count: u64,
    pub unknown_event_count: u64,
    pub unknown_capability_count: u64,
    pub last_event_at_unix_ms: Option<i64>,
    pub no_detected_gaps: bool,
    pub coverage_proven: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeProjectionAudit {
    pub event_tool_group_count: u64,
    pub capability_projection_count: u64,
    pub consistent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitObserveRequest {
    pub workspace: String,
    #[serde(default)]
    pub base_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitSnapshotListRequest {
    pub workspace: String,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitSnapshotGetRequest {
    pub workspace: String,
    pub snapshot_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceFinding {
    pub code: String,
    pub severity: GovernanceSeverity,
    pub evidence: String,
    pub requires_independent_review: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitPathChange {
    pub source: String,
    pub status: String,
    pub path: String,
    pub previous_path: Option<String>,
    pub governance_sensitive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitWorktreeObservation {
    pub head_commit: Option<String>,
    pub branch: Option<String>,
    pub detached: bool,
    pub locked: bool,
    pub prunable: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct GitSnapshotDraft {
    pub workspace: String,
    pub repository_root: String,
    pub head_commit: Option<String>,
    pub head_tree: Option<String>,
    pub branch: Option<String>,
    pub detached: bool,
    pub upstream_ref: Option<String>,
    pub base_ref: Option<String>,
    pub base_commit: Option<String>,
    pub merge_base: Option<String>,
    pub ahead_count: Option<u64>,
    pub behind_count: Option<u64>,
    pub dirty: bool,
    pub staged_count: u64,
    pub unstaged_count: u64,
    pub untracked_count: u64,
    pub local_branch_count: u64,
    pub head_parent_count: Option<u32>,
    pub head_has_signature: bool,
    pub paths_truncated: bool,
    pub changed_paths: Vec<GitPathChange>,
    pub worktrees: Vec<GitWorktreeObservation>,
    pub remotes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitSnapshot {
    pub sequence: u64,
    pub snapshot_id: String,
    pub repository_root: String,
    pub head_commit: Option<String>,
    pub head_tree: Option<String>,
    pub branch: Option<String>,
    pub detached: bool,
    pub upstream_ref: Option<String>,
    pub base_ref: Option<String>,
    pub base_commit: Option<String>,
    pub merge_base: Option<String>,
    pub ahead_count: Option<u64>,
    pub behind_count: Option<u64>,
    pub remote_state_fresh: bool,
    pub dirty: bool,
    pub staged_count: u64,
    pub unstaged_count: u64,
    pub untracked_count: u64,
    pub local_branch_count: u64,
    pub head_parent_count: Option<u32>,
    pub head_has_signature: bool,
    pub successful_receipt_bound: bool,
    pub paths_truncated: bool,
    pub changed_paths: Vec<GitPathChange>,
    pub worktrees: Vec<GitWorktreeObservation>,
    pub remotes: Vec<String>,
    pub findings: Vec<GovernanceFinding>,
    pub policy_version: u32,
    pub snapshot_sha256: String,
    pub captured_at_unix_ms: i64,
    pub approval_proven: bool,
    pub authority_notice: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitSnapshotAudit {
    pub snapshot_count: u64,
    pub digest_mismatch_count: u64,
    pub invalid_json_count: u64,
    pub consistent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeliberationNodeKind {
    Question,
    Premise,
    Claim,
    Objection,
    Counterexample,
    Falsifier,
    ValueConstraint,
    MaterialUnknown,
    Proposal,
    Revision,
}

impl DeliberationNodeKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Question => "question",
            Self::Premise => "premise",
            Self::Claim => "claim",
            Self::Objection => "objection",
            Self::Counterexample => "counterexample",
            Self::Falsifier => "falsifier",
            Self::ValueConstraint => "value_constraint",
            Self::MaterialUnknown => "material_unknown",
            Self::Proposal => "proposal",
            Self::Revision => "revision",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeliberationEdgeKind {
    Support,
    Attack,
    Undercut,
    Dependency,
    Falsification,
    Revision,
}

impl DeliberationEdgeKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Support => "support",
            Self::Attack => "attack",
            Self::Undercut => "undercut",
            Self::Dependency => "dependency",
            Self::Falsification => "falsification",
            Self::Revision => "revision",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeliberationCreateRequest {
    pub workspace: String,
    pub title: String,
    pub question: String,
    pub git_snapshot_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeliberationRelationRequest {
    pub target_node_id: String,
    pub kind: DeliberationEdgeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeliberationNodeAddRequest {
    pub workspace: String,
    pub deliberation_id: String,
    pub kind: DeliberationNodeKind,
    pub statement: String,
    #[serde(default)]
    pub material: bool,
    #[serde(default)]
    pub evidence_id: Option<String>,
    #[serde(default)]
    pub claim_id: Option<String>,
    #[serde(default)]
    pub relations: Vec<DeliberationRelationRequest>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeliberationDecisionRequest {
    pub workspace: String,
    pub deliberation_id: String,
    pub proposal_node_id: String,
    pub summary: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeliberationGetRequest {
    pub workspace: String,
    pub deliberation_id: String,
    #[serde(default)]
    pub node_after_sequence: Option<u64>,
    #[serde(default)]
    pub edge_after_sequence: Option<u64>,
    #[serde(default)]
    pub decision_after_sequence: Option<u64>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeliberationListRequest {
    pub workspace: String,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deliberation {
    pub sequence: u64,
    pub deliberation_id: String,
    pub title: String,
    pub question: String,
    pub git_snapshot_id: String,
    pub bound_head_commit: Option<String>,
    pub bound_head_tree: Option<String>,
    pub deliberation_sha256: String,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliberationNode {
    pub sequence: u64,
    pub node_id: String,
    pub deliberation_id: String,
    pub kind: DeliberationNodeKind,
    pub statement: String,
    pub material: bool,
    pub evidence_id: Option<String>,
    pub claim_id: Option<String>,
    pub node_sha256: String,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliberationEdge {
    pub sequence: u64,
    pub edge_id: String,
    pub deliberation_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub kind: DeliberationEdgeKind,
    pub edge_sha256: String,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliberationDecision {
    pub sequence: u64,
    pub decision_id: String,
    pub deliberation_id: String,
    pub proposal_node_id: String,
    pub summary: String,
    pub open_material_issues: u64,
    pub decision_sha256: String,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliberationGraph {
    pub deliberation: Deliberation,
    pub nodes: Vec<DeliberationNode>,
    pub edges: Vec<DeliberationEdge>,
    pub decisions: Vec<DeliberationDecision>,
    pub total_node_count: u64,
    pub total_edge_count: u64,
    pub total_decision_count: u64,
    pub truncated: bool,
    pub next_node_after_sequence: Option<u64>,
    pub next_edge_after_sequence: Option<u64>,
    pub next_decision_after_sequence: Option<u64>,
    pub current_git_snapshot_id: Option<String>,
    pub stale: bool,
    pub open_material_node_ids: Vec<String>,
    pub total_open_material_issue_count: u64,
    pub open_material_issues_truncated: bool,
    pub provisional_only: bool,
    pub approval_proven: bool,
    pub authority_notice: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliberationSummary {
    pub deliberation_id: String,
    pub title: String,
    pub git_snapshot_id: String,
    pub current_git_snapshot_id: Option<String>,
    pub stale: bool,
    pub node_count: u64,
    pub edge_count: u64,
    pub decision_count: u64,
    pub open_material_issue_count: u64,
    pub created_at_unix_ms: i64,
    pub provisional_only: bool,
    pub approval_proven: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliberationOutcome {
    pub graph: DeliberationGraph,
    pub duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliberationAudit {
    pub deliberation_count: u64,
    pub node_count: u64,
    pub edge_count: u64,
    pub decision_count: u64,
    pub digest_mismatch_count: u64,
    pub cross_graph_edge_count: u64,
    pub consistent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityProviderKind {
    BuiltIn,
    ExternalArtifact,
    WorkspaceManifest,
}

impl CapabilityProviderKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::BuiltIn => "built_in",
            Self::ExternalArtifact => "external_artifact",
            Self::WorkspaceManifest => "workspace_manifest",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityEffectClass {
    Observe,
    RecordLocal,
    VerifyLocal,
    ExternalEffect,
    Privileged,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityMaturity {
    #[default]
    Observe,
    Propose,
    SandboxedExecute,
    ConnectedEffect,
    PersistentRoutine,
}

impl CapabilityMaturity {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Propose => "propose",
            Self::SandboxedExecute => "sandboxed_execute",
            Self::ConnectedEffect => "connected_effect",
            Self::PersistentRoutine => "persistent_routine",
        }
    }

    pub(crate) fn rank(&self) -> u8 {
        match self {
            Self::Observe => 0,
            Self::Propose => 1,
            Self::SandboxedExecute => 2,
            Self::ConnectedEffect => 3,
            Self::PersistentRoutine => 4,
        }
    }
}

impl CapabilityEffectClass {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::RecordLocal => "record_local",
            Self::VerifyLocal => "verify_local",
            Self::ExternalEffect => "external_effect",
            Self::Privileged => "privileged",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityCatalogState {
    Registered,
    Available,
    Disabled,
    Deprecated,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilityRegisterRequest {
    pub session_id: String,
    pub capability_id: String,
    pub version: String,
    pub provider_kind: CapabilityProviderKind,
    pub title: String,
    pub description: String,
    pub effect_class: CapabilityEffectClass,
    #[serde(default)]
    pub maturity: Option<CapabilityMaturity>,
    #[serde(default)]
    pub reads_private_data: bool,
    #[serde(default)]
    pub sees_untrusted_content: bool,
    #[serde(default)]
    pub uses_network: bool,
    #[serde(default)]
    pub requires_credentials: bool,
    #[serde(default)]
    pub idempotent: bool,
    #[serde(default)]
    pub reversible: bool,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub evidence_contract: String,
    #[serde(default)]
    pub implementation_sha256: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilitySearchRequest {
    pub workspace: String,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub include_unavailable: bool,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilityGetRequest {
    pub workspace: String,
    pub capability_id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityManifest {
    pub sequence: u64,
    pub capability_id: String,
    pub version: String,
    pub provider_kind: CapabilityProviderKind,
    pub title: String,
    pub description: String,
    pub effect_class: CapabilityEffectClass,
    #[serde(default)]
    pub maturity: CapabilityMaturity,
    pub reads_private_data: bool,
    pub sees_untrusted_content: bool,
    pub uses_network: bool,
    pub requires_credentials: bool,
    pub idempotent: bool,
    pub reversible: bool,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub evidence_contract: String,
    pub implementation_sha256: Option<String>,
    pub manifest_sha256: String,
    pub state: CapabilityCatalogState,
    pub created_at_unix_ms: i64,
    pub executable: bool,
    pub authority_notice: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySummary {
    pub capability_id: String,
    pub version: String,
    pub title: String,
    pub effect_class: CapabilityEffectClass,
    pub maturity: CapabilityMaturity,
    pub state: CapabilityCatalogState,
    pub manifest_sha256: String,
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityOutcome {
    pub capability: CapabilityManifest,
    pub duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentCriterionKind {
    HardGate,
    ParetoDimension,
}

impl ExperimentCriterionKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::HardGate => "hard_gate",
            Self::ParetoDimension => "pareto_dimension",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentComparison {
    MustPass,
    Minimize,
    Maximize,
    Gte,
    Lte,
}

impl ExperimentComparison {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::MustPass => "must_pass",
            Self::Minimize => "minimize",
            Self::Maximize => "maximize",
            Self::Gte => "gte",
            Self::Lte => "lte",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperimentCriterionInput {
    pub name: String,
    pub kind: ExperimentCriterionKind,
    pub comparison: ExperimentComparison,
    #[serde(default)]
    pub threshold: Option<i64>,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperimentCreateRequest {
    pub workspace: String,
    pub title: String,
    pub problem: String,
    pub target_user: String,
    pub desired_outcome: String,
    pub hypothesis: String,
    pub git_snapshot_id: String,
    pub max_variants: u32,
    #[serde(default)]
    pub max_token_budget: Option<u64>,
    pub criteria: Vec<ExperimentCriterionInput>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentDiversityAxis {
    ProductAssumption,
    Ux,
    Architecture,
    DataModel,
    Automation,
    CostSafety,
}

impl ExperimentDiversityAxis {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::ProductAssumption => "product_assumption",
            Self::Ux => "ux",
            Self::Architecture => "architecture",
            Self::DataModel => "data_model",
            Self::Automation => "automation",
            Self::CostSafety => "cost_safety",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperimentVariantAddRequest {
    pub workspace: String,
    pub campaign_id: String,
    pub name: String,
    pub diversity_axis: ExperimentDiversityAxis,
    pub approach: String,
    pub git_snapshot_id: String,
    #[serde(default)]
    pub parent_variant_ids: Vec<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperimentMeasurementAddRequest {
    pub workspace: String,
    pub campaign_id: String,
    pub variant_id: String,
    pub criterion_id: String,
    pub value: i64,
    #[serde(default)]
    pub evidence_id: Option<String>,
    #[serde(default)]
    pub claim_id: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentDecisionKind {
    Advance,
    Eliminate,
    Synthesize,
    Abandon,
    Select,
}

impl ExperimentDecisionKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Advance => "advance",
            Self::Eliminate => "eliminate",
            Self::Synthesize => "synthesize",
            Self::Abandon => "abandon",
            Self::Select => "select",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperimentDecisionRequest {
    pub workspace: String,
    pub campaign_id: String,
    pub kind: ExperimentDecisionKind,
    #[serde(default)]
    pub variant_id: Option<String>,
    pub summary: String,
    #[serde(default)]
    pub deliberation_id: Option<String>,
    #[serde(default)]
    pub deliberation_decision_id: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperimentGetRequest {
    pub workspace: String,
    pub campaign_id: String,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperimentListRequest {
    pub workspace: String,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentCampaign {
    pub sequence: u64,
    pub campaign_id: String,
    pub title: String,
    pub problem: String,
    pub target_user: String,
    pub desired_outcome: String,
    pub hypothesis: String,
    pub git_snapshot_id: String,
    pub bound_base_commit: String,
    pub bound_base_tree: String,
    pub max_variants: u32,
    pub max_token_budget: Option<u64>,
    pub campaign_sha256: String,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentCriterion {
    pub sequence: u64,
    pub criterion_id: String,
    pub contract_revision: u32,
    pub name: String,
    pub kind: ExperimentCriterionKind,
    pub comparison: ExperimentComparison,
    pub threshold: Option<i64>,
    pub unit: String,
    pub criterion_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentVariant {
    pub sequence: u64,
    pub variant_id: String,
    pub name: String,
    pub diversity_axis: ExperimentDiversityAxis,
    pub approach: String,
    pub approach_sha256: String,
    pub git_snapshot_id: String,
    pub head_commit: String,
    pub head_tree: String,
    pub parent_variant_ids: Vec<String>,
    pub variant_sha256: String,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentMeasurement {
    pub sequence: u64,
    pub measurement_id: String,
    pub variant_id: String,
    pub criterion_id: String,
    pub value: i64,
    pub evidence_id: Option<String>,
    pub claim_id: Option<String>,
    pub evidence_qualified: bool,
    pub measurement_sha256: String,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentDecision {
    pub sequence: u64,
    pub decision_id: String,
    pub kind: ExperimentDecisionKind,
    pub variant_id: Option<String>,
    pub summary: String,
    pub deliberation_id: Option<String>,
    pub deliberation_decision_id: Option<String>,
    pub qualified: bool,
    pub decision_sha256: String,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentPortfolio {
    pub campaign: ExperimentCampaign,
    pub criteria: Vec<ExperimentCriterion>,
    pub variants: Vec<ExperimentVariant>,
    pub measurements: Vec<ExperimentMeasurement>,
    pub decisions: Vec<ExperimentDecision>,
    pub pareto_variant_ids: Vec<String>,
    pub hard_gate_failed_variant_ids: Vec<String>,
    pub evidence_incomplete_variant_ids: Vec<String>,
    pub budget_exhausted: bool,
    pub current_git_snapshot_id: Option<String>,
    pub integration_stale: bool,
    pub executable: bool,
    pub approval_proven: bool,
    pub authority_notice: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentSummary {
    pub campaign_id: String,
    pub title: String,
    pub variant_count: u64,
    pub decision_count: u64,
    pub budget_exhausted: bool,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentOutcome {
    pub portfolio: ExperimentPortfolio,
    pub duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SecurityCoverage {
    Complete,
    Partial,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityImportRequest {
    pub session_id: String,
    pub provider_capability_id: String,
    pub provider_version: String,
    pub source_scan_id: String,
    pub git_snapshot_id: String,
    pub manifest_path: String,
    pub findings_path: String,
    pub coverage_path: String,
    pub idempotency_key: String,
}

impl SecurityCoverage {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityArtifactImport {
    pub session_id: String,
    pub provider_capability_id: String,
    pub provider_version: String,
    pub source_scan_id: String,
    pub git_snapshot_id: String,
    pub coverage: SecurityCoverage,
    pub reportable_critical: u32,
    pub reportable_high: u32,
    pub reportable_medium: u32,
    pub reportable_low: u32,
    pub manifest_locator: String,
    pub manifest_sha256: String,
    pub findings_locator: String,
    pub findings_sha256: String,
    pub coverage_locator: String,
    pub coverage_sha256: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SecurityAssessmentGetRequest {
    pub workspace: String,
    pub assessment_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SecurityAssessmentListRequest {
    pub workspace: String,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityAssessment {
    pub sequence: u64,
    pub assessment_id: String,
    pub provider_capability_id: String,
    pub provider_version: String,
    pub source_scan_id: String,
    pub git_snapshot_id: String,
    pub target_commit: String,
    pub target_tree: String,
    pub coverage: SecurityCoverage,
    pub reportable_critical: u32,
    pub reportable_high: u32,
    pub reportable_medium: u32,
    pub reportable_low: u32,
    pub manifest_evidence_id: String,
    pub findings_evidence_id: String,
    pub coverage_evidence_id: String,
    pub assessment_sha256: String,
    pub created_at_unix_ms: i64,
    pub safety_proven: bool,
    pub authority_notice: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityAssessmentOutcome {
    pub assessment: SecurityAssessment,
    pub duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecureCapabilityAudit {
    pub capability_manifest_count: u64,
    pub capability_state_event_count: u64,
    pub experiment_campaign_count: u64,
    pub experiment_criterion_count: u64,
    pub experiment_variant_count: u64,
    pub experiment_measurement_count: u64,
    pub experiment_decision_count: u64,
    pub security_assessment_count: u64,
    pub integrity_failure_count: u64,
    pub consistent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryMode {
    BlindShadow,
    VisibleAdvisory,
}

impl AdvisoryMode {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::BlindShadow => "blind_shadow",
            Self::VisibleAdvisory => "visible_advisory",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryRoleKind {
    Steward,
    Worker,
}

impl AdvisoryRoleKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Steward => "steward",
            Self::Worker => "worker",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrchestrationRunStatus {
    AwaitingReport,
    Sealed,
    Evaluated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryDisposition {
    Abstain,
    Recommend,
    FlagRisk,
    ProposeWork,
}

impl AdvisoryDisposition {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Abstain => "abstain",
            Self::Recommend => "recommend",
            Self::FlagRisk => "flag_risk",
            Self::ProposeWork => "propose_work",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PredictedTaskOutcome {
    Completion,
    NonCompletion,
    Uncertain,
}

impl PredictedTaskOutcome {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Completion => "completion",
            Self::NonCompletion => "non_completion",
            Self::Uncertain => "uncertain",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdvisorySourceKind {
    HostReported,
    DeterministicSimulator,
}

impl AdvisorySourceKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::HostReported => "host_reported",
            Self::DeterministicSimulator => "deterministic_simulator",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActualTaskOutcome {
    Completed,
    Cancelled,
}

impl ActualTaskOutcome {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrchestrationRunCreateRequest {
    pub session_id: String,
    pub task_id: String,
    pub git_snapshot_id: String,
    pub context_exposure_id: String,
    pub mode: AdvisoryMode,
    pub role_kind: AdvisoryRoleKind,
    pub role_id: String,
    #[serde(default)]
    pub role_appointment_id: Option<String>,
    pub objective: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    pub max_input_tokens: u64,
    pub max_output_tokens: u64,
    pub max_duration_seconds: u64,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdvisoryRoleReportRequest {
    pub run_id: String,
    pub source_kind: AdvisorySourceKind,
    pub disposition: AdvisoryDisposition,
    pub predicted_task_outcome: PredictedTaskOutcome,
    pub summary: String,
    pub public_rationale: String,
    #[serde(default)]
    pub recommended_next_action: Option<String>,
    #[serde(default)]
    pub uncertainties: Vec<String>,
    #[serde(default)]
    pub addressed_criteria: Vec<String>,
    #[serde(default)]
    pub reported_input_tokens: Option<u64>,
    #[serde(default)]
    pub reported_output_tokens: Option<u64>,
    #[serde(default)]
    pub reported_duration_ms: Option<u64>,
    #[serde(default)]
    pub claims_task_complete: bool,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ShadowEvaluationRequest {
    pub run_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrchestrationRunGetRequest {
    pub workspace: String,
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrchestrationRunListRequest {
    pub workspace: String,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrchestrationRun {
    pub sequence: u64,
    pub run_id: String,
    pub session_id: String,
    pub task_id: String,
    pub git_snapshot_id: String,
    pub context_exposure_id: String,
    pub mode: AdvisoryMode,
    pub role_kind: AdvisoryRoleKind,
    pub role_id: String,
    pub role_appointment_id: Option<String>,
    pub objective: String,
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
    pub max_input_tokens: u64,
    pub max_output_tokens: u64,
    pub max_duration_seconds: u64,
    pub capability_ceiling: CapabilityMaturity,
    pub status: OrchestrationRunStatus,
    pub bound_head_commit: String,
    pub bound_head_tree: String,
    pub context_policy_sha256: String,
    pub run_sha256: String,
    pub created_at_unix_ms: i64,
    pub sealed_at_unix_ms: Option<i64>,
    pub evaluated_at_unix_ms: Option<i64>,
    pub advisory: bool,
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvisoryRoleReport {
    pub sequence: u64,
    pub report_id: String,
    pub run_id: String,
    pub source_kind: AdvisorySourceKind,
    pub disposition: AdvisoryDisposition,
    pub predicted_task_outcome: PredictedTaskOutcome,
    pub summary: String,
    pub public_rationale: String,
    pub recommended_next_action: Option<String>,
    pub uncertainties: Vec<String>,
    pub addressed_criteria: Vec<String>,
    pub reported_input_tokens: Option<u64>,
    pub reported_output_tokens: Option<u64>,
    pub reported_duration_ms: Option<u64>,
    pub report_sha256: String,
    pub created_at_unix_ms: i64,
    pub evidence_grade: EvidenceGrade,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShadowEvaluation {
    pub sequence: u64,
    pub evaluation_id: String,
    pub run_id: String,
    pub actual_task_outcome: ActualTaskOutcome,
    pub prediction_match: Option<bool>,
    pub acceptance_criterion_count: u64,
    pub addressed_criterion_count: u64,
    pub verified_criterion_count: u64,
    pub git_stale: bool,
    pub evidence_eligible: bool,
    pub evaluation_sha256: String,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrchestrationRunView {
    pub run: OrchestrationRun,
    pub report: Option<AdvisoryRoleReport>,
    pub report_revealed: bool,
    pub evaluation: Option<ShadowEvaluation>,
    pub authority_notice: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrchestrationOutcome {
    pub view: OrchestrationRunView,
    pub duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrchestrationRunSummary {
    pub run_id: String,
    pub task_id: String,
    pub mode: AdvisoryMode,
    pub role_kind: AdvisoryRoleKind,
    pub role_id: String,
    pub status: OrchestrationRunStatus,
    pub report_revealed: bool,
    pub evidence_eligible: Option<bool>,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrchestrationAudit {
    pub run_count: u64,
    pub report_count: u64,
    pub evaluation_count: u64,
    pub digest_mismatch_count: u64,
    pub invalid_reference_count: u64,
    pub consistent: bool,
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
    pub evidence_count: u64,
    pub claim_count: u64,
    pub unresolved_material_unknown_count: u64,
    pub running_execution_count: u64,
    pub interrupted_execution_count: u64,
    pub execution_receipt_count: u64,
    pub git_snapshot_count: u64,
    pub token_usage_receipt_count: u64,
    pub deliberation_count: u64,
    pub deliberation_decision_count: u64,
    pub capability_manifest_count: u64,
    pub experiment_campaign_count: u64,
    pub security_assessment_count: u64,
    pub orchestration_run_count: u64,
    pub shadow_evaluation_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Running,
    Succeeded,
    Failed,
    TimedOut,
    Interrupted,
}

impl ExecutionStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::TimedOut => "timed_out",
            Self::Interrupted => "interrupted",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommandSpecRequest {
    pub session_id: String,
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub workspace_relative_cwd: String,
    #[serde(default)]
    pub expected_exit_code: i32,
    pub timeout_seconds: u64,
    #[serde(default)]
    pub artifact_paths: Vec<String>,
    #[serde(default)]
    pub sandbox_profile: ExecutionSandboxProfile,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SandboxEnforcement {
    #[default]
    Host,
    Required,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SandboxWorkspaceAccess {
    ReadOnly,
    #[default]
    ReadWrite,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SandboxNetworkAccess {
    Deny,
    #[default]
    Inherit,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutionSandboxProfile {
    #[serde(default)]
    pub enforcement: SandboxEnforcement,
    #[serde(default)]
    pub workspace_access: SandboxWorkspaceAccess,
    #[serde(default)]
    pub network_access: SandboxNetworkAccess,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxBackendStatus {
    pub platform: String,
    pub backend: String,
    pub supported: bool,
    pub installed: bool,
    pub required_profiles_fail_closed: bool,
    pub network_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSpec {
    pub spec_id: String,
    pub session_id: String,
    pub workspace: String,
    pub program: String,
    pub args: Vec<String>,
    pub workspace_relative_cwd: String,
    pub expected_exit_code: i32,
    pub timeout_seconds: u64,
    pub artifact_paths: Vec<String>,
    #[serde(default)]
    pub sandbox_profile: ExecutionSandboxProfile,
    pub canonical_sha256: String,
    pub success_claim: String,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSpecOutcome {
    pub spec: CommandSpec,
    pub duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionRun {
    pub run_id: String,
    pub spec_id: String,
    pub session_id: String,
    pub status: ExecutionStatus,
    pub attempt: u32,
    pub retry_of_run_id: Option<String>,
    pub started_at_unix_ms: i64,
    pub finished_at_unix_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionReceipt {
    pub receipt_id: String,
    pub run_id: String,
    pub command_spec_sha256: String,
    pub resolved_executable: Option<String>,
    pub exit_code: Option<i32>,
    pub termination: String,
    #[serde(default = "default_sandbox_backend")]
    pub sandbox_backend: String,
    #[serde(default)]
    pub sandbox_enforced: bool,
    pub stdout_sha256: String,
    pub stdout_bytes: u64,
    pub stderr_sha256: String,
    pub stderr_bytes: u64,
    pub git_head_before: Option<String>,
    pub git_head_after: Option<String>,
    pub worktree_state_before_sha256: Option<String>,
    pub worktree_state_after_sha256: Option<String>,
    pub created_at_unix_ms: i64,
}

fn default_sandbox_backend() -> String {
    "none".to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptArtifact {
    pub artifact_id: String,
    pub receipt_id: String,
    pub workspace_relative_path: String,
    pub sha256: String,
    pub byte_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionOutcome {
    pub run: ExecutionRun,
    pub receipt: Option<ExecutionReceipt>,
    pub artifacts: Vec<ReceiptArtifact>,
    pub verified_claim_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionStart {
    pub spec: CommandSpec,
    pub run: ExecutionRun,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptArtifactInput {
    pub workspace_relative_path: String,
    pub sha256: String,
    pub byte_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionFinish {
    pub run_id: String,
    pub status: ExecutionStatus,
    pub resolved_executable: Option<String>,
    pub exit_code: Option<i32>,
    pub termination: String,
    #[serde(default = "default_sandbox_backend")]
    pub sandbox_backend: String,
    #[serde(default)]
    pub sandbox_enforced: bool,
    pub stdout_sha256: String,
    pub stdout_bytes: u64,
    pub stderr_sha256: String,
    pub stderr_bytes: u64,
    pub git_head_before: Option<String>,
    pub git_head_after: Option<String>,
    pub worktree_state_before_sha256: Option<String>,
    pub worktree_state_after_sha256: Option<String>,
    pub artifacts: Vec<ReceiptArtifactInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutionListRequest {
    pub workspace: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutionGetRequest {
    pub run_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionReplayAudit {
    pub replayed_run_count: u64,
    pub projection_run_count: u64,
    pub mismatches: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    WorkspaceFile,
    CommandResult,
    ExternalSource,
    UserStatement,
    ModelAssessment,
}

impl EvidenceKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::WorkspaceFile => "workspace_file",
            Self::CommandResult => "command_result",
            Self::ExternalSource => "external_source",
            Self::UserStatement => "user_statement",
            Self::ModelAssessment => "model_assessment",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceGrade {
    Direct,
    Reported,
    ModelOnly,
}

impl EvidenceGrade {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Reported => "reported",
            Self::ModelOnly => "model_only",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRequest {
    pub session_id: String,
    pub kind: EvidenceKind,
    pub locator: String,
    pub summary: String,
    #[serde(default)]
    pub content_sha256: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceArtifact {
    pub evidence_id: String,
    pub session_id: String,
    pub kind: EvidenceKind,
    pub grade: EvidenceGrade,
    pub locator: String,
    pub summary: String,
    pub content_sha256: String,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceOutcome {
    pub evidence: EvidenceArtifact,
    pub duplicate: bool,
}

pub fn workspace_file_claim(locator: &str, content_sha256: &str) -> String {
    format!("workspace_file_sha256:{locator}:{content_sha256}")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    Observed,
    Verified,
    Inferred,
    Assumed,
    Intended,
    Unknown,
}

impl ClaimStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Verified => "verified",
            Self::Inferred => "inferred",
            Self::Assumed => "assumed",
            Self::Intended => "intended",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClaimRequest {
    pub session_id: String,
    pub status: ClaimStatus,
    pub statement: String,
    #[serde(default)]
    pub material: bool,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    #[serde(default)]
    pub supersedes_claim_id: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpistemicClaim {
    pub claim_id: String,
    pub session_id: String,
    pub status: ClaimStatus,
    pub statement: String,
    pub material: bool,
    pub evidence_ids: Vec<String>,
    #[serde(default)]
    pub receipt_ids: Vec<String>,
    pub supersedes_claim_id: Option<String>,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimOutcome {
    pub claim: EpistemicClaim,
    pub duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Consequence {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DissentRequest {
    pub session_id: String,
    pub target_claim_id: String,
    pub consequence: Consequence,
    pub actionable_change: String,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DissentAssessment {
    pub surface: bool,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkKind {
    Architecture,
    Implementation,
    Verification,
    Research,
    Extraction,
    Coordination,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkComplexity {
    Bounded,
    Complex,
    Frontier,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelRouteRequest {
    pub work_kind: WorkKind,
    pub complexity: WorkComplexity,
    pub consequence: Consequence,
    pub ambiguity_high: bool,
    #[serde(default)]
    pub independent_review: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRoute {
    pub model: String,
    pub reasoning_effort: String,
    pub verifier_model: Option<String>,
    pub reasons: Vec<String>,
    pub advisory: bool,
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
    pub completion_proofs: Vec<CriterionProof>,
    pub created_at_unix_ms: i64,
    pub updated_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CriterionEvidence {
    pub criterion: String,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CriterionProof {
    pub criterion: String,
    pub verified_claim_id: String,
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
    pub criterion_proofs: Vec<CriterionProof>,
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
    pub evidence: Vec<EvidenceArtifact>,
    pub claims: Vec<EpistemicClaim>,
    pub command_specs: Vec<CommandSpec>,
    pub execution_runs: Vec<ExecutionRun>,
    pub execution_receipts: Vec<ExecutionReceipt>,
    pub receipt_artifacts: Vec<ReceiptArtifact>,
    pub memory_items: Vec<MemoryItem>,
    pub memory_edges: Vec<MemoryEdge>,
    pub memory_exposures: Vec<MemoryExposure>,
    pub runtime_events: Vec<RuntimeEvent>,
    pub capability_observations: Vec<CapabilityObservation>,
    pub git_snapshots: Vec<GitSnapshot>,
    pub token_usage_receipts: Vec<TokenUsageReceipt>,
    pub deliberations: Vec<Deliberation>,
    pub deliberation_nodes: Vec<DeliberationNode>,
    pub deliberation_edges: Vec<DeliberationEdge>,
    pub deliberation_decisions: Vec<DeliberationDecision>,
    pub capability_manifests: Vec<CapabilityManifest>,
    pub experiments: Vec<ExperimentPortfolio>,
    pub security_assessments: Vec<SecurityAssessment>,
    pub orchestration_runs: Vec<OrchestrationRun>,
    pub sealed_advisory_report_count: u64,
    pub advisory_role_reports: Vec<AdvisoryRoleReport>,
    pub shadow_evaluations: Vec<ShadowEvaluation>,
    pub role_appointments: Vec<RoleAppointment>,
    pub office_appointments: Vec<OfficeAppointment>,
    pub product_cells: Vec<ProductCell>,
    pub research_revisions: Vec<crate::research::ResearchRevision>,
    pub events: Vec<ExportEvent>,
}
