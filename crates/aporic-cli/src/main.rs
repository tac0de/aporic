use aporic_boundary::BoundaryPolicy;
use aporic_handoff::{CommitStatus as HandoffCommitStatus, HandoffCapsule, HandoffRef};
use aporic_host::{
    ActionIdentity, CloseDayRequest, ConnectRequest, EffectRequest, HostRuntime, OpenDayRequest,
    ProjectHandoffRequest, ReserveRequest, connect,
};
use aporic_kernel::{CommitStatus as KernelCommitStatus, EffectOutcome};
use aporic_model_control::{
    ApplicationPlan, AuditEvent, HostCapability, LaunchOutcome, ModelControlPolicy, append_audit,
    canonicalize_policy, initialize_audit, load_audit, plan as plan_model_control,
};
use aporic_projects::{BindRequest, bind_project};
use aporic_roles::{HostPolicy, load_role};
use aporic_routing::RoutingSignals;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const CONNECTION_SCHEMA_VERSION: u32 = 1;
const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;
static NEXT_LAUNCH_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BindInput {
    schema_version: u32,
    project_id: String,
    workspace: PathBuf,
    remote_name: String,
    role_directory: PathBuf,
    host_policy: HostPolicy,
    boundary_policy: BoundaryPolicy,
    model_control: ModelControlPolicy,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectionDocument {
    schema_version: u32,
    registry: PathBuf,
    project_id: String,
    role_directory: PathBuf,
    kernel_store: PathBuf,
    handoff_store: PathBuf,
    model_control_store: PathBuf,
    host_policy: HostPolicy,
    boundary_policy: BoundaryPolicy,
    model_control: ModelControlPolicy,
}

#[derive(Debug, Serialize)]
struct StatusDocument<'a> {
    project_id: &'a str,
    scope: &'a str,
    role_id: &'a str,
    profile_sha256: &'a str,
    kernel_revision: u64,
    handoff_revision: u64,
    model_control_records: usize,
    head: String,
    head_changed: bool,
    dirty: bool,
}

