//! Immutable external bindings between an Aporic project ID and a Git checkout.
//!
//! P0 never writes into the governed repository. It records the canonical Git
//! root, an exact remote URL, and the observed HEAD in an external registry.
//! Later observations fail closed if the workspace or remote no longer match.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

pub const PROJECT_BINDING_SCHEMA_VERSION: u32 = 1;
pub const MAX_BINDING_BYTES: usize = 16 * 1_024;
const MAX_ID_BYTES: usize = 128;
const MAX_REMOTE_BYTES: usize = 2_048;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(serde_json::Error),
    InvalidBinding(&'static str),
    Git { operation: &'static str },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Json(error) => write!(formatter, "JSON error: {error}"),
            Self::InvalidBinding(reason) => write!(formatter, "invalid project binding: {reason}"),
            Self::Git { operation } => write!(formatter, "git operation failed: {operation}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindRequest {
    pub project_id: String,
    pub workspace: PathBuf,
    pub remote_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BindingDocument {
    schema_version: u32,
    project_id: String,
    workspace: PathBuf,
    remote_name: String,
    remote_url: String,
    bound_head: String,
    binding_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectBinding(BindingDocument);

impl ProjectBinding {
    pub fn project_id(&self) -> &str {
        &self.0.project_id
    }

    pub fn workspace(&self) -> &Path {
        &self.0.workspace
    }

    pub fn remote_name(&self) -> &str {
        &self.0.remote_name
    }

    pub fn remote_url(&self) -> &str {
        &self.0.remote_url
    }

    pub fn bound_head(&self) -> &str {
        &self.0.bound_head
    }

    pub fn binding_sha256(&self) -> &str {
        &self.0.binding_sha256
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectObservation {
    pub project_id: String,
    pub binding_sha256: String,
    pub head: String,
    pub head_changed: bool,
    pub dirty: bool,
}

#[derive(Serialize)]
struct BindingFingerprint<'a> {
    schema_version: u32,
    project_id: &'a str,
    workspace: &'a Path,
    remote_name: &'a str,
    remote_url: &'a str,
    bound_head: &'a str,
}

pub fn bind_project(registry: impl AsRef<Path>, request: BindRequest) -> Result<ProjectBinding> {
    validate_name(&request.project_id, "project id is invalid")?;
    validate_name(&request.remote_name, "remote name is invalid")?;
    let git = inspect_git(&request.workspace, &request.remote_name)?;

    let registry = registry.as_ref();
    if !registry.is_absolute() {
        return Err(Error::InvalidBinding("registry path must be absolute"));
    }
    let prospective_registry = prospective_canonical_path(registry)?;
    if prospective_registry.starts_with(&git.workspace) {
        return Err(Error::InvalidBinding(
            "registry must remain outside the governed workspace",
        ));
    }
    std::fs::create_dir_all(registry)?;
    let registry = std::fs::canonicalize(registry)?;
    if registry.starts_with(&git.workspace) {
        return Err(Error::InvalidBinding(
            "registry must remain outside the governed workspace",
        ));
    }

    let binding_sha256 = binding_fingerprint(
        &request.project_id,
        &git.workspace,
        &request.remote_name,
        &git.remote_url,
        &git.head,
    )?;
    let document = BindingDocument {
        schema_version: PROJECT_BINDING_SCHEMA_VERSION,
        project_id: request.project_id,
        workspace: git.workspace,
        remote_name: request.remote_name,
        remote_url: git.remote_url,
        bound_head: git.head,
        binding_sha256,
    };
    let encoded = serde_json::to_vec_pretty(&document)?;
    if encoded.len() > MAX_BINDING_BYTES {
        return Err(Error::InvalidBinding("binding exceeds the byte limit"));
    }
    let path = binding_path(&registry, &document.project_id);
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(&encoded)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    sync_directory(&registry)?;
    Ok(ProjectBinding(document))
}

pub fn load_project(registry: impl AsRef<Path>, project_id: &str) -> Result<ProjectBinding> {
    validate_name(project_id, "project id is invalid")?;
    let registry = std::fs::canonicalize(registry.as_ref())?;
    let path = binding_path(&registry, project_id);
    let metadata = std::fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Error::InvalidBinding("binding must be a real regular file"));
    }
    if metadata.len() > MAX_BINDING_BYTES as u64 {
        return Err(Error::InvalidBinding("binding exceeds the byte limit"));
    }
    let bytes = std::fs::read(path)?;
    if bytes.is_empty() || bytes.len() > MAX_BINDING_BYTES || !bytes.ends_with(b"\n") {
        return Err(Error::InvalidBinding(
            "binding must be bounded and newline-terminated",
        ));
    }
    let document: BindingDocument = serde_json::from_slice(&bytes[..bytes.len() - 1])?;
    validate_document(&document, project_id)?;
    if registry.starts_with(&document.workspace) {
        return Err(Error::InvalidBinding(
            "registry must remain outside the governed workspace",
        ));
    }
    Ok(ProjectBinding(document))
}

pub fn observe_project(binding: &ProjectBinding) -> Result<ProjectObservation> {
    let git = inspect_git(binding.workspace(), binding.remote_name())?;
    if git.workspace != binding.workspace() {
        return Err(Error::InvalidBinding("workspace identity changed"));
    }
    if git.remote_url != binding.remote_url() {
        return Err(Error::InvalidBinding("remote URL changed"));
    }
    Ok(ProjectObservation {
        project_id: binding.project_id().into(),
        binding_sha256: binding.binding_sha256().into(),
        head_changed: git.head != binding.bound_head(),
        head: git.head,
        dirty: git.dirty,
    })
}

struct GitObservation {
    workspace: PathBuf,
    remote_url: String,
    head: String,
    dirty: bool,
}

fn inspect_git(workspace: &Path, remote_name: &str) -> Result<GitObservation> {
    let workspace = std::fs::canonicalize(workspace)?;
    if !workspace.is_dir() {
        return Err(Error::InvalidBinding("workspace must be a directory"));
    }
    let root = git_output(&workspace, &["rev-parse", "--show-toplevel"], "find root")?;
    let root = std::fs::canonicalize(root)?;
    let head = git_output(&root, &["rev-parse", "--verify", "HEAD"], "read HEAD")?;
    validate_head(&head)?;
    let remote_url = git_output(
        &root,
        &["remote", "get-url", remote_name],
        "read remote URL",
    )?;
    validate_remote(&remote_url)?;
    let status = git_output(
        &root,
        &["status", "--porcelain=v1", "--untracked-files=normal"],
        "read status",
    )?;
    Ok(GitObservation {
        workspace: root,
        remote_url,
        head,
        dirty: !status.is_empty(),
    })
}

fn git_output(workspace: &Path, arguments: &[&str], operation: &'static str) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace)
        .args(arguments)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()?;
    if !output.status.success() {
        return Err(Error::Git { operation });
    }
    let value = String::from_utf8(output.stdout)
        .map_err(|_| Error::InvalidBinding("git output must be valid UTF-8"))?;
    Ok(value.trim().to_owned())
}

fn validate_document(document: &BindingDocument, expected_project_id: &str) -> Result<()> {
    if document.schema_version != PROJECT_BINDING_SCHEMA_VERSION {
        return Err(Error::InvalidBinding(
            "unsupported project binding schema version",
        ));
    }
    if document.project_id != expected_project_id {
        return Err(Error::InvalidBinding("project id does not match filename"));
    }
    validate_name(&document.project_id, "project id is invalid")?;
    validate_name(&document.remote_name, "remote name is invalid")?;
    if !document.workspace.is_absolute() {
        return Err(Error::InvalidBinding("workspace path must be absolute"));
    }
    validate_remote(&document.remote_url)?;
    validate_head(&document.bound_head)?;
    let expected = binding_fingerprint(
        &document.project_id,
        &document.workspace,
        &document.remote_name,
        &document.remote_url,
        &document.bound_head,
    )?;
    if document.binding_sha256 != expected {
        return Err(Error::InvalidBinding("binding fingerprint mismatch"));
    }
    Ok(())
}

fn binding_fingerprint(
    project_id: &str,
    workspace: &Path,
    remote_name: &str,
    remote_url: &str,
    bound_head: &str,
) -> Result<String> {
    let encoded = serde_json::to_vec(&BindingFingerprint {
        schema_version: PROJECT_BINDING_SCHEMA_VERSION,
        project_id,
        workspace,
        remote_name,
        remote_url,
        bound_head,
    })?;
    Ok(format!("{:x}", Sha256::digest(encoded)))
}

fn binding_path(registry: &Path, project_id: &str) -> PathBuf {
    registry.join(format!("{project_id}.json"))
}

fn prospective_canonical_path(path: &Path) -> Result<PathBuf> {
    if path
        .components()
        .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(Error::InvalidBinding(
            "registry path must not contain dot components",
        ));
    }
    let mut cursor = path;
    let mut missing = Vec::new();
    while !cursor.try_exists()? {
        let name = cursor
            .file_name()
            .ok_or(Error::InvalidBinding("registry path is invalid"))?;
        missing.push(name.to_owned());
        cursor = cursor
            .parent()
            .ok_or(Error::InvalidBinding("registry path is invalid"))?;
    }
    let mut resolved = std::fs::canonicalize(cursor)?;
    for component in missing.iter().rev() {
        resolved.push(component);
    }
    Ok(resolved)
}

fn validate_name(value: &str, reason: &'static str) -> Result<()> {
    let valid = !value.is_empty()
        && value.len() <= MAX_ID_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        });
    if !valid {
        return Err(Error::InvalidBinding(reason));
    }
    Ok(())
}

fn validate_head(value: &str) -> Result<()> {
    if !matches!(value.len(), 40 | 64) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::InvalidBinding("HEAD must be a full Git object ID"));
    }
    Ok(())
}

fn validate_remote(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > MAX_REMOTE_BYTES
        || value.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(Error::InvalidBinding("remote URL is invalid"));
    }
    if let Some((_, remainder)) = value.split_once("://") {
        let authority = remainder.split('/').next().unwrap_or(remainder);
        if authority.contains('@') {
            return Err(Error::InvalidBinding(
                "remote URL must not contain embedded credentials",
            ));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<()> {
    Ok(())
}
