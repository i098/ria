// Ria original, no Ardia source.

//! Stubs for the functions of sibling state folders that the browser store calls. The 2.4 skeleton has no signature for
//! them. Each stub has its Ardia name and a `todo!()` body, and the slice that ports the sibling folder replaces it with
//! the real call. `rust/replay/browser/PENDING.md` lists each stub and the recorded sequences that reach it.

#![allow(unused_variables)]

use std::collections::HashMap;

use serde_json::Value;

use super::types::*;

/// A step of `settle.ts` after `patched(..)`: the new state, or `None` when the step changes nothing.
pub type Step = fn(&BrowserState, &BrowserState) -> Option<BrowserState>;

macro_rules! step {
    ($name:ident, $loc:literal, $slice:literal) => {
        // shortcut: stub for $loc (the settle step $name), replace with the real call when slice $slice merges
        pub fn $name(s: &BrowserState, prev: &BrowserState) -> Option<BrowserState> {
            todo!(concat!("slice ", $slice, ": ", stringify!($name)))
        }
    };
}

step!(
    splits_where_unavailable,
    "splits::splitsWhereUnavailable (Ardia splits/splits.ts)",
    "2.14"
);
step!(
    forget_closed_window_mutes,
    "tabs::forgetClosedWindowMutes (Ardia tabs/mute.ts)",
    "2.8"
);
step!(
    forget_closed_window_clean_ups,
    "pins::forgetClosedWindowCleanUps (Ardia pins/tabCleanup.ts)",
    "2.13"
);
step!(
    forget_moved_anchors,
    "pins::forgetMovedAnchors (Ardia pins/organize.ts)",
    "2.13"
);
step!(
    settle_nesting,
    "groups::settleNesting (Ardia groups/groups.ts:146)",
    "2.23"
);
step!(
    settle_shown_tabs,
    "groups::settleShownTabs (Ardia groups/shownTabs.ts)",
    "2.23"
);
step!(
    live_pin_tables,
    "pins::livePinTables (Ardia pins/pinChoices.ts)",
    "2.13"
);
step!(
    settle_peeks,
    "peek::settlePeeks (Ardia peek/peek.ts)",
    "2.19"
);
step!(
    settle_pin_homes,
    "pins::settlePinHomes (Ardia pins/pinHomes.ts)",
    "2.13"
);
step!(
    unshare_tabs,
    "tabs::unshareTabs (Ardia tabs/tabShare.ts)",
    "2.8"
);
step!(
    pages_into_splits,
    "tabs::pagesIntoSplits (Ardia tabs/tabShare.ts)",
    "2.8"
);
step!(
    settle_shared_pins,
    "pins::settleSharedPins (Ardia pins/pinShare.ts)",
    "2.13"
);
step!(
    settle_tile_folders,
    "pins::settleTileFolders (Ardia pins/tileFolders.ts)",
    "2.13"
);
step!(
    mirror_pins,
    "pins::mirrorPins (Ardia pins/pinMirror.ts)",
    "2.13"
);
step!(
    mirror_top_apps,
    "pins::mirrorTopApps (Ardia pins/topApps.ts)",
    "2.13"
);
step!(
    mirror_tabs,
    "tabs::mirrorTabs (Ardia tabs/tabShare.ts)",
    "2.8"
);
step!(
    pinned_tabs_leave_live,
    "pins::pinnedTabsLeaveLive (Ardia pins/pinHomes.ts)",
    "2.13"
);

// shortcut: stub for splits::splitTabIds (Ardia splits/splits.ts:61), replace with the real call when slice 2.14 merges
pub fn split_tab_ids(split: &SplitView) -> Vec<String> {
    todo!("slice 2.14: splitTabIds")
}

