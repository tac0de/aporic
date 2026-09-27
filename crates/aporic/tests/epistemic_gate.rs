use aporic::{
    Hub,
    domain::{
        ClaimRequest, ClaimStatus, CloseDisposition, CloseRequest, Consequence, DissentRequest,
        EvidenceGrade, EvidenceKind, EvidenceRequest, ModelRouteRequest, OpenRequest,
        WorkComplexity, WorkKind, workspace_file_claim,
    },
};

#[test]
fn typed_gate_blocks_unsupported_certainty_and_noise() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Exercise the epistemic gate".to_owned(),
            idempotency_key: "open-gate".to_owned(),
        })
        .unwrap()
        .session_id;

    let reported = hub
        .add_evidence(&EvidenceRequest {
            session_id: session_id.clone(),
            kind: EvidenceKind::CommandResult,
            locator: "cargo test".to_owned(),
            summary: "A model says the tests passed".to_owned(),
            content_sha256: Some("b".repeat(64)),
            idempotency_key: "reported".to_owned(),
        })
        .unwrap()
        .evidence;
    assert_eq!(reported.grade, EvidenceGrade::Reported);
    assert!(
        hub.assert_claim(&ClaimRequest {
            session_id: session_id.clone(),
            status: ClaimStatus::Verified,
            statement: "All tests passed".to_owned(),
            material: true,
            evidence_ids: vec![reported.evidence_id],
            subject_key: None,
            supersedes_claim_id: None,
            idempotency_key: "false-certainty".to_owned(),
        })
        .is_err()
    );

    let artifact = std::fs::canonicalize(&workspace)
        .unwrap()
        .join("artifact.txt");
    let artifact_subject = format!("file:{}", artifact.display());
    let unknown = hub
        .assert_claim(&ClaimRequest {
            session_id: session_id.clone(),
            status: ClaimStatus::Unknown,
            statement: "Whether the generated artifact matches the workspace".to_owned(),
            material: true,
            evidence_ids: vec![],
            subject_key: Some(artifact_subject.clone()),
            supersedes_claim_id: None,
            idempotency_key: "unknown".to_owned(),
        })
        .unwrap()
        .claim;
    assert!(
        hub.close_session(&CloseRequest {
            session_id: session_id.clone(),
            disposition: CloseDisposition::Completed,
            summary: "Premature completion".to_owned(),
            next_action: None,
            idempotency_key: "premature-close".to_owned(),
        })
        .is_err()
    );

    std::fs::write(&artifact, "observed state\n").unwrap();
    let direct = hub
        .add_evidence(&EvidenceRequest {
            session_id: session_id.clone(),
            kind: EvidenceKind::WorkspaceFile,
            locator: artifact.to_string_lossy().into_owned(),
            summary: "Aporic read and hashed the workspace artifact".to_owned(),
            content_sha256: None,
            idempotency_key: "direct".to_owned(),
        })
        .unwrap()
        .evidence;
    assert_eq!(direct.grade, EvidenceGrade::Direct);
    let direct_statement = workspace_file_claim(&direct.locator, &direct.content_sha256);
    let verified = hub
        .assert_claim(&ClaimRequest {
            session_id: session_id.clone(),
            status: ClaimStatus::Verified,
            statement: direct_statement,
            material: true,
            evidence_ids: vec![direct.evidence_id.clone()],
            subject_key: Some(artifact_subject),
            supersedes_claim_id: Some(unknown.claim_id),
            idempotency_key: "resolve-unknown".to_owned(),
        })
        .unwrap()
        .claim;

    let noise = hub
        .assess_dissent(&DissentRequest {
            session_id: session_id.clone(),
            target_claim_id: verified.claim_id.clone(),
            consequence: Consequence::Medium,
            actionable_change: "Reconsider naming".to_owned(),
            evidence_ids: vec![],
        })
        .unwrap();
    assert!(!noise.surface);
    assert!(noise.reasons.contains(&"consequence_below_high".to_owned()));

    let material = hub
        .assess_dissent(&DissentRequest {
            session_id: session_id.clone(),
            target_claim_id: verified.claim_id,
            consequence: Consequence::High,
            actionable_change: "Re-open the artifact and compare its digest".to_owned(),
            evidence_ids: vec![direct.evidence_id],
        })
        .unwrap();
    assert!(material.surface);

    hub.close_session(&CloseRequest {
        session_id,
        disposition: CloseDisposition::Completed,
        summary: "Epistemic gate simulation completed".to_owned(),
        next_action: None,
        idempotency_key: "verified-close".to_owned(),
    })
    .unwrap();
}

