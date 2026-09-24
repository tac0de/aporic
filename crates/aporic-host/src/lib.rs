//! Minimal host boundary joining project, role, admission, and kernel slices.
//!
//! The four route handles are deliberately separate so a concrete host can
//! expose only the surface appropriate to each caller. This crate does not
//! authenticate callers or execute tools itself.

use aporic_boundary::{
    AdmissionOutcome, BoundaryPolicy, ReserveRequest as BoundaryReserveRequest, reserve,
};
use aporic_handoff::{
    CommitOutcome as HandoffOutcome, CommitRequest as HandoffCommitRequest, Day,
    Event as HandoffEvent, HandoffCapsule, HandoffRef, WorkspaceCheckpoint,
    commit as commit_handoff, handoff_sha256, load as load_handoffs,
};
use aporic_kernel::{
    CommitOutcome, CommitRequest, CommitStatus, EffectOutcome, Event, Grant, VerificationResult,
    commit, load,
};
use aporic_projects::{ProjectBinding, ProjectObservation, load_project, observe_project};
use aporic_roles::{ExecutionProfile, HostPolicy, load_role};
use aporic_routing::{RoutingDecision, RoutingSignals, select as select_routing};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    Boundary(aporic_boundary::Error),
    Handoff(aporic_handoff::Error),
    Io(std::io::Error),
    Kernel(aporic_kernel::Error),
    Project(aporic_projects::Error),
    Role(aporic_roles::Error),
    InvalidConfiguration(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Boundary(error) => write!(formatter, "boundary error: {error}"),
            Self::Handoff(error) => write!(formatter, "handoff error: {error}"),
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Kernel(error) => write!(formatter, "kernel error: {error}"),
            Self::Project(error) => write!(formatter, "project error: {error}"),
            Self::Role(error) => write!(formatter, "role error: {error}"),
            Self::InvalidConfiguration(reason) => {
                write!(formatter, "invalid host configuration: {reason}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<aporic_boundary::Error> for Error {
    fn from(error: aporic_boundary::Error) -> Self {
        Self::Boundary(error)
    }
}

impl From<aporic_handoff::Error> for Error {
    fn from(error: aporic_handoff::Error) -> Self {
        Self::Handoff(error)
    }
}

impl From<aporic_kernel::Error> for Error {
    fn from(error: aporic_kernel::Error) -> Self {
        Self::Kernel(error)
    }
}

impl From<aporic_projects::Error> for Error {
    fn from(error: aporic_projects::Error) -> Self {
        Self::Project(error)
    }
}

impl From<aporic_roles::Error> for Error {
    fn from(error: aporic_roles::Error) -> Self {
        Self::Role(error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct ConnectRequest {
    pub registry: PathBuf,
    pub project_id: String,
    pub role_directory: PathBuf,
    pub kernel_store: PathBuf,
    pub handoff_store: PathBuf,
    pub host_policy: HostPolicy,
    pub boundary_policy: BoundaryPolicy,
}

#[derive(Debug)]
pub struct HostRuntime {
    binding: ProjectBinding,
    profile: ExecutionProfile,
    boundary_policy: BoundaryPolicy,
    kernel_store: PathBuf,
    handoff_store: PathBuf,
    scope: String,
}

impl HostRuntime {
    pub fn binding(&self) -> &ProjectBinding {
        &self.binding
    }

    pub fn profile(&self) -> &ExecutionProfile {
        &self.profile
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }

    pub fn observe(&self) -> Result<ProjectObservation> {
        Ok(observe_project(&self.binding)?)
    }

    pub fn control(&self) -> ControlPlane<'_> {
        ControlPlane { runtime: self }
    }

    pub fn agent(&self) -> AgentPlane<'_> {
        AgentPlane { runtime: self }
    }

    pub fn effects(&self) -> EffectPlane<'_> {
        EffectPlane { runtime: self }
    }

    pub fn verifier(&self) -> VerifierPlane<'_> {
        VerifierPlane { runtime: self }
    }

    pub fn continuity(&self) -> ContinuityPlane<'_> {
        ContinuityPlane { runtime: self }
    }

    pub fn route(&self, signals: &RoutingSignals) -> RoutingDecision {
        select_routing(
            self.profile.default_routing(),
            self.profile.routing_ceiling(),
            signals,
        )
    }
}

