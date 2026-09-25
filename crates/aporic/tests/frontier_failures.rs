use aporic::{
    Hub,
    domain::{
        CloseDisposition, CloseRequest, OpenRequest, RecallRequest, ReconcileRequest, RecordKind,
        RecordRequest,
    },
};
use rusqlite::Connection;
use serde_json::json;

#[test]
fn deterministic_frontier_failure_suite() {
    let area = tempfile::tempdir().unwrap();
    let workspace = area.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let database = area.path().join("aporic.sqlite3");
    let hub = Hub::open(&database).unwrap();

    let first = open(
        &hub,
        &workspace,
        "Choose the durable store",
        "open-store-v1",
    );
    let old_decision = hub
        .record(&RecordRequest {
            session_id: first.clone(),
            kind: RecordKind::Decision,
            content: "Store state in JSON files.".to_owned(),
            evidence: Some("Initial prototype constraint".to_owned()),
            supersedes_record_id: None,
            verifies_effect_id: None,
            idempotency_key: "decision-store-v1".to_owned(),
        })
        .unwrap()
        .record;
    complete(
        &hub,
        &first,
        "Initial storage decision recorded",
        "close-store-v1",
    );

    let revised = open(
        &hub,
        &workspace,
        "Revise the durable store",
        "open-store-v2",
    );
    hub.record(&RecordRequest {
        session_id: revised.clone(),
        kind: RecordKind::Decision,
        content: "Use SQLite WAL for durable state.".to_owned(),
        evidence: Some("Concurrent writer and restart tests".to_owned()),
        supersedes_record_id: Some(old_decision.record_id.clone()),
        verifies_effect_id: None,
        idempotency_key: "decision-store-v2".to_owned(),
    })
    .unwrap();
    complete(&hub, &revised, "Storage decision revised", "close-store-v2");

    drop(hub);
    let restarted = Hub::open(&database).unwrap();
    let capsule = restarted
        .recall(&RecallRequest {
            workspace: workspace.clone(),
            limit: Some(20),
        })
        .unwrap();
    let restart_memory = capsule
        .recent_records
        .iter()
        .any(|record| record.content.contains("SQLite WAL"));
    let stale_decision_suppressed = capsule
        .recent_records
        .iter()
        .all(|record| record.record_id != old_decision.record_id);

    let effect_session = open(
        &restarted,
        &workspace,
        "Change production state safely",
        "open-effect",
    );
    let effect = restarted
        .record(&RecordRequest {
            session_id: effect_session.clone(),
            kind: RecordKind::Effect,
            content: "The configuration was changed.".to_owned(),
            evidence: Some("Exact changed path and digest".to_owned()),
            supersedes_record_id: None,
            verifies_effect_id: None,
            idempotency_key: "effect".to_owned(),
        })
        .unwrap()
        .record;
    let unsupported_completion_rejected = restarted
        .close_session(&CloseRequest {
            session_id: effect_session.clone(),
            disposition: CloseDisposition::Completed,
            summary: "Claiming completion too early".to_owned(),
            next_action: None,
            idempotency_key: "premature-close".to_owned(),
        })
        .is_err();
    restarted
        .record(&RecordRequest {
            session_id: effect_session.clone(),
            kind: RecordKind::Verification,
            content: "The changed configuration was read back and matched.".to_owned(),
            evidence: Some("Read-back digest matched the intended digest".to_owned()),
            supersedes_record_id: None,
            verifies_effect_id: Some(effect.record_id),
            idempotency_key: "verify-effect".to_owned(),
        })
        .unwrap();
    complete(
        &restarted,
        &effect_session,
        "Effect linked to verification",
        "verified-close",
    );

    let interrupted = open(
        &restarted,
        &workspace,
        "Resume interrupted work",
        "open-interrupted",
    );
    let duplicate_work_rejected = restarted
        .open_session(&OpenRequest {
            workspace: workspace.clone(),
            objective: "  resume INTERRUPTED work  ".to_owned(),
            idempotency_key: "duplicate-objective".to_owned(),
        })
        .is_err();
    Connection::open(&database)
        .unwrap()
        .execute(
            "UPDATE sessions SET last_activity_at_unix_ms = 0 WHERE session_id = ?1",
            [&interrupted],
        )
        .unwrap();
    let reconciled = restarted
        .reconcile(&ReconcileRequest {
            workspace: workspace.clone(),
            stale_after_seconds: 60,
            idempotency_key: "reconcile-stale".to_owned(),
        })
        .unwrap();
    let abandoned_session_removed = reconciled
        .abandoned_sessions
        .iter()
        .any(|session| session.session_id == interrupted)
        && restarted
            .recall(&RecallRequest {
                workspace,
                limit: Some(20),
            })
            .unwrap()
            .active_sessions
            .is_empty();

    let checks = [
        restart_memory,
        stale_decision_suppressed,
        unsupported_completion_rejected,
        duplicate_work_rejected,
        abandoned_session_removed,
    ];
    let hardened_score = checks.iter().filter(|passed| **passed).count();
    let baseline_score = 1;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "suite": "frontier_failure_regressions_v1",
            "baseline_score": baseline_score,
            "hardened_score": hardened_score,
            "max_score": checks.len(),
            "checks": {
                "restart_memory": restart_memory,
                "stale_decision_suppressed": stale_decision_suppressed,
                "unsupported_completion_rejected": unsupported_completion_rejected,
                "duplicate_work_rejected": duplicate_work_rejected,
                "abandoned_session_removed": abandoned_session_removed
            }
        }))
        .unwrap()
    );
    assert_eq!(hardened_score, checks.len());
    assert!(hardened_score > baseline_score);
}

fn open(hub: &Hub, workspace: &str, objective: &str, key: &str) -> String {
    hub.open_session(&OpenRequest {
        workspace: workspace.to_owned(),
        objective: objective.to_owned(),
        idempotency_key: key.to_owned(),
    })
    .unwrap()
    .session_id
}

fn complete(hub: &Hub, session_id: &str, summary: &str, key: &str) {
    hub.close_session(&CloseRequest {
        session_id: session_id.to_owned(),
        disposition: CloseDisposition::Completed,
        summary: summary.to_owned(),
        next_action: None,
        idempotency_key: key.to_owned(),
    })
    .unwrap();
}
