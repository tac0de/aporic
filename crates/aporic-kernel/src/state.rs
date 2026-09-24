use crate::protocol::{EffectOutcome, Event, Grant, Reservation, VerificationResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_ID_BYTES: usize = 256;
pub const MAX_SCOPE_BYTES: usize = 1_024;
pub const MAX_REF_BYTES: usize = 2_048;
pub const MAX_INPUT_BYTES: usize = 65_536;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantState {
    pub grant: Grant,
    pub reserved_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectRecord {
    pub outcome: EffectOutcome,
    pub observation_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationRecord {
    pub verifier: String,
    pub result: VerificationResult,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservationState {
    pub reservation: Reservation,
    pub effect: Option<EffectRecord>,
    pub verification: Option<VerificationRecord>,
    pub abandonment_reason: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub revision: u64,
    pub grants: BTreeMap<String, GrantState>,
    pub reservations: BTreeMap<String, ReservationState>,
}

impl State {
    pub(crate) fn apply(&mut self, sequence: u64, event: &Event) -> Result<(), &'static str> {
        if sequence != self.revision + 1 {
            return Err("NONCONTIGUOUS_SEQUENCE");
        }

        match event {
            Event::AuthorityGranted { grant } => self.apply_grant(grant)?,
            Event::ActionReserved { reservation } => self.apply_reservation(reservation)?,
            Event::EffectRecorded {
                reservation_id,
                outcome,
                observation_ref,
            } => self.apply_effect(reservation_id, *outcome, observation_ref)?,
            Event::VerificationRecorded {
                reservation_id,
                verifier,
                result,
                evidence_ref,
            } => self.apply_verification(reservation_id, verifier, *result, evidence_ref)?,
            Event::ReservationAbandoned {
                reservation_id,
                reason,
            } => self.apply_abandonment(reservation_id, reason)?,
        }

        self.revision = sequence;
        Ok(())
    }

    fn apply_grant(&mut self, grant: &Grant) -> Result<(), &'static str> {
        validate_grant(grant)?;
        if self.grants.contains_key(&grant.grant_id) {
            return Err("GRANT_ID_CONFLICT");
        }
        self.grants.insert(
            grant.grant_id.clone(),
            GrantState {
                grant: grant.clone(),
                reserved_by: None,
            },
        );
        Ok(())
    }

    fn apply_reservation(&mut self, reservation: &Reservation) -> Result<(), &'static str> {
        validate_reservation(reservation)?;
        if self.reservations.contains_key(&reservation.reservation_id) {
            return Err("RESERVATION_ID_CONFLICT");
        }
        let grant = self
            .grants
            .get_mut(&reservation.grant_id)
            .ok_or("GRANT_NOT_FOUND")?;
        if grant.reserved_by.is_some() {
            return Err("GRANT_ALREADY_RESERVED");
        }
        if grant.grant.principal != reservation.principal {
            return Err("PRINCIPAL_MISMATCH");
        }
        if grant.grant.task_ref != reservation.task_ref {
            return Err("TASK_MISMATCH");
        }
        if grant.grant.session_ref != reservation.session_ref {
            return Err("SESSION_MISMATCH");
        }
        if grant.grant.profile_ref != reservation.profile_ref {
            return Err("PROFILE_MISMATCH");
        }
        if grant.grant.scope != reservation.scope {
            return Err("SCOPE_MISMATCH");
        }
        if grant.grant.action != reservation.action {
            return Err("ACTION_MISMATCH");
        }
        if grant.grant.input != reservation.input {
            return Err("INPUT_MISMATCH");
        }

        grant.reserved_by = Some(reservation.reservation_id.clone());
        self.reservations.insert(
            reservation.reservation_id.clone(),
            ReservationState {
                reservation: reservation.clone(),
                effect: None,
                verification: None,
                abandonment_reason: None,
            },
        );
        Ok(())
    }

    fn apply_effect(
        &mut self,
        reservation_id: &str,
        outcome: EffectOutcome,
        observation_ref: &str,
    ) -> Result<(), &'static str> {
        validate_id(reservation_id)?;
        validate_ref(observation_ref)?;
        let reservation = self
            .reservations
            .get_mut(reservation_id)
            .ok_or("RESERVATION_NOT_FOUND")?;
        if reservation.abandonment_reason.is_some() {
            return Err("RESERVATION_ABANDONED");
        }
        if reservation.effect.is_some() {
            return Err("EFFECT_ALREADY_RECORDED");
        }
        reservation.effect = Some(EffectRecord {
            outcome,
            observation_ref: observation_ref.into(),
        });
        Ok(())
    }

    fn apply_verification(
        &mut self,
        reservation_id: &str,
        verifier: &str,
        result: VerificationResult,
        evidence_ref: &str,
    ) -> Result<(), &'static str> {
        validate_id(reservation_id)?;
        validate_id(verifier)?;
        validate_ref(evidence_ref)?;
        let reservation = self
            .reservations
            .get_mut(reservation_id)
            .ok_or("RESERVATION_NOT_FOUND")?;
        if reservation.abandonment_reason.is_some() {
            return Err("RESERVATION_ABANDONED");
        }
        if reservation.effect.is_none() {
            return Err("EFFECT_REQUIRED");
        }
        if reservation.reservation.principal == verifier {
            return Err("SELF_VERIFICATION_FORBIDDEN");
        }
        if reservation.verification.is_some() {
            return Err("VERIFICATION_ALREADY_RECORDED");
        }
        reservation.verification = Some(VerificationRecord {
            verifier: verifier.into(),
            result,
            evidence_ref: evidence_ref.into(),
        });
        Ok(())
    }

    fn apply_abandonment(
        &mut self,
        reservation_id: &str,
        reason: &str,
    ) -> Result<(), &'static str> {
        validate_id(reservation_id)?;
        validate_ref(reason)?;
        let reservation = self
            .reservations
            .get_mut(reservation_id)
            .ok_or("RESERVATION_NOT_FOUND")?;
        if reservation.effect.is_some() {
            return Err("EFFECT_ALREADY_RECORDED");
        }
        if reservation.abandonment_reason.is_some() {
            return Err("RESERVATION_ALREADY_ABANDONED");
        }
        reservation.abandonment_reason = Some(reason.into());
        Ok(())
    }
}

