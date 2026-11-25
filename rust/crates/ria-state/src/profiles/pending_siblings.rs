// Ported from ardia apps/browser/src/state/profiles/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Functions of sibling state folders that `state/profiles` calls and that have no signature in the slice 2.4 skeleton.
//! Each body is `todo!()`. The sibling slice that owns the function replaces the stub with the real call.
//! `rust/replay/profiles/PENDING.md` lists the recorded sequences that reach a stub.
//!
//! An Ardia function that takes the store value `s` and returns a new one takes `app: &mut AppState` here and changes it in place.

#![allow(unused_variables, clippy::todo)]

use crate::kernel::AppState;
use crate::settings::Settings;

// shortcut: stub for browser::newId (Ardia state/browser/model.ts:11), replace with the real call when slice 2.5 merges
pub fn new_id(prefix: &str) -> String {
    todo!("slice 2.5: newId")
}

// shortcut: stub for browser::isIncognitoProfile (Ardia state/browser/model.ts:20), replace with the real call when slice 2.5 merges
pub fn is_incognito_profile(profile_id: &str) -> bool {
    todo!("slice 2.5: isIncognitoProfile")
}

// shortcut: stub for browser::incognitoOriginal (Ardia state/browser/model.ts:21), replace with the real call when slice 2.5 merges
pub fn incognito_original(profile_id: &str) -> String {
    todo!("slice 2.5: incognitoOriginal")
}

// shortcut: stub for browser::engineProfile (Ardia state/browser/model.ts:26), replace with the real call when slice 2.5 merges
pub fn engine_profile(app: &AppState, profile_id: &str) -> String {
    todo!("slice 2.5: engineProfile")
}

// shortcut: stub for browser::viewTabIds (Ardia state/browser/model.ts:107), replace with the real call when slice 2.5 merges
pub fn view_tab_ids(app: &AppState, window_id: &str) -> Vec<String> {
    todo!("slice 2.5: viewTabIds")
}

// shortcut: stub for bookmarks::ensureRoots (Ardia state/bookmarks/bookmarks.ts:101), replace with the real call when slice 2.16 merges
/// Gives the bookmark roots of `engine` to the bookmarks state. Ardia returns `[bookmarks, roots]`; the caller uses `[0]`.
pub fn ensure_roots(app: &mut AppState, engine: &str) {
    todo!("slice 2.16: ensureRoots")
}

// shortcut: stub for bookmarks::engineBookmarkRoots (Ardia state/bookmarks/bookmarks.ts:82), replace with the real call when slice 2.16 merges
/// The ids of the bar root and the other root of `engine`, or `None`.
pub fn engine_bookmark_roots(app: &AppState, engine: &str) -> Option<[String; 2]> {
    todo!("slice 2.16: engineBookmarkRoots")
}

// shortcut: stub for bookmarks::removeBookmarkTree (Ardia state/bookmarks/bookmarks.ts), replace with the real call when slice 2.16 merges
pub fn remove_bookmark_tree(app: &mut AppState, roots: &[String]) {
    todo!("slice 2.16: removeBookmarkTree")
}

// shortcut: stub for pins::handPagesOutOfProfile (Ardia state/pins/pinShare.ts:248), replace with the real call when slice 2.13 merges
pub fn hand_pages_out_of_profile(app: &mut AppState, profile_id: &str) {
    todo!("slice 2.13: handPagesOutOfProfile")
}

// shortcut: stub for settings::settingsWith (Ardia state/settings/settings.ts:273), replace with the real call when slice 2.7 merges
/// `patch` is the `Partial<Settings>` as JSON.
pub fn settings_with(settings: &Settings, patch: serde_json::Value) -> Settings {
    todo!("slice 2.7: settingsWith")
}

// shortcut: stub for windows::closeWindow (Ardia state/windows/windows.ts:66), replace with the real call when slice 2.9 merges
pub fn close_window(app: &mut AppState, window_id: &str) {
    todo!("slice 2.9: closeWindow")
}

// shortcut: stub for tabs::newTab (Ardia state/tabs/tabsSlice.ts:29), replace with the real call when slice 2.8 merges
pub fn new_tab(app: &mut AppState, window_id: &str) -> String {
    todo!("slice 2.8: newTab")
}

// shortcut: stub for tabs::withNewTab (Ardia state/tabs/tabs.ts), replace with the real call when slice 2.8 merges
/// `withNewTab(s, windowId, { profileId, background: true })`, then `apply(next, activated(next, tab))`.
pub fn new_background_tab_activated(app: &mut AppState, window_id: &str, profile_id: &str) {
    todo!("slice 2.8: withNewTab")
}

// shortcut: stub for tabs::activated (Ardia state/tabs/tabs.ts:103), replace with the real call when slice 2.8 merges
/// `apply(s, activated(s, id))` (`tabs.ts:128`).
pub fn activate_tab(app: &mut AppState, tab_id: &str) {
    todo!("slice 2.8: activated")
}