#[test]
fn keyed_unknown_rejects_unrelated_direct_file_evidence() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Bind claims to evidence subjects".to_owned(),
            idempotency_key: "open-subject-binding".to_owned(),
        })
        .unwrap()
        .session_id;
    let subject_file = workspace.join("subject.txt");
    let unrelated_file = workspace.join("unrelated.txt");
    std::fs::write(&subject_file, "subject").unwrap();
    std::fs::write(&unrelated_file, "unrelated").unwrap();
    let subject_key = format!(
        "file:{}",
        std::fs::canonicalize(&subject_file).unwrap().display()
    );
    let unrelated_key = format!(
        "file:{}",
        std::fs::canonicalize(&unrelated_file).unwrap().display()
    );

    let missing_key = hub.assert_claim(&ClaimRequest {
        session_id: session_id.clone(),
        status: ClaimStatus::Unknown,
        statement: "Whether subject.txt is current".to_owned(),
        subject_key: None,
        material: true,
        evidence_ids: Vec::new(),
        supersedes_claim_id: None,
        idempotency_key: "missing-subject-key".to_owned(),
    });
    assert!(missing_key.unwrap_err().to_string().contains("subject_key"));
    let unknown = hub
        .assert_claim(&ClaimRequest {
            session_id: session_id.clone(),
            status: ClaimStatus::Unknown,
            statement: "Whether subject.txt is current".to_owned(),
            subject_key: Some(subject_key.clone()),
            material: true,
            evidence_ids: Vec::new(),
            supersedes_claim_id: None,
            idempotency_key: "keyed-unknown".to_owned(),
        })
        .unwrap()
        .claim;
    let unrelated = hub
        .add_evidence(&EvidenceRequest {
            session_id: session_id.clone(),
            kind: EvidenceKind::WorkspaceFile,
            locator: unrelated_file.to_string_lossy().into_owned(),
            summary: "Read unrelated file".to_owned(),
            content_sha256: None,
            idempotency_key: "unrelated-evidence".to_owned(),
        })
        .unwrap()
        .evidence;
    for (key, idempotency_key) in [
        (unrelated_key, "mismatched-subject"),
        (subject_key.clone(), "false-matching-subject"),
    ] {
        let result = hub.assert_claim(&ClaimRequest {
            session_id: session_id.clone(),
            status: ClaimStatus::Verified,
            statement: workspace_file_claim(&unrelated.locator, &unrelated.content_sha256),
            subject_key: Some(key),
            material: true,
            evidence_ids: vec![unrelated.evidence_id.clone()],
            supersedes_claim_id: Some(unknown.claim_id.clone()),
            idempotency_key: idempotency_key.to_owned(),
        });
        assert!(
            result.is_err(),
            "unrelated file must not resolve keyed unknown"
        );
    }
    let subject = hub
        .add_evidence(&EvidenceRequest {
            session_id: session_id.clone(),
            kind: EvidenceKind::WorkspaceFile,
            locator: subject_file.to_string_lossy().into_owned(),
            summary: "Read subject file".to_owned(),
            content_sha256: None,
            idempotency_key: "subject-evidence".to_owned(),
        })
        .unwrap()
        .evidence;
    let resolution = hub
        .assert_claim(&ClaimRequest {
            session_id,
            status: ClaimStatus::Verified,
            statement: workspace_file_claim(&subject.locator, &subject.content_sha256),
            subject_key: Some(subject_key.clone()),
            material: true,
            evidence_ids: vec![subject.evidence_id],
            supersedes_claim_id: Some(unknown.claim_id),
            idempotency_key: "matching-subject".to_owned(),
        })
        .unwrap()
        .claim;
    assert_eq!(
        resolution.subject_key.as_deref(),
        Some(subject_key.as_str())
    );
}

#[cfg(unix)]
#[test]
fn keyed_unknown_rejects_retargeted_symlink_evidence() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let first = workspace.join("first.txt");
    let second = workspace.join("second.txt");
    let link = workspace.join("link.txt");
    std::fs::write(&first, "first bytes").unwrap();
    std::fs::write(&second, "second bytes").unwrap();
    std::os::unix::fs::symlink(&first, &link).unwrap();
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Reject mutable evidence identity".to_owned(),
            idempotency_key: "open-retarget".to_owned(),
        })
        .unwrap()
        .session_id;
    let subject_key = format!("file:{}", std::fs::canonicalize(&second).unwrap().display());
    let unknown = hub
        .assert_claim(&ClaimRequest {
            session_id: session_id.clone(),
            status: ClaimStatus::Unknown,
            statement: "Whether second.txt matches evidence".to_owned(),
            subject_key: Some(subject_key.clone()),
            material: true,
            evidence_ids: Vec::new(),
            supersedes_claim_id: None,
            idempotency_key: "unknown-retarget".to_owned(),
        })
        .unwrap()
        .claim;
    let evidence = hub
        .add_evidence(&EvidenceRequest {
            session_id: session_id.clone(),
            kind: EvidenceKind::WorkspaceFile,
            locator: link.to_string_lossy().into_owned(),
            summary: "Link originally pointed at first.txt".to_owned(),
            content_sha256: None,
            idempotency_key: "evidence-retarget".to_owned(),
        })
        .unwrap()
        .evidence;
    std::fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink(&second, &link).unwrap();
    let resolution = hub.assert_claim(&ClaimRequest {
        session_id,
        status: ClaimStatus::Verified,
        statement: workspace_file_claim(&evidence.locator, &evidence.content_sha256),
        subject_key: Some(subject_key),
        material: true,
        evidence_ids: vec![evidence.evidence_id],
        supersedes_claim_id: Some(unknown.claim_id),
        idempotency_key: "false-retarget-resolution".to_owned(),
    });
    assert!(resolution.is_err());
}

