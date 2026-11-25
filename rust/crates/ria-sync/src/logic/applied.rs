// Ported from ardia packages/sync/src/applied.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Which log files a scope has applied: the version vector `vv` plus the sequence numbers
//! above a gap (`extra`), sorted.
//!
//! Changes from Ardia:
//! - Sequence numbers are `u64` (`scope_state.rs`), so a fraction or a negative number cannot
//!   occur; a caller that has `Infinity` (`f.s ?? Infinity`) passes `u64::MAX`.

use super::scope_state::ScopeState;
use indexmap::IndexMap;

/// Port of `covered` (`packages/sync/src/applied.ts:3`).
pub fn covered(vv: &IndexMap<String, u64>, (device, seq): (&str, u64)) -> bool {
    vv.get(device).copied().unwrap_or(0) >= seq
}

/// Port of `sortedIndex` (`packages/sync/src/applied.ts:5`): the first index whose number is not
/// below `seq`.
fn sorted_index(list: &[u64], seq: u64) -> usize {
    list.partition_point(|&s| s < seq)
}

/// `map[key] = value` of a JS object: a known key keeps its place, a new key goes last.
fn set(map: &mut IndexMap<String, u64>, key: &str, value: u64) {
    match map.get_mut(key) {
        Some(slot) => *slot = value,
        None => {
            map.insert(key.to_owned(), value);
        }
    }
}

/// Port of `markApplied` (`packages/sync/src/applied.ts:16`).
pub fn mark_applied(state: &mut ScopeState, device: &str, seq: u64) {
    if covered(&state.vv, (device, seq)) {
        return;
    }
    let mut top = state.vv.get(device).copied().unwrap_or(0);
    if !state.extra.contains_key(device) {
        state.extra.insert(device.to_owned(), Vec::new());
    }
    let Some(extra) = state.extra.get_mut(device) else {
        return;
    };
    let at = sorted_index(extra, seq);
    if extra.get(at) != Some(&seq) {
        extra.insert(at, seq);
    }
    let mut taken = 0;
    while extra
        .get(taken)
        .is_some_and(|&s| Some(s) == top.checked_add(1))
    {
        top = extra[taken];
        taken += 1;
    }
    extra.drain(..taken);
    let left = !extra.is_empty();
    set(&mut state.vv, device, top);
    if !left {
        state.extra.shift_remove(device);
    }
}

/// Port of `isApplied` (`packages/sync/src/applied.ts:30`).
pub fn is_applied(state: &ScopeState, (device, seq): (&str, u64)) -> bool {
    if covered(&state.vv, (device, seq)) {
        return true;
    }
    state
        .extra
        .get(device)
        .is_some_and(|extra| extra.get(sorted_index(extra, seq)) == Some(&seq))
}

/// Port of `raiseApplied` (`packages/sync/src/applied.ts:36`).
pub fn raise_applied(state: &mut ScopeState, device: &str, seq: u64) {
    if seq <= state.vv.get(device).copied().unwrap_or(0) {
        return;
    }
    set(&mut state.vv, device, seq);
    let mut extra: Vec<u64> = state
        .extra
        .get(device)
        .map(|e| e.iter().copied().filter(|&s| s > seq).collect())
        .unwrap_or_default();
    let mut taken = 0;
    let mut top = seq;
    while extra
        .get(taken)
        .is_some_and(|&s| Some(s) == top.checked_add(1))
    {
        set(&mut state.vv, device, extra[taken]);
        taken += 1;
        top += 1;
    }
    extra.drain(..taken);
    let left = extra;
    if left.is_empty() {
        state.extra.shift_remove(device);
    } else if let Some(slot) = state.extra.get_mut(device) {
        *slot = left;
    } else {
        state.extra.insert(device.to_owned(), left);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::scope_state::empty_scope;

    #[test]
    fn gaps_close_in_order() {
        let mut state = empty_scope();
        mark_applied(&mut state, "a", 3);
        mark_applied(&mut state, "a", 2);
        assert_eq!(state.extra["a"], vec![2, 3]);
        assert!(!is_applied(&state, ("a", 1)));
        assert!(is_applied(&state, ("a", 3)));
        mark_applied(&mut state, "a", 1);
        assert_eq!(state.vv["a"], 3);
        assert!(!state.extra.contains_key("a"));
        state.extra.insert("a".into(), vec![5, 6, 8]);
        raise_applied(&mut state, "a", 4);
        assert_eq!((state.vv["a"], &state.extra["a"]), (6, &vec![8]));
    }
}
