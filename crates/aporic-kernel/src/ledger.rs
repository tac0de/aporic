use crate::protocol::{CommitOutcome, CommitRequest, PROTOCOL_EPOCH, StoredEvent};
use crate::state::{State, validate_request_text};
use crate::{Error, Result};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

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
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.sync_all()?;
    Ok(())
}

pub fn load(path: impl AsRef<Path>) -> Result<Ledger> {
    let mut file = OpenOptions::new().read(true).open(path)?;
    file.lock_shared()?;
    let result = read_locked(&mut file);
    let unlock_result = file.unlock();
    match (result, unlock_result) {
        (Ok(ledger), Ok(())) => Ok(ledger),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(Error::Io(error)),
    }
}

pub fn commit(path: impl AsRef<Path>, request: CommitRequest) -> Result<CommitOutcome> {
    let mut file = OpenOptions::new()
        .read(true)
        .append(true)
        .open(path.as_ref())?;
    file.lock()?;
    let result = commit_locked(&mut file, request);
    let unlock_result = file.unlock();
    match (result, unlock_result) {
        (Ok(outcome), Ok(())) => Ok(outcome),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(Error::Io(error)),
    }
}

fn read_locked(file: &mut File) -> Result<Ledger> {
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    replay_bytes(&bytes)
}

fn replay_bytes(bytes: &[u8]) -> Result<Ledger> {
    if bytes.is_empty() {
        return Ok(Ledger::default());
    }
    if !bytes.ends_with(b"\n") {
        return Err(Error::CorruptLog {
            line: bytes.iter().filter(|byte| **byte == b'\n').count() + 1,
            reason: "record is not newline-terminated".into(),
        });
    }

    let mut ledger = Ledger::default();
    for (index, line) in bytes[..bytes.len() - 1]
        .split(|byte| *byte == b'\n')
        .enumerate()
    {
        if line.is_empty() {
            return Err(Error::CorruptLog {
                line: index + 1,
                reason: "blank records are not allowed".into(),
            });
        }
        let stored: StoredEvent =
            serde_json::from_slice(line).map_err(|error| Error::CorruptLog {
                line: index + 1,
                reason: error.to_string(),
            })?;
        if stored.request.protocol_epoch != PROTOCOL_EPOCH {
            return Err(Error::CorruptLog {
                line: index + 1,
                reason: "unsupported protocol epoch".into(),
            });
        }
        if stored.request.expected_revision != ledger.state.revision {
            return Err(Error::CorruptLog {
                line: index + 1,
                reason: "stored request has a stale expected revision".into(),
            });
        }
        validate_request_text(&stored.request.event_id).map_err(|reason| Error::CorruptLog {
            line: index + 1,
            reason: reason.into(),
        })?;
        validate_request_text(&stored.request.idempotency_key).map_err(|reason| {
            Error::CorruptLog {
                line: index + 1,
                reason: reason.into(),
            }
        })?;
        if ledger
            .records
            .iter()
            .any(|record| record.request.event_id == stored.request.event_id)
        {
            return Err(Error::CorruptLog {
                line: index + 1,
                reason: "duplicate event id".into(),
            });
        }
        if ledger
            .records
            .iter()
            .any(|record| record.request.idempotency_key == stored.request.idempotency_key)
        {
            return Err(Error::CorruptLog {
                line: index + 1,
                reason: "duplicate idempotency key".into(),
            });
        }
        ledger
            .state
            .apply(stored.sequence, &stored.request.event)
            .map_err(|reason| Error::CorruptLog {
                line: index + 1,
                reason: reason.into(),
            })?;
        ledger.records.push(stored);
    }
    Ok(ledger)
}

fn commit_locked(file: &mut File, request: CommitRequest) -> Result<CommitOutcome> {
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let mut ledger = replay_bytes(&bytes)?;

    if request.protocol_epoch != PROTOCOL_EPOCH {
        return Ok(CommitOutcome::rejected(
            ledger.state.revision,
            "UNSUPPORTED_PROTOCOL_EPOCH",
        ));
    }
    if let Err(reason) = validate_request_text(&request.event_id) {
        return Ok(CommitOutcome::rejected(ledger.state.revision, reason));
    }
    if let Err(reason) = validate_request_text(&request.idempotency_key) {
        return Ok(CommitOutcome::rejected(ledger.state.revision, reason));
    }
    if let Some(existing) = ledger
        .records
        .iter()
        .find(|record| record.request.idempotency_key == request.idempotency_key)
    {
        return if existing.request == request {
            Ok(CommitOutcome::duplicate(existing.sequence))
        } else {
            Ok(CommitOutcome::rejected(
                ledger.state.revision,
                "IDEMPOTENCY_KEY_CONFLICT",
            ))
        };
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
    if let Err(reason) = ledger.state.apply(sequence, &request.event) {
        return Ok(CommitOutcome::rejected(ledger.state.revision, reason));
    }
    let stored = StoredEvent { sequence, request };
    let encoded = serde_json::to_vec(&stored)?;
    file.write_all(&encoded)?;
    file.write_all(b"\n")?;
    file.sync_data()?;
    Ok(CommitOutcome::committed(sequence))
}
