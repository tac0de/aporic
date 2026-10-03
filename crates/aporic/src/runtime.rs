//! Build and checkout identity for diagnosing stale local releases.
//!
//! These values are observations. They identify bytes and reported Git state,
//! but do not attest authority, permissions, or release provenance.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeIdentity {
    pub package_version: String,
    pub source_sha256: String,
    pub git: GitProvenance,
    pub compiler: CompilerIdentity,
    pub database_schema_version: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GitProvenance {
    pub head: Option<String>,
    pub dirty: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompilerIdentity {
    pub rustc_version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CheckoutIdentity {
    pub checkout: PathBuf,
    pub source_sha256: String,
    pub git: GitProvenance,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReleaseValidation {
    pub runtime: RuntimeIdentity,
    pub checkout: CheckoutIdentity,
    pub source_matches: bool,
    pub package_version_matches: bool,
    pub git_head_matches: Option<bool>,
}

pub fn current() -> RuntimeIdentity {
    RuntimeIdentity {
        package_version: env!("CARGO_PKG_VERSION").to_owned(),
        source_sha256: env!("APORIC_SOURCE_SHA256").to_owned(),
        git: GitProvenance {
            head: known(env!("APORIC_GIT_HEAD")),
            dirty: match env!("APORIC_GIT_DIRTY") {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            },
        },
        compiler: CompilerIdentity {
            rustc_version: env!("APORIC_RUSTC_VERSION").to_owned(),
        },
        database_schema_version: env!("APORIC_SCHEMA_VERSION").parse().ok(),
    }
}

pub fn inspect_checkout(checkout: impl AsRef<Path>) -> std::io::Result<CheckoutIdentity> {
    let checkout = checkout.as_ref().canonicalize()?;
    let manifest_dir = package_directory(&checkout)?;
    Ok(CheckoutIdentity {
        source_sha256: source_digest(&manifest_dir, &checkout)?,
        git: GitProvenance {
            head: git(&checkout, &["rev-parse", "HEAD"]),
            dirty: git(&checkout, &["status", "--porcelain"]).map(|status| !status.is_empty()),
        },
        checkout,
    })
}

pub fn validate_release(checkout: impl AsRef<Path>) -> std::io::Result<ReleaseValidation> {
    let runtime = current();
    let checkout = inspect_checkout(checkout)?;
    let package_version_matches =
        fs::read_to_string(package_directory(&checkout.checkout)?.join("Cargo.toml"))?
            .lines()
            .any(|line| line.trim() == format!("version = \"{}\"", runtime.package_version));
    let git_head_matches = match (&runtime.git.head, &checkout.git.head) {
        (Some(runtime), Some(checkout)) => Some(runtime == checkout),
        _ => None,
    };
    Ok(ReleaseValidation {
        source_matches: runtime.source_sha256 == checkout.source_sha256,
        package_version_matches,
        git_head_matches,
        runtime,
        checkout,
    })
}

pub fn source_digest(
    package_directory: impl AsRef<Path>,
    workspace: impl AsRef<Path>,
) -> std::io::Result<String> {
    let package_directory = package_directory.as_ref();
    let workspace = workspace.as_ref();
    let mut files = Vec::new();
    add_file(
        &mut files,
        package_directory.join("Cargo.toml"),
        "package/Cargo.toml",
    );
    add_file(
        &mut files,
        package_directory.join("build.rs"),
        "package/build.rs",
    );
    add_tree(&mut files, &package_directory.join("src"), "package/src")?;
    add_tree(
        &mut files,
        &workspace.join("migrations"),
        "workspace/migrations",
    )?;
    add_file(
        &mut files,
        workspace.join("Cargo.lock"),
        "workspace/Cargo.lock",
    );
    add_file(
        &mut files,
        workspace.join("Cargo.toml"),
        "workspace/Cargo.toml",
    );
    add_file(
        &mut files,
        workspace.join("canon/KERNEL.md"),
        "workspace/canon/KERNEL.md",
    );
    files.sort_by(|left, right| left.0.cmp(&right.0));

    let mut digest = Sha256::new();
    for (label, path) in files {
        digest.update(label.as_bytes());
        digest.update([0]);
        digest.update(fs::read(path)?);
        digest.update([0]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn package_directory(checkout: &Path) -> std::io::Result<PathBuf> {
    let direct = checkout.join("Cargo.toml");
    if direct.is_file() && checkout.join("src").is_dir() {
        return Ok(checkout.to_owned());
    }
    let package = checkout.join("crates/aporic");
    if package.join("Cargo.toml").is_file() && package.join("src").is_dir() {
        return Ok(package);
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "could not find the aporic package in checkout",
    ))
}

fn add_file(files: &mut Vec<(String, PathBuf)>, path: PathBuf, label: &str) {
    if path.is_file() {
        files.push((label.to_owned(), path));
    }
}

fn add_tree(
    files: &mut Vec<(String, PathBuf)>,
    directory: &Path,
    label: &str,
) -> std::io::Result<()> {
    if !directory.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let entry_label = format!("{label}/{}", entry.file_name().to_string_lossy());
        if path.is_dir() {
            add_tree(files, &path, &entry_label)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension == "rs" || extension == "sql")
        {
            files.push((entry_label, path));
        }
    }
    Ok(())
}

fn git(workspace: &Path, arguments: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace)
        .args(arguments)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn known(value: &str) -> Option<String> {
    (value != "unknown").then(|| value.to_owned())
}
