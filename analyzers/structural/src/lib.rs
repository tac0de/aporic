use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Input {
    schema_version: u32,
    revision: u64,
    claims: Vec<Claim>,
    relations: Vec<Relation>,
    decision_bases: Vec<DecisionBasis>,
}

#[derive(Deserialize)]
struct Claim {
    id: String,
    status: Status,
    superseded: bool,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Status {
    Observed,
    Inferred,
    Hypothesized,
    Verified,
    Refuted,
    Stale,
}

#[derive(Deserialize)]
struct Relation {
    id: String,
    source_claim_id: String,
    target_claim_id: String,
    kind: RelationKind,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum RelationKind {
    Supports,
    Attacks,
    DependsOn,
    Contradicts,
}

#[derive(Deserialize)]
struct DecisionBasis {
    decision_id: String,
    claim_ids: Vec<String>,
}

#[derive(Serialize)]
struct Output {
    schema_version: u32,
    based_on_revision: u64,
    findings: Vec<Finding>,
}

#[derive(Serialize)]
struct Finding {
    code: &'static str,
    severity: Severity,
    subject_ids: Vec<String>,
    message: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Severity {
    Warning,
    Conflict,
}

#[unsafe(no_mangle)]
pub extern "C" fn alloc(length: i32) -> i32 {
    if length <= 0 {
        return -1;
    }
    let mut bytes = Vec::<u8>::with_capacity(length as usize);
    let pointer = bytes.as_mut_ptr();
    std::mem::forget(bytes);
    pointer as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn analyze(pointer: i32, length: i32) -> i64 {
    if pointer < 0 || length <= 0 {
        return 0;
    }
    let input_bytes = unsafe { std::slice::from_raw_parts(pointer as *const u8, length as usize) };
    let Ok(input) = serde_json::from_slice::<Input>(input_bytes) else {
        return 0;
    };
    if input.schema_version != 1 {
        return 0;
    }
    let output = analyze_input(input);
    let Ok(mut bytes) = serde_json::to_vec(&output) else {
        return 0;
    };
    let output_pointer = bytes.as_mut_ptr() as u32;
    let output_length = bytes.len() as u32;
    std::mem::forget(bytes);
    ((u64::from(output_pointer) << 32) | u64::from(output_length)) as i64
}

fn analyze_input(input: Input) -> Output {
    let claims: BTreeMap<_, _> = input
        .claims
        .iter()
        .map(|claim| (claim.id.as_str(), claim))
        .collect();
    let mut findings = Vec::new();
    for relation in &input.relations {
        let Some(source) = claims.get(relation.source_claim_id.as_str()) else {
            continue;
        };
        let Some(target) = claims.get(relation.target_claim_id.as_str()) else {
            continue;
        };
        if relation.kind == RelationKind::Contradicts && active(source) && active(target) {
            findings.push(Finding {
                code: "active_contradiction",
                severity: Severity::Conflict,
                subject_ids: vec![relation.id.clone(), source.id.clone(), target.id.clone()],
                message: "Two active claims have an explicit contradiction relation.",
            });
        }
        if relation.kind == RelationKind::DependsOn && active(source) && invalidated(target) {
            findings.push(Finding {
                code: "invalidated_premise",
                severity: Severity::Warning,
                subject_ids: vec![relation.id.clone(), source.id.clone(), target.id.clone()],
                message: "An active claim relies on a refuted, stale, or superseded premise.",
            });
        }
        if relation.kind == RelationKind::Supports && invalidated(source) && active(target) {
            findings.push(Finding {
                code: "invalidated_support",
                severity: Severity::Warning,
                subject_ids: vec![relation.id.clone(), source.id.clone(), target.id.clone()],
                message: "An active claim is supported by a refuted, stale, or superseded claim.",
            });
        }
        if relation.kind == RelationKind::Attacks
            && source.status == Status::Verified
            && target.status == Status::Verified
            && !source.superseded
            && !target.superseded
        {
            findings.push(Finding {
                code: "verified_attack_conflict",
                severity: Severity::Conflict,
                subject_ids: vec![relation.id.clone(), source.id.clone(), target.id.clone()],
                message: "A verified claim attacks another verified claim.",
            });
        }
    }
    for basis in &input.decision_bases {
        for claim_id in &basis.claim_ids {
            if claims
                .get(claim_id.as_str())
                .is_some_and(|claim| invalidated(claim))
            {
                findings.push(Finding {
                    code: "invalidated_decision_basis",
                    severity: Severity::Warning,
                    subject_ids: vec![basis.decision_id.clone(), claim_id.clone()],
                    message: "A decision basis contains a refuted, stale, or superseded claim.",
                });
            }
        }
    }
    Output {
        schema_version: 1,
        based_on_revision: input.revision,
        findings,
    }
}

fn active(claim: &Claim) -> bool {
    !claim.superseded && !matches!(claim.status, Status::Refuted | Status::Stale)
}

fn invalidated(claim: &Claim) -> bool {
    claim.superseded || matches!(claim.status, Status::Refuted | Status::Stale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_explicit_conflicts_and_invalidated_dependencies() {
        let output = analyze_input(Input {
            schema_version: 1,
            revision: 9,
            claims: vec![
                Claim {
                    id: "active".into(),
                    status: Status::Verified,
                    superseded: false,
                },
                Claim {
                    id: "conflict".into(),
                    status: Status::Observed,
                    superseded: false,
                },
                Claim {
                    id: "premise".into(),
                    status: Status::Refuted,
                    superseded: false,
                },
            ],
            relations: vec![
                Relation {
                    id: "contradiction".into(),
                    source_claim_id: "active".into(),
                    target_claim_id: "conflict".into(),
                    kind: RelationKind::Contradicts,
                },
                Relation {
                    id: "dependency".into(),
                    source_claim_id: "active".into(),
                    target_claim_id: "premise".into(),
                    kind: RelationKind::DependsOn,
                },
            ],
            decision_bases: vec![DecisionBasis {
                decision_id: "decision".into(),
                claim_ids: vec!["premise".into()],
            }],
        });
        let codes = output
            .findings
            .iter()
            .map(|finding| finding.code)
            .collect::<Vec<_>>();
        assert_eq!(
            codes,
            vec![
                "active_contradiction",
                "invalidated_premise",
                "invalidated_decision_basis"
            ]
        );
    }
}
