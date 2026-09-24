//! Deterministic, append-only continuity records between chat sessions.
//!
//! A "day" is only a human-facing name for one session. Handoffs carry bounded
//! facts and work state; they never create or transfer kernel authority.

mod ledger;
mod protocol;
mod state;

pub use ledger::{Ledger, commit, initialize, load};
pub use protocol::{
    CommitOutcome, CommitRequest, CommitStatus, Day, Event, HandoffCapsule, HandoffRef,
    StoredEvent, WorkspaceCheckpoint, handoff_sha256,
};
pub use state::{DayState, ProjectedHandoff, State};

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
                write!(formatter, "corrupt handoff log at line {line}: {reason}")
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
