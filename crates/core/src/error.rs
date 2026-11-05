//! Typed errors for the domain layer.
//!
//! Everything here is data an upper layer can branch on: a malformed catalogue
//! entry, an illegal state transition, a rejected executable path. Errors never
//! carry a `String` reason only — the variant names the failure.

use std::time::Duration;
use thiserror::Error;

use crate::state::RunState;

/// A catalogue entry or a run configuration that cannot be trusted.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("game id is empty")]
    EmptyGameId,

    #[error("game id must be numeric, got {0:?}")]
    NonNumericGameId(String),

    #[error("game name is empty")]
    EmptyGameName,

    #[error("batch size must be in {min}..={max}, got {got}")]
    BatchSizeOutOfRange { got: usize, min: usize, max: usize },

    #[error("hold duration must be in {min:?}..={max:?}, got {got:?}")]
    HoldOutOfRange {
        got: Duration,
        min: Duration,
        max: Duration,
    },
}

/// A state machine transition that the current state does not allow.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("cannot {action} while {from}")]
pub struct TransitionError {
    pub from: RunState,
    pub action: &'static str,
}

impl TransitionError {
    #[must_use]
    pub fn new(from: RunState, action: &'static str) -> Self {
        Self { from, action }
    }
}

/// Why an executable name from the catalogue was rejected.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PathError {
    #[error("executable name is empty")]
    Empty,

    #[error("executable name must be relative, got {0:?}")]
    Absolute(String),

    #[error("path traversal in executable name: {0:?}")]
    Traversal(String),

    #[error("{0:?} is a reserved device name on Windows")]
    ReservedDeviceName(String),

    #[error("executable name has no usable path component: {0:?}")]
    NoUsableComponent(String),
}

/// A failure from an operating-system port (process spawning in M1).
#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("failed to spawn {exe:?}: {source}")]
    Spawn {
        exe: String,
        #[source]
        source: std::io::Error,
    },

    #[error("process {pid} is already gone")]
    Gone { pid: u32 },

    #[error("failed to inspect process {pid}: {source}")]
    Inspect {
        pid: u32,
        #[source]
        source: std::io::Error,
    },
}
