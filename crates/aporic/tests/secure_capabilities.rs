use std::{path::Path, process::Command};

use aporic::{
    Hub,
    domain::{
        CapabilityEffectClass, CapabilityGetRequest, CapabilityMaturity, CapabilityProviderKind,
        CapabilityRegisterRequest, CapabilitySearchRequest, EvidenceKind, EvidenceRequest,
        ExperimentComparison, ExperimentCreateRequest, ExperimentCriterionInput,
        ExperimentCriterionKind, ExperimentDecisionKind, ExperimentDecisionRequest,
        ExperimentDiversityAxis, ExperimentMeasurementAddRequest, ExperimentVariantAddRequest,
        GitObserveRequest, OpenRequest, SecurityAssessmentGetRequest, SecurityCoverage,
        SecurityImportRequest,
    },
};

fn git(workspace: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(workspace)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fixture() -> (tempfile::TempDir, std::path::PathBuf, Hub, String, String) {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    git(&workspace, &["init", "-b", "main"]);
    std::fs::write(workspace.join("product.txt"), "initial\n").unwrap();
    git(&workspace, &["add", "."]);
    git(
        &workspace,
        &[
            "-c",
            "user.name=Aporic Test",
            "-c",
            "user.email=aporic@example.invalid",
            "commit",
            "-m",
            "initial",
        ],
    );
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Exercise secure capability fabric".to_owned(),
            idempotency_key: "open-v12".to_owned(),
        })
        .unwrap()
        .session_id;
    let snapshot_id = hub
        .observe_git(&GitObserveRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            base_ref: None,
        })
        .unwrap()
        .snapshot_id;
    (area, workspace, hub, session_id, snapshot_id)
}

#[test]
fn migrates_v11_through_execution_governance() {
    let area = tempfile::tempdir().unwrap();
    let database = area.path().join("aporic.sqlite3");
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute_batch(include_str!("../../../migrations/0001_initial.sql"))
        .unwrap();
    for migration in [
        include_str!("../../../migrations/0006_authority_bound_context.sql"),
        include_str!("../../../migrations/0007_memory_lifecycle.sql"),
        include_str!("../../../migrations/0008_runtime_trace.sql"),
        include_str!("../../../migrations/0009_git_governance.sql"),
        include_str!("../../../migrations/0010_token_efficiency.sql"),
        include_str!("../../../migrations/0011_commit_bound_deliberation.sql"),
    ] {
        connection.execute_batch(migration).unwrap();
    }
    drop(connection);
    assert_eq!(
        Hub::open(database).unwrap().stats().unwrap().schema_version,
        19
    );
}

#[test]
fn manifests_are_bounded_catalog_data_and_never_executable() {
    let (area, workspace, hub, session_id, _snapshot_id) = fixture();
    let request = CapabilityRegisterRequest {
        session_id,
        capability_id: "aporic.prototype-portfolio".to_owned(),
        version: "1".to_owned(),
        provider_kind: CapabilityProviderKind::BuiltIn,
        title: "Prototype portfolio".to_owned(),
        description: "Compare commit-bound variants with direct evidence.".to_owned(),
        effect_class: CapabilityEffectClass::RecordLocal,
        maturity: Some(CapabilityMaturity::Propose),
        reads_private_data: false,
        sees_untrusted_content: true,
        uses_network: false,
        requires_credentials: false,
        idempotent: true,
        reversible: true,
        input_schema: serde_json::json!({"type":"object","properties":{"campaign":{"type":"string"}}}),
        output_schema: serde_json::json!({"type":"object"}),
        evidence_contract: "Direct evidence is required for hard gates.".to_owned(),
        implementation_sha256: None,
        idempotency_key: "register-portfolio".to_owned(),
    };
    let first = hub.register_capability(&request).unwrap();
    let duplicate = hub.register_capability(&request).unwrap();
    assert!(!first.duplicate);
    assert!(duplicate.duplicate);
    assert!(!first.capability.executable);
    assert_eq!(
        hub.search_capabilities(&CapabilitySearchRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            query: Some("prototype".to_owned()),
            include_unavailable: false,
            limit: None,
        })
        .unwrap()
        .len(),
        1
    );
    assert!(
        !hub.get_capability(&CapabilityGetRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            capability_id: "aporic.prototype-portfolio".to_owned(),
            version: "1".to_owned(),
        })
        .unwrap()
        .executable
    );

    let mut immature = request.clone();
    immature.capability_id = "bad.immature-effect".to_owned();
    immature.effect_class = CapabilityEffectClass::ExternalEffect;
    immature.maturity = Some(CapabilityMaturity::Propose);
    immature.idempotency_key = "reject-immature-effect".to_owned();
    assert!(
        hub.register_capability(&immature)
            .unwrap_err()
            .to_string()
            .contains("below the minimum")
    );

    let mut unsafe_routine = request.clone();
    unsafe_routine.capability_id = "bad.non-idempotent-routine".to_owned();
    unsafe_routine.maturity = Some(CapabilityMaturity::PersistentRoutine);
    unsafe_routine.idempotent = false;
    unsafe_routine.idempotency_key = "reject-non-idempotent-routine".to_owned();
    assert!(
        hub.register_capability(&unsafe_routine)
            .unwrap_err()
            .to_string()
            .contains("idempotent=true")
    );

    let mut secret = request;
    secret.capability_id = "bad.secret".to_owned();
    secret.input_schema = serde_json::json!({"type":"object","token":"do-not-store"});
    secret.idempotency_key = "reject-secret".to_owned();
    assert!(
        hub.register_capability(&secret)
            .unwrap_err()
            .to_string()
            .contains("credential-like")
    );
    assert!(hub.audit_secure_capabilities().unwrap().consistent);
    rusqlite::Connection::open(area.path().join("aporic.sqlite3"))
        .unwrap()
        .execute(
            "UPDATE capability_manifests SET title = 'tampered' WHERE capability_id = ?1",
            ["aporic.prototype-portfolio"],
        )
        .unwrap();
    let audit = hub.audit_secure_capabilities().unwrap();
    assert!(!audit.consistent);
    assert_eq!(audit.integrity_failure_count, 1);
}

