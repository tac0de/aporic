use std::path::{Path, PathBuf};

use crate::{
    domain::{
        AccountabilityAudit, AccountabilityListRequest, AccountabilityOpenRequest,
        AccountabilityOutcome, AccountabilityPlanRequest, AccountabilityReport,
        AccountabilityResolveRequest, CapabilityReport, ClaimOutcome, ClaimRequest, CloseOutcome,
        CloseRequest, CommandSpecOutcome, CommandSpecRequest, Consequence, ContextCapsule,
        CoordinatedTask, CoreEventAudit, DelegationDecisionOutcome, DelegationDecisionRequest,
        DelegationReportOutcome, DelegationReportRequest, DelegationStatus,
        DelegationStatusRequest, DissentAssessment, DissentRequest, EvidenceOutcome,
        EvidenceRequest, ExecutionFinish, ExecutionGetRequest, ExecutionListRequest,
        ExecutionOutcome, ExecutionReplayAudit, ExecutionRun, ExecutionStart, GitObserveRequest,
        GitSnapshot, GitSnapshotAudit, GitSnapshotGetRequest, GitSnapshotListRequest,
        HookHealthReport, HubStats, InitiativeArtifactRequest, InitiativeOutcome,
        InitiativePlanRequest, InitiativeStatus, InitiativeStatusRequest,
        InitiativeTaskLinkRequest, MemoryGetRequest, MemoryItem, MemoryProjectionAudit,
        MemorySearchRequest, MemorySearchResult, ModelRoute, ModelRouteRequest, OpenOutcome,
        OpenRequest, ProjectExport, RecallRequest, ReconcileOutcome, ReconcileRequest,
        RecordOutcome, RecordRequest, ResumeBrief, ResumeRequest, RuntimeEvent, RuntimeObservation,
        RuntimeProjectionAudit, RuntimeTraceGetRequest, RuntimeTraceListRequest,
        RuntimeWorkspaceRequest, SessionDelegationDecisionOutcome,
        SessionDelegationDecisionRequest, SessionDelegationReportOutcome,
        SessionDelegationReportRequest, SessionDelegationStatus, SessionDelegationStatusRequest,
        TaskBriefOutcome, TaskBriefRequest, TaskCancelRequest, TaskClaimRequest,
        TaskCompleteRequest, TaskCreateRequest, TaskListRequest, TaskMemoryUse,
        TaskMemoryUseListRequest, TaskMemoryUseOutcome, TaskMemoryUseRequest, TaskOutcome,
        TaskResearchAttachRequest, TaskResearchAudit, TaskResearchItem, TaskResearchListRequest,
        TaskResearchOutcome, TaskWorkPacket, TaskWorkPacketRequest, TokenEfficiencyReport,
        TokenEfficiencyReportRequest, TokenUsageAudit, TokenUsageListRequest, TokenUsageOutcome,
        TokenUsageReceipt, TokenUsageRecordRequest, WorkComplexity, WorkKind,
        WorkflowAdvanceRequest, WorkflowOutcome, WorkflowPlanRequest, WorkflowStatus,
        WorkflowStatusRequest, WorkflowStepRecordRequest, WorkflowSteps, WorkflowStepsRequest,
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

    pub fn resume(&self, request: &ResumeRequest) -> Result<ResumeBrief> {
        self.store.resume(request)
    }

    pub fn plan_initiative(&self, request: &InitiativePlanRequest) -> Result<InitiativeOutcome> {
        self.store.plan_initiative(request)
    }

    pub fn record_initiative_artifact(
        &self,
        request: &InitiativeArtifactRequest,
    ) -> Result<InitiativeOutcome> {
        self.store.record_initiative_artifact(request)
    }

    pub fn link_initiative_task(
        &self,
        request: &InitiativeTaskLinkRequest,
    ) -> Result<InitiativeOutcome> {
        self.store.link_initiative_task(request)
    }

    pub fn initiative_status(&self, request: &InitiativeStatusRequest) -> Result<InitiativeStatus> {
        self.store.initiative_status(request)
    }

    pub fn open_accountability_case(
        &self,
        request: &AccountabilityOpenRequest,
    ) -> Result<AccountabilityOutcome> {
        self.store.open_accountability_case(request)
    }

    pub fn plan_accountability_repair(
        &self,
        request: &AccountabilityPlanRequest,
    ) -> Result<AccountabilityOutcome> {
        self.store.plan_accountability_repair(request)
    }

    pub fn resolve_accountability_case(
        &self,
        request: &AccountabilityResolveRequest,
    ) -> Result<AccountabilityOutcome> {
        self.store.resolve_accountability_case(request)
    }

    pub fn list_accountability_cases(
        &self,
        request: &AccountabilityListRequest,
    ) -> Result<AccountabilityReport> {
        self.store.list_accountability_cases(request)
    }

    pub fn audit_accountability(&self) -> Result<AccountabilityAudit> {
        self.store.audit_accountability()
    }

    pub fn memory_search(&self, request: &MemorySearchRequest) -> Result<MemorySearchResult> {
        self.store.memory_search(request)
    }

    pub fn research_search(
        &self,
        request: &crate::research::ResearchSearchRequest,
    ) -> Result<crate::research::ResearchSearchResult> {
        self.store.research_search(request)
    }

    pub fn research_get(
        &self,
        request: &crate::research::ResearchGetRequest,
    ) -> Result<crate::research::ResearchItem> {
        self.store.research_get(request)
    }

    pub fn research_fetch_task(
        &self,
        request: &crate::research::ResearchFetchRequest,
    ) -> Result<crate::research::ResearchFetchOutcome> {
        self.store
            .research_task(&request.workspace, &request.task_id)?;
        let sync = crate::research::sync(
            &self.store,
            &request.workspace,
            &request.source,
            &request.query,
        )?;
        let results = self
            .store
            .research_search(&crate::research::ResearchSearchRequest {
                workspace: request.workspace.clone(),
                query: request.query.clone(),
                source: Some(request.source.clone()),
                limit: Some(20),
                max_bytes: Some(16_384),
            })?;
        Ok(crate::research::ResearchFetchOutcome {
            task_id: request.task_id.clone(),
            sync,
            results,
        })
    }

    pub fn research_sync(
        &self,
        workspace: &str,
        source: &str,
        query: &str,
    ) -> Result<crate::research::SyncOutcome> {
        crate::research::sync(&self.store, workspace, source, query)
    }

    pub fn attach_task_research(
        &self,
        request: &TaskResearchAttachRequest,
    ) -> Result<TaskResearchOutcome> {
        self.store.attach_task_research(request)
    }

    pub fn list_task_research(
        &self,
        request: &TaskResearchListRequest,
    ) -> Result<Vec<TaskResearchItem>> {
        self.store.list_task_research(request)
    }

    pub fn audit_task_research(&self) -> Result<TaskResearchAudit> {
        self.store.audit_task_research()
    }

    pub fn audit_research(&self) -> Result<crate::research::ResearchAudit> {
        self.store.audit_research()
    }

    pub fn memory_get(&self, request: &MemoryGetRequest) -> Result<MemoryItem> {
        self.store.memory_get(request)
    }

    pub fn audit_memory_projection(&self) -> Result<MemoryProjectionAudit> {
        self.store.audit_memory_projection()
    }

    pub fn audit_core_events(&self) -> Result<CoreEventAudit> {
        self.store.audit_core_events()
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
            ("gpt-6-luna", "medium", "bounded_well_specified_work")
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

    pub fn plan_workflow(&self, request: &WorkflowPlanRequest) -> Result<WorkflowOutcome> {
        self.store.plan_workflow(request)
    }

    pub fn advance_workflow(&self, request: &WorkflowAdvanceRequest) -> Result<WorkflowOutcome> {
        self.store.advance_workflow(request)
    }

    pub fn workflow_status(&self, request: &WorkflowStatusRequest) -> Result<WorkflowStatus> {
        self.store.workflow_status(request)
    }

    pub fn workflow_steps(&self, request: &WorkflowStepsRequest) -> Result<WorkflowSteps> {
        self.store.workflow_steps(request)
    }

    pub fn record_workflow_step(
        &self,
        request: &WorkflowStepRecordRequest,
    ) -> Result<WorkflowOutcome> {
        self.store.record_workflow_step(request)
    }

    pub fn task_work_packet(&self, request: &TaskWorkPacketRequest) -> Result<TaskWorkPacket> {
        let (task, memory_uses) = self.store.task_work_packet_source(request)?;
        let delegation = self.store.delegation_status(&DelegationStatusRequest {
            workspace: request.workspace.clone(),
            task_id: request.task_id.clone(),
        })?;
        let route = self.route_model(&request.route);
        let reviewer_reasoning_effort = route.verifier_model.as_ref().map(|_| "high".to_owned());
        Ok(TaskWorkPacket {
            task,
            route,
            memory_uses,
            delegation,
            reviewer_reasoning_effort,
            advisory: true,
            executable: false,
        })
    }

    pub fn task_brief(&self, request: &TaskBriefRequest) -> Result<TaskBriefOutcome> {
        let max_context_bytes = request.max_context_bytes.unwrap_or(4_096);
        if !(256..=8_192).contains(&max_context_bytes) {
            return Err(crate::store::Error::Invalid(
                "max_context_bytes must be between 256 and 8192".to_owned(),
            ));
        }
        let task = self.store.task_brief_task(request)?;
        let capsule = self.recall(&RecallRequest {
            workspace: request.workspace.clone(),
            limit: Some(24),
            objective: Some(task.objective.clone()),
            focus_paths: task.write_scope.clone(),
            max_bytes: Some(max_context_bytes),
            compact: false,
        })?;
        let assembly = crate::brief::assemble_variant(
            &task,
            &capsule,
            max_context_bytes as usize,
            request.variant.as_deref().unwrap_or("baseline"),
        )?;
        let (receipt, duplicate) =
            self.store
                .record_task_brief(request, &assembly, &capsule.policy_sha256)?;
        Ok(TaskBriefOutcome {
            brief: assembly.text,
            receipt,
            duplicate,
            advisory: true,
            executable: false,
        })
    }

    pub fn assess_delegation(
        &self,
        request: &DelegationDecisionRequest,
    ) -> Result<DelegationDecisionOutcome> {
        self.store.assess_delegation(request)
    }

    pub fn assess_session_delegation(
        &self,
        request: &SessionDelegationDecisionRequest,
    ) -> Result<SessionDelegationDecisionOutcome> {
        self.store.assess_session_delegation(request)
    }

    pub fn report_session_delegation(
        &self,
        request: &SessionDelegationReportRequest,
    ) -> Result<SessionDelegationReportOutcome> {
        self.store.report_session_delegation(request)
    }

    pub fn session_delegation_status(
        &self,
        request: &SessionDelegationStatusRequest,
    ) -> Result<SessionDelegationStatus> {
        self.store.session_delegation_status(request)
    }

    pub fn report_delegation(
        &self,
        request: &DelegationReportRequest,
    ) -> Result<DelegationReportOutcome> {
        self.store.report_delegation(request)
    }

    pub fn delegation_status(&self, request: &DelegationStatusRequest) -> Result<DelegationStatus> {
        self.store.delegation_status(request)
    }

    pub fn apply_task_memory(
        &self,
        request: &TaskMemoryUseRequest,
    ) -> Result<TaskMemoryUseOutcome> {
        self.store.apply_task_memory(request)
    }

    pub fn list_task_memory_uses(
        &self,
        request: &TaskMemoryUseListRequest,
    ) -> Result<Vec<TaskMemoryUse>> {
        self.store.list_task_memory_uses(request)
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
