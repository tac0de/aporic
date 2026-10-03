//! Local execution bindings. MCP may inspect these, but cannot manufacture one.
use std::{collections::BTreeMap, io::Read, path::Path};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    Hub,
    bounded::sha256_file_bounded,
    delivery::{
        self, DeliveryCheck, DeliveryGap, DeliveryManifest, DeliveryValidateRequest,
        DeliveryValidationReport,
    },
    domain::{
        CommandSpec, EvidenceKind, EvidenceOutcome, EvidenceRequest, ExecutionGetRequest,
        ExecutionOutcome, ExecutionStatus,
    },
    store::{Error, Result},
};

const MAX_RESULT_BYTES: u64 = 1024 * 1024;
const MAX_PROGRAM_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeliveryRegisterRequest {
    pub session_id: String,
    pub manifest: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeliveryVerifyRequest {
    pub workspace: String,
    pub manifest: String,
    pub check_id: String,
    pub spec_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeliveryResult {
    pub check_id: String,
    pub input_sha256: BTreeMap<String, String>,
    pub execution_sha256: String,
    pub passed: bool,
}

// Constructed only by this crate's runner after direct pre/post observation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeliveryRunBinding {
    pub run_id: String,
    pub workspace: String,
    pub project_id: String,
    pub check_id: String,
    pub check_sha256: String,
    pub execution_sha256: String,
    pub command_spec_sha256: String,
    pub input_sha256: BTreeMap<String, String>,
    pub input_file_sha256: BTreeMap<String, String>,
    pub result_path: String,
    pub result_sha256: String,
    pub program_path: String,
    pub program_sha256: String,
    pub host_os: String,
    pub host_arch: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeliveryVerifyOutcome {
    pub execution: ExecutionOutcome,
    pub bound: bool,
    pub gaps: Vec<String>,
    pub notice: &'static str,
}

fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}

fn check_fingerprint(check: &DeliveryCheck) -> Result<String> {
    let mut canonical = check.clone();
    canonical.run_id = None;
    canonical.scenario_ids.sort();
    canonical.categories.sort();
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&canonical)?)
    ))
}

fn files_for_check(
    root: &Path,
    manifest: &DeliveryManifest,
    check: &DeliveryCheck,
) -> Result<BTreeMap<String, String>> {
    let inputs = delivery::check_input_fingerprints(manifest, check)?;
    let mut result = BTreeMap::new();
    for node in &manifest.nodes {
        if inputs.contains_key(&node.id)
            && let Some(file) = &node.file
        {
            let path = delivery::workspace_path(root, &file.path)?;
            let (digest, _) = sha256_file_bounded(
                &path,
                delivery::MAX_NODE_FILE_BYTES,
                "delivery prerequisite",
            )?;
            if digest != file.sha256 {
                return Err(invalid(format!("stale delivery prerequisite {}", node.id)));
            }
            result.insert(node.id.clone(), digest);
        }
    }
    Ok(result)
}

fn check_command(
    root: &Path,
    manifest: &DeliveryManifest,
    check: &DeliveryCheck,
    spec: &CommandSpec,
) -> Result<()> {
    let expected = manifest
        .execution
        .commands
        .get(&check.command_id)
        .ok_or_else(|| invalid("missing check command"))?;
    let actual: Vec<String> = std::iter::once(spec.program.clone())
        .chain(spec.args.clone())
        .collect();
    let cwd = manifest
        .execution
        .command_cwds
        .get(&check.command_id)
        .map(String::as_str)
        .unwrap_or(".");
    if Path::new(&spec.workspace) != root
        || actual != *expected
        || spec.workspace_relative_cwd != cwd
        || spec.expected_exit_code != 0
    {
        return Err(invalid(
            "delivery command must match runner workspace, exact argv, cwd and zero success exit",
        ));
    }
    Ok(())
}

