//! Domain types: what a game is, what a run is, and which invariants hold.

use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::DomainError;

/// Discord's numeric application id, kept as a string so no precision is lost.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct GameId(String);

impl GameId {
    /// Rejects anything that is not a plain digits-only id: the catalogue is
    /// remote data, so this is a trust boundary, not a formality.
    pub fn new(raw: impl Into<String>) -> Result<Self, DomainError> {
        let raw = raw.into();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(DomainError::EmptyGameId);
        }
        if !trimmed.chars().all(|c| c.is_ascii_digit()) {
            return Err(DomainError::NonNumericGameId(trimmed.to_owned()));
        }
        Ok(Self(trimmed.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for GameId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for GameId {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<GameId> for String {
    fn from(value: GameId) -> Self {
        value.0
    }
}

/// How the activity for a game is produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpoofMethod {
    /// A real executable with the game's file name is launched and kept alive.
    Process,
    /// The activity is reported over the local Discord IPC socket.
    Rpc,
}

/// One spoofable game from Discord's detectable-applications catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Game {
    pub id: GameId,
    pub name: String,
    /// Executable path relative to the catalogue, already sanitized.
    pub executable: Option<String>,
    pub arguments: Option<String>,
    pub method: SpoofMethod,
}

impl Game {
    pub fn new(
        id: GameId,
        name: impl Into<String>,
        executable: Option<String>,
        method: SpoofMethod,
    ) -> Result<Self, DomainError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(DomainError::EmptyGameName);
        }
        Ok(Self {
            id,
            name,
            executable,
            arguments: None,
            method,
        })
    }

    /// File name only, lowercased — two games sharing it cannot run side by side.
    #[must_use]
    pub fn exe_basename(&self) -> Option<String> {
        let path = self.executable.as_deref()?;
        let base = path.rsplit(['/', '\\']).next()?;
        let base = base.trim();
        if base.is_empty() {
            None
        } else {
            Some(base.to_lowercase())
        }
    }
}

/// How a run picks games out of the catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunMode {
    /// Rotate through the pool in batches until the target count is reached.
    Batch,
    /// Keep one fixed set running for the whole duration (hour farm, quests).
    Fixed,
}

/// Everything needed to start a run, validated on construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSpec {
    pub mode: RunMode,
    pub batch_size: usize,
    pub hold: Duration,
    pub target: Option<usize>,
}

impl RunSpec {
    pub const MIN_BATCH: usize = 1;
    pub const MAX_BATCH: usize = 20;
    pub const MIN_HOLD: Duration = Duration::from_secs(3);
    pub const MAX_HOLD: Duration = Duration::from_secs(8 * 60 * 60);

    pub fn new(mode: RunMode, batch_size: usize, hold: Duration) -> Result<Self, DomainError> {
        if !(Self::MIN_BATCH..=Self::MAX_BATCH).contains(&batch_size) {
            return Err(DomainError::BatchSizeOutOfRange {
                got: batch_size,
                min: Self::MIN_BATCH,
                max: Self::MAX_BATCH,
            });
        }
        if !(Self::MIN_HOLD..=Self::MAX_HOLD).contains(&hold) {
            return Err(DomainError::HoldOutOfRange {
                got: hold,
                min: Self::MIN_HOLD,
                max: Self::MAX_HOLD,
            });
        }
        Ok(Self {
            mode,
            batch_size,
            hold,
            target: None,
        })
    }

    #[must_use]
    pub fn with_target(mut self, target: usize) -> Self {
        self.target = Some(target.max(1));
        self
    }
}