// shortcut: stub for settings::withToolbarKeptWhileScrolling (Ardia settings/toolbarScrollDefault.ts), replace with the real call when slice 2.7 merges
pub fn with_toolbar_kept_while_scrolling(
    settings: Option<&crate::settings::Settings>,
    meta: &crate::settings::SettingsMeta,
) -> (
    Option<crate::settings::Settings>,
    crate::settings::SettingsMeta,
) {
    todo!("slice 2.7: withToolbarKeptWhileScrolling")
}

// shortcut: stub for settings::parseSettingsMeta (Ardia settings/settings.ts), replace with the real call when slice 2.7 merges
pub fn parse_settings_meta(
    meta: Option<&crate::settings::SettingsMeta>,
) -> crate::settings::SettingsMeta {
    todo!("slice 2.7: parseSettingsMeta")
}

// shortcut: stub for settings::parseSettings (Ardia settings/settings.ts), replace with the real call when slice 2.7 merges
pub fn parse_settings(settings: Option<&crate::settings::Settings>) -> crate::settings::Settings {
    todo!("slice 2.7: parseSettings")
}

// shortcut: stub for pins::notePinChoices (Ardia pins/pinChoices.ts), replace with the real call when slice 2.13 merges
pub fn note_pin_choices(choices: &HashMap<String, HashMap<String, Value>>) {
    todo!("slice 2.13: notePinChoices")
}

// shortcut: stub for pins::pinHome (Ardia pins/pinHomes.ts), replace with the real call when slice 2.13 merges
pub fn pin_home(tab: &Tab, pinned: bool) -> Option<String> {
    todo!("slice 2.13: pinHome")
}

// shortcut: stub for groups::orderSections (Ardia groups/groups.ts:54), replace with the real call when slice 2.23 merges
pub fn order_sections(
    ids: Vec<String>,
    tabs: &HashMap<String, Tab>,
    groups: &HashMap<String, TabGroup>,
) -> Vec<String> {
    todo!("slice 2.23: orderSections")
}

// shortcut: stub for splits::sanitizeSplits (Ardia splits/splits.ts), replace with the real call when slice 2.14 merges
pub fn sanitize_splits(
    splits: &HashMap<String, SplitView>,
    tabs: &HashMap<String, Tab>,
) -> HashMap<String, SplitView> {
    todo!("slice 2.14: sanitizeSplits")
}

// shortcut: stub for profiles::engineProfiles (Ardia profiles/engineProfiles.ts), replace with the real call when slice 2.12 merges
pub fn engine_profiles(profiles: &HashMap<String, Profile>) -> Vec<String> {
    todo!("slice 2.12: engineProfiles")
}

// shortcut: stub for bookmarks::ensureRoots (Ardia bookmarks/bookmarks.ts), replace with the real call when slice 2.16 merges
pub fn ensure_roots(bookmarks: Bookmarks, engine: &str) -> Bookmarks {
    todo!("slice 2.16: ensureRoots")
}

// shortcut: stub for pins::keptDeletedGroups (Ardia pins/closedTabs.ts), replace with the real call when slice 2.13 merges
pub fn kept_deleted_groups(groups: Vec<ClosedGroup>) -> Vec<ClosedGroup> {
    todo!("slice 2.13: keptDeletedGroups")
}

// shortcut: stub for pins::keptCleanedTabs (Ardia pins/closedTabs.ts), replace with the real call when slice 2.13 merges
pub fn kept_cleaned_tabs(tabs: Vec<ClosedTab>) -> Vec<ClosedTab> {
    todo!("slice 2.13: keptCleanedTabs")
}

// shortcut: stub for tabs::apply(s, activated(s, tabId)) (Ardia tabs/tabs.ts), replace with the real call when slice 2.8 merges
pub fn activate_tab(s: BrowserState, tab_id: &str) -> BrowserState {
    todo!("slice 2.8: apply(s, activated(s, tabId))")
}

// shortcut: stub for tabs::forgetAllLive (Ardia tabs/tabLive.ts), replace with the real call when slice 2.8 merges
pub fn forget_all_live() {
    todo!("slice 2.8: forgetAllLive")
}

