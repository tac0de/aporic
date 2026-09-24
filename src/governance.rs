use crate::policy::{EffectiveToolPolicy, PolicyDocument};
use crate::{RiskLevel, State};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy)]
pub struct Action<'a> {
    pub scope: &'a str,
    pub session_id: &'a str,
    pub tool_name: &'a str,
    pub tool_use_id: Option<&'a str>,
    pub tool_input: Option<&'a Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    Allowed,
    Held,
    IntentBoundPlanRequired,
    PlanNotExecutionEligible,
    PlanAuthorizationRequired,
    ExecutionGrantRequired,
    ToolUseAlreadyConsumed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActionEvaluation {
    pub revision: u64,
    pub status: GateStatus,
    pub reason_code: &'static str,
    pub active_hold_count: usize,
    pub matching_plan_authorization_count: usize,
    pub matching_execution_grant_count: usize,
    pub selected_execution_grant_id: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct AuthorityMatches {
    plan_authorizations: usize,
    grants: usize,
    auto_allowed_plans: usize,
    raw_plan_authorizations: usize,
    raw_grants: usize,
    has_unready_governed_authority: bool,
}

fn plan_is_eligible(state: &State, plan_id: &str, scope: &str, require_intent: bool) -> bool {
    state.plan_is_execution_eligible(plan_id, scope, require_intent)
}

pub fn evaluate_action(
    state: &State,
    policy: &PolicyDocument,
    action: Action<'_>,
) -> Result<Option<ActionEvaluation>, &'static str> {
    if action.scope.trim().is_empty() || action.scope.len() > crate::codex::MAX_SCOPE_BYTES {
        return Err("scope must fit the byte limit");
    }
    if action.session_id.trim().is_empty()
        || action.session_id.len() > crate::codex::MAX_SESSION_ID_BYTES
    {
        return Err("session id must fit the byte limit");
    }
    let Some(tool_policy) = policy.effective_tool(action.tool_name) else {
        return Ok(None);
    };

    let active_hold_count = state
        .tool_holds
        .values()
        .filter(|hold| {
            hold.active && hold.scope == action.scope && hold.tool_name == action.tool_name
        })
        .count();
    let matching_plan_authorizations = state
        .plan_authorizations
        .values()
        .filter(|authorization| {
            authorization.active
                && authorization.scope == action.scope
                && authorization.session_id == action.session_id
                && authorization.tool_name == action.tool_name
        })
        .collect::<Vec<_>>();
    let raw_matching_plan_authorization_count = matching_plan_authorizations.len();
    let has_unready_governed_plan_authorization =
        matching_plan_authorizations.iter().any(|authorization| {
            state.governed_plans.contains_key(&authorization.plan_id)
                && !state.governed_plan_is_ready(&authorization.plan_id)
        });
    let matching_plan_authorization_count = matching_plan_authorizations
        .iter()
        .filter(|authorization| {
            plan_is_eligible(
                state,
                &authorization.plan_id,
                action.scope,
                tool_policy.require_intent,
            )
        })
        .count();
    let raw_matching_grants: Vec<_> = state
        .execution_grants
        .values()
        .filter(|grant| {
            grant.active
                && grant.consumed_uses < grant.max_uses
                && grant.scope == action.scope
                && grant.session_id == action.session_id
                && grant.tool_name == action.tool_name
                && action
                    .tool_input
                    .is_some_and(|input| input == &grant.tool_input)
        })
        .collect();
    let raw_matching_execution_grant_count = raw_matching_grants.len();
    let has_unready_governed_grant = raw_matching_grants.iter().any(|grant| {
        state.governed_plans.contains_key(&grant.plan_id)
            && !state.governed_plan_is_ready(&grant.plan_id)
    });
    let matching_grants = raw_matching_grants
        .into_iter()
        .filter(|grant| {
            plan_is_eligible(
                state,
                &grant.plan_id,
                action.scope,
                tool_policy.require_intent,
            )
        })
        .collect::<Vec<_>>();
    let matching_execution_grant_count = matching_grants.len();
    let selected_execution_grant_id = matching_grants.first().map(|grant| grant.id.clone());
    let auto_allowed_profiles = tool_policy
        .auto_allow_low_risk_profiles
        .iter()
        .map(|profile| {
            (
                profile.profile_id.as_str(),
                profile.profile_version.as_str(),
            )
        })
        .collect::<BTreeSet<_>>();
    let auto_allow_candidates = state
        .governed_plans
        .values()
        .filter(|governed| {
            governed.risk_snapshot.risk_level == RiskLevel::Low
                && auto_allowed_profiles.contains(&(
                    governed.risk_snapshot.profile_id.as_str(),
                    governed.risk_snapshot.profile_version.as_str(),
                ))
                && state
                    .plans
                    .get(&governed.id)
                    .is_some_and(|plan| plan.scope == action.scope)
        })
        .collect::<Vec<_>>();
    let matching_auto_allowed_plan_count = auto_allow_candidates
        .iter()
        .filter(|governed| state.governed_plan_is_ready(&governed.id))
        .count();
    let has_unready_auto_allow_candidate =
        auto_allow_candidates.len() > matching_auto_allowed_plan_count;
    let tool_use_already_consumed = action
        .tool_use_id
        .is_some_and(|id| state.consumed_tool_uses.contains_key(id));

    let (status, reason_code) = decide(
        &tool_policy,
        active_hold_count,
        AuthorityMatches {
            plan_authorizations: matching_plan_authorization_count,
            grants: matching_execution_grant_count,
            auto_allowed_plans: matching_auto_allowed_plan_count,
            raw_plan_authorizations: raw_matching_plan_authorization_count,
            raw_grants: raw_matching_execution_grant_count,
            has_unready_governed_authority: has_unready_governed_plan_authorization
                || has_unready_governed_grant
                || has_unready_auto_allow_candidate,
        },
        tool_use_already_consumed,
    );

    Ok(Some(ActionEvaluation {
        revision: state.revision,
        status,
        reason_code,
        active_hold_count,
        matching_plan_authorization_count,
        matching_execution_grant_count,
        selected_execution_grant_id,
    }))
}

