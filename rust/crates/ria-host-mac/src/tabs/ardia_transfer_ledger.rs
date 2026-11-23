// Ported from ardia packages/ardiacore/ios/ArdiaTransferLedger.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::collections::HashMap;
use std::num::NonZeroU64;

pub type Generation = NonZeroU64;

#[derive(Default)]
struct Transfer {
    prepared: Option<Generation>,
    reload: Option<Generation>,
    parked: Option<Generation>,
}

#[derive(Default)]
pub struct TransferLedger {
    last: u64,
    entries: HashMap<String, Transfer>,
}

impl TransferLedger {
    pub fn prepare(&mut self, key: &str) -> Generation {
        self.replace(key, false)
    }

    pub fn hold_for_reload(&mut self, key: &str) -> Generation {
        self.replace(key, true)
    }

    fn replace(&mut self, key: &str, hold: bool) -> Generation {
        let generation = self
            .last
            .checked_add(1)
            .and_then(Generation::new)
            .expect("transfer generations exhausted");
        self.last = generation.get();
        let transfer = self.entries.entry(key.into()).or_default();
        if transfer.prepared.is_some() && transfer.parked == transfer.prepared {
            transfer.parked = Some(generation);
        }
        transfer.prepared = Some(generation);
        transfer.reload = hold.then_some(generation);
        generation
    }

    pub fn is_prepared(&self, key: &str) -> bool {
        self.entries
            .get(key)
            .is_some_and(|entry| entry.prepared.is_some())
    }

    pub fn park(&mut self, key: &str) -> Option<Generation> {
        let entry = self.entries.get_mut(key)?;
        let generation = entry.prepared?;
        entry.parked = Some(generation);
        Some(generation)
    }

    pub fn drop_preparation(&mut self, key: &str) -> Option<Generation> {
        let entry = self.entries.get_mut(key)?;
        entry.reload = None;
        let generation = entry.prepared.take();
        if entry.parked.is_none() {
            self.entries.remove(key);
        }
        generation
    }

    pub fn claim(&mut self, key: &str) {
        self.entries.remove(key);
    }

    pub fn expire_reload_hold(&mut self, key: &str, generation: Generation) -> bool {
        if !self.entries.get(key).is_some_and(|entry| {
            entry.reload == Some(generation) && entry.prepared == Some(generation)
        }) {
            return false;
        }
        self.drop_preparation(key);
        true
    }

    pub fn retire_parked(&mut self, key: &str, generation: Generation) -> bool {
        if self.parked_generation(key) != Some(generation) {
            return false;
        }
        self.unpark(key);
        true
    }

    pub fn unpark(&mut self, key: &str) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.parked = None;
            if entry.prepared.is_none() {
                self.entries.remove(key);
            }
        }
    }

    pub fn parked_generation(&self, key: &str) -> Option<Generation> {
        self.entries.get(key)?.parked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_reload_and_retirement_leave_the_new_generation_intact() {
        let mut ledger = TransferLedger::default();
        let first = ledger.hold_for_reload("tab");
        assert_eq!(ledger.park("tab"), Some(first));
        let second = ledger.hold_for_reload("tab");
        assert!(!ledger.expire_reload_hold("tab", first));
        assert!(!ledger.retire_parked("tab", first));
        assert!(ledger.expire_reload_hold("tab", second));
        assert!(!ledger.is_prepared("tab"));
        assert_eq!(ledger.parked_generation("tab"), Some(second));
        assert!(ledger.retire_parked("tab", second));
        assert_eq!(ledger.parked_generation("tab"), None);
    }
}