// shortcut: stub for profiles::DEFAULT_PROFILE (Ardia profiles/profiles.ts), replace with the real call when slice 2.12 merges
pub fn default_profile() -> Profile {
    todo!("slice 2.12: DEFAULT_PROFILE")
}

// shortcut: stub for windows::focusedWindowOf (Ardia windows/ui.ts), replace with the real call when slice 2.9 merges
pub fn focused_window_of(s: &BrowserState) -> Option<String> {
    todo!("slice 2.9: focusedWindowOf")
}

// shortcut: stub for windows::setFocusedWindow (Ardia windows/ui.ts), replace with the real call when slice 2.9 merges
pub fn set_focused_window(s: &mut BrowserState, window_id: &str) {
    todo!("slice 2.9: setFocusedWindow")
}

// shortcut: stub for tabs::activate (Ardia tabs/tabsSlice.ts), replace with the real call when slice 2.8 merges
pub fn activate(s: &mut BrowserState, tab_id: &str) {
    todo!("slice 2.8: activate")
}

// shortcut: stub for tabs::isSiteMuted (Ardia tabs/mute.ts), replace with the real call when slice 2.8 merges
pub fn is_site_muted(s: &BrowserState, tab: &Tab) -> bool {
    todo!("slice 2.8: isSiteMuted")
}

// shortcut: stub for tabs::setMuted (Ardia tabs/mute.ts), replace with the real call when slice 2.8 merges
pub fn set_muted(s: &mut BrowserState, tab_ids: &[String], muted: bool) {
    todo!("slice 2.8: setMuted")
}

// shortcut: stub for tabs::setSiteMuted (Ardia tabs/mute.ts), replace with the real call when slice 2.8 merges
pub fn set_site_muted(s: &mut BrowserState, tab_id: &str, muted: bool) {
    todo!("slice 2.8: setSiteMuted")
}

// shortcut: stub for windows::createWindow (Ardia windows/windows.ts), replace with the real call when slice 2.9 merges
pub fn create_window(
    s: &mut BrowserState,
    options: Option<&crate::windows::CreateWindowOptions>,
) -> String {
    todo!("slice 2.9: createWindow")
}

// shortcut: stub for tabs::newTab (Ardia tabs/tabsSlice.ts), replace with the real call when slice 2.8 merges
pub fn new_tab(s: &mut BrowserState, window_id: &str, url: &str) {
    todo!("slice 2.8: newTab")
}

// shortcut: stub for profiles::switchProfile (Ardia profiles/profiles.ts), replace with the real call when slice 2.12 merges
pub fn switch_profile(s: &mut BrowserState, window_id: &str, profile_id: &str) {
    todo!("slice 2.12: switchProfile")
}

// shortcut: stub for groups::moveGroupToWindow (Ardia groups/groups.ts), replace with the real call when slice 2.23 merges
pub fn move_group_to_window(
    s: &mut BrowserState,
    group_id: &str,
    target: Option<&str>,
) -> Option<String> {
    todo!("slice 2.23: moveGroupToWindow")
}

// shortcut: stub for tabs::moveTabsToWindow (Ardia tabs/tabsSlice.ts), replace with the real call when slice 2.8 merges
pub fn move_tabs_to_window(
    s: &mut BrowserState,
    tab_ids: &[String],
    target: Option<&str>,
) -> Option<String> {
    todo!("slice 2.8: moveTabsToWindow")
}

// shortcut: stub for windows::pageToProfile (Ardia windows/profilePager.ts), replace with the real call when slice 2.9 merges
pub fn page_to_profile(s: &mut BrowserState, window_id: &str, profile_id: &str) {
    todo!("slice 2.9: pageToProfile")
}

// shortcut: stub for platform::stepPager (Ardia platform/pager.ts), replace with the real call when slice 2.3 adds a pager call
pub fn step_pager(window_id: &str, delta: i64, profile_order: &[String]) -> bool {
    todo!("slice 2.3: stepPager")
}
