use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

pub const POLICY_SCHEMA_VERSION: u32 = 4;
pub const MIN_POLICY_SCHEMA_VERSION: u32 = 1;
pub const MAX_RISK_PROFILE_ID_BYTES: usize = 128;
pub const MAX_AUTO_ALLOWED_LOW_RISK_PROFILES: usize = 32;
pub const MAX_POLICY_TOOLS: usize = 256;
pub const MAX_POLICY_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleMode {
    Development,
    Maintenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyDocument {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle_mode: Option<LifecycleMode>,
    pub tools: BTreeMap<String, ToolPolicy>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LowRiskProfileRef {
    pub profile_id: String,
    pub profile_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolPolicy {
    pub require_plan: bool,
    pub require_grant: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_intent: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_allow_low_risk_profiles: Option<Vec<LowRiskProfileRef>>,
}

#[derive(Debug, Clone, Copy)]
pub struct EffectiveToolPolicy<'a> {
    pub require_plan: bool,
    pub require_grant: bool,
    pub require_intent: bool,
    pub auto_allow_low_risk_profiles: &'a [LowRiskProfileRef],
}

impl ToolPolicy {
    pub fn requires_intent(&self) -> bool {
        self.require_intent.unwrap_or(false)
    }

    pub fn auto_allowed_low_risk_profiles(&self) -> &[LowRiskProfileRef] {
        self.auto_allow_low_risk_profiles.as_deref().unwrap_or(&[])
    }
}

impl PolicyDocument {
    pub fn validate(&self) -> Result<(), String> {
        if !(MIN_POLICY_SCHEMA_VERSION..=POLICY_SCHEMA_VERSION).contains(&self.schema_version) {
            return Err(format!(
                "unsupported policy schema {}; expected {MIN_POLICY_SCHEMA_VERSION}..={POLICY_SCHEMA_VERSION}",
                self.schema_version
            ));
        }
        if self.tools.is_empty() || self.tools.len() > MAX_POLICY_TOOLS {
            return Err(format!("policy must protect 1..={MAX_POLICY_TOOLS} tools"));
        }
        if self.schema_version <= 3 && self.lifecycle_mode.is_some() {
            return Err(format!(
                "policy schema {} cannot declare lifecycle_mode",
                self.schema_version
            ));
        }
        if self.schema_version == 4 && self.lifecycle_mode.is_none() {
            return Err("policy schema 4 must declare lifecycle_mode".into());
        }
        for name in self.tools.keys() {
            if name.is_empty()
                || name.len() > crate::codex::MAX_TOOL_NAME_BYTES
                || !name.bytes().all(|byte| byte.is_ascii_graphic())
            {
                return Err(format!(
                    "tool names must be 1..={} visible ASCII bytes",
                    crate::codex::MAX_TOOL_NAME_BYTES
                ));
            }
        }
        for (name, policy) in &self.tools {
            if self.schema_version == 1
                && (policy.require_intent.is_some()
                    || policy.auto_allow_low_risk_profiles.is_some())
            {
                return Err(format!(
                    "tool {name} cannot declare newer policy fields under policy schema 1"
                ));
            }
            if self.schema_version == 2
                && (policy.require_intent.is_none()
                    || policy.auto_allow_low_risk_profiles.is_some())
            {
                return Err(format!(
                    "tool {name} must declare require_intent and cannot declare schema-3 fields under policy schema 2"
                ));
            }
            if matches!(self.schema_version, 3 | 4)
                && (policy.require_intent.is_none()
                    || policy.auto_allow_low_risk_profiles.is_none())
            {
                return Err(format!(
                    "tool {name} must declare require_intent and auto_allow_low_risk_profiles under policy schema 3 or 4"
                ));
            }
            if policy.requires_intent() && !policy.require_plan && !policy.require_grant {
                return Err(format!(
                    "tool {name} requires plan or grant enforcement when require_intent is true"
                ));
            }
            let profiles = policy.auto_allowed_low_risk_profiles();
            if profiles.len() > MAX_AUTO_ALLOWED_LOW_RISK_PROFILES
                || profiles.iter().any(|profile| {
                    [&profile.profile_id, &profile.profile_version]
                        .into_iter()
                        .any(|value| {
                            value.is_empty()
                                || value.len() > MAX_RISK_PROFILE_ID_BYTES
                                || !value.bytes().all(|byte| byte.is_ascii_graphic())
                        })
                })
                || profiles
                    .iter()
                    .enumerate()
                    .any(|(index, profile)| profiles[..index].iter().any(|prior| prior == profile))
            {
                return Err(format!(
                    "tool {name} may declare at most {MAX_AUTO_ALLOWED_LOW_RISK_PROFILES} unique auto-allowed risk profile id/version pairs using visible ASCII strings of 1..={MAX_RISK_PROFILE_ID_BYTES} bytes"
                ));
            }
            if !profiles.is_empty()
                && (!policy.require_plan || policy.require_grant || !policy.requires_intent())
            {
                return Err(format!(
                    "tool {name} low-risk auto-allow requires plan and intent enforcement and cannot bypass an execution grant"
                ));
            }
        }
        Ok(())
    }

    pub fn single(tool_name: impl Into<String>, require_plan: bool) -> Result<Self, String> {
        let mut tools = BTreeMap::new();
        tools.insert(
            tool_name.into(),
            ToolPolicy {
                require_plan,
                require_grant: false,
                require_intent: Some(false),
                auto_allow_low_risk_profiles: Some(Vec::new()),
            },
        );
        let policy = Self {
            schema_version: POLICY_SCHEMA_VERSION,
            lifecycle_mode: Some(LifecycleMode::Development),
            tools,
        };
        policy.validate()?;
        Ok(policy)
    }

    pub fn tool(&self, name: &str) -> Option<&ToolPolicy> {
        self.tools.get(name)
    }

    pub fn lifecycle_mode(&self) -> LifecycleMode {
        self.lifecycle_mode.unwrap_or(LifecycleMode::Development)
    }

    pub fn effective_tool(&self, name: &str) -> Option<EffectiveToolPolicy<'_>> {
        let policy = self.tool(name)?;
        Some(match self.lifecycle_mode() {
            LifecycleMode::Development => EffectiveToolPolicy {
                require_plan: policy.require_plan,
                require_grant: policy.require_grant,
                require_intent: policy.requires_intent(),
                auto_allow_low_risk_profiles: policy.auto_allowed_low_risk_profiles(),
            },
            LifecycleMode::Maintenance => EffectiveToolPolicy {
                require_plan: false,
                require_grant: true,
                require_intent: true,
                auto_allow_low_risk_profiles: &[],
            },
        })
    }
}

pub fn read_policy_document(path: &Path) -> Result<PolicyDocument, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|error| error.to_string())?
        .take((MAX_POLICY_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > MAX_POLICY_BYTES {
        return Err(format!("policy exceeds {MAX_POLICY_BYTES} bytes"));
    }
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}
