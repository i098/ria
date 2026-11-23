// Ported from ardia packages/ardiacore/ios/ArdiaTabPresses.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::collections::{HashSet, VecDeque};
use std::num::NonZeroU64;

pub type PressId = NonZeroU64;
const MAX_KEPT: usize = 64;
const SAME_EVENT_MS: f64 = 0.5;

#[derive(Default)]
pub struct TabPresses {
    last: u64,
    unheard: VecDeque<(String, PressId, f64)>,
    switches: VecDeque<(String, Option<PressId>)>,
    split_keys: HashSet<String>,
}

fn keep<T>(entries: &mut VecDeque<T>, entry: T) {
    if entries.len() == MAX_KEPT {
        entries.pop_front();
    }
    entries.push_back(entry);
}

impl TabPresses {
    pub fn began(&mut self, key: &str, stamp_ms: f64) -> PressId {
        let press = self
            .last
            .checked_add(1)
            .and_then(PressId::new)
            .expect("press sequence exhausted");
        self.last = press.get();
        keep(&mut self.unheard, (key.into(), press, stamp_ms));
        press
    }

    pub fn claim(&mut self, key: &str, stamp_ms: f64) -> Option<PressId> {
        let index = self.unheard.iter().position(|(recorded_key, _, when)| {
            recorded_key == key && (when - stamp_ms).abs() <= SAME_EVENT_MS
        })?;
        self.unheard.remove(index).map(|(_, press, _)| press)
    }

    pub fn take_unheard(&mut self, through: PressId) -> impl Iterator<Item = PressId> + '_ {
        let end = self
            .unheard
            .iter()
            .take_while(|(_, press, _)| *press <= through)
            .count();
        self.unheard.drain(..end).map(|(_, press, _)| press)
    }

    pub fn note_switch(&mut self, key: &str, press: Option<PressId>) {
        if key.is_empty() {
            return;
        }
        self.switches
            .retain(|(recorded_key, _)| recorded_key != key);
        keep(&mut self.switches, (key.into(), press));
    }

    pub fn take_switch(&mut self, key: &str) -> Option<PressId> {
        let index = self
            .switches
            .iter()
            .rposition(|(recorded_key, _)| recorded_key == key)?;
        self.switches
            .drain(..=index)
            .next_back()
            .and_then(|(_, press)| press)
    }

    pub fn forget_switches(&mut self, press: PressId) {
        self.switches
            .retain(|(_, recorded)| *recorded != Some(press));
    }

    pub fn set_splits(&mut self, keys: impl IntoIterator<Item = String>) {
        self.split_keys.clear();
        self.split_keys.extend(keys);
    }

    pub fn in_split(&self, key: &str) -> bool {
        self.split_keys.contains(key)
    }

    pub fn reset_bridge(&mut self) {
        self.unheard.clear();
        self.switches.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claim_matches_both_tab_and_timestamp_and_removes_only_its_press() {
        let mut presses = TabPresses::default();
        let first = presses.began("first", 100.0);
        let next = presses.began("next", 100.5);
        let later = presses.began("next", 101.5);
        assert_eq!(presses.claim("next", 100.0), Some(next));
        assert_eq!(presses.claim("next", 100.0), None);
        assert!(presses.take_unheard(next).eq([first]));
        assert_eq!(presses.claim("next", 101.5), Some(later));
    }

    #[test]
    fn taking_switch_drains_older_notes_but_not_newer_ones() {
        let mut presses = TabPresses::default();
        let first = presses.began("first", 100.0);
        let next = presses.began("next", 101.0);
        let later = presses.began("later", 102.0);
        presses.note_switch("first", Some(first));
        presses.note_switch("next", Some(next));
        presses.note_switch("later", Some(later));
        assert_eq!(presses.take_switch("next"), Some(next));
        assert_eq!(presses.take_switch("first"), None);
        assert_eq!(presses.take_switch("later"), Some(later));
    }
}
