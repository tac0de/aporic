use aporic::{
    Hub,
    domain::{
        ClaimRequest, ClaimStatus, EvidenceKind, EvidenceRequest, OpenRequest, TokenCountSource,
        TokenEfficiencyReportRequest, TokenUsageListRequest, TokenUsageRecordRequest, UsageOutcome,
        workspace_file_claim,
    },
};
use rusqlite::Connection;

#[test]
fn migrates_v9_to_v10_without_usage_receipts() {
    let area = tempfile::tempdir().unwrap();
    let database = area.path().join("aporic.sqlite3");
    let connection = Connection::open(&database).unwrap();
    connection
        .execute_batch(include_str!("../../../migrations/0001_initial.sql"))
        .unwrap();
    connection
        .execute_batch(include_str!(
            "../../../migrations/0006_authority_bound_context.sql"
        ))
        .unwrap();
    connection
        .execute_batch(include_str!(
            "../../../migrations/0007_memory_lifecycle.sql"
        ))
        .unwrap();
    connection
        .execute_batch(include_str!("../../../migrations/0008_runtime_trace.sql"))
        .unwrap();
    connection
        .execute_batch(include_str!("../../../migrations/0009_git_governance.sql"))
        .unwrap();
    drop(connection);

    let hub = Hub::open(database).unwrap();
    assert_eq!(hub.stats().unwrap().schema_version, 16);
    let audit = hub.audit_token_usage().unwrap();
    assert_eq!(audit.receipt_count, 0);
    assert!(audit.consistent);
}

#[test]
fn usage_receipts_separate_measurement_estimates_and_verified_outcomes() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Measure verified token efficiency".to_owned(),
            idempotency_key: "open-token-test".to_owned(),
        })
        .unwrap()
        .session_id;

    let artifact = workspace.join("verified.txt");
    std::fs::write(&artifact, "verified\n").unwrap();
    let evidence = hub
        .add_evidence(&EvidenceRequest {
            session_id: session_id.clone(),
            kind: EvidenceKind::WorkspaceFile,
            locator: artifact.to_string_lossy().into_owned(),
            summary: "Locally hashed verification artifact".to_owned(),
            content_sha256: None,
            idempotency_key: "token-evidence".to_owned(),
        })
        .unwrap()
        .evidence;
    let claim = hub
        .assert_claim(&ClaimRequest {
            session_id: session_id.clone(),
            status: ClaimStatus::Verified,
            statement: workspace_file_claim(&evidence.locator, &evidence.content_sha256),
            material: true,
            evidence_ids: vec![evidence.evidence_id],
            supersedes_claim_id: None,
            idempotency_key: "token-verified-claim".to_owned(),
        })
        .unwrap()
        .claim;

    let measured_request = TokenUsageRecordRequest {
        workspace: workspace.to_string_lossy().into_owned(),
        session_id: Some(session_id),
        scope_kind: "task".to_owned(),
        scope_id: "verified-change".to_owned(),
        model: Some("host/frontier".to_owned()),
        source_kind: TokenCountSource::HostReported,
        input_tokens: Some(100),
        output_tokens: Some(20),
        cached_input_tokens: Some(40),
        reasoning_tokens: Some(10),
        context_bytes: Some(300),
        outcome: UsageOutcome::VerifiedSuccess,
        verification_ref: Some(claim.claim_id),
        idempotency_key: "measured-usage".to_owned(),
    };
    let first = hub.record_token_usage(&measured_request).unwrap();
    let duplicate = hub.record_token_usage(&measured_request).unwrap();
    assert!(!first.duplicate);
    assert!(duplicate.duplicate);
    assert_eq!(first.receipt.receipt_id, duplicate.receipt.receipt_id);

    hub.record_token_usage(&TokenUsageRecordRequest {
        workspace: workspace.to_string_lossy().into_owned(),
        session_id: None,
        scope_kind: "context_capsule".to_owned(),
        scope_id: "capsule-1".to_owned(),
        model: None,
        source_kind: TokenCountSource::ConservativeByteUpperBound,
        input_tokens: None,
        output_tokens: None,
        cached_input_tokens: None,
        reasoning_tokens: None,
        context_bytes: Some(60),
        outcome: UsageOutcome::Unverified,
        verification_ref: None,
        idempotency_key: "estimated-usage".to_owned(),
    })
    .unwrap();

    let listed = hub
        .list_token_usage(&TokenUsageListRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            limit: None,
        })
        .unwrap();
    assert_eq!(listed.len(), 2);
    let report = hub
        .token_efficiency_report(&TokenEfficiencyReportRequest {
            workspace: workspace.to_string_lossy().into_owned(),
        })
        .unwrap();
    assert_eq!(report.measured_input_tokens, 100);
    assert_eq!(report.measured_output_tokens, 20);
    assert_eq!(report.measured_reasoning_tokens, 10);
    assert_eq!(report.cached_input_tokens, 40);
    assert_eq!(report.estimated_input_token_upper_bound, 60);
    assert_eq!(report.verified_successes, 1);
    assert_eq!(report.measured_tokens_per_verified_success, Some(130.0));
    assert!(!report.measurement_complete);
    assert!(
        report
            .warnings
            .contains(&"estimated_tokens_are_not_provider_counts".to_owned())
    );
    assert!(hub.audit_token_usage().unwrap().consistent);
    assert_eq!(
        hub.export_project(workspace.to_string_lossy().as_ref())
            .unwrap()
            .token_usage_receipts
            .len(),
        2
    );

    Connection::open(&database)
        .unwrap()
        .execute(
            "UPDATE token_usage_receipts SET input_tokens = 999 WHERE receipt_id = ?1",
            [&first.receipt.receipt_id],
        )
        .unwrap();
    assert!(!hub.audit_token_usage().unwrap().consistent);
}

#[test]
fn usage_receipts_reject_false_precision_and_unsupported_verification() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let base = TokenUsageRecordRequest {
        workspace: workspace.to_string_lossy().into_owned(),
        session_id: None,
        scope_kind: "task".to_owned(),
        scope_id: "unsupported".to_owned(),
        model: None,
        source_kind: TokenCountSource::Unknown,
        input_tokens: Some(10),
        output_tokens: None,
        cached_input_tokens: None,
        reasoning_tokens: None,
        context_bytes: None,
        outcome: UsageOutcome::Unverified,
        verification_ref: None,
        idempotency_key: "false-precision".to_owned(),
    };
    assert!(hub.record_token_usage(&base).is_err());

    let mut unsupported = base;
    unsupported.source_kind = TokenCountSource::HostReported;
    unsupported.outcome = UsageOutcome::VerifiedSuccess;
    unsupported.verification_ref = Some("invented-proof".to_owned());
    unsupported.idempotency_key = "unsupported-proof".to_owned();
    assert!(hub.record_token_usage(&unsupported).is_err());
}

#[test]
fn offline_efficiency_simulation_preserves_essential_context() {
    let report = aporic::eval::simulate_token_efficiency();
    assert!(report.deterministic);
    assert_eq!(
        report.essential_items_retained,
        report.essential_items_expected
    );
    assert_eq!(report.unresolved_unknowns_retained, 1);
    assert!(report.optimized_context_bytes < report.baseline_context_bytes);
    assert_eq!(report.exact_token_claims_from_byte_estimates, 0);
    assert_eq!(report.network_or_model_calls, 0);
}
