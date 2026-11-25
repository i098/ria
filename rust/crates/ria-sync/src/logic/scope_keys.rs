// Ported from ardia packages/sync/src/scopeKeys.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The writes to `records` and `base` of a scope, with the trackers and the base index that
//! watch them.
//!
//! Changes from Ardia:
//! - The `touching` and `baseIndexes` `WeakMap`s are `state.runtime.touching` and
//!   `state.runtime.base_index`.
//! - `trackTouchedKeys` returns a `Tracker`; its `stop` takes the state, because Rust cannot keep
//!   the borrowed state in the closure.
//! - `withBaseIndex` takes an async closure that gets the state, as the run of Ardia captures it.
//! - `baseKeysWith` returns a copy of the cached key list, not the list itself.
//! - Keys keep insertion order; a JS object puts integer-like keys first. No adapter prefix
//!   makes a key integer-like, so the app cannot reach the difference.

use super::scope_state::{BaseIndex, ScopeState, SyncRecord, TouchedKeys};
use indexmap::IndexMap;
use serde_json::Value;
use std::cell::RefCell;
use std::rc::Rc;

/// Port of `visible` (`packages/sync/src/scopeKeys.ts:3`).
pub fn visible(state: &ScopeState, prefix: &str) -> IndexMap<String, Value> {
    state
        .records
        .iter()
        .filter(|(key, record)| key.starts_with(prefix) && !record.v.is_null())
        .map(|(key, record)| (key.clone(), record.v.clone()))
        .collect()
}

/// Port of `recordsWith` (`packages/sync/src/scopeKeys.ts:14`).
pub fn records_with(state: &ScopeState, prefix: &str) -> Vec<(String, SyncRecord)> {
    state
        .records
        .iter()
        .filter(|(key, _)| key.starts_with(prefix))
        .map(|(key, record)| (key.clone(), record.clone()))
        .collect()
}

/// What `trackTouchedKeys` returns: the keys touched so far, and `stop`.
pub struct Tracker {
    pub touched: Rc<RefCell<TouchedKeys>>,
}

impl Tracker {
    /// Port of `stop` (`packages/sync/src/scopeKeys.ts:31`).
    pub fn stop(&self, state: &mut ScopeState) {
        state
            .runtime
            .touching
            .retain(|t| !Rc::ptr_eq(t, &self.touched));
    }
}

/// Port of `trackTouchedKeys` (`packages/sync/src/scopeKeys.ts:24`).
pub fn track_touched_keys(state: &mut ScopeState) -> Tracker {
    let touched = Rc::new(RefCell::new(TouchedKeys::default()));
    state.runtime.touching.push(Rc::clone(&touched));
    Tracker { touched }
}

/// Port of `touch` (`packages/sync/src/scopeKeys.ts:38`); `records` picks the kind.
fn touch(state: &ScopeState, records: bool, key: &str) {
    for touched in &state.runtime.touching {
        let mut touched = touched.borrow_mut();
        let set = if records {
            &mut touched.records
        } else {
            &mut touched.base
        };
        set.insert(key.to_string());
    }
}

/// Port of `putRecord` (`packages/sync/src/scopeKeys.ts:43`).
pub fn put_record(state: &mut ScopeState, key: &str, record: SyncRecord) {
    state.records.insert(key.to_string(), record);
    touch(state, true, key);
}

/// Port of `dropRecord` (`packages/sync/src/scopeKeys.ts:48`).
pub fn drop_record(state: &mut ScopeState, key: &str) {
    if state.records.shift_remove(key).is_none() {
        return;
    }
    touch(state, true, key);
}

/// Port of `baseKeysWith` (`packages/sync/src/scopeKeys.ts:57`).
pub fn base_keys_with(state: &mut ScopeState, prefix: &str) -> Vec<String> {
    let Some(index) = state.runtime.base_index.as_mut() else {
        return state
            .base
            .keys()
            .filter(|key| key.starts_with(prefix))
            .cloned()
            .collect();
    };
    let all = &index.keys;
    index
        .by_prefix
        .entry(prefix.to_string())
        .or_insert_with(|| {
            all.iter()
                .filter(|key| key.starts_with(prefix))
                .cloned()
                .collect()
        })
        .clone()
}

/// Port of `putBase` (`packages/sync/src/scopeKeys.ts:70`).
pub fn put_base(state: &mut ScopeState, key: &str, json: String) {
    let known = state.base.get(key);
    if known == Some(&json) {
        return;
    }
    let is_new = known.is_none();
    state.base.insert(key.to_string(), json);
    touch(state, false, key);
    let Some(index) = state.runtime.base_index.as_mut().filter(|_| is_new) else {
        return;
    };
    index.keys.push(key.to_string());
    for (prefix, keys) in &mut index.by_prefix {
        if key.starts_with(prefix.as_str()) {
            keys.push(key.to_string());
        }
    }
}

/// Port of `dropBase` (`packages/sync/src/scopeKeys.ts:83`).
pub fn drop_base(state: &mut ScopeState, key: &str) {
    if state.base.shift_remove(key).is_none() {
        return;
    }
    touch(state, false, key);
}

/// Port of `withBaseIndex` (`packages/sync/src/scopeKeys.ts:89`).
pub async fn with_base_index<T>(
    state: &mut ScopeState,
    run: impl AsyncFnOnce(&mut ScopeState) -> T,
) -> T {
    state.runtime.base_index = Some(BaseIndex {
        keys: state.base.keys().cloned().collect(),
        by_prefix: Default::default(),
    });
    let out = run(state).await;
    state.runtime.base_index = None;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::scope_state::empty_scope;

    #[test]
    fn base_index_keeps_dropped_keys_and_adds_re_puts() {
        let mut state = empty_scope();
        put_base(&mut state, "x:a", "1".into());
        pollster::block_on(with_base_index(
            &mut state,
            async |state: &mut ScopeState| {
                assert_eq!(base_keys_with(state, "x:"), ["x:a"]);
                drop_base(state, "x:a");
                assert_eq!(base_keys_with(state, "x:"), ["x:a"]);
                put_base(state, "x:a", "2".into());
                put_base(state, "x:a", "3".into());
                assert_eq!(base_keys_with(state, "x:"), ["x:a", "x:a"]);
            },
        ));
        assert!(state.runtime.base_index.is_none());
        assert_eq!(base_keys_with(&mut state, "x:"), ["x:a"]);
    }

    #[test]
    fn trackers_see_writes_until_stopped() {
        let mut state = empty_scope();
        let tracker = track_touched_keys(&mut state);
        put_base(&mut state, "k", "1".into());
        drop_record(&mut state, "missing");
        tracker.stop(&mut state);
        put_base(&mut state, "j", "1".into());
        let touched = tracker.touched.borrow();
        assert_eq!(touched.base.iter().collect::<Vec<_>>(), ["k"]);
        assert!(touched.records.is_empty());
    }
}
