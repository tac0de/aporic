use aporic_boundary::BoundaryPolicy;
use aporic_handoff::CommitStatus as HandoffCommitStatus;
use aporic_host::{ConnectRequest, HostRuntime, connect};
use aporic_kernel::CommitStatus as KernelCommitStatus;
use aporic_projects::{BindRequest, bind_project};
use aporic_roles::{HostPolicy, load_role};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const CONNECTION_SCHEMA_VERSION: u32 = 1;
const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;

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
    host_policy: HostPolicy,
    boundary_policy: BoundaryPolicy,
}

#[derive(Debug, Serialize)]
struct StatusDocument<'a> {
    project_id: &'a str,
    scope: &'a str,
    role_id: &'a str,
    profile_sha256: &'a str,
    kernel_revision: u64,
    handoff_revision: u64,
    head: String,
    head_changed: bool,
    dirty: bool,
}

struct CommandOutput {
    command: &'static str,
    value: Value,
    rejected: bool,
}

fn main() {
    let exit_code = match run() {
        Ok(output) => {
            println!(
                "{}",
                serde_json::to_string(&json!({
                    "ok": !output.rejected,
                    "command": output.command,
                    "result": output.value,
                }))
                .expect("JSON output is serializable")
            );
            if output.rejected { 2 } else { 0 }
        }
        Err(error) => {
            eprintln!(
                "{}",
                serde_json::to_string(&json!({"ok": false, "error": error.to_string()}))
                    .expect("JSON error is serializable")
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
    if arguments.next().is_some() {
        return Err("too many arguments".into());
    }
    if !connection_path.is_absolute() {
        return Err("connection file path must be absolute".into());
    }

    match command.as_str() {
        "bind" => bind(&connection_path),
        "status" => with_runtime(&connection_path, |runtime, connection| {
            let observation = runtime.observe()?;
            let kernel = aporic_kernel::load(&connection.kernel_store)?;
            let handoff = aporic_handoff::load(&connection.handoff_store)?;
            success(
                "status",
                serde_json::to_value(StatusDocument {
                    project_id: runtime.binding().project_id(),
                    scope: runtime.scope(),
                    role_id: runtime.profile().role_id(),
                    profile_sha256: runtime.profile().profile_sha256(),
                    kernel_revision: kernel.state().revision,
                    handoff_revision: handoff.state().revision,
                    head: observation.head,
                    head_changed: observation.head_changed,
                    dirty: observation.dirty,
                })?,
            )
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
    let metadata = std::fs::symlink_metadata(parent)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("connection parent must be a real directory".into());
    }
    let parent = std::fs::canonicalize(parent)?;
    let connection_path = parent.join(path.file_name().ok_or("connection filename is missing")?);
    let role_directory = std::fs::canonicalize(&input.role_directory)?;
    load_role(&role_directory, &input.host_policy)?;

    let registry = parent.join("projects");
    let kernel_store = parent.join("kernel.jsonl");
    let handoff_store = parent.join("handoff.jsonl");
    if connection_path.try_exists()? || kernel_store.try_exists()? || handoff_store.try_exists()? {
        return Err("connection or state store already exists".into());
    }

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
    let connection = ConnectionDocument {
        schema_version: CONNECTION_SCHEMA_VERSION,
        registry,
        project_id: input.project_id,
        role_directory,
        kernel_store,
        handoff_store,
        host_policy: input.host_policy,
        boundary_policy: input.boundary_policy,
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
    let runtime = connect(ConnectRequest {
        registry: connection.registry.clone(),
        project_id: connection.project_id.clone(),
        role_directory: connection.role_directory.clone(),
        kernel_store: connection.kernel_store.clone(),
        handoff_store: connection.handoff_store.clone(),
        host_policy: connection.host_policy.clone(),
        boundary_policy: connection.boundary_policy.clone(),
    })?;
    operation(&runtime, &connection)
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
