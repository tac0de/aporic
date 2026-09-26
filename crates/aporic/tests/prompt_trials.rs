use aporic::{
    Hub,
    domain::{
        ClaimRequest, ClaimStatus, EvidenceKind, EvidenceRequest, OpenRequest,
        PromptComparisonRequest, PromptCriterionAssessment, PromptCriterionRating,
        PromptTrialRequest, TaskBriefRequest, TaskCreateRequest, workspace_file_claim,
    },
};
use sha2::{Digest, Sha256};

#[test]
fn compares_two_variants_without_promoting_reported_quality_to_causal_proof() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let response = b"RAW-SECRET-RESPONSE-CONTENT";
    let answer_path = workspace.join("answer.txt");
    std::fs::write(&answer_path, response).unwrap();
    let response_sha256 = format!("{:x}", Sha256::digest(response));
    let criterion = workspace_file_claim(answer_path.to_string_lossy().as_ref(), &response_sha256);
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Compare task brief variants".to_owned(),
            idempotency_key: "prompt-session".to_owned(),
        })
        .unwrap()
        .session_id;
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id: session_id.clone(),
            objective: "Compare task brief variants".to_owned(),
            acceptance_criteria: vec![criterion.clone()],
            write_scope: vec!["answer.txt".to_owned()],
            depends_on: Vec::new(),
            idempotency_key: "prompt-task".to_owned(),
        })
        .unwrap()
        .task
        .task_id;
    let baseline = hub
        .task_brief(&TaskBriefRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            task_id: task_id.clone(),
            max_context_bytes: Some(2_048),
            variant: Some("baseline".to_owned()),
            idempotency_key: "baseline-brief".to_owned(),
        })
        .unwrap();
    let evidence_first = hub
        .task_brief(&TaskBriefRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            task_id: task_id.clone(),
            max_context_bytes: Some(2_048),
            variant: Some("evidence_first".to_owned()),
            idempotency_key: "evidence-first-brief".to_owned(),
        })
        .unwrap();
    assert_ne!(
        baseline.receipt.template_id,
        evidence_first.receipt.template_id
    );
    assert_ne!(
        baseline.receipt.template_sha256,
        evidence_first.receipt.template_sha256
    );
    assert_ne!(baseline.brief, evidence_first.brief);

    let evidence = hub
        .add_evidence(&EvidenceRequest {
            session_id: session_id.clone(),
            kind: EvidenceKind::WorkspaceFile,
            locator: answer_path.to_string_lossy().into_owned(),
            summary: "Response artifact for the pilot".to_owned(),
            content_sha256: None,
            idempotency_key: "answer-evidence".to_owned(),
        })
        .unwrap()
        .evidence;
    let claim_id = hub
        .assert_claim(&ClaimRequest {
            session_id,
            status: ClaimStatus::Verified,
            statement: criterion.clone(),
            material: false,
            evidence_ids: vec![evidence.evidence_id.clone()],
            supersedes_claim_id: None,
            idempotency_key: "answer-claim".to_owned(),
        })
        .unwrap()
        .claim
        .claim_id;
    let baseline_trial = PromptTrialRequest {
        workspace: workspace.to_string_lossy().into_owned(),
        brief_receipt_id: baseline.receipt.receipt_id,
        response_sha256: response_sha256.clone(),
        response_evidence_id: Some(evidence.evidence_id),
        reported_model: Some("host-model".to_owned()),
        assessments: vec![PromptCriterionAssessment {
            criterion_index: 0,
            criterion: criterion.clone(),
            rating: PromptCriterionRating::Met,
            verified_claim_id: Some(claim_id),
        }],
        idempotency_key: "baseline-trial".to_owned(),
    };
    let first = hub.record_prompt_trial(&baseline_trial).unwrap();
    assert!(first.trial.response_file_verified);
    assert_eq!(first.trial.rating_provenance, "host_reported");
    assert!(hub.record_prompt_trial(&baseline_trial).unwrap().duplicate);
    let reported_trial = PromptTrialRequest {
        workspace: workspace.to_string_lossy().into_owned(),
        brief_receipt_id: evidence_first.receipt.receipt_id,
        response_sha256: "b".repeat(64),
        response_evidence_id: None,
        reported_model: None,
        assessments: vec![PromptCriterionAssessment {
            criterion_index: 0,
            criterion,
            rating: PromptCriterionRating::Unknown,
            verified_claim_id: None,
        }],
        idempotency_key: "evidence-first-trial".to_owned(),
    };
    hub.record_prompt_trial(&reported_trial).unwrap();
    let comparison_request = PromptComparisonRequest {
        workspace: workspace.to_string_lossy().into_owned(),
        task_id,
    };
    let comparison = hub.compare_prompt_trials(&comparison_request).unwrap();
    assert!(comparison.advisory);
    assert!(!comparison.attribution_verified);
    assert_eq!(comparison.rating_provenance, "host_reported");
    assert_eq!(comparison.variants.len(), 2);
    let baseline_summary = comparison
        .variants
        .iter()
        .find(|item| item.template_id == "aporic.task_brief")
        .unwrap();
    assert_eq!(baseline_summary.reported_met, 1);
    assert_eq!(
        baseline_summary.template_version,
        baseline.receipt.template_version
    );
    assert_eq!(
        baseline_summary.template_sha256,
        baseline.receipt.template_sha256
    );
    assert_eq!(baseline_summary.associated_verified_claims, 1);
    assert_eq!(baseline_summary.response_files_verified, 1);
    let alternative_summary = comparison
        .variants
        .iter()
        .find(|item| item.template_id == "aporic.task_brief.evidence_first")
        .unwrap();
    assert_eq!(alternative_summary.reported_unknown, 1);
    assert_eq!(alternative_summary.associated_verified_claims, 0);

    let restarted = Hub::open(&database).unwrap();
    assert_eq!(restarted.stats().unwrap().schema_version, 25);
    assert_eq!(
        restarted
            .compare_prompt_trials(&comparison_request)
            .unwrap(),
        comparison
    );
    let export = restarted
        .export_project(workspace.to_string_lossy().as_ref())
        .unwrap();
    assert_eq!(export.prompt_trials.len(), 2);
    assert_eq!(export.format_version, 21);
    let serialized = serde_json::to_string(&export).unwrap();
    assert!(!serialized.contains("RAW-SECRET-RESPONSE-CONTENT"));
}

