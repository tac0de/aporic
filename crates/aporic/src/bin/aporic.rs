use std::{error::Error, fs, path::Path};

use aporic::domain::{
    CloseDisposition, CloseRequest, GovernmentBootstrapRequest, GovernmentWorkspaceRequest,
    OpenRequest,
};
use aporic::{AporicMcp, Hub, default_database_path};
use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command] if command == "doctor" => doctor(),
        [government, bootstrap, flag, workspace]
            if government == "government" && bootstrap == "bootstrap" && flag == "--workspace" =>
        {
            bootstrap_government(workspace)
        }
        [government, roster, flag, workspace]
            if government == "government" && roster == "roster" && flag == "--workspace" =>
        {
            show_government_roster(workspace)
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
        [security, import, flag, request]
            if security == "security" && import == "import-codex" && flag == "--request" =>
        {
            import_codex_security(request)
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
        [eval, deliberation] if eval == "eval" && deliberation == "deliberation" => {
            simulate_deliberation_eval()
        }
        [eval, capabilities] if eval == "eval" && capabilities == "capabilities" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&aporic::eval::simulate_capability_fabric())?
            );
            Ok(())
        }
        [eval, experiments] if eval == "eval" && experiments == "experiments" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&aporic::eval::simulate_experiment_portfolio())?
            );
            Ok(())
        }
        [eval, security_import] if eval == "eval" && security_import == "security-import" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&aporic::eval::simulate_security_import())?
            );
            Ok(())
        }
        [tokens, report, flag, workspace]
            if tokens == "tokens" && report == "report" && flag == "--workspace" =>
        {
            token_report(workspace)
        }
        [
            deliberation,
            show,
            workspace_flag,
            workspace,
            id_flag,
            deliberation_id,
        ] if deliberation == "deliberation"
            && show == "show"
            && workspace_flag == "--workspace"
            && id_flag == "--id" =>
        {
            show_deliberation(workspace, deliberation_id)
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
                "usage: aporic doctor | aporic government bootstrap --workspace PATH | aporic government roster --workspace PATH | aporic design validate --workspace PATH --manifest RELATIVE_PATH | aporic backup --to PATH | aporic backup prune --dir DIR --keep COUNT | aporic restore --dry-run PATH | aporic restore --from BACKUP --to DATABASE | aporic security import-codex --request REQUEST.json | aporic research sync --workspace PATH --source github|stackoverflow --query TEXT | aporic export --workspace PATH | aporic trace export --workspace PATH | aporic git inspect --workspace PATH | aporic tokens report --workspace PATH | aporic deliberation show --workspace PATH --id ID | aporic verify --spec SPEC_ID | aporic eval simulate --actor calibrated|overclaiming|contrarian | aporic eval context | aporic eval memory | aporic eval runtime | aporic eval git | aporic eval tokens | aporic eval deliberation | aporic eval capabilities | aporic eval experiments | aporic eval security-import | aporic executions reconcile --stale-after SECONDS | aporic hook codex | aporic mcp serve --stdio"
            );
            std::process::exit(2);
        }
    }
}

fn bootstrap_government(workspace: &str) -> Result<(), Box<dyn Error>> {
    let database = default_database_path().map_err(std::io::Error::other)?;
    let hub = Hub::open(database)?;
    let session = hub.open_session(&OpenRequest {
        workspace: workspace.to_owned(),
        objective: "Initialize the named advisory government roster".to_owned(),
        idempotency_key: format!("government-cli-open:{}", uuid::Uuid::now_v7()),
    })?;
    let result = hub.bootstrap_government(&GovernmentBootstrapRequest {
        session_id: session.session_id.clone(),
        idempotency_key: "initial-cabinet-v2".to_owned(),
    });
    let disposition = if result.is_ok() {
        CloseDisposition::Completed
    } else {
        CloseDisposition::Handoff
    };
    hub.close_session(&CloseRequest {
        session_id: session.session_id,
        disposition,
        summary: if result.is_ok() {
            "The initial named advisory government roster was initialized or already present."
                .to_owned()
        } else {
            "Government roster initialization did not complete.".to_owned()
        },
        next_action: if result.is_ok() {
            None
        } else {
            Some(
                "Inspect the reported bootstrap error and existing office appointments.".to_owned(),
            )
        },
        idempotency_key: format!("government-cli-close:{}", uuid::Uuid::now_v7()),
    })?;
    let outcome = result?;
    println!("{}", serde_json::to_string_pretty(&outcome)?);
    Ok(())
}