fn validate_grant(grant: &Grant) -> Result<(), &'static str> {
    validate_id(&grant.grant_id)?;
    validate_id(&grant.principal)?;
    validate_id(&grant.task_ref)?;
    validate_id(&grant.session_ref)?;
    validate_id(&grant.profile_ref)?;
    validate_scope(&grant.scope)?;
    validate_id(&grant.action)?;
    validate_input(&grant.input)?;
    validate_ref(&grant.authority_ref)
}

fn validate_reservation(reservation: &Reservation) -> Result<(), &'static str> {
    validate_id(&reservation.reservation_id)?;
    validate_id(&reservation.grant_id)?;
    validate_id(&reservation.principal)?;
    validate_id(&reservation.task_ref)?;
    validate_id(&reservation.session_ref)?;
    validate_id(&reservation.profile_ref)?;
    validate_scope(&reservation.scope)?;
    validate_id(&reservation.action)?;
    validate_input(&reservation.input)
}

pub(crate) fn validate_request_text(value: &str) -> Result<(), &'static str> {
    validate_id(value)
}

fn validate_id(value: &str) -> Result<(), &'static str> {
    if value.trim().is_empty() || value.len() > MAX_ID_BYTES {
        return Err("INVALID_ID");
    }
    Ok(())
}

fn validate_scope(value: &str) -> Result<(), &'static str> {
    if value.trim().is_empty() || value.len() > MAX_SCOPE_BYTES {
        return Err("INVALID_SCOPE");
    }
    Ok(())
}

fn validate_ref(value: &str) -> Result<(), &'static str> {
    if value.trim().is_empty() || value.len() > MAX_REF_BYTES {
        return Err("INVALID_REFERENCE");
    }
    Ok(())
}

fn validate_input(value: &serde_json::Value) -> Result<(), &'static str> {
    let bytes = serde_json::to_vec(value).map_err(|_| "INVALID_INPUT")?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err("INPUT_TOO_LARGE");
    }
    Ok(())
}
