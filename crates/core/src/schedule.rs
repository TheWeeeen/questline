//! Batch planning: which games go next, and never two of them at once that
//! share an executable file name.
//!
//! Two games whose executables share a base name cannot run side by side — the
//! second spawn would land on a hardlink that already exists — so the planner
//! drops duplicates instead of letting the launcher fail on them.

use std::collections::HashSet;

use crate::domain::{Game, RunSpec};
use crate::error::DomainError;

/// Builds consecutive batches out of a game pool.
#[derive(Debug, Clone)]
pub struct BatchPlanner {
    batch_size: usize,
}

impl BatchPlanner {
    pub fn new(batch_size: usize) -> Result<Self, DomainError> {
        if !(RunSpec::MIN_BATCH..=RunSpec::MAX_BATCH).contains(&batch_size) {
            return Err(DomainError::BatchSizeOutOfRange {
                got: batch_size,
                min: RunSpec::MIN_BATCH,
                max: RunSpec::MAX_BATCH,
            });
        }
        Ok(Self { batch_size })
    }

    #[must_use]
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Takes the next batch out of `pool`, removing the chosen games from it.
    ///
    /// `remaining` is how many games the run still wants (`None` = no limit);
    /// a batch never exceeds it, so the last batch of a limited run is exact.
    /// Games skipped for a duplicate executable stay in the pool for later.
    pub fn plan(&self, pool: &mut Vec<Game>, remaining: Option<usize>) -> Vec<Game> {
        let cap = remaining.map_or(self.batch_size, |left| left.min(self.batch_size));
        if cap == 0 {
            return Vec::new();
        }

        let mut taken: Vec<usize> = Vec::with_capacity(cap);
        let mut seen_exes: HashSet<String> = HashSet::new();

        for (index, game) in pool.iter().enumerate() {
            if taken.len() >= cap {
                break;
            }
            if let Some(exe) = game.exe_basename() {
                if !seen_exes.insert(exe) {
                    continue;
                }
            }
            taken.push(index);
        }

        let mut batch: Vec<Game> = Vec::with_capacity(taken.len());
        for index in &taken {
            batch.push(pool[*index].clone());
        }
        // Remove from the back so the earlier indices stay valid.
        for index in taken.into_iter().rev() {
            pool.remove(index);
        }

        batch
    }
}

/// How many batches a pool of `pool_len` games needs at this batch size.
#[must_use]
pub fn batch_count(pool_len: usize, batch_size: usize) -> usize {
    if batch_size == 0 {
        return 0;
    }
    pool_len.div_ceil(batch_size)
}
