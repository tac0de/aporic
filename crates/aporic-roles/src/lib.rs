//! Loads a bounded Markdown + JSON role package and compiles it into a profile
//! that can only narrow host-provided capabilities and limits.
//!
//! Roles are behavioral configuration, not authority. The host remains
//! responsible for enforcing the returned profile and authenticating callers.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub const ROLE_SCHEMA_VERSION: u32 = 1;
pub const MAX_ROLE_JSON_BYTES: usize = 16 * 1_024;
pub const MAX_ROLE_MARKDOWN_BYTES: usize = 4 * 1_024;
const MAX_CAPABILITIES: usize = 64;
const MAX_TOOL_CALLS: u32 = 10_000;
const MAX_PARALLEL_TASKS: u16 = 64;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(serde_json::Error),
    InvalidPackage(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Json(error) => write!(formatter, "JSON error: {error}"),
            Self::InvalidPackage(reason) => write!(formatter, "invalid role package: {reason}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingTier {
    Economy,
    Balanced,
    Deep,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingPolicy {
    pub default: RoutingTier,
    pub ceiling: RoutingTier,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleLimits {
    pub max_tool_calls: u32,
    pub max_parallel_tasks: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportFormat {
    DecisionCard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportPolicy {
    pub format: ReportFormat,
    pub max_items: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleDocument {
    pub schema_version: u32,
    pub role_id: String,
    pub role_version: u32,
    pub capabilities: Vec<String>,
    pub routing: RoutingPolicy,
    pub limits: RoleLimits,
    pub may_delegate: bool,
    pub report: ReportPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostPolicy {
    pub capabilities: BTreeSet<String>,
    pub default_routing: RoutingTier,
    pub routing_ceiling: RoutingTier,
    pub max_tool_calls: u32,
    pub max_parallel_tasks: u16,
    pub may_delegate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecutionProfile {
    role_id: String,
    role_version: u32,
    source_sha256: String,
    profile_sha256: String,
    instructions: String,
    capabilities: Vec<String>,
    unavailable_capabilities: Vec<String>,
    default_routing: RoutingTier,
    routing_ceiling: RoutingTier,
    max_tool_calls: u32,
    max_parallel_tasks: u16,
    may_delegate: bool,
    report: ReportPolicy,
}

impl ExecutionProfile {
    pub fn role_id(&self) -> &str {
        &self.role_id
    }

    pub fn role_version(&self) -> u32 {
        self.role_version
    }

    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }

    pub fn profile_sha256(&self) -> &str {
        &self.profile_sha256
    }

    pub fn instructions(&self) -> &str {
        &self.instructions
    }

    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }

    pub fn unavailable_capabilities(&self) -> &[String] {
        &self.unavailable_capabilities
    }

    pub fn default_routing(&self) -> RoutingTier {
        self.default_routing
    }

    pub fn routing_ceiling(&self) -> RoutingTier {
        self.routing_ceiling
    }

    pub fn max_tool_calls(&self) -> u32 {
        self.max_tool_calls
    }

    pub fn max_parallel_tasks(&self) -> u16 {
        self.max_parallel_tasks
    }

    pub fn may_delegate(&self) -> bool {
        self.may_delegate
    }

    pub fn report(&self) -> &ReportPolicy {
        &self.report
    }
}

pub fn load_role(directory: impl AsRef<Path>, host: &HostPolicy) -> Result<ExecutionProfile> {
    validate_host(host)?;
    let directory = directory.as_ref();
    let metadata = std::fs::symlink_metadata(directory)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::InvalidPackage(
            "role package path must be a real directory",
        ));
    }

    let json_bytes = read_regular_bounded(
        &directory.join("role.json"),
        MAX_ROLE_JSON_BYTES,
        "role.json",
    )?;
    let markdown_bytes = read_regular_bounded(
        &directory.join("role.md"),
        MAX_ROLE_MARKDOWN_BYTES,
        "role.md",
    )?;
    let role: RoleDocument = serde_json::from_slice(&json_bytes)?;
    validate_role(&role)?;
    let instructions = String::from_utf8(markdown_bytes.clone())
        .map_err(|_| Error::InvalidPackage("role.md must be valid UTF-8"))?;
    if instructions.trim().is_empty() {
        return Err(Error::InvalidPackage("role.md must not be empty"));
    }

    let requested = role.capabilities.iter().cloned().collect::<BTreeSet<_>>();
    let capabilities: Vec<String> = requested
        .intersection(&host.capabilities)
        .cloned()
        .collect();
    let unavailable_capabilities: Vec<String> =
        requested.difference(&host.capabilities).cloned().collect();
    let routing_ceiling = role.routing.ceiling.min(host.routing_ceiling);
    let default_routing = role
        .routing
        .default
        .min(host.default_routing)
        .min(routing_ceiling);

    let mut hasher = Sha256::new();
    hasher.update((json_bytes.len() as u64).to_be_bytes());
    hasher.update(&json_bytes);
    hasher.update((markdown_bytes.len() as u64).to_be_bytes());
    hasher.update(&markdown_bytes);
    let source_sha256 = format!("{:x}", hasher.finalize());

    let profile_sha256 = effective_profile_fingerprint(
        &role,
        &source_sha256,
        &capabilities,
        &unavailable_capabilities,
        default_routing,
        routing_ceiling,
        role.limits.max_tool_calls.min(host.max_tool_calls),
        role.limits.max_parallel_tasks.min(host.max_parallel_tasks),
        role.may_delegate && host.may_delegate,
    )?;

    Ok(ExecutionProfile {
        role_id: role.role_id,
        role_version: role.role_version,
        source_sha256,
        profile_sha256,
        instructions,
        capabilities,
        unavailable_capabilities,
        default_routing,
        routing_ceiling,
        max_tool_calls: role.limits.max_tool_calls.min(host.max_tool_calls),
        max_parallel_tasks: role.limits.max_parallel_tasks.min(host.max_parallel_tasks),
        may_delegate: role.may_delegate && host.may_delegate,
        report: role.report,
    })
}

#[derive(Serialize)]
struct EffectiveProfileFingerprint<'a> {
    role_id: &'a str,
    role_version: u32,
    source_sha256: &'a str,
    capabilities: &'a [String],
    unavailable_capabilities: &'a [String],
    default_routing: RoutingTier,
    routing_ceiling: RoutingTier,
    max_tool_calls: u32,
    max_parallel_tasks: u16,
    may_delegate: bool,
    report: &'a ReportPolicy,
}

#[allow(clippy::too_many_arguments)]
fn effective_profile_fingerprint(
    role: &RoleDocument,
    source_sha256: &str,
    capabilities: &[String],
    unavailable_capabilities: &[String],
    default_routing: RoutingTier,
    routing_ceiling: RoutingTier,
    max_tool_calls: u32,
    max_parallel_tasks: u16,
    may_delegate: bool,
) -> Result<String> {
    let material = EffectiveProfileFingerprint {
        role_id: &role.role_id,
        role_version: role.role_version,
        source_sha256,
        capabilities,
        unavailable_capabilities,
        default_routing,
        routing_ceiling,
        max_tool_calls,
        max_parallel_tasks,
        may_delegate,
        report: &role.report,
    };
    let encoded = serde_json::to_vec(&material)?;
    Ok(format!("{:x}", Sha256::digest(encoded)))
}

fn read_regular_bounded(path: &Path, limit: usize, name: &'static str) -> Result<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Error::InvalidPackage(match name {
            "role.json" => "role.json must be a real regular file",
            _ => "role.md must be a real regular file",
        }));
    }
    if metadata.len() > limit as u64 {
        return Err(Error::InvalidPackage(match name {
            "role.json" => "role.json exceeds the byte limit",
            _ => "role.md exceeds the byte limit",
        }));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)?
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(Error::InvalidPackage(match name {
            "role.json" => "role.json exceeds the byte limit",
            _ => "role.md exceeds the byte limit",
        }));
    }
    Ok(bytes)
}

