use aporic::{
    Hub,
    domain::{
        HostResearchObservation, OpenRequest, TaskCreateRequest, TaskResearchAttachRequest,
        TaskResearchListRequest,
    },
    research::FetchedDocument,
    store::Store,
};

fn setup_workspace(hub: &Hub, root: &std::path::Path, label: &str) -> (String, String) {
    let path = root.join(label);
    std::fs::create_dir(&path).unwrap();
    let workspace = path.to_string_lossy().into_owned();
    let session_id = hub
        .open_session(&OpenRequest {
            workspace: workspace.clone(),
            objective: "Research a task".into(),
            idempotency_key: format!("{label}-session"),
        })
        .unwrap()
        .session_id;
    let task_id = hub
        .create_task(&TaskCreateRequest {
            session_id,
            objective: "Compare research sources".into(),
            acceptance_criteria: vec!["Cited source is visible".into()],
            write_scope: vec![],
            depends_on: vec![],
            idempotency_key: format!("{label}-task"),
        })
        .unwrap()
        .task
        .task_id;
    (workspace, task_id)
}

#[test]
fn task_research_links_are_scoped_idempotent_and_exported() {
    let area = tempfile::tempdir().unwrap();
    let db = area.path().join("db.sqlite3");
    let hub = Hub::open(&db).unwrap();
    let store = Store::open(&db).unwrap();
    let (workspace, task_id) = setup_workspace(&hub, area.path(), "one");
    let (other_workspace, _) = setup_workspace(&hub, area.path(), "two");
    let revision = store
        .ingest_research_document(
            &workspace,
            &FetchedDocument {
                source: "github".into(),
                source_id: "1".into(),
                source_url: "https://github.com/example/repo/issues/1".into(),
                title: "Agent memory".into(),
                body: "Useful external report".into(),
                author_name: None,
                content_license: None,
                published_at_unix_ms: None,
            },
        )
        .unwrap();
    let request = TaskResearchAttachRequest {
        workspace: workspace.clone(),
        task_id: task_id.clone(),
        revision_id: Some(revision.revision_id),
        host_observation: None,
        relevance_note: "Relevant to retrieval design".into(),
        idempotency_key: "cite-1".into(),
    };
    let first = store.attach_task_research(&request).unwrap();
    assert!(!first.duplicate);
    assert_eq!(first.item.provenance, "aporic_api");
    assert!(store.attach_task_research(&request).unwrap().duplicate);
    assert!(
        store
            .attach_task_research(&TaskResearchAttachRequest {
                relevance_note: "Changed note".into(),
                ..request.clone()
            })
            .is_err()
    );
    assert!(
        store
            .attach_task_research(&TaskResearchAttachRequest {
                workspace: other_workspace,
                idempotency_key: "cross-workspace".into(),
                ..request.clone()
            })
            .is_err()
    );

    let reddit = TaskResearchAttachRequest {
        workspace: workspace.clone(),
        task_id: task_id.clone(),
        revision_id: None,
        host_observation: Some(HostResearchObservation {
            source: "reddit".into(),
            source_url: "https://www.reddit.com/r/rust/comments/abc/example/".into(),
            title: "Rust user report".into(),
            excerpt: "Observed by the host; not API verified".into(),
        }),
        relevance_note: "User experience signal".into(),
        idempotency_key: "reddit-1".into(),
    };
    let reported = store.attach_task_research(&reddit).unwrap();
    assert_eq!(reported.item.provenance, "host_reported");
    assert!(reported.authority_notice.contains("not independently"));
    assert!(
        store
            .attach_task_research(&TaskResearchAttachRequest {
                host_observation: Some(HostResearchObservation {
                    source_url: "https://reddit.com.evil.test/item".into(),
                    ..reddit.host_observation.clone().unwrap()
                }),
                idempotency_key: "spoof".into(),
                ..reddit.clone()
            })
            .is_err()
    );
    assert!(
        store
            .attach_task_research(&TaskResearchAttachRequest {
                revision_id: request.revision_id.clone(),
                idempotency_key: "ambiguous".into(),
                ..reddit
            })
            .is_err()
    );

    let restarted = Store::open(&db).unwrap();
    let linked = restarted
        .list_task_research(&TaskResearchListRequest {
            workspace: workspace.clone(),
            task_id,
            limit: None,
        })
        .unwrap();
    assert_eq!(linked.len(), 2);
    let export = hub.export_project(&workspace).unwrap();
    assert_eq!(export.format_version, 21);
    assert_eq!(export.task_research_items, linked);
    assert!(hub.audit_task_research().unwrap().consistent);
    let connection = rusqlite::Connection::open(&db).unwrap();
    connection
        .execute(
            "UPDATE task_research_items SET excerpt = 'tampered' WHERE item_id = ?1",
            [&reported.item.item_id],
        )
        .unwrap();
    assert!(!hub.audit_task_research().unwrap().consistent);
    connection
        .execute(
            "UPDATE task_research_items SET excerpt = ?1 WHERE item_id = ?2",
            rusqlite::params![reported.item.excerpt, reported.item.item_id],
        )
        .unwrap();
    assert!(hub.audit_task_research().unwrap().consistent);
    connection
        .execute(
            "UPDATE task_research_items SET relevance_note = 'tampered' WHERE item_id = ?1",
            [&reported.item.item_id],
        )
        .unwrap();
    assert!(!hub.audit_task_research().unwrap().consistent);
    connection
        .execute(
            "UPDATE task_research_items SET relevance_note = ?1 WHERE item_id = ?2",
            rusqlite::params![reported.item.relevance_note, reported.item.item_id],
        )
        .unwrap();
    connection
        .execute(
            "DELETE FROM task_research_items WHERE item_id = ?1",
            [&reported.item.item_id],
        )
        .unwrap();
    assert!(!hub.audit_task_research().unwrap().consistent);
}

#[test]
fn schema_24_upgrades_to_task_research_without_prior_items() {
    let area = tempfile::tempdir().unwrap();
    let db = area.path().join("db.sqlite3");
    drop(Hub::open(&db).unwrap());
    let connection = rusqlite::Connection::open(&db).unwrap();
    connection
        .execute_batch("DROP TABLE task_research_items; PRAGMA user_version = 24;")
        .unwrap();
    drop(connection);
    let upgraded = Hub::open(&db).unwrap();
    assert_eq!(upgraded.stats().unwrap().schema_version, 25);
    assert!(upgraded.audit_task_research().unwrap().consistent);
}
