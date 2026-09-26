#[cfg(unix)]
#[test]
fn hidden_restore_preserves_a_dangling_symlink_destination() {
    use std::os::unix::fs::symlink;

    let area = tempfile::tempdir().unwrap();
    let source_database = area.path().join("source.sqlite3");
    let hub = Hub::open(&source_database).unwrap();
    let backup = area.path().join("aporic-backup-source.sqlite3");
    hub.backup_to(&backup).unwrap();

    let destination = area.path().join("restored.sqlite3");
    let missing_target = area.path().join("absent-target.sqlite3");
    symlink(&missing_target, &destination).unwrap();
    assert!(!destination.exists());
    assert!(std::fs::symlink_metadata(&destination)
        .unwrap()
        .file_type()
        .is_symlink());

    assert!(restore_to(&backup, &destination).is_err());
    assert_eq!(std::fs::read_link(&destination).unwrap(), missing_target);
    assert!(!missing_target.exists());
    let residue = std::fs::read_dir(area.path())
        .unwrap()
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_string_lossy().starts_with(".restored.sqlite3.restore-"));
    assert!(!residue, "failed restore left a temporary database");
}
