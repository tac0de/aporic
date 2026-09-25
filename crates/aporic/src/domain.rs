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
    pub idempotency_key: String,
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
    pub events: Vec<ExportEvent>,
}
