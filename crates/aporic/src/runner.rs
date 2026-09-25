use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command as StdCommand,
    time::Duration,
};

use sha2::{Digest, Sha256};
use tokio::{process::Command, time::timeout};

use crate::{
    Hub,
    domain::{ExecutionFinish, ExecutionOutcome, ExecutionStatus, ReceiptArtifactInput},
    store::{Error, Result},
};

pub async fn verify(hub: &Hub, spec_id: &str) -> Result<ExecutionOutcome> {
    let started = hub.start_execution(spec_id)?;
    let spec = started.spec;
    let run_id = started.run.run_id;
    let workspace = PathBuf::from(&spec.workspace);
    let cwd = workspace.join(&spec.workspace_relative_cwd);
    let resolved_executable = resolve_executable(&spec.program, &cwd);
    let (git_head_before, worktree_state_before_sha256) = git_snapshot(&workspace);

    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .current_dir(&cwd)
        .kill_on_drop(true)
        .env_clear();
    copy_safe_environment(&mut command);

    let execution = timeout(Duration::from_secs(spec.timeout_seconds), command.output()).await;
    let (mut status, exit_code, termination, stdout, stderr) = match execution {
        Ok(Ok(output)) => {
            let exit_code = output.status.code();
            let succeeded = exit_code == Some(spec.expected_exit_code);
            (
                if succeeded {
                    ExecutionStatus::Succeeded
                } else {
                    ExecutionStatus::Failed
                },
                exit_code,
                if output.status.code().is_some() {
                    "exited"
                } else {
                    "signaled"
                }
                .to_owned(),
                output.stdout,
                output.stderr,
            )
        }
        Ok(Err(error)) => (
            ExecutionStatus::Failed,
            None,
            "spawn_error".to_owned(),
            Vec::new(),
            error.to_string().into_bytes(),
        ),
        Err(_) => (
            ExecutionStatus::TimedOut,
            None,
            "timeout".to_owned(),
            Vec::new(),
            Vec::new(),
        ),
    };

    let mut artifacts = Vec::new();
    let mut missing_artifact = false;
    for relative in &spec.artifact_paths {
        match observe_artifact(&workspace, relative) {
            Ok(artifact) => artifacts.push(artifact),
            Err(_) => missing_artifact = true,
        }
    }
    let termination = if status == ExecutionStatus::Succeeded && missing_artifact {
        status = ExecutionStatus::Failed;
        "missing_or_invalid_artifact".to_owned()
    } else {
        termination
    };
    let (git_head_after, worktree_state_after_sha256) = git_snapshot(&workspace);
    let finish = ExecutionFinish {
        run_id,
        status,
        resolved_executable,
        exit_code,
        termination,
        stdout_sha256: sha256(&stdout),
        stdout_bytes: stdout.len() as u64,
        stderr_sha256: sha256(&stderr),
        stderr_bytes: stderr.len() as u64,
        git_head_before,
        git_head_after,
        worktree_state_before_sha256,
        worktree_state_after_sha256,
        artifacts,
    };
    hub.finish_execution(&finish)
}

fn copy_safe_environment(command: &mut Command) {
    for name in [
        "PATH",
        "HOME",
        "TMPDIR",
        "LANG",
        "LC_ALL",
        "CARGO_HOME",
        "RUSTUP_HOME",
    ] {
        if let Some(value) = env::var_os(name) {
            command.env(name, value);
        }
    }
}

fn resolve_executable(program: &str, cwd: &Path) -> Option<String> {
    let path = Path::new(program);
    if path.components().count() > 1 {
        let candidate = if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        };
        return fs::canonicalize(candidate)
            .ok()
            .map(|path| path.to_string_lossy().into_owned());
    }
    env::var_os("PATH").and_then(|path| {
        env::split_paths(&path)
            .map(|directory| directory.join(program))
            .find(|candidate| candidate.is_file())
            .and_then(|candidate| fs::canonicalize(candidate).ok())
            .map(|candidate| candidate.to_string_lossy().into_owned())
    })
}

fn observe_artifact(workspace: &Path, relative: &str) -> Result<ReceiptArtifactInput> {
    let path = fs::canonicalize(workspace.join(relative))?;
    if !path.starts_with(workspace) || !path.is_file() {
        return Err(Error::Invalid(format!("invalid artifact path {relative}")));
    }
    let bytes = fs::read(path)?;
    Ok(ReceiptArtifactInput {
        workspace_relative_path: relative.to_owned(),
        sha256: sha256(&bytes),
        byte_length: bytes.len() as u64,
    })
}

fn git_snapshot(workspace: &Path) -> (Option<String>, Option<String>) {
    let head = git_output(workspace, &["rev-parse", "HEAD"])
        .map(|bytes| String::from_utf8_lossy(&bytes).trim().to_owned())
        .filter(|value| !value.is_empty());
    let state =
        git_output(workspace, &["status", "--porcelain=v2", "-z"]).map(|bytes| sha256(&bytes));
    (head, state)
}

fn git_output(workspace: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = StdCommand::new("git")
        .arg("-C")
        .arg(workspace)
        .args(args)
        .env_clear()
        .env("PATH", env::var_os("PATH")?)
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
