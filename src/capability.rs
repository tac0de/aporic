use serde::Serialize;
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityEffect {
    ReadOnly,
    EvidenceWrite,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Capability {
    pub name: &'static str,
    pub description: &'static str,
    pub effect: CapabilityEffect,
    pub approval: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

pub fn registry() -> Vec<Capability> {
    vec![
        Capability {
            name: "capabilities",
            description: "List Aporic's bounded tool capabilities and their effect classes.",
            effect: CapabilityEffect::ReadOnly,
            approval: "approve",
            input_schema: object_schema(json!({}), &[]),
        },
        Capability {
            name: "project_status",
            description: "Read a count-only status summary for an Aporic-bound workspace.",
            effect: CapabilityEffect::ReadOnly,
            approval: "approve",
            input_schema: object_schema(
                json!({"workspace": {"type": "string", "minLength": 1}}),
                &["workspace"],
            ),
        },
        Capability {
            name: "explain_action",
            description: "Evaluate one exact tool action against the bound project's current policy without mutating state.",
            effect: CapabilityEffect::ReadOnly,
            approval: "approve",
            input_schema: object_schema(
                json!({
                    "workspace": {"type": "string", "minLength": 1},
                    "session_id": {"type": "string", "minLength": 1, "maxLength": 256},
                    "tool_name": {"type": "string", "minLength": 1, "maxLength": 256},
                    "tool_use_id": {"type": "string", "minLength": 1, "maxLength": 256},
                    "tool_input": {}
                }),
                &["workspace", "session_id", "tool_name"],
            ),
        },
        Capability {
            name: "ingest_verifier_report",
            description: "Record a bounded evidence-authored verifier report for an existing effect receipt and plan check.",
            effect: CapabilityEffect::EvidenceWrite,
            approval: "prompt",
            input_schema: object_schema(
                json!({
                    "workspace": {"type": "string", "minLength": 1},
                    "schema_version": {"type": "integer", "const": 1},
                    "verification_id": {"type": "string", "minLength": 1},
                    "receipt_id": {"type": "string", "minLength": 1},
                    "verifier_id": {"type": "string", "minLength": 1},
                    "provenance": {"type": "string", "minLength": 1},
                    "plan_id": {"type": "string", "minLength": 1},
                    "check_index": {"type": "integer", "minimum": 0},
                    "result": {"enum": ["passed", "failed", "inconclusive"]},
                    "evidence_refs": {"type": "array", "minItems": 1, "items": {"type": "string", "minLength": 1}}
                }),
                &[
                    "workspace",
                    "schema_version",
                    "verification_id",
                    "receipt_id",
                    "verifier_id",
                    "provenance",
                    "plan_id",
                    "check_index",
                    "result",
                    "evidence_refs",
                ],
            ),
        },
    ]
}

fn object_schema(properties: Value, required: &[&str]) -> Value {
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}
