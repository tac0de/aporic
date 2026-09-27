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
    /// Return only selected, budgeted items; omit raw session, handoff, and record lists.
    #[serde(default)]
    pub compact: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AccountabilityOpenRequest {
    pub session_id: String,
    pub source_task_id: String,
    pub evidence_id: String,
    #[serde(default)]
    pub reported_assignee_id: Option<String>,
    pub expected_behavior: String,
    pub observed_behavior: String,
    pub impact: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AccountabilityPlanRequest {
    pub case_id: String,
    pub repair_task_id: String,
    pub root_cause_hypothesis: String,
    pub prevention_change: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AccountabilityResolveRequest {
    pub case_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AccountabilityListRequest {
    pub workspace: String,
    #[serde(default)]
    pub reported_assignee_id: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountabilityStatus {
    Open,
    Repaired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountabilityCase {
    pub case_id: String,
    pub project_id: String,
    pub session_id: String,
    pub source_task_id: String,
    pub evidence_id: String,
    pub evidence_grade: EvidenceGrade,
    pub reported_assignee_id: Option<String>,
    pub expected_behavior: String,
    pub observed_behavior: String,
    pub impact: String,
    pub case_sha256: String,
    pub status: AccountabilityStatus,
    pub repair_task_id: Option<String>,
    pub root_cause_hypothesis: Option<String>,
    pub prevention_change: Option<String>,
    pub plan_revision: u32,
    pub created_at_unix_ms: i64,
    pub repaired_at_unix_ms: Option<i64>,
    pub advisory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountabilityOutcome {
    pub case: AccountabilityCase,
    pub duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountabilityNotice {
    pub case_id: String,
    pub source_task_id: String,
    pub observed_behavior: String,
    pub repair_task_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountabilityReport {
    pub open_count: u64,
    pub repaired_count: u64,
    pub cases: Vec<AccountabilityCase>,
    pub advisory: bool,
    pub authority_notice: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountabilityAudit {
    pub case_count: u64,
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

/// Comparison is limited to source rows with decodable post-operation event snapshots.
/// Uncovered rows are historical data, not replay-verified data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreEventAudit {
    pub covered_records: u64,
    pub covered_evidence: u64,
    pub covered_claims: u64,
    pub covered_tasks: u64,
    pub uncovered_legacy_count: u64,
    pub uncovered_runner_claim_count: u64,
    pub unsupported_event_count: u64,
    pub mismatch_count: u64,
    pub mismatch_sample: Vec<String>,
    pub uncovered_sample: Vec<String>,
    pub covered_consistent: bool,
    pub scope_notice: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenOutcome {
    pub session_id: String,
    pub project_id: String,
    pub kernel_sha256: String,
    pub context: ContextCapsule,
    #[serde(default)]
    pub open_repair_count: u64,
    #[serde(default)]
    pub open_repair_obligations: Vec<AccountabilityNotice>,
    #[serde(default)]
    pub accountability_notice: String,
    #[serde(default)]
    pub session_delegation: Option<SessionDelegationStatus>,
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
    pub open_repair_case_count: u64,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_locator: Option<String>,
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
    /// Stable subject identity. Direct workspace-file claims use `file:<canonical absolute path>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_key: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_key: Option<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStage {
    Intake,
    Planning,
    Design,
    Implementation,
    Verification,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowProcedureProfile {
    General,
    Ui,
    Frontend,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowProcedureDepth {
    Light,
    Standard,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStepDisposition {
    Completed,
    Skipped,
    ReworkRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowStepStatus {
    pub step_id: String,
    pub stage: WorkflowStage,
    pub disposition: WorkflowStepDisposition,
    pub evidence_ids: Vec<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowStepDefinition {
    pub step_id: String,
    pub stage: WorkflowStage,
    pub description: String,
    pub skippable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowStepRecordRequest {
    pub task_id: String,
    pub step_id: String,
    pub disposition: WorkflowStepDisposition,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    pub reason: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowStepsRequest {
    pub workspace: String,
    pub task_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowSteps {
    pub task_id: String,
    pub template_version: Option<u32>,
    pub definitions: Vec<WorkflowStepDefinition>,
    pub status: WorkflowStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPlanRequest {
    pub task_id: String,
    pub objective: String,
    pub target_user: String,
    pub constraints: String,
    pub success_measure: String,
    #[serde(default)]
    pub material_unknowns: Vec<String>,
    #[serde(default)]
    pub unknown_resolutions: Vec<WorkflowUnknownResolution>,
    pub scope_change_evidence_id: Option<String>,
    pub requires_user_decision: bool,
    pub material_change: bool,
    #[serde(default)]
    pub procedure_profile: Option<WorkflowProcedureProfile>,
    #[serde(default)]
    pub procedure_depth: Option<WorkflowProcedureDepth>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowUnknownResolution {
    pub unknown: String,
    pub evidence_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowPlan {
    pub revision_id: String,
    pub created_at_unix_ms: i64,
    pub objective: String,
    pub target_user: String,
    pub constraints: String,
    pub success_measure: String,
    pub material_unknowns: Vec<String>,
    pub unknown_resolutions: Vec<WorkflowUnknownResolution>,
    pub scope_change_evidence_id: Option<String>,
    pub requires_user_decision: bool,
    pub material_change: bool,
    #[serde(default)]
    pub procedure_profile: Option<WorkflowProcedureProfile>,
    #[serde(default)]
    pub procedure_depth: Option<WorkflowProcedureDepth>,
    #[serde(default)]
    pub procedure_template_version: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowAdvanceRequest {
    pub task_id: String,
    pub expected_stage: WorkflowStage,
    #[serde(default)]
    pub artifact_evidence_ids: Vec<String>,
    pub user_decision_evidence_id: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowStatusRequest {
    pub workspace: String,
    pub task_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTransition {
    pub stage: WorkflowStage,
    pub artifact_evidence_ids: Vec<String>,
    pub user_decision_evidence_id: Option<String>,
    pub delegation_decision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowStatus {
    pub task_id: String,
    pub stage: WorkflowStage,
    pub plan: Option<WorkflowPlan>,
    pub transitions: Vec<WorkflowTransition>,
    #[serde(default)]
    pub step_statuses: Vec<WorkflowStepStatus>,
    pub missing_for_next_stage: Vec<String>,
    pub advisory: bool,
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowOutcome {
    pub status: WorkflowStatus,
    pub duplicate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskListRequest {
    pub workspace: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RelatedWorkspaceHint {
    pub path: String,
    #[serde(default)]
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RelatedWorkspaceRequest {
    pub workspace: String,
    pub concept: String,
    #[serde(default)]
    pub roots: Vec<String>,
    #[serde(default)]
    pub hints: Vec<RelatedWorkspaceHint>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelatedWorkspaceCandidate {
    pub path: String,
    pub match_reason: String,
    pub modified_at_unix_ms: Option<i64>,
    pub historical_context_only: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskWorkPacketRequest {
    pub workspace: String,
    pub task_id: String,
    pub route: ModelRouteRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskWorkPacket {
    pub task: CoordinatedTask,
    pub route: ModelRoute,
    pub memory_uses: Vec<TaskMemoryUse>,
    pub delegation: DelegationStatus,
    pub reviewer_reasoning_effort: Option<String>,
    pub advisory: bool,
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskBriefRequest {
    pub workspace: String,
    pub task_id: String,
    pub max_context_bytes: Option<u32>,
    #[serde(default)]
    pub variant: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBriefReceipt {
    pub receipt_id: String,
    pub task_id: String,
    pub template_id: String,
    pub template_version: u32,
    pub template_sha256: String,
    pub context_policy_sha256: String,
    pub selected_item_ids: Vec<String>,
    pub brief_sha256: String,
    pub brief_bytes: u32,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBriefOutcome {
    pub brief: String,
    pub receipt: TaskBriefReceipt,
    pub duplicate: bool,
    pub advisory: bool,
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskResearchAttachRequest {
    pub workspace: String,
    pub task_id: String,
    pub revision_id: Option<String>,
    pub host_observation: Option<HostResearchObservation>,
    pub relevance_note: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HostResearchObservation {
    pub source: String,
    pub source_url: String,
    pub title: String,
    pub excerpt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskResearchListRequest {
    pub workspace: String,
    pub task_id: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskResearchItem {
    pub item_id: String,
    pub task_id: String,
    pub revision_id: Option<String>,
    pub source: String,
    pub source_url: String,
    pub title: String,
    pub excerpt: String,
    pub content_sha256: String,
    pub provenance: String,
    pub relevance_note: String,
    pub observed_at_unix_ms: i64,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskResearchOutcome {
    pub item: TaskResearchItem,
    pub duplicate: bool,
    pub authority_notice: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskResearchAudit {
    pub consistent: bool,
    pub item_count: usize,
    pub mismatches: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DelegationDisposition {
    Delegate,
    Skip,
    NotRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DelegationChoice {
    pub disposition: DelegationDisposition,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DelegationDecisionRequest {
    pub task_id: String,
    pub parallel_paths: u8,
    pub material_change: bool,
    pub worker: DelegationChoice,
    pub reviewer: DelegationChoice,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationDecision {
    pub decision_id: String,
    pub task_id: String,
    pub parallel_paths: u8,
    pub material_change: bool,
    pub worker: DelegationChoice,
    pub reviewer: DelegationChoice,
    pub evidence_grade: EvidenceGrade,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationDecisionOutcome {
    pub decision: DelegationDecision,
    pub duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DelegationDimension {
    Worker,
    Reviewer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DelegationRunOutcome {
    Started,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DelegationReportRequest {
    pub task_id: String,
    pub decision_id: String,
    pub dimension: DelegationDimension,
    pub host_agent_id: String,
    pub model: String,
    pub reasoning_effort: String,
    pub outcome: DelegationRunOutcome,
    pub result_summary: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationReport {
    pub report_id: String,
    pub task_id: String,
    pub decision_id: String,
    pub dimension: DelegationDimension,
    pub host_agent_id: String,
    pub model: String,
    pub reasoning_effort: String,
    pub outcome: DelegationRunOutcome,
    pub result_summary: String,
    pub evidence_grade: EvidenceGrade,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationReportOutcome {
    pub report: DelegationReport,
    pub duplicate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DelegationStatusRequest {
    pub workspace: String,
    pub task_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationStatus {
    pub task_id: String,
    pub decisions: Vec<DelegationDecision>,
    pub reports: Vec<DelegationReport>,
    pub advisory_gaps: Vec<String>,
    pub advisory: bool,
    pub executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionDelegationDecisionRequest {
    pub session_id: String,
    pub parallel_paths: u8,
    pub material_change: bool,
    pub worker: DelegationChoice,
    pub reviewer: DelegationChoice,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionDelegationDecision {
    pub decision_id: String,
    pub session_id: String,
    pub parallel_paths: u8,
    pub material_change: bool,
    pub worker: DelegationChoice,
    pub reviewer: DelegationChoice,
    pub evidence_grade: EvidenceGrade,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionDelegationDecisionOutcome {
    pub decision: SessionDelegationDecision,
    pub duplicate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionDelegationReportRequest {
    pub session_id: String,
    pub decision_id: String,
    pub dimension: DelegationDimension,
    pub host_agent_id: String,
    pub model: String,
    pub reasoning_effort: String,
    pub outcome: DelegationRunOutcome,
    pub result_summary: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionDelegationReport {
    pub report_id: String,
    pub session_id: String,
    pub decision_id: String,
    pub dimension: DelegationDimension,
    pub host_agent_id: String,
    pub model: String,
    pub reasoning_effort: String,
    pub outcome: DelegationRunOutcome,
    pub result_summary: String,
    pub evidence_grade: EvidenceGrade,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionDelegationReportOutcome {
    pub report: SessionDelegationReport,
    pub duplicate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionDelegationStatusRequest {
    pub workspace: String,
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionDelegationStatus {
    pub session_id: String,
    pub decisions: Vec<SessionDelegationDecision>,
    pub reports: Vec<SessionDelegationReport>,
    pub task_assessment_count: u64,
    pub assessment_missing: bool,
    pub advisory_gaps: Vec<String>,
    pub advisory: bool,
    pub executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskMemoryUseRequest {
    pub task_id: String,
    pub memory_id: String,
    pub criterion: String,
    pub intended_action: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskMemoryUseListRequest {
    pub workspace: String,
    pub task_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskMemoryUse {
    pub use_id: String,
    pub task_id: String,
    pub memory_id: String,
    pub criterion: String,
    pub intended_action: String,
    pub memory_influence_class: InfluenceClass,
    pub memory_lifecycle_state: MemoryLifecycle,
    pub criterion_verified_claim_id: Option<String>,
    pub created_at_unix_ms: i64,
    pub advisory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskMemoryUseOutcome {
    pub memory_use: TaskMemoryUse,
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
    pub task_memory_uses: Vec<TaskMemoryUse>,
    #[serde(default)]
    pub task_brief_receipts: Vec<TaskBriefReceipt>,
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
    pub accountability_cases: Vec<AccountabilityCase>,
    pub research_revisions: Vec<crate::research::ResearchRevision>,
    #[serde(default)]
    pub task_research_items: Vec<TaskResearchItem>,
    pub events: Vec<ExportEvent>,
}
