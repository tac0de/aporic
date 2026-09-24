use crate::capability::{Capability, CapabilityEffect, registry};
use crate::codex::{GatePolicy, PreToolUseInput, explain_action};
use crate::policy::read_policy_document;
use crate::project::{default_data_root, discover_project};
use crate::verifier::{VerifierReportInput, ingest_verifier_report};
use crate::{SCHEMA_VERSION, State, load_nonblocking};
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

pub const MAX_MCP_MESSAGE_BYTES: usize = 1_048_576;
const MCP_PROTOCOL_VERSION: &str = "2025-06-18";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceInput {
    workspace: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExplainInput {
    workspace: String,
    session_id: String,
    tool_name: String,
    #[serde(default = "default_tool_use_id")]
    tool_use_id: String,
    #[serde(default)]
    tool_input: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifierToolInput {
    workspace: String,
    #[serde(flatten)]
    report: VerifierReportInput,
}

fn default_tool_use_id() -> String {
    "aporic-explain".into()
}

pub fn serve_stdio() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    serve(&mut reader, &mut writer)
}

fn serve(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut initialized = false;
    loop {
        let line = match read_bounded_line(reader) {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                serde_json::to_writer(
                    &mut *writer,
                    &rpc_error(Value::Null, -32700, "MCP request exceeds the byte limit"),
                )?;
                writer.write_all(b"\n")?;
                writer.flush()?;
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let response = match serde_json::from_slice::<Value>(&line) {
            Ok(request) => handle_request(request, &mut initialized),
            Err(error) => Some(rpc_error(
                Value::Null,
                -32700,
                format!("parse error: {error}"),
            )),
        };
        if let Some(response) = response {
            serde_json::to_writer(&mut *writer, &response)?;
            writer.write_all(b"\n")?;
            writer.flush()?;
        }
    }
    Ok(())
}

fn read_bounded_line(reader: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Ok(Some(line))
            };
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let consumed = newline.map_or(available.len(), |index| index + 1);
        if line.len().saturating_add(consumed) > MAX_MCP_MESSAGE_BYTES {
            reader.consume(consumed);
            while newline.is_none() {
                let rest = reader.fill_buf()?;
                if rest.is_empty() {
                    break;
                }
                let rest_newline = rest.iter().position(|byte| *byte == b'\n');
                let rest_consumed = rest_newline.map_or(rest.len(), |index| index + 1);
                reader.consume(rest_consumed);
                if rest_newline.is_some() {
                    break;
                }
            }
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("MCP message exceeds {MAX_MCP_MESSAGE_BYTES} bytes"),
            ));
        }
        line.extend_from_slice(&available[..consumed]);
        reader.consume(consumed);
        if newline.is_some() {
            if line.last() == Some(&b'\n') {
                line.pop();
            }
            return Ok(Some(line));
        }
    }
}

fn handle_request(request: Value, initialized: &mut bool) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = request.get("method").and_then(Value::as_str);
    if request.get("jsonrpc") != Some(&Value::String("2.0".into())) {
        return id.map(|id| rpc_error(id, -32600, "invalid JSON-RPC version"));
    }
    let id = id?;
    if !matches!(id, Value::String(_) | Value::Number(_)) {
        return Some(rpc_error(Value::Null, -32600, "invalid request id"));
    }
    let result = match method {
        Some("initialize") => {
            *initialized = true;
            Ok(json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {"name": "aporic", "version": env!("CARGO_PKG_VERSION")}
            }))
        }
        Some("ping") => Ok(json!({})),
        Some("tools/list" | "tools/call") if !*initialized => {
            return Some(rpc_error(id, -32002, "server is not initialized"));
        }
        Some("tools/list") => Ok(json!({
            "tools": registry().iter().map(mcp_tool).collect::<Vec<_>>()
        })),
        Some("tools/call") => call_tool(request.get("params")),
        Some(_) => return Some(rpc_error(id, -32601, "method not found")),
        None => return Some(rpc_error(id, -32600, "invalid request")),
    };
    Some(match result {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err(message) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {"content": [{"type": "text", "text": message}], "isError": true}
        }),
    })
}