struct CommandOutput {
    command: &'static str,
    value: Value,
    rejected: bool,
    raw: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogRegisterInput {
    connection_file: PathBuf,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogEntry {
    schema_version: u32,
    connection_file: PathBuf,
    workspace: PathBuf,
}

#[derive(Debug, Deserialize)]
struct CodexHookInput {
    session_id: String,
    cwd: PathBuf,
    hook_event_name: String,
    model: Option<String>,
    source: Option<String>,
    tool_name: Option<String>,
    tool_use_id: Option<String>,
    tool_input: Option<Value>,
}

fn main() {
    let exit_code = match run() {
        Ok(output) => {
            let encoded = if output.raw {
                output.value
            } else {
                json!({
                    "ok": !output.rejected,
                    "command": output.command,
                    "result": output.value,
                })
            };
            println!(
                "{}",
                serde_json::to_string(&encoded).expect("JSON output is serializable")
            );
            if output.rejected { 2 } else { 0 }
        }
        Err(error) => {
            let encoded = json!({"ok": false, "error": error.to_string()});
            eprintln!(
                "{}",
                serde_json::to_string(&encoded).expect("JSON error is serializable")
            );
            1
        }
    };
    std::process::exit(exit_code);
}

fn run() -> Result<CommandOutput, Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let command = arguments
        .next()
        .and_then(|value| value.into_string().ok())
        .ok_or("usage: aporicctl <command> <absolute-connection-file>")?;
    let connection_path = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: aporicctl <command> <absolute-connection-file>")?;
    let trailing = arguments.collect::<Vec<_>>();
    if !connection_path.is_absolute() {
        return Err("connection file path must be absolute".into());
    }

    if command != "codex-launch" && !trailing.is_empty() {
        return Err("too many arguments".into());
    }

    match command.as_str() {
        "bind" => bind(&connection_path),
        "catalog-register" => catalog_register(&connection_path),
        "codex-hook" => codex_hook(&connection_path),
        "status" => with_runtime(&connection_path, |runtime, connection| {
            let observation = runtime.observe()?;
            let kernel = aporic_kernel::load(&connection.kernel_store)?;
            let handoff = aporic_handoff::load(&connection.handoff_store)?;
            let model_control = load_audit(&connection.model_control_store)?;
            let status = StatusDocument {
                project_id: runtime.binding().project_id(),
                scope: runtime.scope(),
                role_id: runtime.profile().role_id(),
                profile_sha256: runtime.profile().profile_sha256(),
                kernel_revision: kernel.state().revision,
                handoff_revision: handoff.state().revision,
                model_control_records: model_control.records().len(),
                head: observation.head,
                head_changed: observation.head_changed,
                dirty: observation.dirty,
            };
            success("status", serde_json::to_value(status)?)
        }),
        "day-open" => with_runtime(&connection_path, |runtime, _| {
            let outcome = runtime.continuity().open(read_stdin()?)?;
            operation(
                "day-open",
                serde_json::to_value(&outcome)?,
                outcome.status == HandoffCommitStatus::Rejected,
            )
        }),
        "handoff-project" => with_runtime(&connection_path, |runtime, _| {
            let outcome = runtime.continuity().project(read_stdin()?)?;
            let rejected = outcome.commit.status == HandoffCommitStatus::Rejected;
            operation("handoff-project", serde_json::to_value(outcome)?, rejected)
        }),
        "day-close" => with_runtime(&connection_path, |runtime, _| {
            let outcome = runtime.continuity().close(read_stdin()?)?;
            operation(
                "day-close",
                serde_json::to_value(&outcome)?,
                outcome.status == HandoffCommitStatus::Rejected,
            )
        }),
        "grant" => with_runtime(&connection_path, |runtime, _| {
            kernel_operation("grant", runtime.control().grant(read_stdin()?)?)
        }),
        "reserve" => with_runtime(&connection_path, |runtime, _| {
            kernel_operation("reserve", runtime.agent().reserve(read_stdin()?)?)
        }),
        "effect" => with_runtime(&connection_path, |runtime, _| {
            kernel_operation("effect", runtime.effects().record(read_stdin()?)?)
        }),
        "verify" => with_runtime(&connection_path, |runtime, _| {
            kernel_operation("verify", runtime.verifier().record(read_stdin()?)?)
        }),
        "abandon" => with_runtime(&connection_path, |runtime, _| {
            kernel_operation("abandon", runtime.effects().abandon(read_stdin()?)?)
        }),
        "route" => with_runtime(&connection_path, |runtime, _| {
            success(
                "route",
                serde_json::to_value(runtime.route(&read_stdin()?))?,
            )
        }),
        "model-plan" => with_runtime(&connection_path, |runtime, connection| {
            let decision = runtime.route(&read_stdin()?);
            let plan = plan_model_control(
                &connection.model_control,
                &decision,
                HostCapability::CodexCliLaunch,
            )?;
            success("model-plan", serde_json::to_value(plan)?)
        }),
        "codex-launch" => codex_launch(&connection_path, &trailing),
        _ => Err(format!("unknown command: {command}").into()),
    }
}

fn bind(path: &Path) -> Result<CommandOutput, Box<dyn std::error::Error>> {
    let input: BindInput = read_stdin()?;
    if input.schema_version != CONNECTION_SCHEMA_VERSION {
        return Err("unsupported bind schema version".into());
    }
    let parent = path
        .parent()
        .filter(|candidate| !candidate.as_os_str().is_empty())
        .ok_or("connection file must have a parent directory")?;
    let parent_metadata = std::fs::symlink_metadata(parent)?;
    if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
        return Err("connection parent must be a real directory".into());
    }
    let parent = std::fs::canonicalize(parent)?;
    let connection_path = parent.join(path.file_name().ok_or("connection filename is missing")?);

    let role_directory = std::fs::canonicalize(&input.role_directory)?;
    load_role(&role_directory, &input.host_policy)?;
    let registry = parent.join("projects");
    let kernel_store = parent.join("kernel.jsonl");
    let handoff_store = parent.join("handoff.jsonl");
    let model_control_store = parent.join("model-control.jsonl");
    if connection_path.try_exists()?
        || kernel_store.try_exists()?
        || handoff_store.try_exists()?
        || model_control_store.try_exists()?
    {
        return Err("connection or state store already exists".into());
    }

    let model_control = canonicalize_policy(input.model_control)?;

