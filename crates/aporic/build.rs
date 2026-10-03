use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use sha2::{Digest, Sha256};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let workspace = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");

    for watched in [
        manifest_dir.join("src"),
        workspace.join("migrations"),
        workspace.join("canon"),
    ] {
        println!("cargo:rerun-if-changed={}", watched.display());
    }
    for watched in git_watch_paths(workspace) {
        println!("cargo:rerun-if-changed={}", watched.display());
    }

    let source_sha256 = source_digest(&manifest_dir, workspace);
    println!("cargo:rustc-env=APORIC_SOURCE_SHA256={source_sha256}");
    println!(
        "cargo:rustc-env=APORIC_GIT_HEAD={}",
        git(workspace, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_owned())
    );
    println!(
        "cargo:rustc-env=APORIC_GIT_DIRTY={}",
        git(workspace, &["status", "--porcelain"])
            .map(|status| (!status.is_empty()).to_string())
            .unwrap_or_else(|| "unknown".to_owned())
    );
    println!(
        "cargo:rustc-env=APORIC_RUSTC_VERSION={}",
        rustc_version().unwrap_or_else(|| "unknown".to_owned())
    );
    println!(
        "cargo:rustc-env=APORIC_SCHEMA_VERSION={}",
        schema_version(&manifest_dir).unwrap_or_else(|| "unknown".to_owned())
    );
}

fn source_digest(manifest_dir: &Path, workspace: &Path) -> String {
    let mut files = Vec::new();
    add_file(
        &mut files,
        manifest_dir.join("Cargo.toml"),
        "package/Cargo.toml",
    );
    add_file(
        &mut files,
        manifest_dir.join("build.rs"),
        "package/build.rs",
    );
    add_tree(&mut files, &manifest_dir.join("src"), "package/src");
    add_tree(
        &mut files,
        &workspace.join("migrations"),
        "workspace/migrations",
    );
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
        println!("cargo:rerun-if-changed={}", path.display());
        digest.update(label.as_bytes());
        digest.update([0]);
        digest.update(fs::read(path).expect("read compile input"));
        digest.update([0]);
    }
    format!("{:x}", digest.finalize())
}

fn add_file(files: &mut Vec<(String, PathBuf)>, path: PathBuf, label: &str) {
    if path.is_file() {
        files.push((label.to_owned(), path));
    }
}

fn add_tree(files: &mut Vec<(String, PathBuf)>, directory: &Path, label: &str) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let entry_label = format!("{label}/{}", entry.file_name().to_string_lossy());
        if path.is_dir() {
            add_tree(files, &path, &entry_label);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "rs" || extension == "sql")
        {
            files.push((entry_label, path));
        }
    }
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

fn rustc_version() -> Option<String> {
    let rustc = env::var("RUSTC").ok()?;
    let output = Command::new(rustc).arg("--version").output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn schema_version(manifest_dir: &Path) -> Option<String> {
    let store = fs::read_to_string(manifest_dir.join("src/store.rs")).ok()?;
    let marker = "const SCHEMA_VERSION: u32 = ";
    store
        .lines()
        .find_map(|line| line.trim().strip_prefix(marker))
        .and_then(|value| value.strip_suffix(';'))
        .map(str::to_owned)
}

fn git_watch_paths(workspace: &Path) -> Vec<PathBuf> {
    ["HEAD", "index", "packed-refs"]
        .into_iter()
        .filter_map(|name| {
            let output = Command::new("git")
                .arg("-C")
                .arg(workspace)
                .args(["rev-parse", "--path-format=absolute", "--git-path", name])
                .output()
                .ok()?;
            output
                .status
                .success()
                .then(|| PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()))
        })
        .collect()
}
