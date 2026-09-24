//! Host-side model control for applying an M0 routing decision.
//!
//! This crate is outside the authorization kernel. It validates an explicit
//! tier-to-model mapping, produces a host-specific application plan, and keeps
//! a replayable audit of launch attempts. It never infers model names from
//! natural language and never claims that a merely planned change was applied.

use aporic_roles::RoutingTier;
use aporic_routing::RoutingDecision;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub const POLICY_SCHEMA_VERSION: u32 = 1;
pub const AUDIT_SCHEMA_VERSION: u32 = 1;
const MAX_AUDIT_BYTES: usize = 4 * 1_024 * 1_024;
const MAX_TEXT_BYTES: usize = 256;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(serde_json::Error),
    InvalidPolicy(&'static str),
    EnforcementUnavailable,
    CorruptAudit { line: usize, reason: String },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Json(error) => write!(formatter, "JSON error: {error}"),
            Self::InvalidPolicy(reason) => {
                write!(formatter, "invalid model-control policy: {reason}")
            }
            Self::EnforcementUnavailable => write!(
                formatter,
                "required model control is unavailable on this host"
            ),
            Self::CorruptAudit { line, reason } => write!(
                formatter,
                "corrupt model-control audit at line {line}: {reason}"
            ),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Enforcement {
    Advisory,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostCapability {
    AdvisoryOnly,
    CodexCliLaunch,
    AgentsApiSessionUpdate { session_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelTarget {
    pub model: String,
    pub reasoning_effort: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TierTargets {
    pub economy: ModelTarget,
    pub balanced: ModelTarget,
    pub deep: ModelTarget,
}

impl TierTargets {
    pub fn target(&self, tier: RoutingTier) -> &ModelTarget {
        match tier {
            RoutingTier::Economy => &self.economy,
            RoutingTier::Balanced => &self.balanced,
            RoutingTier::Deep => &self.deep,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelControlPolicy {
    pub schema_version: u32,
    pub enforcement: Enforcement,
    pub codex_executable: PathBuf,
    pub tiers: TierTargets,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ApplicationPlan {
    Advisory,
    CodexCliLaunch {
        executable: PathBuf,
        arguments: Vec<String>,
    },
    AgentsApiSessionUpdate {
        session_id: String,
        method: String,
        path: String,
        body: serde_json::Value,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelControlPlan {
    pub tier: RoutingTier,
    pub model: String,
    pub reasoning_effort: String,
    pub reasons: Vec<String>,
    pub enforcement: Enforcement,
    pub application: ApplicationPlan,
}

pub fn validate_policy(policy: &ModelControlPolicy) -> Result<()> {
    if policy.schema_version != POLICY_SCHEMA_VERSION {
        return Err(Error::InvalidPolicy("unsupported schema version"));
    }
    if !policy.codex_executable.is_absolute() {
        return Err(Error::InvalidPolicy(
            "Codex executable path must be absolute",
        ));
    }
    for target in [
        &policy.tiers.economy,
        &policy.tiers.balanced,
        &policy.tiers.deep,
    ] {
        validate_atom(&target.model, "model")?;
        validate_effort(&target.reasoning_effort)?;
    }
    Ok(())
}

pub fn canonicalize_policy(mut policy: ModelControlPolicy) -> Result<ModelControlPolicy> {
    validate_policy(&policy)?;
    let executable = std::fs::canonicalize(&policy.codex_executable)?;
    if !std::fs::metadata(&executable)?.is_file() {
        return Err(Error::InvalidPolicy(
            "Codex executable must resolve to a regular file",
        ));
    }
    policy.codex_executable = executable;
    Ok(policy)
}

pub fn plan(
    policy: &ModelControlPolicy,
    decision: &RoutingDecision,
    capability: HostCapability,
) -> Result<ModelControlPlan> {
    validate_policy(policy)?;
    let target = policy.tiers.target(decision.tier);
    let application = match capability {
        HostCapability::AdvisoryOnly => {
            if policy.enforcement == Enforcement::Required {
                return Err(Error::EnforcementUnavailable);
            }
            ApplicationPlan::Advisory
        }
        HostCapability::CodexCliLaunch => ApplicationPlan::CodexCliLaunch {
            executable: policy.codex_executable.clone(),
            arguments: vec![
                "--model".into(),
                target.model.clone(),
                "--config".into(),
                format!("model_reasoning_effort=\"{}\"", target.reasoning_effort),
            ],
        },
        HostCapability::AgentsApiSessionUpdate { session_id } => {
            validate_atom(&session_id, "session id")?;
            ApplicationPlan::AgentsApiSessionUpdate {
                path: format!("/v1/agents/sessions/{session_id}"),
                session_id,
                method: "POST".into(),
                body: serde_json::json!({
                    "agent": {
                        "model": target.model,
                        "reasoning": {"effort": target.reasoning_effort}
                    }
                }),
            }
        }
    };
    let plan = ModelControlPlan {
        tier: decision.tier,
        model: target.model.clone(),
        reasoning_effort: target.reasoning_effort.clone(),
        reasons: decision.reasons.clone(),
        enforcement: policy.enforcement,
        application,
    };
    validate_plan(&plan)?;
    Ok(plan)
}

pub fn validate_plan(plan: &ModelControlPlan) -> Result<()> {
    validate_atom(&plan.model, "model")?;
    validate_effort(&plan.reasoning_effort)?;
    if plan.reasons.is_empty() || plan.reasons.len() > 16 {
        return Err(Error::InvalidPolicy(
            "routing reasons must contain between 1 and 16 entries",
        ));
    }
    for reason in &plan.reasons {
        validate_atom(reason, "identifier")?;
    }
    match &plan.application {
        ApplicationPlan::Advisory => {
            if plan.enforcement == Enforcement::Required {
                return Err(Error::EnforcementUnavailable);
            }
        }
        ApplicationPlan::CodexCliLaunch {
            executable,
            arguments,
        } => {
            if !executable.is_absolute() {
                return Err(Error::InvalidPolicy(
                    "Codex executable path must be absolute",
                ));
            }
            let expected = vec![
                "--model".to_string(),
                plan.model.clone(),
                "--config".to_string(),
                format!("model_reasoning_effort=\"{}\"", plan.reasoning_effort),
            ];
            if *arguments != expected {
                return Err(Error::InvalidPolicy(
                    "Codex launch arguments do not match the selected target",
                ));
            }
        }
        ApplicationPlan::AgentsApiSessionUpdate {
            session_id,
            method,
            path,
            body,
        } => {
            validate_atom(session_id, "session id")?;
            let expected_path = format!("/v1/agents/sessions/{session_id}");
            let expected_body = serde_json::json!({
                "agent": {
                    "model": plan.model,
                    "reasoning": {"effort": plan.reasoning_effort}
                }
            });
            if method != "POST" || path != &expected_path || body != &expected_body {
                return Err(Error::InvalidPolicy(
                    "Agents API update does not match the selected target",
                ));
            }
        }
    }
    Ok(())
}

fn validate_atom(value: &str, label: &'static str) -> Result<()> {
    if value.is_empty()
        || value.len() > MAX_TEXT_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(Error::InvalidPolicy(match label {
            "model" => "model contains invalid characters",
            "reasoning effort" => "reasoning effort contains invalid characters",
            "session id" => "session id contains invalid characters",
            _ => "identifier contains invalid characters",
        }));
    }
    Ok(())
}

fn validate_effort(value: &str) -> Result<()> {
    if matches!(
        value,
        "none" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max" | "ultra"
    ) {
        Ok(())
    } else {
        Err(Error::InvalidPolicy("unsupported reasoning effort"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchOutcome {
    Exited { code: i32 },
    Signaled,
    SpawnFailed { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuditEvent {
    Planned {
        launch_id: String,
        plan: ModelControlPlan,
    },
    Finished {
        launch_id: String,
        outcome: LaunchOutcome,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredAuditEvent {
    pub schema_version: u32,
    pub sequence: u64,
    pub event: AuditEvent,
}

#[derive(Debug, Default)]
pub struct Audit {
    records: Vec<StoredAuditEvent>,
}

impl Audit {
    pub fn records(&self) -> &[StoredAuditEvent] {
        &self.records
    }
}

pub fn initialize_audit(path: impl AsRef<Path>) -> Result<()> {
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.sync_all()?;
    Ok(())
}

pub fn load_audit(path: impl AsRef<Path>) -> Result<Audit> {
    let mut file = OpenOptions::new().read(true).open(path)?;
    file.lock_shared()?;
    let result = read_audit(&mut file);
    let unlock = file.unlock();
    match (result, unlock) {
        (Ok(audit), Ok(())) => Ok(audit),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(Error::Io(error)),
    }
}

pub fn append_audit(path: impl AsRef<Path>, event: AuditEvent) -> Result<u64> {
    let mut file = OpenOptions::new().read(true).append(true).open(path)?;
    file.lock()?;
    let result = append_locked(&mut file, event);
    let unlock = file.unlock();
    match (result, unlock) {
        (Ok(sequence), Ok(())) => Ok(sequence),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(Error::Io(error)),
    }
}

fn append_locked(file: &mut File, event: AuditEvent) -> Result<u64> {
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.take((MAX_AUDIT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_AUDIT_BYTES {
        return Err(Error::CorruptAudit {
            line: 0,
            reason: "audit exceeds size limit".into(),
        });
    }
    let mut audit = replay_audit(&bytes)?;
    apply_audit(&audit, &event, audit.records.len() + 1)?;
    let sequence = audit.records.len() as u64 + 1;
    let stored = StoredAuditEvent {
        schema_version: AUDIT_SCHEMA_VERSION,
        sequence,
        event,
    };
    let encoded = serde_json::to_vec(&stored)?;
    if bytes.len() + encoded.len() + 1 > MAX_AUDIT_BYTES {
        return Err(Error::CorruptAudit {
            line: 0,
            reason: "audit exceeds size limit".into(),
        });
    }
    file.write_all(&encoded)?;
    file.write_all(b"\n")?;
    file.sync_data()?;
    audit.records.push(stored);
    Ok(sequence)
}

fn read_audit(file: &mut File) -> Result<Audit> {
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.take((MAX_AUDIT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_AUDIT_BYTES {
        return Err(Error::CorruptAudit {
            line: 0,
            reason: "audit exceeds size limit".into(),
        });
    }
    replay_audit(&bytes)
}

fn replay_audit(bytes: &[u8]) -> Result<Audit> {
    if bytes.is_empty() {
        return Ok(Audit::default());
    }
    if !bytes.ends_with(b"\n") {
        return Err(Error::CorruptAudit {
            line: bytes.iter().filter(|byte| **byte == b'\n').count() + 1,
            reason: "record is not newline-terminated".into(),
        });
    }
    let mut audit = Audit::default();
    for (index, line) in bytes[..bytes.len() - 1]
        .split(|byte| *byte == b'\n')
        .enumerate()
    {
        if line.is_empty() {
            return Err(Error::CorruptAudit {
                line: index + 1,
                reason: "blank records are not allowed".into(),
            });
        }
        let stored: StoredAuditEvent =
            serde_json::from_slice(line).map_err(|error| Error::CorruptAudit {
                line: index + 1,
                reason: error.to_string(),
            })?;
        if stored.schema_version != AUDIT_SCHEMA_VERSION || stored.sequence != index as u64 + 1 {
            return Err(Error::CorruptAudit {
                line: index + 1,
                reason: "invalid schema version or sequence".into(),
            });
        }
        apply_audit(&audit, &stored.event, index + 1)?;
        audit.records.push(stored);
    }
    Ok(audit)
}

fn apply_audit(audit: &Audit, event: &AuditEvent, line: usize) -> Result<()> {
    let mut planned = BTreeSet::new();
    let mut finished = BTreeSet::new();
    for record in &audit.records {
        match &record.event {
            AuditEvent::Planned { launch_id, .. } => {
                planned.insert(launch_id.as_str());
            }
            AuditEvent::Finished { launch_id, .. } => {
                finished.insert(launch_id.as_str());
            }
        }
    }
    let invalid = match event {
        AuditEvent::Planned { launch_id, plan } => {
            validate_atom(launch_id, "session id").is_err()
                || validate_plan(plan).is_err()
                || planned.contains(launch_id.as_str())
        }
        AuditEvent::Finished { launch_id, .. } => {
            !planned.contains(launch_id.as_str()) || finished.contains(launch_id.as_str())
        }
    };
    if invalid {
        return Err(Error::CorruptAudit {
            line,
            reason: "invalid launch lifecycle".into(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aporic_routing::RoutingDecision;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_PATH: AtomicU64 = AtomicU64::new(1);

    fn policy(enforcement: Enforcement) -> ModelControlPolicy {
        ModelControlPolicy {
            schema_version: 1,
            enforcement,
            codex_executable: PathBuf::from("/usr/bin/true"),
            tiers: TierTargets {
                economy: ModelTarget {
                    model: "economy-model".into(),
                    reasoning_effort: "low".into(),
                },
                balanced: ModelTarget {
                    model: "balanced-model".into(),
                    reasoning_effort: "medium".into(),
                },
                deep: ModelTarget {
                    model: "deep-model".into(),
                    reasoning_effort: "high".into(),
                },
            },
        }
    }

    fn decision() -> RoutingDecision {
        RoutingDecision {
            tier: RoutingTier::Deep,
            reasons: vec!["verification_failed".into()],
            capped: false,
        }
    }

    #[test]
    fn creates_exact_codex_and_agents_api_plans() {
        let cli = plan(
            &policy(Enforcement::Required),
            &decision(),
            HostCapability::CodexCliLaunch,
        )
        .unwrap();
        assert_eq!(cli.model, "deep-model");
        assert!(matches!(
            cli.application,
            ApplicationPlan::CodexCliLaunch { .. }
        ));

        let api = plan(
            &policy(Enforcement::Required),
            &decision(),
            HostCapability::AgentsApiSessionUpdate {
                session_id: "session_123".into(),
            },
        )
        .unwrap();
        let ApplicationPlan::AgentsApiSessionUpdate { body, .. } = api.application else {
            panic!("expected API plan")
        };
        assert_eq!(body["agent"]["reasoning"]["effort"], "high");
    }

    #[test]
    fn required_control_fails_closed_on_advisory_host() {
        assert!(matches!(
            plan(
                &policy(Enforcement::Required),
                &decision(),
                HostCapability::AdvisoryOnly
            ),
            Err(Error::EnforcementUnavailable)
        ));
    }

    #[test]
    fn audit_replays_only_valid_launch_lifecycles() {
        let sequence = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "aporic-model-control-{}-{sequence}.jsonl",
            std::process::id()
        ));
        initialize_audit(&path).unwrap();
        let selected = plan(
            &policy(Enforcement::Required),
            &decision(),
            HostCapability::CodexCliLaunch,
        )
        .unwrap();
        append_audit(
            &path,
            AuditEvent::Planned {
                launch_id: "launch-1".into(),
                plan: selected,
            },
        )
        .unwrap();
        append_audit(
            &path,
            AuditEvent::Finished {
                launch_id: "launch-1".into(),
                outcome: LaunchOutcome::Exited { code: 0 },
            },
        )
        .unwrap();
        assert_eq!(load_audit(&path).unwrap().records().len(), 2);
        std::fs::remove_file(path).unwrap();
    }
}
