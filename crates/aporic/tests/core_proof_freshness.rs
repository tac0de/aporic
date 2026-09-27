use aporic::{
    Hub,
    domain::{
        ClaimRequest, ClaimStatus, CriterionProof, EvidenceKind, EvidenceRequest, OpenRequest,
        TaskClaimRequest, TaskCompleteRequest, TaskCreateRequest, workspace_file_claim,
    },
};
use sha2::{Digest, Sha256};

#[test]
fn task_completion_rejects_changed_file_and_superseded_claim() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let file = workspace.join("proof.txt");
    std::fs::write(&file, "first version").unwrap();
    let locator = file.to_string_lossy().into_owned();
    let digest = format!("{:x}", Sha256::digest(b"first version"));
    let criterion = workspace_file_claim(&locator, &digest);
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Check proof freshness".into(),
            idempotency_key: "open".into(),
        })
        .unwrap()
        .session_id;
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id: session_id.clone(),
            objective: "Produce verified proof file".into(),
            acceptance_criteria: vec![criterion.clone()],
            write_scope: vec![],
            depends_on: vec![],
            idempotency_key: "task".into(),
        })
        .unwrap()
        .task
        .task_id;
    hub.claim_task(&TaskClaimRequest {
        task_id: task_id.clone(),
        worker_id: "worker".into(),
        lease_seconds: 300,
        idempotency_key: "lease".into(),
    })
    .unwrap();
    let evidence_id = hub
        .add_evidence(&EvidenceRequest {
            session_id: session_id.clone(),
            kind: EvidenceKind::WorkspaceFile,
            locator: locator.clone(),
            summary: "Read back proof file".into(),
            content_sha256: None,
            idempotency_key: "evidence".into(),
        })
        .unwrap()
        .evidence
        .evidence_id;
    let first_claim = hub
        .assert_claim(&ClaimRequest {
            session_id: session_id.clone(),
            status: ClaimStatus::Verified,
            statement: criterion.clone(),
            material: true,
            evidence_ids: vec![evidence_id.clone()],
            subject_key: None,
            supersedes_claim_id: None,
            idempotency_key: "claim-first".into(),
        })
        .unwrap()
        .claim
        .claim_id;
    let complete_with = |claim_id: String, key: &str| {
        hub.complete_task(&TaskCompleteRequest {
            task_id: task_id.clone(),
            worker_id: "worker".into(),
            outcome_summary: "Proof accepted".into(),
            criterion_proofs: vec![CriterionProof {
                criterion: criterion.clone(),
                verified_claim_id: claim_id,
            }],
            idempotency_key: key.into(),
        })
    };

    std::fs::write(&file, "changed version").unwrap();
    assert!(complete_with(first_claim.clone(), "changed-file").is_err());

    std::fs::write(&file, "first version").unwrap();
    let replacement_claim = hub
        .assert_claim(&ClaimRequest {
            session_id,
            status: ClaimStatus::Verified,
            statement: criterion.clone(),
            material: true,
            evidence_ids: vec![evidence_id],
            subject_key: None,
            supersedes_claim_id: Some(first_claim.clone()),
            idempotency_key: "claim-replacement".into(),
        })
        .unwrap()
        .claim
        .claim_id;
    assert!(complete_with(first_claim, "superseded-claim").is_err());
    complete_with(replacement_claim, "active-claim").unwrap();
}

#[cfg(unix)]
#[test]
fn task_completion_rejects_retargeted_proof_even_with_matching_bytes() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let first = workspace.join("first.txt");
    let second = workspace.join("second.txt");
    let link = workspace.join("proof.txt");
    std::fs::write(&first, "same bytes").unwrap();
    std::fs::write(&second, "same bytes").unwrap();
    std::os::unix::fs::symlink(&first, &link).unwrap();
    let locator = link.to_string_lossy().into_owned();
    let digest = format!("{:x}", Sha256::digest(b"same bytes"));
    let criterion = workspace_file_claim(&locator, &digest);
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Reject moved proof identity".into(),
            idempotency_key: "identity-open".into(),
        })
        .unwrap()
        .session_id;
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id: session_id.clone(),
            objective: "Verify original file".into(),
            acceptance_criteria: vec![criterion.clone()],
            write_scope: vec![],
            depends_on: vec![],
            idempotency_key: "identity-task".into(),
        })
        .unwrap()
        .task
        .task_id;
    hub.claim_task(&TaskClaimRequest {
        task_id: task_id.clone(),
        worker_id: "worker".into(),
        lease_seconds: 300,
        idempotency_key: "identity-lease".into(),
    })
    .unwrap();
    let evidence = hub
        .add_evidence(&EvidenceRequest {
            session_id: session_id.clone(),
            kind: EvidenceKind::WorkspaceFile,
            locator,
            summary: "Observed first file through link".into(),
            content_sha256: None,
            idempotency_key: "identity-evidence".into(),
        })
        .unwrap()
        .evidence;
    let subject_key = format!("file:{}", evidence.canonical_locator.as_ref().unwrap());
    let claim_id = hub
        .assert_claim(&ClaimRequest {
            session_id,
            status: ClaimStatus::Verified,
            statement: criterion.clone(),
            material: true,
            evidence_ids: vec![evidence.evidence_id],
            subject_key: Some(subject_key),
            supersedes_claim_id: None,
            idempotency_key: "identity-claim".into(),
        })
        .unwrap()
        .claim
        .claim_id;
    std::fs::write(&first, "changed bytes").unwrap();
    std::fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink(&second, &link).unwrap();
    let completion = hub.complete_task(&TaskCompleteRequest {
        task_id,
        worker_id: "worker".into(),
        outcome_summary: "Wrong file must not be accepted".into(),
        criterion_proofs: vec![CriterionProof {
            criterion,
            verified_claim_id: claim_id,
        }],
        idempotency_key: "identity-complete".into(),
    });
    assert!(completion.is_err());
}
