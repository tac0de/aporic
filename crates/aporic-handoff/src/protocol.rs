use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const PROTOCOL_EPOCH: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceCheckpoint {
    pub binding_sha256: String,
    pub head: String,
    pub dirty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandoffCapsule {
    pub objective: String,
    pub constraints: Vec<String>,
    pub accepted_decisions: Vec<String>,
    pub completed_checks: Vec<String>,
    pub open_questions: Vec<String>,
    pub next_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandoffRef {
    pub day_id: String,
    pub handoff_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Day {
    pub day_id: String,
    pub lineage_ref: String,
    pub task_ref: String,
    pub session_ref: String,
    pub scope: String,
    pub predecessor: Option<HandoffRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    DayOpened {
        day: Day,
        workspace: WorkspaceCheckpoint,
    },
    HandoffProjected {
        day_id: String,
        capsule: HandoffCapsule,
        workspace: WorkspaceCheckpoint,
        handoff_sha256: String,
    },
    DayClosed {
        day_id: String,
        handoff_sha256: String,
        workspace: WorkspaceCheckpoint,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitRequest {
    pub protocol_epoch: u32,
    pub event_id: String,
    pub idempotency_key: String,
    pub expected_revision: u64,
    pub event: Event,
}

impl CommitRequest {
    pub fn new(
        event_id: impl Into<String>,
        idempotency_key: impl Into<String>,
        expected_revision: u64,
        event: Event,
    ) -> Self {
        Self {
            protocol_epoch: PROTOCOL_EPOCH,
            event_id: event_id.into(),
            idempotency_key: idempotency_key.into(),
            expected_revision,
            event,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitStatus {
    Committed,
    Duplicate,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitOutcome {
    pub status: CommitStatus,
    pub revision: u64,
    pub reason_code: String,
}

impl CommitOutcome {
    pub(crate) fn committed(revision: u64) -> Self {
        Self {
            status: CommitStatus::Committed,
            revision,
            reason_code: "COMMITTED".into(),
        }
    }

    pub(crate) fn duplicate(revision: u64) -> Self {
        Self {
            status: CommitStatus::Duplicate,
            revision,
            reason_code: "DUPLICATE".into(),
        }
    }

    pub(crate) fn rejected(revision: u64, reason_code: impl Into<String>) -> Self {
        Self {
            status: CommitStatus::Rejected,
            revision,
            reason_code: reason_code.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredEvent {
    pub sequence: u64,
    pub request: CommitRequest,
}

#[derive(Serialize)]
struct HandoffFingerprint<'a> {
    schema_version: u32,
    day: &'a Day,
    capsule: &'a HandoffCapsule,
    workspace: &'a WorkspaceCheckpoint,
}

pub fn handoff_sha256(
    day: &Day,
    capsule: &HandoffCapsule,
    workspace: &WorkspaceCheckpoint,
) -> crate::Result<String> {
    let encoded = serde_json::to_vec(&HandoffFingerprint {
        schema_version: 1,
        day,
        capsule,
        workspace,
    })?;
    Ok(format!("{:x}", Sha256::digest(encoded)))
}
