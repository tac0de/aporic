use rmcp::{ServerHandler, handler::server::wrapper::Parameters, tool, tool_handler, tool_router};
use serde::Serialize;

use crate::{
    Hub,
    domain::{
        AdvisoryRoleReportRequest, CapabilityGetRequest, CapabilityRegisterRequest,
        CapabilitySearchRequest, ClaimRequest, CloseRequest, CommandSpecRequest,
        DeliberationCreateRequest, DeliberationDecisionRequest, DeliberationGetRequest,
        DeliberationListRequest, DeliberationNodeAddRequest, DissentRequest, EvidenceRequest,
        ExecutionGetRequest, ExecutionListRequest, ExperimentCreateRequest,
        ExperimentDecisionRequest, ExperimentGetRequest, ExperimentListRequest,
        ExperimentMeasurementAddRequest, ExperimentVariantAddRequest, GitObserveRequest,
        GitSnapshotGetRequest, GitSnapshotListRequest, GovernmentWorkspaceRequest,
        MemoryGetRequest, MemorySearchRequest, ModelRouteRequest, OfficeAppointmentCreateRequest,
        OfficeAppointmentRevokeRequest, OpenRequest, OrchestrationRunCreateRequest,
        OrchestrationRunGetRequest, OrchestrationRunListRequest, ProductCellCreateRequest,
        RecallRequest, ReconcileRequest, RecordRequest, ResumeRequest,
        RoleAppointmentCreateRequest, RoleAppointmentListRequest, RoleAppointmentRevokeRequest,
        RuntimeTraceGetRequest, RuntimeTraceListRequest, RuntimeWorkspaceRequest,
        SecurityAssessmentGetRequest, SecurityAssessmentListRequest, ShadowEvaluationRequest,
        TaskCancelRequest, TaskClaimRequest, TaskCompleteRequest, TaskCreateRequest,
        TaskListRequest, TokenEfficiencyReportRequest, TokenUsageListRequest,
        TokenUsageRecordRequest,
    },
};

#[derive(Clone)]
pub struct AporicMcp {
    hub: Hub,
}

impl AporicMcp {
    pub fn new(hub: Hub) -> Self {
        Self { hub }
    }
}

