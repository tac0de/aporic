use aporic_boundary::{ActionRule, BoundaryPolicy, ReserveRequest, reserve};
use aporic_kernel::{CommitRequest, CommitStatus, Event, Grant, commit, initialize, load};
use aporic_roles::{HostPolicy, RoutingTier, load_role};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_PATH: AtomicU64 = AtomicU64::new(1);

struct TestStore(PathBuf);

impl TestStore {
    fn new() -> Self {
        let sequence = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "aporic-boundary-{}-{sequence}.jsonl",
            std::process::id()
        ));
        initialize(&path).unwrap();
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

fn role_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../roles/generalist")
}

fn host(max_tool_calls: u32, max_parallel_tasks: u16) -> HostPolicy {
    HostPolicy {
        capabilities: ["workspace.read", "workspace.write"]
            .into_iter()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>(),
        default_routing: RoutingTier::Balanced,
        routing_ceiling: RoutingTier::Balanced,
        max_tool_calls,
        max_parallel_tasks,
        may_delegate: false,
    }
}

fn policy(delegates: bool) -> BoundaryPolicy {
    BoundaryPolicy {
        actions: BTreeMap::from([
            (
                "read_file".into(),
                ActionRule {
                    capability: "workspace.read".into(),
                    delegates: false,
                },
            ),
            (
                "write_file".into(),
                ActionRule {
                    capability: "workspace.write".into(),
                    delegates,
                },
            ),
            (
                "run_process".into(),
                ActionRule {
                    capability: "process.execute".into(),
                    delegates: false,
                },
            ),
        ]),
    }
}

fn request(revision: u64, suffix: &str, action: &str) -> ReserveRequest {
    ReserveRequest {
        event_id: format!("reserve-{suffix}"),
        idempotency_key: format!("reserve-key-{suffix}"),
        expected_revision: revision,
        reservation_id: format!("reservation-{suffix}"),
        grant_id: format!("grant-{suffix}"),
        principal: "agent-1".into(),
        task_ref: "task-1".into(),
        session_ref: "day-1".into(),
        scope: "repo".into(),
        action: action.into(),
        input: json!({"path": "README.md"}),
        selected_routing: RoutingTier::Balanced,
        routing_reasons: vec!["test".into()],
    }
}

fn issue_grant(store: &TestStore, profile_ref: &str, revision: u64, suffix: &str, action: &str) {
    let candidate = request(revision, suffix, action);
    let outcome = commit(
        store.path(),
        CommitRequest::new(
            format!("grant-{suffix}"),
            format!("grant-key-{suffix}"),
            revision,
            Event::AuthorityGranted {
                grant: Grant {
                    grant_id: candidate.grant_id,
                    principal: candidate.principal,
                    task_ref: candidate.task_ref,
                    session_ref: candidate.session_ref,
                    profile_ref: profile_ref.into(),
                    scope: candidate.scope,
                    action: candidate.action,
                    input: candidate.input,
                    authority_ref: "human:approval-1".into(),
                },
            },
        ),
    )
    .unwrap();
    assert_eq!(outcome.status, CommitStatus::Committed);
}

#[test]
fn admitted_action_is_bound_to_the_effective_profile_and_day() {
    let store = TestStore::new();
    let profile = load_role(role_path(), &host(10, 1)).unwrap();
    issue_grant(&store, profile.profile_sha256(), 0, "1", "write_file");

    let candidate = request(1, "1", "write_file");
    let outcome = reserve(store.path(), &profile, &policy(false), candidate.clone()).unwrap();
    assert_eq!(outcome.status, CommitStatus::Committed);
    let retry = reserve(store.path(), &profile, &policy(false), candidate).unwrap();
    assert_eq!(retry.status, CommitStatus::Duplicate);
    assert_eq!(retry.revision, 2);
    let ledger = load(store.path()).unwrap();
    let reservation = &ledger.state().reservations["reservation-1"].reservation;
    assert_eq!(reservation.session_ref, "day-1");
    assert_eq!(reservation.profile_ref, profile.profile_sha256());
}

