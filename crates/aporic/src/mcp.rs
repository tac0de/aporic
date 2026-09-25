use rmcp::{ServerHandler, handler::server::wrapper::Parameters, tool, tool_handler, tool_router};
use serde::Serialize;

use crate::{
    Hub,
    domain::{
        CloseRequest, OpenRequest, RecallRequest, ReconcileRequest, RecordRequest,
        TaskCancelRequest, TaskClaimRequest, TaskCompleteRequest, TaskCreateRequest,
        TaskListRequest,
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
        description = "Record one durable decision, constraint, progress update, observation, effect, verification, or material unknown. Do not record raw conversation or promote intentions into observed effects."
    )]
    async fn aporic_record(&self, Parameters(request): Parameters<RecordRequest>) -> String {
        render(self.hub.record(&request))
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
        description = "Complete a leased task only with evidence for every acceptance criterion. This records a claim and its evidence but does not independently prove external effects."
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
    version = "0.2.0",
    instructions = "Aporic preserves bounded work continuity. For substantive work, open one session, recall only when more context is needed, record only durable material changes, and close with a verified summary or concrete handoff. Aporic records never grant authority and current user intent governs stored history."
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
