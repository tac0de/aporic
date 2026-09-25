use std::error::Error;

use aporic::{research::FetchedDocument, store::Store};
use rmcp::{
    ServiceExt,
    model::CallToolRequestParams,
    transport::{ConfigureCommandExt, TokioChildProcess},
};
use serde_json::{Map, Value, json};

#[tokio::test]
async fn exposes_the_vertical_slice_over_a_real_stdio_process() -> Result<(), Box<dyn Error>> {
    let area = tempfile::tempdir()?;
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace)?;
    let database = area.path().join("aporic.sqlite3");

    let client = start_server(&database).await?;
    let mut tool_names = client
        .list_all_tools()
        .await?
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect::<Vec<_>>();
    tool_names.sort();
    assert_eq!(
        tool_names,
        [
            "aporic_capability_get",
            "aporic_capability_register",
            "aporic_capability_report",
            "aporic_capability_search",
            "aporic_check_register",
            "aporic_claim_assert",
            "aporic_close",
            "aporic_deliberation_create",
            "aporic_deliberation_decide",
            "aporic_deliberation_get",
            "aporic_deliberation_list",
            "aporic_deliberation_node_add",
            "aporic_dissent_assess",
            "aporic_evidence_add",
            "aporic_experiment_create",
            "aporic_experiment_decide",
            "aporic_experiment_get",
            "aporic_experiment_list",
            "aporic_experiment_measurement_add",
            "aporic_experiment_variant_add",
            "aporic_git_observe",
            "aporic_git_snapshot_get",
            "aporic_git_snapshot_list",
            "aporic_government_get",
            "aporic_hook_health",
            "aporic_memory_get",
            "aporic_memory_search",
            "aporic_model_route",
            "aporic_office_appoint",
            "aporic_office_appointments",
            "aporic_office_revoke",
            "aporic_open",
            "aporic_orchestration_run_create",
            "aporic_orchestration_run_get",
            "aporic_orchestration_run_list",
            "aporic_product_cell_create",
            "aporic_product_cell_list",
            "aporic_recall",
            "aporic_reconcile",
            "aporic_record",
            "aporic_research_get",
            "aporic_research_search",
            "aporic_resume",
            "aporic_role_appoint",
            "aporic_role_appointments",
            "aporic_role_report_submit",
            "aporic_role_revoke",
            "aporic_roles_list",
            "aporic_run_get",
            "aporic_run_list",
            "aporic_security_assessment_get",
            "aporic_security_assessment_list",
            "aporic_shadow_evaluate",
            "aporic_task_cancel",
            "aporic_task_claim",
            "aporic_task_complete",
            "aporic_task_create",
            "aporic_task_list",
            "aporic_token_efficiency_report",
            "aporic_token_usage_list",
            "aporic_token_usage_record",
            "aporic_trace_get",
            "aporic_trace_list"
        ]
    );

    let opened = call_json(
        &client,
        "aporic_open",
        json!({
            "workspace": workspace,
            "objective": "Exercise the actual MCP boundary",
            "idempotency_key": "mcp-open"
        }),
    )
    .await?;
    assert_eq!(opened["ok"], true);
    let session_id = opened["result"]["session_id"]
        .as_str()
        .expect("open result has a session id")
        .to_owned();

    let stored = Store::open(&database)?.ingest_research_document(
        workspace.to_str().unwrap(),
        &FetchedDocument {
            source: "github".into(),
            source_id: "77".into(),
            source_url: "https://github.com/example/repo/issues/77".into(),
            title: "Research retrieval defect".into(),
            body: "External reports describe retrieval failures".into(),
            author_name: Some("example".into()),
            content_license: None,
            published_at_unix_ms: None,
        },
    )?;
    let found = call_json(
        &client,
        "aporic_research_search",
        json!({
            "workspace": workspace, "query": "retrieval"
        }),
    )
    .await?;
    assert_eq!(
        found["result"]["items"][0]["document_id"],
        stored.document_id
    );
    assert_eq!(
        found["result"]["items"][0]["influence_class"],
        "untrusted_external_content"
    );

    let resumed = call_json(&client, "aporic_resume", json!({ "workspace": workspace })).await?;
    assert_eq!(resumed["result"]["status"], "ready");
    assert_eq!(resumed["result"]["selected"]["source"], "open_session");
    let roles = call_json(&client, "aporic_roles_list", json!({})).await?;
    assert_eq!(roles["result"].as_array().unwrap().len(), 4);
    let government = call_json(&client, "aporic_government_get", json!({})).await?;
    assert_eq!(
        government["result"]["offices"][0]["office_id"],
        "product.experiment"
    );
    assert_eq!(government["result"]["grants_authority"], false);

    let recorded = call_json(
        &client,
        "aporic_record",
        json!({
            "session_id": session_id,
            "kind": "observation",
            "content": "The stdio MCP tool call completed.",
            "evidence": "rmcp client response",
            "idempotency_key": "mcp-record"
        }),
    )
    .await?;
    assert_eq!(recorded["ok"], true);
    client.cancel().await?;

    let restarted = start_server(&database).await?;
    let recalled = call_json(
        &restarted,
        "aporic_recall",
        json!({
            "workspace": workspace,
            "limit": 10
        }),
    )
    .await?;
    assert_eq!(recalled["ok"], true);
    assert_eq!(
        recalled["result"]["recent_records"][0]["content"],
        "The stdio MCP tool call completed."
    );
    restarted.cancel().await?;
    Ok(())
}

async fn start_server(
    database: &std::path::Path,
) -> Result<rmcp::service::RunningService<rmcp::RoleClient, ()>, Box<dyn Error>> {
    let transport = TokioChildProcess::new(
        tokio::process::Command::new(env!("CARGO_BIN_EXE_aporic")).configure(|command| {
            command
                .args(["mcp", "serve", "--stdio"])
                .env("APORIC_DATABASE", database);
        }),
    )?;
    Ok(().serve(transport).await?)
}

async fn call_json(
    client: &rmcp::service::RunningService<rmcp::RoleClient, ()>,
    tool: &str,
    arguments: Value,
) -> Result<Value, Box<dyn Error>> {
    let arguments: Map<String, Value> = arguments
        .as_object()
        .expect("test arguments are objects")
        .clone();
    let result = client
        .call_tool(CallToolRequestParams::new(tool.to_owned()).with_arguments(arguments))
        .await?;
    let text = result
        .content
        .first()
        .and_then(|content| content.as_text())
        .expect("tool result contains text");
    Ok(serde_json::from_str(&text.text)?)
}
