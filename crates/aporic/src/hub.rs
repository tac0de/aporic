use std::path::PathBuf;

use crate::{
    domain::{
        CloseOutcome, CloseRequest, ContextCapsule, CoordinatedTask, HubStats, OpenOutcome,
        OpenRequest, ProjectExport, RecallRequest, ReconcileOutcome, ReconcileRequest,
        RecordOutcome, RecordRequest, TaskCancelRequest, TaskClaimRequest, TaskCompleteRequest,
        TaskCreateRequest, TaskListRequest, TaskOutcome,
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

    pub fn close_session(&self, request: &CloseRequest) -> Result<CloseOutcome> {
        self.store.close_session(request)
    }

    pub fn reconcile(&self, request: &ReconcileRequest) -> Result<ReconcileOutcome> {
        self.store.reconcile(request)
    }

    pub fn event_count(&self) -> Result<u64> {
        self.store.event_count()
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
