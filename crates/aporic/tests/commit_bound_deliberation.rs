use std::{path::Path, process::Command};

use aporic::{
    Hub,
    domain::{
        DeliberationCreateRequest, DeliberationDecisionRequest, DeliberationEdgeKind,
        DeliberationGetRequest, DeliberationListRequest, DeliberationNodeAddRequest,
        DeliberationNodeKind, DeliberationRelationRequest, EvidenceKind, EvidenceRequest,
        GitObserveRequest, OpenRequest,
    },
};
use rusqlite::Connection;

fn git(workspace: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(workspace)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?}: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn commit(workspace: &Path, message: &str) {
    git(workspace, &["add", "."]);
    git(
        workspace,
        &[
            "-c",
            "user.name=Aporic Test",
            "-c",
            "user.email=aporic@example.invalid",
            "commit",
            "-m",
            message,
        ],
    );
}

fn fixture() -> (tempfile::TempDir, std::path::PathBuf, Hub, String) {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    git(&workspace, &["init", "-b", "main"]);
    std::fs::write(workspace.join("design.md"), "initial\n").unwrap();
    commit(&workspace, "initial");
    let hub = Hub::open(area.path().join("aporic.sqlite3")).unwrap();
    let session = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into_owned(),
            objective: "Deliberate against immutable repository evidence".to_owned(),
            idempotency_key: "open-deliberation-test".to_owned(),
        })
        .unwrap()
        .session_id;
    (area, workspace, hub, session)
}

#[test]
fn migrates_v10_to_v11_without_deliberations() {
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
    connection
        .execute_batch(include_str!(
            "../../../migrations/0010_token_efficiency.sql"
        ))
        .unwrap();
    drop(connection);

    let hub = Hub::open(database).unwrap();
    assert_eq!(hub.stats().unwrap().schema_version, 12);
    let audit = hub.audit_deliberations().unwrap();
    assert_eq!(audit.deliberation_count, 0);
    assert!(audit.consistent);
}

#[test]
fn rejects_binding_to_a_corrupted_git_snapshot() {
    let (area, workspace, hub, _session_id) = fixture();
    let workspace_text = workspace.to_string_lossy().into_owned();
    let snapshot = hub
        .observe_git(&GitObserveRequest {
            workspace: workspace_text.clone(),
            base_ref: None,
        })
        .unwrap();
    std::fs::write(workspace.join("dirty.txt"), "not committed\n").unwrap();
    let dirty = hub
        .observe_git(&GitObserveRequest {
            workspace: workspace_text.clone(),
            base_ref: None,
        })
        .unwrap();
    assert!(
        hub.create_deliberation(&DeliberationCreateRequest {
            workspace: workspace_text.clone(),
            title: "Dirty evidence".to_owned(),
            question: "Should uncommitted state seed a decision?".to_owned(),
            git_snapshot_id: dirty.snapshot_id,
            idempotency_key: "reject-dirty-snapshot".to_owned(),
        })
        .unwrap_err()
        .to_string()
        .contains("clean committed Git snapshot")
    );
    Connection::open(area.path().join("aporic.sqlite3"))
        .unwrap()
        .execute(
            "UPDATE git_snapshots SET head_tree = 'tampered' WHERE snapshot_id = ?1",
            [&snapshot.snapshot_id],
        )
        .unwrap();
    assert!(
        hub.create_deliberation(&DeliberationCreateRequest {
            workspace: workspace_text,
            title: "Corrupted evidence".to_owned(),
            question: "Should corrupted evidence seed a decision?".to_owned(),
            git_snapshot_id: snapshot.snapshot_id,
            idempotency_key: "reject-corrupt-snapshot".to_owned(),
        })
        .unwrap_err()
        .to_string()
        .contains("digest mismatch")
    );
}

