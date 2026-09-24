use aporic_kernel::{
    CommitRequest, CommitStatus, EffectOutcome, Error, Event, Grant, Reservation,
    VerificationResult, commit, initialize, load,
};
use serde_json::{Value, json};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_PATH: AtomicU64 = AtomicU64::new(1);

struct TestStore(PathBuf);

impl TestStore {
    fn new() -> Self {
        let sequence = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "aporic-kernel-{}-{sequence}.jsonl",
            std::process::id()
        ));
        initialize(&path).expect("initialize store");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestStore {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn grant(input: Value) -> Grant {
    Grant {
        grant_id: "grant-1".into(),
        principal: "agent-1".into(),
        task_ref: "task-1".into(),
        session_ref: "session-1".into(),
        profile_ref: "profile-1".into(),
        scope: "repo".into(),
        action: "write_file".into(),
        input,
        authority_ref: "human:approval-1".into(),
    }
}

fn reservation(input: Value) -> Reservation {
    Reservation {
        reservation_id: "reservation-1".into(),
        grant_id: "grant-1".into(),
        principal: "agent-1".into(),
        task_ref: "task-1".into(),
        session_ref: "session-1".into(),
        profile_ref: "profile-1".into(),
        scope: "repo".into(),
        action: "write_file".into(),
        input,
    }
}

fn request(revision: u64, id: &str, event: Event) -> CommitRequest {
    CommitRequest::new(id, format!("key-{id}"), revision, event)
}

fn issue_grant(store: &TestStore, input: Value) {
    let outcome = commit(
        store.path(),
        request(
            0,
            "grant-event",
            Event::AuthorityGranted {
                grant: grant(input),
            },
        ),
    )
    .expect("commit grant");
    assert_eq!(outcome.status, CommitStatus::Committed);
}

#[test]
fn completes_and_replays_the_one_shot_vertical_slice() {
    let store = TestStore::new();
    let input = json!({"path": "README.md", "content": "hello"});
    issue_grant(&store, input.clone());

    let reserve = request(
        1,
        "reserve-event",
        Event::ActionReserved {
            reservation: reservation(input),
        },
    );
    assert_eq!(
        commit(store.path(), reserve.clone()).unwrap().status,
        CommitStatus::Committed
    );
    assert_eq!(
        commit(store.path(), reserve).unwrap().status,
        CommitStatus::Duplicate
    );

    let second = request(
        2,
        "reserve-again",
        Event::ActionReserved {
            reservation: Reservation {
                reservation_id: "reservation-2".into(),
                ..reservation(json!({"path": "README.md", "content": "hello"}))
            },
        },
    );
    assert_eq!(
        commit(store.path(), second).unwrap().reason_code,
        "GRANT_ALREADY_RESERVED"
    );

    assert_eq!(
        commit(
            store.path(),
            request(
                2,
                "effect-event",
                Event::EffectRecorded {
                    reservation_id: "reservation-1".into(),
                    outcome: EffectOutcome::Succeeded,
                    observation_ref: "host:receipt-1".into(),
                },
            ),
        )
        .unwrap()
        .status,
        CommitStatus::Committed
    );
    assert_eq!(
        commit(
            store.path(),
            request(
                3,
                "verification-event",
                Event::VerificationRecorded {
                    reservation_id: "reservation-1".into(),
                    verifier: "verifier-1".into(),
                    result: VerificationResult::Passed,
                    evidence_ref: "test:run-1".into(),
                },
            ),
        )
        .unwrap()
        .status,
        CommitStatus::Committed
    );

    let first = load(store.path()).unwrap();
    let second = load(store.path()).unwrap();
    assert_eq!(first.state(), second.state());
    assert_eq!(first.state().revision, 4);
    assert_eq!(
        first.state().grants["grant-1"].reserved_by.as_deref(),
        Some("reservation-1")
    );
    assert_eq!(
        first.state().reservations["reservation-1"]
            .verification
            .as_ref()
            .map(|record| record.result),
        Some(VerificationResult::Passed)
    );
}

#[test]
fn rejects_every_grant_binding_mismatch() {
    type ReservationMutation = fn(&mut Reservation);
    let cases: [(&str, ReservationMutation); 7] = [
        ("PRINCIPAL_MISMATCH", |value| {
            value.principal = "other".into()
        }),
        ("TASK_MISMATCH", |value| value.task_ref = "other".into()),
        ("SESSION_MISMATCH", |value| {
            value.session_ref = "other".into()
        }),
        ("PROFILE_MISMATCH", |value| {
            value.profile_ref = "other".into()
        }),
        ("SCOPE_MISMATCH", |value| value.scope = "other".into()),
        ("ACTION_MISMATCH", |value| value.action = "other".into()),
        ("INPUT_MISMATCH", |value| {
            value.input = json!({"other": true})
        }),
    ];

    for (expected, mutate) in cases {
        let store = TestStore::new();
        let input = json!({"path": "README.md"});
        issue_grant(&store, input.clone());
        let mut candidate = reservation(input);
        mutate(&mut candidate);
        let outcome = commit(
            store.path(),
            request(
                1,
                expected,
                Event::ActionReserved {
                    reservation: candidate,
                },
            ),
        )
        .unwrap();
        assert_eq!(outcome.status, CommitStatus::Rejected);
        assert_eq!(outcome.reason_code, expected);
    }
}

#[test]
fn rejects_conflicting_idempotency_key_and_stale_revision() {
    let store = TestStore::new();
    let input = json!({"path": "README.md"});
    let original = request(
        0,
        "grant-event",
        Event::AuthorityGranted {
            grant: grant(input.clone()),
        },
    );
    assert_eq!(
        commit(store.path(), original.clone()).unwrap().status,
        CommitStatus::Committed
    );
    assert_eq!(
        commit(store.path(), original).unwrap().status,
        CommitStatus::Duplicate
    );

    let mut conflict = request(
        1,
        "other-event",
        Event::ActionReserved {
            reservation: reservation(input.clone()),
        },
    );
    conflict.idempotency_key = "key-grant-event".into();
    assert_eq!(
        commit(store.path(), conflict).unwrap().reason_code,
        "IDEMPOTENCY_KEY_CONFLICT"
    );

    assert_eq!(
        commit(
            store.path(),
            request(
                0,
                "stale-event",
                Event::ActionReserved {
                    reservation: reservation(input),
                },
            ),
        )
        .unwrap()
        .reason_code,
        "STALE_REVISION"
    );
}

#[test]
fn requires_effect_before_verification() {
    let store = TestStore::new();
    let input = json!({"path": "README.md"});
    issue_grant(&store, input.clone());
    commit(
        store.path(),
        request(
            1,
            "reserve-event",
            Event::ActionReserved {
                reservation: reservation(input),
            },
        ),
    )
    .unwrap();

    let outcome = commit(
        store.path(),
        request(
            2,
            "verify-event",
            Event::VerificationRecorded {
                reservation_id: "reservation-1".into(),
                verifier: "verifier-1".into(),
                result: VerificationResult::Passed,
                evidence_ref: "agent:claim".into(),
            },
        ),
    )
    .unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.reason_code, "EFFECT_REQUIRED");
}