#[test]
fn experiments_require_direct_hard_gate_evidence_and_preserve_budget() {
    let (_area, workspace, hub, session_id, snapshot_id) = fixture();
    let workspace_text = workspace.to_string_lossy().into_owned();
    let created = hub
        .create_experiment(&ExperimentCreateRequest {
            workspace: workspace_text.clone(),
            title: "Two prototype tournament".to_owned(),
            problem: "Planning confidence is low before implementation.".to_owned(),
            target_user: "Product team".to_owned(),
            desired_outcome: "Select a tested approach.".to_owned(),
            hypothesis: "Diverse prototypes reduce premature convergence.".to_owned(),
            git_snapshot_id: snapshot_id.clone(),
            max_variants: 2,
            max_token_budget: Some(10_000),
            criteria: vec![
                ExperimentCriterionInput {
                    name: "security gate".to_owned(),
                    kind: ExperimentCriterionKind::HardGate,
                    comparison: ExperimentComparison::MustPass,
                    threshold: None,
                    unit: "boolean".to_owned(),
                },
                ExperimentCriterionInput {
                    name: "latency".to_owned(),
                    kind: ExperimentCriterionKind::ParetoDimension,
                    comparison: ExperimentComparison::Minimize,
                    threshold: None,
                    unit: "ms".to_owned(),
                },
            ],
            idempotency_key: "create-experiment".to_owned(),
        })
        .unwrap();
    let campaign_id = created.portfolio.campaign.campaign_id;
    let hard_id = created
        .portfolio
        .criteria
        .iter()
        .find(|value| value.kind == ExperimentCriterionKind::HardGate)
        .unwrap()
        .criterion_id
        .clone();
    let latency_id = created
        .portfolio
        .criteria
        .iter()
        .find(|value| value.kind == ExperimentCriterionKind::ParetoDimension)
        .unwrap()
        .criterion_id
        .clone();
    let first = hub
        .add_experiment_variant(&ExperimentVariantAddRequest {
            workspace: workspace_text.clone(),
            campaign_id: campaign_id.clone(),
            name: "Local state machine".to_owned(),
            diversity_axis: ExperimentDiversityAxis::Architecture,
            approach: "Use an append-only local state machine.".to_owned(),
            git_snapshot_id: snapshot_id.clone(),
            parent_variant_ids: vec![],
            idempotency_key: "variant-one".to_owned(),
        })
        .unwrap()
        .portfolio
        .variants[0]
        .variant_id
        .clone();
    let second = hub
        .add_experiment_variant(&ExperimentVariantAddRequest {
            workspace: workspace_text.clone(),
            campaign_id: campaign_id.clone(),
            name: "Artifact exchange".to_owned(),
            diversity_axis: ExperimentDiversityAxis::DataModel,
            approach: "Use immutable artifact exchange.".to_owned(),
            git_snapshot_id: snapshot_id.clone(),
            parent_variant_ids: vec![],
            idempotency_key: "variant-two".to_owned(),
        })
        .unwrap()
        .portfolio
        .variants[1]
        .variant_id
        .clone();
    assert!(
        hub.add_experiment_variant(&ExperimentVariantAddRequest {
            workspace: workspace_text.clone(),
            campaign_id: campaign_id.clone(),
            name: "Third".to_owned(),
            diversity_axis: ExperimentDiversityAxis::Ux,
            approach: "A third approach.".to_owned(),
            git_snapshot_id: snapshot_id,
            parent_variant_ids: vec![],
            idempotency_key: "variant-three".to_owned(),
        })
        .unwrap_err()
        .to_string()
        .contains("budget")
    );

    std::fs::write(workspace.join("evidence.txt"), "measured locally\n").unwrap();
    let evidence_id = hub
        .add_evidence(&EvidenceRequest {
            session_id,
            kind: EvidenceKind::WorkspaceFile,
            locator: workspace
                .join("evidence.txt")
                .to_string_lossy()
                .into_owned(),
            summary: "Direct evaluation evidence".to_owned(),
            content_sha256: None,
            idempotency_key: "experiment-evidence".to_owned(),
        })
        .unwrap()
        .evidence
        .evidence_id;
    for (variant_id, criterion_id, value, key) in [
        (&first, &hard_id, 1, "first-hard"),
        (&first, &latency_id, 80, "first-latency"),
        (&second, &hard_id, 0, "second-hard"),
        (&second, &latency_id, 40, "second-latency"),
    ] {
        hub.add_experiment_measurement(&ExperimentMeasurementAddRequest {
            workspace: workspace_text.clone(),
            campaign_id: campaign_id.clone(),
            variant_id: variant_id.clone(),
            criterion_id: criterion_id.clone(),
            value,
            evidence_id: Some(evidence_id.clone()),
            claim_id: None,
            idempotency_key: key.to_owned(),
        })
        .unwrap();
    }
    let portfolio = hub
        .get_experiment(&aporic::domain::ExperimentGetRequest {
            workspace: workspace_text.clone(),
            campaign_id: campaign_id.clone(),
            limit: None,
        })
        .unwrap();
    assert_eq!(portfolio.pareto_variant_ids, vec![first.clone()]);
    assert_eq!(portfolio.hard_gate_failed_variant_ids, vec![second]);
    assert!(portfolio.budget_exhausted);
    assert!(
        hub.decide_experiment(&ExperimentDecisionRequest {
            workspace: workspace_text,
            campaign_id,
            kind: ExperimentDecisionKind::Select,
            variant_id: Some(first),
            summary: "Select the qualified candidate.".to_owned(),
            deliberation_id: None,
            deliberation_decision_id: None,
            idempotency_key: "reject-undeliberated-selection".to_owned(),
        })
        .unwrap_err()
        .to_string()
        .contains("deliberation")
    );
}

