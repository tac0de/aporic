use std::path::PathBuf;

use crate::{
    domain::{
        ClaimOutcome, ClaimRequest, CloseOutcome, CloseRequest, CommandSpecOutcome,
        CommandSpecRequest, Consequence, ContextCapsule, CoordinatedTask, DissentAssessment,
        DissentRequest, EvidenceOutcome, EvidenceRequest, ExecutionFinish, ExecutionGetRequest,
        ExecutionListRequest, ExecutionOutcome, ExecutionReplayAudit, ExecutionRun, ExecutionStart,
        HubStats, ModelRoute, ModelRouteRequest, OpenOutcome, OpenRequest, ProjectExport,
        RecallRequest, ReconcileOutcome, ReconcileRequest, RecordOutcome, RecordRequest,
        TaskCancelRequest, TaskClaimRequest, TaskCompleteRequest, TaskCreateRequest,
        TaskListRequest, TaskOutcome, WorkComplexity, WorkKind,
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

    pub fn open_session(&self, request: &OpenRequest) -> Result<OpenOutcome> {
        self.store.open_session(request, kernel::EXPECTED_SHA256)
    }

    pub fn recall(&self, request: &RecallRequest) -> Result<ContextCapsule> {
        self.store.recall(request)
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
