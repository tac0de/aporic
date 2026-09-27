use aporic::{
    Hub,
    domain::{OpenRequest, RecallRequest},
    recovery::{prune_backups, restore_to},
    store::Error,
};

#[test]
fn restores_a_validated_backup_into_a_new_database() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let source_database = area.path().join("source.sqlite3");
    let hub = Hub::open(&source_database).unwrap();
    hub.open_session(&OpenRequest {
        workspace: workspace.to_string_lossy().into_owned(),
        objective: "survive restore".to_owned(),
        idempotency_key: "restore-open".to_owned(),
    })
    .unwrap();
    let backup = area.path().join("aporic-backup-001.sqlite3");
    hub.backup_to(&backup).unwrap();
    let restored = area.path().join("missing-parent/restored.sqlite3");

    let outcome = restore_to(&backup, &restored).unwrap();
    assert_eq!(outcome.source_schema_version, 27);
    assert_eq!(outcome.restored_schema_version, 27);
    assert!(outcome.byte_length > 0);
    assert!(
        restore_to(&backup, &restored)
            .unwrap_err()
            .to_string()
            .contains("already exists")
    );

    let restored_hub = Hub::open(&restored).unwrap();
    let recalled = restored_hub
        .recall(&RecallRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: None,
            focus_paths: Vec::new(),
            limit: Some(20),
            max_bytes: Some(16_384),
            compact: false,
        })
        .unwrap();
    assert_eq!(recalled.active_sessions.len(), 1);
}

fn assert_destination_conflict(backup: &std::path::Path, destination: &std::path::Path) {
    let error = restore_to(backup, destination).unwrap_err();
    assert!(
        matches!(error, Error::Conflict(ref message) if message == &format!(
            "restore destination {} already exists", destination.display()
        ))
    );
}

#[test]
fn restore_preserves_existing_files_and_directories() {
    let area = tempfile::tempdir().unwrap();
    let hub = Hub::open(area.path().join("source.sqlite3")).unwrap();
    let backup = area.path().join("backup.sqlite3");
    hub.backup_to(&backup).unwrap();

    let file = area.path().join("existing.sqlite3");
    std::fs::write(&file, b"existing file").unwrap();
    assert_destination_conflict(&backup, &file);
    assert_eq!(std::fs::read(&file).unwrap(), b"existing file");

    let directory = area.path().join("existing-directory");
    std::fs::create_dir(&directory).unwrap();
    let child = directory.join("child");
    std::fs::write(&child, b"existing child").unwrap();
    assert_destination_conflict(&backup, &directory);
    assert!(directory.is_dir());
    assert_eq!(std::fs::read(&child).unwrap(), b"existing child");
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn restore_preserves_dangling_and_live_symlinks() {
    use std::os::unix::fs::symlink;

    let area = tempfile::tempdir().unwrap();
    let hub = Hub::open(area.path().join("source.sqlite3")).unwrap();
    let backup = area.path().join("backup.sqlite3");
    hub.backup_to(&backup).unwrap();

    for live in [false, true] {
        let target = area.path().join(format!("target-{live}"));
        let destination = area.path().join(format!("destination-{live}"));
        if live {
            std::fs::write(&target, b"existing target").unwrap();
        }
        symlink(&target, &destination).unwrap();
        assert!(
            std::fs::symlink_metadata(&destination)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        let original_link = std::fs::read_link(&destination).unwrap();

        assert_destination_conflict(&backup, &destination);

        assert!(
            std::fs::symlink_metadata(&destination)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(std::fs::read_link(&destination).unwrap(), original_link);
        if live {
            assert_eq!(std::fs::read(&target).unwrap(), b"existing target");
        } else {
            assert_eq!(
                std::fs::symlink_metadata(&target).unwrap_err().kind(),
                std::io::ErrorKind::NotFound
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn restore_propagates_destination_metadata_errors() {
    let area = tempfile::tempdir().unwrap();
    let hub = Hub::open(area.path().join("source.sqlite3")).unwrap();
    let backup = area.path().join("backup.sqlite3");
    hub.backup_to(&backup).unwrap();
    let parent = area.path().join("loop");
    std::os::unix::fs::symlink(&parent, &parent).unwrap();
    let destination = parent.join("restored.sqlite3");
    let expected = std::fs::symlink_metadata(&destination).unwrap_err();
    assert_ne!(expected.kind(), std::io::ErrorKind::NotFound);

    let error = restore_to(&backup, &destination).unwrap_err();
    assert!(
        matches!(error, Error::Io(ref error) if error.raw_os_error() == expected.raw_os_error())
    );
    assert_eq!(std::fs::read_link(&parent).unwrap(), parent);
}

#[test]
fn retention_only_removes_strictly_named_backup_files() {
    let area = tempfile::tempdir().unwrap();
    for name in [
        "aporic-backup-001.sqlite3",
        "aporic-backup-002.sqlite3",
        "aporic-backup-003.sqlite3",
        "unrelated.sqlite3",
    ] {
        std::fs::write(area.path().join(name), name).unwrap();
    }
    let outcome = prune_backups(area.path(), 2).unwrap();
    assert_eq!(outcome.removed.len(), 1);
    assert!(area.path().join("unrelated.sqlite3").exists());
    assert_eq!(
        std::fs::read_dir(area.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with("aporic-backup-"))
            .count(),
        2
    );
}

#[test]
fn restore_rejects_an_unrelated_sqlite_database() {
    let area = tempfile::tempdir().unwrap();
    let unrelated = area.path().join("unrelated.sqlite3");
    let connection = rusqlite::Connection::open(&unrelated).unwrap();
    connection
        .execute("CREATE TABLE unrelated (value TEXT)", [])
        .unwrap();
    drop(connection);
    let destination = area.path().join("missing-parent/restored.sqlite3");

    let error = restore_to(&unrelated, &destination).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("not a recognized Aporic database")
    );
    assert!(!destination.exists());
    assert!(!destination.parent().unwrap().exists());

    let existing = area.path().join("existing.sqlite3");
    std::fs::write(&existing, b"existing file").unwrap();
    let error = restore_to(&unrelated, &existing).unwrap_err();
    assert!(
        matches!(error, Error::Invalid(ref message) if message.contains("not a recognized Aporic database"))
    );
    assert_eq!(std::fs::read(&existing).unwrap(), b"existing file");
}
