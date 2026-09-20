use crate::State;
use crate::policy::{PolicyDocument, ToolPolicy};
use serde::Serialize;
use serde_json::Value;

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
    let Some(tool_policy) = policy.tool(action.tool_name) else {
        return Ok(None);
    };

    let active_hold_count = state
        .tool_holds
        .values()
        .filter(|hold| {
            hold.active && hold.scope == action.scope && hold.tool_name == action.tool_name
        })
        .count();
    let matching_plan_authorization_count = state
        .plan_authorizations
        .values()
        .filter(|authorization| {
            authorization.active
                && authorization.scope == action.scope
                && authorization.session_id == action.session_id
                && authorization.tool_name == action.tool_name
        })
        .count();
    let matching_grants: Vec<_> = state
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
    let matching_execution_grant_count = matching_grants.len();
    let selected_execution_grant_id = matching_grants.first().map(|grant| grant.id.clone());
    let tool_use_already_consumed = action
        .tool_use_id
        .is_some_and(|id| state.consumed_tool_uses.contains_key(id));

    let (status, reason_code) = decide(
        tool_policy,
        active_hold_count,
        matching_plan_authorization_count,
        matching_execution_grant_count,
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
    policy: &ToolPolicy,
    holds: usize,
    plan_authorizations: usize,
    grants: usize,
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
        && plan_authorizations == 0
        && !(policy.require_grant && grants > 0)
    {
        (
            GateStatus::PlanAuthorizationRequired,
            "PLAN_AUTHORIZATION_REQUIRED",
        )
    } else if policy.require_grant && grants == 0 {
        (
            GateStatus::ExecutionGrantRequired,
            "EXECUTION_GRANT_REQUIRED",
        )
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
