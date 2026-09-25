use std::path::PathBuf;

use crate::{
    domain::{
        CloseOutcome, CloseRequest, ContextCapsule, OpenOutcome, OpenRequest, RecallRequest,
        RecordOutcome, RecordRequest,
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

    pub fn event_count(&self) -> Result<u64> {
        self.store.event_count()
    }
}
