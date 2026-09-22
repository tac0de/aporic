use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const POLICY_SCHEMA_VERSION: u32 = 2;
pub const MIN_POLICY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyDocument {
    pub schema_version: u32,
    pub tools: BTreeMap<String, ToolPolicy>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolPolicy {
    pub require_plan: bool,
    pub require_grant: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_intent: Option<bool>,
}

impl ToolPolicy {
    pub fn requires_intent(&self) -> bool {
        self.require_intent.unwrap_or(false)
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
        if self.tools.is_empty() {
            return Err("policy must protect at least one tool".into());
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
            if self.schema_version == 1 && policy.require_intent.is_some() {
                return Err(format!(
                    "tool {name} cannot declare require_intent under policy schema 1"
                ));
            }
            if self.schema_version == 2 && policy.require_intent.is_none() {
                return Err(format!(
                    "tool {name} must declare require_intent under policy schema 2"
                ));
            }
            if policy.requires_intent() && !policy.require_plan && !policy.require_grant {
                return Err(format!(
                    "tool {name} requires plan or grant enforcement when require_intent is true"
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
            },
        );
        let policy = Self {
            schema_version: POLICY_SCHEMA_VERSION,
            tools,
        };
        policy.validate()?;
        Ok(policy)
    }

    pub fn tool(&self, name: &str) -> Option<&ToolPolicy> {
        self.tools.get(name)
    }
}