    let binding = bind_project(
        &registry,
        BindRequest {
            project_id: input.project_id.clone(),
            workspace: input.workspace,
            remote_name: input.remote_name,
        },
    )?;
    aporic_kernel::initialize(&kernel_store)?;
    aporic_handoff::initialize(&handoff_store)?;
    initialize_audit(&model_control_store)?;
    let connection = ConnectionDocument {
        schema_version: CONNECTION_SCHEMA_VERSION,
        registry,
        project_id: input.project_id,
        role_directory,
        kernel_store,
        handoff_store,
        model_control_store,
        host_policy: input.host_policy,
        boundary_policy: input.boundary_policy,
        model_control,
    };
    write_new_document(&connection_path, &connection)?;
    success(
        "bind",
        json!({
            "connection": connection_path,
            "project_id": binding.project_id(),
            "binding_sha256": binding.binding_sha256(),
        }),
    )
}

fn codex_launch(
    connection_path: &Path,
    trailing: &[std::ffi::OsString],
) -> Result<CommandOutput, Box<dyn std::error::Error>> {
    let (signals_path, codex_arguments) = trailing.split_first().ok_or(
        "usage: aporicctl codex-launch CONNECTION ABSOLUTE-SIGNALS-FILE [-- CODEX-ARGS...]",
    )?;
    let signals_path = PathBuf::from(signals_path);
    if !signals_path.is_absolute() {
        return Err("routing signals file path must be absolute".into());
    }
    let codex_arguments = codex_arguments
        .strip_prefix(&[std::ffi::OsString::from("--")])
        .unwrap_or(codex_arguments);
    reject_model_overrides(codex_arguments)?;

    let connection: ConnectionDocument = read_document(connection_path)?;
    let runtime = connect_runtime(&connection)?;
    let signals: RoutingSignals = read_document(&signals_path)?;
    let decision = runtime.route(&signals);
    let plan = plan_model_control(
        &connection.model_control,
        &decision,
        HostCapability::CodexCliLaunch,
    )?;
    let ApplicationPlan::CodexCliLaunch {
        executable,
        arguments,
    } = &plan.application
    else {
        return Err("model-control policy did not produce a Codex CLI launch plan".into());
    };

    let launch_id = next_launch_id()?;
    append_audit(
        &connection.model_control_store,
        AuditEvent::Planned {
            launch_id: launch_id.clone(),
            plan: plan.clone(),
        },
    )?;

    let status = Command::new(executable)
        .args(arguments)
        .arg("--cd")
        .arg(runtime.binding().workspace())
        .args(codex_arguments)
        .status();
    let outcome = match status {
        Ok(status) => match status.code() {
            Some(code) => LaunchOutcome::Exited { code },
            None => LaunchOutcome::Signaled,
        },
        Err(error) => LaunchOutcome::SpawnFailed {
            error: bounded_error(&error.to_string()),
        },
    };
    append_audit(
        &connection.model_control_store,
        AuditEvent::Finished {
            launch_id: launch_id.clone(),
            outcome: outcome.clone(),
        },
    )?;
    let rejected = !matches!(outcome, LaunchOutcome::Exited { code: 0 });
    operation(
        "codex-launch",
        json!({"launch_id": launch_id, "plan": plan, "outcome": outcome}),
        rejected,
    )
}

fn reject_model_overrides(
    arguments: &[std::ffi::OsString],
) -> Result<(), Box<dyn std::error::Error>> {
    for argument in arguments {
        let Some(argument) = argument.to_str() else {
            return Err("Codex arguments must be valid UTF-8".into());
        };
        if matches!(
            argument,
            "-m" | "--model" | "-p" | "--profile" | "-c" | "--config"
        ) || argument.starts_with("--model=")
            || argument.starts_with("--profile=")
            || argument.starts_with("--config=")
        {
            return Err(
                "Codex model, reasoning, config, and profile overrides are controlled by Aporic"
                    .into(),
            );
        }
    }
    Ok(())
}

fn next_launch_id() -> Result<String, Box<dyn std::error::Error>> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let sequence = NEXT_LAUNCH_ID.fetch_add(1, Ordering::Relaxed);
    Ok(format!("launch-{nanos}-{}-{sequence}", std::process::id()))
}

fn bounded_error(error: &str) -> String {
    error.chars().take(256).collect()
}