#[test]
fn trial_rejects_forged_verification_and_wrong_workspace() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    let other = area.path().join("other");
    std::fs::create_dir(&workspace).unwrap();
    std::fs::create_dir(&other).unwrap();
    let hub = Hub::open(area.path().join("db.sqlite3")).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Inspect prompt trial validation".to_owned(),
            idempotency_key: "validation-session".to_owned(),
        })
        .unwrap()
        .session_id;
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id,
            objective: "Inspect prompt trial validation".to_owned(),
            acceptance_criteria: vec!["Criterion A".to_owned()],
            write_scope: Vec::new(),
            depends_on: Vec::new(),
            idempotency_key: "validation-task".to_owned(),
        })
        .unwrap()
        .task
        .task_id;
    let receipt = hub
        .task_brief(&TaskBriefRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            task_id,
            max_context_bytes: None,
            variant: None,
            idempotency_key: "validation-brief".to_owned(),
        })
        .unwrap()
        .receipt;
    let mut trial = PromptTrialRequest {
        workspace: workspace.to_string_lossy().into_owned(),
        brief_receipt_id: receipt.receipt_id,
        response_sha256: "a".repeat(64),
        response_evidence_id: None,
        reported_model: None,
        assessments: vec![PromptCriterionAssessment {
            criterion_index: 0,
            criterion: "Criterion A".to_owned(),
            rating: PromptCriterionRating::Met,
            verified_claim_id: Some("not-a-claim".to_owned()),
        }],
        idempotency_key: "validation-trial".to_owned(),
    };
    assert!(hub.record_prompt_trial(&trial).is_err());
    trial.assessments[0].verified_claim_id = None;
    trial.workspace = other.to_string_lossy().into_owned();
    assert!(hub.record_prompt_trial(&trial).is_err());
    trial.workspace = workspace.to_string_lossy().into_owned();
    trial.assessments[0].criterion = "Different criterion".to_owned();
    assert!(hub.record_prompt_trial(&trial).is_err());
}

