//! The run state machine.
//!
//! The upstream project tracked run status with an `AtomicBool` plus a `RwLock`
//! and let the UI decide what a status meant. Here the legal states and legal
//! transitions are the only way to describe a run, so an illegal sequence is a
//! compile-time visible error instead of a stuck spinner.

use std::fmt;

use crate::error::TransitionError;

/// Where a run currently is. Every variant carries the batch it belongs to,
/// so an event from a previous batch cannot be mistaken for the current one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunState {
    #[default]
    Idle,
    Preparing,
    Running {
        batch: u32,
    },
    Paused {
        batch: u32,
    },
    Stopping {
        batch: u32,
    },
    Finished {
        batches: u32,
    },
    Failed {
        batch: u32,
    },
}

impl RunState {
    /// True while the run owns processes or is about to.
    #[must_use]
    pub fn is_active(self) -> bool {
        matches!(
            self,
            Self::Preparing | Self::Running { .. } | Self::Paused { .. } | Self::Stopping { .. }
        )
    }

    #[must_use]
    pub fn batch(self) -> Option<u32> {
        match self {
            Self::Running { batch }
            | Self::Paused { batch }
            | Self::Stopping { batch }
            | Self::Failed { batch } => Some(batch),
            Self::Finished { batches } => Some(batches),
            Self::Idle | Self::Preparing => None,
        }
    }
}

impl fmt::Display for RunState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => f.write_str("idle"),
            Self::Preparing => f.write_str("preparing"),
            Self::Running { batch } => write!(f, "running batch {batch}"),
            Self::Paused { batch } => write!(f, "paused in batch {batch}"),
            Self::Stopping { batch } => write!(f, "stopping batch {batch}"),
            Self::Finished { batches } => write!(f, "finished after {batches} batches"),
            Self::Failed { batch } => write!(f, "failed in batch {batch}"),
        }
    }
}

/// Owns the state and refuses illegal transitions.
#[derive(Debug, Clone, Default)]
pub struct RunMachine {
    state: RunState,
    /// Highest batch number seen; `0` means no batch has started yet.
    last_batch: u32,
}

impl RunMachine {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn state(&self) -> RunState {
        self.state
    }

    /// Idle, finished and failed runs may be started again; an active one may not.
    pub fn start(&mut self) -> Result<RunState, TransitionError> {
        match self.state {
            RunState::Idle | RunState::Finished { .. } | RunState::Failed { .. } => {
                self.state = RunState::Preparing;
                self.last_batch = 0;
                Ok(self.state)
            }
            from => Err(TransitionError::new(from, "start a run")),
        }
    }

    /// First batch of a run, or the next one after the previous batch ended.
    pub fn batch_started(&mut self, batch: u32) -> Result<RunState, TransitionError> {
        match self.state {
            RunState::Preparing | RunState::Running { .. } => {
                self.last_batch = batch.max(self.last_batch);
                self.state = RunState::Running { batch };
                Ok(self.state)
            }
            from => Err(TransitionError::new(from, "start a batch")),
        }
    }

    pub fn pause(&mut self) -> Result<RunState, TransitionError> {
        match self.state {
            RunState::Running { batch } => {
                self.state = RunState::Paused { batch };
                Ok(self.state)
            }
            from => Err(TransitionError::new(from, "pause")),
        }
    }

    pub fn resume(&mut self) -> Result<RunState, TransitionError> {
        match self.state {
            RunState::Paused { batch } => {
                self.state = RunState::Running { batch };
                Ok(self.state)
            }
            from => Err(TransitionError::new(from, "resume")),
        }
    }

    /// Stopping is allowed from every active state; the batch number is kept so
    /// the caller can still clean up the right processes.
    pub fn request_stop(&mut self) -> Result<RunState, TransitionError> {
        let batch = match self.state {
            RunState::Preparing => 0,
            RunState::Running { batch } | RunState::Paused { batch } => batch,
            from => return Err(TransitionError::new(from, "stop")),
        };
        self.state = RunState::Stopping { batch };
        Ok(self.state)
    }

    /// A stop request completes here, and so does a run that simply ran out of work.
    pub fn complete(&mut self) -> Result<RunState, TransitionError> {
        let batches = match self.state {
            RunState::Running { batch } | RunState::Stopping { batch } => batch,
            RunState::Preparing => 0,
            from => return Err(TransitionError::new(from, "complete")),
        };
        self.state = RunState::Finished { batches };
        Ok(self.state)
    }

    /// Any active state can fail; the batch stays known so cleanup can target it.
    pub fn fail(&mut self) -> Result<RunState, TransitionError> {
        let batch = match self.state {
            RunState::Failed { .. } => {
                return Err(TransitionError::new(self.state, "fail"));
            }
            RunState::Preparing => 0,
            RunState::Running { batch }
            | RunState::Paused { batch }
            | RunState::Stopping { batch } => batch,
            RunState::Idle | RunState::Finished { .. } => self.last_batch,
        };
        self.state = RunState::Failed { batch };
        Ok(self.state)
    }

    pub fn reset(&mut self) -> Result<RunState, TransitionError> {
        match self.state {
            RunState::Finished { .. } | RunState::Failed { .. } => {
                self.state = RunState::Idle;
                self.last_batch = 0;
                Ok(self.state)
            }
            from => Err(TransitionError::new(from, "reset")),
        }
    }
}
