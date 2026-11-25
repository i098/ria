// Ported from ardia apps/browser/src/state/browser/store.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! `hydrate` of the browser store: it builds the state from the saved data.
//!
//! Changes from Ardia:
//! - `Object.keys`, `Object.values` and `Object.entries` list keys in insertion order. A `HashMap` has no order, so
//!   `hydrate` lists the keys it must order (`profileOrder`, `windowOrder` and the tab list of a window) in sorted order
//!   after the saved order.
//! - The zustand wiring of store.ts (`create`, `batched`, `transactions`, `storeAction`, `useBrowser`) is not data.
//!   The kernel (`AppState`, `get_mut` and the change counters) replaces it, and `apply` in `mod.rs` runs `settle`
//!   after each action that changed the state, as the transaction middleware does after each write.

use std::collections::{HashMap, HashSet};

use super::model::{engine_profile, is_incognito_profile};
use super::pending_siblings::*;
use super::types::*;

fn sorted_keys<T>(map: &HashMap<String, T>) -> Vec<String> {
    let mut keys: Vec<String> = map.keys().cloned().collect();
    keys.sort();
    keys
}

/// `[...order.filter(in map), ...keys(map).filter(not in order)]` (store.ts:106-109, 139).
fn ordered<T>(order: &[String], map: &HashMap<String, T>) -> Vec<String> {
    let mut ids: Vec<String> = order
        .iter()
        .filter(|id| map.contains_key(*id))
        .cloned()
        .collect();
    ids.extend(
        sorted_keys(map)
            .into_iter()
            .filter(|id| !order.contains(id)),
    );
    ids
}

