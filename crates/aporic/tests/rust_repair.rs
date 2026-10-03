use aporic::{
    Hub,
    domain::{
        CommandSpecRequest, CriterionProof, EvidenceKind, EvidenceRequest, OpenRequest,
        RustRepairLearnRequest, RustRepairOpenRequest, RustRepairSearchRequest, TaskClaimRequest,
        TaskCompleteRequest, TaskCreateRequest,
    },
};
use sha2::{Digest, Sha256};

#[test]
fn schema_29_upgrades_to_rust_repair_schema() {
    let area = tempfile::tempdir().unwrap();
    let database = area.path().join("aporic.sqlite3");
    drop(Hub::open(&database).unwrap());
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection.execute_batch("DROP TABLE rustc_input_snapshots; DROP TABLE rust_repair_lessons; DROP TABLE rust_repair_cases; PRAGMA user_version = 29;").unwrap();
    drop(connection);
    let hub = Hub::open(&database).unwrap();
    assert_eq!(hub.stats().unwrap().schema_version, 30);
    drop(hub);
    assert_eq!(
        Hub::open(&database)
            .unwrap()
            .stats()
            .unwrap()
            .schema_version,
        30
    );
}

#[tokio::test]
async fn unavailable_rustc_snapshot_does_not_block_verification() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("fixture");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::write(
        workspace.join("large.rs"),
        format!("{}pub fn bad() -> i32 {{ \"bad\" }}", " ".repeat(1_000_001)),
    )
    .unwrap();
    let sysroot = String::from_utf8(
        std::process::Command::new("rustc")
            .args(["--print", "sysroot"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let rustc = std::path::Path::new(sysroot.trim())
        .join("bin")
        .join(if cfg!(windows) { "rustc.exe" } else { "rustc" })
        .to_string_lossy()
        .into_owned();
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Verify without a snapshot".into(),
            idempotency_key: "open".into(),
        })
        .unwrap()
        .session_id;
    for source in ["large.rs", "missing.rs"] {
        let spec = hub
            .register_command_spec(&CommandSpecRequest {
                session_id: session_id.clone(),
                program: rustc.clone(),
                args: vec![
                    "--edition=2021".into(),
                    "--crate-type=lib".into(),
                    "--error-format=json".into(),
                    source.into(),
                ],
                workspace_relative_cwd: ".".into(),
                expected_exit_code: 1,
                timeout_seconds: 30,
                artifact_paths: vec![],
                sandbox_profile: Default::default(),
                idempotency_key: source.into(),
            })
            .unwrap()
            .spec;
        let outcome = hub.verify(&spec.spec_id).await.unwrap();
        assert_eq!(outcome.receipt.unwrap().exit_code, Some(1));
    }
    assert!(
        hub.export_project(&workspace.to_string_lossy())
            .unwrap()
            .rustc_input_snapshots
            .is_empty()
    );
}

