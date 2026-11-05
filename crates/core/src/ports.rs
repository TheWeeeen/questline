//! Ports: the few side effects the core is allowed to cause.
//!
//! The core never spawns a process or reads the filesystem itself. It asks for a
//! `ProcessSpawner` and gets a `ProcessHandle` back, so tests can drive the whole
//! run lifecycle with a fake and the real implementation lives in the app crate.

use std::path::PathBuf;

use crate::error::ProcessError;

/// One process to launch: an executable we wrote plus its arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnRequest {
    /// Absolute path of the executable inside our own temporary root.
    pub exe: PathBuf,
    pub args: Vec<String>,
    /// Human label for logs (the game name), not passed to the process.
    pub label: String,
}

/// A running process we may later inspect and stop.
pub trait ProcessHandle: Send {
    fn pid(&self) -> u32;

    /// True while the process is alive. Implementations must not reap the child
    /// in a way that makes `kill` fail on a live process.
    fn is_running(&mut self) -> Result<bool, ProcessError>;

    /// Best-effort stop; calling it on an already dead process is not an error.
    fn kill(&mut self) -> Result<(), ProcessError>;
}

/// Creates processes. Real implementation: the app crate; fake: tests.
pub trait ProcessSpawner: Send + Sync {
    fn spawn(&self, request: &SpawnRequest) -> Result<Box<dyn ProcessHandle>, ProcessError>;
}