#[test]
fn execution_principal_cannot_verify_its_own_effect() {
    let store = TestStore::new();
    let input = json!({"path": "README.md"});
    issue_grant(&store, input.clone());
    commit(
        store.path(),
        request(
            1,
            "reserve-event",
            Event::ActionReserved {
                reservation: reservation(input),
            },
        ),
    )
    .unwrap();
    commit(
        store.path(),
        request(
            2,
            "effect-event",
            Event::EffectRecorded {
                reservation_id: "reservation-1".into(),
                outcome: EffectOutcome::Succeeded,
                observation_ref: "host:receipt-1".into(),
            },
        ),
    )
    .unwrap();

    let outcome = commit(
        store.path(),
        request(
            3,
            "verify-event",
            Event::VerificationRecorded {
                reservation_id: "reservation-1".into(),
                verifier: "agent-1".into(),
                result: VerificationResult::Passed,
                evidence_ref: "agent:self-claim".into(),
            },
        ),
    )
    .unwrap();
    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.reason_code, "SELF_VERIFICATION_FORBIDDEN");
}

#[test]
fn abandonment_is_terminal_and_does_not_restore_the_grant() {
    let store = TestStore::new();
    let input = json!({"path": "README.md"});
    issue_grant(&store, input.clone());
    commit(
        store.path(),
        request(
            1,
            "reserve-event",
            Event::ActionReserved {
                reservation: reservation(input.clone()),
            },
        ),
    )
    .unwrap();
    assert_eq!(
        commit(
            store.path(),
            request(
                2,
                "abandon-event",
                Event::ReservationAbandoned {
                    reservation_id: "reservation-1".into(),
                    reason: "host crashed before effect was observed".into(),
                },
            ),
        )
        .unwrap()
        .status,
        CommitStatus::Committed
    );

    let outcome = commit(
        store.path(),
        request(
            3,
            "reserve-again",
            Event::ActionReserved {
                reservation: Reservation {
                    reservation_id: "reservation-2".into(),
                    ..reservation(input)
                },
            },
        ),
    )
    .unwrap();
    assert_eq!(outcome.reason_code, "GRANT_ALREADY_RESERVED");
}