#[test]
fn comparison_keeps_template_versions_and_digests_separate() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let database = area.path().join("db.sqlite3");
    let hub = Hub::open(&database).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Compare template revisions".to_owned(),
            idempotency_key: "version-session".to_owned(),
        })
        .unwrap()
        .session_id;
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id,
            objective: "Compare template revisions".to_owned(),
            acceptance_criteria: vec!["Reviewed".to_owned()],
            write_scope: Vec::new(),
            depends_on: Vec::new(),
            idempotency_key: "version-task".to_owned(),
        })
        .unwrap()
        .task
        .task_id;
    let first = hub
        .task_brief(&TaskBriefRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            task_id: task_id.clone(),
            max_context_bytes: None,
            variant: Some("baseline".to_owned()),
            idempotency_key: "version-brief".to_owned(),
        })
        .unwrap();
    // Simulate a future immutable template revision without changing today's built-in template.
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute(
            "INSERT INTO task_brief_receipts
             (receipt_id, project_id, task_id, idempotency_key, template_id,
              template_version, template_sha256, context_policy_sha256,
              selected_item_ids_json, brief_sha256, brief_bytes, created_at_unix_ms)
             SELECT 'future-brief', project_id, task_id, 'future-brief-key', template_id,
                    2, ?1, context_policy_sha256, selected_item_ids_json,
                    brief_sha256, brief_bytes, created_at_unix_ms
             FROM task_brief_receipts WHERE receipt_id = ?2",
            rusqlite::params!["c".repeat(64), first.receipt.receipt_id],
        )
        .unwrap();
    for (receipt_id, idempotency_key) in [
        (first.receipt.receipt_id, "first-trial"),
        ("future-brief".to_owned(), "future-trial"),
    ] {
        hub.record_prompt_trial(&PromptTrialRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            brief_receipt_id: receipt_id,
            response_sha256: "a".repeat(64),
            response_evidence_id: None,
            reported_model: None,
            assessments: vec![PromptCriterionAssessment {
                criterion_index: 0,
                criterion: "Reviewed".to_owned(),
                rating: PromptCriterionRating::Unknown,
                verified_claim_id: None,
            }],
            idempotency_key: idempotency_key.to_owned(),
        })
        .unwrap();
    }
    let comparison = hub
        .compare_prompt_trials(&PromptComparisonRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            task_id,
        })
        .unwrap();
    assert_eq!(comparison.variants.len(), 2);
    assert_eq!(comparison.variants[0].template_version, 1);
    assert_eq!(comparison.variants[1].template_version, 2);
    assert_eq!(comparison.variants[0].trial_count, 1);
    assert_eq!(comparison.variants[1].trial_count, 1);
    assert_ne!(
        comparison.variants[0].template_sha256,
        comparison.variants[1].template_sha256
    );
}

#[test]
fn trial_accepts_more_than_thirty_two_criteria_and_rejects_duplicate_indices() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let hub = Hub::open(area.path().join("db.sqlite3")).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Assess all criteria".to_owned(),
            idempotency_key: "many-session".to_owned(),
        })
        .unwrap()
        .session_id;
    let criteria = (0..33)
        .map(|index| format!("Criterion {index}"))
        .collect::<Vec<_>>();
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id,
            objective: "Assess all criteria".to_owned(),
            acceptance_criteria: criteria.clone(),
            write_scope: Vec::new(),
            depends_on: Vec::new(),
            idempotency_key: "many-task".to_owned(),
        })
        .unwrap()
        .task
        .task_id;
    let receipt = hub
        .task_brief(&TaskBriefRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            task_id,
            max_context_bytes: None,
            variant: None,
            idempotency_key: "many-brief".to_owned(),
        })
        .unwrap();
    let mut request = PromptTrialRequest {
        workspace: workspace.to_string_lossy().into_owned(),
        brief_receipt_id: receipt.receipt.receipt_id,
        response_sha256: "a".repeat(64),
        response_evidence_id: None,
        reported_model: None,
        assessments: criteria
            .into_iter()
            .enumerate()
            .map(|(index, criterion)| PromptCriterionAssessment {
                criterion_index: index as u32,
                criterion,
                rating: PromptCriterionRating::Unknown,
                verified_claim_id: None,
            })
            .collect(),
        idempotency_key: "many-trial".to_owned(),
    };
    request.assessments[1].criterion_index = 0;
    assert!(hub.record_prompt_trial(&request).is_err());
    request.assessments[1].criterion_index = 1;
    let trial = hub.record_prompt_trial(&request).unwrap();
    assert_eq!(trial.trial.assessments.len(), 33);
}