#[test]
fn historical_unkeyed_unknown_keeps_legacy_resolution_rule() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Read a historical unkeyed unknown".to_owned(),
            idempotency_key: "open-legacy-unknown".to_owned(),
        })
        .unwrap()
        .session_id;
    rusqlite::Connection::open(&database)
        .unwrap()
        .execute(
            "INSERT INTO claims
             (claim_id, session_id, status, statement, material, created_at_unix_ms)
             VALUES ('legacy-unknown', ?1, 'unknown', 'Historical question', 1, 1)",
            [&session_id],
        )
        .unwrap();
    let path = workspace.join("legacy-proof.txt");
    std::fs::write(&path, "observed").unwrap();
    let evidence = hub
        .add_evidence(&EvidenceRequest {
            session_id: session_id.clone(),
            kind: EvidenceKind::WorkspaceFile,
            locator: path.to_string_lossy().into_owned(),
            summary: "Legacy direct proof".to_owned(),
            content_sha256: None,
            idempotency_key: "legacy-proof".to_owned(),
        })
        .unwrap()
        .evidence;
    let resolution = hub
        .assert_claim(&ClaimRequest {
            session_id,
            status: ClaimStatus::Verified,
            statement: workspace_file_claim(&evidence.locator, &evidence.content_sha256),
            subject_key: None,
            material: true,
            evidence_ids: vec![evidence.evidence_id],
            supersedes_claim_id: Some("legacy-unknown".to_owned()),
            idempotency_key: "legacy-resolution".to_owned(),
        })
        .unwrap()
        .claim;
    assert_eq!(resolution.subject_key, None);
    let exported = hub
        .export_project(workspace.to_string_lossy().as_ref())
        .unwrap();
    assert!(
        exported
            .claims
            .iter()
            .any(|claim| { claim.claim_id == "legacy-unknown" && claim.subject_key.is_none() })
    );
}

#[test]
fn workspace_evidence_rejects_files_above_the_streaming_limit() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let oversized = workspace.join("oversized.bin");
    std::fs::File::create(&oversized)
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Reject oversized direct evidence".to_owned(),
            idempotency_key: "oversized-evidence-open".to_owned(),
        })
        .unwrap()
        .session_id;

    let error = hub
        .add_evidence(&EvidenceRequest {
            session_id,
            kind: EvidenceKind::WorkspaceFile,
            locator: oversized.to_string_lossy().into_owned(),
            summary: "Must not be buffered".to_owned(),
            content_sha256: None,
            idempotency_key: "oversized-evidence".to_owned(),
        })
        .unwrap_err();
    assert!(error.to_string().contains("67108864-byte limit"));
    assert_eq!(hub.stats().unwrap().evidence_count, 0);
}

#[test]
fn model_router_uses_luna_sol_and_astra_without_conferring_authority() {
    let area = tempfile::tempdir().unwrap();
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let luna = hub.route_model(&ModelRouteRequest {
        work_kind: WorkKind::Extraction,
        complexity: WorkComplexity::Bounded,
        consequence: Consequence::Low,
        ambiguity_high: false,
        independent_review: false,
    });
    assert_eq!(luna.model, "gpt-6-luna");
    assert!(luna.advisory);

    let sol = hub.route_model(&ModelRouteRequest {
        work_kind: WorkKind::Implementation,
        complexity: WorkComplexity::Complex,
        consequence: Consequence::Medium,
        ambiguity_high: false,
        independent_review: false,
    });
    assert_eq!(sol.model, "gpt-6-sol");

    let astra = hub.route_model(&ModelRouteRequest {
        work_kind: WorkKind::Architecture,
        complexity: WorkComplexity::Frontier,
        consequence: Consequence::High,
        ambiguity_high: true,
        independent_review: true,
    });
    assert_eq!(astra.model, "gpt-6-astra");
    assert_eq!(astra.verifier_model.as_deref(), Some("gpt-6-sol"));
    assert!(
        astra
            .reasons
            .contains(&"model_output_is_not_evidence".to_owned())
    );
}
