//! questline core: the rules of a run, with no operating system in the way.
//!
//! Layout:
//! - [`domain`] — games, run specifications and their invariants;
//! - [`state`] — the run state machine and its legal transitions;
//! - [`sanitize`] — turning untrusted catalogue names into safe relative paths;
//! - [`schedule`] — batch planning without duplicate executables;
//! - [`ports`] — the only traits through which the core touches the outside world;
//! - [`error`] — typed failures.
//!
//! Nothing here does I/O, spawns processes or talks to Discord, so the whole
//! crate is testable on any machine without a network or a Discord client.

pub mod domain;
pub mod error;
pub mod ports;
pub mod sanitize;
pub mod schedule;
pub mod state;

pub use domain::{Game, GameId, RunMode, RunSpec, SpoofMethod};
pub use error::{DomainError, PathError, ProcessError, TransitionError};
pub use sanitize::{sanitize_executable_rel_path, Platform};
pub use schedule::{batch_count, BatchPlanner};
pub use state::{RunMachine, RunState};