fn read_result(root: &Path, relative: &str) -> Result<(DeliveryResult, String)> {
    let path = delivery::workspace_path(root, relative)?;
    if !path.metadata()?.is_file() {
        return Err(invalid("delivery result must be a regular file"));
    }
    let mut file = crate::bounded::open_regular_file(&path, "delivery result")?;
    if !file.metadata()?.is_file() {
        return Err(invalid("delivery result must be a regular file"));
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_RESULT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_RESULT_BYTES {
        return Err(invalid("delivery result exceeds byte limit"));
    }
    Ok((
        serde_json::from_slice(&bytes)?,
        format!("{:x}", Sha256::digest(&bytes)),
    ))
}

pub fn register(hub: &Hub, request: &DeliveryRegisterRequest) -> Result<EvidenceOutcome> {
    let workspace = hub.delivery_session_workspace(&request.session_id)?;
    let (root, _, sha) = delivery::read_manifest(&DeliveryValidateRequest {
        workspace,
        manifest: request.manifest.clone(),
    })?;
    hub.add_evidence(&EvidenceRequest {
        session_id: request.session_id.clone(),
        kind: EvidenceKind::WorkspaceFile,
        locator: delivery::workspace_path(&root, &request.manifest)?
            .to_string_lossy()
            .into_owned(),
        summary:
            "Versioned product delivery manifest bytes; no semantic approval or execution claim"
                .into(),
        content_sha256: Some(sha),
        idempotency_key: request.idempotency_key.clone(),
    })
}

/// Inventories metadata/files and revalidates private local-runner bindings.
pub fn validate(hub: &Hub, request: &DeliveryValidateRequest) -> Result<DeliveryValidationReport> {
    let mut report = delivery::validate(request)?;
    let (root, manifest, sha) = delivery::read_manifest(request)?;
    if sha != report.manifest_sha256 {
        return Err(invalid("delivery manifest changed during validation"));
    }
    for (check, status) in manifest.checks.iter().zip(&mut report.checks) {
        if !status.coverage_complete || !status.inputs_current {
            continue;
        }
        let proof = validate_binding(hub, &root, &manifest, check);
        if let Err(error) = proof {
            let message = error.to_string();
            status.gaps.push(message.clone());
            report.gaps.push(DeliveryGap {
                id: check.id.clone(),
                message,
            });
        } else if status.coverage_complete && status.inputs_current {
            status.execution_verified = true;
        }
    }
    report.valid = report.gaps.is_empty() && report.profile_gaps.is_empty();
    report.assurance_boundary = "Advisory delivery evidence. execution_verified means a matching local command succeeded, produced a fresh captured passed result and had matching declared inputs before/after execution. It does not prove semantic assertion adequacy, undeclared input coverage, browser/device identity, usability or product readiness. Host OS/architecture and executable bytes are observed; environment descriptions remain declarations.".into();
    Ok(report)
}

fn validate_binding(
    hub: &Hub,
    root: &Path,
    manifest: &DeliveryManifest,
    check: &DeliveryCheck,
) -> Result<()> {
    let run_id = check
        .run_id
        .as_deref()
        .ok_or_else(|| invalid("delivery check has no bound run_id"))?;
    let binding = hub
        .delivery_binding(run_id)?
        .ok_or_else(|| invalid("run has no runner-observed delivery input binding"))?;
    let outcome = hub.get_execution(&ExecutionGetRequest {
        run_id: run_id.into(),
    })?;
    let spec = hub.delivery_command(&outcome.run.spec_id)?;
    check_command(root, manifest, check, &spec)?;
    let receipt = outcome
        .receipt
        .as_ref()
        .ok_or_else(|| invalid("delivery execution has no receipt"))?;
    if outcome.run.status != ExecutionStatus::Succeeded
        || outcome.verified_claim_id.is_none()
        || receipt.command_spec_sha256 != spec.canonical_sha256
        || receipt.exit_code != Some(0)
        || receipt.termination != "exited"
    {
        return Err(invalid(
            "delivery execution is not a successful matching runner receipt",
        ));
    }
    let result_path = check
        .result_path
        .as_deref()
        .ok_or_else(|| invalid("check has no result_path"))?;
    let inputs = delivery::check_input_fingerprints(manifest, check)?;
    let execution = delivery::execution_fingerprint(&manifest.execution)?;
    if binding.workspace != root.to_string_lossy()
        || binding.project_id != manifest.project_id
        || binding.check_id != check.id
        || binding.check_sha256 != check_fingerprint(check)?
        || binding.execution_sha256 != execution
        || binding.command_spec_sha256 != spec.canonical_sha256
        || binding.input_sha256 != inputs
        || binding.input_file_sha256 != files_for_check(root, manifest, check)?
        || binding.result_path != result_path
        || binding.host_os != std::env::consts::OS
        || binding.host_arch != std::env::consts::ARCH
        || receipt.resolved_executable.as_deref() != Some(binding.program_path.as_str())
    {
        return Err(invalid(
            "delivery binding inputs, command, host or check changed",
        ));
    }
    if sha256_file_bounded(
        Path::new(&binding.program_path),
        MAX_PROGRAM_BYTES,
        "delivery executable",
    )?
    .0 != binding.program_sha256
    {
        return Err(invalid("delivery executable bytes changed"));
    }
    if crate::runner::resolve_executable(&spec.program, &root.join(&spec.workspace_relative_cwd))
        .as_deref()
        != Some(binding.program_path.as_str())
    {
        return Err(invalid("delivery executable resolution changed"));
    }
    let (result, digest) = read_result(root, result_path)?;
    if digest != binding.result_sha256
        || !outcome
            .artifacts
            .iter()
            .any(|a| a.workspace_relative_path == result_path && a.sha256 == digest)
        || !result.passed
        || result.check_id != check.id
        || result.input_sha256 != inputs
        || result.execution_sha256 != execution
    {
        return Err(invalid(
            "delivery result changed, failed or does not match captured receipt/input snapshot",
        ));
    }
    Ok(())
}

/// Public impact reports retain only eligibility backed by a current local run.
pub fn impact(
    hub: &Hub,
    request: &delivery::DeliveryImpactRequest,
) -> Result<delivery::DeliveryImpactReport> {
    let mut report = delivery::impact(request)?;
    let validation = validate(
        hub,
        &DeliveryValidateRequest {
            workspace: request.workspace.clone(),
            manifest: request.current.clone(),
        },
    )?;
    report.preserved_check_ids.retain(|id| {
        let verified = validation
            .checks
            .iter()
            .any(|check| &check.id == id && check.execution_verified);
        if !verified {
            report.invalidated_check_ids.push(id.clone());
        }
        verified
    });
    report.invalidated_check_ids.sort();
    report.invalidated_check_ids.dedup();
    report.reasons.push(
        "CLI/MCP preservation also requires current runner-only input/output/executable binding."
            .into(),
    );
    Ok(report)
}

/// Local-only execution. Fresh output is required, so capturing an old passed
/// JSON with a no-op process cannot establish a new delivery execution binding.
pub async fn verify(hub: &Hub, request: &DeliveryVerifyRequest) -> Result<DeliveryVerifyOutcome> {
    let validate_request = DeliveryValidateRequest {
        workspace: request.workspace.clone(),
        manifest: request.manifest.clone(),
    };
    let report = delivery::validate(&validate_request)?;
    let (root, manifest, sha) = delivery::read_manifest(&validate_request)?;
    if report.manifest_sha256 != sha {
        return Err(invalid("delivery manifest changed before execution"));
    }
    let check = manifest
        .checks
        .iter()
        .find(|c| c.id == request.check_id)
        .ok_or_else(|| invalid("unknown delivery check"))?;
    let status = report
        .checks
        .iter()
        .find(|c| c.id == check.id)
        .ok_or_else(|| invalid("missing check validation"))?;
    if !status.coverage_complete || !status.inputs_current {
        return Err(invalid(format!(
            "delivery check coverage/inputs incomplete: {}",
            status.gaps.join("; ")
        )));
    }
    let spec = hub.delivery_command(&request.spec_id)?;
    check_command(&root, &manifest, check, &spec)?;
    let result_path = check
        .result_path
        .as_deref()
        .ok_or_else(|| invalid("delivery check requires result_path"))?;
    if !spec.artifact_paths.iter().any(|p| p == result_path) {
        return Err(invalid("result_path must be a registered runner artifact"));
    }
    let output = delivery::workspace_path(&root, result_path)?;
    if std::fs::symlink_metadata(&output).is_ok() {
        return Err(invalid(
            "delivery result already exists; select a fresh output path before running",
        ));
    }
    let inputs = delivery::check_input_fingerprints(&manifest, check)?;
    let input_files = files_for_check(&root, &manifest, check)?;
    let execution_sha = delivery::execution_fingerprint(&manifest.execution)?;
    let program_before =
        crate::runner::resolve_executable(&spec.program, &root.join(&spec.workspace_relative_cwd))
            .ok_or_else(|| invalid("delivery executable cannot be resolved before execution"))?;
    let program_sha_before = sha256_file_bounded(
        Path::new(&program_before),
        MAX_PROGRAM_BYTES,
        "delivery executable",
    )?
    .0;
    let outcome = hub.verify(&request.spec_id).await?;
    let mut gaps = Vec::new();
    let post = (|| -> Result<DeliveryRunBinding> {
        if outcome.run.status != ExecutionStatus::Succeeded {
            return Err(invalid("delivery command did not succeed"));
        }
        let (_, after, after_sha) = delivery::read_manifest(&validate_request)?;
        if after_sha != sha
            || delivery::execution_fingerprint(&after.execution)? != execution_sha
            || files_for_check(&root, &after, check)? != input_files
        {
            return Err(invalid("delivery inputs changed during execution"));
        }
        let (result, result_sha) = read_result(&root, result_path)?;
        if !result.passed
            || result.check_id != check.id
            || result.input_sha256 != inputs
            || result.execution_sha256 != execution_sha
        {
            return Err(invalid(
                "delivery result failed or input/execution/check snapshot differs",
            ));
        }
        let receipt = outcome
            .receipt
            .as_ref()
            .ok_or_else(|| invalid("delivery receipt missing"))?;
        let program = receipt
            .resolved_executable
            .clone()
            .ok_or_else(|| invalid("delivery executable identity missing"))?;
        // Capture actual resolved executable bytes rather than PATH declarations.
        let program_sha = sha256_file_bounded(
            Path::new(&program),
            MAX_PROGRAM_BYTES,
            "delivery executable",
        )?
        .0;
        if program != program_before || program_sha != program_sha_before {
            return Err(invalid(
                "delivery executable identity changed during execution",
            ));
        }
        Ok(DeliveryRunBinding {
            run_id: outcome.run.run_id.clone(),
            workspace: root.to_string_lossy().into_owned(),
            project_id: manifest.project_id.clone(),
            check_id: check.id.clone(),
            check_sha256: check_fingerprint(check)?,
            execution_sha256: execution_sha,
            command_spec_sha256: spec.canonical_sha256.clone(),
            input_sha256: inputs,
            input_file_sha256: input_files,
            result_path: result_path.into(),
            result_sha256: result_sha,
            program_path: program,
            program_sha256: program_sha,
            host_os: std::env::consts::OS.into(),
            host_arch: std::env::consts::ARCH.into(),
        })
    })();
    let bound = match post {
        Ok(binding) => {
            hub.bind_delivery_run(&binding)?;
            true
        }
        Err(error) => {
            gaps.push(error.to_string());
            false
        }
    };
    Ok(DeliveryVerifyOutcome {
        execution: outcome,
        bound,
        gaps,
        notice: "Local command and declared input/output observations only; semantic assertion adequacy and undeclared dependencies are not proven.",
    })
}
