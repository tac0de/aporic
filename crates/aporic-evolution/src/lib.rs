//! Append-only requests for improving Aporic after project work completes.
//!
//! This ledger is workflow evidence, not kernel authority. A request cannot
//! modify the running kernel, and a reconnect requires a recorded implementation.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

pub const PROTOCOL_EPOCH: u32 = 1;
const MAX_LOG_BYTES: usize = 4 * 1024 * 1024;
const MAX_ID_BYTES: usize = 256;
const MAX_TEXT_BYTES: usize = 4096;
const MAX_EVIDENCE: usize = 16;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(serde_json::Error),
    CorruptLog { line: usize, reason: String },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Json(error) => write!(formatter, "JSON error: {error}"),
            Self::CorruptLog { line, reason } => {
                write!(formatter, "corrupt evolution log at line {line}: {reason}")
            }
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
pub enum Component {
    Kernel,
    Role,
    Adapter,
    Integration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Change {
    Add,
    Modify,
    Merge,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Improvement {
    pub request_id: String,
    pub source_project_id: String,
    pub source_binding_sha256: String,
    pub component: Component,
    pub change: Change,
    pub summary: String,
    pub rationale: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    Requested {
        improvement: Improvement,
    },
    Implemented {
        request_id: String,
        aporic_commit: String,
        checks: Vec<String>,
    },
    Reconnected {
        request_id: String,
        aporic_commit: String,
        target_binding_sha256: String,
        profile_sha256: String,
        adapter_sha256: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitRequest {
    pub protocol_epoch: u32,
    pub event_id: String,
    pub idempotency_key: String,
    pub expected_revision: u64,
    pub event: Event,
}

impl CommitRequest {
    pub fn new(
        event_id: impl Into<String>,
        idempotency_key: impl Into<String>,
        expected_revision: u64,
        event: Event,
    ) -> Self {
        Self {
            protocol_epoch: PROTOCOL_EPOCH,
            event_id: event_id.into(),
            idempotency_key: idempotency_key.into(),
            expected_revision,
            event,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitStatus {
    Committed,
    Duplicate,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitOutcome {
    pub status: CommitStatus,
    pub revision: u64,
    pub reason_code: String,
}

impl CommitOutcome {
    fn committed(revision: u64) -> Self {
        Self {
            status: CommitStatus::Committed,
            revision,
            reason_code: "COMMITTED".into(),
        }
    }

    fn duplicate(revision: u64) -> Self {
        Self {
            status: CommitStatus::Duplicate,
            revision,
            reason_code: "DUPLICATE".into(),
        }
    }

    fn rejected(revision: u64, reason: impl Into<String>) -> Self {
        Self {
            status: CommitStatus::Rejected,
            revision,
            reason_code: reason.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredEvent {
    pub sequence: u64,
    pub request: CommitRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImprovementState {
    pub improvement: Improvement,
    pub aporic_commit: Option<String>,
    pub checks: Vec<String>,
    pub reconnected: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub revision: u64,
    pub improvements: BTreeMap<String, ImprovementState>,
}

#[derive(Debug, Default)]
pub struct Ledger {
    state: State,
    records: Vec<StoredEvent>,
}

impl Ledger {
    pub fn state(&self) -> &State {
        &self.state
    }
    pub fn records(&self) -> &[StoredEvent] {
        &self.records
    }
}

pub fn initialize(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent().filter(|value| !value.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .sync_all()?;
    Ok(())
}

pub fn load(path: impl AsRef<Path>) -> Result<Ledger> {
    let mut file = OpenOptions::new().read(true).open(path)?;
    file.lock_shared()?;
    let result = read_locked(&mut file);
    let unlocked = file.unlock();
    match (result, unlocked) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(Error::Io(error)),
    }
}

pub fn commit(path: impl AsRef<Path>, request: CommitRequest) -> Result<CommitOutcome> {
    let mut file = OpenOptions::new().read(true).append(true).open(path)?;
    file.lock()?;
    let result = commit_locked(&mut file, request);
    let unlocked = file.unlock();
    match (result, unlocked) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(Error::Io(error)),
    }
}

fn read_locked(file: &mut File) -> Result<Ledger> {
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.take((MAX_LOG_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_LOG_BYTES {
        return Err(Error::CorruptLog {
            line: 0,
            reason: "log exceeds byte limit".into(),
        });
    }
    replay(&bytes)
}

fn replay(bytes: &[u8]) -> Result<Ledger> {
    if bytes.is_empty() {
        return Ok(Ledger::default());
    }
    if !bytes.ends_with(b"\n") {
        return Err(Error::CorruptLog {
            line: bytes.iter().filter(|b| **b == b'\n').count() + 1,
            reason: "record is not newline-terminated".into(),
        });
    }
    let mut ledger = Ledger::default();
    let mut event_ids = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for (index, line) in bytes[..bytes.len() - 1].split(|b| *b == b'\n').enumerate() {
        let line_number = index + 1;
        if line.is_empty() {
            return corrupt(line_number, "blank records are not allowed");
        }
        let stored: StoredEvent =
            serde_json::from_slice(line).map_err(|error| Error::CorruptLog {
                line: line_number,
                reason: error.to_string(),
            })?;
        if stored.request.protocol_epoch != PROTOCOL_EPOCH {
            return corrupt(line_number, "unsupported protocol epoch");
        }
        if stored.request.expected_revision != ledger.state.revision
            || stored.sequence != ledger.state.revision + 1
        {
            return corrupt(line_number, "noncontiguous revision");
        }
        validate_id(&stored.request.event_id).map_err(|reason| Error::CorruptLog {
            line: line_number,
            reason: reason.into(),
        })?;
        validate_id(&stored.request.idempotency_key).map_err(|reason| Error::CorruptLog {
            line: line_number,
            reason: reason.into(),
        })?;
        if !event_ids.insert(stored.request.event_id.clone()) {
            return corrupt(line_number, "duplicate event id");
        }
        if !keys.insert(stored.request.idempotency_key.clone()) {
            return corrupt(line_number, "duplicate idempotency key");
        }
        apply(&mut ledger.state, stored.sequence, &stored.request.event).map_err(|reason| {
            Error::CorruptLog {
                line: line_number,
                reason: reason.into(),
            }
        })?;
        ledger.records.push(stored);
    }
    Ok(ledger)
}

fn commit_locked(file: &mut File, request: CommitRequest) -> Result<CommitOutcome> {
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.take((MAX_LOG_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_LOG_BYTES {
        return Err(Error::CorruptLog {
            line: 0,
            reason: "log exceeds byte limit".into(),
        });
    }
    let mut ledger = replay(&bytes)?;
    if request.protocol_epoch != PROTOCOL_EPOCH {
        return Ok(CommitOutcome::rejected(
            ledger.state.revision,
            "UNSUPPORTED_PROTOCOL_EPOCH",
        ));
    }
    if let Err(reason) =
        validate_id(&request.event_id).and_then(|_| validate_id(&request.idempotency_key))
    {
        return Ok(CommitOutcome::rejected(ledger.state.revision, reason));
    }
    if let Some(existing) = ledger
        .records
        .iter()
        .find(|record| record.request.idempotency_key == request.idempotency_key)
    {
        return Ok(if existing.request == request {
            CommitOutcome::duplicate(existing.sequence)
        } else {
            CommitOutcome::rejected(ledger.state.revision, "IDEMPOTENCY_KEY_CONFLICT")
        });
    }
    if ledger
        .records
        .iter()
        .any(|record| record.request.event_id == request.event_id)
    {
        return Ok(CommitOutcome::rejected(
            ledger.state.revision,
            "EVENT_ID_CONFLICT",
        ));
    }
    if request.expected_revision != ledger.state.revision {
        return Ok(CommitOutcome::rejected(
            ledger.state.revision,
            "STALE_REVISION",
        ));
    }
    let sequence = ledger.state.revision + 1;
    if let Err(reason) = apply(&mut ledger.state, sequence, &request.event) {
        return Ok(CommitOutcome::rejected(ledger.state.revision, reason));
    }
    let encoded = serde_json::to_vec(&StoredEvent { sequence, request })?;
    if bytes.len() + encoded.len() + 1 > MAX_LOG_BYTES {
        return Ok(CommitOutcome::rejected(
            ledger.state.revision,
            "LOG_LIMIT_EXCEEDED",
        ));
    }
    file.write_all(&encoded)?;
    file.write_all(b"\n")?;
    file.sync_data()?;
    Ok(CommitOutcome::committed(sequence))
}

fn apply(state: &mut State, sequence: u64, event: &Event) -> std::result::Result<(), &'static str> {
    if sequence != state.revision + 1 {
        return Err("NONCONTIGUOUS_SEQUENCE");
    }
    match event {
        Event::Requested { improvement } => {
            validate_improvement(improvement)?;
            if state.improvements.contains_key(&improvement.request_id) {
                return Err("REQUEST_ID_CONFLICT");
            }
            state.improvements.insert(
                improvement.request_id.clone(),
                ImprovementState {
                    improvement: improvement.clone(),
                    aporic_commit: None,
                    checks: Vec::new(),
                    reconnected: false,
                },
            );
        }
        Event::Implemented {
            request_id,
            aporic_commit,
            checks,
        } => {
            validate_id(request_id)?;
            validate_git_oid(aporic_commit)?;
            validate_items(checks)?;
            let item = state
                .improvements
                .get_mut(request_id)
                .ok_or("REQUEST_NOT_FOUND")?;
            if item.aporic_commit.is_some() {
                return Err("REQUEST_ALREADY_IMPLEMENTED");
            }
            item.aporic_commit = Some(aporic_commit.clone());
            item.checks = checks.clone();
        }
        Event::Reconnected {
            request_id,
            aporic_commit,
            target_binding_sha256,
            profile_sha256,
            adapter_sha256,
        } => {
            validate_id(request_id)?;
            validate_git_oid(aporic_commit)?;
            validate_digest(target_binding_sha256)?;
            validate_digest(profile_sha256)?;
            validate_digest(adapter_sha256)?;
            let item = state
                .improvements
                .get_mut(request_id)
                .ok_or("REQUEST_NOT_FOUND")?;
            if item.aporic_commit.is_none() {
                return Err("IMPLEMENTATION_REQUIRED");
            }
            if item.aporic_commit.as_deref() != Some(aporic_commit) {
                return Err("IMPLEMENTATION_COMMIT_MISMATCH");
            }
            if item.reconnected {
                return Err("REQUEST_ALREADY_RECONNECTED");
            }
            if item.improvement.source_binding_sha256 != *target_binding_sha256 {
                return Err("TARGET_BINDING_MISMATCH");
            }
            item.reconnected = true;
        }
    }
    state.revision = sequence;
    Ok(())
}

fn validate_improvement(value: &Improvement) -> std::result::Result<(), &'static str> {
    validate_id(&value.request_id)?;
    validate_id(&value.source_project_id)?;
    validate_digest(&value.source_binding_sha256)?;
    validate_text(&value.summary)?;
    validate_text(&value.rationale)?;
    validate_items(&value.evidence_refs)
}

fn validate_items(values: &[String]) -> std::result::Result<(), &'static str> {
    if values.len() > MAX_EVIDENCE {
        return Err("TOO_MANY_ITEMS");
    }
    values.iter().try_for_each(|value| validate_text(value))
}

fn validate_id(value: &str) -> std::result::Result<(), &'static str> {
    if value.trim().is_empty()
        || value.len() > MAX_ID_BYTES
        || value.bytes().any(|b| b.is_ascii_control())
    {
        Err("INVALID_ID")
    } else {
        Ok(())
    }
}

fn validate_text(value: &str) -> std::result::Result<(), &'static str> {
    if value.trim().is_empty() || value.len() > MAX_TEXT_BYTES || value.bytes().any(|b| b == 0) {
        Err("INVALID_TEXT")
    } else {
        Ok(())
    }
}

fn validate_digest(value: &str) -> std::result::Result<(), &'static str> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        Err("INVALID_DIGEST")
    } else {
        Ok(())
    }
}

fn validate_git_oid(value: &str) -> std::result::Result<(), &'static str> {
    if matches!(value.len(), 40 | 64) && value.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err("INVALID_GIT_OID")
    }
}

fn corrupt<T>(line: usize, reason: &str) -> Result<T> {
    Err(Error::CorruptLog {
        line,
        reason: reason.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    fn path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "aporic-evolution-{}-{}.jsonl",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn improvement() -> Improvement {
        Improvement {
            request_id: "request-1".into(),
            source_project_id: "demo".into(),
            source_binding_sha256: "a".repeat(64),
            component: Component::Kernel,
            change: Change::Modify,
            summary: "Preserve a missing invariant".into(),
            rationale: "Observed during project work".into(),
            evidence_refs: vec!["test:failure".into()],
        }
    }

    #[test]
    fn request_must_be_implemented_before_reconnection() {
        let path = path();
        initialize(&path).unwrap();
        let requested = commit(
            &path,
            CommitRequest::new(
                "event-1",
                "key-1",
                0,
                Event::Requested {
                    improvement: improvement(),
                },
            ),
        )
        .unwrap();
        assert_eq!(requested.status, CommitStatus::Committed);
        let early = commit(
            &path,
            CommitRequest::new(
                "event-2",
                "key-2",
                1,
                Event::Reconnected {
                    request_id: "request-1".into(),
                    aporic_commit: "d".repeat(40),
                    target_binding_sha256: "a".repeat(64),
                    profile_sha256: "b".repeat(64),
                    adapter_sha256: "c".repeat(64),
                },
            ),
        )
        .unwrap();
        assert_eq!(early.reason_code, "IMPLEMENTATION_REQUIRED");
        commit(
            &path,
            CommitRequest::new(
                "event-3",
                "key-3",
                1,
                Event::Implemented {
                    request_id: "request-1".into(),
                    aporic_commit: "d".repeat(40),
                    checks: vec!["cargo test".into()],
                },
            ),
        )
        .unwrap();
        commit(
            &path,
            CommitRequest::new(
                "event-4",
                "key-4",
                2,
                Event::Reconnected {
                    request_id: "request-1".into(),
                    aporic_commit: "d".repeat(40),
                    target_binding_sha256: "a".repeat(64),
                    profile_sha256: "b".repeat(64),
                    adapter_sha256: "c".repeat(64),
                },
            ),
        )
        .unwrap();
        let ledger = load(&path).unwrap();
        assert!(ledger.state().improvements["request-1"].reconnected);
        std::fs::remove_file(path).unwrap();
    }
}