fn mcp_tool(capability: &Capability) -> Value {
    json!({
        "name": capability.name,
        "description": capability.description,
        "inputSchema": capability.input_schema,
        "annotations": {
            "readOnlyHint": capability.effect == CapabilityEffect::ReadOnly,
            "destructiveHint": false,
            "idempotentHint": true,
            "openWorldHint": false
        }
    })
}

fn call_tool(params: Option<&Value>) -> Result<Value, String> {
    let params = params.ok_or("missing tool call parameters")?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or("missing tool name")?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let value = match name {
        "capabilities" => {
            serde_json::from_value::<EmptyInput>(arguments).map_err(invalid_input)?;
            serde_json::to_value(registry()).map_err(|error| error.to_string())?
        }
        "project_status" => {
            project_status(serde_json::from_value(arguments).map_err(invalid_input)?)?
        }
        "explain_action" => explain(serde_json::from_value(arguments).map_err(invalid_input)?)?,
        "ingest_verifier_report" => {
            ingest(serde_json::from_value(arguments).map_err(invalid_input)?)?
        }
        _ => return Err(format!("unknown Aporic tool {name:?}")),
    };
    let text = serde_json::to_string(&value).map_err(|error| error.to_string())?;
    Ok(json!({
        "content": [{"type": "text", "text": text}],
        "structuredContent": value,
        "isError": false
    }))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyInput {}

fn invalid_input(error: serde_json::Error) -> String {
    format!("invalid tool input: {error}")
}

fn resolve(workspace: &str) -> Result<crate::project::ResolvedProject, String> {
    let workspace = PathBuf::from(workspace);
    if !workspace.is_absolute() {
        return Err("workspace must be absolute".into());
    }
    discover_project(
        &workspace,
        default_data_root().map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?
    .ok_or_else(|| "workspace is not bound to Aporic".into())
}

fn state_count(state: &State, scope: &str) -> Value {
    json!({
        "active_intents": state.intents.values().filter(|item| item.scope == scope && item.superseded_by.is_none()).count(),
        "plans": state.plans.values().filter(|item| item.scope == scope).count(),
        "active_plan_authorizations": state.plan_authorizations.values().filter(|item| item.scope == scope && item.active).count(),
        "active_execution_grants": state.execution_grants.values().filter(|item| item.scope == scope && item.active && item.consumed_uses < item.max_uses).count(),
        "active_holds": state.tool_holds.values().filter(|item| item.scope == scope && item.active).count(),
        "effect_receipts": state.effect_receipts.values().filter(|item| item.scope == scope).count(),
        "effect_verifications": state.verifications.values().filter(|item| item.scope == scope && item.effect_receipt_id.is_some()).count()
    })
}

fn project_status(input: WorkspaceInput) -> Result<Value, String> {
    let project = resolve(&input.workspace)?;
    let log = load_nonblocking(&project.store_path).map_err(|error| error.to_string())?;
    let policy = read_policy_document(&project.policy_path)?;
    policy.validate()?;
    Ok(json!({
        "workspace": project.workspace,
        "scope": project.scope,
        "revision": log.state().revision,
        "event_schema": SCHEMA_VERSION,
        "policy_schema": policy.schema_version,
        "lifecycle_mode": policy.lifecycle_mode(),
        "protected_tool_count": policy.tools.len(),
        "counts": state_count(log.state(), &project.scope)
    }))
}

fn explain(input: ExplainInput) -> Result<Value, String> {
    let project = resolve(&input.workspace)?;
    let policy = read_policy_document(&project.policy_path)?;
    let policy = GatePolicy::from_document(policy)?;
    let log = load_nonblocking(&project.store_path).map_err(|error| error.to_string())?;
    let hook_input = PreToolUseInput {
        session_id: input.session_id,
        hook_event_name: "PreToolUse".into(),
        cwd: project.workspace.to_string_lossy().into_owned(),
        turn_id: "aporic-mcp-explain".into(),
        tool_name: input.tool_name,
        tool_use_id: input.tool_use_id,
        tool_input: input.tool_input,
        model: None,
        permission_mode: None,
    };
    serde_json::to_value(
        explain_action(log.state(), &hook_input, &project.scope, &policy)
            .map_err(str::to_string)?,
    )
    .map_err(|error| error.to_string())
}

fn ingest(input: VerifierToolInput) -> Result<Value, String> {
    let project = resolve(&input.workspace)?;
    ingest_verifier_report(&project.store_path, &project.scope, &input.report)
        .map_err(|error| error.to_string())?;
    Ok(json!({
        "status": "accepted",
        "verification_id": input.report.verification_id
    }))
}

fn rpc_error(id: Value, code: i32, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message.into()}
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn registry_names_are_unique_and_only_verifier_ingestion_writes() {
        let capabilities = registry();
        for (index, capability) in capabilities.iter().enumerate() {
            assert!(
                !capabilities[..index]
                    .iter()
                    .any(|other| other.name == capability.name)
            );
        }
        assert_eq!(
            capabilities
                .iter()
                .filter(|capability| capability.effect != CapabilityEffect::ReadOnly)
                .map(|capability| capability.name)
                .collect::<Vec<_>>(),
            ["ingest_verifier_report"]
        );
    }

    #[test]
    fn bounded_reader_rejects_oversized_lines() {
        let input = vec![b'x'; MAX_MCP_MESSAGE_BYTES + 1];
        assert_eq!(
            read_bounded_line(&mut Cursor::new(input))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn unknown_methods_fail_closed() {
        let response = handle_request(
            json!({"jsonrpc": "2.0", "id": 7, "method": "authority/grant"}),
            &mut true,
        )
        .unwrap();
        assert_eq!(response["error"]["code"], -32601);
    }

    #[test]
    fn verifier_tool_input_accepts_only_workspace_and_report_fields() {
        let input: VerifierToolInput = serde_json::from_value(json!({
            "workspace": "/tmp/repo",
            "schema_version": 1,
            "verification_id": "verification",
            "receipt_id": "receipt",
            "verifier_id": "verifier",
            "provenance": "test",
            "plan_id": "plan",
            "check_index": 0,
            "result": "passed",
            "evidence_refs": ["evidence"]
        }))
        .unwrap();
        assert_eq!(input.report.verification_id, "verification");
        assert!(
            serde_json::from_value::<VerifierToolInput>(json!({
                "workspace": "/tmp/repo",
                "schema_version": 1,
                "verification_id": "verification",
                "receipt_id": "receipt",
                "verifier_id": "verifier",
                "provenance": "test",
                "plan_id": "plan",
                "check_index": 0,
                "result": "passed",
                "evidence_refs": ["evidence"],
                "authority_ref": "must be rejected"
            }))
            .is_err()
        );
    }

    #[test]
    fn oversized_request_is_bounded_and_does_not_stop_the_server() {
        let mut input = vec![b'x'; MAX_MCP_MESSAGE_BYTES + 1];
        input.push(b'\n');
        input.extend_from_slice(br#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#);
        input.push(b'\n');
        let mut output = Vec::new();
        serve(&mut Cursor::new(input), &mut output).unwrap();
        let responses = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0]["error"]["code"], -32700);
        assert_eq!(responses[1]["result"]["serverInfo"]["name"], "aporic");
    }

    #[test]
    fn tools_require_a_valid_version_and_initialize_request() {
        let mut initialized = false;
        let before_initialize = handle_request(
            json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
            &mut initialized,
        )
        .unwrap();
        assert_eq!(before_initialize["error"]["code"], -32002);
        let invalid_version = handle_request(
            json!({"jsonrpc": "1.0", "id": 2, "method": "initialize"}),
            &mut initialized,
        )
        .unwrap();
        assert_eq!(invalid_version["error"]["code"], -32600);
        assert!(!initialized);
    }
}