/// `hydrate` (store.ts:98): the new state for `data`, from the old state `s`.
pub fn hydrate(s: &BrowserState, data: HydrateData) -> BrowserState {
    let profiles = match data.profiles {
        Some(p) if !p.is_empty() => p,
        _ => {
            let default = default_profile();
            HashMap::from([(default.id.clone(), default)])
        }
    };
    let profile_order = ordered(data.profile_order.as_deref().unwrap_or(&[]), &profiles);
    let kept = with_toolbar_kept_while_scrolling(
        data.settings.as_ref(),
        &parse_settings_meta(data.settings_meta.as_ref()),
    );
    let mut settings = parse_settings(kept.0.as_ref());
    if !profiles.contains_key(&settings.default_profile_id) {
        settings.default_profile_id = profile_order[0].clone();
    }

    let pin_choices = data.pin_choices.unwrap_or_default();
    note_pin_choices(&pin_choices);
    let raw_windows = data.windows.unwrap_or_default();
    let profile_ok = |p: &str| profiles.contains_key(p) || is_incognito_profile(p);
    let mut tabs: HashMap<String, Tab> = HashMap::new();
    for t in data.tabs.unwrap_or_default().into_values() {
        if !raw_windows.contains_key(&t.window_id) || !profile_ok(&t.profile_id) {
            continue;
        }
        let pinned_url = t
            .pinned_url
            .clone()
            .or_else(|| pin_home(&t, t.pin.is_some()));
        tabs.insert(t.id.clone(), Tab { pinned_url, ..t });
    }
    let groups_in = data.groups.unwrap_or_default();
    let mut windows: HashMap<String, BrowserWindow> = HashMap::new();
    for w in raw_windows.into_values() {
        let own: Vec<String> = w
            .tab_ids
            .iter()
            .filter(|id| tabs.get(*id).is_some_and(|t| t.window_id == w.id))
            .cloned()
            .collect();
        let tab_ids = order_sections(own, &tabs, &groups_in);
        if tab_ids.is_empty() {
            continue;
        }
        let mut active_tab_ids: HashMap<String, String> = w
            .active_tab_ids
            .iter()
            .filter(|(p, id)| tabs.get(*id).is_some_and(|t| &t.profile_id == *p))
            .map(|(p, id)| (p.clone(), id.clone()))
            .collect();
        let profile_id = if tab_ids.iter().any(|id| tabs[id].profile_id == w.profile_id) {
            w.profile_id.clone()
        } else {
            tabs[&tab_ids[0]].profile_id.clone()
        };
        if !active_tab_ids.contains_key(&profile_id) {
            let first = tab_ids
                .iter()
                .find(|id| tabs[*id].profile_id == profile_id)
                .cloned()
                .unwrap_or_default();
            active_tab_ids.insert(profile_id.clone(), first);
        }
        windows.insert(
            w.id.clone(),
            BrowserWindow {
                tab_ids,
                profile_id,
                active_tab_ids,
                ..w
            },
        );
    }
    tabs.retain(|_, t| windows.contains_key(&t.window_id));
    let window_order = ordered(data.window_order.as_deref().unwrap_or(&[]), &windows);
    let groups: HashMap<String, TabGroup> = groups_in
        .into_values()
        .map(|g| TabGroup {
            tab_ids: g
                .tab_ids
                .iter()
                .filter(|id| tabs.get(*id).is_some_and(|t| t.window_id == g.window_id))
                .cloned()
                .collect(),
            ..g
        })
        .filter(|g| !g.tab_ids.is_empty())
        .map(|g| (g.id.clone(), g))
        .collect();
    let splits = sanitize_splits(&data.splits.unwrap_or_default(), &tabs);
    let mut bookmarks = s.bookmarks.clone();
    for engine in engine_profiles(&profiles) {
        bookmarks = ensure_roots(bookmarks, &engine);
    }
    let focused = data
        .focused_window_id
        .filter(|id| windows.contains_key(id))
        .or_else(|| window_order.last().cloned());

    let engines: HashSet<String> = profiles
        .keys()
        .map(|id| engine_profile(&profiles, id))
        .collect();
    let keep_profiles = |p: &String| profiles.contains_key(p);
    let mut ui = s.ui.clone();
    ui.focus_order = focused.map_or_else(Vec::new, |f| {
        std::iter::once(f.clone())
            .chain(window_order.iter().rev().filter(|id| **id != f).cloned())
            .collect()
    });
    ui.last_profile_id = data.last_profile_id.filter(|id| profiles.contains_key(id));

    let mut next = BrowserState {
        orphaned_profile_data: data
            .orphaned_profile_data
            .unwrap_or_default()
            .into_iter()
            .filter(|e| !engines.contains(e))
            .collect(),
        settings,
        settings_meta: kept.1,
        windows,
        window_order,
        tabs,
        groups,
        splits,
        closed_tabs: data.closed_tabs.unwrap_or_default(),
        closed_windows: data.closed_windows.unwrap_or_default(),
        parked_pins: data
            .parked_pins
            .unwrap_or_default()
            .into_iter()
            .filter(|(p, _)| keep_profiles(p))
            .collect(),
        pin_choices: pin_choices
            .into_iter()
            .filter(|(p, _)| keep_profiles(p))
            .collect(),
        pin_sync: data
            .pin_sync
            .unwrap_or_default()
            .into_iter()
            .filter(|(p, _)| keep_profiles(p))
            .collect(),
        top_apps_merged: data.top_apps_merged.unwrap_or_default(),
        closed_groups: data.closed_groups.unwrap_or_default(),
        deleted_groups: kept_deleted_groups(data.deleted_groups.unwrap_or_default()),
        cleaned_tabs: kept_cleaned_tabs(
            data.cleaned_tabs
                .unwrap_or_default()
                .into_iter()
                .filter(|c| !is_incognito_profile(&c.tab.profile_id))
                .collect(),
        ),
        private_cleaned_tabs: HashMap::new(),
        private_site_mutes: HashMap::new(),
        selection: HashMap::new(),
        selection_anchors: HashMap::new(),
        history: HashMap::new(),
        bookmarks,
        bookmarks_ready: HashMap::new(),
        downloads: data.downloads.unwrap_or_default(),
        window_ui: HashMap::new(),
        peeks: HashMap::new(),
        find: HashMap::new(),
        ui,
        profile_order,
        profiles,
    };
    for id in next.ui.focus_order.clone() {
        let w = &next.windows[&id];
        let active = w.active_tab_ids[&w.profile_id].clone();
        next = activate_tab(next, &active);
    }
    forget_all_live();
    next
}
