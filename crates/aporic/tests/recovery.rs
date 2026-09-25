use aporic::{
    Hub,
    domain::{OpenRequest, RecallRequest},
    recovery::{prune_backups, restore_to},
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
    let restored = area.path().join("restored.sqlite3");

    let outcome = restore_to(&backup, &restored).unwrap();
    assert_eq!(outcome.source_schema_version, 12);
    assert_eq!(outcome.restored_schema_version, 12);
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
        })
        .unwrap();
    assert_eq!(recalled.active_sessions.len(), 1);
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
    let destination = area.path().join("restored.sqlite3");

    let error = restore_to(&unrelated, &destination).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("not a recognized Aporic database")
    );
    assert!(!destination.exists());
}