#[test]
fn unavailable_capability_and_unknown_action_do_not_mutate_the_ledger() {
    let store = TestStore::new();
    let profile = load_role(role_path(), &host(10, 1)).unwrap();

    let unavailable = reserve(
        store.path(),
        &profile,
        &policy(false),
        request(0, "1", "run_process"),
    )
    .unwrap();
    assert_eq!(unavailable.reason_code, "CAPABILITY_NOT_ALLOWED");
    let unknown = reserve(
        store.path(),
        &profile,
        &policy(false),
        request(0, "2", "unknown"),
    )
    .unwrap();
    assert_eq!(unknown.reason_code, "ACTION_NOT_MAPPED");
    assert_eq!(load(store.path()).unwrap().state().revision, 0);
}

#[test]
fn routing_ceiling_and_delegation_are_enforced_before_reservation() {
    let store = TestStore::new();
    let profile = load_role(role_path(), &host(10, 1)).unwrap();
    let mut deep = request(0, "1", "write_file");
    deep.selected_routing = RoutingTier::Deep;
    assert_eq!(
        reserve(store.path(), &profile, &policy(false), deep)
            .unwrap()
            .reason_code,
        "ROUTING_CEILING_EXCEEDED"
    );
    assert_eq!(
        reserve(
            store.path(),
            &profile,
            &policy(true),
            request(0, "2", "write_file"),
        )
        .unwrap()
        .reason_code,
        "DELEGATION_NOT_ALLOWED"
    );
    assert_eq!(load(store.path()).unwrap().state().revision, 0);
}

#[test]
fn active_reservation_blocks_parallel_work_for_the_same_day() {
    let store = TestStore::new();
    let profile = load_role(role_path(), &host(10, 1)).unwrap();
    issue_grant(&store, profile.profile_sha256(), 0, "1", "write_file");
    assert_eq!(
        reserve(
            store.path(),
            &profile,
            &policy(false),
            request(1, "1", "write_file")
        )
        .unwrap()
        .status,
        CommitStatus::Committed
    );
    issue_grant(&store, profile.profile_sha256(), 2, "2", "write_file");

    let outcome = reserve(
        store.path(),
        &profile,
        &policy(false),
        request(3, "2", "write_file"),
    )
    .unwrap();
    assert_eq!(outcome.reason_code, "PARALLEL_LIMIT_REACHED");
    assert_eq!(load(store.path()).unwrap().state().revision, 3);
}

#[test]
fn daily_tool_call_limit_counts_consumed_and_abandoned_reservations() {
    let store = TestStore::new();
    let profile = load_role(role_path(), &host(1, 1)).unwrap();
    issue_grant(&store, profile.profile_sha256(), 0, "1", "write_file");
    reserve(
        store.path(),
        &profile,
        &policy(false),
        request(1, "1", "write_file"),
    )
    .unwrap();
    commit(
        store.path(),
        CommitRequest::new(
            "abandon-1",
            "abandon-key-1",
            2,
            Event::ReservationAbandoned {
                reservation_id: "reservation-1".into(),
                reason: "effect was not observed".into(),
            },
        ),
    )
    .unwrap();
    issue_grant(&store, profile.profile_sha256(), 3, "2", "write_file");

    let outcome = reserve(
        store.path(),
        &profile,
        &policy(false),
        request(4, "2", "write_file"),
    )
    .unwrap();
    assert_eq!(outcome.reason_code, "TOOL_CALL_LIMIT_REACHED");
    assert_eq!(load(store.path()).unwrap().state().revision, 4);
}

#[test]
fn grant_for_another_profile_cannot_be_used() {
    let store = TestStore::new();
    let profile = load_role(role_path(), &host(10, 1)).unwrap();
    issue_grant(&store, "another-profile", 0, "1", "write_file");

    let outcome = reserve(
        store.path(),
        &profile,
        &policy(false),
        request(1, "1", "write_file"),
    )
    .unwrap();
    assert_eq!(outcome.reason_code, "PROFILE_MISMATCH");
    assert_eq!(load(store.path()).unwrap().state().revision, 1);
}
