use aporic_boundary::{ActionRule, BoundaryPolicy};
use aporic_handoff::{CommitStatus as HandoffCommitStatus, HandoffCapsule, HandoffRef};
use aporic_host::{
    ActionIdentity, CloseDayRequest, ConnectRequest, EffectRequest, Error, GrantRequest,
    OpenDayRequest, ProjectHandoffRequest, ReserveRequest, VerificationRequest, connect,
};
use aporic_kernel::{CommitStatus, EffectOutcome, VerificationResult, initialize, load};
use aporic_projects::{BindRequest, bind_project};
use aporic_roles::{HostPolicy, RoutingTier};
use aporic_routing::RoutingSignals;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_PATH: AtomicU64 = AtomicU64::new(1);

struct TestArea(PathBuf);

impl TestArea {
    fn new() -> Self {
        let sequence = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("aporic-host-{}-{sequence}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn repo(&self) -> PathBuf {
        self.0.join("repo")
    }

    fn registry(&self) -> PathBuf {
        self.0.join("registry")
    }

    fn store(&self) -> PathBuf {
        self.0.join("kernel.jsonl")
    }

    fn handoff_store(&self) -> PathBuf {
        self.0.join("handoff.jsonl")
    }
}

impl Drop for TestArea {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn git(directory: &Path, arguments: &[&str]) {
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(directory)
            .args(arguments)
            .status()
            .unwrap()
            .success()
    );
}

fn role_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../roles/generalist")
}

fn setup(area: &TestArea) {
    let repo = area.repo();
    std::fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "--quiet"]);
    git(
        &repo,
        &[
            "-c",
            "user.name=Aporic Test",
            "-c",
            "user.email=aporic@example.invalid",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "initial",
        ],
    );
    git(
        &repo,
        &[
            "remote",
            "add",
            "origin",
            "https://example.invalid/acme/demo.git",
        ],
    );
    bind_project(
        area.registry(),
        BindRequest {
            project_id: "demo".into(),
            workspace: repo,
            remote_name: "origin".into(),
        },
    )
    .unwrap();
    initialize(area.store()).unwrap();
    aporic_handoff::initialize(area.handoff_store()).unwrap();
}

