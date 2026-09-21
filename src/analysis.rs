use crate::{ArgumentRelation, DecisionBasis, EpistemicStatus, Error, Result, State};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;
use wasmtime::{Config, Engine, Instance, Module, Store, StoreLimits, StoreLimitsBuilder};

pub const ANALYZER_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_FUEL: u64 = 2_000_000;
pub const DEFAULT_MEMORY_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_FUEL: u64 = 50_000_000;
pub const MAX_MEMORY_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_TABLE_ELEMENTS: usize = 10_000;
pub const MAX_ANALYZER_INPUT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_ANALYZER_OUTPUT_BYTES: usize = 1024 * 1024;
pub const MAX_ANALYZER_MODULE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzerClaim {
    pub id: String,
    pub status: EpistemicStatus,
    pub superseded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzerDecisionBasis {
    pub decision_id: String,
    pub claim_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzerInput {
    pub schema_version: u32,
    pub revision: u64,
    pub claims: Vec<AnalyzerClaim>,
    pub relations: Vec<ArgumentRelation>,
    pub decision_bases: Vec<AnalyzerDecisionBasis>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Info,
    Warning,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzerFinding {
    pub code: String,
    pub severity: FindingSeverity,
    pub subject_ids: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyzerOutput {
    pub schema_version: u32,
    pub based_on_revision: u64,
    pub findings: Vec<AnalyzerFinding>,
}

struct RuntimeState {
    limits: StoreLimits,
}

impl AnalyzerInput {
    pub fn from_state(state: &State, scope: &str) -> Self {
        Self {
            schema_version: ANALYZER_SCHEMA_VERSION,
            revision: state.revision,
            claims: state
                .claims
                .values()
                .filter(|claim| claim.scope == scope)
                .map(|claim| AnalyzerClaim {
                    id: claim.id.clone(),
                    status: claim.status,
                    superseded: claim.superseded_by.is_some(),
                })
                .collect(),
            relations: state
                .argument_relations
                .values()
                .filter(|relation| relation.scope == scope && relation.active)
                .cloned()
                .collect(),
            decision_bases: state
                .decision_bases
                .values()
                .filter(|basis| basis.scope == scope)
                .map(AnalyzerDecisionBasis::from)
                .collect(),
        }
    }
}

impl From<&DecisionBasis> for AnalyzerDecisionBasis {
    fn from(value: &DecisionBasis) -> Self {
        Self {
            decision_id: value.decision_id.clone(),
            claim_ids: value.claim_ids.clone(),
        }
    }
}

pub fn run_wasm_analyzer(
    module_path: impl AsRef<Path>,
    input: &AnalyzerInput,
    fuel: u64,
    memory_bytes: usize,
) -> Result<AnalyzerOutput> {
    if fuel == 0 || memory_bytes == 0 || fuel > MAX_FUEL || memory_bytes > MAX_MEMORY_BYTES {
        return Err(Error::Invariant(
            "analyzer fuel and memory must be positive and within host limits".into(),
        ));
    }
    let input_bytes = serde_json::to_vec(input)?;
    if input_bytes.len() > MAX_ANALYZER_INPUT_BYTES {
        return Err(Error::Invariant("analyzer input exceeds byte limit".into()));
    }

    let mut config = Config::new();
    config.consume_fuel(true);
    config.max_wasm_stack(512 * 1024);
    let engine = Engine::new(&config).map_err(runtime_error)?;
    let module_bytes = std::fs::read(module_path)?;
    if module_bytes.len() > MAX_ANALYZER_MODULE_BYTES {
        return Err(Error::Invariant(
            "analyzer module exceeds byte limit".into(),
        ));
    }
    let module = Module::new(&engine, module_bytes).map_err(runtime_error)?;
    if module.imports().next().is_some() {
        return Err(Error::Invariant(
            "analyzer modules must not import host capabilities".into(),
        ));
    }

    let limits = StoreLimitsBuilder::new()
        .memory_size(memory_bytes)
        .instances(1)
        .memories(1)
        .tables(1)
        .table_elements(MAX_TABLE_ELEMENTS)
        .build();
    let mut store = Store::new(&engine, RuntimeState { limits });
    store.limiter(|state| &mut state.limits);
    store.set_fuel(fuel).map_err(runtime_error)?;
    let instance = Instance::new(&mut store, &module, &[]).map_err(runtime_error)?;
    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or_else(|| Error::Invariant("analyzer must export memory".into()))?;
    let allocate = instance
        .get_typed_func::<i32, i32>(&mut store, "alloc")
        .map_err(runtime_error)?;
    let analyze = instance
        .get_typed_func::<(i32, i32), i64>(&mut store, "analyze")
        .map_err(runtime_error)?;
    let input_len = i32::try_from(input_bytes.len())
        .map_err(|_| Error::Invariant("analyzer input length does not fit i32".into()))?;
    let input_ptr = allocate
        .call(&mut store, input_len)
        .map_err(runtime_error)?;
    if input_ptr < 0 {
        return Err(Error::Invariant(
            "analyzer returned a negative input pointer".into(),
        ));
    }
    memory
        .write(&mut store, input_ptr as usize, &input_bytes)
        .map_err(runtime_error)?;
    let packed = analyze
        .call(&mut store, (input_ptr, input_len))
        .map_err(runtime_error)? as u64;
    let output_ptr = (packed >> 32) as u32 as usize;
    let output_len = (packed & u64::from(u32::MAX)) as u32 as usize;
    if output_len == 0 || output_len > MAX_ANALYZER_OUTPUT_BYTES {
        return Err(Error::Invariant(
            "analyzer output is empty or exceeds byte limit".into(),
        ));
    }
    let mut output_bytes = vec![0_u8; output_len];
    memory
        .read(&store, output_ptr, &mut output_bytes)
        .map_err(runtime_error)?;
    let output: AnalyzerOutput = serde_json::from_slice(&output_bytes)?;
    validate_output(input, &output)?;
    Ok(output)
}

fn validate_output(input: &AnalyzerInput, output: &AnalyzerOutput) -> Result<()> {
    if output.schema_version != ANALYZER_SCHEMA_VERSION {
        return Err(Error::Invariant(
            "analyzer returned an unsupported schema version".into(),
        ));
    }
    if output.based_on_revision != input.revision {
        return Err(Error::Invariant(
            "analyzer output revision does not match its input".into(),
        ));
    }
    let known_subjects = input
        .claims
        .iter()
        .map(|claim| claim.id.as_str())
        .chain(input.relations.iter().map(|relation| relation.id.as_str()))
        .chain(
            input
                .decision_bases
                .iter()
                .map(|basis| basis.decision_id.as_str()),
        )
        .collect::<BTreeSet<_>>();
    for finding in &output.findings {
        if finding.code.trim().is_empty()
            || finding.message.trim().is_empty()
            || finding.subject_ids.is_empty()
            || finding
                .subject_ids
                .iter()
                .any(|value| value.trim().is_empty())
            || finding
                .subject_ids
                .iter()
                .enumerate()
                .any(|(index, value)| finding.subject_ids[..index].contains(value))
            || finding
                .subject_ids
                .iter()
                .any(|value| !known_subjects.contains(value.as_str()))
        {
            return Err(Error::Invariant(
                "analyzer findings require code, message, and unique known subject ids".into(),
            ));
        }
    }
    Ok(())
}

fn runtime_error(error: impl std::fmt::Display) -> Error {
    Error::Invariant(format!("Wasm analyzer failure: {error}"))
}
