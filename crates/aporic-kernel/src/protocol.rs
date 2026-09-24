use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_EPOCH: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub grant_id: String,
    pub principal: String,
    pub task_ref: String,
    pub session_ref: String,
    pub profile_ref: String,
    pub scope: String,
    pub action: String,
    pub input: Value,
    pub authority_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reservation {
    pub reservation_id: String,
    pub grant_id: String,
    pub principal: String,
    pub task_ref: String,
    pub session_ref: String,
    pub profile_ref: String,
    pub scope: String,
    pub action: String,
    pub input: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectOutcome {
    Succeeded,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationResult {
    Passed,
    Failed,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    AuthorityGranted {
        grant: Grant,
    },
    ActionReserved {
        reservation: Reservation,
    },
    EffectRecorded {
        reservation_id: String,
        outcome: EffectOutcome,
        observation_ref: String,
    },
    VerificationRecorded {
        reservation_id: String,
        verifier: String,
        result: VerificationResult,
        evidence_ref: String,
    },
    ReservationAbandoned {
        reservation_id: String,
        reason: String,
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
