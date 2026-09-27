use aporic::{
    Hub,
    domain::{OpenRequest, RecordKind, RecordRequest},
};

fn doctor(database: &std::path::Path) -> serde_json::Value {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_aporic"))
        .arg("doctor")
        .env("APORIC_DATABASE", database)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn doctor_reports_core_and_extension_integrity_separately() {
    let area = tempfile::tempdir().unwrap();
    let database = area.path().join("aporic.sqlite3");
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let hub = Hub::open(&database).unwrap();
    let session = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Audit doctor health layers".to_owned(),
            idempotency_key: "doctor-open".to_owned(),
        })
        .unwrap();
    let record = hub
        .record(&RecordRequest {
            session_id: session.session_id,
            kind: RecordKind::Decision,
            content: "Keep doctor health layers distinct".to_owned(),
            evidence: None,
            supersedes_record_id: None,
            verifies_effect_id: None,
            idempotency_key: "doctor-record".to_owned(),
        })
        .unwrap();
    let clean = doctor(&database);
    assert_eq!(clean["ok"], true);
    assert_eq!(clean["core_ok"], true);
    assert_eq!(clean["extensions_ok"], true);
    assert_eq!(clean["overall_ok"], true);

    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute(
            "INSERT INTO capability_observations (
                observation_id, project_id, host_provider, tool_name, capability_class,
                first_seen_at_unix_ms, last_seen_at_unix_ms, event_count,
                succeeded_count, failed_count
             ) SELECT 'orphan-observation', project_id, 'test', 'test-tool', 'read',
                      1, 1, 1, 0, 0 FROM projects LIMIT 1",
            [],
        )
        .unwrap();
    let extension_drift = doctor(&database);
    assert_eq!(extension_drift["ok"], false);
    assert_eq!(extension_drift["core_ok"], true);
    assert_eq!(extension_drift["extensions_ok"], false);
    assert_eq!(extension_drift["overall_ok"], false);
    assert_eq!(extension_drift["runtime_projection"]["consistent"], false);

    connection
        .execute(
            "UPDATE memory_items SET lifecycle_state = 'superseded'
             WHERE memory_id = ?1",
            [format!("record:{}", record.record.record_id)],
        )
        .unwrap();
    let core_drift = doctor(&database);
    assert_eq!(core_drift["ok"], false);
    assert_eq!(core_drift["core_ok"], false);
    assert_eq!(core_drift["extensions_ok"], false);
    assert_eq!(core_drift["overall_ok"], false);
    assert_eq!(core_drift["memory_projection"]["consistent"], false);
}