#[test]
fn public_graph_preserves_material_aporia_and_detects_git_staleness() {
    let (area, workspace, hub, session_id) = fixture();
    let workspace_text = workspace.to_string_lossy().into_owned();
    let snapshot = hub
        .observe_git(&GitObserveRequest {
            workspace: workspace_text.clone(),
            base_ref: None,
        })
        .unwrap();
    let request = DeliberationCreateRequest {
        workspace: workspace_text.clone(),
        title: "Storage choice".to_owned(),
        question: "Should the append-only graph use the existing SQLite store?".to_owned(),
        git_snapshot_id: snapshot.snapshot_id,
        idempotency_key: "create-storage-deliberation".to_owned(),
    };
    let created = hub.create_deliberation(&request).unwrap();
    let duplicate = hub.create_deliberation(&request).unwrap();
    assert!(!created.duplicate);
    assert!(duplicate.duplicate);
    assert_eq!(created.graph.deliberation, duplicate.graph.deliberation);
    assert_eq!(created.graph.nodes.len(), 1);
    let question_id = created.graph.nodes[0].node_id.clone();

    let proposal = hub
        .add_deliberation_node(&DeliberationNodeAddRequest {
            workspace: workspace_text.clone(),
            deliberation_id: created.graph.deliberation.deliberation_id.clone(),
            kind: DeliberationNodeKind::Proposal,
            statement: "Reuse SQLite and bind every record to its graph.".to_owned(),
            material: true,
            evidence_id: None,
            claim_id: None,
            relations: vec![DeliberationRelationRequest {
                target_node_id: question_id,
                kind: DeliberationEdgeKind::Support,
            }],
            idempotency_key: "add-proposal".to_owned(),
        })
        .unwrap();
    let proposal_id = proposal.graph.nodes.last().unwrap().node_id.clone();

    let unsupported = hub.add_deliberation_node(&DeliberationNodeAddRequest {
        workspace: workspace_text.clone(),
        deliberation_id: created.graph.deliberation.deliberation_id.clone(),
        kind: DeliberationNodeKind::Objection,
        statement: "A model asserted that another database is safer.".to_owned(),
        material: true,
        evidence_id: None,
        claim_id: None,
        relations: vec![],
        idempotency_key: "unsupported-objection".to_owned(),
    });
    assert!(
        unsupported
            .unwrap_err()
            .to_string()
            .contains("direct evidence")
    );

    let evidence_path = workspace.join("risk.txt");
    std::fs::write(
        &evidence_path,
        "migration rollback has not been exercised\n",
    )
    .unwrap();
    let evidence = hub
        .add_evidence(&EvidenceRequest {
            session_id: session_id.clone(),
            kind: EvidenceKind::WorkspaceFile,
            locator: evidence_path.to_string_lossy().into_owned(),
            summary: "Direct local rollback gap".to_owned(),
            content_sha256: None,
            idempotency_key: "rollback-evidence".to_owned(),
        })
        .unwrap()
        .evidence;
    let challenged = hub
        .add_deliberation_node(&DeliberationNodeAddRequest {
            workspace: workspace_text.clone(),
            deliberation_id: created.graph.deliberation.deliberation_id.clone(),
            kind: DeliberationNodeKind::MaterialUnknown,
            statement: "Rollback behavior remains unverified.".to_owned(),
            material: true,
            evidence_id: Some(evidence.evidence_id),
            claim_id: None,
            relations: vec![DeliberationRelationRequest {
                target_node_id: proposal_id.clone(),
                kind: DeliberationEdgeKind::Attack,
            }],
            idempotency_key: "add-material-unknown".to_owned(),
        })
        .unwrap();
    let unknown_id = challenged.graph.nodes.last().unwrap().node_id.clone();
    assert_eq!(
        challenged.graph.open_material_node_ids,
        vec![unknown_id.clone()]
    );

    let with_noise = hub
        .add_deliberation_node(&DeliberationNodeAddRequest {
            workspace: workspace_text.clone(),
            deliberation_id: created.graph.deliberation.deliberation_id.clone(),
            kind: DeliberationNodeKind::Objection,
            statement: "A different color palette might be nicer.".to_owned(),
            material: false,
            evidence_id: None,
            claim_id: None,
            relations: vec![DeliberationRelationRequest {
                target_node_id: proposal_id.clone(),
                kind: DeliberationEdgeKind::Attack,
            }],
            idempotency_key: "add-non-material-objection".to_owned(),
        })
        .unwrap();
    assert_eq!(
        with_noise.graph.open_material_node_ids,
        vec![unknown_id.clone()]
    );

    let decided = hub
        .record_deliberation_decision(&DeliberationDecisionRequest {
            workspace: workspace_text.clone(),
            deliberation_id: created.graph.deliberation.deliberation_id.clone(),
            proposal_node_id: proposal_id.clone(),
            summary: "Proceed provisionally while retaining the rollback unknown.".to_owned(),
            idempotency_key: "provisional-decision".to_owned(),
        })
        .unwrap();
    assert_eq!(decided.graph.decisions[0].open_material_issues, 1);
    assert!(decided.graph.provisional_only);
    assert!(!decided.graph.approval_proven);

    let narrative_resolution = hub.add_deliberation_node(&DeliberationNodeAddRequest {
        workspace: workspace_text.clone(),
        deliberation_id: created.graph.deliberation.deliberation_id.clone(),
        kind: DeliberationNodeKind::Revision,
        statement: "Assume the rollback concern is resolved.".to_owned(),
        material: true,
        evidence_id: None,
        claim_id: None,
        relations: vec![DeliberationRelationRequest {
            target_node_id: unknown_id.clone(),
            kind: DeliberationEdgeKind::Revision,
        }],
        idempotency_key: "narrative-resolution".to_owned(),
    });
    assert!(
        narrative_resolution
            .unwrap_err()
            .to_string()
            .contains("cannot be closed by narrative revision")
    );

    let revised = hub
        .add_deliberation_node(&DeliberationNodeAddRequest {
            workspace: workspace_text.clone(),
            deliberation_id: created.graph.deliberation.deliberation_id.clone(),
            kind: DeliberationNodeKind::Revision,
            statement: "Require an automated rollback exercise before release.".to_owned(),
            material: true,
            evidence_id: None,
            claim_id: None,
            relations: vec![DeliberationRelationRequest {
                target_node_id: proposal_id.clone(),
                kind: DeliberationEdgeKind::Revision,
            }],
            idempotency_key: "address-rollback-unknown".to_owned(),
        })
        .unwrap();
    assert_eq!(
        revised.graph.open_material_node_ids,
        vec![unknown_id.clone()]
    );

    let result_path = workspace.join("rollback-result.txt");
    std::fs::write(&result_path, "rollback exercise passed\n").unwrap();
    let result_evidence = hub
        .add_evidence(&EvidenceRequest {
            session_id,
            kind: EvidenceKind::WorkspaceFile,
            locator: result_path.to_string_lossy().into_owned(),
            summary: "Direct rollback result".to_owned(),
            content_sha256: None,
            idempotency_key: "rollback-result-evidence".to_owned(),
        })
        .unwrap()
        .evidence;
    let resolved = hub
        .add_deliberation_node(&DeliberationNodeAddRequest {
            workspace: workspace_text.clone(),
            deliberation_id: created.graph.deliberation.deliberation_id.clone(),
            kind: DeliberationNodeKind::Claim,
            statement: "The automated rollback exercise passed.".to_owned(),
            material: true,
            evidence_id: Some(result_evidence.evidence_id),
            claim_id: None,
            relations: vec![DeliberationRelationRequest {
                target_node_id: unknown_id,
                kind: DeliberationEdgeKind::Undercut,
            }],
            idempotency_key: "resolve-rollback-unknown".to_owned(),
        })
        .unwrap();
    assert!(resolved.graph.open_material_node_ids.is_empty());

    std::fs::write(workspace.join("design.md"), "changed\n").unwrap();
    commit(&workspace, "change design");
    hub.observe_git(&GitObserveRequest {
        workspace: workspace_text.clone(),
        base_ref: None,
    })
    .unwrap();
    let stale = hub
        .get_deliberation(&DeliberationGetRequest {
            workspace: workspace_text.clone(),
            deliberation_id: created.graph.deliberation.deliberation_id.clone(),
            node_after_sequence: None,
            edge_after_sequence: None,
            decision_after_sequence: None,
            limit: None,
        })
        .unwrap();
    assert!(stale.stale);
    assert!(!stale.truncated);
    assert_eq!(stale.next_node_after_sequence, None);
    assert_eq!(stale.next_edge_after_sequence, None);
    assert_eq!(stale.next_decision_after_sequence, None);
    assert_eq!(stale.total_node_count, 6);
    assert_eq!(stale.total_edge_count, 5);
    assert_eq!(stale.total_decision_count, 1);
    assert_eq!(stale.total_open_material_issue_count, 0);
    assert!(!stale.open_material_issues_truncated);
    assert!(
        hub.record_deliberation_decision(&DeliberationDecisionRequest {
            workspace: workspace_text.clone(),
            deliberation_id: created.graph.deliberation.deliberation_id.clone(),
            proposal_node_id: proposal_id.clone(),
            summary: "This stale decision must be rejected.".to_owned(),
            idempotency_key: "reject-stale-decision".to_owned(),
        })
        .unwrap_err()
        .to_string()
        .contains("stale deliberation")
    );
    let summaries = hub
        .list_deliberations(&DeliberationListRequest {
            workspace: workspace_text.clone(),
            limit: Some(10),
        })
        .unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].node_count, 6);
    assert_eq!(summaries[0].open_material_issue_count, 0);
    assert!(summaries[0].stale);
    let first_page = hub
        .get_deliberation(&DeliberationGetRequest {
            workspace: workspace_text.clone(),
            deliberation_id: created.graph.deliberation.deliberation_id.clone(),
            node_after_sequence: None,
            edge_after_sequence: None,
            decision_after_sequence: None,
            limit: Some(1),
        })
        .unwrap();
    assert!(first_page.truncated);
    assert_eq!(first_page.nodes.len(), 1);
    assert_eq!(first_page.edges.len(), 1);
    assert_eq!(first_page.decisions.len(), 1);
    assert!(first_page.next_node_after_sequence.is_some());
    assert!(first_page.next_edge_after_sequence.is_some());
    assert_eq!(first_page.next_decision_after_sequence, None);
    let second_node_page = hub
        .get_deliberation(&DeliberationGetRequest {
            workspace: workspace_text.clone(),
            deliberation_id: created.graph.deliberation.deliberation_id.clone(),
            node_after_sequence: first_page.next_node_after_sequence,
            edge_after_sequence: None,
            decision_after_sequence: None,
            limit: Some(1),
        })
        .unwrap();
    assert_ne!(
        second_node_page.nodes[0].node_id,
        first_page.nodes[0].node_id
    );
    assert!(hub.audit_deliberations().unwrap().consistent);
    let export = hub.export_project(&workspace_text).unwrap();
    assert_eq!(export.deliberations.len(), 1);
    assert_eq!(export.deliberation_nodes.len(), 6);
    assert_eq!(export.deliberation_edges.len(), 5);
    assert_eq!(export.deliberation_decisions.len(), 1);

    Connection::open(area.path().join("aporic.sqlite3"))
        .unwrap()
        .execute(
            "UPDATE deliberation_nodes SET statement = 'tampered' WHERE node_id = ?1",
            [&proposal_id],
        )
        .unwrap();
    assert!(!hub.audit_deliberations().unwrap().consistent);
}

#[test]
fn offline_deliberation_simulation_is_bounded_and_non_authoritative() {
    let report = aporic::eval::simulate_deliberation();
    assert_eq!(
        report.material_challenges_expected,
        report.material_challenges_preserved
    );
    assert_eq!(report.unsupported_material_challenges_rejected, 1);
    assert_eq!(report.irrelevant_objections_blocking, 0);
    assert_eq!(report.stale_decisions_detected, 1);
    assert_eq!(report.approvals_issued, 0);
    assert_eq!(report.hidden_reasoning_fields, 0);
    assert!(report.deterministic);
    assert_eq!(report.network_or_model_calls, 0);
    assert!(
        serde_json::from_value::<DeliberationNodeAddRequest>(serde_json::json!({
            "workspace": "/tmp/project",
            "deliberation_id": "graph",
            "kind": "claim",
            "statement": "public conclusion",
            "material": false,
            "relations": [],
            "hidden_chain_of_thought": "must not be accepted",
            "idempotency_key": "reject-private-reasoning"
        }))
        .is_err()
    );
}