fn decide(
    policy: &EffectiveToolPolicy<'_>,
    holds: usize,
    matches: AuthorityMatches,
    tool_use_already_consumed: bool,
) -> (GateStatus, &'static str) {
    if holds > 0 {
        (GateStatus::Held, "TOOL_HELD")
    } else if tool_use_already_consumed {
        (
            GateStatus::ToolUseAlreadyConsumed,
            "TOOL_USE_ALREADY_CONSUMED",
        )
    } else if policy.require_plan
        && matches.plan_authorizations == 0
        && matches.auto_allowed_plans == 0
        && !(policy.require_grant && matches.grants > 0)
    {
        if matches.has_unready_governed_authority {
            (
                GateStatus::PlanNotExecutionEligible,
                "PLAN_NOT_EXECUTION_ELIGIBLE",
            )
        } else if policy.require_intent
            && (matches.raw_plan_authorizations > 0
                || (policy.require_grant && matches.raw_grants > 0))
        {
            (
                GateStatus::IntentBoundPlanRequired,
                "INTENT_BOUND_PLAN_REQUIRED",
            )
        } else {
            (
                GateStatus::PlanAuthorizationRequired,
                "PLAN_AUTHORIZATION_REQUIRED",
            )
        }
    } else if policy.require_grant && matches.grants == 0 {
        if matches.has_unready_governed_authority {
            (
                GateStatus::PlanNotExecutionEligible,
                "PLAN_NOT_EXECUTION_ELIGIBLE",
            )
        } else if policy.require_intent && matches.raw_grants > 0 {
            (
                GateStatus::IntentBoundPlanRequired,
                "INTENT_BOUND_PLAN_REQUIRED",
            )
        } else {
            (
                GateStatus::ExecutionGrantRequired,
                "EXECUTION_GRANT_REQUIRED",
            )
        }
    } else {
        (GateStatus::Allowed, "ALLOW")
    }
}

pub fn input_identity(input: &Value) -> String {
    let bytes = serde_json::to_vec(input).expect("JSON value serialization is infallible");
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}
