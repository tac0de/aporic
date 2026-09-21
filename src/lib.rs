use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

pub mod analysis;
pub mod codex;
pub mod governance;
pub mod policy;
pub mod project;

pub const SCHEMA_VERSION: u32 = 4;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(serde_json::Error),
    CorruptLog { line: usize, reason: String },
    Invariant(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::Json(error) => write!(f, "JSON error: {error}"),
            Self::CorruptLog { line, reason } => {
                write!(f, "corrupt event log at line {line}: {reason}")
            }
            Self::Invariant(reason) => write!(f, "invariant violation: {reason}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    Human,
    Agent,
    Evidence,
    Host,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    pub kind: ActorKind,
    pub id: String,
    pub provenance: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionKind {
    DecisionCommit,
    PlanAuthorize,
    ScopeChange,
    DirectionSupersede,
    RiskAccept,
    CompletionClaim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    UserStatement,
    RepositoryState,
    TestResult,
    RuntimeObservation,
    ExternalSource,
    AgentInference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpistemicStatus {
    Observed,
    Inferred,
    Hypothesized,
    Verified,
    Refuted,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionOutcome {
    Succeeded,
    Failed,
    Interrupted,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationResult {
    Passed,
    Failed,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointState {
    Open,
    Claimed,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArgumentRelationKind {
    Supports,
    Attacks,
    DependsOn,
    Contradicts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionReviewOutcome {
    Confirmed,
    Revised,
    Reversed,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitRequest {
    pub schema_version: u32,
    pub event_id: String,
    pub idempotency_key: String,
    pub expected_revision: u64,
    pub actor: Actor,
    pub scope: String,
    pub event: Event,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    AporiaOpened {
        aporia_id: String,
        question: String,
        blocks: Vec<TransitionKind>,
    },
    AporiaResolved {
        aporia_id: String,
        resolution_ref: String,
    },
    DelegationGranted {
        delegation_id: String,
        grantee: String,
        transition_kinds: Vec<TransitionKind>,
    },
    DelegationRevoked {
        delegation_id: String,
    },
    DecisionCommitted {
        decision_id: String,
        question: String,
        value: String,
        authority_ref: Option<String>,
    },
    DecisionSuperseded {
        decision_id: String,
        replacement_id: String,
    },
    RiskAccepted {
        risk_id: String,
        subject: String,
        review_condition: String,
    },
    ToolHoldPlaced {
        tool_hold_id: String,
        tool_name: String,
        reason: String,
    },
    ToolHoldReleased {
        tool_hold_id: String,
    },
    PlanRegistered {
        plan_id: String,
        objective: String,
        acceptance_checks: Vec<String>,
        unresolved_questions: Vec<String>,
    },
    PlanAuthorized {
        authorization_id: String,
        plan_id: String,
        session_id: String,
        tool_name: String,
        authority_ref: Option<String>,
    },
    PlanAuthorizationRevoked {
        authorization_id: String,
    },
    ExecutionGrantIssued {
        grant_id: String,
        plan_id: String,
        session_id: String,
        tool_name: String,
        tool_input: serde_json::Value,
        max_uses: u32,
        authority_ref: Option<String>,
    },
    ExecutionGrantRevoked {
        grant_id: String,
    },
    ExecutionGrantConsumed {
        grant_id: String,
        tool_use_id: String,
    },
    EvidenceRecorded {
        evidence_id: String,
        kind: EvidenceKind,
        locator: String,
        digest: Option<String>,
    },
    ClaimRecorded {
        claim_id: String,
        statement: String,
        status: EpistemicStatus,
        evidence_refs: Vec<String>,
    },
    ClaimStatusChanged {
        claim_id: String,
        status: EpistemicStatus,
        evidence_refs: Vec<String>,
    },
    ClaimSuperseded {
        claim_id: String,
        replacement_id: String,
    },
    ArgumentRelationRecorded {
        relation_id: String,
        source_claim_id: String,
        target_claim_id: String,
        kind: ArgumentRelationKind,
    },
    ArgumentRelationRetracted {
        relation_id: String,
        evidence_refs: Vec<String>,
    },
    BeliefRevisionRecorded {
        revision_id: String,
        claim_id: String,
        prior_status: EpistemicStatus,
        revised_status: EpistemicStatus,
        trigger_claim_ids: Vec<String>,
        evidence_refs: Vec<String>,
        rationale: String,
    },
    DecisionBasisLinked {
        decision_id: String,
        claim_ids: Vec<String>,
        evidence_refs: Vec<String>,
    },
    DecisionReviewRecorded {
        review_id: String,
        decision_id: String,
        outcome: DecisionReviewOutcome,
        evidence_refs: Vec<String>,
        lessons: Vec<String>,
        follow_up_claim_ids: Vec<String>,
    },
    PlanBasisLinked {
        plan_id: String,
        evidence_refs: Vec<String>,
        assumption_claim_ids: Vec<String>,
    },
    ActionOutcomeRecorded {
        session_id: String,
        tool_name: String,
        tool_use_id: String,
        outcome: ActionOutcome,
        evidence_refs: Vec<String>,
    },
    VerificationRecorded {
        verification_id: String,
        plan_id: String,
        check_index: u32,
        result: VerificationResult,
        evidence_refs: Vec<String>,
    },
    PlanCompleted {
        plan_id: String,
        residual_risk_refs: Vec<String>,
    },
    CheckpointPublished {
        checkpoint_id: String,
        session_id: String,
        objective: String,
        verified_claim_ids: Vec<String>,
        unresolved_claim_ids: Vec<String>,
        aporia_ids: Vec<String>,
        next_checks: Vec<String>,
        artifact_refs: Vec<String>,
    },
    CheckpointClaimed {
        checkpoint_id: String,
        session_id: String,
    },
    CheckpointExpired {
        checkpoint_id: String,
    },
}

impl Event {
    fn transition_kind(&self) -> Option<TransitionKind> {
        match self {
            Self::DecisionCommitted { .. } => Some(TransitionKind::DecisionCommit),
            Self::DecisionSuperseded { .. } => Some(TransitionKind::DirectionSupersede),
            Self::RiskAccepted { .. } => Some(TransitionKind::RiskAccept),
            Self::PlanAuthorized { .. } => Some(TransitionKind::PlanAuthorize),
            Self::PlanCompleted { .. } => Some(TransitionKind::CompletionClaim),
            Self::ToolHoldPlaced { .. }
            | Self::ToolHoldReleased { .. }
            | Self::PlanRegistered { .. }
            | Self::PlanAuthorizationRevoked { .. }
            | Self::ExecutionGrantRevoked { .. }
            | Self::ExecutionGrantConsumed { .. }
            | Self::EvidenceRecorded { .. }
            | Self::ClaimRecorded { .. }
            | Self::ClaimStatusChanged { .. }
            | Self::ClaimSuperseded { .. }
            | Self::ArgumentRelationRecorded { .. }
            | Self::ArgumentRelationRetracted { .. }
            | Self::BeliefRevisionRecorded { .. }
            | Self::DecisionBasisLinked { .. }
            | Self::DecisionReviewRecorded { .. }
            | Self::PlanBasisLinked { .. }
            | Self::ActionOutcomeRecorded { .. }
            | Self::VerificationRecorded { .. }
            | Self::CheckpointPublished { .. }
            | Self::CheckpointClaimed { .. }
            | Self::CheckpointExpired { .. } => None,
            Self::ExecutionGrantIssued { .. } => Some(TransitionKind::PlanAuthorize),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredEvent {
    pub sequence: u64,
    #[serde(flatten)]
    pub request: CommitRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Aporia {
    pub id: String,
    pub question: String,
    pub scope: String,
    pub blocks: Vec<TransitionKind>,
    pub resolution_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Delegation {
    pub id: String,
    pub grantee: String,
    pub scope: String,
    pub transition_kinds: Vec<TransitionKind>,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub id: String,
    pub question: String,
    pub value: String,
    pub scope: String,
    pub authority_ref: Option<String>,
    pub superseded_by: Option<String>,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptedRisk {
    pub id: String,
    pub subject: String,
    pub scope: String,
    pub actor: Actor,
    pub review_condition: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolHold {
    pub id: String,
    pub tool_name: String,
    pub reason: String,
    pub scope: String,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub id: String,
    pub objective: String,
    pub scope: String,
    pub acceptance_checks: Vec<String>,
    pub unresolved_questions: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub assumption_claim_ids: Vec<String>,
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRecord {
    pub id: String,
    pub kind: EvidenceKind,
    pub locator: String,
    pub digest: Option<String>,
    pub scope: String,
    pub actor: Actor,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub id: String,
    pub statement: String,
    pub status: EpistemicStatus,
    pub evidence_refs: Vec<String>,
    pub scope: String,
    pub superseded_by: Option<String>,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArgumentRelation {
    pub id: String,
    pub source_claim_id: String,
    pub target_claim_id: String,
    pub kind: ArgumentRelationKind,
    pub scope: String,
    pub active: bool,
    pub retraction_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeliefRevision {
    pub id: String,
    pub claim_id: String,
    pub prior_status: EpistemicStatus,
    pub revised_status: EpistemicStatus,
    pub trigger_claim_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub rationale: String,
    pub scope: String,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionBasis {
    pub decision_id: String,
    pub claim_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub scope: String,
    pub decision_sequence: u64,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionReview {
    pub id: String,
    pub decision_id: String,
    pub outcome: DecisionReviewOutcome,
    pub evidence_refs: Vec<String>,
    pub lessons: Vec<String>,
    pub follow_up_claim_ids: Vec<String>,
    pub scope: String,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionOutcomeRecord {
    pub session_id: String,
    pub tool_name: String,
    pub tool_use_id: String,
    pub outcome: ActionOutcome,
    pub evidence_refs: Vec<String>,
    pub scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Verification {
    pub id: String,
    pub plan_id: String,
    pub check_index: u32,
    pub result: VerificationResult,
    pub evidence_refs: Vec<String>,
    pub scope: String,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub id: String,
    pub session_id: String,
    pub objective: String,
    pub verified_claim_ids: Vec<String>,
    pub unresolved_claim_ids: Vec<String>,
    pub aporia_ids: Vec<String>,
    pub next_checks: Vec<String>,
    pub artifact_refs: Vec<String>,
    pub scope: String,
    pub state: CheckpointState,
    pub claimed_by_session: Option<String>,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanAuthorization {
    pub id: String,
    pub plan_id: String,
    pub session_id: String,
    pub tool_name: String,
    pub scope: String,
    pub actor: Actor,
    pub authority_ref: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionGrant {
    pub id: String,
    pub plan_id: String,
    pub session_id: String,
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    pub max_uses: u32,
    pub consumed_uses: u32,
    pub scope: String,
    pub actor: Actor,
    pub authority_ref: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub revision: u64,
    pub aporias: BTreeMap<String, Aporia>,
    pub delegations: BTreeMap<String, Delegation>,
    pub decisions: BTreeMap<String, Decision>,
    pub accepted_risks: BTreeMap<String, AcceptedRisk>,
    pub tool_holds: BTreeMap<String, ToolHold>,
    pub plans: BTreeMap<String, Plan>,
    pub plan_authorizations: BTreeMap<String, PlanAuthorization>,
    pub execution_grants: BTreeMap<String, ExecutionGrant>,
    pub consumed_tool_uses: BTreeMap<String, String>,
    pub evidence: BTreeMap<String, EvidenceRecord>,
    pub claims: BTreeMap<String, Claim>,
    pub argument_relations: BTreeMap<String, ArgumentRelation>,
    pub belief_revisions: BTreeMap<String, BeliefRevision>,
    pub decision_bases: BTreeMap<String, DecisionBasis>,
    pub decision_reviews: BTreeMap<String, DecisionReview>,
    pub action_outcomes: BTreeMap<String, ActionOutcomeRecord>,
    pub verifications: BTreeMap<String, Verification>,
    pub checkpoints: BTreeMap<String, Checkpoint>,
}

impl State {
    fn apply(&mut self, stored: &StoredEvent) -> Result<()> {
        if stored.sequence != self.revision + 1 {
            return Err(Error::Invariant(format!(
                "expected sequence {}, got {}",
                self.revision + 1,
                stored.sequence
            )));
        }

        let scope = stored.request.scope.clone();
        match &stored.request.event {
            Event::AporiaOpened {
                aporia_id,
                question,
                blocks,
            } => {
                if self.aporias.contains_key(aporia_id) {
                    return Err(Error::Invariant(format!(
                        "aporia {aporia_id} already exists"
                    )));
                }
                self.aporias.insert(
                    aporia_id.clone(),
                    Aporia {
                        id: aporia_id.clone(),
                        question: question.clone(),
                        scope,
                        blocks: blocks.clone(),
                        resolution_ref: None,
                    },
                );
            }
            Event::AporiaResolved {
                aporia_id,
                resolution_ref,
            } => {
                let aporia = self
                    .aporias
                    .get_mut(aporia_id)
                    .ok_or_else(|| Error::Invariant(format!("unknown aporia {aporia_id}")))?;
                if aporia.resolution_ref.is_some() {
                    return Err(Error::Invariant(format!(
                        "aporia {aporia_id} is already resolved"
                    )));
                }
                aporia.resolution_ref = Some(resolution_ref.clone());
            }
            Event::DelegationGranted {
                delegation_id,
                grantee,
                transition_kinds,
            } => {
                if self.delegations.contains_key(delegation_id) {
                    return Err(Error::Invariant(format!(
                        "delegation {delegation_id} already exists"
                    )));
                }
                self.delegations.insert(
                    delegation_id.clone(),
                    Delegation {
                        id: delegation_id.clone(),
                        grantee: grantee.clone(),
                        scope,
                        transition_kinds: transition_kinds.clone(),
                        active: true,
                    },
                );
            }
            Event::DelegationRevoked { delegation_id } => {
                let delegation = self.delegations.get_mut(delegation_id).ok_or_else(|| {
                    Error::Invariant(format!("unknown delegation {delegation_id}"))
                })?;
                if !delegation.active {
                    return Err(Error::Invariant(format!(
                        "delegation {delegation_id} is already revoked"
                    )));
                }
                delegation.active = false;
            }
            Event::DecisionCommitted {
                decision_id,
                question,
                value,
                authority_ref,
            } => {
                if self.decisions.contains_key(decision_id) {
                    return Err(Error::Invariant(format!(
                        "decision {decision_id} already exists"
                    )));
                }
                self.decisions.insert(
                    decision_id.clone(),
                    Decision {
                        id: decision_id.clone(),
                        question: question.clone(),
                        value: value.clone(),
                        scope,
                        authority_ref: authority_ref.clone(),
                        superseded_by: None,
                        sequence: stored.sequence,
                    },
                );
            }
            Event::DecisionSuperseded {
                decision_id,
                replacement_id,
            } => {
                if !self.decisions.contains_key(replacement_id) {
                    return Err(Error::Invariant(format!(
                        "unknown replacement decision {replacement_id}"
                    )));
                }
                let decision = self
                    .decisions
                    .get_mut(decision_id)
                    .ok_or_else(|| Error::Invariant(format!("unknown decision {decision_id}")))?;
                if decision.superseded_by.is_some() {
                    return Err(Error::Invariant(format!(
                        "decision {decision_id} is already superseded"
                    )));
                }
                decision.superseded_by = Some(replacement_id.clone());
            }
            Event::RiskAccepted {
                risk_id,
                subject,
                review_condition,
            } => {
                if self.accepted_risks.contains_key(risk_id) {
                    return Err(Error::Invariant(format!("risk {risk_id} already exists")));
                }
                self.accepted_risks.insert(
                    risk_id.clone(),
                    AcceptedRisk {
                        id: risk_id.clone(),
                        subject: subject.clone(),
                        scope,
                        actor: stored.request.actor.clone(),
                        review_condition: review_condition.clone(),
                    },
                );
            }
            Event::ToolHoldPlaced {
                tool_hold_id,
                tool_name,
                reason,
            } => {
                if self.tool_holds.contains_key(tool_hold_id) {
                    return Err(Error::Invariant(format!(
                        "tool hold {tool_hold_id} already exists"
                    )));
                }
                self.tool_holds.insert(
                    tool_hold_id.clone(),
                    ToolHold {
                        id: tool_hold_id.clone(),
                        tool_name: tool_name.clone(),
                        reason: reason.clone(),
                        scope,
                        active: true,
                    },
                );
            }
            Event::ToolHoldReleased { tool_hold_id } => {
                let hold = self
                    .tool_holds
                    .get_mut(tool_hold_id)
                    .ok_or_else(|| Error::Invariant(format!("unknown tool hold {tool_hold_id}")))?;
                if !hold.active {
                    return Err(Error::Invariant(format!(
                        "tool hold {tool_hold_id} is already released"
                    )));
                }
                hold.active = false;
            }
            Event::PlanRegistered {
                plan_id,
                objective,
                acceptance_checks,
                unresolved_questions,
            } => {
                if self.plans.contains_key(plan_id) {
                    return Err(Error::Invariant(format!("plan {plan_id} already exists")));
                }
                self.plans.insert(
                    plan_id.clone(),
                    Plan {
                        id: plan_id.clone(),
                        objective: objective.clone(),
                        scope,
                        acceptance_checks: acceptance_checks.clone(),
                        unresolved_questions: unresolved_questions.clone(),
                        evidence_refs: Vec::new(),
                        assumption_claim_ids: Vec::new(),
                        completed: false,
                    },
                );
            }
            Event::PlanAuthorized {
                authorization_id,
                plan_id,
                session_id,
                tool_name,
                authority_ref,
            } => {
                if self.plan_authorizations.contains_key(authorization_id) {
                    return Err(Error::Invariant(format!(
                        "plan authorization {authorization_id} already exists"
                    )));
                }
                self.plan_authorizations.insert(
                    authorization_id.clone(),
                    PlanAuthorization {
                        id: authorization_id.clone(),
                        plan_id: plan_id.clone(),
                        session_id: session_id.clone(),
                        tool_name: tool_name.clone(),
                        scope,
                        actor: stored.request.actor.clone(),
                        authority_ref: authority_ref.clone(),
                        active: true,
                    },
                );
            }
            Event::PlanAuthorizationRevoked { authorization_id } => {
                let authorization = self
                    .plan_authorizations
                    .get_mut(authorization_id)
                    .ok_or_else(|| {
                        Error::Invariant(format!("unknown plan authorization {authorization_id}"))
                    })?;
                if !authorization.active {
                    return Err(Error::Invariant(format!(
                        "plan authorization {authorization_id} is already revoked"
                    )));
                }
                authorization.active = false;
            }
            Event::ExecutionGrantIssued {
                grant_id,
                plan_id,
                session_id,
                tool_name,
                tool_input,
                max_uses,
                authority_ref,
            } => {
                if self.execution_grants.contains_key(grant_id) {
                    return Err(Error::Invariant(format!(
                        "execution grant {grant_id} already exists"
                    )));
                }
                self.execution_grants.insert(
                    grant_id.clone(),
                    ExecutionGrant {
                        id: grant_id.clone(),
                        plan_id: plan_id.clone(),
                        session_id: session_id.clone(),
                        tool_name: tool_name.clone(),
                        tool_input: tool_input.clone(),
                        max_uses: *max_uses,
                        consumed_uses: 0,
                        scope,
                        actor: stored.request.actor.clone(),
                        authority_ref: authority_ref.clone(),
                        active: true,
                    },
                );
            }
            Event::ExecutionGrantRevoked { grant_id } => {
                let grant = self.execution_grants.get_mut(grant_id).ok_or_else(|| {
                    Error::Invariant(format!("unknown execution grant {grant_id}"))
                })?;
                if !grant.active {
                    return Err(Error::Invariant(format!(
                        "execution grant {grant_id} is already inactive"
                    )));
                }
                grant.active = false;
            }
            Event::ExecutionGrantConsumed {
                grant_id,
                tool_use_id,
            } => {
                if self.consumed_tool_uses.contains_key(tool_use_id) {
                    return Err(Error::Invariant(format!(
                        "tool use {tool_use_id} is already consumed"
                    )));
                }
                let grant = self.execution_grants.get_mut(grant_id).ok_or_else(|| {
                    Error::Invariant(format!("unknown execution grant {grant_id}"))
                })?;
                if !grant.active || grant.consumed_uses >= grant.max_uses {
                    return Err(Error::Invariant(format!(
                        "execution grant {grant_id} has no remaining uses"
                    )));
                }
                grant.consumed_uses += 1;
                if grant.consumed_uses == grant.max_uses {
                    grant.active = false;
                }
                self.consumed_tool_uses
                    .insert(tool_use_id.clone(), grant_id.clone());
            }
            Event::EvidenceRecorded {
                evidence_id,
                kind,
                locator,
                digest,
            } => {
                self.evidence.insert(
                    evidence_id.clone(),
                    EvidenceRecord {
                        id: evidence_id.clone(),
                        kind: *kind,
                        locator: locator.clone(),
                        digest: digest.clone(),
                        scope,
                        actor: stored.request.actor.clone(),
                        sequence: stored.sequence,
                    },
                );
            }
            Event::ClaimRecorded {
                claim_id,
                statement,
                status,
                evidence_refs,
            } => {
                self.claims.insert(
                    claim_id.clone(),
                    Claim {
                        id: claim_id.clone(),
                        statement: statement.clone(),
                        status: *status,
                        evidence_refs: evidence_refs.clone(),
                        scope,
                        superseded_by: None,
                        sequence: stored.sequence,
                    },
                );
            }
            Event::ClaimStatusChanged {
                claim_id,
                status,
                evidence_refs,
            } => {
                let claim = self
                    .claims
                    .get_mut(claim_id)
                    .ok_or_else(|| Error::Invariant(format!("unknown claim {claim_id}")))?;
                claim.status = *status;
                for evidence_ref in evidence_refs {
                    if !claim.evidence_refs.contains(evidence_ref) {
                        claim.evidence_refs.push(evidence_ref.clone());
                    }
                }
            }
            Event::ClaimSuperseded {
                claim_id,
                replacement_id,
            } => {
                let claim = self
                    .claims
                    .get_mut(claim_id)
                    .ok_or_else(|| Error::Invariant(format!("unknown claim {claim_id}")))?;
                claim.superseded_by = Some(replacement_id.clone());
            }
            Event::ArgumentRelationRecorded {
                relation_id,
                source_claim_id,
                target_claim_id,
                kind,
            } => {
                self.argument_relations.insert(
                    relation_id.clone(),
                    ArgumentRelation {
                        id: relation_id.clone(),
                        source_claim_id: source_claim_id.clone(),
                        target_claim_id: target_claim_id.clone(),
                        kind: *kind,
                        scope,
                        active: true,
                        retraction_evidence_refs: Vec::new(),
                    },
                );
            }
            Event::ArgumentRelationRetracted {
                relation_id,
                evidence_refs,
            } => {
                let relation = self
                    .argument_relations
                    .get_mut(relation_id)
                    .ok_or_else(|| {
                        Error::Invariant(format!("unknown argument relation {relation_id}"))
                    })?;
                relation.active = false;
                relation.retraction_evidence_refs = evidence_refs.clone();
            }
            Event::BeliefRevisionRecorded {
                revision_id,
                claim_id,
                prior_status,
                revised_status,
                trigger_claim_ids,
                evidence_refs,
                rationale,
            } => {
                let claim = self
                    .claims
                    .get_mut(claim_id)
                    .ok_or_else(|| Error::Invariant(format!("unknown claim {claim_id}")))?;
                claim.status = *revised_status;
                for evidence_ref in evidence_refs {
                    if !claim.evidence_refs.contains(evidence_ref) {
                        claim.evidence_refs.push(evidence_ref.clone());
                    }
                }
                self.belief_revisions.insert(
                    revision_id.clone(),
                    BeliefRevision {
                        id: revision_id.clone(),
                        claim_id: claim_id.clone(),
                        prior_status: *prior_status,
                        revised_status: *revised_status,
                        trigger_claim_ids: trigger_claim_ids.clone(),
                        evidence_refs: evidence_refs.clone(),
                        rationale: rationale.clone(),
                        scope,
                        sequence: stored.sequence,
                    },
                );
            }
            Event::DecisionBasisLinked {
                decision_id,
                claim_ids,
                evidence_refs,
            } => {
                let decision_sequence = self
                    .decisions
                    .get(decision_id)
                    .ok_or_else(|| Error::Invariant(format!("unknown decision {decision_id}")))?
                    .sequence;
                self.decision_bases.insert(
                    decision_id.clone(),
                    DecisionBasis {
                        decision_id: decision_id.clone(),
                        claim_ids: claim_ids.clone(),
                        evidence_refs: evidence_refs.clone(),
                        scope,
                        decision_sequence,
                        sequence: stored.sequence,
                    },
                );
            }
            Event::DecisionReviewRecorded {
                review_id,
                decision_id,
                outcome,
                evidence_refs,
                lessons,
                follow_up_claim_ids,
            } => {
                self.decision_reviews.insert(
                    review_id.clone(),
                    DecisionReview {
                        id: review_id.clone(),
                        decision_id: decision_id.clone(),
                        outcome: *outcome,
                        evidence_refs: evidence_refs.clone(),
                        lessons: lessons.clone(),
                        follow_up_claim_ids: follow_up_claim_ids.clone(),
                        scope,
                        sequence: stored.sequence,
                    },
                );
            }
            Event::PlanBasisLinked {
                plan_id,
                evidence_refs,
                assumption_claim_ids,
            } => {
                let plan = self
                    .plans
                    .get_mut(plan_id)
                    .ok_or_else(|| Error::Invariant(format!("unknown plan {plan_id}")))?;
                plan.evidence_refs = evidence_refs.clone();
                plan.assumption_claim_ids = assumption_claim_ids.clone();
            }
            Event::ActionOutcomeRecorded {
                session_id,
                tool_name,
                tool_use_id,
                outcome,
                evidence_refs,
            } => {
                let outcome_key = action_outcome_key(&scope, session_id, tool_use_id);
                self.action_outcomes.insert(
                    outcome_key,
                    ActionOutcomeRecord {
                        session_id: session_id.clone(),
                        tool_name: tool_name.clone(),
                        tool_use_id: tool_use_id.clone(),
                        outcome: *outcome,
                        evidence_refs: evidence_refs.clone(),
                        scope,
                    },
                );
            }
            Event::VerificationRecorded {
                verification_id,
                plan_id,
                check_index,
                result,
                evidence_refs,
            } => {
                self.verifications.insert(
                    verification_id.clone(),
                    Verification {
                        id: verification_id.clone(),
                        plan_id: plan_id.clone(),
                        check_index: *check_index,
                        result: *result,
                        evidence_refs: evidence_refs.clone(),
                        scope,
                        sequence: stored.sequence,
                    },
                );
            }
            Event::PlanCompleted { plan_id, .. } => {
                let plan = self
                    .plans
                    .get_mut(plan_id)
                    .ok_or_else(|| Error::Invariant(format!("unknown plan {plan_id}")))?;
                plan.completed = true;
            }
            Event::CheckpointPublished {
                checkpoint_id,
                session_id,
                objective,
                verified_claim_ids,
                unresolved_claim_ids,
                aporia_ids,
                next_checks,
                artifact_refs,
            } => {
                self.checkpoints.insert(
                    checkpoint_id.clone(),
                    Checkpoint {
                        id: checkpoint_id.clone(),
                        session_id: session_id.clone(),
                        objective: objective.clone(),
                        verified_claim_ids: verified_claim_ids.clone(),
                        unresolved_claim_ids: unresolved_claim_ids.clone(),
                        aporia_ids: aporia_ids.clone(),
                        next_checks: next_checks.clone(),
                        artifact_refs: artifact_refs.clone(),
                        scope,
                        state: CheckpointState::Open,
                        claimed_by_session: None,
                        sequence: stored.sequence,
                    },
                );
            }
            Event::CheckpointClaimed {
                checkpoint_id,
                session_id,
            } => {
                let checkpoint = self.checkpoints.get_mut(checkpoint_id).ok_or_else(|| {
                    Error::Invariant(format!("unknown checkpoint {checkpoint_id}"))
                })?;
                checkpoint.state = CheckpointState::Claimed;
                checkpoint.claimed_by_session = Some(session_id.clone());
            }
            Event::CheckpointExpired { checkpoint_id } => {
                let checkpoint = self.checkpoints.get_mut(checkpoint_id).ok_or_else(|| {
                    Error::Invariant(format!("unknown checkpoint {checkpoint_id}"))
                })?;
                checkpoint.state = CheckpointState::Expired;
            }
        }

        self.revision = stored.sequence;
        Ok(())
    }
}

fn action_outcome_key(scope: &str, session_id: &str, tool_use_id: &str) -> String {
    serde_json::to_string(&(scope, session_id, tool_use_id))
        .expect("string-triple serialization is infallible")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Allow,
    Clarify,
    Challenge,
    Deny,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evaluation {
    pub verdict: Verdict,
    pub reason_code: String,
    pub message: String,
}

impl Evaluation {
    fn allow() -> Self {
        Self {
            verdict: Verdict::Allow,
            reason_code: "ALLOW".into(),
            message: "transition satisfies the deterministic invariants".into(),
        }
    }

    fn deny(code: &str, message: impl Into<String>) -> Self {
        Self {
            verdict: Verdict::Deny,
            reason_code: code.into(),
            message: message.into(),
        }
    }

    fn clarify(code: &str, message: impl Into<String>) -> Self {
        Self {
            verdict: Verdict::Clarify,
            reason_code: code.into(),
            message: message.into(),
        }
    }
}

pub fn evaluate(state: &State, request: &CommitRequest) -> Evaluation {
    if request.schema_version != SCHEMA_VERSION {
        return Evaluation::deny(
            "UNSUPPORTED_SCHEMA",
            format!("expected schema version {SCHEMA_VERSION}"),
        );
    }
    if request.event_id.trim().is_empty()
        || request.idempotency_key.trim().is_empty()
        || request.actor.id.trim().is_empty()
        || request.actor.provenance.trim().is_empty()
        || request.scope.trim().is_empty()
    {
        return Evaluation::clarify("MISSING_REQUIRED_FIELD", "required string is empty");
    }

    let required = |values: &[&str]| values.iter().all(|value| !value.trim().is_empty());
    let unique_transitions = |values: &[TransitionKind]| {
        values
            .iter()
            .enumerate()
            .all(|(index, value)| !values[..index].contains(value))
    };
    let unique_nonempty_strings = |values: &[String]| {
        values
            .iter()
            .enumerate()
            .all(|(index, value)| !value.trim().is_empty() && !values[..index].contains(value))
    };

    let structural_error = match &request.event {
        Event::AporiaOpened {
            aporia_id,
            question,
            blocks,
        } => {
            if !required(&[aporia_id, question]) {
                Some((
                    "MISSING_REQUIRED_FIELD",
                    "aporia id and question are required",
                ))
            } else if !unique_transitions(blocks) {
                Some((
                    "DUPLICATE_TRANSITION_KIND",
                    "aporia blocks contain duplicates",
                ))
            } else if state.aporias.contains_key(aporia_id) {
                Some(("APORIA_ALREADY_EXISTS", "aporia id already exists"))
            } else {
                None
            }
        }
        Event::AporiaResolved {
            aporia_id,
            resolution_ref,
        } => {
            if !required(&[aporia_id, resolution_ref]) {
                Some((
                    "MISSING_REQUIRED_FIELD",
                    "aporia id and resolution ref are required",
                ))
            } else if let Some(aporia) = state.aporias.get(aporia_id) {
                if aporia.resolution_ref.is_some() {
                    Some(("APORIA_ALREADY_RESOLVED", "aporia is already resolved"))
                } else if aporia.scope != request.scope {
                    Some((
                        "SCOPE_MISMATCH",
                        "resolution scope differs from aporia scope",
                    ))
                } else {
                    None
                }
            } else {
                Some(("UNKNOWN_APORIA", "aporia does not exist"))
            }
        }
        Event::DelegationGranted {
            delegation_id,
            grantee,
            transition_kinds,
        } => {
            if !required(&[delegation_id, grantee]) || transition_kinds.is_empty() {
                Some((
                    "MISSING_REQUIRED_FIELD",
                    "delegation id, grantee, and at least one transition are required",
                ))
            } else if !unique_transitions(transition_kinds) {
                Some((
                    "DUPLICATE_TRANSITION_KIND",
                    "delegation transitions contain duplicates",
                ))
            } else if state.delegations.contains_key(delegation_id) {
                Some(("DELEGATION_ALREADY_EXISTS", "delegation id already exists"))
            } else {
                None
            }
        }
        Event::DelegationRevoked { delegation_id } => {
            if !required(&[delegation_id]) {
                Some(("MISSING_REQUIRED_FIELD", "delegation id is required"))
            } else if let Some(delegation) = state.delegations.get(delegation_id) {
                if !delegation.active {
                    Some((
                        "DELEGATION_ALREADY_REVOKED",
                        "delegation is already revoked",
                    ))
                } else if delegation.scope != request.scope {
                    Some((
                        "SCOPE_MISMATCH",
                        "revocation scope differs from delegation scope",
                    ))
                } else {
                    None
                }
            } else {
                Some(("UNKNOWN_DELEGATION", "delegation does not exist"))
            }
        }
        Event::DecisionCommitted {
            decision_id,
            question,
            value,
            ..
        } => {
            if !required(&[decision_id, question, value]) {
                Some((
                    "MISSING_REQUIRED_FIELD",
                    "decision id, question, and value are required",
                ))
            } else if state.decisions.contains_key(decision_id) {
                Some(("DECISION_ALREADY_EXISTS", "decision id already exists"))
            } else {
                None
            }
        }
        Event::DecisionSuperseded {
            decision_id,
            replacement_id,
        } => {
            if !required(&[decision_id, replacement_id]) {
                Some((
                    "MISSING_REQUIRED_FIELD",
                    "decision id and replacement id are required",
                ))
            } else if decision_id == replacement_id {
                Some(("INVALID_REPLACEMENT", "decision cannot replace itself"))
            } else if let (Some(decision), Some(replacement)) = (
                state.decisions.get(decision_id),
                state.decisions.get(replacement_id),
            ) {
                if decision.superseded_by.is_some() {
                    Some((
                        "DECISION_ALREADY_SUPERSEDED",
                        "decision is already superseded",
                    ))
                } else if decision.scope != request.scope || replacement.scope != request.scope {
                    Some((
                        "SCOPE_MISMATCH",
                        "decision scopes do not match request scope",
                    ))
                } else {
                    None
                }
            } else {
                Some(("UNKNOWN_DECISION", "decision or replacement does not exist"))
            }
        }
        Event::RiskAccepted {
            risk_id,
            subject,
            review_condition,
        } => {
            if !required(&[risk_id, subject, review_condition]) {
                Some((
                    "MISSING_REQUIRED_FIELD",
                    "risk id, subject, and review condition are required",
                ))
            } else if state.accepted_risks.contains_key(risk_id) {
                Some(("RISK_ALREADY_EXISTS", "risk id already exists"))
            } else {
                None
            }
        }
        Event::ToolHoldPlaced {
            tool_hold_id,
            tool_name,
            reason,
        } => {
            if !required(&[tool_hold_id, tool_name, reason]) {
                Some((
                    "MISSING_REQUIRED_FIELD",
                    "tool hold id, tool name, and reason are required",
                ))
            } else if state.tool_holds.contains_key(tool_hold_id) {
                Some(("TOOL_HOLD_ALREADY_EXISTS", "tool hold id already exists"))
            } else {
                None
            }
        }
        Event::ToolHoldReleased { tool_hold_id } => {
            if !required(&[tool_hold_id]) {
                Some(("MISSING_REQUIRED_FIELD", "tool hold id is required"))
            } else if let Some(hold) = state.tool_holds.get(tool_hold_id) {
                if !hold.active {
                    Some((
                        "TOOL_HOLD_ALREADY_RELEASED",
                        "tool hold is already released",
                    ))
                } else if hold.scope != request.scope {
                    Some((
                        "SCOPE_MISMATCH",
                        "release scope differs from tool hold scope",
                    ))
                } else {
                    None
                }
            } else {
                Some(("UNKNOWN_TOOL_HOLD", "tool hold does not exist"))
            }
        }
        Event::PlanRegistered {
            plan_id,
            objective,
            acceptance_checks,
            unresolved_questions,
        } => {
            if !required(&[plan_id, objective]) || acceptance_checks.is_empty() {
                Some((
                    "MISSING_REQUIRED_FIELD",
                    "plan id, objective, and at least one acceptance check are required",
                ))
            } else if !unique_nonempty_strings(acceptance_checks)
                || !unique_nonempty_strings(unresolved_questions)
            {
                Some((
                    "INVALID_PLAN_LIST",
                    "plan lists must contain unique non-empty strings",
                ))
            } else if state.plans.contains_key(plan_id) {
                Some(("PLAN_ALREADY_EXISTS", "plan id already exists"))
            } else {
                None
            }
        }
        Event::PlanAuthorized {
            authorization_id,
            plan_id,
            session_id,
            tool_name,
            ..
        } => {
            if !required(&[authorization_id, plan_id, session_id, tool_name]) {
                Some((
                    "MISSING_REQUIRED_FIELD",
                    "authorization id, plan id, session id, and tool name are required",
                ))
            } else if state.plan_authorizations.contains_key(authorization_id) {
                Some((
                    "PLAN_AUTHORIZATION_ALREADY_EXISTS",
                    "plan authorization id already exists",
                ))
            } else if let Some(plan) = state.plans.get(plan_id) {
                if plan.scope != request.scope {
                    Some((
                        "SCOPE_MISMATCH",
                        "authorization scope differs from plan scope",
                    ))
                } else {
                    None
                }
            } else {
                Some(("UNKNOWN_PLAN", "plan does not exist"))
            }
        }
        Event::PlanAuthorizationRevoked { authorization_id } => {
            if !required(&[authorization_id]) {
                Some((
                    "MISSING_REQUIRED_FIELD",
                    "plan authorization id is required",
                ))
            } else if let Some(authorization) = state.plan_authorizations.get(authorization_id) {
                if !authorization.active {
                    Some((
                        "PLAN_AUTHORIZATION_ALREADY_REVOKED",
                        "plan authorization is already revoked",
                    ))
                } else if authorization.scope != request.scope {
                    Some((
                        "SCOPE_MISMATCH",
                        "revocation scope differs from authorization scope",
                    ))
                } else {
                    None
                }
            } else {
                Some((
                    "UNKNOWN_PLAN_AUTHORIZATION",
                    "plan authorization does not exist",
                ))
            }
        }
        Event::ExecutionGrantIssued {
            grant_id,
            plan_id,
            session_id,
            tool_name,
            max_uses,
            ..
        } => {
            if !required(&[grant_id, plan_id, session_id, tool_name]) || *max_uses == 0 {
                Some((
                    "INVALID_EXECUTION_GRANT",
                    "grant id, plan id, session id, tool name, and a positive max_uses are required",
                ))
            } else if state.execution_grants.contains_key(grant_id) {
                Some((
                    "EXECUTION_GRANT_ALREADY_EXISTS",
                    "execution grant id already exists",
                ))
            } else if let Some(plan) = state.plans.get(plan_id) {
                if plan.scope != request.scope {
                    Some((
                        "SCOPE_MISMATCH",
                        "execution grant scope differs from plan scope",
                    ))
                } else {
                    None
                }
            } else {
                Some(("UNKNOWN_PLAN", "plan does not exist"))
            }
        }
        Event::ExecutionGrantRevoked { grant_id } => {
            if !required(&[grant_id]) {
                Some(("MISSING_REQUIRED_FIELD", "execution grant id is required"))
            } else if let Some(grant) = state.execution_grants.get(grant_id) {
                if !grant.active {
                    Some((
                        "EXECUTION_GRANT_ALREADY_INACTIVE",
                        "execution grant is already revoked or consumed",
                    ))
                } else if grant.scope != request.scope {
                    Some((
                        "SCOPE_MISMATCH",
                        "revocation scope differs from execution grant scope",
                    ))
                } else {
                    None
                }
            } else {
                Some(("UNKNOWN_EXECUTION_GRANT", "execution grant does not exist"))
            }
        }
        Event::ExecutionGrantConsumed {
            grant_id,
            tool_use_id,
        } => {
            if !required(&[grant_id, tool_use_id]) {
                Some((
                    "MISSING_REQUIRED_FIELD",
                    "execution grant id and tool use id are required",
                ))
            } else if state.consumed_tool_uses.contains_key(tool_use_id) {
                Some((
                    "TOOL_USE_ALREADY_CONSUMED",
                    "tool use id was already consumed",
                ))
            } else if let Some(grant) = state.execution_grants.get(grant_id) {
                if !grant.active || grant.consumed_uses >= grant.max_uses {
                    Some((
                        "EXECUTION_GRANT_EXHAUSTED",
                        "execution grant has no remaining uses",
                    ))
                } else if grant.scope != request.scope {
                    Some((
                        "SCOPE_MISMATCH",
                        "consumption scope differs from execution grant scope",
                    ))
                } else {
                    None
                }
            } else {
                Some(("UNKNOWN_EXECUTION_GRANT", "execution grant does not exist"))
            }
        }
        Event::EvidenceRecorded {
            evidence_id,
            locator,
            digest,
            ..
        } => {
            if !required(&[evidence_id, locator])
                || digest.as_ref().is_some_and(|value| value.trim().is_empty())
            {
                Some((
                    "MISSING_REQUIRED_FIELD",
                    "evidence id, locator, and any supplied digest must be non-empty",
                ))
            } else if state.evidence.contains_key(evidence_id) {
                Some(("EVIDENCE_ALREADY_EXISTS", "evidence id already exists"))
            } else {
                None
            }
        }
        Event::ClaimRecorded {
            claim_id,
            statement,
            status,
            evidence_refs,
        } => {
            if !required(&[claim_id, statement]) || !unique_nonempty_strings(evidence_refs) {
                Some((
                    "INVALID_CLAIM",
                    "claim id, statement, and unique non-empty evidence refs are required",
                ))
            } else if matches!(
                status,
                EpistemicStatus::Verified | EpistemicStatus::Refuted | EpistemicStatus::Stale
            ) {
                Some((
                    "INVALID_INITIAL_CLAIM_STATUS",
                    "a new claim must begin as observed, inferred, or hypothesized",
                ))
            } else if state.claims.contains_key(claim_id) {
                Some(("CLAIM_ALREADY_EXISTS", "claim id already exists"))
            } else if let Some(reference) = evidence_refs.iter().find(|reference| {
                state
                    .evidence
                    .get(*reference)
                    .is_none_or(|evidence| evidence.scope != request.scope)
            }) {
                let _ = reference;
                Some((
                    "INVALID_EVIDENCE_REFERENCE",
                    "claim evidence must exist in the same scope",
                ))
            } else if *status == EpistemicStatus::Observed
                && (evidence_refs.is_empty()
                    || evidence_refs.iter().any(|reference| {
                        state
                            .evidence
                            .get(reference)
                            .is_some_and(|evidence| evidence.kind == EvidenceKind::AgentInference)
                    }))
            {
                Some((
                    "OBSERVATION_EVIDENCE_REQUIRED",
                    "an observed claim requires non-inference evidence",
                ))
            } else {
                None
            }
        }
        Event::ClaimStatusChanged {
            claim_id,
            status,
            evidence_refs,
        } => {
            if !required(&[claim_id]) || !unique_nonempty_strings(evidence_refs) {
                Some((
                    "INVALID_CLAIM_STATUS_CHANGE",
                    "claim id and unique non-empty evidence refs are required",
                ))
            } else if let Some(claim) = state.claims.get(claim_id) {
                if claim.scope != request.scope {
                    Some(("SCOPE_MISMATCH", "claim scope differs from request scope"))
                } else if claim.superseded_by.is_some() {
                    Some((
                        "CLAIM_SUPERSEDED",
                        "a superseded claim cannot change status",
                    ))
                } else if claim.status == *status {
                    Some(("CLAIM_STATUS_UNCHANGED", "claim already has this status"))
                } else if claim.status == EpistemicStatus::Refuted
                    && *status != EpistemicStatus::Stale
                {
                    Some((
                        "REFUTED_CLAIM_IMMUTABLE",
                        "a refuted claim must be replaced by a new claim",
                    ))
                } else if matches!(status, EpistemicStatus::Verified | EpistemicStatus::Refuted)
                    && evidence_refs.is_empty()
                {
                    Some((
                        "STATUS_EVIDENCE_REQUIRED",
                        "verified and refuted statuses require evidence",
                    ))
                } else if evidence_refs.iter().any(|reference| {
                    state
                        .evidence
                        .get(reference)
                        .is_none_or(|evidence| evidence.scope != request.scope)
                }) {
                    Some((
                        "INVALID_EVIDENCE_REFERENCE",
                        "status evidence must exist in the same scope",
                    ))
                } else if *status == EpistemicStatus::Verified
                    && !evidence_refs.iter().any(|reference| {
                        state
                            .evidence
                            .get(reference)
                            .is_some_and(|evidence| evidence.kind != EvidenceKind::AgentInference)
                    })
                {
                    Some((
                        "DIRECT_EVIDENCE_REQUIRED",
                        "verified status requires at least one non-inference evidence record",
                    ))
                } else {
                    None
                }
            } else {
                Some(("UNKNOWN_CLAIM", "claim does not exist"))
            }
        }
        Event::ClaimSuperseded {
            claim_id,
            replacement_id,
        } => {
            if !required(&[claim_id, replacement_id]) || claim_id == replacement_id {
                Some((
                    "INVALID_REPLACEMENT",
                    "claim and distinct replacement ids are required",
                ))
            } else if let (Some(claim), Some(replacement)) =
                (state.claims.get(claim_id), state.claims.get(replacement_id))
            {
                if claim.scope != request.scope || replacement.scope != request.scope {
                    Some(("SCOPE_MISMATCH", "claim scopes do not match request scope"))
                } else if claim.superseded_by.is_some() {
                    Some(("CLAIM_ALREADY_SUPERSEDED", "claim is already superseded"))
                } else {
                    None
                }
            } else {
                Some(("UNKNOWN_CLAIM", "claim or replacement does not exist"))
            }
        }
        Event::ArgumentRelationRecorded {
            relation_id,
            source_claim_id,
            target_claim_id,
            kind,
        } => {
            if !required(&[relation_id, source_claim_id, target_claim_id])
                || source_claim_id == target_claim_id
            {
                Some((
                    "INVALID_ARGUMENT_RELATION",
                    "a relation requires an id and two distinct claim ids",
                ))
            } else if state.argument_relations.contains_key(relation_id) {
                Some((
                    "ARGUMENT_RELATION_ALREADY_EXISTS",
                    "argument relation id already exists",
                ))
            } else if state.argument_relations.values().any(|relation| {
                relation.active
                    && relation.scope == request.scope
                    && relation.source_claim_id == *source_claim_id
                    && relation.target_claim_id == *target_claim_id
                    && relation.kind == *kind
            }) {
                Some((
                    "DUPLICATE_ARGUMENT_RELATION",
                    "an identical active relation already exists",
                ))
            } else if [source_claim_id, target_claim_id].iter().any(|claim_id| {
                !state.claims.get(*claim_id).is_some_and(|claim| {
                    claim.scope == request.scope
                        && claim.superseded_by.is_none()
                        && !matches!(
                            claim.status,
                            EpistemicStatus::Refuted | EpistemicStatus::Stale
                        )
                })
            }) {
                Some((
                    "INVALID_ARGUMENT_CLAIM_REFERENCE",
                    "relation claims must be active and in the same scope",
                ))
            } else {
                None
            }
        }
        Event::ArgumentRelationRetracted {
            relation_id,
            evidence_refs,
        } => {
            if !required(&[relation_id])
                || evidence_refs.is_empty()
                || !unique_nonempty_strings(evidence_refs)
            {
                Some((
                    "INVALID_ARGUMENT_RETRACTION",
                    "relation retraction requires unique supporting evidence",
                ))
            } else if let Some(relation) = state.argument_relations.get(relation_id) {
                if relation.scope != request.scope {
                    Some((
                        "SCOPE_MISMATCH",
                        "argument relation scope differs from request scope",
                    ))
                } else if !relation.active {
                    Some((
                        "ARGUMENT_RELATION_INACTIVE",
                        "argument relation is already retracted",
                    ))
                } else if evidence_refs.iter().any(|reference| {
                    state
                        .evidence
                        .get(reference)
                        .is_none_or(|evidence| evidence.scope != request.scope)
                }) {
                    Some((
                        "INVALID_EVIDENCE_REFERENCE",
                        "retraction evidence must exist in the same scope",
                    ))
                } else {
                    None
                }
            } else {
                Some((
                    "UNKNOWN_ARGUMENT_RELATION",
                    "argument relation does not exist",
                ))
            }
        }
        Event::BeliefRevisionRecorded {
            revision_id,
            claim_id,
            prior_status,
            revised_status,
            trigger_claim_ids,
            evidence_refs,
            rationale,
        } => {
            if !required(&[revision_id, claim_id, rationale])
                || prior_status == revised_status
                || !unique_nonempty_strings(trigger_claim_ids)
                || !unique_nonempty_strings(evidence_refs)
                || trigger_claim_ids.iter().any(|trigger| trigger == claim_id)
                || (trigger_claim_ids.is_empty() && evidence_refs.is_empty())
            {
                Some((
                    "INVALID_BELIEF_REVISION",
                    "belief revision requires distinct statuses, rationale, and unique non-self references",
                ))
            } else if state.belief_revisions.contains_key(revision_id) {
                Some((
                    "BELIEF_REVISION_ALREADY_EXISTS",
                    "belief revision id already exists",
                ))
            } else if let Some(claim) = state.claims.get(claim_id) {
                if claim.scope != request.scope {
                    Some(("SCOPE_MISMATCH", "claim scope differs from request scope"))
                } else if claim.superseded_by.is_some() {
                    Some(("CLAIM_SUPERSEDED", "a superseded claim cannot be revised"))
                } else if claim.status != *prior_status {
                    Some((
                        "STALE_BELIEF_REVISION",
                        "prior status does not match the current claim status",
                    ))
                } else if *prior_status == EpistemicStatus::Refuted
                    && *revised_status != EpistemicStatus::Stale
                {
                    Some((
                        "REFUTED_CLAIM_IMMUTABLE",
                        "a refuted claim must be replaced by a new claim",
                    ))
                } else if trigger_claim_ids.iter().any(|trigger_id| {
                    !state.claims.get(trigger_id).is_some_and(|trigger| {
                        trigger.scope == request.scope && trigger.superseded_by.is_none()
                    })
                }) {
                    Some((
                        "INVALID_TRIGGER_CLAIM",
                        "trigger claims must be active and in the same scope",
                    ))
                } else if evidence_refs.iter().any(|reference| {
                    state
                        .evidence
                        .get(reference)
                        .is_none_or(|evidence| evidence.scope != request.scope)
                }) {
                    Some((
                        "INVALID_EVIDENCE_REFERENCE",
                        "revision evidence must exist in the same scope",
                    ))
                } else if matches!(
                    revised_status,
                    EpistemicStatus::Observed
                        | EpistemicStatus::Verified
                        | EpistemicStatus::Refuted
                ) && (evidence_refs.is_empty()
                    || !evidence_refs.iter().any(|reference| {
                        state.evidence.get(reference).is_some_and(|evidence| {
                            evidence.scope == request.scope
                                && evidence.kind != EvidenceKind::AgentInference
                        })
                    }))
                {
                    Some((
                        "DIRECT_EVIDENCE_REQUIRED",
                        "observed, verified, and refuted revisions require non-inference evidence",
                    ))
                } else {
                    None
                }
            } else {
                Some(("UNKNOWN_CLAIM", "claim does not exist"))
            }
        }
        Event::DecisionBasisLinked {
            decision_id,
            claim_ids,
            evidence_refs,
        } => {
            if !required(&[decision_id])
                || (claim_ids.is_empty() && evidence_refs.is_empty())
                || !unique_nonempty_strings(claim_ids)
                || !unique_nonempty_strings(evidence_refs)
            {
                Some((
                    "INVALID_DECISION_BASIS",
                    "decision basis requires unique claim or evidence references",
                ))
            } else if state.decision_bases.contains_key(decision_id) {
                Some((
                    "DECISION_BASIS_ALREADY_LINKED",
                    "decision basis is already linked",
                ))
            } else if state
                .decisions
                .get(decision_id)
                .is_none_or(|decision| decision.scope != request.scope)
            {
                Some((
                    "UNKNOWN_DECISION",
                    "decision does not exist in the same scope",
                ))
            } else if claim_ids.iter().any(|claim_id| {
                !state.claims.get(claim_id).is_some_and(|claim| {
                    claim.scope == request.scope && claim.superseded_by.is_none()
                })
            }) || evidence_refs.iter().any(|reference| {
                state
                    .evidence
                    .get(reference)
                    .is_none_or(|evidence| evidence.scope != request.scope)
            }) {
                Some((
                    "INVALID_DECISION_BASIS_REFERENCE",
                    "decision basis references must be active and in the same scope",
                ))
            } else if state.decisions.get(decision_id).is_some_and(|decision| {
                claim_ids.iter().any(|claim_id| {
                    state
                        .claims
                        .get(claim_id)
                        .is_some_and(|claim| claim.sequence > decision.sequence)
                }) || evidence_refs.iter().any(|reference| {
                    state
                        .evidence
                        .get(reference)
                        .is_some_and(|evidence| evidence.sequence > decision.sequence)
                })
            }) {
                Some((
                    "NONCONTEMPORANEOUS_DECISION_BASIS",
                    "decision basis references must predate the decision",
                ))
            } else {
                None
            }
        }
        Event::DecisionReviewRecorded {
            review_id,
            decision_id,
            evidence_refs,
            lessons,
            follow_up_claim_ids,
            ..
        } => {
            if !required(&[review_id, decision_id])
                || evidence_refs.is_empty()
                || lessons.is_empty()
                || !unique_nonempty_strings(evidence_refs)
                || !unique_nonempty_strings(lessons)
                || !unique_nonempty_strings(follow_up_claim_ids)
            {
                Some((
                    "INVALID_DECISION_REVIEW",
                    "decision review requires ids, evidence, and at least one lesson",
                ))
            } else if state.decision_reviews.contains_key(review_id) {
                Some((
                    "DECISION_REVIEW_ALREADY_EXISTS",
                    "decision review id already exists",
                ))
            } else if state
                .decisions
                .get(decision_id)
                .is_none_or(|decision| decision.scope != request.scope)
            {
                Some((
                    "UNKNOWN_DECISION",
                    "decision does not exist in the same scope",
                ))
            } else if state
                .decision_bases
                .get(decision_id)
                .is_none_or(|basis| basis.scope != request.scope)
            {
                Some((
                    "DECISION_BASIS_REQUIRED",
                    "decision review requires a contemporaneous decision basis",
                ))
            } else if evidence_refs.iter().any(|reference| {
                state
                    .evidence
                    .get(reference)
                    .is_none_or(|evidence| evidence.scope != request.scope)
            }) || follow_up_claim_ids.iter().any(|claim_id| {
                !state.claims.get(claim_id).is_some_and(|claim| {
                    claim.scope == request.scope && claim.superseded_by.is_none()
                })
            }) {
                Some((
                    "INVALID_DECISION_REVIEW_REFERENCE",
                    "review references must exist in the same scope",
                ))
            } else if !evidence_refs.iter().any(|reference| {
                state.evidence.get(reference).is_some_and(|evidence| {
                    evidence.scope == request.scope && evidence.kind != EvidenceKind::AgentInference
                })
            }) {
                Some((
                    "DIRECT_EVIDENCE_REQUIRED",
                    "decision review requires non-inference evidence",
                ))
            } else {
                None
            }
        }
        Event::PlanBasisLinked {
            plan_id,
            evidence_refs,
            assumption_claim_ids,
        } => {
            if !required(&[plan_id])
                || (evidence_refs.is_empty() && assumption_claim_ids.is_empty())
                || !unique_nonempty_strings(evidence_refs)
                || !unique_nonempty_strings(assumption_claim_ids)
            {
                Some((
                    "INVALID_PLAN_BASIS",
                    "plan basis requires unique evidence or assumption references",
                ))
            } else if let Some(plan) = state.plans.get(plan_id) {
                if plan.scope != request.scope {
                    Some(("SCOPE_MISMATCH", "plan scope differs from request scope"))
                } else if !plan.evidence_refs.is_empty() || !plan.assumption_claim_ids.is_empty() {
                    Some(("PLAN_BASIS_ALREADY_LINKED", "plan basis is already linked"))
                } else if evidence_refs.iter().any(|reference| {
                    state
                        .evidence
                        .get(reference)
                        .is_none_or(|evidence| evidence.scope != request.scope)
                }) || assumption_claim_ids.iter().any(|claim_id| {
                    !state.claims.get(claim_id).is_some_and(|claim| {
                        claim.scope == request.scope && claim.superseded_by.is_none()
                    })
                }) {
                    Some((
                        "INVALID_PLAN_BASIS_REFERENCE",
                        "plan basis references must be active and in the same scope",
                    ))
                } else {
                    None
                }
            } else {
                Some(("UNKNOWN_PLAN", "plan does not exist"))
            }
        }
        Event::ActionOutcomeRecorded {
            session_id,
            tool_name,
            tool_use_id,
            evidence_refs,
            ..
        } => {
            if !required(&[session_id, tool_name, tool_use_id])
                || !unique_nonempty_strings(evidence_refs)
            {
                Some((
                    "INVALID_ACTION_OUTCOME",
                    "session, tool, tool use, and unique evidence refs are required",
                ))
            } else if state.action_outcomes.contains_key(&action_outcome_key(
                &request.scope,
                session_id,
                tool_use_id,
            )) {
                Some((
                    "ACTION_OUTCOME_ALREADY_EXISTS",
                    "tool use already has an outcome",
                ))
            } else if evidence_refs.iter().any(|reference| {
                state
                    .evidence
                    .get(reference)
                    .is_none_or(|evidence| evidence.scope != request.scope)
            }) {
                Some((
                    "INVALID_EVIDENCE_REFERENCE",
                    "outcome evidence must exist in the same scope",
                ))
            } else {
                None
            }
        }
        Event::VerificationRecorded {
            verification_id,
            plan_id,
            check_index,
            evidence_refs,
            ..
        } => {
            if !required(&[verification_id, plan_id])
                || evidence_refs.is_empty()
                || !unique_nonempty_strings(evidence_refs)
            {
                Some((
                    "INVALID_VERIFICATION",
                    "verification requires ids and unique supporting evidence",
                ))
            } else if state.verifications.contains_key(verification_id) {
                Some((
                    "VERIFICATION_ALREADY_EXISTS",
                    "verification id already exists",
                ))
            } else if let Some(plan) = state.plans.get(plan_id) {
                if plan.scope != request.scope {
                    Some(("SCOPE_MISMATCH", "plan scope differs from request scope"))
                } else if (*check_index as usize) >= plan.acceptance_checks.len() {
                    Some((
                        "UNKNOWN_ACCEPTANCE_CHECK",
                        "check index is outside the plan",
                    ))
                } else if evidence_refs.iter().any(|reference| {
                    state
                        .evidence
                        .get(reference)
                        .is_none_or(|evidence| evidence.scope != request.scope)
                }) {
                    Some((
                        "INVALID_EVIDENCE_REFERENCE",
                        "verification evidence must exist in the same scope",
                    ))
                } else if !evidence_refs.iter().any(|reference| {
                    state.evidence.get(reference).is_some_and(|evidence| {
                        evidence.scope == request.scope
                            && evidence.kind != EvidenceKind::AgentInference
                    })
                }) {
                    Some((
                        "DIRECT_EVIDENCE_REQUIRED",
                        "verification requires at least one non-inference evidence record",
                    ))
                } else {
                    None
                }
            } else {
                Some(("UNKNOWN_PLAN", "plan does not exist"))
            }
        }
        Event::PlanCompleted {
            plan_id,
            residual_risk_refs,
        } => {
            if !required(&[plan_id]) || !unique_nonempty_strings(residual_risk_refs) {
                Some((
                    "INVALID_COMPLETION",
                    "plan id and unique residual risk refs are required",
                ))
            } else if let Some(plan) = state.plans.get(plan_id) {
                if plan.scope != request.scope {
                    Some(("SCOPE_MISMATCH", "plan scope differs from request scope"))
                } else if plan.completed {
                    Some(("PLAN_ALREADY_COMPLETED", "plan is already completed"))
                } else if residual_risk_refs.iter().any(|risk_id| {
                    state
                        .accepted_risks
                        .get(risk_id)
                        .is_none_or(|risk| risk.scope != request.scope)
                }) {
                    Some((
                        "INVALID_RESIDUAL_RISK",
                        "residual risks must name accepted risks in the same scope",
                    ))
                } else {
                    let all_passed = (0..plan.acceptance_checks.len()).all(|check_index| {
                        state
                            .verifications
                            .values()
                            .filter(|verification| {
                                verification.plan_id == *plan_id
                                    && verification.check_index as usize == check_index
                            })
                            .max_by_key(|verification| verification.sequence)
                            .is_some_and(|verification| {
                                verification.result == VerificationResult::Passed
                            })
                    });
                    if all_passed || !residual_risk_refs.is_empty() {
                        None
                    } else {
                        Some((
                            "PLAN_VERIFICATION_INCOMPLETE",
                            "every acceptance check must pass or residual risk must be accepted",
                        ))
                    }
                }
            } else {
                Some(("UNKNOWN_PLAN", "plan does not exist"))
            }
        }
        Event::CheckpointPublished {
            checkpoint_id,
            session_id,
            objective,
            verified_claim_ids,
            unresolved_claim_ids,
            aporia_ids,
            next_checks,
            artifact_refs,
        } => {
            if !required(&[checkpoint_id, session_id, objective])
                || !unique_nonempty_strings(verified_claim_ids)
                || !unique_nonempty_strings(unresolved_claim_ids)
                || !unique_nonempty_strings(aporia_ids)
                || !unique_nonempty_strings(next_checks)
                || !unique_nonempty_strings(artifact_refs)
            {
                Some((
                    "INVALID_CHECKPOINT",
                    "checkpoint fields and lists must be non-empty and unique where present",
                ))
            } else if state.checkpoints.contains_key(checkpoint_id) {
                Some(("CHECKPOINT_ALREADY_EXISTS", "checkpoint id already exists"))
            } else if verified_claim_ids.iter().any(|claim_id| {
                !state.claims.get(claim_id).is_some_and(|claim| {
                    claim.scope == request.scope
                        && claim.status == EpistemicStatus::Verified
                        && claim.superseded_by.is_none()
                })
            }) || unresolved_claim_ids.iter().any(|claim_id| {
                !state.claims.get(claim_id).is_some_and(|claim| {
                    claim.scope == request.scope
                        && matches!(
                            claim.status,
                            EpistemicStatus::Inferred | EpistemicStatus::Hypothesized
                        )
                        && claim.superseded_by.is_none()
                })
            }) || aporia_ids.iter().any(|aporia_id| {
                !state.aporias.get(aporia_id).is_some_and(|aporia| {
                    aporia.scope == request.scope && aporia.resolution_ref.is_none()
                })
            }) || artifact_refs.iter().any(|evidence_id| {
                state
                    .evidence
                    .get(evidence_id)
                    .is_none_or(|evidence| evidence.scope != request.scope)
            }) {
                Some((
                    "INVALID_CHECKPOINT_REFERENCE",
                    "checkpoint references must be current and in the same scope",
                ))
            } else {
                None
            }
        }
        Event::CheckpointClaimed {
            checkpoint_id,
            session_id,
        } => {
            if !required(&[checkpoint_id, session_id]) {
                Some((
                    "MISSING_REQUIRED_FIELD",
                    "checkpoint and receiving session ids are required",
                ))
            } else if let Some(checkpoint) = state.checkpoints.get(checkpoint_id) {
                if checkpoint.scope != request.scope {
                    Some((
                        "SCOPE_MISMATCH",
                        "checkpoint scope differs from request scope",
                    ))
                } else if checkpoint.state != CheckpointState::Open {
                    Some(("CHECKPOINT_NOT_OPEN", "checkpoint is not open"))
                } else if checkpoint.session_id == *session_id {
                    Some((
                        "CHECKPOINT_SELF_CLAIM",
                        "the publishing session cannot claim its own checkpoint",
                    ))
                } else if state.checkpoints.values().any(|candidate| {
                    candidate.scope == request.scope
                        && candidate.claimed_by_session.as_deref() == Some(session_id)
                }) {
                    Some((
                        "SESSION_ALREADY_CLAIMED_CHECKPOINT",
                        "receiving session already claimed a checkpoint",
                    ))
                } else {
                    None
                }
            } else {
                Some(("UNKNOWN_CHECKPOINT", "checkpoint does not exist"))
            }
        }
        Event::CheckpointExpired { checkpoint_id } => {
            if !required(&[checkpoint_id]) {
                Some(("MISSING_REQUIRED_FIELD", "checkpoint id is required"))
            } else if let Some(checkpoint) = state.checkpoints.get(checkpoint_id) {
                if checkpoint.scope != request.scope {
                    Some((
                        "SCOPE_MISMATCH",
                        "checkpoint scope differs from request scope",
                    ))
                } else if checkpoint.state != CheckpointState::Open {
                    Some(("CHECKPOINT_NOT_OPEN", "only an open checkpoint can expire"))
                } else {
                    None
                }
            } else {
                Some(("UNKNOWN_CHECKPOINT", "checkpoint does not exist"))
            }
        }
    };
    if let Some((code, message)) = structural_error {
        return Evaluation::deny(code, message);
    }

    if let Some(kind) = request.event.transition_kind()
        && state.aporias.values().any(|aporia| {
            aporia.resolution_ref.is_none()
                && aporia.scope == request.scope
                && aporia.blocks.contains(&kind)
        })
    {
        return Evaluation::clarify(
            "OPEN_MATERIAL_APORIA",
            "an open material aporia blocks this transition",
        );
    }

    match &request.event {
        Event::DelegationGranted { .. }
        | Event::DelegationRevoked { .. }
        | Event::DecisionSuperseded { .. } => {
            if request.actor.kind != ActorKind::Human {
                return Evaluation::deny(
                    "HUMAN_AUTHORITY_REQUIRED",
                    "this transition requires an event declared as human-authored",
                );
            }
        }
        Event::AporiaResolved { .. }
            if !matches!(request.actor.kind, ActorKind::Human | ActorKind::Evidence) =>
        {
            return Evaluation::deny(
                "RESOLUTION_AUTHORITY_REQUIRED",
                "aporia resolution requires an event declared as human or evidence-authored",
            );
        }
        Event::DecisionCommitted { authority_ref, .. } => match request.actor.kind {
            ActorKind::Human => {}
            ActorKind::Agent => {
                let Some(reference) = authority_ref else {
                    return Evaluation::deny(
                        "DELEGATION_REQUIRED",
                        "agent decision requires an authority_ref",
                    );
                };
                let Some(delegation) = state.delegations.get(reference) else {
                    return Evaluation::deny(
                        "UNKNOWN_DELEGATION",
                        "authority_ref does not name a delegation",
                    );
                };
                if !delegation.active
                    || delegation.grantee != request.actor.id
                    || delegation.scope != request.scope
                    || !delegation
                        .transition_kinds
                        .contains(&TransitionKind::DecisionCommit)
                {
                    return Evaluation::deny(
                        "DELEGATION_SCOPE_VIOLATION",
                        "delegation is inactive or does not cover actor, scope, and transition",
                    );
                }
            }
            _ => {
                return Evaluation::deny(
                    "DECISION_AUTHORITY_REQUIRED",
                    "decision requires human authority or a delegated agent",
                );
            }
        },
        Event::RiskAccepted { .. } if request.actor.kind != ActorKind::Human => {
            return Evaluation::deny(
                "HUMAN_RISK_OWNER_REQUIRED",
                "risk acceptance requires an event declared as human-authored",
            );
        }
        Event::ToolHoldReleased { .. }
            if !matches!(request.actor.kind, ActorKind::Human | ActorKind::Evidence) =>
        {
            return Evaluation::deny(
                "RELEASE_AUTHORITY_REQUIRED",
                "tool hold release requires an event declared as human or evidence-authored",
            );
        }
        Event::ActionOutcomeRecorded { .. }
            if !matches!(request.actor.kind, ActorKind::Host | ActorKind::Evidence) =>
        {
            return Evaluation::deny(
                "OUTCOME_OBSERVER_REQUIRED",
                "action outcome requires a host or evidence-authored event",
            );
        }
        Event::CheckpointClaimed { .. } if request.actor.kind != ActorKind::Host => {
            return Evaluation::deny(
                "CHECKPOINT_RECEIVER_REQUIRED",
                "checkpoint claim requires a host-authored event",
            );
        }
        Event::CheckpointExpired { .. }
            if !matches!(request.actor.kind, ActorKind::Human | ActorKind::Host) =>
        {
            return Evaluation::deny(
                "CHECKPOINT_EXPIRY_AUTHORITY_REQUIRED",
                "checkpoint expiry requires a human or host-authored event",
            );
        }
        Event::PlanCompleted { .. } => match request.actor.kind {
            ActorKind::Human => {}
            ActorKind::Agent => {
                let delegated = state.delegations.values().any(|delegation| {
                    delegation.active
                        && delegation.grantee == request.actor.id
                        && delegation.scope == request.scope
                        && delegation
                            .transition_kinds
                            .contains(&TransitionKind::CompletionClaim)
                });
                if !delegated {
                    return Evaluation::deny(
                        "COMPLETION_AUTHORITY_REQUIRED",
                        "agent completion requires an active completion-claim delegation",
                    );
                }
            }
            _ => {
                return Evaluation::deny(
                    "COMPLETION_AUTHORITY_REQUIRED",
                    "plan completion requires human authority or a delegated agent",
                );
            }
        },
        Event::PlanAuthorized { authority_ref, .. }
        | Event::ExecutionGrantIssued { authority_ref, .. } => match request.actor.kind {
            ActorKind::Human => {}
            ActorKind::Agent => {
                let Some(reference) = authority_ref else {
                    return Evaluation::deny(
                        "DELEGATION_REQUIRED",
                        "agent plan authorization requires an authority_ref",
                    );
                };
                let Some(delegation) = state.delegations.get(reference) else {
                    return Evaluation::deny(
                        "UNKNOWN_DELEGATION",
                        "authority_ref does not name a delegation",
                    );
                };
                if !delegation.active
                    || delegation.grantee != request.actor.id
                    || delegation.scope != request.scope
                    || !delegation
                        .transition_kinds
                        .contains(&TransitionKind::PlanAuthorize)
                {
                    return Evaluation::deny(
                        "DELEGATION_SCOPE_VIOLATION",
                        "delegation is inactive or does not cover actor, scope, and plan authorization",
                    );
                }
            }
            _ => {
                return Evaluation::deny(
                    "PLAN_AUTHORITY_REQUIRED",
                    "plan authorization requires human authority or a delegated agent",
                );
            }
        },
        Event::PlanAuthorizationRevoked { .. } | Event::ExecutionGrantRevoked { .. }
            if !matches!(request.actor.kind, ActorKind::Human | ActorKind::Evidence) =>
        {
            return Evaluation::deny(
                "REVOCATION_AUTHORITY_REQUIRED",
                "plan authorization revocation requires an event declared as human or evidence-authored",
            );
        }
        Event::ExecutionGrantConsumed { .. } if request.actor.kind != ActorKind::Host => {
            return Evaluation::deny(
                "HOST_CONSUMPTION_REQUIRED",
                "execution grant consumption requires a host-authored event",
            );
        }
        _ => {}
    }

    Evaluation::allow()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub evaluation: Evaluation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationOutcome {
    pub from_schema: u32,
    pub to_schema: u32,
    pub revision: u64,
    pub records: usize,
}

#[derive(Debug, Default)]
pub struct EventLog {
    state: State,
    events: Vec<StoredEvent>,
}

impl EventLog {
    pub fn state(&self) -> &State {
        &self.state
    }

    fn find_idempotency_key(&self, key: &str) -> Option<&StoredEvent> {
        self.events
            .iter()
            .find(|event| event.request.idempotency_key == key)
    }

    fn find_event_id(&self, event_id: &str) -> Option<&StoredEvent> {
        self.events
            .iter()
            .find(|event| event.request.event_id == event_id)
    }
}

fn replay_bytes(bytes: &[u8]) -> Result<EventLog> {
    if bytes.is_empty() {
        return Ok(EventLog::default());
    }
    if !bytes.ends_with(b"\n") {
        return Err(Error::CorruptLog {
            line: bytes.iter().filter(|byte| **byte == b'\n').count() + 1,
            reason: "record is not newline-terminated".into(),
        });
    }

    let mut log = EventLog::default();
    for (index, line) in bytes.split(|byte| *byte == b'\n').enumerate() {
        if line.is_empty() {
            if index + 1 == bytes.split(|byte| *byte == b'\n').count() {
                continue;
            }
            return Err(Error::CorruptLog {
                line: index + 1,
                reason: "blank records are not allowed".into(),
            });
        }
        let stored: StoredEvent =
            serde_json::from_slice(line).map_err(|error| Error::CorruptLog {
                line: index + 1,
                reason: error.to_string(),
            })?;
        if stored.request.schema_version != SCHEMA_VERSION {
            return Err(Error::CorruptLog {
                line: index + 1,
                reason: format!(
                    "unsupported schema version {}; expected {SCHEMA_VERSION}",
                    stored.request.schema_version
                ),
            });
        }
        log.state.apply(&stored)?;
        log.events.push(stored);
    }
    Ok(log)
}

pub fn load(path: impl AsRef<Path>) -> Result<EventLog> {
    let mut file = OpenOptions::new().read(true).open(path)?;
    file.lock_shared()?;
    read_locked(&mut file)
}

/// Loads an event log without waiting for another writer to release its lock.
pub fn load_nonblocking(path: impl AsRef<Path>) -> Result<EventLog> {
    let mut file = OpenOptions::new().read(true).open(path)?;
    file.try_lock_shared().map_err(std::io::Error::from)?;
    read_locked(&mut file)
}

pub(crate) fn transact_nonblocking<T>(
    path: impl AsRef<Path>,
    operation: impl FnOnce(&EventLog) -> Result<(T, Option<CommitRequest>)>,
) -> Result<T> {
    let mut file = OpenOptions::new().read(true).append(true).open(path)?;
    file.try_lock().map_err(std::io::Error::from)?;
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let log = replay_bytes(&bytes)?;
    let (value, request) = operation(&log)?;
    if let Some(request) = request {
        let outcome = commit_locked(&mut file, request)?;
        if outcome.status != CommitStatus::Committed {
            let unlock_result = file.unlock();
            unlock_result?;
            return Err(Error::Invariant(format!(
                "transactional commit rejected: {}",
                outcome.evaluation.reason_code
            )));
        }
    }
    file.unlock()?;
    Ok(value)
}

fn read_locked(file: &mut File) -> Result<EventLog> {
    let mut bytes = Vec::new();
    let read_result = file.read_to_end(&mut bytes);
    let unlock_result = file.unlock();
    match (read_result, unlock_result) {
        (Ok(_), Ok(())) => replay_bytes(&bytes),
        (Err(error), _) => Err(Error::Io(error)),
        (Ok(_), Err(error)) => Err(Error::Io(error)),
    }
}

/// Creates a new, empty event log and refuses to replace an existing store.
pub fn initialize(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.sync_data()?;
    Ok(())
}

pub fn migrate_to_current(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    from_schema: u32,
) -> Result<MigrationOutcome> {
    if !matches!(from_schema, 1..=3) || from_schema >= SCHEMA_VERSION {
        return Err(Error::Invariant(format!(
            "migration supports schema versions 1, 2, or 3 below current version {SCHEMA_VERSION}"
        )));
    }
    let source = source.as_ref();
    let destination = destination.as_ref();
    if source == destination {
        return Err(Error::Invariant(
            "migration source and destination must differ".into(),
        ));
    }

    let mut source_file = OpenOptions::new().read(true).open(source)?;
    source_file.lock_shared()?;
    let mut source_bytes = Vec::new();
    source_file.read_to_end(&mut source_bytes)?;
    if !source_bytes.is_empty() && !source_bytes.ends_with(b"\n") {
        return Err(Error::CorruptLog {
            line: source_bytes.iter().filter(|byte| **byte == b'\n').count() + 1,
            reason: "record is not newline-terminated".into(),
        });
    }

    let mut migrated = Vec::new();
    let mut records = 0_usize;
    for (index, line) in source_bytes.split(|byte| *byte == b'\n').enumerate() {
        if line.is_empty() {
            if index + 1 == source_bytes.split(|byte| *byte == b'\n').count() {
                continue;
            }
            return Err(Error::CorruptLog {
                line: index + 1,
                reason: "blank records are not allowed".into(),
            });
        }
        let mut value: serde_json::Value =
            serde_json::from_slice(line).map_err(|error| Error::CorruptLog {
                line: index + 1,
                reason: error.to_string(),
            })?;
        let version = value
            .get("schema_version")
            .and_then(serde_json::Value::as_u64);
        if version != Some(u64::from(from_schema)) {
            return Err(Error::CorruptLog {
                line: index + 1,
                reason: format!("expected schema version {from_schema}, got {version:?}"),
            });
        }
        let event_type = value
            .get("event")
            .and_then(|event| event.get("type"))
            .and_then(serde_json::Value::as_str);
        let v1_event = matches!(
            event_type,
            Some(
                "aporia_opened"
                    | "aporia_resolved"
                    | "delegation_granted"
                    | "delegation_revoked"
                    | "decision_committed"
                    | "decision_superseded"
                    | "risk_accepted"
                    | "tool_hold_placed"
                    | "tool_hold_released"
                    | "plan_registered"
                    | "plan_authorized"
                    | "plan_authorization_revoked"
            )
        );
        let v2_event = v1_event
            || matches!(
                event_type,
                Some(
                    "execution_grant_issued"
                        | "execution_grant_revoked"
                        | "execution_grant_consumed"
                )
            );
        let v3_event = v2_event
            || matches!(
                event_type,
                Some(
                    "evidence_recorded"
                        | "claim_recorded"
                        | "claim_status_changed"
                        | "claim_superseded"
                        | "plan_basis_linked"
                        | "action_outcome_recorded"
                        | "verification_recorded"
                        | "plan_completed"
                        | "checkpoint_published"
                        | "checkpoint_claimed"
                        | "checkpoint_expired"
                )
            );
        if (from_schema == 1 && !v1_event)
            || (from_schema == 2 && !v2_event)
            || (from_schema == 3 && !v3_event)
        {
            return Err(Error::CorruptLog {
                line: index + 1,
                reason: format!(
                    "event type {event_type:?} is not part of schema version {from_schema}"
                ),
            });
        }
        value["schema_version"] = serde_json::Value::from(SCHEMA_VERSION);
        let stored: StoredEvent =
            serde_json::from_value(value).map_err(|error| Error::CorruptLog {
                line: index + 1,
                reason: error.to_string(),
            })?;
        migrated.extend(serde_json::to_vec(&stored)?);
        migrated.push(b'\n');
        records += 1;
    }
    let validated = replay_bytes(&migrated)?;

    if let Some(parent) = destination.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut destination_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    if let Err(error) = destination_file
        .write_all(&migrated)
        .and_then(|()| destination_file.sync_data())
    {
        drop(destination_file);
        let _ = std::fs::remove_file(destination);
        return Err(Error::Io(error));
    }
    source_file.unlock()?;

    Ok(MigrationOutcome {
        from_schema,
        to_schema: SCHEMA_VERSION,
        revision: validated.state.revision,
        records,
    })
}

pub fn migrate_v1_to_v4(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> Result<MigrationOutcome> {
    migrate_to_current(source, destination, 1)
}

#[deprecated(note = "use migrate_v1_to_v4; the destination schema is now version 4")]
pub fn migrate_v1_to_v2(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> Result<MigrationOutcome> {
    migrate_v1_to_v4(source, destination)
}

#[deprecated(note = "use migrate_v1_to_v4; the destination schema is now version 4")]
pub fn migrate_v1_to_v3(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> Result<MigrationOutcome> {
    migrate_v1_to_v4(source, destination)
}

pub fn migrate_v2_to_v4(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> Result<MigrationOutcome> {
    migrate_to_current(source, destination, 2)
}

#[deprecated(note = "use migrate_v2_to_v4; the destination schema is now version 4")]
pub fn migrate_v2_to_v3(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> Result<MigrationOutcome> {
    migrate_v2_to_v4(source, destination)
}

pub fn migrate_v3_to_v4(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> Result<MigrationOutcome> {
    migrate_to_current(source, destination, 3)
}

pub fn commit(path: impl AsRef<Path>, request: CommitRequest) -> Result<CommitOutcome> {
    let path = path.as_ref();
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .open(path)?;
    file.lock()?;

    let result = commit_locked(&mut file, request);
    let unlock_result = file.unlock();
    match (result, unlock_result) {
        (Ok(outcome), Ok(())) => Ok(outcome),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(Error::Io(error)),
    }
}

fn commit_locked(file: &mut File, request: CommitRequest) -> Result<CommitOutcome> {
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let mut log = replay_bytes(&bytes)?;

    if let Some(existing) = log.find_idempotency_key(&request.idempotency_key) {
        if existing.request == request {
            return Ok(CommitOutcome {
                status: CommitStatus::Duplicate,
                revision: existing.sequence,
                evaluation: Evaluation::allow(),
            });
        }
        return Ok(CommitOutcome {
            status: CommitStatus::Rejected,
            revision: log.state.revision,
            evaluation: Evaluation::deny(
                "IDEMPOTENCY_KEY_CONFLICT",
                "idempotency key was already used for a different payload",
            ),
        });
    }

    if log.find_event_id(&request.event_id).is_some() {
        return Ok(CommitOutcome {
            status: CommitStatus::Rejected,
            revision: log.state.revision,
            evaluation: Evaluation::deny(
                "EVENT_ID_CONFLICT",
                "event id was already used by another commit",
            ),
        });
    }

    if request.expected_revision != log.state.revision {
        return Ok(CommitOutcome {
            status: CommitStatus::Rejected,
            revision: log.state.revision,
            evaluation: Evaluation::deny(
                "STALE_REVISION",
                format!(
                    "expected revision {}, current revision is {}",
                    request.expected_revision, log.state.revision
                ),
            ),
        });
    }

    let evaluation = evaluate(&log.state, &request);
    if evaluation.verdict != Verdict::Allow {
        return Ok(CommitOutcome {
            status: CommitStatus::Rejected,
            revision: log.state.revision,
            evaluation,
        });
    }

    let stored = StoredEvent {
        sequence: log.state.revision + 1,
        request,
    };
    log.state.apply(&stored)?;
    let mut record = serde_json::to_vec(&stored)?;
    record.push(b'\n');
    file.write_all(&record)?;
    file.sync_data()?;

    Ok(CommitOutcome {
        status: CommitStatus::Committed,
        revision: stored.sequence,
        evaluation,
    })
}
