//! Deterministic routing recommendations from explicit, structured signals.
//!
//! This crate does not inspect natural language or change a host model. It
//! chooses a bounded tier that a capable host may apply and that Aporic records
//! with the reservation.

use aporic_roles::RoutingTier;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingSignals {
    pub requested: Option<RoutingTier>,
    pub multi_step: bool,
    pub uncertainty: bool,
    pub verification_failed: bool,
    pub retry_count: u8,
    pub burn_budget: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingDecision {
    pub tier: RoutingTier,
    pub reasons: Vec<String>,
    pub capped: bool,
}

pub fn select(
    default: RoutingTier,
    ceiling: RoutingTier,
    signals: &RoutingSignals,
) -> RoutingDecision {
    let mut selected = default.min(ceiling);
    let mut reasons = vec![format!("profile_default:{default:?}").to_lowercase()];

    if let Some(requested) = signals.requested {
        if requested > selected {
            selected = requested;
        }
        reasons.push(format!("explicit_request:{requested:?}").to_lowercase());
    }
    if signals.multi_step || signals.retry_count == 1 {
        selected = selected.max(RoutingTier::Balanced);
        reasons.push(if signals.multi_step {
            "multi_step".into()
        } else {
            "first_retry".into()
        });
    }
    if signals.uncertainty
        || signals.verification_failed
        || signals.retry_count >= 2
        || signals.burn_budget
    {
        selected = RoutingTier::Deep;
        if signals.uncertainty {
            reasons.push("material_uncertainty".into());
        }
        if signals.verification_failed {
            reasons.push("verification_failed".into());
        }
        if signals.retry_count >= 2 {
            reasons.push("repeated_failure".into());
        }
        if signals.burn_budget {
            reasons.push("explicit_burn_budget".into());
        }
    }

    let capped = selected > ceiling;
    if capped {
        selected = ceiling;
        reasons.push("profile_ceiling".into());
    }
    RoutingDecision {
        tier: selected,
        reasons,
        capped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conserves_by_default_and_escalates_only_on_explicit_signals() {
        let quiet = select(
            RoutingTier::Economy,
            RoutingTier::Deep,
            &RoutingSignals::default(),
        );
        assert_eq!(quiet.tier, RoutingTier::Economy);

        let burst = select(
            RoutingTier::Economy,
            RoutingTier::Deep,
            &RoutingSignals {
                burn_budget: true,
                ..RoutingSignals::default()
            },
        );
        assert_eq!(burst.tier, RoutingTier::Deep);
        assert!(burst.reasons.contains(&"explicit_burn_budget".into()));
    }

    #[test]
    fn role_ceiling_always_wins() {
        let decision = select(
            RoutingTier::Economy,
            RoutingTier::Balanced,
            &RoutingSignals {
                uncertainty: true,
                ..RoutingSignals::default()
            },
        );
        assert_eq!(decision.tier, RoutingTier::Balanced);
        assert!(decision.capped);
    }
}
