use std::{error::Error, io::Read};

use aporic::{AporicMcp, Hub, default_database_path};
use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command] if command == "doctor" => doctor(),
        [command, flag, workspace] if command == "export" && flag == "--workspace" => {
            export(workspace)
        }
        [trace, export_command, flag, workspace]
            if trace == "trace" && export_command == "export" && flag == "--workspace" =>
        {
            export_runtime_trace(workspace)
        }
        [git, inspect, flag, workspace]
            if git == "git" && inspect == "inspect" && flag == "--workspace" =>
        {
            inspect_git(workspace)
        }
        [command, flag, spec_id] if command == "verify" && flag == "--spec" => {
            verify(spec_id).await
        }
        [eval, simulate, flag, actor]
            if eval == "eval" && simulate == "simulate" && flag == "--actor" =>
        {
            simulate_eval(actor)
        }
        [eval, context] if eval == "eval" && context == "context" => simulate_context_eval(),
        [eval, memory] if eval == "eval" && memory == "memory" => simulate_memory_eval(),
        [eval, runtime] if eval == "eval" && runtime == "runtime" => simulate_runtime_eval(),
        [eval, git] if eval == "eval" && git == "git" => simulate_git_eval(),
        [eval, tokens] if eval == "eval" && tokens == "tokens" => simulate_token_eval(),
        [tokens, report, flag, workspace]
            if tokens == "tokens" && report == "report" && flag == "--workspace" =>
        {
            token_report(workspace)
        }
        [executions, reconcile, flag, seconds]
            if executions == "executions"
                && reconcile == "reconcile"
                && flag == "--stale-after" =>
        {
            reconcile_executions(seconds)
        }
        [hook, codex] if hook == "hook" && codex == "codex" => codex_hook(),
        [mcp, serve, transport] if mcp == "mcp" && serve == "serve" && transport == "--stdio" => {
            serve_stdio().await
        }
        _ => {
            eprintln!(
                "usage: aporic doctor | aporic export --workspace PATH | aporic trace export --workspace PATH | aporic git inspect --workspace PATH | aporic tokens report --workspace PATH | aporic verify --spec SPEC_ID | aporic eval simulate --actor calibrated|overclaiming|contrarian | aporic eval context | aporic eval memory | aporic eval runtime | aporic eval git | aporic eval tokens | aporic executions reconcile --stale-after SECONDS | aporic hook codex | aporic mcp serve --stdio"
            );
            std::process::exit(2);
        }
    }
}

fn simulate_context_eval() -> Result<(), Box<dyn Error>> {
    println!(
        "{}",
        serde_json::to_string_pretty(&aporic::eval::simulate_context_selection())?
    );
    Ok(())
}

fn simulate_memory_eval() -> Result<(), Box<dyn Error>> {
    println!(
        "{}",
        serde_json::to_string_pretty(&aporic::eval::simulate_memory_lifecycle())?
    );
    Ok(())
}

fn simulate_runtime_eval() -> Result<(), Box<dyn Error>> {
    println!(
        "{}",
        serde_json::to_string_pretty(&aporic::eval::simulate_runtime_trace())?
    );
    Ok(())
}

fn simulate_git_eval() -> Result<(), Box<dyn Error>> {
    println!(
        "{}",
        serde_json::to_string_pretty(&aporic::eval::simulate_git_governance())?
    );
    Ok(())
}

fn simulate_token_eval() -> Result<(), Box<dyn Error>> {
    println!(
        "{}",
        serde_json::to_string_pretty(&aporic::eval::simulate_token_efficiency())?
    );
    Ok(())
}

fn token_report(workspace: &str) -> Result<(), Box<dyn Error>> {
    let database = default_database_path().map_err(std::io::Error::other)?;
    let hub = Hub::open(database)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&hub.token_efficiency_report(
            &aporic::domain::TokenEfficiencyReportRequest {
                workspace: workspace.to_owned(),
            },
        )?)?
    );
    Ok(())
}

fn codex_hook() -> Result<(), Box<dyn Error>> {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        println!("{{}}");
        return Ok(());
    }
    let response = default_database_path()
        .map_err(std::io::Error::other)
        .and_then(|path| Hub::open(path).map_err(std::io::Error::other))
        .map_or_else(
            |_| serde_json::json!({}),
            |hub| aporic::hook::handle_codex_hook(&hub, &input),
        );
    println!("{}", serde_json::to_string(&response)?);
    Ok(())
}

fn simulate_eval(actor: &str) -> Result<(), Box<dyn Error>> {
    let report = aporic::eval::simulate(actor)
        .ok_or_else(|| format!("unknown deterministic actor: {actor}"))?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

async fn verify(spec_id: &str) -> Result<(), Box<dyn Error>> {
    let database = default_database_path().map_err(std::io::Error::other)?;
    let hub = Hub::open(database)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&hub.verify(spec_id).await?)?
    );
    Ok(())
}

fn reconcile_executions(seconds: &str) -> Result<(), Box<dyn Error>> {
    let stale_after_seconds = seconds.parse::<u64>()?;
    let database = default_database_path().map_err(std::io::Error::other)?;
    let hub = Hub::open(database)?;
    let interrupted = hub.reconcile_executions(stale_after_seconds)?;
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "ok": true,
            "interrupted_execution_count": interrupted
        }))?
    );
    Ok(())
}

fn doctor() -> Result<(), Box<dyn Error>> {
    let database = default_database_path().map_err(std::io::Error::other)?;
    let hub = Hub::open(&database)?;
    let stats = hub.stats()?;
    let execution_replay = hub.audit_execution_replay()?;
    let memory_projection = hub.audit_memory_projection()?;
    let runtime_projection = hub.audit_runtime_projection()?;
    let git_snapshots = hub.audit_git_snapshots()?;
    let token_usage = hub.audit_token_usage()?;
    let replay_ok = execution_replay.mismatches.is_empty()
        && memory_projection.consistent
        && runtime_projection.consistent
        && git_snapshots.consistent
        && token_usage.consistent;
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "ok": replay_ok,
            "kernel_sha256": aporic::kernel::digest(),
            "database": hub.database_path(),
            "stats": stats,
            "execution_replay": execution_replay,
            "memory_projection": memory_projection,
            "runtime_projection": runtime_projection
            ,"git_snapshots": git_snapshots,
            "token_usage": token_usage
        }))?
    );
    Ok(())
}

fn inspect_git(workspace: &str) -> Result<(), Box<dyn Error>> {
    let database = default_database_path().map_err(std::io::Error::other)?;
    let hub = Hub::open(database)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&hub.observe_git(&aporic::domain::GitObserveRequest {
            workspace: workspace.to_owned(),
            base_ref: None,
        })?)?
    );
    Ok(())
}

fn export(workspace: &str) -> Result<(), Box<dyn Error>> {
    let database = default_database_path().map_err(std::io::Error::other)?;
    let hub = Hub::open(database)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&hub.export_project(workspace)?)?
    );
    Ok(())
}

fn export_runtime_trace(workspace: &str) -> Result<(), Box<dyn Error>> {
    let database = default_database_path().map_err(std::io::Error::other)?;
    let hub = Hub::open(database)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&hub.export_runtime_otel(
            &aporic::domain::RuntimeWorkspaceRequest {
                workspace: workspace.to_owned(),
            },
        )?)?
    );
    Ok(())
}

async fn serve_stdio() -> Result<(), Box<dyn Error>> {
    let database = default_database_path().map_err(std::io::Error::other)?;
    let hub = Hub::open(database)?;
    let service = AporicMcp::new(hub).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