fn validate_host(host: &HostPolicy) -> Result<()> {
    if host.max_tool_calls == 0
        || host.max_tool_calls > MAX_TOOL_CALLS
        || host.max_parallel_tasks == 0
        || host.max_parallel_tasks > MAX_PARALLEL_TASKS
        || host.default_routing > host.routing_ceiling
    {
        return Err(Error::InvalidPackage("host policy is invalid"));
    }
    for capability in &host.capabilities {
        validate_name(capability, "host capability is invalid")?;
    }
    Ok(())
}

fn validate_role(role: &RoleDocument) -> Result<()> {
    if role.schema_version != ROLE_SCHEMA_VERSION {
        return Err(Error::InvalidPackage("unsupported role schema version"));
    }
    validate_name(&role.role_id, "role id is invalid")?;
    if role.role_version == 0 {
        return Err(Error::InvalidPackage("role version must be positive"));
    }
    if role.capabilities.len() > MAX_CAPABILITIES {
        return Err(Error::InvalidPackage("too many role capabilities"));
    }
    let mut unique = BTreeSet::new();
    for capability in &role.capabilities {
        validate_name(capability, "role capability is invalid")?;
        if !unique.insert(capability) {
            return Err(Error::InvalidPackage("duplicate role capability"));
        }
    }
    if role.routing.default > role.routing.ceiling {
        return Err(Error::InvalidPackage(
            "default routing exceeds the role ceiling",
        ));
    }
    if role.limits.max_tool_calls == 0 || role.limits.max_tool_calls > MAX_TOOL_CALLS {
        return Err(Error::InvalidPackage("max_tool_calls is invalid"));
    }
    if role.limits.max_parallel_tasks == 0 || role.limits.max_parallel_tasks > MAX_PARALLEL_TASKS {
        return Err(Error::InvalidPackage("max_parallel_tasks is invalid"));
    }
    if role.report.max_items == 0 || role.report.max_items > 8 {
        return Err(Error::InvalidPackage("report max_items is invalid"));
    }
    Ok(())
}

fn validate_name(value: &str, reason: &'static str) -> Result<()> {
    let valid = !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        });
    if !valid {
        return Err(Error::InvalidPackage(reason));
    }
    Ok(())
}
