// Ria original, no Ardia source.

//! The sibling helpers that the `splits` folder calls and that their own slices have not ported yet.
//!
//! Each function names its Ardia source. A sibling slice replaces the body of its function and deletes it from here;
//! until then a replay that reaches one stops at its `todo!` and is listed in `rust/replay/splits/PENDING.md`.
//! This file holds no logic of its own.

use std::collections::{HashMap, HashSet};

use super::splits::Work;
use crate::browser::{BrowserWindow, Tab, TabGroup};
use crate::kernel::AppState;

/// The `TabPlacement` that `gather` passes to `placing` (`pins/placing.ts`).
#[allow(dead_code)] // shortcut: read by the `placing` body, slice 2.x pins
pub struct Placement {
    pub pinned: bool,
    pub row: bool,
    pub before_id: String,
}

/// The `NewTabOptions` that `openSplitPane` passes to `withNewTab`; `background` is always true.
#[allow(dead_code)] // shortcut: read by the `withNewTab` body, tabs slice
pub struct NewTab {
    pub url: Option<String>,
    pub adopt_id: Option<String>,
    pub profile_id: String,
    pub opener_id: Option<String>,
}

/// `newId` (`browser/model.ts:11`).
pub fn new_id(prefix: &str) -> String {
    todo!("slice 2.5 browser: newId({prefix})")
}

/// `inPinnedContainer` (`browser/model.ts:101`).
pub fn in_pinned_container(
    _tabs: &HashMap<String, Tab>,
    _groups: &HashMap<String, TabGroup>,
    id: &str,
) -> bool {
    todo!("slice 2.5 browser: inPinnedContainer({id})")
}

/// `tabLayoutOf` (`browser/model.ts:161`): "sidebar" or "top".
pub fn tab_layout_of(_app: &AppState, window_id: &str) -> &'static str {
    todo!("slice 2.5 browser: tabLayoutOf({window_id})")
}

/// `choose(s, { tabLayout })` (`settings/settings.ts:282`).
pub fn choose_tab_layout(_app: &mut AppState, layout: &str) {
    todo!("settings: choose({{ tabLayout: {layout} }})")
}

/// `placing` (`pins/placing.ts:21`).
pub fn placing(_w: &mut Work, ids: &[String], _p: Placement) {
    todo!("pins: placing({ids:?})")
}

/// `withNewTab` (`tabs/tabs.ts:281`): adds a background tab and returns its id.
pub fn with_new_tab(_w: &mut Work, window_id: &str, _o: NewTab) -> String {
    todo!("tabs: withNewTab({window_id})")
}

/// `removeTabs` (`tabs/tabs.ts:130`).
pub fn remove_tabs(_w: &mut Work, ids: &[String], _record: bool) {
    todo!("tabs: removeTabs({ids:?})")
}

/// `leaveGroups` (`groups/groups.ts:94`).
pub fn leave_groups(
    _groups: &HashMap<String, TabGroup>,
    ids: &HashSet<String>,
    _keep: Option<&HashSet<String>>,
) -> HashMap<String, TabGroup> {
    todo!("groups: leaveGroups({ids:?})")
}

/// `syncGroupOrder` (`groups/groups.ts:41`).
pub fn sync_group_order(
    _groups: &HashMap<String, TabGroup>,
    window: &BrowserWindow,
) -> HashMap<String, TabGroup> {
    todo!(
        "groups: syncGroupOrder({})",
        super::splits::window_id(window)
    )
}

/// `innermostGroupOf` (`groups/groupNesting.ts`): the id of the group.
pub fn innermost_group_of(_groups: &HashMap<String, TabGroup>, id: &str) -> Option<String> {
    todo!("groups: innermostGroupOf({id})")
}

/// `noteSplit` (`telemetry/usage.ts`).
pub fn note_split(kind: &str) {
    todo!("telemetry: noteSplit({kind})")
}

/// `returnFocusToPage` (`sidebar/actions.ts`).
pub fn return_focus_to_page(window_id: &str) {
    todo!("sidebar: returnFocusToPage({window_id})")
}

/// `showMenu` (`platform/menus.ts`) with the split menu: the platform call is not in `PlatformCall` yet.
pub fn show_split_menu(tab_id: &str) {
    todo!("platform: showMenu for showSplitMenu({tab_id})")
}
