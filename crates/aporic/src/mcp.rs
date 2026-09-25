use rmcp::{ServerHandler, handler::server::wrapper::Parameters, tool, tool_handler, tool_router};
use serde::Serialize;

use crate::{
    Hub,
    domain::{CloseRequest, OpenRequest, RecallRequest, RecordRequest},
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
}

#[tool_handler(
    name = "aporic",
    version = "0.1.0",
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
