use std::{error::Error, fs, path::Path};

use aporic::{AporicMcp, Hub, default_database_path};
use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [
            delivery,
            validate,
            workspace_flag,
            workspace,
            manifest_flag,
            manifest,
        ] if delivery == "delivery"
            && validate == "validate"
            && workspace_flag == "--workspace"
            && manifest_flag == "--manifest" =>
        {
            let hub = Hub::open(default_database_path()?)?;
            let report = aporic::delivery_receipts::validate(
                &hub,
                &aporic::delivery::DeliveryValidateRequest {
                    workspace: workspace.clone(),
                    manifest: manifest.clone(),
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            if report.valid {
                Ok(())
            } else {
                Err("delivery has unresolved evidence or coverage gaps".into())
            }
        }
        [
            delivery,
            impact,
            workspace_flag,
            workspace,
            previous_flag,
            previous,
            current_flag,
            current,
        ] if delivery == "delivery"
            && impact == "impact"
            && workspace_flag == "--workspace"
            && previous_flag == "--previous"
            && current_flag == "--current" =>
        {
            let hub = Hub::open(default_database_path()?)?;
            let report = aporic::delivery_receipts::impact(
                &hub,
                &aporic::delivery::DeliveryImpactRequest {
                    workspace: workspace.clone(),
                    previous: previous.clone(),
                    current: current.clone(),
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
        [
            delivery,
            brief,
            workspace_flag,
            workspace,
            manifest_flag,
            manifest,
            focus_flag,
            focus,
        ] if delivery == "delivery"
            && brief == "brief"
            && workspace_flag == "--workspace"
            && manifest_flag == "--manifest"
            && focus_flag == "--focus" =>
        {
            let report = aporic::delivery::brief(&aporic::delivery::DeliveryBriefRequest {
                workspace: workspace.clone(),
                manifest: manifest.clone(),
                focus_ids: vec![focus.clone()],
                max_nodes: 32,
                max_bytes: 8192,
            })?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
        [
            delivery,
            verify,
            workspace_flag,
            workspace,
            manifest_flag,
            manifest,
            check_flag,
            check_id,
            spec_flag,
            spec_id,
        ] if delivery == "delivery"
            && verify == "verify"
            && workspace_flag == "--workspace"
            && manifest_flag == "--manifest"
            && check_flag == "--check"
            && spec_flag == "--spec" =>
        {
            let hub = Hub::open(default_database_path()?)?;
            let result = aporic::delivery_receipts::verify(
                &hub,
                &aporic::delivery_receipts::DeliveryVerifyRequest {
                    workspace: workspace.clone(),
                    manifest: manifest.clone(),
                    check_id: check_id.clone(),
                    spec_id: spec_id.clone(),
                },
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            if result.bound {
                Ok(())
            } else {
                Err("delivery execution did not establish a current binding".into())
            }
        }
        [command] if command == "doctor" => doctor(),
        [command] if command == "identity" => identity(),
        [release, validate, checkout_flag, checkout]
            if release == "release" && validate == "validate" && checkout_flag == "--checkout" =>
        {
            validate_release(checkout)
        }
        [command, flag, workspace] if command == "export" && flag == "--workspace" => {
            export(workspace)
        }
        [
            design,
            validate,
            workspace_flag,
            workspace,
            manifest_flag,
            manifest,
        ] if design == "design"
            && validate == "validate"
            && workspace_flag == "--workspace"
            && manifest_flag == "--manifest" =>
        {
            let report = aporic::design::validate(&aporic::design::DesignValidateRequest {
                workspace: workspace.clone(),
                manifest: manifest.clone(),
            })?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
        [backup_command, flag, target] if backup_command == "backup" && flag == "--to" => {
            backup(target)
        }
        [restore_command, dry_run, source]
            if restore_command == "restore" && dry_run == "--dry-run" =>
        {
            validate_backup(source)
        }
        [restore_command, from_flag, source, to_flag, destination]
            if restore_command == "restore" && from_flag == "--from" && to_flag == "--to" =>
        {
            restore_backup(source, destination)
        }
        [backup_command, prune, dir_flag, directory, keep_flag, keep]
            if backup_command == "backup"
                && prune == "prune"
                && dir_flag == "--dir"
                && keep_flag == "--keep" =>
        {
            prune_backups(directory, keep)
        }
        [
            research,
            sync,
            workspace_flag,
            workspace,
            source_flag,
            source,
            query_flag,
            query,
        ] if research == "research"
            && sync == "sync"
            && workspace_flag == "--workspace"
            && source_flag == "--source"
            && query_flag == "--query" =>
        {
            sync_research(workspace, source, query)
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
            serve_stdio(aporic::mcp::McpProfile::Core).await
        }
        [mcp, serve, transport, profile_flag, profile]
            if mcp == "mcp"
                && serve == "serve"
                && transport == "--stdio"
                && profile_flag == "--profile"
                && profile == "full" =>
        {
            serve_stdio(aporic::mcp::McpProfile::Full).await
        }
        _ => {
            eprintln!(
                "usage: aporic delivery validate --workspace PATH --manifest REL | aporic delivery brief --workspace PATH --manifest REL --focus ID | aporic delivery impact --workspace PATH --previous REL --current REL | aporic delivery verify --workspace PATH --manifest REL --check ID --spec SPEC_ID | aporic doctor | aporic identity | aporic release validate --checkout PATH | aporic design validate --workspace PATH --manifest RELATIVE_PATH | aporic backup --to PATH | aporic backup prune --dir DIR --keep COUNT | aporic restore --dry-run PATH | aporic restore --from BACKUP --to DATABASE | aporic research sync --workspace PATH --source github|stackoverflow --query TEXT | aporic export --workspace PATH | aporic trace export --workspace PATH | aporic git inspect --workspace PATH | aporic tokens report --workspace PATH | aporic verify --spec SPEC_ID | aporic eval simulate --actor calibrated|overclaiming|contrarian | aporic eval context | aporic eval memory | aporic eval runtime | aporic eval git | aporic eval tokens | aporic executions reconcile --stale-after SECONDS | aporic hook codex | aporic mcp serve --stdio [--profile full]"
            );
            std::process::exit(2);
        }
    }
}

fn backup(target: &str) -> Result<(), Box<dyn Error>> {
    let database = default_database_path().map_err(std::io::Error::other)?;
    let hub = Hub::open(database)?;
    let schema_version = hub.stats()?.schema_version;
    hub.backup_to(Path::new(target))?;
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "ok": true,
            "backup": fs::canonicalize(target)?,
            "schema_version": schema_version,
        }))?
    );
    Ok(())
}

fn validate_backup(source: &str) -> Result<(), Box<dyn Error>> {
    let schema_version = Hub::validate_backup(Path::new(source))?;
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "ok": true,
            "backup": fs::canonicalize(source)?,
            "schema_version": schema_version,
            "restored": false,
        }))?
    );
    Ok(())
}

