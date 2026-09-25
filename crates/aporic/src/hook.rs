use serde_json::{Value, json};

use crate::{Hub, context, domain::RecallRequest};

const HOOK_CONTEXT_BYTES: usize = 8_192;

/// Handles one Codex lifecycle hook without writing hook input to Aporic.
/// Any malformed or unavailable state fails open with an empty JSON object.
pub fn handle_codex_hook(hub: &Hub, input: &str) -> Value {
    let Ok(event) = serde_json::from_str::<Value>(input) else {
        return json!({});
    };
    let event_name =
        string_field(&event, &["hook_event_name", "event_name", "event"]).unwrap_or_default();
    if !matches!(event_name, "SessionStart" | "UserPromptSubmit") {
        return json!({});
    }
    let Some(workspace) = string_field(&event, &["cwd", "workspace"]) else {
        return json!({});
    };
    let objective = if event_name == "UserPromptSubmit" {
        string_field(&event, &["prompt", "user_prompt"]).map(str::to_owned)
    } else {
        None
    };
    let request = RecallRequest {
        workspace: workspace.to_owned(),
        limit: Some(24),
        objective,
        focus_paths: Vec::new(),
        max_bytes: Some(6_144),
    };
    let Ok(capsule) = hub.recall(&request) else {
        return json!({});
    };
    let mut additional_context =
        context::render_for_model(&capsule.selected_items, HOOK_CONTEXT_BYTES);
    let observation = json!({
        "item_type": "host_observation",
        "origin_channel": "codex_hook",
        "influence_class": "historical_context",
        "attestation": false,
        "model": string_field(&event, &["model"]),
        "permission_mode": string_field(&event, &["permission_mode"]),
        "event": event_name,
    });
    let observation_line = observation.to_string() + "\n";
    if additional_context.len() + observation_line.len() <= HOOK_CONTEXT_BYTES {
        additional_context.push_str(&observation_line);
    }
    json!({
        "hookSpecificOutput": {
            "hookEventName": event_name,
            "additionalContext": additional_context
        }
    })
}

fn string_field<'a>(value: &'a Value, names: &[&str]) -> Option<&'a str> {
    names
        .iter()
        .find_map(|name| value.get(*name).and_then(Value::as_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_and_irrelevant_events_fail_open() {
        let directory = tempfile::tempdir().unwrap();
        let hub = Hub::open(directory.path().join("aporic.sqlite3")).unwrap();
        assert_eq!(handle_codex_hook(&hub, "not-json"), json!({}));
        assert_eq!(
            handle_codex_hook(
                &hub,
                &json!({"hook_event_name":"Stop","cwd":directory.path()}).to_string()
            ),
            json!({})
        );
    }

    #[test]
    fn prompt_is_used_ephemerally_and_not_echoed() {
        let directory = tempfile::tempdir().unwrap();
        let hub = Hub::open(directory.path().join("aporic.sqlite3")).unwrap();
        let secret_prompt = "do not persist this prompt 8f28b9";
        let response = handle_codex_hook(
            &hub,
            &json!({
                "hook_event_name": "UserPromptSubmit",
                "cwd": directory.path(),
                "prompt": secret_prompt,
                "transcript_path": "/private/transcript.jsonl",
                "last_assistant_message": "private output",
                "model": "gpt-6-astra",
                "permission_mode": "workspace-write"
            })
            .to_string(),
        );
        let rendered = response["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap();
        assert!(!rendered.contains(secret_prompt));
        assert!(!rendered.contains("transcript.jsonl"));
        assert!(!rendered.contains("private output"));
        assert!(rendered.contains("\"attestation\":false"));
    }
}
