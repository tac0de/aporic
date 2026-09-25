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
            supersedes_claim_id: None,
            idempotency_key: "false-certainty".to_owned(),
        })
        .is_err()
    );

    let unknown = hub
        .assert_claim(&ClaimRequest {
            session_id: session_id.clone(),
            status: ClaimStatus::Unknown,
            statement: "Whether the generated artifact matches the workspace".to_owned(),
            material: true,
            evidence_ids: vec![],
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

    let artifact = workspace.join("artifact.txt");
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
fn model_router_uses_terra_sol_and_astra_without_conferring_authority() {
    let area = tempfile::tempdir().unwrap();
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let terra = hub.route_model(&ModelRouteRequest {
        work_kind: WorkKind::Extraction,
        complexity: WorkComplexity::Bounded,
        consequence: Consequence::Low,
        ambiguity_high: false,
        independent_review: false,
    });
    assert_eq!(terra.model, "gpt-5.6-terra");
    assert!(terra.advisory);

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
