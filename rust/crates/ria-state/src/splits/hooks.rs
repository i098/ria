// Ported from ardia apps/browser/src/state/splits/hooks.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The two selectors of `hooks.ts`.
//!
//! Changes from Ardia: the React hooks `useDragFallbackPane` and `useSplitSuggestions` are plain functions over the state;
//! the view that calls them reads the state and runs them each frame, so no subscription code is needed.

use std::collections::{HashMap, HashSet};

use super::splits::{split_tab_ids, window_tab_ids};
use crate::browser::{BrowserWindow, SplitView, Tab};

/// `useDragFallbackPane` (`hooks.ts:5`): the most recent other tab of the window, for a drag that leaves one pane.
pub fn drag_fallback_pane(
    tabs: &HashMap<String, Tab>,
    windows: &HashMap<String, BrowserWindow>,
    dragging: Option<&str>,
    panes: &HashSet<String>,
    window_id: &str,
) -> Option<String> {
    let dragging = dragging.filter(|d| !d.is_empty())?;
    if panes.len() != 1 || !panes.contains(dragging) {
        return None;
    }
    let profile = tabs.get(dragging).map(|t| t.profile_id.as_str());
    let ids = windows.get(window_id).map(window_tab_ids);
    let mut others: Vec<&Tab> = ids
        .into_iter()
        .flatten()
        .filter_map(|id| tabs.get(id))
        .filter(|t| t.id != dragging && Some(t.profile_id.as_str()) == profile)
        .collect();
    others.sort_by(|a, b| b.last_active_at.0.total_cmp(&a.last_active_at.0));
    others.first().map(|t| t.id.clone())
}

/// `useSplitSuggestions` (`hooks.ts:17`): up to 5 tabs of the window, newest first, that can join the split of `tab_id`.
pub fn split_suggestions(
    tabs: &HashMap<String, Tab>,
    windows: &HashMap<String, BrowserWindow>,
    splits: &HashMap<String, SplitView>,
    tab_id: &str,
) -> Vec<String> {
    let Some(tab) = tabs.get(tab_id) else {
        return vec![];
    };
    let Some(w) = windows.get(&tab.window_id) else {
        return vec![];
    };
    let mut in_split: HashSet<String> = splits
        .values()
        .map(split_tab_ids)
        .find(|ids| ids.iter().any(|id| id == tab_id))
        .unwrap_or_default()
        .into_iter()
        .collect();
    if in_split.is_empty() {
        in_split.insert(tab_id.to_string());
    }
    let mut out: Vec<&Tab> = window_tab_ids(w)
        .iter()
        .filter_map(|id| tabs.get(id))
        .filter(|t| {
            !t.url.is_empty() && t.profile_id == tab.profile_id && !in_split.contains(&t.id)
        })
        .collect();
    out.sort_by(|a, b| b.last_active_at.0.total_cmp(&a.last_active_at.0));
    out.into_iter().take(5).map(|t| t.id.clone()).collect()
}
