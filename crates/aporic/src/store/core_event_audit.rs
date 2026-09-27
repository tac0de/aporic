use std::collections::{BTreeMap, BTreeSet};

use rusqlite::TransactionBehavior;

use crate::domain::{
    ClaimOutcome, CoreEventAudit, DurableRecord, EpistemicClaim, EvidenceArtifact, EvidenceOutcome,
    InfluenceClass, OriginChannel, RecordOutcome, TaskOutcome,
};

use super::{
    Result, Store, load_project_claims, load_task, parse_evidence_grade_value,
    parse_evidence_kind_value, parse_influence_class, parse_origin_channel, parse_record_kind,
};

const SAMPLE_LIMIT: usize = 20;

#[derive(Default)]
struct Findings {
    mismatch_count: u64,
    mismatch_sample: Vec<String>,
    uncovered_legacy_count: u64,
    uncovered_runner_claim_count: u64,
    uncovered_sample: Vec<String>,
}

impl Findings {
    fn mismatch(&mut self, label: String) {
        self.mismatch_count += 1;
        if self.mismatch_sample.len() < SAMPLE_LIMIT {
            self.mismatch_sample.push(label);
        }
    }

    fn uncovered(&mut self, label: String, runner_claim: bool) {
        if runner_claim {
            self.uncovered_runner_claim_count += 1;
        } else {
            self.uncovered_legacy_count += 1;
        }
        if self.uncovered_sample.len() < SAMPLE_LIMIT {
            self.uncovered_sample.push(label);
        }
    }
}

fn compare<T: PartialEq>(
    kind: &str,
    expected: &BTreeMap<String, T>,
    projected: &BTreeMap<String, T>,
    runner_claims: &BTreeSet<String>,
    findings: &mut Findings,
) {
    for (id, expected_row) in expected {
        if projected.get(id) != Some(expected_row) {
            findings.mismatch(format!("{kind}:{id}"));
        }
    }
    for id in projected.keys() {
        if !expected.contains_key(id) {
            findings.uncovered(
                format!("{kind}:{id}"),
                kind == "claim" && runner_claims.contains(id),
            );
        }
    }
}

