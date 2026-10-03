use std::{fs, path::Path};

use aporic::{
    Hub,
    delivery::{self, DeliveryManifest, DeliveryValidateRequest},
    delivery_receipts::{self, DeliveryRegisterRequest, DeliveryResult, DeliveryVerifyRequest},
    domain::{CommandSpecRequest, ExecutionSandboxProfile, OpenRequest},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn write_manifest(root: &Path, manifest: &DeliveryManifest) {
    fs::write(
        root.join("manifest.json"),
        serde_json::to_vec_pretty(manifest).unwrap(),
    )
    .unwrap();
}

fn copy_command() -> Vec<String> {
    #[cfg(unix)]
    {
        vec![
            "/bin/cp".into(),
            "template.json".into(),
            "result.json".into(),
        ]
    }
    #[cfg(windows)]
    {
        vec![
            std::env::var("COMSPEC").unwrap(),
            "/D".into(),
            "/C".into(),
            "copy /Y template.json result.json".into(),
        ]
    }
}

fn package(root: &Path, argv: &[String]) -> DeliveryManifest {
    let mut nodes = vec![
        json!({"id":"requirement","kind":"requirement","description":"Restore saved state","depends_on":[]}),
        json!({"id":"scenario","kind":"scenario","description":"Reload restores","depends_on":["requirement"],"scenario":{"requirements":["requirement"],"preconditions":["A saved state exists"],"actions":["Reload"],"expected":["State restored"],"failure_recovery":["Invalid state is reported"]}}),
        json!({"id":"design","kind":"design","description":"State contract","depends_on":["scenario"]}),
        json!({"id":"implementation","kind":"implementation","description":"Save implementation","depends_on":["design","source"]}),
        json!({"id":"test","kind":"test","description":"Receipt-integrity fixture; semantic scenario adequacy is not asserted","depends_on":["implementation","dependency","build","environment"]}),
    ];
    for kind in ["source", "dependency", "build", "environment"] {
        let path = format!("{kind}.txt");
        let bytes = format!("version-one-{kind}");
        fs::write(root.join(&path), &bytes).unwrap();
        nodes.push(json!({"id":kind,"kind":kind,"description":format!("{kind} input"),"depends_on":[],"file":{"path":path,"sha256":digest(bytes.as_bytes())}}));
    }
    let categories: Vec<_> = aporic::delivery_profiles::definitions("general", 1)
        .unwrap()
        .into_iter()
        .map(|c| c.id)
        .collect();
    let mut manifest:DeliveryManifest = serde_json::from_value(json!({
        "schema_version":1,"project_id":"receipt-fixture","revision":"v1",
        "execution":{"stack":["Rust"],"architecture":["Local fixture"],"commands":{"test":argv},"supported_environments":["Local test host"],"config_refs":[],"constraints":["No release"],"quality_targets":["Reject stale proofs"]},
        "profiles":[{"id":"general","version":1}],"nodes":nodes,
        "checks":[{"id":"check","command_id":"test","scenario_ids":["scenario"],"test_id":"test","categories":categories,"input_sha256":{},"result_path":"result.json"}]
    })).unwrap();
    manifest.checks[0].input_sha256 =
        delivery::check_input_fingerprints(&manifest, &manifest.checks[0]).unwrap();
    write_manifest(root, &manifest);
    manifest
}

fn template(root: &Path, manifest: &DeliveryManifest, passed: bool) {
    // Copy is deliberately only an execution/byte-provenance check. The copied
    // assertion report is not a claim that a real game was tested.
    let output = DeliveryResult {
        check_id: "check".into(),
        input_sha256: manifest.checks[0].input_sha256.clone(),
        execution_sha256: delivery::execution_fingerprint(&manifest.execution).unwrap(),
        passed,
    };
    fs::write(
        root.join("template.json"),
        serde_json::to_vec(&output).unwrap(),
    )
    .unwrap();
}
fn session(hub: &Hub, root: &Path, key: &str) -> String {
    hub.open_session(&OpenRequest {
        workspace: root.to_string_lossy().into_owned(),
        objective: "Delivery receipt fixture".into(),
        idempotency_key: key.into(),
    })
    .unwrap()
    .session_id
}
fn spec(hub: &Hub, session_id: &str, argv: &[String], key: &str) -> String {
    hub.register_command_spec(&CommandSpecRequest {
        session_id: session_id.into(),
        program: argv[0].clone(),
        args: argv[1..].to_vec(),
        workspace_relative_cwd: ".".into(),
        expected_exit_code: 0,
        timeout_seconds: 10,
        artifact_paths: vec!["result.json".into()],
        sandbox_profile: ExecutionSandboxProfile::default(),
        idempotency_key: key.into(),
    })
    .unwrap()
    .spec
    .spec_id
}
fn request(root: &Path, spec_id: &str) -> DeliveryVerifyRequest {
    DeliveryVerifyRequest {
        workspace: root.to_string_lossy().into_owned(),
        manifest: "manifest.json".into(),
        check_id: "check".into(),
        spec_id: spec_id.into(),
    }
}
fn validate_request(root: &Path) -> DeliveryValidateRequest {
    DeliveryValidateRequest {
        workspace: root.to_string_lossy().into_owned(),
        manifest: "manifest.json".into(),
    }
}

#[tokio::test]
async fn binds_real_runner_inputs_and_persists_across_restart_and_export() {
    let area = tempfile::tempdir().unwrap();
    let root = area.path().join("workspace");
    fs::create_dir(&root).unwrap();
    let db = area.path().join("db.sqlite3");
    let hub = Hub::open(&db).unwrap();
    let argv = copy_command();
    let mut manifest = package(&root, &argv);
    template(&root, &manifest, true);
    let sid = session(&hub, &root, "open");
    let spec_id = spec(&hub, &sid, &argv, "spec");
    let registration = DeliveryRegisterRequest {
        session_id: sid.clone(),
        manifest: "manifest.json".into(),
        idempotency_key: "register".into(),
    };
    let evidence = delivery_receipts::register(&hub, &registration).unwrap();
    assert!(
        delivery_receipts::register(&hub, &registration)
            .unwrap()
            .duplicate
    );
    let before = delivery_receipts::validate(&hub, &validate_request(&root)).unwrap();
    assert!(!before.valid);
    assert!(!before.checks[0].execution_verified);
    let out = delivery_receipts::verify(&hub, &request(&root, &spec_id))
        .await
        .unwrap();
    assert!(out.bound, "{:?}", out.gaps);
    manifest.checks[0].run_id = Some(out.execution.run.run_id);
    write_manifest(&root, &manifest);
    let current = delivery_receipts::validate(&hub, &validate_request(&root)).unwrap();
    assert!(current.valid, "{:?}", current.gaps);
    assert!(current.checks[0].execution_verified);
    assert!(current.assurance_boundary.contains("semantic"));
    drop(hub);
    let hub = Hub::open(&db).unwrap();
    assert!(
        delivery_receipts::validate(&hub, &validate_request(&root))
            .unwrap()
            .valid
    );
    let impact = delivery_receipts::impact(
        &hub,
        &delivery::DeliveryImpactRequest {
            workspace: root.to_string_lossy().into_owned(),
            previous: "manifest.json".into(),
            current: "manifest.json".into(),
        },
    )
    .unwrap();
    assert_eq!(impact.preserved_check_ids, vec!["check"]);
    let export = hub.export_project(&root.to_string_lossy()).unwrap();
    assert!(
        export
            .events
            .iter()
            .any(|event| event.kind == "delivery_run_bound")
    );
    assert!(
        export
            .evidence
            .iter()
            .any(|item| item.evidence_id == evidence.evidence.evidence_id)
    );
    assert!(hub.audit_execution_replay().unwrap().mismatches.is_empty());
    // Replacing recorded output with another passed report is still stale.
    fs::write(root.join("result.json"), b"{}").unwrap();
    assert!(
        !delivery_receipts::validate(&hub, &validate_request(&root))
            .unwrap()
            .checks[0]
            .execution_verified
    );
    assert!(
        delivery_receipts::impact(
            &hub,
            &delivery::DeliveryImpactRequest {
                workspace: root.to_string_lossy().into_owned(),
                previous: "manifest.json".into(),
                current: "manifest.json".into()
            }
        )
        .unwrap()
        .preserved_check_ids
        .is_empty()
    );
    fs::copy(root.join("template.json"), root.join("result.json")).unwrap();
    // Git status can stay the same while bytes change; direct input hashes catch it.
    fs::write(root.join("source.txt"), b"version-two-source").unwrap();
    assert!(
        !delivery_receipts::validate(&hub, &validate_request(&root))
            .unwrap()
            .checks[0]
            .execution_verified
    );
}

#[tokio::test]
async fn rejects_reported_unbound_foreign_failed_and_preexisting_results() {
    let area = tempfile::tempdir().unwrap();
    let root = area.path().join("workspace");
    fs::create_dir(&root).unwrap();
    let hub = Hub::open(area.path().join("db.sqlite3")).unwrap();
    let argv = copy_command();
    let mut manifest = package(&root, &argv);
    template(&root, &manifest, true);
    let sid = session(&hub, &root, "open");
    let spec_id = spec(&hub, &sid, &argv, "spec");
    // Existing ordinary local runner receipt has no pre-execution delivery input observations.
    let ordinary = hub.verify(&spec_id).await.unwrap();
    manifest.checks[0].run_id = Some(ordinary.run.run_id.clone());
    write_manifest(&root, &manifest);
    let validation = delivery_receipts::validate(&hub, &validate_request(&root)).unwrap();
    assert!(!validation.checks[0].execution_verified);
    assert!(
        validation
            .gaps
            .iter()
            .any(|g| g.message.contains("input binding"))
    );
    assert!(
        delivery_receipts::verify(&hub, &request(&root, &spec_id))
            .await
            .unwrap_err()
            .to_string()
            .contains("already exists")
    );
    fs::remove_file(root.join("result.json")).unwrap();
    template(&root, &manifest, false);
    let failed = delivery_receipts::verify(&hub, &request(&root, &spec_id))
        .await
        .unwrap();
    assert!(!failed.bound);
    fs::remove_file(root.join("result.json")).unwrap();
    template(&root, &manifest, true);
    let foreign = area.path().join("foreign");
    fs::create_dir(&foreign).unwrap();
    let other_sid = session(&hub, &foreign, "other-open");
    let other_spec = spec(&hub, &other_sid, &argv, "other-spec");
    assert!(
        delivery_receipts::verify(&hub, &request(&root, &other_spec))
            .await
            .unwrap_err()
            .to_string()
            .contains("workspace")
    );
    let mut wrong_argv = argv.clone();
    wrong_argv.push("unexpected-argument".into());
    let wrong_spec = spec(&hub, &sid, &wrong_argv, "wrong-argv");
    assert!(
        delivery_receipts::verify(&hub, &request(&root, &wrong_spec))
            .await
            .unwrap_err()
            .to_string()
            .contains("exact argv")
    );
    fs::create_dir(root.join("subdir")).unwrap();
    let wrong_cwd = hub
        .register_command_spec(&CommandSpecRequest {
            session_id: sid.clone(),
            program: argv[0].clone(),
            args: argv[1..].to_vec(),
            workspace_relative_cwd: "subdir".into(),
            expected_exit_code: 0,
            timeout_seconds: 10,
            artifact_paths: vec!["result.json".into()],
            sandbox_profile: ExecutionSandboxProfile::default(),
            idempotency_key: "wrong-cwd".into(),
        })
        .unwrap()
        .spec
        .spec_id;
    assert!(
        delivery_receipts::verify(&hub, &request(&root, &wrong_cwd))
            .await
            .unwrap_err()
            .to_string()
            .contains("cwd")
    );
    manifest.checks[0].run_id = Some("invented-run".into());
    write_manifest(&root, &manifest);
    assert!(
        !delivery_receipts::validate(&hub, &validate_request(&root))
            .unwrap()
            .valid
    );
}

#[tokio::test]
async fn failed_process_cannot_bind_a_check() {
    let area = tempfile::tempdir().unwrap();
    let root = area.path().join("workspace");
    fs::create_dir(&root).unwrap();
    let hub = Hub::open(area.path().join("db.sqlite3")).unwrap();
    #[cfg(unix)]
    let argv = vec!["/bin/sh".into(), "-c".into(), "exit 1".into()];
    #[cfg(windows)]
    let argv = vec![
        std::env::var("COMSPEC").unwrap(),
        "/D".into(),
        "/C".into(),
        "exit 1".into(),
    ];
    let manifest = package(&root, &argv);
    template(&root, &manifest, true);
    let sid = session(&hub, &root, "open");
    let spec_id = spec(&hub, &sid, &argv, "spec");
    let out = delivery_receipts::verify(&hub, &request(&root, &spec_id))
        .await
        .unwrap();
    assert!(!out.bound);
    assert_eq!(
        out.execution.run.status,
        aporic::domain::ExecutionStatus::Failed
    );
}

#[cfg(unix)]
#[tokio::test]
async fn replaced_result_and_input_fifos_are_rejected_without_blocking() {
    let area = tempfile::tempdir().unwrap();
    let root = area.path().join("workspace");
    fs::create_dir(&root).unwrap();
    let hub = Hub::open(area.path().join("db.sqlite3")).unwrap();
    let argv = copy_command();
    let mut manifest = package(&root, &argv);
    template(&root, &manifest, true);
    let sid = session(&hub, &root, "open");
    let spec_id = spec(&hub, &sid, &argv, "spec");
    let out = delivery_receipts::verify(&hub, &request(&root, &spec_id))
        .await
        .unwrap();
    assert!(out.bound);
    manifest.checks[0].run_id = Some(out.execution.run.run_id);
    write_manifest(&root, &manifest);
    for relative in ["result.json", "source.txt"] {
        let path = root.join(relative);
        fs::remove_file(&path).unwrap();
        let cpath = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(cpath.as_ptr(), 0o600) }, 0);
        let report = delivery_receipts::validate(&hub, &validate_request(&root)).unwrap();
        assert!(!report.checks[0].execution_verified);
        fs::remove_file(&path).unwrap();
        if relative == "result.json" {
            fs::copy(root.join("template.json"), path).unwrap();
        }
    }
}