#[test]
fn imports_security_artifacts_without_claiming_safety_and_validates_backup() {
    let (area, _workspace, hub, session_id, snapshot_id) = fixture();
    let artifacts = area.path().join("security");
    std::fs::create_dir(&artifacts).unwrap();
    let manifest = artifacts.join("scan-manifest.json");
    let findings = artifacts.join("findings.json");
    let coverage = artifacts.join("coverage.json");
    std::fs::write(&manifest, r#"{"scanId":"scan-1"}"#).unwrap();
    std::fs::write(&findings, r#"{"findings":[]}"#).unwrap();
    std::fs::write(&coverage, r#"{"completeness":"partial"}"#).unwrap();
    let imported = hub
        .import_codex_security(&SecurityImportRequest {
            session_id,
            provider_capability_id: "openai.codex-security".to_owned(),
            provider_version: "artifact-v1".to_owned(),
            source_scan_id: "scan-1".to_owned(),
            git_snapshot_id: snapshot_id,
            manifest_path: manifest.to_string_lossy().into_owned(),
            findings_path: findings.to_string_lossy().into_owned(),
            coverage_path: coverage.to_string_lossy().into_owned(),
            idempotency_key: "import-security".to_owned(),
        })
        .unwrap();
    assert_eq!(imported.assessment.coverage, SecurityCoverage::Partial);
    assert!(!imported.assessment.safety_proven);
    let read = hub
        .get_security_assessment(&SecurityAssessmentGetRequest {
            workspace: area.path().join("workspace").to_string_lossy().into_owned(),
            assessment_id: imported.assessment.assessment_id,
        })
        .unwrap();
    assert!(!read.safety_proven);

    let backup = area.path().join("backup.sqlite3");
    hub.backup_to(&backup).unwrap();
    assert_eq!(Hub::validate_backup(&backup).unwrap(), 19);
    assert!(
        hub.backup_to(&backup)
            .unwrap_err()
            .to_string()
            .contains("already exists")
    );
}
