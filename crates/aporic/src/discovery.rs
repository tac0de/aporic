//! Explicit, metadata-only local repository discovery. No file contents or Git subprocesses.
use std::{collections::BTreeSet, fs, path::Path, time::UNIX_EPOCH};

use crate::{
    Hub,
    domain::{RelatedWorkspaceCandidate, RelatedWorkspaceRequest},
    store::{Error, Result},
};

impl Hub {
    pub fn related_workspaces(
        &self,
        request: &RelatedWorkspaceRequest,
    ) -> Result<Vec<RelatedWorkspaceCandidate>> {
        if request.roots.is_empty() {
            return Ok(Vec::new());
        }
        if request.roots.len() > 8 || request.hints.len() > 32 {
            return Err(Error::Invalid(
                "related-workspace roots or hints exceed the limit".into(),
            ));
        }
        if request.concept.trim().is_empty() || request.concept.len() > 128 {
            return Err(Error::Invalid("concept must contain 1 to 128 bytes".into()));
        }
        let current = fs::canonicalize(&request.workspace)?;
        let mut roots = Vec::new();
        for raw in &request.roots {
            let root = match fs::canonicalize(raw) {
                Ok(root) => root,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            if !root.is_dir() {
                return Err(Error::Invalid("root must be a directory".into()));
            }
            roots.push(root);
        }
        let mut hints = Vec::new();
        for hint in &request.hints {
            if hint.labels.len() > 8 || hint.labels.iter().any(|label| label.len() > 128) {
                return Err(Error::Invalid("hint labels exceed the limit".into()));
            }
            let path = match fs::canonicalize(&hint.path) {
                Ok(path) => path,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            if !roots.iter().any(|root| path.starts_with(root)) {
                return Err(Error::Invalid(
                    "hint path is outside configured roots".into(),
                ));
            }
            hints.push((path, &hint.labels));
        }
        let concept_tokens = tokens(&request.concept);
        let mut paths = BTreeSet::new();
        for root in &roots {
            if is_repository(root) {
                paths.insert(root.clone());
            }
            let mut entries = fs::read_dir(root)?
                .take(257)
                .collect::<std::io::Result<Vec<_>>>()?;
            if entries.len() > 256 {
                return Err(Error::Invalid(
                    "root has more than 256 immediate entries".into(),
                ));
            }
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries.into_iter().take(256) {
                if entry.file_type()?.is_symlink() || !entry.file_type()?.is_dir() {
                    continue;
                }
                let path = entry.path();
                if is_repository(&path) {
                    paths.insert(path);
                }
            }
        }
        let mut candidates = Vec::new();
        for path in paths {
            if path == current {
                continue;
            }
            let mut names = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let mut hinted = false;
            for (hint_path, labels) in &hints {
                if *hint_path == path {
                    for label in *labels {
                        names.push(' ');
                        names.push_str(label);
                    }
                    hinted = true;
                }
            }
            let overlap = tokens(&names)
                .intersection(&concept_tokens)
                .cloned()
                .collect::<Vec<_>>();
            if overlap.is_empty() {
                continue;
            }
            let modified_at_unix_ms = fs::metadata(&path)
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .and_then(|duration| i64::try_from(duration.as_millis()).ok());
            candidates.push(RelatedWorkspaceCandidate {
                path: path.to_string_lossy().into_owned(),
                match_reason: format!(
                    "{} metadata token: {}",
                    if hinted {
                        "name/explicit label"
                    } else {
                        "name"
                    },
                    overlap.join(", ")
                ),
                modified_at_unix_ms,
                historical_context_only: true,
            });
        }
        candidates.sort_by(|a, b| a.path.cmp(&b.path));
        candidates.truncate(request.limit.unwrap_or(10).clamp(1, 20) as usize);
        Ok(candidates)
    }
}

fn is_repository(path: &Path) -> bool {
    let dot_git = path.join(".git");
    dot_git.is_dir() || dot_git.is_file()
}

fn tokens(text: &str) -> BTreeSet<String> {
    text.split(|ch: char| !ch.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|token| {
            token.chars().count() >= 4
                && !["repo", "project", "game", "the", "with"].contains(&token.as_str())
        })
        .collect()
}