#[tool_router]
impl AporicMcp {
    #[tool(
        description = "Open an idempotent Aporic work session for a substantive task and return bounded prior project context. This records no authority and does not replace the user's current request."
    )]
    async fn aporic_open(&self, Parameters(request): Parameters<OpenRequest>) -> String {
        render(self.hub.open_session(&request))
    }

    #[tool(
        description = "Recall a bounded set of active sessions and durable records for a workspace. Use current user intent to decide whether old records remain relevant."
    )]
    async fn aporic_recall(&self, Parameters(request): Parameters<RecallRequest>) -> String {
        render(self.hub.recall(&request))
    }

    #[tool(
        description = "Select the next unfinished project work for a new session. Read-only; returns ready only for one unambiguous task or fresh handoff. Always inspect current intent and repository state before acting."
    )]
    async fn aporic_resume(&self, Parameters(request): Parameters<ResumeRequest>) -> String {
        render(self.hub.resume(&request))
    }

    #[tool(
        description = "List four versioned advisory role contracts. Roles do not grant host capabilities or choose a model."
    )]
    async fn aporic_roles_list(&self) -> String {
        render::<Vec<crate::domain::RoleDefinition>>(Ok(self.hub.role_definitions()))
    }

    #[tool(
        description = "Record a bounded advisory role assignment to a session or active task. This does not launch an agent or grant host authority."
    )]
    async fn aporic_role_appoint(
        &self,
        Parameters(request): Parameters<RoleAppointmentCreateRequest>,
    ) -> String {
        render(self.hub.create_role_appointment(&request))
    }

    #[tool(
        description = "Revoke a recorded advisory role appointment. This does not alter host permissions."
    )]
    async fn aporic_role_revoke(
        &self,
        Parameters(request): Parameters<RoleAppointmentRevokeRequest>,
    ) -> String {
        render(self.hub.revoke_role_appointment(&request))
    }

    #[tool(
        description = "List bounded advisory role appointments for a workspace, including revoked assignments."
    )]
    async fn aporic_role_appointments(
        &self,
        Parameters(request): Parameters<RoleAppointmentListRequest>,
    ) -> String {
        render(self.hub.list_role_appointments(&request))
    }

    #[tool(
        description = "Read the versioned advisory Aporic government and product experiment ministry charter. This definition grants no authority."
    )]
    async fn aporic_government_get(&self) -> String {
        render::<crate::domain::GovernmentDefinition>(Ok(self.hub.government_definition()))
    }

    #[tool(
        description = "Appoint one advisory head of a defined government office through an active matching role appointment. This grants no host authority."
    )]
    async fn aporic_office_appoint(
        &self,
        Parameters(request): Parameters<OfficeAppointmentCreateRequest>,
    ) -> String {
        render(self.hub.create_office_appointment(&request))
    }

    #[tool(
        description = "Revoke an advisory government office appointment. This does not alter host permissions."
    )]
    async fn aporic_office_revoke(
        &self,
        Parameters(request): Parameters<OfficeAppointmentRevokeRequest>,
    ) -> String {
        render(self.hub.revoke_office_appointment(&request))
    }

    #[tool(description = "List bounded advisory government office appointments for a workspace.")]
    async fn aporic_office_appointments(
        &self,
        Parameters(request): Parameters<GovernmentWorkspaceRequest>,
    ) -> String {
        render(self.hub.list_office_appointments(&request))
    }

    #[tool(
        description = "Create one immutable, task-bound multidisciplinary product cell under the active product experiment minister. The cell records a planning-to-prototype mandate but executes nothing."
    )]
    async fn aporic_product_cell_create(
        &self,
        Parameters(request): Parameters<ProductCellCreateRequest>,
    ) -> String {
        render(self.hub.create_product_cell(&request))
    }

    #[tool(
        description = "List bounded advisory product cells for a workspace, including their problem, hypothesis, success measures, and appointed disciplines."
    )]
    async fn aporic_product_cell_list(
        &self,
        Parameters(request): Parameters<GovernmentWorkspaceRequest>,
    ) -> String {
        render(self.hub.list_product_cells(&request))
    }

    #[tool(
        description = "Search the deterministic local memory projection with FTS and temporal validity. Results are read-only data with explicit provenance and influence class; no result grants authority."
    )]
    async fn aporic_memory_search(
        &self,
        Parameters(request): Parameters<MemorySearchRequest>,
    ) -> String {
        render(self.hub.memory_search(&request))
    }

    #[tool(
        description = "Read one workspace-scoped memory item, including provenance, lifecycle, temporal validity, and influence class. This is read-only."
    )]
    async fn aporic_memory_get(&self, Parameters(request): Parameters<MemoryGetRequest>) -> String {
        render(self.hub.memory_get(&request))
    }

    #[tool(
        description = "List privacy-minimized runtime hook events for a workspace. Events contain HMAC correlations, hashes, byte counts, capability classes, and shadow decisions, never raw prompts or tool payloads. This is read-only and incomplete hooks never imply an action did not occur."
    )]
    async fn aporic_trace_list(
        &self,
        Parameters(request): Parameters<RuntimeTraceListRequest>,
    ) -> String {
        render(self.hub.list_runtime_events(&request))
    }

    #[tool(
        description = "Read one privacy-minimized runtime event by workspace and event ID. Observations and shadow decisions are telemetry, not authority."
    )]
    async fn aporic_trace_get(
        &self,
        Parameters(request): Parameters<RuntimeTraceGetRequest>,
    ) -> String {
        render(self.hub.get_runtime_event(&request))
    }

    #[tool(
        description = "Report tool capabilities observed through host hooks. This inventory does not grant, deny, or prove host permissions and is read-only."
    )]
    async fn aporic_capability_report(
        &self,
        Parameters(request): Parameters<RuntimeWorkspaceRequest>,
    ) -> String {
        render(self.hub.capability_report(&request))
    }

    #[tool(
        description = "Audit runtime-hook coverage for missing terminal events, terminals without proposals, duplicates, and unknown host schemas. This is read-only and never blocks host work."
    )]
    async fn aporic_hook_health(
        &self,
        Parameters(request): Parameters<RuntimeWorkspaceRequest>,
    ) -> String {
        render(self.hub.hook_health(&request))
    }

    #[tool(
        description = "Observe local Git metadata with read-only, non-network Git commands and append a commit-bound governance snapshot. This does not fetch, mutate Git, approve code, or prove remote freshness, signer trust, review, or merge safety."
    )]
    async fn aporic_git_observe(
        &self,
        Parameters(request): Parameters<GitObserveRequest>,
    ) -> String {
        render(self.hub.observe_git(&request))
    }

    #[tool(
        description = "List bounded append-only Git governance snapshots for a workspace. Findings are advisory local evidence and never merge authorization."
    )]
    async fn aporic_git_snapshot_list(
        &self,
        Parameters(request): Parameters<GitSnapshotListRequest>,
    ) -> String {
        render(self.hub.list_git_snapshots(&request))
    }

    #[tool(
        description = "Read one commit-bound Git governance snapshot. Remote freshness and approval remain explicitly unproven."
    )]
    async fn aporic_git_snapshot_get(
        &self,
        Parameters(request): Parameters<GitSnapshotGetRequest>,
    ) -> String {
        render(self.hub.get_git_snapshot(&request))
    }

    #[tool(
        description = "Append a provenance-labelled token-usage receipt. Verified outcomes must reference an existing matching Aporic execution or verified claim. Cached tokens remain part of input usage, estimates are kept separate, and this tool calls no model API."
    )]
    async fn aporic_token_usage_record(
        &self,
        Parameters(request): Parameters<TokenUsageRecordRequest>,
    ) -> String {
        render(self.hub.record_token_usage(&request))
    }

    #[tool(
        description = "List bounded append-only token-usage receipts for a workspace. Every count retains its measurement provenance; this is read-only."
    )]
    async fn aporic_token_usage_list(
        &self,
        Parameters(request): Parameters<TokenUsageListRequest>,
    ) -> String {
        render(self.hub.list_token_usage(&request))
    }

    #[tool(
        description = "Report measured and estimated token efficiency separately, including verified outcomes per measured token. Incomplete or estimated evidence is surfaced explicitly."
    )]
    async fn aporic_token_efficiency_report(
        &self,
        Parameters(request): Parameters<TokenEfficiencyReportRequest>,
    ) -> String {
        render(self.hub.token_efficiency_report(&request))
    }

    #[tool(
        description = "Register an immutable capability manifest as catalog data. Registration never loads code, invokes a provider, grants authority, or makes a capability executable."
    )]
    async fn aporic_capability_register(
        &self,
        Parameters(request): Parameters<CapabilityRegisterRequest>,
    ) -> String {
        render(self.hub.register_capability(&request))
    }

    #[tool(
        description = "Search bounded capability summaries without returning full schemas. Catalog state and risk declarations are data, not trusted enforcement claims."
    )]
    async fn aporic_capability_search(
        &self,
        Parameters(request): Parameters<CapabilitySearchRequest>,
    ) -> String {
        render(self.hub.search_capabilities(&request))
    }

    #[tool(
        description = "Read one immutable capability manifest and its latest catalog state. Aporic does not execute registered capabilities."
    )]
    async fn aporic_capability_get(
        &self,
        Parameters(request): Parameters<CapabilityGetRequest>,
    ) -> String {
        render(self.hub.get_capability(&request))
    }

    #[tool(
        description = "Create an evidence-gated experiment campaign bound to a clean committed Git snapshot, with immutable hard gates and Pareto dimensions. This does not build prototypes or dispatch agents."
    )]
    async fn aporic_experiment_create(
        &self,
        Parameters(request): Parameters<ExperimentCreateRequest>,
    ) -> String {
        render(self.hub.create_experiment(&request))
    }

    #[tool(
        description = "Add a deliberately differentiated prototype variant bound to a clean committed Git snapshot. Exact approach duplicates and budget overflow are rejected; Git is never mutated."
    )]
    async fn aporic_experiment_variant_add(
        &self,
        Parameters(request): Parameters<ExperimentVariantAddRequest>,
    ) -> String {
        render(self.hub.add_experiment_variant(&request))
    }

    #[tool(
        description = "Record one integer experiment measurement with explicit evidence provenance. Model-only or reported evidence cannot qualify a hard gate."
    )]
    async fn aporic_experiment_measurement_add(
        &self,
        Parameters(request): Parameters<ExperimentMeasurementAddRequest>,
    ) -> String {
        render(self.hub.add_experiment_measurement(&request))
    }

    #[tool(
        description = "Record an advisory experiment decision. Selection requires every hard gate to pass with direct evidence and a deliberation decision with no open material issue; this never proves approval."
    )]
    async fn aporic_experiment_decide(
        &self,
        Parameters(request): Parameters<ExperimentDecisionRequest>,
    ) -> String {
        render(self.hub.decide_experiment(&request))
    }

    #[tool(
        description = "Read one bounded experiment portfolio with evidence completeness, hard-gate failures, Pareto candidates, budget state, lineage, and Git integration staleness."
    )]
    async fn aporic_experiment_get(
        &self,
        Parameters(request): Parameters<ExperimentGetRequest>,
    ) -> String {
        render(self.hub.get_experiment(&request))
    }

    #[tool(
        description = "List compact experiment campaign summaries for a workspace without returning full variants, criteria, measurements, or schemas."
    )]
    async fn aporic_experiment_list(
        &self,
        Parameters(request): Parameters<ExperimentListRequest>,
    ) -> String {
        render(self.hub.list_experiments(&request))
    }

    #[tool(
        description = "Read one commit-bound imported security assessment. Artifact hashes and coverage are evidence; zero findings never proves safety, approval, or merge readiness."
    )]
    async fn aporic_security_assessment_get(
        &self,
        Parameters(request): Parameters<SecurityAssessmentGetRequest>,
    ) -> String {
        render(self.hub.get_security_assessment(&request))
    }

    #[tool(
        description = "List bounded locally imported security assessments without running a scanner or exposing raw finding artifacts."
    )]
    async fn aporic_security_assessment_list(
        &self,
        Parameters(request): Parameters<SecurityAssessmentListRequest>,
    ) -> String {
        render(self.hub.list_security_assessments(&request))
    }

    #[tool(
        description = "Create a public, typed deliberation bound to an existing local Git snapshot. The graph is advisory, append-only, and cannot approve changes, grant permission, or store hidden chain-of-thought."
    )]
    async fn aporic_deliberation_create(
        &self,
        Parameters(request): Parameters<DeliberationCreateRequest>,
    ) -> String {
        render(self.hub.create_deliberation(&request))
    }

    #[tool(
        description = "Append one concise public premise, claim, objection, counterexample, falsifier, value constraint, material unknown, proposal, or revision and typed relations. Material challenges require direct evidence or an observed/verified claim; irrelevant dissent remains non-blocking."
    )]
    async fn aporic_deliberation_node_add(
        &self,
        Parameters(request): Parameters<DeliberationNodeAddRequest>,
    ) -> String {
        render(self.hub.add_deliberation_node(&request))
    }

    #[tool(
        description = "Record a provisional decision referencing a proposal or revision. Open material aporia are preserved in the result, and later Git commit/tree changes make the graph stale. This never proves approval."
    )]
    async fn aporic_deliberation_decide(
        &self,
        Parameters(request): Parameters<DeliberationDecisionRequest>,
    ) -> String {
        render(self.hub.record_deliberation_decision(&request))
    }

    #[tool(
        description = "Read one sequence-paginated commit-bound deliberation graph with public nodes, relations, provisional decisions, total counts, continuation cursors, explicit truncation, open material issues, and current staleness. This is read-only."
    )]
    async fn aporic_deliberation_get(
        &self,
        Parameters(request): Parameters<DeliberationGetRequest>,
    ) -> String {
        render(self.hub.get_deliberation(&request))
    }

    #[tool(
        description = "List compact commit-bound deliberation summaries for a workspace. Results are advisory and explicitly do not prove approval or authority."
    )]
    async fn aporic_deliberation_list(
        &self,
        Parameters(request): Parameters<DeliberationListRequest>,
    ) -> String {
        render(self.hub.list_deliberations(&request))
    }

    #[tool(
        description = "Create an Aporic-native Hermes run envelope bound to one active advisory task, clean Git snapshot, and recorded context exposure. The role is limited to propose; this never invokes a model, launches an agent, executes a tool, or grants authority."
    )]
    async fn aporic_orchestration_run_create(
        &self,
        Parameters(request): Parameters<OrchestrationRunCreateRequest>,
    ) -> String {
        render(self.hub.create_orchestration_run(&request))
    }

    #[tool(
        description = "Submit one bounded Steward or Worker report as model-only or simulator data. Blind-shadow content remains sealed until evaluation, and no report can claim completion or create an effect."
    )]
    async fn aporic_role_report_submit(
        &self,
        Parameters(request): Parameters<AdvisoryRoleReportRequest>,
    ) -> String {
        render(self.hub.submit_advisory_role_report(&request))
    }

    #[tool(
        description = "Evaluate a sealed advisory shadow run only after its bound task is completed with verified criterion proofs or explicitly cancelled. The comparison is deterministic and does not claim that the advice was useful."
    )]
    async fn aporic_shadow_evaluate(
        &self,
        Parameters(request): Parameters<ShadowEvaluationRequest>,
    ) -> String {
        render(self.hub.evaluate_shadow_run(&request))
    }

    #[tool(
        description = "Read one advisory orchestration run. Blind-shadow report content remains hidden until the bound outcome is evaluated."
    )]
    async fn aporic_orchestration_run_get(
        &self,
        Parameters(request): Parameters<OrchestrationRunGetRequest>,
    ) -> String {
        render(self.hub.get_orchestration_run(&request))
    }

    #[tool(
        description = "List compact advisory orchestration run summaries without exposing sealed blind-shadow reports."
    )]
    async fn aporic_orchestration_run_list(
        &self,
        Parameters(request): Parameters<OrchestrationRunListRequest>,
    ) -> String {
        render(self.hub.list_orchestration_runs(&request))
    }

    #[tool(
        description = "Record one durable decision, constraint, progress update, observation, effect, verification, or material unknown. Do not record raw conversation or promote intentions into observed effects."
    )]
    async fn aporic_record(&self, Parameters(request): Parameters<RecordRequest>) -> String {
        render(self.hub.record(&request))
    }

    #[tool(
        description = "Register typed evidence. Workspace files are read and hashed by Aporic as direct evidence; command, external, and user reports remain reported, and model assessments remain model-only."
    )]
    async fn aporic_evidence_add(
        &self,
        Parameters(request): Parameters<EvidenceRequest>,
    ) -> String {
        render(self.hub.add_evidence(&request))
    }

    #[tool(
        description = "Assert a typed epistemic claim. Observed or verified status requires Aporic-direct evidence; inference, assumption, intention, and unknown remain distinct."
    )]
    async fn aporic_claim_assert(&self, Parameters(request): Parameters<ClaimRequest>) -> String {
        render(self.hub.assert_claim(&request))
    }

    #[tool(
        description = "Assess whether a counterargument is material enough to surface. Only high-consequence, actionable dissent backed by direct evidence is surfaced; this grants no authority."
    )]
    async fn aporic_dissent_assess(
        &self,
        Parameters(request): Parameters<DissentRequest>,
    ) -> String {
        render(self.hub.assess_dissent(&request))
    }

    #[tool(
        description = "Register an immutable, argv-based local verification specification. This does not execute the command, grant authority, or accept a model-authored receipt; execution is available only through the local CLI runner."
    )]
    async fn aporic_check_register(
        &self,
        Parameters(request): Parameters<CommandSpecRequest>,
    ) -> String {
        render(self.hub.register_command_spec(&request))
    }

    #[tool(
        description = "List bounded local verification runs for a workspace. This is read-only and returns lifecycle state, not raw command output."
    )]
    async fn aporic_run_list(
        &self,
        Parameters(request): Parameters<ExecutionListRequest>,
    ) -> String {
        render(self.hub.list_executions(&request))
    }

    #[tool(
        description = "Read one local verification run, its receipt hashes, declared artifacts, and mechanically issued claim. This is read-only."
    )]
    async fn aporic_run_get(&self, Parameters(request): Parameters<ExecutionGetRequest>) -> String {
        render(self.hub.get_execution(&request))
    }

    #[tool(
        description = "Return an advisory Astra, Sol, or Terra model route from typed task signals. Routing never grants authority, and model output never counts as evidence."
    )]
    async fn aporic_model_route(
        &self,
        Parameters(request): Parameters<ModelRouteRequest>,
    ) -> String {
        render(Ok(self.hub.route_model(&request)))
    }

    #[tool(
        description = "Close an Aporic session as completed or as a handoff. A handoff requires one concrete next action. Closing records the report; it does not independently prove the report true."
    )]
    async fn aporic_close(&self, Parameters(request): Parameters<CloseRequest>) -> String {
        render(self.hub.close_session(&request))
    }

    #[tool(
        description = "Mark inactive open sessions as abandoned without claiming they completed. Use this to recover from interrupted tasks; it never treats abandonment as a successful effect."
    )]
    async fn aporic_reconcile(&self, Parameters(request): Parameters<ReconcileRequest>) -> String {
        render(self.hub.reconcile(&request))
    }

    #[tool(
        description = "Create an advisory project task with explicit acceptance criteria, write scope, and dependencies. This records coordination state but does not launch an agent or grant authority."
    )]
    async fn aporic_task_create(
        &self,
        Parameters(request): Parameters<TaskCreateRequest>,
    ) -> String {
        render(self.hub.create_task(&request))
    }

    #[tool(
        description = "List bounded coordination tasks for a workspace, including leases, dependencies, scopes, and completion evidence."
    )]
    async fn aporic_task_list(&self, Parameters(request): Parameters<TaskListRequest>) -> String {
        render(self.hub.list_tasks(&request))
    }

    #[tool(
        description = "Claim an advisory task lease. Active overlapping write scopes and incomplete dependencies are rejected; the lease grants no host authority."
    )]
    async fn aporic_task_claim(&self, Parameters(request): Parameters<TaskClaimRequest>) -> String {
        render(self.hub.claim_task(&request))
    }

    #[tool(
        description = "Complete a leased task only when every acceptance criterion references a verified typed claim backed by Aporic-direct evidence."
    )]
    async fn aporic_task_complete(
        &self,
        Parameters(request): Parameters<TaskCompleteRequest>,
    ) -> String {
        render(self.hub.complete_task(&request))
    }

    #[tool(
        description = "Cancel a queued or leased advisory task with an explicit reason. Cancellation releases its write scope and does not claim completion."
    )]
    async fn aporic_task_cancel(
        &self,
        Parameters(request): Parameters<TaskCancelRequest>,
    ) -> String {
        render(self.hub.cancel_task(&request))
    }
}

