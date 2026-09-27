use std::error::Error;

use aporic::{research::FetchedDocument, store::Store};
use rmcp::{
    ServiceExt,
    model::CallToolRequestParams,
    transport::{ConfigureCommandExt, TokioChildProcess},
};
use serde_json::{Map, Value, json};

#[tokio::test]
async fn session_delegation_is_available_without_creating_a_task() -> Result<(), Box<dyn Error>> {
    let area = tempfile::tempdir()?;
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace)?;
    let database = area.path().join("aporic.sqlite3");
    let client = start_server(&database).await?;
    let opened = call_json(
        &client,
        "aporic_open",
        json!({"workspace": workspace, "objective": "Two independent paths", "idempotency_key": "session-delegation-open"}),
    )
    .await?;
    assert_eq!(opened["ok"], true);
    assert_eq!(
        opened["result"]["session_delegation"]["assessment_missing"],
        true
    );
    let session_id = opened["result"]["session_id"].as_str().unwrap();
    let decision = call_json(
        &client,
        "aporic_session_delegation_assess",
        json!({
            "session_id": session_id,
            "parallel_paths": 2,
            "material_change": true,
            "worker": {"disposition": "delegate", "reason": "Independent code and documentation paths"},
            "reviewer": {"disposition": "delegate", "reason": "Material core change"},
            "idempotency_key": "session-delegation-assess"
        }),
    )
    .await?;
    assert_eq!(decision["ok"], true);
    let decision_id = decision["result"]["decision"]["decision_id"]
        .as_str()
        .unwrap();
    let status = call_json(
        &client,
        "aporic_session_delegation_status",
        json!({"workspace": workspace, "session_id": session_id}),
    )
    .await?;
    assert_eq!(status["ok"], true);
    assert_eq!(status["result"]["assessment_missing"], false);
    assert_eq!(status["result"]["executable"], false);
    let reported = call_json(
        &client,
        "aporic_session_delegation_report",
        json!({
            "session_id": session_id,
            "decision_id": decision_id,
            "dimension": "worker",
            "host_agent_id": "host-agent-1",
            "model": "gpt-6-sol",
            "reasoning_effort": "high",
            "outcome": "started",
            "result_summary": "Host reported an actual start",
            "idempotency_key": "session-delegation-start"
        }),
    )
    .await?;
    assert_eq!(reported["ok"], true);
    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn exposes_frontend_browser_module_over_stdio() -> Result<(), Box<dyn Error>> {
    let area = tempfile::tempdir()?;
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace)?;
    let database = area.path().join("aporic.sqlite3");
    let client = start_server(&database).await?;
    let opened = call_json(
        &client,
        "aporic_open",
        json!({"workspace": workspace, "objective": "Frontend module", "idempotency_key": "frontend-open"}),
    )
    .await?;
    assert_eq!(opened["ok"], true);
    let session_id = opened["result"]["session_id"].as_str().unwrap();
    let task = call_json(
        &client,
        "aporic_task_create",
        json!({"session_id": session_id, "objective": "Review browser flow", "acceptance_criteria": ["Browser reviewed"], "idempotency_key": "frontend-task"}),
    )
    .await?;
    let task_id = task["result"]["task"]["task_id"].as_str().unwrap();
    let plan = call_json(
        &client,
        "aporic_workflow_plan",
        json!({
            "task_id": task_id, "objective": "Review browser flow", "target_user": "browser user",
            "constraints": "local", "success_measure": "flow reviewed",
            "requires_user_decision": false, "material_change": true,
            "procedure_profile": "frontend", "procedure_depth": "standard",
            "idempotency_key": "frontend-plan"
        }),
    )
    .await?;
    assert_eq!(plan["ok"], true);
    let steps = call_json(
        &client,
        "aporic_workflow_steps",
        json!({"workspace": workspace, "task_id": task_id}),
    )
    .await?;
    assert_eq!(steps["result"]["template_version"], 2);
    assert!(
        steps["result"]["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|step| step["step_id"] == "rendered_browser_review"
                && step["module_id"] == "frontend.browser_review")
    );
    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn exposes_the_vertical_slice_over_a_real_stdio_process() -> Result<(), Box<dyn Error>> {
    let area = tempfile::tempdir()?;
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace)?;
    let database = area.path().join("aporic.sqlite3");

    let client = start_server(&database).await?;
    let tool_names = client
        .list_all_tools()
        .await?
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect::<Vec<_>>();
    assert_eq!(tool_names.len(), 51, "default MCP surface changed");
    for required in [
        "aporic_open",
        "aporic_recall",
        "aporic_resume",
        "aporic_record",
        "aporic_close",
        "aporic_evidence_add",
        "aporic_claim_assert",
        "aporic_task_create",
        "aporic_intake_create",
        "aporic_intake_get",
        "aporic_rust_repair_open",
        "aporic_rust_repair_learn",
        "aporic_rust_repair_search",
        "aporic_task_work_packet",
        "aporic_workflow_plan",
        "aporic_workflow_advance",
        "aporic_initiative_plan",
        "aporic_initiative_artifact_record",
        "aporic_initiative_task_link",
        "aporic_initiative_status",
        "aporic_session_delegation_assess",
        "aporic_research_search",
        "aporic_design_validate",
        "aporic_run_list",
        "aporic_git_snapshot_list",
    ] {
        assert!(
            tool_names.iter().any(|name| name == required),
            "missing {required}"
        );
    }
    for retired_or_optional in [
        "aporic_prompt_trial_record",
        "aporic_prompt_compare",
        "aporic_deliberation_create",
        "aporic_capability_register",
        "aporic_experiment_create",
        "aporic_security_assessment_get",
        "aporic_improvement_submit",
        "aporic_prototype_brief_create",
        "aporic_trace_list",
        "aporic_token_usage_record",
        "aporic_accountability_open",
    ] {
        assert!(
            !tool_names.iter().any(|name| name == retired_or_optional),
            "default profile unexpectedly exposes {retired_or_optional}"
        );
    }
    assert!(
        client
            .call_tool(CallToolRequestParams::new("aporic_trace_list"))
            .await
            .is_err(),
        "core profile must reject calls to hidden optional tools"
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
    std::fs::write(
        workspace.join("design.json"),
        serde_json::to_vec(&json!({
            "schema_version": 1,
            "task_id": "stdio-design",
            "objective": "Exercise design validation",
            "target_user": "Contributor",
            "references": [],
            "selected_direction": null,
            "artifacts": []
        }))?,
    )?;
    let design = call_json(
        &client,
        "aporic_design_validate",
        json!({"workspace": workspace, "manifest": "design.json"}),
    )
    .await?;
    assert_eq!(design["ok"], true);
    assert_eq!(design["result"]["design_artifacts_complete"], false);
    let session_id = opened["result"]["session_id"]
        .as_str()
        .expect("open result has a session id")
        .to_owned();

    let initiative = call_json(
        &client,
        "aporic_initiative_plan",
        json!({
            "session_id": session_id,
            "objective": "Track a product outcome",
            "target_user": "Contributor",
            "success_measure": "One verified scenario",
            "requirements": [{
                "requirement_id": "R1",
                "statement": "Scenario works",
                "acceptance_criterion": "Scenario verified"
            }],
            "idempotency_key": "mcp-initiative"
        }),
    )
    .await?;
    assert_eq!(initiative["ok"], true);
    let initiative_id = initiative["result"]["status"]["initiative_id"]
        .as_str()
        .unwrap();
    let initiative_status = call_json(
        &client,
        "aporic_initiative_status",
        json!({"workspace": workspace, "initiative_id": initiative_id}),
    )
    .await?;
    assert_eq!(initiative_status["result"]["ready_to_claim"], false);

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
    let rejected_fetch = call_json(
        &client,
        "aporic_research_fetch",
        json!({
            "workspace": workspace,
            "task_id": "missing-task",
            "source": "github",
            "query": "retrieval"
        }),
    )
    .await?;
    assert_eq!(rejected_fetch["ok"], false);
    let resumed = call_json(&client, "aporic_resume", json!({ "workspace": workspace })).await?;
    assert_eq!(resumed["result"]["status"], "ready");
    assert_eq!(resumed["result"]["selected"]["source"], "open_session");
    let research_task = call_json(
        &client,
        "aporic_task_create",
        json!({
            "session_id": session_id,
            "objective": "Review research",
            "acceptance_criteria": ["Cite one source"],
            "idempotency_key": "mcp-research-task"
        }),
    )
    .await?;
    let research_task_id = research_task["result"]["task"]["task_id"].as_str().unwrap();
    let attached = call_json(
        &client,
        "aporic_task_research_attach",
        json!({
            "workspace": workspace,
            "task_id": research_task_id,
            "revision_id": stored.revision_id,
            "relevance_note": "Retrieval failure report",
            "idempotency_key": "mcp-research-attach"
        }),
    )
    .await?;
    assert_eq!(attached["ok"], true);
    let linked = call_json(
        &client,
        "aporic_task_research_list",
        json!({"workspace": workspace, "task_id": research_task_id}),
    )
    .await?;
    assert_eq!(linked["result"].as_array().unwrap().len(), 1);
    assert_eq!(linked["result"][0]["provenance"], "aporic_api");

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
    let memory_id = format!(
        "record:{}",
        recorded["result"]["record"]["record_id"].as_str().unwrap()
    );
    let task = call_json(
        &client,
        "aporic_task_create",
        json!({
            "session_id": session_id,
            "objective": "Check that a recalled observation is applied",
            "acceptance_criteria": ["The observation is reviewed"],
            "idempotency_key": "mcp-memory-task"
        }),
    )
    .await?;
    let task_id = task["result"]["task"]["task_id"].as_str().unwrap();
    let workflow = call_json(
        &client,
        "aporic_workflow_status",
        json!({"workspace": workspace, "task_id": task_id}),
    )
    .await?;
    assert_eq!(
        workflow["result"]["missing_for_next_stage"][0],
        "workflow_plan_missing"
    );
    let planned = call_json(
        &client,
        "aporic_workflow_plan",
        json!({
            "task_id": task_id, "objective": "Review observation", "target_user": "maintainer",
            "constraints": "local", "success_measure": "observation reviewed",
            "material_unknowns": ["selection pending"], "requires_user_decision": true,
            "material_change": true, "procedure_profile": "general",
            "procedure_depth": "light", "idempotency_key": "mcp-workflow-plan"
        }),
    )
    .await?;
    assert_eq!(planned["ok"], true);
    let steps = call_json(
        &client,
        "aporic_workflow_steps",
        json!({"workspace": workspace, "task_id": task_id}),
    )
    .await?;
    assert_eq!(steps["result"]["template_version"], 1);
    assert_eq!(
        steps["result"]["definitions"][0]["step_id"],
        "problem_and_outcome"
    );
    let evidence = call_json(
        &client,
        "aporic_evidence_add",
        json!({
            "session_id": session_id, "kind": "workspace_file",
            "locator": workspace.join("design.json"), "summary": "MCP procedure artifact",
            "idempotency_key": "mcp-procedure-evidence"
        }),
    )
    .await?;
    assert_eq!(evidence["ok"], true);
    let step_record = call_json(
        &client,
        "aporic_workflow_step_record",
        json!({
            "task_id": task_id, "step_id": "problem_and_outcome",
            "disposition": "completed",
            "evidence_ids": [evidence["result"]["evidence"]["evidence_id"]],
            "idempotency_key": "mcp-procedure-step"
        }),
    )
    .await?;
    assert_eq!(step_record["ok"], true);
    let rejected = call_json(
        &client,
        "aporic_workflow_advance",
        json!({
            "task_id": task_id, "expected_stage": "intake",
            "idempotency_key": "mcp-workflow-early"
        }),
    )
    .await?;
    assert_eq!(rejected["ok"], false);
    let decision = call_json(
        &client,
        "aporic_delegation_assess",
        json!({
            "task_id": task_id,
            "parallel_paths": 2,
            "material_change": true,
            "worker": {"disposition": "delegate", "reason": "Independent implementation path"},
            "reviewer": {"disposition": "delegate", "reason": "Material code change"},
            "idempotency_key": "mcp-delegation-assess"
        }),
    )
    .await?;
    assert_eq!(decision["ok"], true);
    let decision_id = decision["result"]["decision"]["decision_id"]
        .as_str()
        .unwrap();
    let reported = call_json(
        &client,
        "aporic_delegation_report",
        json!({
            "task_id": task_id,
            "decision_id": decision_id,
            "dimension": "worker",
            "host_agent_id": "host-agent-1",
            "model": "gpt-6-luna",
            "reasoning_effort": "low",
            "outcome": "started",
            "result_summary": "Host reported agent start",
            "idempotency_key": "mcp-delegation-report"
        }),
    )
    .await?;
    assert_eq!(reported["ok"], true);
    let applied = call_json(
        &client,
        "aporic_task_memory_apply",
        json!({
            "task_id": task_id,
            "memory_id": memory_id,
            "criterion": "The observation is reviewed",
            "intended_action": "Review the observation before completing the task",
            "idempotency_key": "mcp-memory-apply"
        }),
    )
    .await?;
    assert_eq!(applied["ok"], true);
    let packet = call_json(
        &client,
        "aporic_task_work_packet",
        json!({
            "workspace": workspace,
            "task_id": task_id,
            "route": {
                "work_kind": "implementation",
                "complexity": "bounded",
                "consequence": "low",
                "ambiguity_high": false,
                "independent_review": true
            }
        }),
    )
    .await?;
    assert_eq!(packet["result"]["task"]["task_id"], task_id);
    let brief = call_json(
        &client,
        "aporic_task_brief",
        json!({
            "workspace": workspace,
            "task_id": task_id,
            "max_context_bytes": 2048,
            "idempotency_key": "mcp-task-brief"
        }),
    )
    .await?;
    assert_eq!(brief["ok"], true);
    assert_eq!(brief["result"]["receipt"]["template_version"], 1);
    assert_eq!(brief["result"]["advisory"], true);
    assert_eq!(packet["result"]["memory_uses"][0]["memory_id"], memory_id);
    assert_eq!(
        packet["result"]["delegation"]["decisions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        packet["result"]["delegation"]["reports"][0]["host_agent_id"],
        "host-agent-1"
    );
    assert_eq!(packet["result"]["executable"], false);
    let uses = call_json(
        &client,
        "aporic_task_memory_list",
        json!({"workspace": workspace, "task_id": task_id}),
    )
    .await?;
    assert_eq!(uses["result"][0]["memory_id"], memory_id);
    let delegation = call_json(
        &client,
        "aporic_delegation_status",
        json!({"workspace": workspace, "task_id": task_id}),
    )
    .await?;
    assert_eq!(delegation["result"]["advisory"], true);
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
    let compact = call_json(
        &restarted,
        "aporic_recall",
        json!({ "workspace": workspace, "limit": 10, "compact": true }),
    )
    .await?;
    assert_eq!(compact["ok"], true);
    assert_eq!(
        compact["result"]["selected_items"],
        recalled["result"]["selected_items"]
    );
    assert_eq!(compact["result"]["budget"], recalled["result"]["budget"]);
    assert_eq!(compact["result"]["recent_records"], json!([]));
    assert!(serde_json::to_vec(&compact)?.len() < serde_json::to_vec(&recalled)?.len());
    restarted.cancel().await?;
    Ok(())
}

async fn start_server(
    database: &std::path::Path,
) -> Result<rmcp::service::RunningService<rmcp::RoleClient, ()>, Box<dyn Error>> {
    start_server_with_profile(database, false).await
}

async fn start_server_with_profile(
    database: &std::path::Path,
    full: bool,
) -> Result<rmcp::service::RunningService<rmcp::RoleClient, ()>, Box<dyn Error>> {
    let transport = TokioChildProcess::new(
        tokio::process::Command::new(env!("CARGO_BIN_EXE_aporic")).configure(|command| {
            command.args(["mcp", "serve", "--stdio"]);
            if full {
                command.args(["--profile", "full"]);
            }
            command.env("APORIC_DATABASE", database);
        }),
    )?;
    Ok(().serve(transport).await?)
}

#[tokio::test]
async fn full_mcp_profile_is_explicit_and_reveals_optional_diagnostics()
-> Result<(), Box<dyn Error>> {
    let area = tempfile::tempdir()?;
    let database = area.path().join("aporic.sqlite3");
    let client = start_server_with_profile(&database, true).await?;
    let names = client
        .list_all_tools()
        .await?
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect::<Vec<_>>();
    assert_eq!(names.len(), 65, "full MCP surface changed");
    assert!(names.iter().any(|name| name == "aporic_trace_list"));
    assert!(
        names
            .iter()
            .any(|name| name == "aporic_accountability_list")
    );
    assert!(names.iter().any(|name| name == "aporic_open"));
    assert!(
        !names
            .iter()
            .any(|name| name == "aporic_prompt_trial_record")
    );
    assert!(
        !names
            .iter()
            .any(|name| name == "aporic_deliberation_create")
    );
    client.cancel().await?;
    Ok(())
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
