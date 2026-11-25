// Ported from ardia apps/browser/src/state/browser/actions.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The exported store actions of `actions.ts` that need no dialog answer. Each one reads and writes the browser store and
//! asks the platform through `Effects`.
//!
//! Changes from Ardia:
//! - `store()` is the `BrowserState` of the module; a native call is a `PlatformCall` pushed on `Effects`.
//! - `mayWrite` and `foreignWriteHooks.around` (foreignWriteHooks.ts, actions.ts:53) are callbacks of the UI, not
//!   data, so they are not in the action JSON. The write always runs.
//! - The actions that await a dialog (`closeTab`, `closeGroup`, `deleteProfile`, `moveTabsToProfile`,
//!   `moveTabToProfile`, `moveGroupToProfile`, `dropTabsInProfile`, `openProfileMenu`) become two actions (plan
//!   section 6.5). They are not written yet; see `rust/replay/browser/PENDING.md`.

use super::model::resolve_window_id;
use super::pending_siblings::*;
use super::types::*;
use crate::kernel::Effects;
use crate::platform::PlatformCall;

/// `focus` (actions.ts:43).
pub fn focus(s: &mut BrowserState, fx: &mut Effects, window_id: &str) {
    set_focused_window(s, window_id);
    fx.push(PlatformCall::AppWindowsFocusWindow {
        id: window_id.to_string(),
    });
}

/// `switchToTab` (actions.ts:48).
pub fn switch_to_tab(s: &mut BrowserState, fx: &mut Effects, tab_id: &str) {
    let Some(window_id) = s.tabs.get(tab_id).map(|t| t.window_id.clone()) else {
        return;
    };
    activate(s, tab_id);
    if Some(&window_id) != focused_window_of(s).as_ref() {
        focus(s, fx, &window_id);
    }
}

/// `toggleMute` (actions.ts:97).
pub fn toggle_mute(s: &mut BrowserState, tab_id: &str) {
    let Some(tab) = s.tabs.get(tab_id).cloned() else {
        return;
    };
    if !is_site_muted(s, &tab) {
        return set_muted(s, &[tab_id.to_string()], !tab.muted);
    }
    set_site_muted(s, tab_id, false);
    if tab.muted {
        set_muted(s, &[tab_id.to_string()], false);
    }
}

/// `toggleSiteMute` (actions.ts:109).
pub fn toggle_site_mute(s: &mut BrowserState, tab_id: &str) {
    if let Some(tab) = s.tabs.get(tab_id).cloned() {
        let muted = is_site_muted(s, &tab);
        set_site_muted(s, tab_id, !muted);
    }
}

/// `moveGroupToWindow` (actions.ts:216).
pub fn move_group_to_window_action(
    s: &mut BrowserState,
    fx: &mut Effects,
    group_id: &str,
    target: Option<&str>,
) {
    if let Some(window_id) = move_group_to_window(s, group_id, target) {
        focus(s, fx, &window_id);
    }
}

/// `moveTabToWindow` (actions.ts:221).
pub fn move_tab_to_window(
    s: &mut BrowserState,
    fx: &mut Effects,
    tab_id: &str,
    target: Option<&str>,
) {
    if let Some(window_id) = move_tabs_to_window(s, &[tab_id.to_string()], target) {
        focus(s, fx, &window_id);
    }
}

/// `switchProfile` (actions.ts:226). The animated move runs the pager of the window (`pageToProfile`).
pub fn switch_profile_action(
    s: &mut BrowserState,
    window_id: &str,
    profile_id: &str,
    animated: bool,
) {
    if animated {
        page_to_profile(s, window_id, profile_id);
    } else {
        switch_profile(s, window_id, profile_id);
    }
}

/// `cycleTarget` (actions.ts:249).
pub fn cycle_target(s: &BrowserState, window_id: &str, delta: i64) -> Option<String> {
    let w = s.windows.get(window_id)?;
    if w.kind == WindowKind::Private || s.profile_order.len() < 2 {
        return None;
    }
    let n = s.profile_order.len() as i64;
    let i = s
        .profile_order
        .iter()
        .position(|p| p == &w.profile_id)
        .map_or(-1, |i| i as i64);
    Some(s.profile_order[(i + delta).rem_euclid(n) as usize].clone())
}

/// `cycleProfile` (actions.ts:233): the pager steps first; when it has no pager to step, the store switches.
pub fn cycle_profile(s: &mut BrowserState, window_id: &str, delta: i64) {
    if cycle_target(s, window_id, delta).is_none() {
        return;
    }
    let profile_order = s.profile_order.clone();
    if !step_pager(window_id, delta, &profile_order)
        && let Some(target) = cycle_target(s, window_id, delta)
    {
        switch_profile_action(s, window_id, &target, true);
    }
}

/// `openUrls` (actions.ts:261).
pub fn open_urls(s: &mut BrowserState, urls: &[String], window_id: Option<&str>) {
    let mut target = resolve_window_id(s, window_id);
    for url in urls {
        let private = target
            .as_ref()
            .and_then(|t| s.windows.get(t))
            .is_some_and(|w| w.kind == WindowKind::Private);
        match &target {
            Some(t) if !private => new_tab(s, t, url),
            _ => {
                let options = crate::windows::CreateWindowOptions {
                    url: Some(url.clone()),
                    ..Default::default()
                };
                target = Some(create_window(s, Some(&options)));
            }
        }
    }
}
