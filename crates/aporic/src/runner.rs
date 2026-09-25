use std::{
    env, fs,
    path::{Path, PathBuf},
    time::Duration,
};

use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    task::JoinHandle,
    time::timeout,
};

use crate::{
    Hub,
    bounded::{
        MAX_GIT_OUTPUT_BYTES, MAX_RECEIPT_ARTIFACT_BYTES, MAX_RECEIPT_ARTIFACT_TOTAL_BYTES,
        sha256_file_bounded,
    },
    domain::{
        ExecutionFinish, ExecutionOutcome, ExecutionStatus, ReceiptArtifactInput,
        SandboxEnforcement,
    },
    git_process::run_hardened_git,
    sandbox::prepare_command,
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

    let Some(executable) = resolved_executable.as_deref() else {
        let (git_head_after, worktree_state_after_sha256) = git_snapshot(&workspace);
        let finish = ExecutionFinish {
            run_id,
            status: ExecutionStatus::Failed,
            resolved_executable: None,
            exit_code: None,
            termination: "executable_not_found".to_owned(),
            sandbox_backend: "none".to_owned(),
            sandbox_enforced: false,
            stdout_sha256: sha256(&[]),
            stdout_bytes: 0,
            stderr_sha256: sha256(&[]),
            stderr_bytes: 0,
            git_head_before,
            git_head_after,
            worktree_state_before_sha256,
            worktree_state_after_sha256,
            artifacts: Vec::new(),
        };
        return hub.finish_execution(&finish);
    };

    // Execute the path resolved before spawn so a later PATH change cannot
    // select a different binary than the one named in the receipt.
    let mut prepared = match prepare_command(&spec, Path::new(executable), &cwd) {
        Ok(prepared) => prepared,
        Err(termination) => {
            let (git_head_after, worktree_state_after_sha256) = git_snapshot(&workspace);
            return hub.finish_execution(&ExecutionFinish {
                run_id,
                status: ExecutionStatus::Failed,
                resolved_executable,
                exit_code: None,
                termination,
                sandbox_backend: "unavailable".to_owned(),
                sandbox_enforced: false,
                stdout_sha256: sha256(&[]),
                stdout_bytes: 0,
                stderr_sha256: sha256(&[]),
                stderr_bytes: 0,
                git_head_before,
                git_head_after,
                worktree_state_before_sha256,
                worktree_state_after_sha256,
                artifacts: Vec::new(),
            });
        }
    };
    let sandbox_backend = prepared.backend.to_owned();
    let command = &mut prepared.command;

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.as_std_mut().process_group(0);
    }

    let (mut status, exit_code, mut termination, stdout_digest, stderr_digest) = match command
        .spawn()
    {
        Ok(mut child) => {
            let stdout = child.stdout.take().expect("piped stdout");
            let stderr = child.stderr.take().expect("piped stderr");
            let mut stdout_task = tokio::spawn(digest_stream(stdout));
            let mut stderr_task = tokio::spawn(digest_stream(stderr));
            let child_pid = child.id();
            let waited = timeout(Duration::from_secs(spec.timeout_seconds), child.wait()).await;
            let (status, exit_code, termination) = match waited {
                Ok(Ok(output_status)) => {
                    let exit_code = output_status.code();
                    let succeeded = exit_code == Some(spec.expected_exit_code);
                    (
                        if succeeded {
                            ExecutionStatus::Succeeded
                        } else {
                            ExecutionStatus::Failed
                        },
                        exit_code,
                        if output_status.code().is_some() {
                            "exited"
                        } else {
                            "signaled"
                        }
                        .to_owned(),
                    )
                }
                Ok(Err(error)) => (ExecutionStatus::Failed, None, format!("wait_error:{error}")),
                Err(_) => {
                    kill_process_group(child_pid);
                    let _ = child.kill().await;
                    let _ = timeout(Duration::from_secs(2), child.wait()).await;
                    (ExecutionStatus::TimedOut, None, "timeout".to_owned())
                }
            };
            let (stdout_digest, stdout_complete) = collect_digest(&mut stdout_task).await;
            let (stderr_digest, stderr_complete) = collect_digest(&mut stderr_task).await;
            let termination = if stdout_complete && stderr_complete {
                termination
            } else {
                format!("{termination}:output_pipe_unclosed")
            };
            (status, exit_code, termination, stdout_digest, stderr_digest)
        }
        Err(error) => (
            ExecutionStatus::Failed,
            None,
            format!("spawn_error:{error}"),
            empty_digest(),
            empty_digest(),
        ),
    };

    let mut artifacts = Vec::new();
    let mut missing_artifact = false;
    let mut artifact_bytes = 0_u64;
    for relative in &spec.artifact_paths {
        let remaining = MAX_RECEIPT_ARTIFACT_TOTAL_BYTES.saturating_sub(artifact_bytes);
        if remaining == 0 {
            missing_artifact = true;
            break;
        }
        match observe_artifact(
            &workspace,
            relative,
            remaining.min(MAX_RECEIPT_ARTIFACT_BYTES),
        ) {
            Ok(artifact) => {
                artifact_bytes = artifact_bytes.saturating_add(artifact.byte_length);
                artifacts.push(artifact);
            }
            Err(_) => {
                missing_artifact = true;
                break;
            }
        }
    }
    if status == ExecutionStatus::Succeeded && termination != "exited" {
        status = ExecutionStatus::Failed;
    }
    let sandbox_enforced = prepared.sandbox_enforced();
    if spec.sandbox_profile.enforcement == SandboxEnforcement::Required && !sandbox_enforced {
        status = ExecutionStatus::Failed;
        termination = "sandbox_setup_failed".to_owned();
    }
    termination = if status == ExecutionStatus::Succeeded && missing_artifact {
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
        sandbox_backend,
        sandbox_enforced,
        stdout_sha256: stdout_digest.0,
        stdout_bytes: stdout_digest.1,
        stderr_sha256: stderr_digest.0,
        stderr_bytes: stderr_digest.1,
        git_head_before,
        git_head_after,
        worktree_state_before_sha256,
        worktree_state_after_sha256,
        artifacts,
    };
    hub.finish_execution(&finish)
}

