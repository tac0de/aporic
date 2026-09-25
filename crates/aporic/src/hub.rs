use std::{
    fs,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::{
    domain::{
        CapabilityGetRequest, CapabilityManifest, CapabilityOutcome, CapabilityRegisterRequest,
        CapabilityReport, CapabilitySearchRequest, CapabilitySummary, ClaimOutcome, ClaimRequest,
        CloseOutcome, CloseRequest, CommandSpecOutcome, CommandSpecRequest, Consequence,
        ContextCapsule, CoordinatedTask, DeliberationAudit, DeliberationCreateRequest,
        DeliberationDecisionRequest, DeliberationGetRequest, DeliberationGraph,
        DeliberationListRequest, DeliberationNodeAddRequest, DeliberationOutcome,
        DeliberationSummary, DissentAssessment, DissentRequest, EvidenceOutcome, EvidenceRequest,
        ExecutionFinish, ExecutionGetRequest, ExecutionListRequest, ExecutionOutcome,
        ExecutionReplayAudit, ExecutionRun, ExecutionStart, ExperimentCreateRequest,
        ExperimentDecisionRequest, ExperimentGetRequest, ExperimentListRequest,
        ExperimentMeasurementAddRequest, ExperimentOutcome, ExperimentPortfolio, ExperimentSummary,
        ExperimentVariantAddRequest, GitObserveRequest, GitSnapshot, GitSnapshotAudit,
        GitSnapshotGetRequest, GitSnapshotListRequest, HookHealthReport, HubStats,
        MemoryGetRequest, MemoryItem, MemoryProjectionAudit, MemorySearchRequest,
        MemorySearchResult, ModelRoute, ModelRouteRequest, OpenOutcome, OpenRequest, ProjectExport,
        RecallRequest, ReconcileOutcome, ReconcileRequest, RecordOutcome, RecordRequest,
        RuntimeEvent, RuntimeObservation, RuntimeProjectionAudit, RuntimeTraceGetRequest,
        RuntimeTraceListRequest, RuntimeWorkspaceRequest, SecureCapabilityAudit,
        SecurityArtifactImport, SecurityAssessment, SecurityAssessmentGetRequest,
        SecurityAssessmentListRequest, SecurityAssessmentOutcome, SecurityCoverage,
        SecurityImportRequest, TaskCancelRequest, TaskClaimRequest, TaskCompleteRequest,
        TaskCreateRequest, TaskListRequest, TaskOutcome, TokenEfficiencyReport,
        TokenEfficiencyReportRequest, TokenUsageAudit, TokenUsageListRequest, TokenUsageOutcome,
        TokenUsageReceipt, TokenUsageRecordRequest, WorkComplexity, WorkKind,
    },
    kernel,
    store::{Result, Store},
};

#[derive(Debug, Clone)]
pub struct Hub {
    store: Store,
}

impl Hub {
    pub fn open(database_path: impl Into<PathBuf>) -> Result<Self> {
        kernel::verify().map_err(crate::store::Error::Invalid)?;
        Ok(Self {
            store: Store::open(database_path)?,
        })
    }

    pub fn database_path(&self) -> &std::path::Path {
        self.store.path()
    }

    pub fn backup_to(&self, target: &Path) -> Result<()> {
        self.store.backup_to(target)
    }

    pub fn validate_backup(path: &Path) -> Result<u32> {
        Store::validate_backup(path)
    }

    pub fn open_session(&self, request: &OpenRequest) -> Result<OpenOutcome> {
        self.store.open_session(request, kernel::EXPECTED_SHA256)
    }

    pub fn recall(&self, request: &RecallRequest) -> Result<ContextCapsule> {
        self.store.recall(request)
    }

    pub fn memory_search(&self, request: &MemorySearchRequest) -> Result<MemorySearchResult> {
        self.store.memory_search(request)
    }

    pub fn memory_get(&self, request: &MemoryGetRequest) -> Result<MemoryItem> {
        self.store.memory_get(request)
    }

    pub fn audit_memory_projection(&self) -> Result<MemoryProjectionAudit> {
        self.store.audit_memory_projection()
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
        self.store.record_memory_exposure(
            workspace,
            event_kind,
            host_session_id,
            host_turn_id,
            memory_ids,
            content_bytes,
        )
    }

    pub(crate) fn record_runtime_observation(
        &self,
        observation: &RuntimeObservation,
    ) -> Result<Option<RuntimeEvent>> {
        self.store.record_runtime_observation(observation)
    }

    pub fn list_runtime_events(
        &self,
        request: &RuntimeTraceListRequest,
    ) -> Result<Vec<RuntimeEvent>> {
        self.store.list_runtime_events(request)
    }

    pub fn get_runtime_event(&self, request: &RuntimeTraceGetRequest) -> Result<RuntimeEvent> {
        self.store.get_runtime_event(request)
    }

    pub fn capability_report(&self, request: &RuntimeWorkspaceRequest) -> Result<CapabilityReport> {
        self.store.capability_report(request)
    }

    pub fn hook_health(&self, request: &RuntimeWorkspaceRequest) -> Result<HookHealthReport> {
        self.store.hook_health(request)
    }

    pub fn audit_runtime_projection(&self) -> Result<RuntimeProjectionAudit> {
        self.store.audit_runtime_projection()
    }

    pub fn export_runtime_otel(
        &self,
        request: &RuntimeWorkspaceRequest,
    ) -> Result<serde_json::Value> {
        self.store.export_runtime_otel(request)
    }

    pub fn observe_git(&self, request: &GitObserveRequest) -> Result<GitSnapshot> {
        let draft = crate::git::capture(request)?;
        self.store.record_git_snapshot(&draft)
    }

    pub fn list_git_snapshots(&self, request: &GitSnapshotListRequest) -> Result<Vec<GitSnapshot>> {
        self.store.list_git_snapshots(request)
    }

    pub fn get_git_snapshot(&self, request: &GitSnapshotGetRequest) -> Result<GitSnapshot> {
        self.store.get_git_snapshot(request)
    }

    pub fn audit_git_snapshots(&self) -> Result<GitSnapshotAudit> {
        self.store.audit_git_snapshots()
    }

    pub fn record_token_usage(
        &self,
        request: &TokenUsageRecordRequest,
    ) -> Result<TokenUsageOutcome> {
        self.store.record_token_usage(request)
    }

    pub fn list_token_usage(
        &self,
        request: &TokenUsageListRequest,
    ) -> Result<Vec<TokenUsageReceipt>> {
        self.store.list_token_usage(request)
    }

    pub fn token_efficiency_report(
        &self,
        request: &TokenEfficiencyReportRequest,
    ) -> Result<TokenEfficiencyReport> {
        self.store.token_efficiency_report(request)
    }

    pub fn audit_token_usage(&self) -> Result<TokenUsageAudit> {
        self.store.audit_token_usage()
    }

    pub fn register_capability(
        &self,
        request: &CapabilityRegisterRequest,
    ) -> Result<CapabilityOutcome> {
        self.store.register_capability(request)
    }

    pub fn search_capabilities(
        &self,
        request: &CapabilitySearchRequest,
    ) -> Result<Vec<CapabilitySummary>> {
        self.store.search_capabilities(request)
    }

    pub fn get_capability(&self, request: &CapabilityGetRequest) -> Result<CapabilityManifest> {
        self.store.get_capability(request)
    }

    pub fn audit_secure_capabilities(&self) -> Result<SecureCapabilityAudit> {
        self.store.audit_secure_capabilities()
    }

    pub fn create_experiment(
        &self,
        request: &ExperimentCreateRequest,
    ) -> Result<ExperimentOutcome> {
        self.store.create_experiment(request)
    }

    pub fn add_experiment_variant(
        &self,
        request: &ExperimentVariantAddRequest,
    ) -> Result<ExperimentOutcome> {
        self.store.add_experiment_variant(request)
    }

    pub fn add_experiment_measurement(
        &self,
        request: &ExperimentMeasurementAddRequest,
    ) -> Result<ExperimentOutcome> {
        self.store.add_experiment_measurement(request)
    }

    pub fn decide_experiment(
        &self,
        request: &ExperimentDecisionRequest,
    ) -> Result<ExperimentOutcome> {
        self.store.decide_experiment(request)
    }

    pub fn get_experiment(&self, request: &ExperimentGetRequest) -> Result<ExperimentPortfolio> {
        self.store.get_experiment(request)
    }

    pub fn list_experiments(
        &self,
        request: &ExperimentListRequest,
    ) -> Result<Vec<ExperimentSummary>> {
        self.store.list_experiments(request)
    }

    pub fn import_codex_security(
        &self,
        request: &SecurityImportRequest,
    ) -> Result<SecurityAssessmentOutcome> {
        let (manifest_path, manifest, manifest_sha256) =
            read_security_json(&request.manifest_path)?;
        let (findings_path, findings, findings_sha256) =
            read_security_json(&request.findings_path)?;
        let (coverage_path, coverage, coverage_sha256) =
            read_security_json(&request.coverage_path)?;
        if let Some(scan_id) = manifest
            .get("scanId")
            .or_else(|| manifest.get("scan_id"))
            .and_then(serde_json::Value::as_str)
            && scan_id != request.source_scan_id
        {
            return Err(crate::store::Error::Conflict(
                "security manifest scan identity does not match source_scan_id".to_owned(),
            ));
        }
        let coverage_state = coverage
            .get("completeness")
            .or_else(|| coverage.pointer("/coverage/completeness"))
            .and_then(serde_json::Value::as_str)
            .map(|value| match value {
                "complete" => SecurityCoverage::Complete,
                "partial" => SecurityCoverage::Partial,
                _ => SecurityCoverage::Unknown,
            })
            .unwrap_or(SecurityCoverage::Unknown);
        let findings_array = findings
            .as_array()
            .or_else(|| {
                findings
                    .get("findings")
                    .and_then(serde_json::Value::as_array)
            })
            .ok_or_else(|| {
                crate::store::Error::Invalid(
                    "security findings artifact must be an array or contain a findings array"
                        .to_owned(),
                )
            })?;
        let mut counts = [0_u32; 4];
        for finding in findings_array {
            if finding
                .get("disposition")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| value != "reportable")
            {
                continue;
            }
            match finding.get("severity").and_then(serde_json::Value::as_str) {
                Some("critical") => counts[0] += 1,
                Some("high") => counts[1] += 1,
                Some("medium") => counts[2] += 1,
                Some("low") => counts[3] += 1,
                Some(value) => {
                    return Err(crate::store::Error::Invalid(format!(
                        "unsupported security finding severity {value}"
                    )));
                }
                None => {
                    return Err(crate::store::Error::Invalid(
                        "every imported security finding requires a severity".to_owned(),
                    ));
                }
            }
        }
        self.store
            .import_security_assessment(&SecurityArtifactImport {
                session_id: request.session_id.clone(),
                provider_capability_id: request.provider_capability_id.clone(),
                provider_version: request.provider_version.clone(),
                source_scan_id: request.source_scan_id.clone(),
                git_snapshot_id: request.git_snapshot_id.clone(),
                coverage: coverage_state,
                reportable_critical: counts[0],
                reportable_high: counts[1],
                reportable_medium: counts[2],
                reportable_low: counts[3],
                manifest_locator: manifest_path.to_string_lossy().into_owned(),
                manifest_sha256,
                findings_locator: findings_path.to_string_lossy().into_owned(),
                findings_sha256,
                coverage_locator: coverage_path.to_string_lossy().into_owned(),
                coverage_sha256,
                idempotency_key: request.idempotency_key.clone(),
            })
    }

    pub fn get_security_assessment(
        &self,
        request: &SecurityAssessmentGetRequest,
    ) -> Result<SecurityAssessment> {
        self.store.get_security_assessment(request)
    }

    pub fn list_security_assessments(
        &self,
        request: &SecurityAssessmentListRequest,
    ) -> Result<Vec<SecurityAssessment>> {
        self.store.list_security_assessments(request)
    }

    pub fn create_deliberation(
        &self,
        request: &DeliberationCreateRequest,
    ) -> Result<DeliberationOutcome> {
        self.store.create_deliberation(request)
    }

    pub fn add_deliberation_node(
        &self,
        request: &DeliberationNodeAddRequest,
    ) -> Result<DeliberationOutcome> {
        self.store.add_deliberation_node(request)
    }

    pub fn record_deliberation_decision(
        &self,
        request: &DeliberationDecisionRequest,
    ) -> Result<DeliberationOutcome> {
        self.store.record_deliberation_decision(request)
    }

    pub fn get_deliberation(&self, request: &DeliberationGetRequest) -> Result<DeliberationGraph> {
        self.store.get_deliberation(request)
    }

    pub fn list_deliberations(
        &self,
        request: &DeliberationListRequest,
    ) -> Result<Vec<DeliberationSummary>> {
        self.store.list_deliberations(request)
    }

    pub fn audit_deliberations(&self) -> Result<DeliberationAudit> {
        self.store.audit_deliberations()
    }

    pub fn record(&self, request: &RecordRequest) -> Result<RecordOutcome> {
        self.store.record(request)
    }

    pub fn add_evidence(&self, request: &EvidenceRequest) -> Result<EvidenceOutcome> {
        self.store.add_evidence(request)
    }

    pub fn assert_claim(&self, request: &ClaimRequest) -> Result<ClaimOutcome> {
        self.store.assert_claim(request)
    }

    pub fn assess_dissent(&self, request: &DissentRequest) -> Result<DissentAssessment> {
        self.store.assess_dissent(request)
    }

    pub fn register_command_spec(
        &self,
        request: &CommandSpecRequest,
    ) -> Result<CommandSpecOutcome> {
        self.store.register_command_spec(request)
    }

    pub async fn verify(&self, spec_id: &str) -> Result<ExecutionOutcome> {
        crate::runner::verify(self, spec_id).await
    }

    pub fn list_executions(&self, request: &ExecutionListRequest) -> Result<Vec<ExecutionRun>> {
        self.store.list_executions(request)
    }

    pub fn get_execution(&self, request: &ExecutionGetRequest) -> Result<ExecutionOutcome> {
        self.store.get_execution(request)
    }

    pub fn reconcile_executions(&self, stale_after_seconds: u64) -> Result<u64> {
        self.store.reconcile_executions(stale_after_seconds)
    }

    pub fn audit_execution_replay(&self) -> Result<ExecutionReplayAudit> {
        self.store.audit_execution_replay()
    }

    pub(crate) fn start_execution(&self, spec_id: &str) -> Result<ExecutionStart> {
        self.store.start_execution(spec_id)
    }

    pub(crate) fn finish_execution(&self, finish: &ExecutionFinish) -> Result<ExecutionOutcome> {
        self.store.finish_execution(finish)
    }

    pub fn route_model(&self, request: &ModelRouteRequest) -> ModelRoute {
        let frontier = request.complexity == WorkComplexity::Frontier
            || request.consequence == Consequence::Critical
            || (request.consequence == Consequence::High && request.ambiguity_high);
        let bounded = request.complexity == WorkComplexity::Bounded
            && matches!(request.consequence, Consequence::Low | Consequence::Medium);
        let (model, effort, reason) = if frontier {
            (
                "gpt-6-astra",
                "high",
                "frontier_or_high_consequence_ambiguity",
            )
        } else if bounded {
            ("gpt-5.6-terra", "medium", "bounded_well_specified_work")
        } else {
            (
                "gpt-6-sol",
                "high",
                match request.work_kind {
                    WorkKind::Implementation => "default_complex_implementation",
                    _ => "default_complex_agentic_work",
                },
            )
        };
        let verifier_model = request.independent_review.then(|| {
            if model == "gpt-6-astra" {
                "gpt-6-sol"
            } else {
                "gpt-6-astra"
            }
            .to_owned()
        });
        ModelRoute {
            model: model.to_owned(),
            reasoning_effort: effort.to_owned(),
            verifier_model,
            reasons: vec![reason.to_owned(), "model_output_is_not_evidence".to_owned()],
            advisory: true,
        }
    }

    pub fn close_session(&self, request: &CloseRequest) -> Result<CloseOutcome> {
        self.store.close_session(request)
    }

    pub fn reconcile(&self, request: &ReconcileRequest) -> Result<ReconcileOutcome> {
        self.store.reconcile(request)
    }

    pub fn stats(&self) -> Result<HubStats> {
        self.store.stats()
    }

    pub fn export_project(&self, workspace: &str) -> Result<ProjectExport> {
        self.store.export_project(workspace)
    }

    pub fn create_task(&self, request: &TaskCreateRequest) -> Result<TaskOutcome> {
        self.store.create_task(request)
    }

    pub fn list_tasks(&self, request: &TaskListRequest) -> Result<Vec<CoordinatedTask>> {
        self.store.list_tasks(request)
    }

    pub fn claim_task(&self, request: &TaskClaimRequest) -> Result<TaskOutcome> {
        self.store.claim_task(request)
    }

    pub fn complete_task(&self, request: &TaskCompleteRequest) -> Result<TaskOutcome> {
        self.store.complete_task(request)
    }

    pub fn cancel_task(&self, request: &TaskCancelRequest) -> Result<TaskOutcome> {
        self.store.cancel_task(request)
    }
}

fn read_security_json(raw_path: &str) -> Result<(PathBuf, serde_json::Value, String)> {
    if raw_path.trim().is_empty() {
        return Err(crate::store::Error::Invalid(
            "security artifact path must not be empty".to_owned(),
        ));
    }
    let path = Path::new(raw_path);
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(crate::store::Error::Invalid(
            "security artifacts must be regular non-symlink files".to_owned(),
        ));
    }
    if metadata.len() > 4 * 1024 * 1024 {
        return Err(crate::store::Error::Invalid(
            "security artifact exceeds the 4 MiB import limit".to_owned(),
        ));
    }
    let canonical = fs::canonicalize(path)?;
    let bytes = fs::read(&canonical)?;
    let value = serde_json::from_slice(&bytes)?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    Ok((canonical, value, digest))
}
