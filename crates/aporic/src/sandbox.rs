use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Stdio,
};

use tokio::process::Command;
#[cfg(target_os = "linux")]
use uuid::Uuid;

use crate::domain::{
    CommandSpec, SandboxBackendStatus, SandboxEnforcement, SandboxNetworkAccess,
    SandboxWorkspaceAccess,
};

#[cfg(target_os = "linux")]
pub(crate) fn backend_status() -> SandboxBackendStatus {
    SandboxBackendStatus {
        platform: env::consts::OS.to_owned(),
        backend: "linux_bubblewrap".to_owned(),
        supported: true,
        installed: resolve_bwrap().is_some(),
        required_profiles_fail_closed: true,
        network_policy: "deny_only".to_owned(),
    }
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn backend_status() -> SandboxBackendStatus {
    SandboxBackendStatus {
        platform: env::consts::OS.to_owned(),
        backend: "none".to_owned(),
        supported: false,
        installed: false,
        required_profiles_fail_closed: true,
        network_policy: "unsupported".to_owned(),
    }
}

pub(crate) struct PreparedCommand {
    pub command: Command,
    pub backend: &'static str,
    control_directory: Option<PathBuf>,
}

impl PreparedCommand {
    pub fn sandbox_enforced(&self) -> bool {
        self.control_directory
            .as_ref()
            .is_some_and(|directory| directory.join("ready").is_file())
    }
}

impl Drop for PreparedCommand {
    fn drop(&mut self) {
        if let Some(directory) = &self.control_directory {
            let _ = fs::remove_file(directory.join("ready"));
            let _ = fs::remove_dir(directory);
        }
    }
}

pub(crate) fn prepare_command(
    spec: &CommandSpec,
    executable: &Path,
    cwd: &Path,
) -> std::result::Result<PreparedCommand, String> {
    match spec.sandbox_profile.enforcement {
        SandboxEnforcement::Host => Ok(prepare_host_command(spec, executable, cwd)),
        SandboxEnforcement::Required => prepare_required_command(spec, executable),
    }
}

fn prepare_host_command(spec: &CommandSpec, executable: &Path, cwd: &Path) -> PreparedCommand {
    let mut command = Command::new(executable);
    command
        .args(&spec.args)
        .current_dir(cwd)
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear();
    copy_host_environment(&mut command);
    PreparedCommand {
        command,
        backend: "none",
        control_directory: None,
    }
}

#[cfg(target_os = "linux")]
fn prepare_required_command(
    spec: &CommandSpec,
    executable: &Path,
) -> std::result::Result<PreparedCommand, String> {
    let bwrap = resolve_bwrap().ok_or_else(|| {
        "sandbox_unavailable: bubblewrap was not found; required isolation will not fall back"
            .to_owned()
    })?;
    if !visible_in_linux_sandbox(executable, Path::new(&spec.workspace)) {
        return Err(format!(
            "sandbox_unavailable: executable {} is outside the sandbox system and workspace roots",
            executable.display()
        ));
    }
    let sandbox_executable = executable
        .strip_prefix(&spec.workspace)
        .map(|relative| Path::new("/workspace").join(relative))
        .unwrap_or_else(|_| executable.to_path_buf());
    let sandbox_shell = if Path::new("/usr/bin/sh").is_file() {
        "/usr/bin/sh"
    } else {
        "/bin/sh"
    };
    let control_directory =
        env::temp_dir().join(format!("aporic-sandbox-control-{}", Uuid::now_v7()));
    fs::create_dir(&control_directory).map_err(|error| {
        format!("sandbox_unavailable: could not create control directory: {error}")
    })?;
    let sandbox_working_directory = sandbox_cwd(&spec.workspace_relative_cwd);

    let mut command = Command::new(bwrap);
    command
        .args([
            "--unshare-all",
            "--die-with-parent",
            "--new-session",
            "--cap-drop",
            "ALL",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
            "--dir",
            "/etc",
            "--dir",
            "/tmp/aporic-home",
            "--dir",
            "/workspace",
            "--dir",
            "/aporic-control",
        ])
        .args(system_mount_arguments())
        .arg(match spec.sandbox_profile.workspace_access {
            SandboxWorkspaceAccess::ReadOnly => "--ro-bind",
            SandboxWorkspaceAccess::ReadWrite => "--bind",
        })
        .arg(&spec.workspace)
        .arg("/workspace")
        .arg("--bind")
        .arg(&control_directory)
        .arg("/aporic-control")
        .arg("--chdir")
        .arg(&sandbox_working_directory)
        .args([
            "--clearenv",
            "--setenv",
            "HOME",
            "/tmp/aporic-home",
            "--setenv",
            "PATH",
            "/usr/local/bin:/usr/bin:/bin",
            "--setenv",
            "LANG",
            "C.UTF-8",
            "--",
            sandbox_shell,
            "-c",
            "printf ready > /aporic-control/ready; exec \"$@\"",
            "aporic-sandbox",
        ])
        .arg(sandbox_executable)
        .args(&spec.args)
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear();

    Ok(PreparedCommand {
        command,
        backend: "linux_bubblewrap",
        control_directory: Some(control_directory),
    })
}

#[cfg(not(target_os = "linux"))]
fn prepare_required_command(
    _spec: &CommandSpec,
    _executable: &Path,
) -> std::result::Result<PreparedCommand, String> {
    Err(format!(
        "sandbox_unavailable: required OS isolation has no backend for {}",
        env::consts::OS
    ))
}

#[cfg(target_os = "linux")]
fn resolve_bwrap() -> Option<PathBuf> {
    if let Some(configured) = env::var_os("APORIC_BWRAP") {
        let configured = PathBuf::from(configured);
        if !configured.is_absolute() {
            return None;
        }
        return fs::canonicalize(configured)
            .ok()
            .filter(|path| path.is_file());
    }
    env::var_os("PATH").and_then(|path| {
        env::split_paths(&path)
            .map(|directory| directory.join("bwrap"))
            .find(|candidate| candidate.is_file())
            .and_then(|candidate| fs::canonicalize(candidate).ok())
    })
}

#[cfg(target_os = "linux")]
fn visible_in_linux_sandbox(executable: &Path, workspace: &Path) -> bool {
    ["/usr", "/bin", "/sbin", "/lib", "/lib64"]
        .iter()
        .any(|root| executable.starts_with(Path::new(root)))
        || executable.starts_with(workspace)
}

#[cfg(target_os = "linux")]
fn system_mount_arguments() -> Vec<String> {
    let mut arguments = Vec::new();
    for path in ["/usr", "/bin", "/sbin", "/lib", "/lib64"] {
        if Path::new(path).exists() {
            arguments.extend(["--ro-bind".to_owned(), path.to_owned(), path.to_owned()]);
        }
    }
    for path in [
        "/etc/ld.so.cache",
        "/etc/ld.so.conf",
        "/etc/ld.so.conf.d",
        "/etc/passwd",
        "/etc/group",
        "/etc/nsswitch.conf",
        "/etc/localtime",
    ] {
        if Path::new(path).exists() {
            arguments.extend(["--ro-bind".to_owned(), path.to_owned(), path.to_owned()]);
        }
    }
    arguments
}

#[cfg(target_os = "linux")]
fn sandbox_cwd(relative: &str) -> String {
    if relative == "." {
        "/workspace".to_owned()
    } else {
        format!("/workspace/{relative}")
    }
}

fn copy_host_environment(command: &mut Command) {
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
    #[cfg(windows)]
    for name in [
        "SYSTEMROOT",
        "WINDIR",
        "COMSPEC",
        "PATHEXT",
        "TEMP",
        "TMP",
        "USERPROFILE",
    ] {
        if let Some(value) = env::var_os(name) {
            command.env(name, value);
        }
    }
}

pub(crate) fn validate_profile(
    enforcement: &SandboxEnforcement,
    workspace: &SandboxWorkspaceAccess,
    network: &SandboxNetworkAccess,
) -> std::result::Result<(), &'static str> {
    match enforcement {
        SandboxEnforcement::Host
            if *workspace != SandboxWorkspaceAccess::ReadWrite
                || *network != SandboxNetworkAccess::Inherit =>
        {
            Err("host execution cannot claim unenforced workspace or network restrictions")
        }
        SandboxEnforcement::Required if *network != SandboxNetworkAccess::Deny => {
            Err("required isolation currently requires network_access=deny")
        }
        _ => Ok(()),
    }
}