async fn digest_stream(mut stream: impl AsyncRead + Unpin) -> std::io::Result<(String, u64)> {
    let mut hasher = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = stream.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        bytes = bytes.saturating_add(read as u64);
    }
    Ok((format!("{:x}", hasher.finalize()), bytes))
}

async fn collect_digest(
    task: &mut JoinHandle<std::io::Result<(String, u64)>>,
) -> ((String, u64), bool) {
    match timeout(Duration::from_secs(2), &mut *task).await {
        Ok(Ok(Ok(digest))) => (digest, true),
        Ok(_) => (empty_digest(), false),
        Err(_) => {
            task.abort();
            (empty_digest(), false)
        }
    }
}

fn empty_digest() -> (String, u64) {
    (sha256(&[]), 0)
}

#[cfg(unix)]
fn kill_process_group(pid: Option<u32>) {
    if let Some(pid) = pid {
        // Negative PID addresses the process group established immediately
        // before spawn. This is best-effort containment, not an OS sandbox.
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
}

#[cfg(not(unix))]
fn kill_process_group(_pid: Option<u32>) {}

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

fn observe_artifact(
    workspace: &Path,
    relative: &str,
    remaining_bytes: u64,
) -> Result<ReceiptArtifactInput> {
    let path = fs::canonicalize(workspace.join(relative))?;
    if !path.starts_with(workspace) || !path.is_file() {
        return Err(Error::Invalid(format!("invalid artifact path {relative}")));
    }
    let (digest, byte_length) = sha256_file_bounded(&path, remaining_bytes, "receipt artifact")?;
    Ok(ReceiptArtifactInput {
        workspace_relative_path: relative.to_owned(),
        sha256: digest,
        byte_length,
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
    let output = run_hardened_git(workspace, args, MAX_GIT_OUTPUT_BYTES).ok()?;
    output.status.success().then_some(output.stdout)
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