fn connect_runtime(area: &TestArea) -> aporic_host::HostRuntime {
    connect(ConnectRequest {
        registry: area.registry(),
        project_id: "demo".into(),
        role_directory: role_path(),
        kernel_store: area.store(),
        handoff_store: area.handoff_store(),
        host_policy: HostPolicy {
            capabilities: ["workspace.read", "workspace.write"]
                .into_iter()
                .map(str::to_owned)
                .collect::<BTreeSet<_>>(),
            default_routing: RoutingTier::Balanced,
            routing_ceiling: RoutingTier::Balanced,
            max_tool_calls: 8,
            max_parallel_tasks: 1,
            may_delegate: false,
        },
        boundary_policy: BoundaryPolicy {
            actions: BTreeMap::from([
                (
                    "write_file".into(),
                    ActionRule {
                        capability: "workspace.write".into(),
                        delegates: false,
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
        },
    })
    .unwrap()
}

fn action(action: &str) -> ActionIdentity {
    ActionIdentity {
        principal: "agent-1".into(),
        task_ref: "task-1".into(),
        session_ref: "day-1".into(),
        action: action.into(),
        input: json!({"path": "README.md", "content": "hello"}),
    }
}

fn capsule(next_action: &str) -> HandoffCapsule {
    HandoffCapsule {
        objective: "Build Aporic".into(),
        constraints: vec!["Keep the core small".into()],
        accepted_decisions: vec!["One session is one day".into()],
        completed_checks: vec!["H0 tests passed".into()],
        open_questions: vec!["Which adapter is next?".into()],
        next_action: next_action.into(),
    }
}

#[test]
fn routes_one_action_through_all_four_surfaces() {
    let area = TestArea::new();
    setup(&area);
    let runtime = connect_runtime(&area);
    assert!(runtime.scope().starts_with("project:"));

    assert_eq!(
        runtime
            .control()
            .grant(GrantRequest {
                event_id: "grant-event".into(),
                idempotency_key: "grant-key".into(),
                expected_revision: 0,
                grant_id: "grant-1".into(),
                action: action("write_file"),
                authority_ref: "human:approval-1".into(),
            })
            .unwrap()
            .status,
        CommitStatus::Committed
    );
    assert_eq!(
        runtime
            .agent()
            .reserve(ReserveRequest {
                event_id: "reserve-event".into(),
                idempotency_key: "reserve-key".into(),
                expected_revision: 1,
                reservation_id: "reservation-1".into(),
                grant_id: "grant-1".into(),
                action: action("write_file"),
                routing_signals: RoutingSignals::default(),
            })
            .unwrap()
            .status,
        CommitStatus::Committed
    );
    assert_eq!(
        runtime
            .effects()
            .record(EffectRequest {
                event_id: "effect-event".into(),
                idempotency_key: "effect-key".into(),
                expected_revision: 2,
                reservation_id: "reservation-1".into(),
                outcome: EffectOutcome::Succeeded,
                observation_ref: "host:receipt-1".into(),
            })
            .unwrap()
            .status,
        CommitStatus::Committed
    );
    assert_eq!(
        runtime
            .verifier()
            .record(VerificationRequest {
                event_id: "verify-event".into(),
                idempotency_key: "verify-key".into(),
                expected_revision: 3,
                reservation_id: "reservation-1".into(),
                verifier: "verifier-1".into(),
                result: VerificationResult::Passed,
                evidence_ref: "test:run-1".into(),
            })
            .unwrap()
            .status,
        CommitStatus::Committed
    );

    let ledger = load(area.store()).unwrap();
    let reservation = &ledger.state().reservations["reservation-1"];
    assert_eq!(reservation.reservation.scope, runtime.scope());
    assert_eq!(reservation.reservation.session_ref, "day-1");
    assert_eq!(
        reservation.verification.as_ref().unwrap().result,
        VerificationResult::Passed
    );
}

#[test]
fn hands_off_between_days_without_transferring_kernel_authority() {
    let area = TestArea::new();
    setup(&area);
    let runtime = connect_runtime(&area);

    assert_eq!(
        runtime
            .continuity()
            .open(OpenDayRequest {
                event_id: "open-1-event".into(),
                idempotency_key: "open-1-key".into(),
                expected_revision: 0,
                day_id: "day-1".into(),
                lineage_ref: "build-aporic".into(),
                task_ref: "task-1".into(),
                session_ref: "session-1".into(),
                predecessor: None,
            })
            .unwrap()
            .status,
        HandoffCommitStatus::Committed
    );
    let projected = runtime
        .continuity()
        .project(ProjectHandoffRequest {
            event_id: "project-1-event".into(),
            idempotency_key: "project-1-key".into(),
            expected_revision: 1,
            day_id: "day-1".into(),
            capsule: capsule("Implement D1"),
        })
        .unwrap();
    let digest = projected.handoff_sha256;
    runtime
        .continuity()
        .close(CloseDayRequest {
            event_id: "close-1-event".into(),
            idempotency_key: "close-1-key".into(),
            expected_revision: 2,
            day_id: "day-1".into(),
            handoff_sha256: digest.clone(),
        })
        .unwrap();
    runtime
        .continuity()
        .open(OpenDayRequest {
            event_id: "open-2-event".into(),
            idempotency_key: "open-2-key".into(),
            expected_revision: 3,
            day_id: "day-2".into(),
            lineage_ref: "build-aporic".into(),
            task_ref: "task-2".into(),
            session_ref: "session-2".into(),
            predecessor: Some(HandoffRef {
                day_id: "day-1".into(),
                handoff_sha256: digest,
            }),
        })
        .unwrap();

    assert_eq!(
        aporic_handoff::load(area.handoff_store())
            .unwrap()
            .state()
            .revision,
        4
    );
    assert_eq!(load(area.store()).unwrap().state().revision, 0);
}

#[test]
fn changed_remote_blocks_new_work_but_not_effect_recording() {
    let area = TestArea::new();
    setup(&area);
    let runtime = connect_runtime(&area);
    runtime
        .control()
        .grant(GrantRequest {
            event_id: "grant-event".into(),
            idempotency_key: "grant-key".into(),
            expected_revision: 0,
            grant_id: "grant-1".into(),
            action: action("write_file"),
            authority_ref: "human:approval-1".into(),
        })
        .unwrap();
    runtime
        .agent()
        .reserve(ReserveRequest {
            event_id: "reserve-event".into(),
            idempotency_key: "reserve-key".into(),
            expected_revision: 1,
            reservation_id: "reservation-1".into(),
            grant_id: "grant-1".into(),
            action: action("write_file"),
            routing_signals: RoutingSignals::default(),
        })
        .unwrap();
    git(
        &area.repo(),
        &[
            "remote",
            "set-url",
            "origin",
            "https://example.invalid/acme/changed.git",
        ],
    );

    assert_eq!(
        runtime
            .control()
            .grant(GrantRequest {
                event_id: "grant-event".into(),
                idempotency_key: "grant-key".into(),
                expected_revision: 0,
                grant_id: "grant-1".into(),
                action: action("write_file"),
                authority_ref: "human:approval-1".into(),
            })
            .unwrap()
            .status,
        CommitStatus::Duplicate
    );
    assert_eq!(
        runtime
            .agent()
            .reserve(ReserveRequest {
                event_id: "reserve-event".into(),
                idempotency_key: "reserve-key".into(),
                expected_revision: 1,
                reservation_id: "reservation-1".into(),
                grant_id: "grant-1".into(),
                action: action("write_file"),
                routing_signals: RoutingSignals::default(),
            })
            .unwrap()
            .status,
        CommitStatus::Duplicate
    );

    assert!(matches!(
        runtime.control().grant(GrantRequest {
            event_id: "grant-event-2".into(),
            idempotency_key: "grant-key-2".into(),
            expected_revision: 2,
            grant_id: "grant-2".into(),
            action: action("write_file"),
            authority_ref: "human:approval-2".into(),
        }),
        Err(Error::Project(aporic_projects::Error::InvalidBinding(
            "remote URL changed"
        )))
    ));
    assert_eq!(
        runtime
            .effects()
            .record(EffectRequest {
                event_id: "effect-event".into(),
                idempotency_key: "effect-key".into(),
                expected_revision: 2,
                reservation_id: "reservation-1".into(),
                outcome: EffectOutcome::Unknown,
                observation_ref: "host:identity-changed".into(),
            })
            .unwrap()
            .status,
        CommitStatus::Committed
    );
}

#[test]
fn disallowed_action_is_rejected_before_authority_is_written() {
    let area = TestArea::new();
    setup(&area);
    let runtime = connect_runtime(&area);
    let outcome = runtime
        .control()
        .grant(GrantRequest {
            event_id: "grant-event".into(),
            idempotency_key: "grant-key".into(),
            expected_revision: 0,
            grant_id: "grant-1".into(),
            action: action("run_process"),
            authority_ref: "human:approval-1".into(),
        })
        .unwrap();

    assert_eq!(outcome.status, CommitStatus::Rejected);
    assert_eq!(outcome.reason_code, "CAPABILITY_NOT_ALLOWED");
    assert_eq!(load(area.store()).unwrap().state().revision, 0);
}

#[test]
fn kernel_store_inside_project_is_rejected() {
    let area = TestArea::new();
    setup(&area);
    let inside = area.repo().join("kernel.jsonl");
    initialize(&inside).unwrap();

    let result = connect(ConnectRequest {
        registry: area.registry(),
        project_id: "demo".into(),
        role_directory: role_path(),
        kernel_store: inside,
        handoff_store: area.handoff_store(),
        host_policy: HostPolicy {
            capabilities: BTreeSet::new(),
            default_routing: RoutingTier::Economy,
            routing_ceiling: RoutingTier::Economy,
            max_tool_calls: 1,
            max_parallel_tasks: 1,
            may_delegate: false,
        },
        boundary_policy: BoundaryPolicy::default(),
    });
    assert!(matches!(
        result,
        Err(Error::InvalidConfiguration(
            "kernel store must remain outside the governed workspace"
        ))
    ));
}

#[test]
fn handoff_store_inside_project_is_rejected() {
    let area = TestArea::new();
    setup(&area);
    let inside = area.repo().join("handoff.jsonl");
    aporic_handoff::initialize(&inside).unwrap();

    let result = connect(ConnectRequest {
        registry: area.registry(),
        project_id: "demo".into(),
        role_directory: role_path(),
        kernel_store: area.store(),
        handoff_store: inside,
        host_policy: HostPolicy {
            capabilities: BTreeSet::new(),
            default_routing: RoutingTier::Economy,
            routing_ceiling: RoutingTier::Economy,
            max_tool_calls: 1,
            max_parallel_tasks: 1,
            may_delegate: false,
        },
        boundary_policy: BoundaryPolicy::default(),
    });
    assert!(matches!(
        result,
        Err(Error::InvalidConfiguration(
            "handoff store must remain outside the governed workspace"
        ))
    ));
}