fn catalog_register(path: &Path) -> Result<CommandOutput, Box<dyn std::error::Error>> {
    let input: CatalogRegisterInput = read_stdin()?;
    let connection: ConnectionDocument = read_document(&input.connection_file)?;
    let runtime = connect_runtime(&connection)?;
    let parent = path.parent().ok_or("catalog path must have a parent")?;
    let parent = std::fs::canonicalize(parent)?;
    let catalog = parent.join(
        path.file_name()
            .ok_or("catalog directory name is missing")?,
    );
    if catalog.starts_with(runtime.binding().workspace()) {
        return Err("catalog must remain outside the governed workspace".into());
    }
    std::fs::create_dir_all(&catalog)?;
    let catalog = std::fs::canonicalize(catalog)?;
    let entry = CatalogEntry {
        schema_version: 1,
        connection_file: std::fs::canonicalize(input.connection_file)?,
        workspace: runtime.binding().workspace().to_path_buf(),
    };
    let entry_path = catalog.join(format!("{}.json", runtime.binding().binding_sha256()));
    if entry_path.exists() {
        let existing: CatalogEntry = read_document(&entry_path)?;
        if existing != entry {
            return Err("catalog entry conflicts with the existing project binding".into());
        }
    } else {
        write_new_document(&entry_path, &entry)?;
    }
    success(
        "catalog-register",
        json!({"catalog": catalog, "entry": entry_path}),
    )
}

fn codex_hook(catalog: &Path) -> Result<CommandOutput, Box<dyn std::error::Error>> {
    let input: CodexHookInput = read_stdin()?;
    let resolved = match resolve_catalog(catalog, &input.cwd) {
        Ok(value) => value,
        Err(error) if input.hook_event_name == "PreToolUse" => {
            return raw(pre_tool_denial(&format!("Aporic unavailable: {error}")));
        }
        Err(error) => return Err(error),
    };
    let Some((connection, runtime)) = resolved else {
        return raw(json!({}));
    };
    match input.hook_event_name.as_str() {
        "SessionStart" => hook_session_start(&connection, &runtime, &input),
        "PreToolUse" => hook_pre_tool(&connection, &runtime, &input),
        "PostToolUse" => hook_post_tool(&connection, &runtime, &input),
        "PreCompact" => {
            project_session_handoff(&connection, &runtime, &input.session_id)?;
            raw(json!({"continue": true}))
        }
        "SessionEnd" => {
            close_session_day(&connection, &runtime, &input.session_id)?;
            raw(json!({}))
        }
        _ => raw(json!({})),
    }
}

fn resolve_catalog(
    catalog: &Path,
    cwd: &Path,
) -> Result<Option<(ConnectionDocument, HostRuntime)>, Box<dyn std::error::Error>> {
    if !catalog.exists() {
        return Ok(None);
    }
    let cwd = std::fs::canonicalize(cwd)?;
    let mut selected: Option<(usize, ConnectionDocument, HostRuntime)> = None;
    for item in std::fs::read_dir(catalog)? {
        let path = item?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let entry: CatalogEntry = read_document(&path)?;
        if entry.schema_version != 1 {
            return Err("unsupported catalog entry schema version".into());
        }
        let connection: ConnectionDocument = read_document(&entry.connection_file)?;
        let runtime = connect_runtime(&connection)?;
        let workspace = runtime.binding().workspace();
        if cwd.starts_with(workspace) {
            let depth = workspace.components().count();
            if selected.as_ref().is_none_or(|current| depth > current.0) {
                selected = Some((depth, connection, runtime));
            }
        }
    }
    Ok(selected.map(|(_, connection, runtime)| (connection, runtime)))
}

