//! Applies a loaded role profile before reserving exact one-shot authority.
//!
//! The boundary maps actions to capabilities using trusted host configuration,
//! enforces effective role limits, then delegates the atomic reservation to the
//! deterministic kernel. It does not interpret role instructions.

use aporic_kernel::{CommitRequest, CommitStatus, Event, Reservation, commit, load};
use aporic_roles::{ExecutionProfile, RoutingTier};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

#[derive(Debug)]
pub enum Error {
    Kernel(aporic_kernel::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Kernel(error) => write!(formatter, "kernel error: {error}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<aporic_kernel::Error> for Error {
    fn from(error: aporic_kernel::Error) -> Self {
        Self::Kernel(error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRule {
    pub capability: String,
    pub delegates: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BoundaryPolicy {
    pub actions: BTreeMap<String, ActionRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReserveRequest {
    pub event_id: String,
    pub idempotency_key: String,
    pub expected_revision: u64,
    pub reservation_id: String,
    pub grant_id: String,
    pub principal: String,
    pub task_ref: String,
    pub session_ref: String,
    pub scope: String,
    pub action: String,
    pub input: Value,
    pub selected_routing: RoutingTier,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdmissionOutcome {
    pub status: CommitStatus,
    pub revision: u64,
    pub reason_code: String,
}

pub fn reserve(
    store: impl AsRef<Path>,
    profile: &ExecutionProfile,
    policy: &BoundaryPolicy,
    request: ReserveRequest,
) -> Result<AdmissionOutcome> {
    let store = store.as_ref();
    let kernel_request = CommitRequest::new(
        request.event_id.clone(),
        request.idempotency_key.clone(),
        request.expected_revision,
        Event::ActionReserved {
            reservation: Reservation {
                reservation_id: request.reservation_id.clone(),
                grant_id: request.grant_id.clone(),
                principal: request.principal.clone(),
                task_ref: request.task_ref.clone(),
                session_ref: request.session_ref.clone(),
                profile_ref: profile.profile_sha256().into(),
                scope: request.scope.clone(),
                action: request.action.clone(),
                input: request.input.clone(),
            },
        },
    );
    let ledger = load(store)?;
    let state = ledger.state();

    if let Some(existing) = ledger
        .records()
        .iter()
        .find(|record| record.request.idempotency_key == request.idempotency_key)
    {
        return if existing.request == kernel_request {
            Ok(AdmissionOutcome {
                status: CommitStatus::Duplicate,
                revision: existing.sequence,
                reason_code: "DUPLICATE".into(),
            })
        } else {
            Ok(rejected(state.revision, "IDEMPOTENCY_KEY_CONFLICT"))
        };
    }
    if ledger
        .records()
        .iter()
        .any(|record| record.request.event_id == request.event_id)
    {
        return Ok(rejected(state.revision, "EVENT_ID_CONFLICT"));
    }
    if request.expected_revision != state.revision {
        return Ok(rejected(state.revision, "STALE_REVISION"));
    }
    let Some(rule) = policy.actions.get(&request.action) else {
        return Ok(rejected(state.revision, "ACTION_NOT_MAPPED"));
    };
    if profile
        .capabilities()
        .binary_search(&rule.capability)
        .is_err()
    {
        return Ok(rejected(state.revision, "CAPABILITY_NOT_ALLOWED"));
    }
    if request.selected_routing > profile.routing_ceiling() {
        return Ok(rejected(state.revision, "ROUTING_CEILING_EXCEEDED"));
    }
    if rule.delegates && !profile.may_delegate() {
        return Ok(rejected(state.revision, "DELEGATION_NOT_ALLOWED"));
    }

    let same_day = state.reservations.values().filter(|candidate| {
        candidate.reservation.principal == request.principal
            && candidate.reservation.session_ref == request.session_ref
            && candidate.reservation.profile_ref == profile.profile_sha256()
    });
    let tool_calls = same_day.clone().count();
    if tool_calls >= profile.max_tool_calls() as usize {
        return Ok(rejected(state.revision, "TOOL_CALL_LIMIT_REACHED"));
    }
    let active = same_day
        .filter(|candidate| candidate.effect.is_none() && candidate.abandonment_reason.is_none())
        .count();
    if active >= profile.max_parallel_tasks() as usize {
        return Ok(rejected(state.revision, "PARALLEL_LIMIT_REACHED"));
    }
    drop(ledger);

    let outcome = commit(store, kernel_request)?;
    Ok(AdmissionOutcome {
        status: outcome.status,
        revision: outcome.revision,
        reason_code: outcome.reason_code,
    })
}

fn rejected(revision: u64, reason_code: &str) -> AdmissionOutcome {
    AdmissionOutcome {
        status: CommitStatus::Rejected,
        revision,
        reason_code: reason_code.into(),
    }
}
