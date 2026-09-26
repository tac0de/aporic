//! Read-only validation of a repository-owned design delivery package.
//!
//! The manifest is a pointer to work, not an approval or a source of host
//! authority. Aporic checks the bytes it can read; it does not judge aesthetics,
//! authenticate a user's choice, or publish the result.

use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{self, Read},
    path::{Component, Path, PathBuf},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::store::{Error, Result};

const MAX_MANIFEST_BYTES: u64 = 256 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 8 * 1024 * 1024;
const MAX_TOTAL_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ARTIFACTS: usize = 32;
const MAX_REFERENCES: usize = 24;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DesignValidateRequest {
    pub workspace: String,
    /// Workspace-relative path to the versioned design manifest.
    pub manifest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesignManifest {
    pub schema_version: u32,
    /// A declared task reference. Validation does not establish task ownership.
    pub task_id: String,
    pub objective: String,
    pub target_user: String,
    pub references: Vec<DesignReference>,
    pub selected_direction: Option<SelectedDirection>,
    pub artifacts: Vec<DesignArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesignReference {
    pub url: String,
    pub observed_at: String,
    pub useful_principle: String,
    pub adaptation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedDirection {
    pub concept_label: String,
    pub source: DirectionSource,
    pub rationale: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectionSource {
    ReportedUserChoice,
    ModelProposal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignArtifactKind {
    Brief,
    ReferenceNotes,
    Concept,
    Tokens,
    Components,
    Responsive,
    AssetManifest,
    Implementation,
    BrowserReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesignArtifact {
    pub kind: DesignArtifactKind,
    /// A distinct label, including a concept variant name where applicable.
    pub label: String,
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactStatus {
    Match,
    Missing,
    DigestMismatch,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactCheck {
    pub kind: DesignArtifactKind,
    pub label: String,
    pub path: String,
    pub status: ArtifactStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct DesignValidationReport {
    pub manifest_sha256: String,
    pub task_id_declared: String,
    pub reference_count: usize,
    pub selected_direction_source: Option<DirectionSource>,
    pub artifact_checks: Vec<ArtifactCheck>,
    pub missing_design_artifacts: Vec<DesignArtifactKind>,
    pub missing_delivery_artifacts: Vec<DesignArtifactKind>,
    pub all_declared_artifacts_match: bool,
    pub design_artifacts_complete: bool,
    pub delivery_artifacts_complete: bool,
    pub notice: &'static str,
}

pub fn validate(request: &DesignValidateRequest) -> Result<DesignValidationReport> {
    let workspace = fs::canonicalize(&request.workspace)?;
    if !workspace.is_dir() {
        return Err(Error::Invalid(
            "design workspace must be a directory".to_owned(),
        ));
    }
    let manifest_path = contained_path(&workspace, &request.manifest)?;
    let metadata = fs::metadata(&manifest_path)?;
    if !metadata.is_file() || metadata.len() > MAX_MANIFEST_BYTES {
        return Err(Error::Invalid(
            "design manifest must be a regular file of at most 256 KiB".to_owned(),
        ));
    }
    let bytes = fs::read(&manifest_path)?;
    let manifest: DesignManifest = serde_json::from_slice(&bytes)
        .map_err(|error| Error::Invalid(format!("invalid design manifest: {error}")))?;
    check_manifest(&manifest)?;

    let mut checks = Vec::with_capacity(manifest.artifacts.len());
    let mut matching_kinds = BTreeSet::new();
    let mut total_bytes = 0_u64;
    for artifact in &manifest.artifacts {
        let status = match contained_path_if_exists(&workspace, &artifact.path)? {
            None => ArtifactStatus::Missing,
            Some(path) => {
                let metadata = fs::metadata(&path)?;
                if !metadata.is_file() || metadata.len() > MAX_ARTIFACT_BYTES {
                    return Err(Error::Invalid(format!(
                        "design artifact {} must be a regular file of at most 8 MiB",
                        artifact.path
                    )));
                }
                total_bytes = total_bytes.saturating_add(metadata.len());
                if total_bytes > MAX_TOTAL_ARTIFACT_BYTES {
                    return Err(Error::Invalid(
                        "design artifacts exceed 64 MiB total".to_owned(),
                    ));
                }
                if file_sha256(&path)? == artifact.sha256 {
                    ArtifactStatus::Match
                } else {
                    ArtifactStatus::DigestMismatch
                }
            }
        };
        if status == ArtifactStatus::Match {
            matching_kinds.insert(artifact.kind);
        }
        checks.push(ArtifactCheck {
            kind: artifact.kind,
            label: artifact.label.clone(),
            path: artifact.path.clone(),
            status,
        });
    }

    const DESIGN: [DesignArtifactKind; 7] = [
        DesignArtifactKind::Brief,
        DesignArtifactKind::ReferenceNotes,
        DesignArtifactKind::Concept,
        DesignArtifactKind::Tokens,
        DesignArtifactKind::Components,
        DesignArtifactKind::Responsive,
        DesignArtifactKind::AssetManifest,
    ];
    const DELIVERY: [DesignArtifactKind; 2] = [
        DesignArtifactKind::Implementation,
        DesignArtifactKind::BrowserReview,
    ];
    let missing_design_artifacts = DESIGN
        .into_iter()
        .filter(|kind| !matching_kinds.contains(kind))
        .collect::<Vec<_>>();
    let missing_delivery_artifacts = DELIVERY
        .into_iter()
        .filter(|kind| !matching_kinds.contains(kind))
        .collect::<Vec<_>>();
    let concept_selected = manifest
        .selected_direction
        .as_ref()
        .is_some_and(|selection| {
            manifest.artifacts.iter().any(|artifact| {
                artifact.kind == DesignArtifactKind::Concept
                    && artifact.label == selection.concept_label
                    && matching_kinds.contains(&DesignArtifactKind::Concept)
                    && checks.iter().any(|check| {
                        check.kind == DesignArtifactKind::Concept
                            && check.label == selection.concept_label
                            && check.status == ArtifactStatus::Match
                    })
            })
        });
    let all_declared_artifacts_match = checks
        .iter()
        .all(|check| check.status == ArtifactStatus::Match);
    let design_artifacts_complete = checks
        .iter()
        .filter(|check| DESIGN.contains(&check.kind))
        .all(|check| check.status == ArtifactStatus::Match)
        && missing_design_artifacts.is_empty()
        && concept_selected
        && !manifest.references.is_empty();

    Ok(DesignValidationReport {
        manifest_sha256: format!("{:x}", Sha256::digest(&bytes)),
        task_id_declared: manifest.task_id,
        reference_count: manifest.references.len(),
        selected_direction_source: manifest
            .selected_direction
            .map(|selection| selection.source),
        artifact_checks: checks,
        missing_design_artifacts,
        missing_delivery_artifacts: missing_delivery_artifacts.clone(),
        all_declared_artifacts_match,
        design_artifacts_complete,
        delivery_artifacts_complete: design_artifacts_complete
            && all_declared_artifacts_match
            && missing_delivery_artifacts.is_empty(),
        notice: "File hashes and required categories are checked locally. References, task binding, user choice, design quality, implementation fidelity, browser findings, and publication remain unverified claims unless separately evidenced. This report grants no authority.",
    })
}

fn check_manifest(manifest: &DesignManifest) -> Result<()> {
    if manifest.schema_version != 1 {
        return Err(Error::Invalid(
            "unsupported design manifest schema_version".to_owned(),
        ));
    }
    for (name, value, limit) in [
        ("task_id", &manifest.task_id, 128),
        ("objective", &manifest.objective, 512),
        ("target_user", &manifest.target_user, 512),
    ] {
        bounded_text(name, value, limit)?;
    }
    if manifest.references.len() > MAX_REFERENCES || manifest.artifacts.len() > MAX_ARTIFACTS {
        return Err(Error::Invalid(
            "design manifest exceeds reference or artifact bounds".to_owned(),
        ));
    }
    for reference in &manifest.references {
        bounded_text("reference URL", &reference.url, 2048)?;
        if !reference.url.starts_with("https://") {
            return Err(Error::Invalid(
                "design reference URL must use https".to_owned(),
            ));
        }
        chrono::DateTime::parse_from_rfc3339(&reference.observed_at)
            .map_err(|_| Error::Invalid("reference observed_at must be RFC3339".to_owned()))?;
        bounded_text("useful_principle", &reference.useful_principle, 512)?;
        bounded_text("adaptation", &reference.adaptation, 512)?;
    }
    if let Some(selection) = &manifest.selected_direction {
        bounded_text("concept_label", &selection.concept_label, 96)?;
        bounded_text("selection rationale", &selection.rationale, 1024)?;
    }
    let mut identities = BTreeSet::new();
    for artifact in &manifest.artifacts {
        bounded_text("artifact label", &artifact.label, 96)?;
        validate_relative(&artifact.path)?;
        if !valid_sha256(&artifact.sha256) {
            return Err(Error::Invalid(format!(
                "invalid sha256 for design artifact {}",
                artifact.path
            )));
        }
        if !identities.insert((artifact.kind, artifact.label.clone(), artifact.path.clone())) {
            return Err(Error::Invalid("duplicate design artifact entry".to_owned()));
        }
    }
    Ok(())
}

fn bounded_text(name: &str, value: &str, max_bytes: usize) -> Result<()> {
    if value.trim().is_empty() || value.len() > max_bytes || value.chars().any(char::is_control) {
        return Err(Error::Invalid(format!(
            "{name} must be nonempty and at most {max_bytes} bytes"
        )));
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn validate_relative(value: &str) -> Result<()> {
    let path = Path::new(value);
    if value.is_empty()
        || value.len() > 512
        || value.contains('\\')
        || path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(Error::Invalid(format!(
            "design path must be workspace-relative: {value}"
        )));
    }
    Ok(())
}

fn contained_path(workspace: &Path, relative: &str) -> Result<PathBuf> {
    validate_relative(relative)?;
    let path = fs::canonicalize(workspace.join(relative))?;
    if !path.starts_with(workspace) {
        return Err(Error::Invalid("design path escapes workspace".to_owned()));
    }
    Ok(path)
}

fn contained_path_if_exists(workspace: &Path, relative: &str) -> Result<Option<PathBuf>> {
    validate_relative(relative)?;
    match fs::canonicalize(workspace.join(relative)) {
        Ok(path) if path.starts_with(workspace) => Ok(Some(path)),
        Ok(_) => Err(Error::Invalid("design path escapes workspace".to_owned())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn file_sha256(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let length = file.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        hasher.update(&buffer[..length]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
