//! Versioned, declarative product-check categories.
//!
//! Category mappings describe what a check claims to exercise. They do not
//! establish assertion quality or authorize host actions.

use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::store::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfileDefinitionRequest {
    pub profile_id: String,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfileCategory {
    pub id: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfileSelection {
    pub id: String,
    pub version: u32,
    #[serde(default)]
    pub not_applicable: Vec<ProfileExclusion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfileExclusion {
    pub category: String,
    pub reason: String,
}

const GENERAL: &[(&str, &str)] = &[
    (
        "general.execution",
        "Declared execution commands, build inputs, dependencies and supported environments are exercised.",
    ),
    (
        "general.traceability",
        "Requirements, scenarios, design, implementation and tests have explicit dependency and coverage mappings.",
    ),
    (
        "general.verification",
        "Checks bind their complete input closure and execution specification to successful local-runner artifacts.",
    ),
    (
        "general.recovery",
        "Scenario failure states and recovery behavior are exercised, including incomplete delivery.",
    ),
];

const WEB: &[(&str, &str)] = &[
    (
        "web.accessibility",
        "Keyboard operation, focus, semantics and accessible names are exercised.",
    ),
    (
        "web.responsive",
        "Supported viewport sizes, zoom and content reflow are exercised.",
    ),
    (
        "web.browser_compatibility",
        "Declared browsers and supported browser capabilities are exercised.",
    ),
    (
        "web.security",
        "Declared web trust boundaries, untrusted inputs and sensitive-data handling are exercised.",
    ),
    (
        "web.network_failure",
        "Offline, timeout, interrupted requests and retry or recovery behavior are exercised.",
    ),
    (
        "web.loading_empty",
        "Loading, empty and unavailable-data states have observable outcomes and recovery behavior.",
    ),
];

const WEB_GAME: &[(&str, &str)] = &[
    (
        "web_game.state",
        "Game state transitions, restart and deterministic outcomes are exercised.",
    ),
    (
        "web_game.input",
        "Supported input devices, repeated input, lost focus and input ordering are exercised.",
    ),
    (
        "web_game.save",
        "Save/load round trips, missing or corrupt saves and schema compatibility are exercised.",
    ),
    (
        "web_game.time",
        "Clock progression, pause/resume, elapsed-time limits and frame-rate variation are exercised.",
    ),
    (
        "web_game.visibility",
        "Hidden tabs, visibility changes and resuming foreground play are exercised.",
    ),
    (
        "web_game.audio",
        "Audio enablement, mute, suspended playback and user-gesture requirements are exercised.",
    ),
    (
        "web_game.performance",
        "Declared frame, memory and asset-loading budgets are exercised under supported workloads.",
    ),
];

const WEB_PLATFORM: &[(&str, &str)] = &[
    (
        "web_platform.permissions",
        "Allowed and denied operations are exercised across declared actor and resource boundaries.",
    ),
    (
        "web_platform.integrity",
        "Data invariants, validation and partial-write or rollback behavior are exercised.",
    ),
    (
        "web_platform.idempotency",
        "Repeated requests and retries preserve the declared operation semantics.",
    ),
    (
        "web_platform.concurrency",
        "Concurrent updates, ordering conflicts and recovery preserve declared invariants.",
    ),
    (
        "web_platform.integration_failure",
        "External-service failures, malformed responses and degraded recovery paths are exercised.",
    ),
];

/// Return the complete category definition for a supported profile version.
/// Specialized web profiles include general and web categories exactly once.
pub fn definitions(id: &str, version: u32) -> Result<Vec<ProfileCategory>> {
    if version != 1 {
        return Err(Error::Invalid(format!(
            "unsupported delivery profile version {version} for {id}"
        )));
    }
    let extra = match id {
        "general" | "web" => &[][..],
        "web_game" => WEB_GAME,
        "web_platform" => WEB_PLATFORM,
        _ => return Err(Error::Invalid(format!("unknown delivery profile {id}"))),
    };
    let mut categories = GENERAL.to_vec();
    if id != "general" {
        categories.extend_from_slice(WEB);
    }
    categories.extend_from_slice(extra);
    Ok(categories
        .into_iter()
        .map(|(id, description)| ProfileCategory {
            id: id.into(),
            description: description.into(),
        })
        .collect())
}

/// Resolve required categories after validating explicit exclusions.
/// General categories are foundational and cannot be excluded. Other
/// exclusions require a nonempty reason; its adequacy remains a human judgment.
pub fn required_categories(selection: &ProfileSelection) -> Result<Vec<ProfileCategory>> {
    let categories = definitions(&selection.id, selection.version)?;
    let known: BTreeSet<_> = categories
        .iter()
        .map(|category| category.id.as_str())
        .collect();
    let mut excluded = BTreeSet::new();
    for exclusion in &selection.not_applicable {
        if !known.contains(exclusion.category.as_str()) {
            return Err(Error::Invalid(format!(
                "unknown category {} in profile {}",
                exclusion.category, selection.id
            )));
        }
        if exclusion.category.starts_with("general.") {
            return Err(Error::Invalid(format!(
                "foundational category {} cannot be excluded",
                exclusion.category
            )));
        }
        if exclusion.reason.trim().is_empty() {
            return Err(Error::Invalid(format!(
                "not-applicable category {} requires a reason",
                exclusion.category
            )));
        }
        if !excluded.insert(exclusion.category.as_str()) {
            return Err(Error::Invalid(format!(
                "duplicate not-applicable category {}",
                exclusion.category
            )));
        }
    }
    Ok(categories
        .into_iter()
        .filter(|category| !excluded.contains(category.id.as_str()))
        .collect())
}