#[test]
fn corrupt_or_incomplete_logs_fail_closed_for_reads_and_writes() {
    let store = TestStore::new();
    OpenOptions::new()
        .append(true)
        .open(store.path())
        .unwrap()
        .write_all(b"{\"incomplete\":true}")
        .unwrap();

    assert!(matches!(load(store.path()), Err(Error::CorruptLog { .. })));
    assert!(matches!(
        commit(
            store.path(),
            request(
                0,
                "grant-event",
                Event::AuthorityGranted {
                    grant: grant(json!({}))
                },
            )
        ),
        Err(Error::CorruptLog { .. })
    ));
}

#[test]
fn replay_rejects_duplicate_idempotency_keys() {
    let store = TestStore::new();
    issue_grant(&store, json!({"path": "README.md"}));
    let mut second_grant = grant(json!({"path": "OTHER.md"}));
    second_grant.grant_id = "grant-2".into();
    let stored = aporic_kernel::StoredEvent {
        sequence: 2,
        request: CommitRequest {
            protocol_epoch: aporic_kernel::PROTOCOL_EPOCH,
            event_id: "grant-event-2".into(),
            idempotency_key: "key-grant-event".into(),
            expected_revision: 1,
            event: Event::AuthorityGranted {
                grant: second_grant,
            },
        },
    };
    let mut file = OpenOptions::new().append(true).open(store.path()).unwrap();
    file.write_all(&serde_json::to_vec(&stored).unwrap())
        .unwrap();
    file.write_all(b"\n").unwrap();
    drop(file);

    assert!(matches!(load(store.path()), Err(Error::CorruptLog { .. })));
}

#[test]
fn concurrent_reservations_commit_at_most_once() {
    let store = TestStore::new();
    let input = json!({"path": "README.md"});
    issue_grant(&store, input.clone());
    let path = store.path().to_path_buf();

    let handles = ["a", "b"].map(|suffix| {
        let path = path.clone();
        let input = input.clone();
        std::thread::spawn(move || {
            let mut value = reservation(input);
            value.reservation_id = format!("reservation-{suffix}");
            commit(
                path,
                request(
                    1,
                    &format!("reserve-{suffix}"),
                    Event::ActionReserved { reservation: value },
                ),
            )
            .unwrap()
        })
    });

    let outcomes = handles.map(|handle| handle.join().unwrap());
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| outcome.status == CommitStatus::Committed)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| outcome.status == CommitStatus::Rejected)
            .count(),
        1
    );
    assert_eq!(load(store.path()).unwrap().state().reservations.len(), 1);
}