// shortcut: stub for tabs::removeTabs (Ardia state/tabs/tabs.ts), replace with the real call when slice 2.8 merges
pub fn remove_tabs(app: &mut AppState, tab_ids: &[String], record: bool) {
    todo!("slice 2.8: removeTabs")
}

// shortcut: stub for tabs::needsPage (Ardia state/tabs/pageSession.ts), replace with the real call when slice 2.8 merges
pub fn needs_page(app: &AppState, tab_id: &str) -> bool {
    todo!("slice 2.8: needsPage")
}

// shortcut: stub for tabs::hasPage (Ardia state/tabs/pageSession.ts:13), replace with the real call when slice 2.8 merges
pub fn has_page(app: &AppState, tab_id: &str) -> bool {
    todo!("slice 2.8: hasPage")
}

// shortcut: stub for tabs::wokenPage (Ardia state/tabs/pageSession.ts), replace with the real call when slice 2.8 merges
/// `updateTab(id, { page: wokenPage(tab) })`.
pub fn wake_tab_page(app: &mut AppState, tab_id: &str) {
    todo!("slice 2.8: wokenPage")
}

// shortcut: stub for tabs::withWake (Ardia state/tabs/pageSession.ts), replace with the real call when slice 2.8 merges
/// `updateTab(id, { page: withWake(IDLE, wakeOf(tab)) })`.
pub fn idle_tab_page(app: &mut AppState, tab_id: &str) {
    todo!("slice 2.8: withWake")
}

// shortcut: stub for sync::adoptProfileTheme (Ardia state/sync/engine.ts:198), replace with the real call when slice 2.31 merges
pub fn adopt_profile_theme(app: &mut AppState, profile_id: &str) {
    todo!("slice 2.31: adoptProfileTheme")
}

// shortcut: stub for telemetry::noteProfileUnloaded (Ardia state/telemetry/tabSleepCounts.ts:39), replace with the real call when slice 2.6 merges
pub fn note_profile_unloaded(app: &mut AppState) {
    todo!("slice 2.6: noteProfileUnloaded")
}

// shortcut: stub for telemetry::noteUnloadedProfileReopened (Ardia state/telemetry/tabSleepCounts.ts:41), replace with the real call when slice 2.6 merges
pub fn note_unloaded_profile_reopened(app: &mut AppState) {
    todo!("slice 2.6: noteUnloadedProfileReopened")
}

// shortcut: stub for telemetry::noteThemePreset (Ardia state/telemetry/themePresetCounts.ts:4), replace with the real call when slice 2.6 merges
pub fn note_theme_preset(app: &mut AppState, event: &str) {
    todo!("slice 2.6: noteThemePreset")
}

// shortcut: stub for telemetry::noteProfileDataDeleted (Ardia state/telemetry/profileDataCounts.ts:7), replace with the real call when slice 2.6 merges
pub fn note_profile_data_deleted(app: &mut AppState, folder_gone: bool) {
    todo!("slice 2.6: noteProfileDataDeleted")
}

// shortcut: stub for telemetry::warnFor (Ardia state/telemetry/logs.ts:7), replace with the real call when slice 2.6 merges
pub fn warn_for(app: &mut AppState, message: &str) {
    todo!("slice 2.6: warnFor")
}

// shortcut: stub for engines::forgetSiteDataProfile (Ardia state/engines/siteData.ts), replace with the real call when slice 2.21 merges
pub fn forget_site_data_profile(app: &mut AppState, engine: &str) {
    todo!("slice 2.21: forgetSiteDataProfile")
}

// shortcut: stub for app::menuSections (Ardia state/app/menuSections.ts:9), replace with the real call when slice 2.10 merges
/// Joins the sections of menu items with separators; a slot that is `false` is dropped.
pub fn menu_sections(sections: Vec<Vec<Option<serde_json::Value>>>) -> Vec<serde_json::Value> {
    todo!("slice 2.10: menuSections")
}

// shortcut: stub for app::chooseFromMenu (Ardia state/app/menuChoice.ts:12), replace with the real call when slice 2.10 merges
/// Shows the menu; the choice comes back as `Action::ProfileMenuChosen`.
pub fn choose_from_menu(app: &mut AppState, items: Vec<serde_json::Value>) {
    todo!("slice 2.10: chooseFromMenu")
}

// shortcut: stub for import::parseArcTheme and the platform importModule and menus calls (Ardia state/profiles/arcThemes.ts:19), replace with the real call when slice 2.40 merges
/// `importArcTheme(profileId)`: lists the Arc spaces, shows the menu, and sets the theme of the picked space.
pub fn import_arc_theme(app: &mut AppState, profile_id: &str) {
    todo!("slice 2.40: importArcTheme")
}