fn restore_backup(source: &str, destination: &str) -> Result<(), Box<dyn Error>> {
    let outcome = aporic::recovery::restore_to(Path::new(source), Path::new(destination))?;
    println!("{}", serde_json::to_string(&outcome)?);
    Ok(())
}

fn prune_backups(directory: &str, keep: &str) -> Result<(), Box<dyn Error>> {
    let keep = keep.parse::<usize>()?;
    let outcome = aporic::recovery::prune_backups(Path::new(directory), keep)?;
    println!("{}", serde_json::to_string(&outcome)?);
    Ok(())
}

fn sync_research(workspace: &str, source: &str, query: &str) -> Result<(), Box<dyn Error>> {
    let database = default_database_path().map_err(std::io::Error::other)?;
    let hub = Hub::open(database)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&hub.research_sync(workspace, source, query)?)?
    );
    Ok(())
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
    let Some(input) = aporic::read_hook_input(&mut std::io::stdin()) else {
        println!("{{}}");
        return Ok(());
    };
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
    let core_events = hub.audit_core_events()?;
    let runtime_projection = hub.audit_runtime_projection()?;
    let git_snapshots = hub.audit_git_snapshots()?;
    let token_usage = hub.audit_token_usage()?;
    let research = hub.audit_research()?;
    let task_research = hub.audit_task_research()?;
    let accountability = hub.audit_accountability()?;
    let core_ok = execution_replay.mismatches.is_empty()
        && memory_projection.consistent
        && core_events.covered_consistent;
    let extensions_ok =
        runtime_projection.consistent && git_snapshots.consistent && token_usage.consistent;
    let extensions_ok = extensions_ok
        && research.consistent
        && task_research.consistent
        && accountability.consistent;
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "ok": core_ok && extensions_ok,
            "core_ok": core_ok,
            "extensions_ok": extensions_ok,
            "overall_ok": core_ok && extensions_ok,
            "kernel_sha256": aporic::kernel::digest(),
            "runtime": aporic::runtime::current(),
            "database": hub.database_path(),
            "stats": stats,
            "execution_replay": execution_replay,
            "memory_projection": memory_projection,
            "core_events": core_events,
            "runtime_projection": runtime_projection,
            "git_snapshots": git_snapshots,
            "token_usage": token_usage,
            "research": research,
            "task_research": task_research,
            "accountability": accountability,
            "sandbox": aporic::sandbox_backend_status()
        }))?
    );
    Ok(())
}

fn identity() -> Result<(), Box<dyn Error>> {
    println!(
        "{}",
        serde_json::to_string_pretty(&aporic::runtime::current())?
    );
    Ok(())
}

fn validate_release(checkout: &str) -> Result<(), Box<dyn Error>> {
    let report = aporic::runtime::validate_release(checkout)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if !report.source_matches
        || !report.package_version_matches
        || report.git_head_matches == Some(false)
    {
        std::process::exit(1);
    }
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

async fn serve_stdio(profile: aporic::mcp::McpProfile) -> Result<(), Box<dyn Error>> {
    let database = default_database_path().map_err(std::io::Error::other)?;
    let hub = Hub::open(database)?;
    let server = match profile {
        aporic::mcp::McpProfile::Core => AporicMcp::core(hub),
        aporic::mcp::McpProfile::Full => AporicMcp::full(hub),
    };
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