impl Store {
    pub fn audit_core_events(&self) -> Result<CoreEventAudit> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Deferred)?;
        let mut records = BTreeMap::<String, DurableRecord>::new();
        let mut evidence = BTreeMap::<String, EvidenceArtifact>::new();
        let mut claims = BTreeMap::<String, EpistemicClaim>::new();
        let mut tasks = BTreeMap::<String, crate::domain::CoordinatedTask>::new();
        let mut unsupported_event_count = 0_u64;

        {
            let mut statement = transaction.prepare(
                "SELECT kind, result_json FROM events WHERE kind IN (
                    'record_added', 'evidence_added', 'claim_asserted',
                    'task_created', 'task_claimed', 'task_completed', 'task_cancelled')
                 ORDER BY sequence ASC",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (kind, json) = row?;
                match kind.as_str() {
                    "record_added" => {
                        if let Ok(mut outcome) = serde_json::from_str::<RecordOutcome>(&json) {
                            // Migration 0007 tightened authority for old model-authored records.
                            if outcome.record.origin_channel == OriginChannel::McpAgent
                                && outcome.record.influence_class
                                    == InfluenceClass::HistoricalContext
                            {
                                outcome.record.influence_class = InfluenceClass::UntrustedContent;
                            }
                            records.insert(outcome.record.record_id.clone(), outcome.record);
                        } else {
                            unsupported_event_count += 1;
                        }
                    }
                    "evidence_added" => {
                        if let Ok(outcome) = serde_json::from_str::<EvidenceOutcome>(&json) {
                            evidence.insert(outcome.evidence.evidence_id.clone(), outcome.evidence);
                        } else {
                            unsupported_event_count += 1;
                        }
                    }
                    "claim_asserted" => {
                        if let Ok(mut outcome) = serde_json::from_str::<ClaimOutcome>(&json) {
                            outcome.claim.evidence_ids.sort();
                            outcome.claim.receipt_ids.sort();
                            claims.insert(outcome.claim.claim_id.clone(), outcome.claim);
                        } else {
                            unsupported_event_count += 1;
                        }
                    }
                    "task_created" | "task_claimed" | "task_completed" | "task_cancelled" => {
                        if let Ok(outcome) = serde_json::from_str::<TaskOutcome>(&json) {
                            tasks.insert(outcome.task.task_id.clone(), outcome.task);
                        } else {
                            unsupported_event_count += 1;
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }

        let projected_records = {
            let mut statement = transaction.prepare(
                "SELECT record_id, session_id, kind, content, evidence,
                        supersedes_record_id, verifies_effect_id, origin_channel,
                        influence_class, created_at_unix_ms FROM records",
            )?;
            statement
                .query_map([], |row| {
                    let record = DurableRecord {
                        record_id: row.get(0)?,
                        session_id: row.get(1)?,
                        kind: parse_record_kind(row.get(2)?)?,
                        content: row.get(3)?,
                        evidence: row.get(4)?,
                        supersedes_record_id: row.get(5)?,
                        verifies_effect_id: row.get(6)?,
                        origin_channel: parse_origin_channel(row.get(7)?)?,
                        influence_class: parse_influence_class(row.get(8)?)?,
                        created_at_unix_ms: row.get(9)?,
                    };
                    Ok((record.record_id.clone(), record))
                })?
                .collect::<std::result::Result<BTreeMap<_, _>, _>>()?
        };
        let projected_evidence = {
            let mut statement = transaction.prepare(
                "SELECT evidence_id, session_id, kind, grade, locator, canonical_locator,
                        summary, content_sha256, created_at_unix_ms FROM evidence_artifacts",
            )?;
            statement
                .query_map([], |row| {
                    let item = EvidenceArtifact {
                        evidence_id: row.get(0)?,
                        session_id: row.get(1)?,
                        kind: parse_evidence_kind_value(&row.get::<_, String>(2)?)?,
                        grade: parse_evidence_grade_value(&row.get::<_, String>(3)?)?,
                        locator: row.get(4)?,
                        canonical_locator: row.get(5)?,
                        summary: row.get(6)?,
                        content_sha256: row.get(7)?,
                        created_at_unix_ms: row.get(8)?,
                    };
                    Ok((item.evidence_id.clone(), item))
                })?
                .collect::<std::result::Result<BTreeMap<_, _>, _>>()?
        };
        let projected_claims = {
            let mut result = BTreeMap::new();
            let mut statement = transaction.prepare("SELECT project_id FROM projects")?;
            let project_ids = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            for project_id in project_ids {
                for claim in load_project_claims(&transaction, &project_id)? {
                    result.insert(claim.claim_id.clone(), claim);
                }
            }
            result
        };
        let projected_tasks = {
            let mut result = BTreeMap::new();
            let mut statement = transaction.prepare("SELECT task_id FROM tasks")?;
            let ids = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            for id in ids {
                if let Some(task) = load_task(&transaction, &id)? {
                    result.insert(id, task);
                }
            }
            result
        };
        let runner_claims = {
            let mut statement =
                transaction.prepare("SELECT DISTINCT claim_id FROM claim_receipts")?;
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<BTreeSet<_>, _>>()?
        };

        let mut findings = Findings::default();
        compare(
            "record",
            &records,
            &projected_records,
            &runner_claims,
            &mut findings,
        );
        compare(
            "evidence",
            &evidence,
            &projected_evidence,
            &runner_claims,
            &mut findings,
        );
        compare(
            "claim",
            &claims,
            &projected_claims,
            &runner_claims,
            &mut findings,
        );
        compare(
            "task",
            &tasks,
            &projected_tasks,
            &runner_claims,
            &mut findings,
        );
        transaction.commit()?;
        Ok(CoreEventAudit {
            covered_records: records.len() as u64,
            covered_evidence: evidence.len() as u64,
            covered_claims: claims.len() as u64,
            covered_tasks: tasks.len() as u64,
            uncovered_legacy_count: findings.uncovered_legacy_count,
            uncovered_runner_claim_count: findings.uncovered_runner_claim_count,
            unsupported_event_count,
            mismatch_count: findings.mismatch_count,
            mismatch_sample: findings.mismatch_sample,
            uncovered_sample: findings.uncovered_sample,
            covered_consistent: findings.mismatch_count == 0,
            scope_notice: "Checks decodable event snapshots for records, evidence, directly asserted claims, and latest task outcomes. Sessions and runner-derived claims are not replayed; legacy and unsupported rows are not attested.".to_owned(),
        })
    }
}
