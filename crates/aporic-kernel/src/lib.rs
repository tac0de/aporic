//! Minimal deterministic authorization kernel for one-shot semantic actions.
//!
//! The kernel compares bounded structured values, reserves exact authority,
//! records effects, and replays append-only state. It does not interpret natural
//! language, authenticate caller identities, select models, or load roles. A
//! host adapter must expose authority, agent, effect, and verification routes
//! only to their corresponding trusted callers.

mod ledger;
mod protocol;
mod state;

pub use ledger::{Ledger, commit, initialize, load};
pub use protocol::{
    CommitOutcome, CommitRequest, CommitStatus, EffectOutcome, Event, Grant, PROTOCOL_EPOCH,
    Reservation, StoredEvent, VerificationResult,
};
pub use state::{EffectRecord, GrantState, ReservationState, State, VerificationRecord};

use std::fmt;

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
                write!(formatter, "corrupt event log at line {line}: {reason}")
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