pub fn connect(request: ConnectRequest) -> Result<HostRuntime> {
    let binding = load_project(&request.registry, &request.project_id)?;
    observe_project(&binding)?;
    let profile = load_role(&request.role_directory, &request.host_policy)?;
    if !request.kernel_store.is_absolute() {
        return Err(Error::InvalidConfiguration(
            "kernel store path must be absolute",
        ));
    }
    let metadata = std::fs::symlink_metadata(&request.kernel_store)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Error::InvalidConfiguration(
            "kernel store must be a real regular file",
        ));
    }
    let kernel_store = std::fs::canonicalize(&request.kernel_store)?;
    if kernel_store.starts_with(binding.workspace()) {
        return Err(Error::InvalidConfiguration(
            "kernel store must remain outside the governed workspace",
        ));
    }
    load(&kernel_store)?;
    if !request.handoff_store.is_absolute() {
        return Err(Error::InvalidConfiguration(
            "handoff store path must be absolute",
        ));
    }
    let metadata = std::fs::symlink_metadata(&request.handoff_store)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Error::InvalidConfiguration(
            "handoff store must be a real regular file",
        ));
    }
    let handoff_store = std::fs::canonicalize(&request.handoff_store)?;
    if handoff_store.starts_with(binding.workspace()) {
        return Err(Error::InvalidConfiguration(
            "handoff store must remain outside the governed workspace",
        ));
    }
    if handoff_store == kernel_store {
        return Err(Error::InvalidConfiguration(
            "kernel and handoff stores must be distinct",
        ));
    }
    load_handoffs(&handoff_store)?;
    let scope = format!("project:{}", binding.binding_sha256());
    Ok(HostRuntime {
        binding,
        profile,
        boundary_policy: request.boundary_policy,
        kernel_store,
        handoff_store,
        scope,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionIdentity {
    pub principal: String,
    pub task_ref: String,
    pub session_ref: String,
    pub action: String,
    pub input: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantRequest {
    pub event_id: String,
    pub idempotency_key: String,
    pub expected_revision: u64,
    pub grant_id: String,
    pub action: ActionIdentity,
    pub authority_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReserveRequest {
    pub event_id: String,
    pub idempotency_key: String,
    pub expected_revision: u64,
    pub reservation_id: String,
    pub grant_id: String,
    pub action: ActionIdentity,
    pub routing_signals: RoutingSignals,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectRequest {
    pub event_id: String,
    pub idempotency_key: String,
    pub expected_revision: u64,
    pub reservation_id: String,
    pub outcome: EffectOutcome,
    pub observation_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationRequest {
    pub event_id: String,
    pub idempotency_key: String,
    pub expected_revision: u64,
    pub reservation_id: String,
    pub verifier: String,
    pub result: VerificationResult,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbandonRequest {
    pub event_id: String,
    pub idempotency_key: String,
    pub expected_revision: u64,
    pub reservation_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationOutcome {
    pub status: CommitStatus,
    pub revision: u64,
    pub reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenDayRequest {
    pub event_id: String,
    pub idempotency_key: String,
    pub expected_revision: u64,
    pub day_id: String,
    pub lineage_ref: String,
    pub task_ref: String,
    pub session_ref: String,
    pub predecessor: Option<HandoffRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectHandoffRequest {
    pub event_id: String,
    pub idempotency_key: String,
    pub expected_revision: u64,
    pub day_id: String,
    pub capsule: HandoffCapsule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloseDayRequest {
    pub event_id: String,
    pub idempotency_key: String,
    pub expected_revision: u64,
    pub day_id: String,
    pub handoff_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectedHandoffOutcome {
    pub commit: HandoffOutcome,
    pub handoff_sha256: String,
}

pub struct ContinuityPlane<'a> {
    runtime: &'a HostRuntime,
}

impl ContinuityPlane<'_> {
    pub fn open(&self, request: OpenDayRequest) -> Result<HandoffOutcome> {
        let workspace = self.workspace_checkpoint()?;
        Ok(commit_handoff(
            &self.runtime.handoff_store,
            HandoffCommitRequest::new(
                request.event_id,
                request.idempotency_key,
                request.expected_revision,
                HandoffEvent::DayOpened {
                    day: Day {
                        day_id: request.day_id,
                        lineage_ref: request.lineage_ref,
                        task_ref: request.task_ref,
                        session_ref: request.session_ref,
                        scope: self.runtime.scope.clone(),
                        predecessor: request.predecessor,
                    },
                    workspace,
                },
            ),
        )?)
    }

    pub fn project(&self, request: ProjectHandoffRequest) -> Result<ProjectedHandoffOutcome> {
        let workspace = self.workspace_checkpoint()?;
        let ledger = load_handoffs(&self.runtime.handoff_store)?;
        let day = ledger
            .state()
            .days
            .get(&request.day_id)
            .ok_or(Error::InvalidConfiguration(
                "day does not belong to this runtime",
            ))?;
        if day.day.scope != self.runtime.scope {
            return Err(Error::InvalidConfiguration(
                "day does not belong to this runtime",
            ));
        }
        let digest = handoff_sha256(&day.day, &request.capsule, &workspace)?;
        let outcome = commit_handoff(
            &self.runtime.handoff_store,
            HandoffCommitRequest::new(
                request.event_id,
                request.idempotency_key,
                request.expected_revision,
                HandoffEvent::HandoffProjected {
                    day_id: request.day_id,
                    capsule: request.capsule,
                    workspace,
                    handoff_sha256: digest.clone(),
                },
            ),
        )?;
        Ok(ProjectedHandoffOutcome {
            commit: outcome,
            handoff_sha256: digest,
        })
    }

    pub fn close(&self, request: CloseDayRequest) -> Result<HandoffOutcome> {
        let workspace = self.workspace_checkpoint()?;
        Ok(commit_handoff(
            &self.runtime.handoff_store,
            HandoffCommitRequest::new(
                request.event_id,
                request.idempotency_key,
                request.expected_revision,
                HandoffEvent::DayClosed {
                    day_id: request.day_id,
                    handoff_sha256: request.handoff_sha256,
                    workspace,
                },
            ),
        )?)
    }

    fn workspace_checkpoint(&self) -> Result<WorkspaceCheckpoint> {
        let observation = observe_project(&self.runtime.binding)?;
        Ok(WorkspaceCheckpoint {
            binding_sha256: observation.binding_sha256,
            head: observation.head,
            dirty: observation.dirty,
        })
    }
}

pub struct ControlPlane<'a> {
    runtime: &'a HostRuntime,
}

impl ControlPlane<'_> {
    pub fn grant(&self, request: GrantRequest) -> Result<OperationOutcome> {
        let kernel_request = CommitRequest::new(
            request.event_id,
            request.idempotency_key,
            request.expected_revision,
            Event::AuthorityGranted {
                grant: Grant {
                    grant_id: request.grant_id,
                    principal: request.action.principal,
                    task_ref: request.action.task_ref,
                    session_ref: request.action.session_ref,
                    profile_ref: self.runtime.profile.profile_sha256().into(),
                    scope: self.runtime.scope.clone(),
                    action: request.action.action,
                    input: request.action.input,
                    authority_ref: request.authority_ref,
                },
            },
        );
        if let Some(outcome) = prior_outcome(self.runtime, &kernel_request)? {
            return Ok(outcome);
        }
        observe_project(&self.runtime.binding)?;
        let action = match &kernel_request.event {
            Event::AuthorityGranted { grant } => grant.action.as_str(),
            _ => unreachable!("control plane constructs an authority event"),
        };
        if let Some(reason) =
            action_denial(&self.runtime.profile, &self.runtime.boundary_policy, action)
        {
            return current_rejection(self.runtime, reason);
        }
        let outcome = commit(&self.runtime.kernel_store, kernel_request)?;
        Ok(outcome.into())
    }
}

pub struct AgentPlane<'a> {
    runtime: &'a HostRuntime,
}

impl AgentPlane<'_> {
    pub fn reserve(&self, request: ReserveRequest) -> Result<OperationOutcome> {
        let routing = self.runtime.route(&request.routing_signals);
        let boundary_request = BoundaryReserveRequest {
            event_id: request.event_id,
            idempotency_key: request.idempotency_key,
            expected_revision: request.expected_revision,
            reservation_id: request.reservation_id,
            grant_id: request.grant_id,
            principal: request.action.principal,
            task_ref: request.action.task_ref,
            session_ref: request.action.session_ref,
            scope: self.runtime.scope.clone(),
            action: request.action.action,
            input: request.action.input,
            selected_routing: routing.tier,
            routing_reasons: routing.reasons,
        };
        let kernel_request = CommitRequest::new(
            boundary_request.event_id.clone(),
            boundary_request.idempotency_key.clone(),
            boundary_request.expected_revision,
            Event::ActionReserved {
                reservation: aporic_kernel::Reservation {
                    reservation_id: boundary_request.reservation_id.clone(),
                    grant_id: boundary_request.grant_id.clone(),
                    principal: boundary_request.principal.clone(),
                    task_ref: boundary_request.task_ref.clone(),
                    session_ref: boundary_request.session_ref.clone(),
                    profile_ref: self.runtime.profile.profile_sha256().into(),
                    scope: boundary_request.scope.clone(),
                    action: boundary_request.action.clone(),
                    input: boundary_request.input.clone(),
                    routing_tier: format!("{:?}", boundary_request.selected_routing).to_lowercase(),
                    routing_reasons: boundary_request.routing_reasons.clone(),
                },
            },
        );
        if let Some(outcome) = prior_outcome(self.runtime, &kernel_request)? {
            return Ok(outcome);
        }
        observe_project(&self.runtime.binding)?;
        let outcome = reserve(
            &self.runtime.kernel_store,
            &self.runtime.profile,
            &self.runtime.boundary_policy,
            boundary_request,
        )?;
        Ok(outcome.into())
    }
}

pub struct EffectPlane<'a> {
    runtime: &'a HostRuntime,
}

impl EffectPlane<'_> {
    pub fn record(&self, request: EffectRequest) -> Result<OperationOutcome> {
        let reservation_id = request.reservation_id.clone();
        self.commit_for_reservation(
            &reservation_id,
            CommitRequest::new(
                request.event_id,
                request.idempotency_key,
                request.expected_revision,
                Event::EffectRecorded {
                    reservation_id: request.reservation_id,
                    outcome: request.outcome,
                    observation_ref: request.observation_ref,
                },
            ),
        )
    }

    pub fn abandon(&self, request: AbandonRequest) -> Result<OperationOutcome> {
        let reservation_id = request.reservation_id.clone();
        self.commit_for_reservation(
            &reservation_id,
            CommitRequest::new(
                request.event_id,
                request.idempotency_key,
                request.expected_revision,
                Event::ReservationAbandoned {
                    reservation_id: request.reservation_id,
                    reason: request.reason,
                },
            ),
        )
    }

    fn commit_for_reservation(
        &self,
        reservation_id: &str,
        request: CommitRequest,
    ) -> Result<OperationOutcome> {
        ensure_runtime_reservation(self.runtime, reservation_id)?;
        Ok(commit(&self.runtime.kernel_store, request)?.into())
    }
}

pub struct VerifierPlane<'a> {
    runtime: &'a HostRuntime,
}

impl VerifierPlane<'_> {
    pub fn record(&self, request: VerificationRequest) -> Result<OperationOutcome> {
        ensure_runtime_reservation(self.runtime, &request.reservation_id)?;
        let outcome = commit(
            &self.runtime.kernel_store,
            CommitRequest::new(
                request.event_id,
                request.idempotency_key,
                request.expected_revision,
                Event::VerificationRecorded {
                    reservation_id: request.reservation_id,
                    verifier: request.verifier,
                    result: request.result,
                    evidence_ref: request.evidence_ref,
                },
            ),
        )?;
        Ok(outcome.into())
    }
}

fn action_denial(
    profile: &ExecutionProfile,
    policy: &BoundaryPolicy,
    action: &str,
) -> Option<&'static str> {
    let Some(rule) = policy.actions.get(action) else {
        return Some("ACTION_NOT_MAPPED");
    };
    if profile
        .capabilities()
        .binary_search(&rule.capability)
        .is_err()
    {
        return Some("CAPABILITY_NOT_ALLOWED");
    }
    if rule.delegates && !profile.may_delegate() {
        return Some("DELEGATION_NOT_ALLOWED");
    }
    None
}

fn prior_outcome(
    runtime: &HostRuntime,
    request: &CommitRequest,
) -> Result<Option<OperationOutcome>> {
    let ledger = load(&runtime.kernel_store)?;
    if let Some(existing) = ledger
        .records()
        .iter()
        .find(|record| record.request.idempotency_key == request.idempotency_key)
    {
        return if existing.request == *request {
            Ok(Some(OperationOutcome {
                status: CommitStatus::Duplicate,
                revision: existing.sequence,
                reason_code: "DUPLICATE".into(),
            }))
        } else {
            Ok(Some(OperationOutcome {
                status: CommitStatus::Rejected,
                revision: ledger.state().revision,
                reason_code: "IDEMPOTENCY_KEY_CONFLICT".into(),
            }))
        };
    }
    if ledger
        .records()
        .iter()
        .any(|record| record.request.event_id == request.event_id)
    {
        return Ok(Some(OperationOutcome {
            status: CommitStatus::Rejected,
            revision: ledger.state().revision,
            reason_code: "EVENT_ID_CONFLICT".into(),
        }));
    }
    Ok(None)
}

fn ensure_runtime_reservation(runtime: &HostRuntime, reservation_id: &str) -> Result<()> {
    let ledger = load(&runtime.kernel_store)?;
    let Some(reservation) = ledger.state().reservations.get(reservation_id) else {
        return Err(Error::InvalidConfiguration(
            "reservation does not belong to this runtime",
        ));
    };
    if reservation.reservation.scope != runtime.scope
        || reservation.reservation.profile_ref != runtime.profile.profile_sha256()
    {
        return Err(Error::InvalidConfiguration(
            "reservation does not belong to this runtime",
        ));
    }
    Ok(())
}

fn current_rejection(runtime: &HostRuntime, reason_code: &str) -> Result<OperationOutcome> {
    let revision = load(&runtime.kernel_store)?.state().revision;
    Ok(OperationOutcome {
        status: CommitStatus::Rejected,
        revision,
        reason_code: reason_code.into(),
    })
}

impl From<CommitOutcome> for OperationOutcome {
    fn from(outcome: CommitOutcome) -> Self {
        Self {
            status: outcome.status,
            revision: outcome.revision,
            reason_code: outcome.reason_code,
        }
    }
}

impl From<AdmissionOutcome> for OperationOutcome {
    fn from(outcome: AdmissionOutcome) -> Self {
        Self {
            status: outcome.status,
            revision: outcome.revision,
            reason_code: outcome.reason_code,
        }
    }
}
