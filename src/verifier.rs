use crate::{
    Actor, ActorKind, CommitRequest, Error, Event, VerificationResult, transact_nonblocking,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const VERIFIER_REPORT_SCHEMA_VERSION: u32 = 1;
pub const MAX_VERIFIER_REPORT_BYTES: usize = 65_536;
pub const MAX_RECEIPT_ID_BYTES: usize = 8_192;
const MAX_REPORT_FIELD_BYTES: usize = 512;
const MAX_EVIDENCE_REFS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifierReportInput {
    pub schema_version: u32,
    pub verification_id: String,
    pub receipt_id: String,
    pub verifier_id: String,
    pub provenance: String,
    pub plan_id: String,
    pub check_index: u32,
    pub result: VerificationResult,
    pub evidence_refs: Vec<String>,
}

impl VerifierReportInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != VERIFIER_REPORT_SCHEMA_VERSION {
            return Err("unsupported verifier report schema");
        }
        let fields = [
            &self.verification_id,
            &self.verifier_id,
            &self.provenance,
            &self.plan_id,
        ];
        if fields
            .iter()
            .any(|field| field.trim().is_empty() || field.len() > MAX_REPORT_FIELD_BYTES)
        {
            return Err("verifier report fields must be non-empty and bounded");
        }
        if self.receipt_id.trim().is_empty() || self.receipt_id.len() > MAX_RECEIPT_ID_BYTES {
            return Err("verifier receipt id must be non-empty and bounded");
        }
        if self.evidence_refs.is_empty() || self.evidence_refs.len() > MAX_EVIDENCE_REFS {
            return Err("verifier report requires a bounded evidence set");
        }
        if self.evidence_refs.iter().enumerate().any(|(index, value)| {
            value.trim().is_empty()
                || value.len() > MAX_REPORT_FIELD_BYTES
                || self.evidence_refs[..index].contains(value)
        }) {
            return Err("verifier evidence references must be unique, non-empty, and bounded");
        }
        Ok(())
    }
}

pub fn ingest_verifier_report(
    path: impl AsRef<Path>,
    scope: &str,
    report: &VerifierReportInput,
) -> crate::Result<()> {
    report
        .validate()
        .map_err(|reason| Error::Invariant(reason.into()))?;
    if scope.trim().is_empty() || scope.len() > crate::codex::MAX_SCOPE_BYTES {
        return Err(Error::Invariant("scope is empty or too large".into()));
    }

    transact_nonblocking(path, |log| {
        if let Some(existing) = log.state().verifications.get(&report.verification_id) {
            if existing.scope == scope
                && existing.plan_id == report.plan_id
                && existing.check_index == report.check_index
                && existing.result == report.result
                && existing.evidence_refs == report.evidence_refs
                && existing.effect_receipt_id.as_deref() == Some(&report.receipt_id)
                && existing.verifier_id.as_deref() == Some(&report.verifier_id)
                && existing.verifier_provenance.as_deref() == Some(&report.provenance)
            {
                return Ok(((), None));
            }
            return Err(Error::Invariant(
                "verification_id already belongs to another verifier report".into(),
            ));
        }

        let identity = format!("effect-verification:{}", report.verification_id);
        Ok((
            (),
            Some(CommitRequest {
                schema_version: crate::SCHEMA_VERSION,
                event_id: identity.clone(),
                idempotency_key: identity,
                expected_revision: log.state().revision,
                actor: Actor {
                    kind: ActorKind::Evidence,
                    id: report.verifier_id.clone(),
                    provenance: report.provenance.clone(),
                },
                scope: scope.into(),
                event: Event::EffectVerificationRecorded {
                    verification_id: report.verification_id.clone(),
                    receipt_id: report.receipt_id.clone(),
                    verifier_id: report.verifier_id.clone(),
                    plan_id: report.plan_id.clone(),
                    check_index: report.check_index,
                    result: report.result,
                    evidence_refs: report.evidence_refs.clone(),
                },
            }),
        ))
    })
}