fn hook_session_start(
    connection: &ConnectionDocument,
    runtime: &HostRuntime,
    input: &CodexHookInput,
) -> Result<CommandOutput, Box<dyn std::error::Error>> {
    let session_ref = session_ref(&input.session_id);
    let mut ledger = aporic_handoff::load(&connection.handoff_store)?;
    if !ledger
        .state()
        .days
        .values()
        .any(|day| day.day.session_ref == session_ref && day.day.scope == runtime.scope())
    {
        let predecessor = ledger
            .state()
            .days
            .values()
            .filter(|day| {
                day.day.scope == runtime.scope()
                    && day.day.lineage_ref == format!("project:{}", connection.project_id)
                    && day.closed
                    && day.successor_day_id.is_none()
            })
            .max_by_key(|day| day.day.day_id.clone())
            .and_then(|day| {
                day.latest_handoff.as_ref().map(|handoff| HandoffRef {
                    day_id: day.day.day_id.clone(),
                    handoff_sha256: handoff.handoff_sha256.clone(),
                })
            });
        let outcome = runtime.continuity().open(OpenDayRequest {
            event_id: format!("open:{}", digest(&input.session_id)),
            idempotency_key: format!("open:{}", digest(&input.session_id)),
            expected_revision: ledger.state().revision,
            day_id: format!("day:{}", digest(&input.session_id)),
            lineage_ref: format!("project:{}", connection.project_id),
            task_ref: session_ref.clone(),
            session_ref: session_ref.clone(),
            predecessor,
        })?;
        if outcome.status == HandoffCommitStatus::Rejected {
            return raw(json!({
                "continue": false,
                "stopReason": format!("Aporic day open rejected: {}", outcome.reason_code)
            }));
        }
        ledger = aporic_handoff::load(&connection.handoff_store)?;
    }
    let route = runtime.route(&RoutingSignals::default());
    let inherited = ledger
        .state()
        .days
        .values()
        .find(|day| day.day.session_ref == session_ref)
        .and_then(|day| day.day.predecessor.as_ref())
        .and_then(|predecessor| ledger.state().days.get(&predecessor.day_id))
        .and_then(|day| day.latest_handoff.as_ref())
        .map(|handoff| {
            format!(
                "Previous objective: {}\nNext action: {}",
                handoff.capsule.objective, handoff.capsule.next_action
            )
        })
        .unwrap_or_else(|| "No predecessor handoff.".into());
    let context = format!(
        "Aporic connection active. scope={} session_ref={} role={} routing_recommendation={:?} source={}. Hooks cannot change the active Codex model ({}). Exact grants remain required before governed tools. {}",
        runtime.scope(),
        session_ref,
        runtime.profile().role_id(),
        route.tier,
        input.source.as_deref().unwrap_or("unknown"),
        input.model.as_deref().unwrap_or("unknown"),
        inherited
    );
    raw(json!({
        "continue": true,
        "hookSpecificOutput": {
            "hookEventName": "SessionStart",
            "additionalContext": context
        }
    }))
}

fn hook_pre_tool(
    connection: &ConnectionDocument,
    runtime: &HostRuntime,
    input: &CodexHookInput,
) -> Result<CommandOutput, Box<dyn std::error::Error>> {
    let tool_name = input
        .tool_name
        .as_deref()
        .ok_or("PreToolUse tool_name is missing")?;
    let tool_use_id = input
        .tool_use_id
        .as_deref()
        .ok_or("PreToolUse tool_use_id is missing")?;
    let tool_input = input
        .tool_input
        .clone()
        .ok_or("PreToolUse tool_input is missing")?;
    let reservation_id = format!("codex:{}", digest(tool_use_id));
    let ledger = aporic_kernel::load(&connection.kernel_store)?;
    if ledger.state().reservations.contains_key(&reservation_id) {
        return raw(json!({}));
    }
    let session_ref = session_ref(&input.session_id);
    let grant = ledger.state().grants.values().find(|candidate| {
        candidate.reserved_by.is_none()
            && candidate.grant.principal == "codex"
            && candidate.grant.task_ref == session_ref
            && candidate.grant.session_ref == session_ref
            && candidate.grant.scope == runtime.scope()
            && candidate.grant.action == tool_name
            && candidate.grant.input == tool_input
    });
    let Some(grant) = grant else {
        return raw(pre_tool_denial(
            "Aporic requires an exact unconsumed grant for this tool input",
        ));
    };
    let outcome = runtime.agent().reserve(ReserveRequest {
        event_id: format!("reserve:{}", digest(tool_use_id)),
        idempotency_key: format!("reserve:{}", digest(tool_use_id)),
        expected_revision: ledger.state().revision,
        reservation_id,
        grant_id: grant.grant.grant_id.clone(),
        action: ActionIdentity {
            principal: "codex".into(),
            task_ref: session_ref.clone(),
            session_ref,
            action: tool_name.into(),
            input: tool_input,
        },
        routing_signals: RoutingSignals::default(),
    })?;
    if outcome.status == KernelCommitStatus::Rejected {
        return raw(pre_tool_denial(&format!(
            "Aporic reservation rejected: {}",
            outcome.reason_code
        )));
    }
    raw(json!({}))
}