#[tokio::test]
async fn rust_repair_lesson_requires_the_completed_tasks_cargo_test_receipt() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("fixture");
    std::fs::create_dir_all(workspace.join("src")).unwrap();
    std::fs::write(
        workspace.join("Cargo.toml"),
        "[package]\nname = \"repair_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    let source = workspace.join("src/lib.rs");
    std::fs::write(&source, "pub fn answer() -> i32 { \"wrong\" }\n").unwrap();
    let repro_path = workspace.join("repro.rs");
    std::fs::copy(&source, &repro_path).unwrap();
    let sysroot = String::from_utf8(
        std::process::Command::new("rustc")
            .args(["--print", "sysroot"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let rustc = std::path::Path::new(sysroot.trim())
        .join("bin")
        .join(if cfg!(windows) { "rustc.exe" } else { "rustc" })
        .to_string_lossy()
        .into_owned();
    // The fixture lives outside the repository's toolchain override. Keep
    // Cargo's Rustup-shim compiler and rustdoc on the same installed toolchain
    // as the absolute Cargo executable used by the verification runner.
    let toolchain = std::path::Path::new(sysroot.trim())
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
    std::fs::write(
        workspace.join("rust-toolchain.toml"),
        format!("[toolchain]\nchannel = {toolchain:?}\n"),
    )
    .unwrap();
    let diagnostic = std::process::Command::new(&rustc)
        .current_dir(&workspace)
        .args([
            "--edition=2021",
            "--crate-type=lib",
            "--error-format=json",
            "repro.rs",
        ])
        .output()
        .unwrap();
    assert!(!diagnostic.status.success());
    assert!(String::from_utf8_lossy(&diagnostic.stderr).contains("E0308"));
    let diagnostic_path = workspace.join("diagnostic.jsonl");
    std::fs::write(&diagnostic_path, diagnostic.stderr).unwrap();
    let version = String::from_utf8(
        std::process::Command::new(&rustc)
            .arg("--version")
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_owned();

    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let workspace_text = workspace.to_string_lossy().into_owned();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace_text.clone(),
            objective: "Repair E0308".into(),
            idempotency_key: "open".into(),
        })
        .unwrap()
        .session_id;
    let evidence = |path: &std::path::Path, key: &str| {
        hub.add_evidence(&EvidenceRequest {
            session_id: session_id.clone(),
            kind: EvidenceKind::WorkspaceFile,
            locator: path.to_string_lossy().into_owned(),
            summary: key.into(),
            content_sha256: None,
            idempotency_key: key.into(),
        })
        .unwrap()
        .evidence
        .evidence_id
    };
    let repro_id = evidence(&repro_path, "repro");
    let repro_spec = hub
        .register_command_spec(&CommandSpecRequest {
            session_id: session_id.clone(),
            program: rustc,
            args: vec![
                "--edition=2021".into(),
                "--crate-type=lib".into(),
                "--error-format=json".into(),
                "repro.rs".into(),
            ],
            workspace_relative_cwd: ".".into(),
            expected_exit_code: 1,
            timeout_seconds: 30,
            artifact_paths: vec![],
            sandbox_profile: Default::default(),
            idempotency_key: "repro-spec".into(),
        })
        .unwrap()
        .spec;
    let repro_run = hub.verify(&repro_spec.spec_id).await.unwrap();
    assert_eq!(repro_run.receipt.as_ref().unwrap().exit_code, Some(1));
    assert_eq!(
        repro_run.receipt.as_ref().unwrap().stderr_sha256,
        format!(
            "{:x}",
            Sha256::digest(std::fs::read(&diagnostic_path).unwrap())
        )
    );
    let diagnostic_id = evidence(&diagnostic_path, "diagnostic");
    let cargo = std::path::Path::new(sysroot.trim())
        .join("bin")
        .join(if cfg!(windows) { "cargo.exe" } else { "cargo" })
        .to_string_lossy()
        .into_owned();
    let spec = hub
        .register_command_spec(&CommandSpecRequest {
            session_id: session_id.clone(),
            program: cargo,
            args: vec!["test".into(), "--offline".into()],
            workspace_relative_cwd: ".".into(),
            expected_exit_code: 0,
            timeout_seconds: 60,
            artifact_paths: vec![],
            sandbox_profile: Default::default(),
            idempotency_key: "test-spec".into(),
        })
        .unwrap()
        .spec;
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id: session_id.clone(),
            objective: "Fix E0308".into(),
            acceptance_criteria: vec![spec.success_claim.clone()],
            write_scope: vec!["src".into()],
            depends_on: vec![],
            idempotency_key: "task".into(),
        })
        .unwrap()
        .task
        .task_id;
    let open_request = RustRepairOpenRequest {
        session_id: session_id.clone(),
        task_id: task_id.clone(),
        diagnostic_code: "E0308".into(),
        rustc_version: version.clone(),
        edition: "2021".into(),
        diagnostic_evidence_id: diagnostic_id,
        reproduction_evidence_id: repro_id,
        reproduction_run_id: repro_run.run.run_id,
        reference_evidence_id: None,
        idempotency_key: "rust-open".into(),
    };
    let original_repro = std::fs::read(&repro_path).unwrap();
    std::fs::write(&repro_path, "changed after evidence capture").unwrap();
    assert!(
        hub.open_rust_repair(&RustRepairOpenRequest {
            idempotency_key: "stale-repro".into(),
            ..open_request.clone()
        })
        .is_err()
    );
    std::fs::write(&repro_path, original_repro).unwrap();
    let safe_path = workspace.join("safe.rs");
    std::fs::write(&safe_path, "pub fn answer() -> i32 { 42 }\n").unwrap();
    let safe_evidence = evidence(&safe_path, "safe-source");
    std::fs::write(&safe_path, "pub fn answer() -> i32 { \"wrong\" }\n").unwrap();
    let rustc = std::path::Path::new(sysroot.trim())
        .join("bin")
        .join(if cfg!(windows) { "rustc.exe" } else { "rustc" })
        .to_string_lossy()
        .into_owned();
    let swapped_diagnostic = std::process::Command::new(&rustc)
        .current_dir(&workspace)
        .args([
            "--edition=2021",
            "--crate-type=lib",
            "--error-format=json",
            "safe.rs",
        ])
        .output()
        .unwrap()
        .stderr;
    let swapped_path = workspace.join("swapped-diagnostic.jsonl");
    std::fs::write(&swapped_path, swapped_diagnostic).unwrap();
    let swapped_spec = hub
        .register_command_spec(&CommandSpecRequest {
            session_id: session_id.clone(),
            program: rustc,
            args: vec![
                "--edition=2021".into(),
                "--crate-type=lib".into(),
                "--error-format=json".into(),
                "safe.rs".into(),
            ],
            workspace_relative_cwd: ".".into(),
            expected_exit_code: 1,
            timeout_seconds: 30,
            artifact_paths: vec![],
            sandbox_profile: Default::default(),
            idempotency_key: "swapped-spec".into(),
        })
        .unwrap()
        .spec;
    let swapped_run = hub.verify(&swapped_spec.spec_id).await.unwrap();
    let swapped_diagnostic_id = evidence(&swapped_path, "swapped-diagnostic");
    std::fs::write(&safe_path, "pub fn answer() -> i32 { 42 }\n").unwrap();
    assert!(
        hub.open_rust_repair(&RustRepairOpenRequest {
            diagnostic_evidence_id: swapped_diagnostic_id,
            reproduction_evidence_id: safe_evidence,
            reproduction_run_id: swapped_run.run.run_id,
            idempotency_key: "swapped-open".into(),
            ..open_request.clone()
        })
        .is_err()
    );
    let case = hub.open_rust_repair(&open_request).unwrap().case;
    assert!(hub.open_rust_repair(&open_request).unwrap().duplicate);
    let search = RustRepairSearchRequest {
        workspace: workspace_text.clone(),
        diagnostic_code: "E0308".into(),
        rustc_version: version.clone(),
        edition: "2021".into(),
        limit: None,
    };
    assert!(hub.search_rust_repairs(&search).unwrap().lessons.is_empty());
    assert!(
        hub.learn_rust_repair(&RustRepairLearnRequest {
            case_id: case.case_id.clone(),
            rule: "Use i32".into(),
            applicability: "E0308".into(),
            counterexample: "String result".into(),
            test_run_id: "missing".into(),
            idempotency_key: "learn-too-early".into(),
        })
        .is_err()
    );
    assert!(
        hub.open_rust_repair(&RustRepairOpenRequest {
            diagnostic_code: "E0309".into(),
            idempotency_key: "same-evidence-other-code".into(),
            ..open_request.clone()
        })
        .is_err()
    );
    hub.claim_task(&TaskClaimRequest {
        task_id: task_id.clone(),
        worker_id: "worker".into(),
        lease_seconds: 120,
        idempotency_key: "claim".into(),
    })
    .unwrap();
    std::fs::write(&source, "pub fn answer() -> i32 { 42 }\n#[cfg(test)] mod tests { #[test] fn repaired() { assert_eq!(super::answer(), 42); } }\n").unwrap();
    let verified = hub.verify(&spec.spec_id).await.unwrap();
    let claim_id = if let Some(claim_id) = verified.verified_claim_id.clone() {
        claim_id
    } else {
        // A runner receipt deliberately retains hashes rather than output.
        // Reproduce only this owned fixture's failed command for CI diagnostics.
        let receipt = verified.receipt.as_ref().unwrap();
        let mut command =
            tokio::process::Command::new(receipt.resolved_executable.as_ref().unwrap());
        command
            .args(&spec.args)
            .current_dir(&workspace)
            .env_clear()
            .kill_on_drop(true);
        for name in [
            "PATH",
            "HOME",
            "TMPDIR",
            "LANG",
            "LC_ALL",
            "CARGO_HOME",
            "RUSTUP_HOME",
            "SYSTEMROOT",
            "WINDIR",
            "COMSPEC",
            "PATHEXT",
            "TEMP",
            "TMP",
            "USERPROFILE",
            "INCLUDE",
            "LIB",
            "LIBPATH",
            "ProgramFiles",
            "ProgramFiles(x86)",
            "VCINSTALLDIR",
            "VSINSTALLDIR",
            "VSCMD_ARG_TGT_ARCH",
            "VCToolsVersion",
            "WindowsSdkDir",
            "WindowsSDKVersion",
        ] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        let diagnostic =
            tokio::time::timeout(std::time::Duration::from_secs(30), command.output()).await;
        let diagnostic = match diagnostic {
            Ok(Ok(output)) => output,
            other => panic!(
                "repaired fixture cargo test did not succeed: {verified:?}\nDiagnostic retry failed: {other:?}"
            ),
        };
        panic!(
            "repaired fixture cargo test did not succeed: {verified:?}\nDiagnostic retry: {}\n{}",
            diagnostic.status,
            String::from_utf8_lossy(&diagnostic.stderr),
        );
    };
    hub.complete_task(&TaskCompleteRequest {
        task_id,
        worker_id: "worker".into(),
        outcome_summary: "E0308 fixture repaired".into(),
        criterion_proofs: vec![CriterionProof {
            criterion: spec.success_claim,
            verified_claim_id: claim_id,
        }],
        idempotency_key: "complete".into(),
    })
    .unwrap();
    let learn_request = RustRepairLearnRequest {
        case_id: case.case_id.clone(),
        rule: "Return the declared i32 value".into(),
        applicability: "A function declared -> i32 returns a string literal".into(),
        counterexample: "A function returning &str should keep a string result".into(),
        test_run_id: verified.run.run_id,
        idempotency_key: "learn".into(),
    };
    let learned = hub.learn_rust_repair(&learn_request).unwrap();
    assert!(!learned.duplicate);
    assert!(hub.learn_rust_repair(&learn_request).unwrap().duplicate);
    let export = hub.export_project(&workspace_text).unwrap();
    assert_eq!(export.format_version, 24);
    assert_eq!(export.rust_repair_cases.len(), 1);
    assert_eq!(export.rust_repair_lessons.len(), 1);
    assert_eq!(export.rustc_input_snapshots.len(), 2);
    assert!(
        export
            .events
            .iter()
            .any(|event| event.kind == "rust_repair_opened")
    );
    assert!(
        export
            .events
            .iter()
            .any(|event| event.kind == "rust_repair_learned")
    );
    let cargo = std::path::Path::new(sysroot.trim())
        .join("bin")
        .join(if cfg!(windows) { "cargo.exe" } else { "cargo" })
        .to_string_lossy()
        .into_owned();
    let help_spec = hub
        .register_command_spec(&CommandSpecRequest {
            session_id: session_id.clone(),
            program: cargo,
            args: vec!["test".into(), "--offline".into(), "--help".into()],
            workspace_relative_cwd: ".".into(),
            expected_exit_code: 0,
            timeout_seconds: 30,
            artifact_paths: vec![],
            sandbox_profile: Default::default(),
            idempotency_key: "help-spec".into(),
        })
        .unwrap()
        .spec;
    let help_task = hub
        .create_task(&TaskCreateRequest {
            session_id,
            objective: "Do not learn from cargo help".into(),
            acceptance_criteria: vec![help_spec.success_claim.clone()],
            write_scope: vec![],
            depends_on: vec![],
            idempotency_key: "help-task".into(),
        })
        .unwrap()
        .task
        .task_id;
    let help_case = hub
        .open_rust_repair(&RustRepairOpenRequest {
            task_id: help_task.clone(),
            idempotency_key: "help-case".into(),
            ..open_request
        })
        .unwrap()
        .case;
    hub.claim_task(&TaskClaimRequest {
        task_id: help_task.clone(),
        worker_id: "worker".into(),
        lease_seconds: 120,
        idempotency_key: "help-claim".into(),
    })
    .unwrap();
    let help_run = hub.verify(&help_spec.spec_id).await.unwrap();
    hub.complete_task(&TaskCompleteRequest {
        task_id: help_task,
        worker_id: "worker".into(),
        outcome_summary: "Help command completed".into(),
        criterion_proofs: vec![CriterionProof {
            criterion: help_spec.success_claim,
            verified_claim_id: help_run.verified_claim_id.unwrap(),
        }],
        idempotency_key: "help-complete".into(),
    })
    .unwrap();
    assert!(
        hub.learn_rust_repair(&RustRepairLearnRequest {
            case_id: help_case.case_id,
            test_run_id: help_run.run.run_id,
            idempotency_key: "help-learn".into(),
            ..learn_request
        })
        .is_err()
    );
    drop(hub);
    let hub = Hub::open(&database).unwrap();
    let found = hub.search_rust_repairs(&search).unwrap();
    assert_eq!(found.lessons.len(), 1);
    assert_eq!(found.lessons[0].case.case_id, case.case_id);
    assert!(found.advisory);
    assert!(
        hub.search_rust_repairs(&RustRepairSearchRequest {
            rustc_version: "rustc 0.0.0".into(),
            ..search
        })
        .unwrap()
        .lessons
        .is_empty()
    );
}
