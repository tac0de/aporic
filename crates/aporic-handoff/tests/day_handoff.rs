use aporic_handoff::{
    CommitRequest, CommitStatus, Day, Event, HandoffCapsule, HandoffRef, WorkspaceCheckpoint,
    commit, handoff_sha256, initialize, load,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_PATH: AtomicU64 = AtomicU64::new(1);

struct TestFile(PathBuf);

impl TestFile {
    fn new() -> Self {
        let sequence = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
        let directory =
            std::env::temp_dir().join(format!("aporic-handoff-{}-{sequence}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("handoff.jsonl");
        initialize(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.0.parent().unwrap());
    }
}

fn workspace(head: char, dirty: bool) -> WorkspaceCheckpoint {
    WorkspaceCheckpoint {
        binding_sha256: "a".repeat(64),
        head: head.to_string().repeat(40),
        dirty,
    }
}

fn day(id: &str, session: &str, predecessor: Option<HandoffRef>) -> Day {
    Day {
        day_id: id.into(),
        lineage_ref: "build-kernel".into(),
        task_ref: format!("task-{id}"),
        session_ref: session.into(),
        scope: format!("project:{}", "a".repeat(64)),
        predecessor,
    }
}

fn capsule(next_action: &str) -> HandoffCapsule {
    HandoffCapsule {
        objective: "Build the smallest useful Aporic kernel".into(),
        constraints: vec!["Rust product logic".into()],
        accepted_decisions: vec!["One chat session is one day".into()],
        completed_checks: vec!["cargo test".into()],
        open_questions: vec!["Which host adapter comes first?".into()],
        next_action: next_action.into(),
    }
}

fn request(revision: u64, id: &str, event: Event) -> CommitRequest {
    CommitRequest::new(format!("event-{id}"), format!("key-{id}"), revision, event)
}

#[test]
fn closes_one_day_and_explicitly_inherits_it_in_the_next() {
    let file = TestFile::new();
    let first = day("day-1", "session-1", None);
    let first_workspace = workspace('1', false);
    assert_eq!(
        commit(
            &file.0,
            request(
                0,
                "open-1",
                Event::DayOpened {
                    day: first.clone(),
                    workspace: first_workspace.clone(),
                },
            ),
        )
        .unwrap()
        .status,
        CommitStatus::Committed
    );

    let capsule = capsule("Implement the host adapter");
    let digest = handoff_sha256(&first, &capsule, &first_workspace).unwrap();
    commit(
        &file.0,
        request(
            1,
            "project-1",
            Event::HandoffProjected {
                day_id: first.day_id.clone(),
                capsule,
                workspace: first_workspace.clone(),
                handoff_sha256: digest.clone(),
            },
        ),
    )
    .unwrap();
    commit(
        &file.0,
        request(
            2,
            "close-1",
            Event::DayClosed {
                day_id: first.day_id.clone(),
                handoff_sha256: digest.clone(),
                workspace: first_workspace.clone(),
            },
        ),
    )
    .unwrap();

    let second = day(
        "day-2",
        "session-2",
        Some(HandoffRef {
            day_id: first.day_id.clone(),
            handoff_sha256: digest,
        }),
    );
    commit(
        &file.0,
        request(
            3,
            "open-2",
            Event::DayOpened {
                day: second.clone(),
                workspace: first_workspace,
            },
        ),
    )
    .unwrap();

    let replayed = load(&file.0).unwrap();
    assert_eq!(replayed.state().revision, 4);
    assert!(replayed.state().days["day-1"].closed);
    assert_eq!(
        replayed.state().days["day-1"].successor_day_id.as_deref(),
        Some("day-2")
    );
    assert_eq!(
        replayed.state().days["day-2"].day.predecessor,
        second.predecessor
    );
}

#[test]
fn requires_a_current_handoff_before_closing() {
    let file = TestFile::new();
    let workspace = workspace('1', false);
    commit(
        &file.0,
        request(
            0,
            "open",
            Event::DayOpened {
                day: day("day-1", "session-1", None),
                workspace: workspace.clone(),
            },
        ),
    )
    .unwrap();

    let outcome = commit(
        &file.0,
        request(
            1,
            "close",
            Event::DayClosed {
                day_id: "day-1".into(),
                handoff_sha256: "b".repeat(64),
                workspace,
            },
        ),
    )
    .unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.reason_code, "HANDOFF_REQUIRED");
    assert_eq!(load(&file.0).unwrap().state().revision, 1);
}

#[test]
fn rejects_a_stale_workspace_at_close() {
    let file = TestFile::new();
    let day = day("day-1", "session-1", None);
    let initial = workspace('1', false);
    commit(
        &file.0,
        request(
            0,
            "open",
            Event::DayOpened {
                day: day.clone(),
                workspace: initial.clone(),
            },
        ),
    )
    .unwrap();
    let capsule = capsule("Continue");
    let digest = handoff_sha256(&day, &capsule, &initial).unwrap();
    commit(
        &file.0,
        request(
            1,
            "project",
            Event::HandoffProjected {
                day_id: day.day_id,
                capsule,
                workspace: initial,
                handoff_sha256: digest.clone(),
            },
        ),
    )
    .unwrap();

    let outcome = commit(
        &file.0,
        request(
            2,
            "close",
            Event::DayClosed {
                day_id: "day-1".into(),
                handoff_sha256: digest,
                workspace: workspace('2', false),
            },
        ),
    )
    .unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.reason_code, "HANDOFF_STALE");
}

#[test]
fn rejects_successor_without_exact_handoff_acknowledgement() {
    let file = TestFile::new();
    let first = day("day-1", "session-1", None);
    let workspace = workspace('1', false);
    commit(
        &file.0,
        request(
            0,
            "open-1",
            Event::DayOpened {
                day: first.clone(),
                workspace: workspace.clone(),
            },
        ),
    )
    .unwrap();
    let capsule = capsule("Continue");
    let digest = handoff_sha256(&first, &capsule, &workspace).unwrap();
    commit(
        &file.0,
        request(
            1,
            "project",
            Event::HandoffProjected {
                day_id: first.day_id.clone(),
                capsule,
                workspace: workspace.clone(),
                handoff_sha256: digest.clone(),
            },
        ),
    )
    .unwrap();
    commit(
        &file.0,
        request(
            2,
            "close",
            Event::DayClosed {
                day_id: first.day_id.clone(),
                handoff_sha256: digest,
                workspace: workspace.clone(),
            },
        ),
    )
    .unwrap();

    let outcome = commit(
        &file.0,
        request(
            3,
            "open-2",
            Event::DayOpened {
                day: day(
                    "day-2",
                    "session-2",
                    Some(HandoffRef {
                        day_id: first.day_id,
                        handoff_sha256: "f".repeat(64),
                    }),
                ),
                workspace,
            },
        ),
    )
    .unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.reason_code, "PREDECESSOR_HANDOFF_MISMATCH");
}

#[test]
fn exact_retry_is_idempotent() {
    let file = TestFile::new();
    let request = request(
        0,
        "open",
        Event::DayOpened {
            day: day("day-1", "session-1", None),
            workspace: workspace('1', false),
        },
    );
    assert_eq!(
        commit(&file.0, request.clone()).unwrap().status,
        CommitStatus::Committed
    );
    let retry = commit(&file.0, request).unwrap();
    assert_eq!(retry.status, CommitStatus::Duplicate);
    assert_eq!(retry.revision, 1);
}
