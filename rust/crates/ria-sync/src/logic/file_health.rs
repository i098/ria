// Ported from ardia packages/sync/src/fileHealth.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The health of the files of a scope after a pull: damaged files, files set aside as bad, and
//! files that the transport still downloads.
//!
//! Changes from Ardia:
//! - `state.pending` is a `u64` (`scope_state.rs`).
//! - `pendingSince` keeps the order of `files.pending`; a JS object puts integer-like file ids
//!   first. Only the order of the saved JSON can differ, not a value.

use super::js::js_min;
use super::scope_state::{FileKind, FileMeta, ScopeState};
use indexmap::{IndexMap, IndexSet};

/// Port of `PulledFiles` (`packages/sync/src/fileHealth.ts:3`).
#[derive(Clone, Debug, Default)]
pub struct PulledFiles {
    pub read: Vec<String>,
    pub unreadable: Vec<String>,
    pub pending: Vec<String>,
    pub present: IndexSet<String>,
}

fn set_aside(state: &ScopeState, id: &str) -> bool {
    state.files.get(id).is_some_and(|f| f.k == FileKind::Bad)
}

/// Port of `noteFileHealth` (`packages/sync/src/fileHealth.ts:5`).
pub fn note_file_health(state: &mut ScopeState, files: &PulledFiles, now: f64, give_up: f64) {
    for id in &files.read {
        state.damaged.shift_remove(id);
    }
    for id in &files.unreadable {
        if set_aside(state, id) {
            continue;
        }
        let since = js_min(state.damaged.get(id).copied().unwrap_or(now), now);
        if now - since > give_up {
            let bad = FileMeta {
                d: String::new(),
                k: FileKind::Bad,
                s: None,
                vv: None,
                at: now,
                b: None,
            };
            match state.files.get_mut(id) {
                Some(slot) => *slot = bad,
                None => {
                    state.files.insert(id.clone(), bad);
                }
            }
            state.damaged.shift_remove(id);
        } else if let Some(slot) = state.damaged.get_mut(id) {
            *slot = since;
        } else {
            state.damaged.insert(id.clone(), since);
        }
    }
    state.damaged.retain(|id, _| files.present.contains(id));
    let pending: Vec<&String> = files
        .pending
        .iter()
        .filter(|id| !set_aside(state, id))
        .collect();
    state.pending = pending.len() as u64;
    state.pending_since = if pending.is_empty() {
        None
    } else {
        let old = state.pending_since.as_ref();
        let since: IndexMap<String, f64> = pending
            .into_iter()
            .map(|id| {
                (
                    id.clone(),
                    js_min(old.and_then(|o| o.get(id)).copied().unwrap_or(now), now),
                )
            })
            .collect();
        Some(since)
    };
}

/// Port of `holdsBack` (`packages/sync/src/fileHealth.ts:25`).
pub fn holds_back(state: &ScopeState, now: f64, window: f64) -> bool {
    !state.damaged.is_empty()
        || state
            .pending_since
            .as_ref()
            .is_some_and(|p| p.values().any(|at| now - at <= window))
}

/// Port of `pullSettled` (`packages/sync/src/fileHealth.ts:28`).
pub fn pull_settled(state: &ScopeState) -> bool {
    state.pending == 0
        && state.damaged.is_empty()
        && !state.files.values().any(|f| f.k == FileKind::Bad)
}
