use std::error::Error;

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
        [command, flag, spec_id] if command == "verify" && flag == "--spec" => {
            verify(spec_id).await
        }
        [eval, simulate, flag, actor]
            if eval == "eval" && simulate == "simulate" && flag == "--actor" =>
        {
            simulate_eval(actor)
        }
        [executions, reconcile, flag, seconds]
            if executions == "executions"
                && reconcile == "reconcile"
                && flag == "--stale-after" =>
        {
            reconcile_executions(seconds)
        }
        [mcp, serve, transport] if mcp == "mcp" && serve == "serve" && transport == "--stdio" => {
            serve_stdio().await
        }
        _ => {
            eprintln!(
                "usage: aporic doctor | aporic export --workspace PATH | aporic verify --spec SPEC_ID | aporic eval simulate --actor calibrated|overclaiming|contrarian | aporic executions reconcile --stale-after SECONDS | aporic mcp serve --stdio"
            );
            std::process::exit(2);
        }
    }
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
    let replay_ok = execution_replay.mismatches.is_empty();
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "ok": replay_ok,
            "kernel_sha256": aporic::kernel::digest(),
            "database": hub.database_path(),
            "stats": stats,
            "execution_replay": execution_replay
        }))?
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

async fn serve_stdio() -> Result<(), Box<dyn Error>> {
    let database = default_database_path().map_err(std::io::Error::other)?;
    let hub = Hub::open(database)?;
    let service = AporicMcp::new(hub).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
