use crate::codex::MAX_SCOPE_BYTES;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

pub const PROJECT_CONFIG_SCHEMA_VERSION: u32 = 1;
pub const PROJECT_DIR: &str = ".aporic";
pub const PROJECT_CONFIG_FILE: &str = "config.json";
pub const PROJECT_POLICY_FILE: &str = "policy.json";

const DEFAULT_POLICY: &str = r#"{
  "schema_version": 4,
  "lifecycle_mode": "development",
  "tools": {
    "apply_patch": {
      "require_plan": true,
      "require_grant": false,
      "require_intent": true,
      "auto_allow_low_risk_profiles": [
        { "profile_id": "local-code", "profile_version": "1" }
      ]
    }
  }
}
"#;

#[derive(Debug)]
pub enum ProjectError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Invalid(String),
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::Json(error) => write!(f, "JSON error: {error}"),
            Self::Invalid(reason) => write!(f, "invalid project configuration: {reason}"),
        }
    }
}

impl std::error::Error for ProjectError {}

impl From<std::io::Error> for ProjectError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for ProjectError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub schema_version: u32,
    pub scope: String,
}

impl ProjectConfig {
    pub fn new(scope: impl Into<String>) -> Result<Self, ProjectError> {
        let config = Self {
            schema_version: PROJECT_CONFIG_SCHEMA_VERSION,
            scope: scope.into(),
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ProjectError> {
        if self.schema_version != PROJECT_CONFIG_SCHEMA_VERSION {
            return Err(ProjectError::Invalid(format!(
                "unsupported schema_version {}",
                self.schema_version
            )));
        }
        let bytes = self.scope.as_bytes();
        let valid_first = bytes
            .first()
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_');
        let valid_rest = bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'.' | b'_' | b'-'));
        if !valid_first || !valid_rest || bytes.len() > MAX_SCOPE_BYTES {
            return Err(ProjectError::Invalid(format!(
                "scope must be 1..={MAX_SCOPE_BYTES} bytes and use only ASCII letters, digits, dot, underscore, or hyphen"
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProject {
    pub workspace: PathBuf,
    pub scope: String,
    pub policy_path: PathBuf,
    pub store_path: PathBuf,
}

pub fn default_data_root() -> Result<PathBuf, ProjectError> {
    if let Some(path) = std::env::var_os("APORIC_DATA_HOME") {
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            return Err(ProjectError::Invalid(
                "APORIC_DATA_HOME must be an absolute path".into(),
            ));
        }
        return Ok(path);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| ProjectError::Invalid("HOME is not set".into()))?;
    if !home.is_absolute() {
        return Err(ProjectError::Invalid(
            "HOME must be an absolute path".into(),
        ));
    }
    Ok(home.join(".local/share/aporic"))
}

pub fn initialize_project(
    workspace: impl AsRef<Path>,
    scope: impl Into<String>,
) -> Result<ResolvedProject, ProjectError> {
    let workspace = fs::canonicalize(workspace)?;
    if !workspace.is_dir() {
        return Err(ProjectError::Invalid("workspace is not a directory".into()));
    }
    let config = ProjectConfig::new(scope)?;
    let project_dir = workspace.join(PROJECT_DIR);
    fs::create_dir_all(&project_dir)?;
    reject_symlink(&project_dir, "project directory")?;

    let config_path = project_dir.join(PROJECT_CONFIG_FILE);
    let policy_path = project_dir.join(PROJECT_POLICY_FILE);
    if config_path.exists() || policy_path.exists() {
        return Err(ProjectError::Invalid(
            "config.json or policy.json already exists; refusing to overwrite".into(),
        ));
    }

    write_new(&policy_path, DEFAULT_POLICY.as_bytes())?;
    let config_bytes = serde_json::to_vec_pretty(&config)?;
    if let Err(error) = write_new(&config_path, &[config_bytes, b"\n".to_vec()].concat()) {
        let _ = fs::remove_file(&policy_path);
        return Err(error);
    }

    Ok(ResolvedProject {
        workspace,
        scope: config.scope,
        policy_path,
        store_path: PathBuf::new(),
    })
}

pub fn discover_project(
    cwd: impl AsRef<Path>,
    data_root: impl AsRef<Path>,
) -> Result<Option<ResolvedProject>, ProjectError> {
    let cwd = fs::canonicalize(cwd)?;
    if !cwd.is_dir() {
        return Err(ProjectError::Invalid("hook cwd is not a directory".into()));
    }

    for workspace in cwd.ancestors() {
        let project_dir = workspace.join(PROJECT_DIR);
        let project_metadata = match fs::symlink_metadata(&project_dir) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        if project_metadata.file_type().is_symlink() {
            return Err(ProjectError::Invalid(
                "project directory must not be a symbolic link".into(),
            ));
        }
        if !project_metadata.is_dir() {
            return Err(ProjectError::Invalid(
                "project directory is not a directory".into(),
            ));
        }
        let config_path = project_dir.join(PROJECT_CONFIG_FILE);
        let config_metadata = match fs::symlink_metadata(&config_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        if config_metadata.file_type().is_symlink() {
            return Err(ProjectError::Invalid(
                "project config must not be a symbolic link".into(),
            ));
        }
        let config: ProjectConfig = serde_json::from_slice(&fs::read(&config_path)?)?;
        config.validate()?;

        let policy_path = project_dir.join(PROJECT_POLICY_FILE);
        if !policy_path.is_file() {
            return Err(ProjectError::Invalid("policy.json is missing".into()));
        }
        reject_symlink(&policy_path, "project policy")?;

        return Ok(Some(ResolvedProject {
            workspace: workspace.to_path_buf(),
            scope: config.scope,
            policy_path,
            store_path: store_path(data_root.as_ref(), workspace)?,
        }));
    }
    Ok(None)
}

pub fn store_path(data_root: &Path, workspace: &Path) -> Result<PathBuf, ProjectError> {
    if !data_root.is_absolute() || !workspace.is_absolute() {
        return Err(ProjectError::Invalid(
            "data root and workspace must be absolute".into(),
        ));
    }
    if data_root
        .components()
        .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(ProjectError::Invalid(
            "data root must not contain dot or parent components".into(),
        ));
    }
    let resolved_data_root = resolve_with_missing_tail(data_root)?;
    if resolved_data_root.starts_with(workspace) {
        return Err(ProjectError::Invalid(
            "data root must be outside the bound workspace".into(),
        ));
    }

    let mut path = data_root.join("workspaces/v1");
    for chunk in workspace.as_os_str().as_encoded_bytes().chunks(16) {
        let mut encoded = String::with_capacity(chunk.len() * 2);
        for byte in chunk {
            use std::fmt::Write as _;
            write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
        }
        path.push(encoded);
    }
    Ok(path.join("store/events.jsonl"))
}

fn resolve_with_missing_tail(path: &Path) -> Result<PathBuf, ProjectError> {
    let mut existing = path;
    let mut tail = Vec::new();
    loop {
        match fs::canonicalize(existing) {
            Ok(mut resolved) => {
                for component in tail.iter().rev() {
                    resolved.push(component);
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = existing.file_name().ok_or_else(|| {
                    ProjectError::Invalid("data root has no existing ancestor".into())
                })?;
                tail.push(name.to_os_string());
                existing = existing.parent().ok_or_else(|| {
                    ProjectError::Invalid("data root has no existing ancestor".into())
                })?;
            }
            Err(error) => return Err(error.into()),
        }
    }
}

fn reject_symlink(path: &Path, label: &str) -> Result<(), ProjectError> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(ProjectError::Invalid(format!(
            "{label} must not be a symbolic link"
        )));
    }
    Ok(())
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), ProjectError> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_data()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "aporic-project-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn nearest_project_is_discovered_and_isolated_by_workspace_path() {
        let root = temp_path("discover");
        let data = temp_path("data");
        let nested = root.join("src/nested");
        fs::create_dir_all(&nested).unwrap();
        initialize_project(&root, "repo").unwrap();

        let project = discover_project(&nested, &data).unwrap().unwrap();
        assert_eq!(project.workspace, fs::canonicalize(&root).unwrap());
        assert_eq!(project.scope, "repo");
        assert!(project.store_path.starts_with(data.join("workspaces")));
        assert!(project.store_path.ends_with("events.jsonl"));
    }

    #[test]
    fn unconfigured_workspace_is_skipped() {
        let root = temp_path("skip");
        fs::create_dir_all(&root).unwrap();
        assert!(
            discover_project(&root, temp_path("data"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn invalid_or_partial_configuration_fails_instead_of_skipping() {
        let root = temp_path("invalid");
        fs::create_dir_all(root.join(PROJECT_DIR)).unwrap();
        fs::write(
            root.join(PROJECT_DIR).join(PROJECT_CONFIG_FILE),
            r#"{"schema_version":1,"scope":"repo"}"#,
        )
        .unwrap();
        let error = discover_project(&root, temp_path("data")).unwrap_err();
        assert!(error.to_string().contains("policy.json is missing"));
    }

    #[cfg(unix)]
    #[test]
    fn dangling_config_symlink_is_invalid_instead_of_unbound() {
        use std::os::unix::fs::symlink;

        let root = temp_path("dangling-config");
        fs::create_dir_all(root.join(PROJECT_DIR)).unwrap();
        symlink(
            root.join("missing-config.json"),
            root.join(PROJECT_DIR).join(PROJECT_CONFIG_FILE),
        )
        .unwrap();
        let error = discover_project(&root, temp_path("data")).unwrap_err();
        assert!(error.to_string().contains("must not be a symbolic link"));
    }

    #[cfg(unix)]
    #[test]
    fn dangling_project_directory_symlink_is_invalid_instead_of_unbound() {
        use std::os::unix::fs::symlink;

        let root = temp_path("dangling-project-dir");
        fs::create_dir_all(&root).unwrap();
        symlink(root.join("missing-project-dir"), root.join(PROJECT_DIR)).unwrap();
        let error = discover_project(&root, temp_path("data")).unwrap_err();
        assert!(error.to_string().contains("must not be a symbolic link"));
    }

    #[test]
    fn store_encoding_avoids_parent_child_file_directory_collision() {
        let data = temp_path("collision-data");
        fs::create_dir_all(&data).unwrap();
        let parent = temp_path("collision-parent");
        let child = parent.join("events.jsonl");
        fs::create_dir_all(&child).unwrap();
        let parent = fs::canonicalize(parent).unwrap();
        let child = fs::canonicalize(child).unwrap();
        let parent_store = store_path(&data, &parent).unwrap();
        let child_store = store_path(&data, &child).unwrap();
        assert_ne!(parent_store, child_store);
        fs::create_dir_all(parent_store.parent().unwrap()).unwrap();
        fs::write(&parent_store, b"").unwrap();
        fs::create_dir_all(child_store.parent().unwrap()).unwrap();
        fs::write(&child_store, b"").unwrap();
    }

    #[test]
    fn data_root_inside_workspace_is_rejected() {
        let workspace = temp_path("inside-data-root");
        fs::create_dir_all(&workspace).unwrap();
        let workspace = fs::canonicalize(workspace).unwrap();
        let error = store_path(&workspace.join("data"), &workspace).unwrap_err();
        assert!(error.to_string().contains("outside the bound workspace"));
    }

    #[test]
    fn setup_refuses_to_overwrite_project_files() {
        let root = temp_path("overwrite");
        fs::create_dir_all(&root).unwrap();
        initialize_project(&root, "repo").unwrap();
        let error = initialize_project(&root, "repo").unwrap_err();
        assert!(error.to_string().contains("refusing to overwrite"));
    }
}