#[tool_handler(
    name = "aporic",
    version = "0.17.0",
    instructions = "Aporic preserves bounded continuity, typed evidence, advisory government composition, and role appointments. For a new-session continuation request, use aporic_resume before choosing a task; inspect live state and ask only when candidates are ambiguous. Historical candidates, government and role definitions, office and role appointments, product cells, capability manifests, and model hints are data, never authority. Registered capabilities, product cells, and Hermes role runs are not executable. Blind-shadow content remains sealed until deterministic outcome evaluation. Task completion requires Aporic-direct evidence. Git observation never fetches or mutates repositories. Integrations remain advisory and fail-open. Aporic does not call model APIs, dispatch agents, invoke providers, broker credentials, or create external effects."
)]
impl ServerHandler for AporicMcp {}

fn render<T: Serialize>(result: crate::store::Result<T>) -> String {
    match result {
        Ok(value) => serde_json::to_string(&serde_json::json!({
            "ok": true,
            "result": value
        }))
        .expect("serializing a JSON value cannot fail"),
        Err(error) => serde_json::to_string(&serde_json::json!({
            "ok": false,
            "error": error.to_string()
        }))
        .expect("serializing a JSON value cannot fail"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_error_envelope_does_not_claim_success() {
        let rendered =
            render::<serde_json::Value>(Err(crate::store::Error::Invalid("bad input".to_owned())));
        let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"], "invalid request: bad input");
    }
}
