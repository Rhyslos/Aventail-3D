//! Each box's last results, kept so only what changed is worked out
//! again.
//!
//! A result is kept under a **key**: a hash of everything it was made
//! from (the box's kind, its settings, the size, its seed, and its inputs'
//! own keys). Changing one box's setting changes its key and the keys of
//! the boxes after it, while everything before it is found again. Results
//! not used by the last run are forgotten, oldest first, once the cache
//! is over its memory budget; those used by the last run (what is on
//! screen) are always kept.

use crate::eval::BoxResult;
use std::collections::HashMap;
use std::sync::Arc;

/// One kept result.
#[derive(Debug)]
struct Entry {
    result: Arc<BoxResult>,
    bytes: usize,
    /// The run that last used it.
    used: u64,
}

/// Results kept between runs.
#[derive(Debug)]
pub struct Cache {
    entries: HashMap<u64, Entry>,
    budget: usize,
    run: u64,
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

impl Cache {
    /// The standard memory budget: 512 MB.
    pub const STANDARD_BUDGET: usize = 512 << 20;

    /// An empty cache with the standard budget.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            budget: Self::STANDARD_BUDGET,
            run: 0,
        }
    }

    /// Keeps results not used by the last run only while all kept results
    /// take at most `bytes`.
    pub fn set_budget(&mut self, bytes: usize) {
        self.budget = bytes;
        self.trim();
    }

    /// How much memory the kept results take, in bytes.
    pub fn memory(&self) -> usize {
        self.entries.values().map(|e| e.bytes).sum()
    }

    /// How many results are kept.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing is kept.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Forgets everything.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Starts a run.
    pub(crate) fn begin(&mut self) {
        self.run += 1;
    }

    /// The result kept under `key`, marked as used by this run.
    pub(crate) fn get(&mut self, key: u64) -> Option<Arc<BoxResult>> {
        let entry = self.entries.get_mut(&key)?;
        entry.used = self.run;
        Some(entry.result.clone())
    }

    /// Keeps `result` under `key`.
    pub(crate) fn put(&mut self, key: u64, result: Arc<BoxResult>) {
        let bytes = result.memory_size();
        self.entries.insert(
            key,
            Entry {
                result,
                bytes,
                used: self.run,
            },
        );
    }

    /// Forgets results not used by this run, oldest first, until within
    /// the budget.
    pub(crate) fn trim(&mut self) {
        let mut memory = self.memory();
        if memory <= self.budget {
            return;
        }
        let mut old: Vec<(u64, u64, usize)> = self
            .entries
            .iter()
            .filter(|(_, e)| e.used < self.run)
            .map(|(k, e)| (e.used, *k, e.bytes))
            .collect();
        old.sort_unstable();
        for (_, key, bytes) in old {
            if memory <= self.budget {
                break;
            }
            self.entries.remove(&key);
            memory -= bytes;
        }
    }
}
