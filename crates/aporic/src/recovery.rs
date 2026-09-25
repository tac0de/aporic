use std::{
    fs::{self, File, OpenOptions},
    io,
    path::{Path, PathBuf},
};

use serde::Serialize;
use uuid::Uuid;

use crate::store::{Error, Result, Store};

#[derive(Debug, Clone, Serialize)]
pub struct RestoreOutcome {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub source_schema_version: u32,
    pub restored_schema_version: u32,
    pub byte_length: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetentionOutcome {
    pub directory: PathBuf,
    pub kept: usize,
    pub removed: Vec<PathBuf>,
}

pub fn restore_to(source: &Path, destination: &Path) -> Result<RestoreOutcome> {
    let source_schema_version = Store::validate_backup(source)?;
    if destination.exists() {
        return Err(Error::Conflict(format!(
            "restore destination {} already exists",
            destination.display()
        )));
    }
    let parent = destination.parent().ok_or_else(|| {
        Error::Invalid("restore destination must have a parent directory".to_owned())
    })?;
    fs::create_dir_all(parent)?;
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::Invalid("restore destination must have a file name".to_owned()))?;
    let temporary = parent.join(format!(".{file_name}.restore-{}.tmp", Uuid::now_v7()));
    let result = restore_via_temporary(source, destination, &temporary, source_schema_version);
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn restore_via_temporary(
    source: &Path,
    destination: &Path,
    temporary: &Path,
    source_schema_version: u32,
) -> Result<RestoreOutcome> {
    let mut input = File::open(source)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary)?;
    let byte_length = io::copy(&mut input, &mut output)?;
    output.sync_all()?;
    drop(output);

    Store::validate_backup(temporary)?;
    drop(Store::open(temporary)?);
    let restored_schema_version = Store::validate_backup(temporary)?;
    fs::rename(temporary, destination)?;

    Ok(RestoreOutcome {
        source: fs::canonicalize(source)?,
        destination: fs::canonicalize(destination)?,
        source_schema_version,
        restored_schema_version,
        byte_length,
    })
}

pub fn prune_backups(directory: &Path, keep: usize) -> Result<RetentionOutcome> {
    if keep == 0 || keep > 1_000 {
        return Err(Error::Invalid(
            "backup retention keep must be between 1 and 1000".to_owned(),
        ));
    }
    let directory = fs::canonicalize(directory)?;
    if !directory.is_dir() {
        return Err(Error::Invalid(
            "backup retention path must be a directory".to_owned(),
        ));
    }
    let mut candidates = fs::read_dir(&directory)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            let metadata = fs::symlink_metadata(&path).ok()?;
            (metadata.is_file()
                && !metadata.file_type().is_symlink()
                && name.starts_with("aporic-backup-")
                && name.ends_with(".sqlite3"))
            .then_some((metadata.modified().ok(), path))
        })
        .collect::<Vec<_>>();
    candidates.sort();
    let remove_count = candidates.len().saturating_sub(keep);
    let mut removed = Vec::with_capacity(remove_count);
    for (_, path) in candidates.into_iter().take(remove_count) {
        fs::remove_file(&path)?;
        removed.push(path);
    }
    Ok(RetentionOutcome {
        directory,
        kept: keep,
        removed,
    })
}
