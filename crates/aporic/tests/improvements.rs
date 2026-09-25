use aporic::{Hub, domain::*};

#[test]
fn cross_workspace_intake_is_queued_scoped_and_idempotent() {
    let area = tempfile::tempdir().unwrap();
    let core = area.path().join("aporic");
    let client = area.path().join("client");
    std::fs::create_dir_all(core.join("crates/aporic")).unwrap();
    std::fs::write(
        core.join("crates/aporic/Cargo.toml"),
        "[package]\nname='aporic'",
    )
    .unwrap();
    std::fs::create_dir(&client).unwrap();
    let db = area.path().join("db.sqlite3");
    let hub = Hub::open(&db).unwrap();
    let source = hub
        .open_session(&OpenRequest {
            workspace: client.to_string_lossy().into(),
            objective: "client work".into(),
            idempotency_key: "open-client".into(),
        })
        .unwrap();
    let evidence = hub
        .add_evidence(&EvidenceRequest {
            session_id: source.session_id.clone(),
            kind: EvidenceKind::UserStatement,
            locator: "feedback summary".into(),
            summary: "Client requested a traceable intake".into(),
            content_sha256: Some("a".repeat(64)),
            idempotency_key: "source-evidence".into(),
        })
        .unwrap();
    let input = ImprovementSubmitRequest {
        source_session_id: source.session_id,
        source_evidence_id: evidence.evidence.evidence_id,
        core_workspace: core.to_string_lossy().into(),
        objective: "Add an intake route".into(),
        acceptance_criteria: vec!["Source can read status".into()],
        idempotency_key: "improvement-one".into(),
    };
    let created = hub.submit_improvement(&input).unwrap();
    assert_eq!(created.request.status, TaskStatus::Queued);
    assert_eq!(created.request.assigned_worker, None);
    assert!(created.request.registration_only);
    assert!(hub.submit_improvement(&input).unwrap().duplicate);
    let mut same = input.clone();
    same.idempotency_key = "another-key".into();
    assert!(hub.submit_improvement(&same).unwrap().possible_duplicate);
    let restarted = Hub::open(&db).unwrap();
    let from_client = restarted
        .list_improvements(&ImprovementListRequest {
            workspace: client.to_string_lossy().into(),
            limit: None,
        })
        .unwrap();
    let from_core = restarted
        .list_improvements(&ImprovementListRequest {
            workspace: core.to_string_lossy().into(),
            limit: None,
        })
        .unwrap();
    assert_eq!(from_client, from_core);
    assert_eq!(from_core[0].task_id, created.request.task_id);
    let exported = restarted.export_project(core.to_str().unwrap()).unwrap();
    assert_eq!(exported.improvement_requests.len(), 1);
    assert!(
        restarted
            .list_tasks(&TaskListRequest {
                workspace: core.to_string_lossy().into(),
                limit: None
            })
            .unwrap()
            .iter()
            .any(|task| task.task_id == created.request.task_id)
    );
    let mut bad = input;
    bad.objective = "Bearer secret".into();
    bad.idempotency_key = "bad-intake".into();
    assert!(hub.submit_improvement(&bad).is_err());
}