fn show_government_roster(workspace: &str) -> Result<(), Box<dyn Error>> {
    let database = default_database_path().map_err(std::io::Error::other)?;
    let hub = Hub::open(database)?;
    let roster = hub.government_roster(&GovernmentWorkspaceRequest {
        workspace: workspace.to_owned(),
        limit: Some(200),
    })?;
    println!("{}", serde_json::to_string_pretty(&roster)?);
    Ok(())
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

fn import_codex_security(request_path: &str) -> Result<(), Box<dyn Error>> {
    let bytes = fs::read(request_path)?;
    if bytes.len() > 65_536 {
        return Err("security import request exceeds 65536 bytes".into());
    }
    let request: aporic::domain::SecurityImportRequest = serde_json::from_slice(&bytes)?;
    let database = default_database_path().map_err(std::io::Error::other)?;
    let hub = Hub::open(database)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&hub.import_codex_security(&request)?)?
    );
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

fn simulate_deliberation_eval() -> Result<(), Box<dyn Error>> {
    println!(
        "{}",
        serde_json::to_string_pretty(&aporic::eval::simulate_deliberation())?
    );
    Ok(())
}

fn show_deliberation(workspace: &str, deliberation_id: &str) -> Result<(), Box<dyn Error>> {
    let database = default_database_path().map_err(std::io::Error::other)?;
    let hub = Hub::open(database)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&hub.get_deliberation(
            &aporic::domain::DeliberationGetRequest {
                workspace: workspace.to_owned(),
                deliberation_id: deliberation_id.to_owned(),
                node_after_sequence: None,
                edge_after_sequence: None,
                decision_after_sequence: None,
                limit: None,
            },
        )?)?
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
    let runtime_projection = hub.audit_runtime_projection()?;
    let git_snapshots = hub.audit_git_snapshots()?;
    let token_usage = hub.audit_token_usage()?;
    let deliberations = hub.audit_deliberations()?;
    let secure_capabilities = hub.audit_secure_capabilities()?;
    let role_appointments = hub.audit_role_appointments()?;
    let government = hub.audit_government()?;
    let orchestration = hub.audit_orchestration()?;
    let research = hub.audit_research()?;
    let accountability = hub.audit_accountability()?;
    let replay_ok = execution_replay.mismatches.is_empty()
        && memory_projection.consistent
        && runtime_projection.consistent
        && git_snapshots.consistent
        && token_usage.consistent
        && deliberations.consistent
        && secure_capabilities.consistent
        && role_appointments.consistent
        && government.consistent
        && orchestration.consistent;
    let replay_ok = replay_ok && research.consistent && accountability.consistent;
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "ok": replay_ok,
            "kernel_sha256": aporic::kernel::digest(),
            "database": hub.database_path(),
            "stats": stats,
            "execution_replay": execution_replay,
            "memory_projection": memory_projection,
            "runtime_projection": runtime_projection,
            "git_snapshots": git_snapshots,
            "token_usage": token_usage,
            "deliberations": deliberations,
            "secure_capabilities": secure_capabilities,
            "role_appointments": role_appointments,
            "government": government,
            "orchestration": orchestration,
            "research": research,
            "accountability": accountability,
            "sandbox": aporic::sandbox_backend_status()
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