fn hook_post_tool(
    connection: &ConnectionDocument,
    runtime: &HostRuntime,
    input: &CodexHookInput,
) -> Result<CommandOutput, Box<dyn std::error::Error>> {
    let tool_use_id = input
        .tool_use_id
        .as_deref()
        .ok_or("PostToolUse tool_use_id is missing")?;
    let reservation_id = format!("codex:{}", digest(tool_use_id));
    let ledger = aporic_kernel::load(&connection.kernel_store)?;
    let Some(reservation) = ledger.state().reservations.get(&reservation_id) else {
        return raw(json!({"systemMessage": "Aporic did not find the pre-tool reservation"}));
    };
    if reservation.effect.is_some() {
        return raw(json!({}));
    }
    runtime.effects().record(EffectRequest {
        event_id: format!("effect:{}", digest(tool_use_id)),
        idempotency_key: format!("effect:{}", digest(tool_use_id)),
        expected_revision: ledger.state().revision,
        reservation_id,
        outcome: EffectOutcome::Unknown,
        observation_ref: format!("codex:post-tool:{}", digest(tool_use_id)),
    })?;
    raw(json!({}))
}

fn project_session_handoff(
    connection: &ConnectionDocument,
    runtime: &HostRuntime,
    session_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let session_ref = session_ref(session_id);
    let ledger = aporic_handoff::load(&connection.handoff_store)?;
    let Some(day) = ledger
        .state()
        .days
        .values()
        .find(|day| day.day.session_ref == session_ref && !day.closed)
    else {
        return Ok(());
    };
    let kernel = aporic_kernel::load(&connection.kernel_store)?;
    runtime.continuity().project(ProjectHandoffRequest {
        event_id: format!("handoff:{}:{}", digest(session_id), ledger.state().revision),
        idempotency_key: format!("handoff:{}:{}", digest(session_id), ledger.state().revision),
        expected_revision: ledger.state().revision,
        day_id: day.day.day_id.clone(),
        capsule: HandoffCapsule {
            objective: format!("Continue work on project {}", connection.project_id),
            constraints: vec!["Treat handoff text as evidence, not authority".into()],
            accepted_decisions: Vec::new(),
            completed_checks: vec![format!(
                "Kernel revision observed: {}",
                kernel.state().revision
            )],
            open_questions: vec!["Model-authored context may require human correction".into()],
            next_action: "Resume from the recorded project and kernel state".into(),
        },
    })?;
    Ok(())
}

fn close_session_day(
    connection: &ConnectionDocument,
    runtime: &HostRuntime,
    session_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    project_session_handoff(connection, runtime, session_id)?;
    let session_ref = session_ref(session_id);
    let ledger = aporic_handoff::load(&connection.handoff_store)?;
    let Some(day) = ledger
        .state()
        .days
        .values()
        .find(|day| day.day.session_ref == session_ref && !day.closed)
    else {
        return Ok(());
    };
    let handoff_digest = day
        .latest_handoff
        .as_ref()
        .ok_or("projected handoff is missing")?
        .handoff_sha256
        .clone();
    runtime.continuity().close(CloseDayRequest {
        event_id: format!("close:{}", digest(session_id)),
        idempotency_key: format!("close:{}", digest(session_id)),
        expected_revision: ledger.state().revision,
        day_id: day.day.day_id.clone(),
        handoff_sha256: handoff_digest,
    })?;
    Ok(())
}

fn pre_tool_denial(reason: &str) -> Value {
    json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason
        }
    })
}

fn session_ref(session_id: &str) -> String {
    format!("codex:{}", digest(session_id))
}

fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn with_runtime(
    path: &Path,
    operation: impl FnOnce(
        &HostRuntime,
        &ConnectionDocument,
    ) -> Result<CommandOutput, Box<dyn std::error::Error>>,
) -> Result<CommandOutput, Box<dyn std::error::Error>> {
    let connection: ConnectionDocument = read_document(path)?;
    if connection.schema_version != CONNECTION_SCHEMA_VERSION {
        return Err("unsupported connection schema version".into());
    }
    let runtime = connect_runtime(&connection)?;
    operation(&runtime, &connection)
}