#[test]
fn prototype_review_keeps_user_value_unknown_without_playtest_report() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("game");
    std::fs::create_dir(&workspace).unwrap();
    let db = area.path().join("db.sqlite3");
    let hub = Hub::open(&db).unwrap();
    let session = hub
        .open_session(&OpenRequest {
            workspace: workspace.to_string_lossy().into(),
            objective: "The Divine Paradox prototype".into(),
            idempotency_key: "open-game".into(),
        })
        .unwrap();
    let task = hub
        .create_task(&TaskCreateRequest {
            session_id: session.session_id.clone(),
            objective: "Build document comparison game".into(),
            acceptance_criteria: vec!["Playtest".into()],
            write_scope: vec![],
            depends_on: vec![],
            idempotency_key: "task-game".into(),
        })
        .unwrap();
    let brief = hub
        .create_prototype_brief(&PrototypeBriefCreateRequest {
            workspace: workspace.to_string_lossy().into(),
            task_id: task.task.task_id.clone(),
            target_user: "Players who compare conflicting records".into(),
            core_experience_hypothesis: "Comparing two documents creates a meaningful choice"
                .into(),
            fidelity: "playable prototype".into(),
            reuse_boundary: "throwaway".into(),
            technology_stack: "Rust and browser UI".into(),
            repository_boundary: "game workspace".into(),
            validation_method: "Observed player session".into(),
            idempotency_key: "brief-game".into(),
        })
        .unwrap();
    assert!(brief.brief.advisory);
    let legacy = workspace.join("legacy-test.log");
    std::fs::write(&legacy, "tests pass").unwrap();
    let smoke = hub
        .add_evidence(&EvidenceRequest {
            session_id: session.session_id.clone(),
            kind: EvidenceKind::WorkspaceFile,
            locator: legacy.to_string_lossy().into(),
            summary: "Unrelated legacy tests".into(),
            content_sha256: None,
            idempotency_key: "legacy-test".into(),
        })
        .unwrap();
    let first = hub
        .review_prototype(&PrototypeReviewRequest {
            workspace: workspace.to_string_lossy().into(),
            task_id: task.task.task_id.clone(),
            smoke_evidence_ids: vec![smoke.evidence.evidence_id.clone()],
            rule_evidence_ids: vec![],
            user_play_evidence_ids: vec![smoke.evidence.evidence_id],
            idempotency_key: "review-one".into(),
        })
        .unwrap();
    assert_eq!(first.review.smoke, PrototypeEvidenceState::Observed);
    assert_eq!(first.review.rules, PrototypeEvidenceState::Unknown);
    assert_eq!(first.review.user_value, PrototypeEvidenceState::Unknown);
    let report = workspace.join("session.playtest.json");
    std::fs::write(&report, r#"{"target_user":"player","participant_count":1,"observed_behavior":"compared documents","session_date":"2026-09-26"}"#).unwrap();
    let play = hub
        .add_evidence(&EvidenceRequest {
            session_id: session.session_id,
            kind: EvidenceKind::WorkspaceFile,
            locator: report.to_string_lossy().into(),
            summary: "Observed playtest report".into(),
            content_sha256: None,
            idempotency_key: "playtest-report".into(),
        })
        .unwrap();
    let second = hub
        .review_prototype(&PrototypeReviewRequest {
            workspace: workspace.to_string_lossy().into(),
            task_id: task.task.task_id.clone(),
            smoke_evidence_ids: vec![],
            rule_evidence_ids: vec![],
            user_play_evidence_ids: vec![play.evidence.evidence_id],
            idempotency_key: "review-two".into(),
        })
        .unwrap();
    assert_eq!(second.review.user_value, PrototypeEvidenceState::Observed);
    let restarted = Hub::open(&db).unwrap();
    let status = restarted
        .get_prototype(&PrototypeGetRequest {
            workspace: workspace.to_string_lossy().into(),
            task_id: task.task.task_id,
        })
        .unwrap()
        .unwrap();
    assert_eq!(
        status.latest_review.unwrap().user_value,
        PrototypeEvidenceState::Observed
    );
    let exported = restarted
        .export_project(workspace.to_str().unwrap())
        .unwrap();
    assert_eq!(exported.prototype_briefs.len(), 1);
    assert_eq!(exported.prototype_reviews.len(), 2);
}

#[test]
fn discovery_is_opt_in_bounded_and_metadata_only() {
    let area = tempfile::tempdir().unwrap();
    let current = area.path().join("current");
    let history = area.path().join("sacho-chronicles");
    let irrelevant = area.path().join("other-game");
    for path in [&current, &history, &irrelevant] {
        std::fs::create_dir(path).unwrap();
        std::fs::create_dir(path.join(".git")).unwrap();
    }
    let outside = tempfile::tempdir().unwrap();
    let hub = Hub::open(area.path().join("db.sqlite3")).unwrap();
    let request = RelatedWorkspaceRequest {
        workspace: current.to_string_lossy().into(),
        concept: "The Divine Paradox".into(),
        roots: vec![area.path().to_string_lossy().into()],
        hints: vec![RelatedWorkspaceHint {
            path: history.to_string_lossy().into(),
            labels: vec!["Divine Paradox prior game".into()],
        }],
        limit: Some(10),
    };
    assert!(
        hub.related_workspaces(&RelatedWorkspaceRequest {
            roots: vec![],
            ..request.clone()
        })
        .unwrap()
        .is_empty()
    );
    let found = hub.related_workspaces(&request).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].path,
        std::fs::canonicalize(&history).unwrap().to_string_lossy()
    );
    assert!(found[0].historical_context_only);
    assert!(found[0].modified_at_unix_ms.is_some());
    let mut bad = request;
    bad.hints.push(RelatedWorkspaceHint {
        path: outside.path().to_string_lossy().into(),
        labels: vec!["Divine".into()],
    });
    assert!(hub.related_workspaces(&bad).is_err());
    let mut missing = bad;
    missing.hints.pop();
    missing
        .roots
        .push(outside.path().join("missing").to_string_lossy().into());
    assert_eq!(hub.related_workspaces(&missing).unwrap().len(), 1);
}

#[test]
fn upgrades_schema_19_to_20() {
    let area = tempfile::tempdir().unwrap();
    let database = area.path().join("db.sqlite3");
    drop(Hub::open(&database).unwrap());
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute_batch(
            "DROP TABLE prototype_reviews;
        DROP TABLE prototype_briefs; DROP TABLE improvement_requests;
        PRAGMA user_version = 19;",
        )
        .unwrap();
    drop(connection);
    let hub = Hub::open(&database).unwrap();
    assert_eq!(hub.stats().unwrap().schema_version, 20);
}
