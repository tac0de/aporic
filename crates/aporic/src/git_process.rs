use std::{
    env,
    io::{self, Read},
    path::Path,
    process::{Child, Command, ExitStatus, Output, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

pub(crate) fn run_hardened_git(
    workspace: &Path,
    args: &[&str],
    max_output_bytes: usize,
) -> io::Result<Output> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(workspace)
        .env_clear()
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_PAGER", "cat")
        .args(["-c", "core.fsmonitor=false"])
        .args(["-c", "diff.external="])
        .args(["-c", "submodule.recurse=false"])
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    copy_git_environment(&mut command);
    run_bounded(command, max_output_bytes)
}

fn copy_git_environment(command: &mut Command) {
    if let Some(path) = env::var_os("PATH") {
        command.env("PATH", path);
    }
    #[cfg(windows)]
    for name in ["SYSTEMROOT", "WINDIR", "COMSPEC", "PATHEXT", "TEMP", "TMP"] {
        if let Some(value) = env::var_os(name) {
            command.env(name, value);
        }
    }
}

fn run_bounded(mut command: Command, max_output_bytes: usize) -> io::Result<Output> {
    let mut child = command.spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("bounded command stdout was not piped"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("bounded command stderr was not piped"))?;
    let exceeded = Arc::new(AtomicBool::new(false));
    let stdout_task = capture_bounded(stdout, max_output_bytes, Arc::clone(&exceeded));
    let stderr_task = capture_bounded(stderr, max_output_bytes, Arc::clone(&exceeded));
    let status = wait_or_kill_on_limit(&mut child, &exceeded)?;
    let stdout = join_capture(stdout_task)?;
    let stderr = join_capture(stderr_task)?;
    if exceeded.load(Ordering::Acquire) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Git metadata output exceeded {max_output_bytes} bytes"),
        ));
    }
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn capture_bounded(
    mut stream: impl Read + Send + 'static,
    max_output_bytes: usize,
    exceeded: Arc<AtomicBool>,
) -> thread::JoinHandle<io::Result<Vec<u8>>> {
    thread::spawn(move || {
        let mut output = Vec::new();
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            let read = stream.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            let remaining = max_output_bytes.saturating_sub(output.len());
            output.extend_from_slice(&buffer[..read.min(remaining)]);
            if read > remaining {
                exceeded.store(true, Ordering::Release);
            }
        }
        Ok(output)
    })
}

fn wait_or_kill_on_limit(child: &mut Child, exceeded: &AtomicBool) -> io::Result<ExitStatus> {
    loop {
        if exceeded.load(Ordering::Acquire) {
            let _ = child.kill();
            return child.wait();
        }
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        thread::sleep(Duration::from_millis(2));
    }
}

fn join_capture(task: thread::JoinHandle<io::Result<Vec<u8>>>) -> io::Result<Vec<u8>> {
    task.join()
        .map_err(|_| io::Error::other("bounded output reader panicked"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_capture_rejects_output_without_retaining_it() {
        let mut command = if cfg!(windows) {
            let mut command = Command::new("cmd");
            command.args(["/D", "/C", "for /L %i in (1,1,200) do @echo 1234567890"]);
            command
        } else {
            let mut command = Command::new("sh");
            command.args(["-c", "yes 1234567890 | head -n 200"]);
            command
        };
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let error = run_bounded(command, 128).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