fn connect_runtime(
    connection: &ConnectionDocument,
) -> Result<HostRuntime, Box<dyn std::error::Error>> {
    if connection.schema_version != CONNECTION_SCHEMA_VERSION {
        return Err("unsupported connection schema version".into());
    }
    let runtime = connect(ConnectRequest {
        registry: connection.registry.clone(),
        project_id: connection.project_id.clone(),
        role_directory: connection.role_directory.clone(),
        kernel_store: connection.kernel_store.clone(),
        handoff_store: connection.handoff_store.clone(),
        host_policy: connection.host_policy.clone(),
        boundary_policy: connection.boundary_policy.clone(),
    })?;
    validate_model_control_connection(connection, &runtime)?;
    Ok(runtime)
}

fn validate_model_control_connection(
    connection: &ConnectionDocument,
    runtime: &HostRuntime,
) -> Result<(), Box<dyn std::error::Error>> {
    let normalized = canonicalize_policy(connection.model_control.clone())?;
    if normalized != connection.model_control {
        return Err("model-control executable must be stored as its canonical path".into());
    }
    if !connection.model_control_store.is_absolute() {
        return Err("model-control audit path must be absolute".into());
    }
    let metadata = std::fs::symlink_metadata(&connection.model_control_store)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("model-control audit must be a real regular file".into());
    }
    let audit = std::fs::canonicalize(&connection.model_control_store)?;
    if audit.starts_with(runtime.binding().workspace()) {
        return Err("model-control audit must remain outside the governed workspace".into());
    }
    if audit == connection.kernel_store || audit == connection.handoff_store {
        return Err("model-control audit must be distinct from other stores".into());
    }
    load_audit(&audit)?;
    Ok(())
}

fn kernel_operation(
    command: &'static str,
    outcome: aporic_host::OperationOutcome,
) -> Result<CommandOutput, Box<dyn std::error::Error>> {
    let rejected = outcome.status == KernelCommitStatus::Rejected;
    operation(command, serde_json::to_value(outcome)?, rejected)
}

fn operation(
    command: &'static str,
    value: Value,
    rejected: bool,
) -> Result<CommandOutput, Box<dyn std::error::Error>> {
    Ok(CommandOutput {
        command,
        value,
        rejected,
        raw: false,
    })
}

fn raw(value: Value) -> Result<CommandOutput, Box<dyn std::error::Error>> {
    Ok(CommandOutput {
        command: "codex-hook",
        value,
        rejected: false,
        raw: true,
    })
}

fn success(
    command: &'static str,
    value: Value,
) -> Result<CommandOutput, Box<dyn std::error::Error>> {
    operation(command, value, false)
}

fn read_stdin<T: DeserializeOwned>() -> Result<T, Box<dyn std::error::Error>> {
    let mut input = std::io::stdin().take((MAX_DOCUMENT_BYTES + 1) as u64);
    let mut bytes = Vec::new();
    input.read_to_end(&mut bytes)?;
    if bytes.is_empty() {
        return Err("stdin JSON is required".into());
    }
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err("stdin JSON exceeds the byte limit".into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn read_document<T: DeserializeOwned>(path: &Path) -> Result<T, Box<dyn std::error::Error>> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("connection must be a real regular file".into());
    }
    if metadata.len() > MAX_DOCUMENT_BYTES as u64 {
        return Err("connection exceeds the byte limit".into());
    }
    let bytes = std::fs::read(path)?;
    if bytes.is_empty() || !bytes.ends_with(b"\n") {
        return Err("connection must be nonempty and newline-terminated".into());
    }
    Ok(serde_json::from_slice(&bytes[..bytes.len() - 1])?)
}

fn write_new_document<T: Serialize>(
    path: &Path,
    document: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    let encoded = serde_json::to_vec_pretty(document)?;
    if encoded.len() > MAX_DOCUMENT_BYTES {
        return Err("connection exceeds the byte limit".into());
    }
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(&encoded)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    if let Some(parent) = path.parent() {
        File::open(parent)?.sync_all()?;
    }
    Ok(())
}