#[cfg(unix)]
#[tokio::test]
async fn refuses_binding_when_a_command_changes_an_input() {
    let area = tempfile::tempdir().unwrap();
    let root = area.path().join("workspace");
    fs::create_dir(&root).unwrap();
    let hub = Hub::open(area.path().join("db.sqlite3")).unwrap();
    let argv = vec![
        "/bin/sh".into(),
        "-c".into(),
        "cp template.json result.json; printf changed > source.txt".into(),
    ];
    let manifest = package(&root, &argv);
    template(&root, &manifest, true);
    let sid = session(&hub, &root, "open");
    let spec_id = spec(&hub, &sid, &argv, "spec");
    let out = delivery_receipts::verify(&hub, &request(&root, &spec_id))
        .await
        .unwrap();
    assert!(!out.bound);
    assert!(
        out.gaps
            .iter()
            .any(|g| g.contains("stale") || g.contains("changed"))
    );
}

#[test]
fn cli_inventory_exits_unsuccessfully_for_unbound_fixture() {
    let area = tempfile::tempdir().unwrap();
    package(area.path(), &copy_command());
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_aporic"))
        .args([
            "delivery",
            "validate",
            "--workspace",
            area.path().to_str().unwrap(),
            "--manifest",
            "manifest.json",
        ])
        .env("APORIC_DATABASE", area.path().join("db.sqlite3"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["valid"], false);
    let brief = std::process::Command::new(env!("CARGO_BIN_EXE_aporic"))
        .args([
            "delivery",
            "brief",
            "--workspace",
            area.path().to_str().unwrap(),
            "--manifest",
            "manifest.json",
            "--focus",
            "test",
        ])
        .output()
        .unwrap();
    assert!(brief.status.success());
    let report: Value = serde_json::from_slice(&brief.stdout).unwrap();
    assert!(report["text"].as_str().unwrap().contains("commands"));
}

#[test]
fn cli_delivery_verification_produces_a_binding_that_cli_validation_recovers() {
    let area = tempfile::tempdir().unwrap();
    let root = area.path().join("workspace");
    fs::create_dir(&root).unwrap();
    let database = area.path().join("db.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let argv = copy_command();
    let mut manifest = package(&root, &argv);
    template(&root, &manifest, true);
    let sid = session(&hub, &root, "open");
    let spec_id = spec(&hub, &sid, &argv, "spec");
    let execution = std::process::Command::new(env!("CARGO_BIN_EXE_aporic"))
        .args([
            "delivery",
            "verify",
            "--workspace",
            root.to_str().unwrap(),
            "--manifest",
            "manifest.json",
            "--check",
            "check",
            "--spec",
            &spec_id,
        ])
        .env("APORIC_DATABASE", &database)
        .output()
        .unwrap();
    assert!(
        execution.status.success(),
        "{}",
        String::from_utf8_lossy(&execution.stderr)
    );
    let outcome: Value = serde_json::from_slice(&execution.stdout).unwrap();
    assert_eq!(outcome["bound"], true);
    manifest.checks[0].run_id = Some(
        outcome["execution"]["run"]["run_id"]
            .as_str()
            .unwrap()
            .into(),
    );
    write_manifest(&root, &manifest);
    let validation = std::process::Command::new(env!("CARGO_BIN_EXE_aporic"))
        .args([
            "delivery",
            "validate",
            "--workspace",
            root.to_str().unwrap(),
            "--manifest",
            "manifest.json",
        ])
        .env("APORIC_DATABASE", &database)
        .output()
        .unwrap();
    assert!(
        validation.status.success(),
        "{}",
        String::from_utf8_lossy(&validation.stderr)
    );
    let report: Value = serde_json::from_slice(&validation.stdout).unwrap();
    assert_eq!(report["checks"][0]["execution_verified"], true);
}
