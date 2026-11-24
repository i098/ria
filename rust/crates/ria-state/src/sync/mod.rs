// Ported from ardia apps/browser/src/state/sync/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `sync` (`apps/browser/src/state/sync`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.31 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SyncedDevice {
    pub id: String,
    pub name: String,
    pub kind: DeviceKind,
    #[serde(rename = "receivesTabs")]
    pub receives_tabs: bool,
    #[serde(rename = "lastSeen")]
    pub last_seen: Option<Number>,
    pub current: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DeviceKind {
    #[default]
    #[serde(rename = "mac")]
    Mac,
    #[serde(rename = "phone")]
    Phone,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProfileLink {
    #[serde(rename = "syncId")]
    pub sync_id: String,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retire: Option<bool>,
    #[serde(
        rename = "themeAdopting",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub theme_adopting: Option<bool>,
    #[serde(
        rename = "tabsRetire",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub tabs_retire: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RemoteProfile {
    #[serde(rename = "syncId")]
    pub sync_id: String,
    pub name: String,
    pub color: RemoteProfileColor,
    pub icon: Option<String>,
    #[serde(
        rename = "sharesWith",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub shares_with: Option<RemoteProfileSharesWith>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiting: Option<bool>,
    #[serde(
        rename = "sharedByOthers",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub shared_by_others: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RemoteProfileColor {
    #[default]
    #[serde(rename = "plum")]
    Plum,
    #[serde(rename = "blue")]
    Blue,
    #[serde(rename = "purple")]
    Purple,
    #[serde(rename = "pink")]
    Pink,
    #[serde(rename = "red")]
    Red,
    #[serde(rename = "orange")]
    Orange,
    #[serde(rename = "yellow")]
    Yellow,
    #[serde(rename = "green")]
    Green,
    #[serde(rename = "neutral")]
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RemoteProfileSharesWith {
    pub name: String,
    pub added: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RemoteTabs {
    #[serde(rename = "deviceId")]
    pub device_id: String,
    pub name: String,
    pub kind: DeviceKind,
    pub tabs: Vec<RemoteTabsTabsItem>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RemoteTabsTabsItem {
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SyncStatus {
    #[default]
    #[serde(rename = "updated")]
    Updated,
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "offline")]
    Offline,
    #[serde(rename = "starting")]
    Starting,
    #[serde(rename = "updating")]
    Updating,
    #[serde(rename = "stalled")]
    Stalled,
    #[serde(rename = "locked")]
    Locked,
    #[serde(rename = "reset")]
    Reset,
    #[serde(rename = "unavailable")]
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateTypes {
    pub tabs: bool,
    pub settings: bool,
    pub bookmarks: bool,
    pub history: bool,
    pub extensions: bool,
    pub pinned: bool,
    pub passwords: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SyncDoc {
    pub v: Number,
    pub enabled: bool,
    pub folder: Option<String>,
    #[serde(rename = "deviceId")]
    pub device_id: Option<String>,
    pub clock: Option<String>,
    pub types: StateTypes,
    pub profiles: std::collections::HashMap<String, ProfileLink>,
    pub scopes: std::collections::HashMap<String, serde_json::Value>,
    #[serde(rename = "lastSyncedAt")]
    pub last_synced_at: Option<Number>,
    #[serde(rename = "setupAt")]
    pub setup_at: Option<Number>,
    pub joining: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seqs: Option<std::collections::HashMap<String, Number>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sends: Option<std::collections::HashMap<String, SendLedger>>,
    #[serde(rename = "tabCloses", default, skip_serializing_if = "Option::is_none")]
    pub tab_closes: Option<std::collections::HashMap<String, Vec<String>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub removing: Option<SyncDocRemoving>,
    #[serde(
        rename = "removingDevices",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub removing_devices: Option<SyncDocRemovingDevices>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SendLedger {
    pub outgoing: std::collections::HashMap<String, serde_json::Value>,
    pub received: std::collections::HashMap<String, Number>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SyncDocRemoving {
    Other0(Vec<String>),
    Other1(std::collections::HashMap<String, ProfileRemoval>),
}

impl Default for SyncDocRemoving {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProfileRemoval {
    pub keys: std::collections::HashMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returned: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SyncDocRemovingDevices {
    Other0(Vec<String>),
    Other1(std::collections::HashMap<String, serde_json::Value>),
}

impl Default for SyncDocRemovingDevices {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProfileRecord {
    pub n: String,
    pub c: ProfileRecordC,
    pub i: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub d: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub o: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub th: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ProfileRecordC {
    #[default]
    #[serde(rename = "plum")]
    Plum,
    #[serde(rename = "blue")]
    Blue,
    #[serde(rename = "purple")]
    Purple,
    #[serde(rename = "pink")]
    Pink,
    #[serde(rename = "red")]
    Red,
    #[serde(rename = "orange")]
    Orange,
    #[serde(rename = "yellow")]
    Yellow,
    #[serde(rename = "green")]
    Green,
    #[serde(rename = "neutral")]
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CommitPinnedEditPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profiles: Option<std::collections::HashMap<String, crate::browser::Profile>>,
    #[serde(
        rename = "profileOrder",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub profile_order: Option<Vec<String>>,
    #[serde(
        rename = "orphanedProfileData",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub orphaned_profile_data: Option<Vec<String>>,
    #[serde(
        rename = "createProfile",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub create_profile: Option<serde_json::Value>,
    #[serde(
        rename = "updateProfile",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub update_profile: Option<serde_json::Value>,
    #[serde(
        rename = "setProfileLook",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub set_profile_look: Option<serde_json::Value>,
    #[serde(
        rename = "deleteProfile",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub delete_profile: Option<serde_json::Value>,
    #[serde(
        rename = "reorderProfiles",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub reorder_profiles: Option<serde_json::Value>,
    #[serde(
        rename = "setDefaultProfile",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub set_default_profile: Option<serde_json::Value>,
    #[serde(
        rename = "profileDataDeleted",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub profile_data_deleted: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows: Option<std::collections::HashMap<String, crate::browser::BrowserWindow>>,
    #[serde(
        rename = "windowOrder",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub window_order: Option<Vec<String>>,
    #[serde(
        rename = "closedWindows",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub closed_windows: Option<Vec<crate::browser::ClosedWindow>>,
    #[serde(
        rename = "parkedPins",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub parked_pins: Option<std::collections::HashMap<String, crate::browser::ParkedPins>>,
    #[serde(
        rename = "createWindow",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub create_window: Option<serde_json::Value>,
    #[serde(
        rename = "closeWindow",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub close_window: Option<serde_json::Value>,
    #[serde(
        rename = "dropStaleLastWindowMark",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub drop_stale_last_window_mark: Option<serde_json::Value>,
    #[serde(
        rename = "setWindowFrame",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub set_window_frame: Option<serde_json::Value>,
    #[serde(
        rename = "setWindowMaximized",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub set_window_maximized: Option<serde_json::Value>,
    #[serde(
        rename = "setWindowFullScreen",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub set_window_full_screen: Option<serde_json::Value>,
    #[serde(
        rename = "toggleSidebar",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub toggle_sidebar: Option<serde_json::Value>,
    #[serde(
        rename = "switchProfile",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub switch_profile: Option<serde_json::Value>,
    #[serde(
        rename = "moveTabsToWindow",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub move_tabs_to_window: Option<serde_json::Value>,
    #[serde(
        rename = "returnRefusedMove",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub return_refused_move: Option<serde_json::Value>,
    #[serde(
        rename = "moveTabToProfile",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub move_tab_to_profile: Option<serde_json::Value>,
    #[serde(
        rename = "dropTabsInProfile",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub drop_tabs_in_profile: Option<serde_json::Value>,
    #[serde(
        rename = "moveGroupToWindow",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub move_group_to_window: Option<serde_json::Value>,
    #[serde(
        rename = "moveGroupToProfile",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub move_group_to_profile: Option<serde_json::Value>,
    #[serde(
        rename = "mergeAllWindows",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub merge_all_windows: Option<serde_json::Value>,
    #[serde(
        rename = "reopenClosed",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub reopen_closed: Option<serde_json::Value>,
    #[serde(
        rename = "reopenClosedTab",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub reopen_closed_tab: Option<serde_json::Value>,
    #[serde(
        rename = "reopenClosedWindow",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub reopen_closed_window: Option<serde_json::Value>,
    #[serde(
        rename = "restoreClosed",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub restore_closed: Option<serde_json::Value>,
    #[serde(
        rename = "clearRecentlyClosed",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub clear_recently_closed: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tabs: Option<std::collections::HashMap<String, crate::browser::Tab>>,
    #[serde(
        rename = "closedTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub closed_tabs: Option<Vec<crate::browser::ClosedTab>>,
    #[serde(rename = "newTab", default, skip_serializing_if = "Option::is_none")]
    pub new_tab: Option<serde_json::Value>,
    #[serde(rename = "closeTab", default, skip_serializing_if = "Option::is_none")]
    pub close_tab: Option<serde_json::Value>,
    #[serde(rename = "closeTabs", default, skip_serializing_if = "Option::is_none")]
    pub close_tabs: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activate: Option<serde_json::Value>,
    #[serde(
        rename = "handOffPin",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub hand_off_pin: Option<serde_json::Value>,
    #[serde(
        rename = "activateIndex",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub activate_index: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cycle: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub navigate: Option<serde_json::Value>,
    #[serde(rename = "updateTab", default, skip_serializing_if = "Option::is_none")]
    pub update_tab: Option<serde_json::Value>,
    #[serde(
        rename = "updateLive",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub update_live: Option<serde_json::Value>,
    #[serde(
        rename = "duplicateTab",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub duplicate_tab: Option<serde_json::Value>,
    #[serde(rename = "togglePin", default, skip_serializing_if = "Option::is_none")]
    pub toggle_pin: Option<serde_json::Value>,
    #[serde(rename = "moveTab", default, skip_serializing_if = "Option::is_none")]
    pub move_tab: Option<serde_json::Value>,
    #[serde(rename = "moveTabBy", default, skip_serializing_if = "Option::is_none")]
    pub move_tab_by: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groups: Option<std::collections::HashMap<String, crate::browser::TabGroup>>,
    #[serde(
        rename = "createGroup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub create_group: Option<serde_json::Value>,
    #[serde(
        rename = "updateGroup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub update_group: Option<serde_json::Value>,
    #[serde(
        rename = "addTabsToGroup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub add_tabs_to_group: Option<serde_json::Value>,
    #[serde(
        rename = "removeTabsFromGroup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub remove_tabs_from_group: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ungroup: Option<serde_json::Value>,
    #[serde(
        rename = "newNestedGroup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub new_nested_group: Option<serde_json::Value>,
    #[serde(
        rename = "closeGroup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub close_group: Option<serde_json::Value>,
    #[serde(
        rename = "deleteGroup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub delete_group: Option<serde_json::Value>,
    #[serde(
        rename = "closedGroups",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub closed_groups: Option<Vec<crate::browser::ClosedGroup>>,
    #[serde(
        rename = "deletedGroups",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub deleted_groups: Option<Vec<crate::browser::ClosedGroup>>,
    #[serde(rename = "groupTabs", default, skip_serializing_if = "Option::is_none")]
    pub group_tabs: Option<serde_json::Value>,
    #[serde(rename = "groupOnto", default, skip_serializing_if = "Option::is_none")]
    pub group_onto: Option<serde_json::Value>,
    #[serde(rename = "moveGroup", default, skip_serializing_if = "Option::is_none")]
    pub move_group: Option<serde_json::Value>,
    #[serde(
        rename = "duplicateGroup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub duplicate_group: Option<serde_json::Value>,
    #[serde(
        rename = "newTabInGroup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub new_tab_in_group: Option<serde_json::Value>,
    #[serde(
        rename = "moveGroupToBookmarksBar",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub move_group_to_bookmarks_bar: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<std::collections::HashMap<String, Vec<String>>>,
    #[serde(
        rename = "selectionAnchors",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub selection_anchors: Option<std::collections::HashMap<String, crate::drag::SelectionAnchor>>,
    #[serde(
        rename = "pinChoices",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub pin_choices: Option<
        std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>>,
    >,
    #[serde(rename = "pinSync", default, skip_serializing_if = "Option::is_none")]
    pub pin_sync: Option<
        std::collections::HashMap<
            String,
            std::collections::HashMap<String, crate::pins::PinSyncEntry>,
        >,
    >,
    #[serde(
        rename = "topAppsMerged",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub top_apps_merged: Option<String>,
    #[serde(
        rename = "setSelection",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub set_selection: Option<serde_json::Value>,
    #[serde(rename = "clickTab", default, skip_serializing_if = "Option::is_none")]
    pub click_tab: Option<serde_json::Value>,
    #[serde(rename = "placeTabs", default, skip_serializing_if = "Option::is_none")]
    pub place_tabs: Option<serde_json::Value>,
    #[serde(rename = "dropTabs", default, skip_serializing_if = "Option::is_none")]
    pub drop_tabs: Option<serde_json::Value>,
    #[serde(rename = "pinTabs", default, skip_serializing_if = "Option::is_none")]
    pub pin_tabs: Option<serde_json::Value>,
    #[serde(
        rename = "returnToPinnedUrl",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub return_to_pinned_url: Option<serde_json::Value>,
    #[serde(
        rename = "setPinnedUrl",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub set_pinned_url: Option<serde_json::Value>,
    #[serde(
        rename = "privateSiteMutes",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub private_site_mutes:
        Option<std::collections::HashMap<String, std::collections::HashMap<String, bool>>>,
    #[serde(rename = "setMuted", default, skip_serializing_if = "Option::is_none")]
    pub set_muted: Option<serde_json::Value>,
    #[serde(
        rename = "setSiteMuted",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub set_site_muted: Option<serde_json::Value>,
    #[serde(
        rename = "cleanedTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub cleaned_tabs: Option<Vec<crate::browser::ClosedTab>>,
    #[serde(
        rename = "privateCleanedTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub private_cleaned_tabs:
        Option<std::collections::HashMap<String, Vec<crate::browser::ClosedTab>>>,
    #[serde(
        rename = "cleanUpTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub clean_up_tabs: Option<serde_json::Value>,
    #[serde(
        rename = "restoreCleaned",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub restore_cleaned: Option<serde_json::Value>,
    #[serde(
        rename = "clearUnpinnedTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub clear_unpinned_tabs: Option<serde_json::Value>,
    #[serde(
        rename = "undoClearTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub undo_clear_tabs: Option<serde_json::Value>,
    #[serde(
        rename = "removeClosedPin",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub remove_closed_pin: Option<serde_json::Value>,
    #[serde(
        rename = "closeAbandonedNewTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub close_abandoned_new_tabs: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub splits: Option<std::collections::HashMap<String, crate::browser::SplitView>>,
    #[serde(
        rename = "createSplit",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub create_split: Option<serde_json::Value>,
    #[serde(
        rename = "openSplitPane",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub open_split_pane: Option<serde_json::Value>,
    #[serde(
        rename = "replaceSplitPane",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub replace_split_pane: Option<serde_json::Value>,
    #[serde(
        rename = "removeTabFromSplit",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub remove_tab_from_split: Option<serde_json::Value>,
    #[serde(
        rename = "separateSplit",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub separate_split: Option<serde_json::Value>,
    #[serde(
        rename = "updateSplit",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub update_split: Option<serde_json::Value>,
    #[serde(rename = "flipSplit", default, skip_serializing_if = "Option::is_none")]
    pub flip_split: Option<serde_json::Value>,
    #[serde(
        rename = "toggleSplitOrientation",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub toggle_split_orientation: Option<serde_json::Value>,
    #[serde(rename = "movePane", default, skip_serializing_if = "Option::is_none")]
    pub move_pane: Option<serde_json::Value>,
    #[serde(rename = "focusPane", default, skip_serializing_if = "Option::is_none")]
    pub focus_pane: Option<serde_json::Value>,
    #[serde(
        rename = "toggleTabLayout",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub toggle_tab_layout: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history: Option<std::collections::HashMap<String, crate::history::HistoryView>>,
    #[serde(
        rename = "removeHistory",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub remove_history: Option<serde_json::Value>,
    #[serde(
        rename = "clearHistory",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub clear_history: Option<serde_json::Value>,
    #[serde(
        rename = "importHistory",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub import_history: Option<serde_json::Value>,
    #[serde(
        rename = "importCleanedTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub import_cleaned_tabs: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bookmarks: Option<crate::browser::Bookmarks>,
    #[serde(
        rename = "bookmarksReady",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bookmarks_ready: Option<std::collections::HashMap<String, bool>>,
    #[serde(
        rename = "addBookmark",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub add_bookmark: Option<serde_json::Value>,
    #[serde(
        rename = "addBookmarkFolder",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub add_bookmark_folder: Option<serde_json::Value>,
    #[serde(
        rename = "updateBookmark",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub update_bookmark: Option<serde_json::Value>,
    #[serde(
        rename = "moveBookmark",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub move_bookmark: Option<serde_json::Value>,
    #[serde(
        rename = "moveBookmarks",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub move_bookmarks: Option<serde_json::Value>,
    #[serde(
        rename = "removeBookmark",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub remove_bookmark: Option<serde_json::Value>,
    #[serde(
        rename = "toggleBookmark",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub toggle_bookmark: Option<serde_json::Value>,
    #[serde(
        rename = "addBookmarkTree",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub add_bookmark_tree: Option<serde_json::Value>,
    #[serde(
        rename = "removeBookmarks",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub remove_bookmarks: Option<serde_json::Value>,
    #[serde(
        rename = "restoreBookmarks",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub restore_bookmarks: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<crate::settings::Settings>,
    #[serde(
        rename = "settingsMeta",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub settings_meta: Option<crate::settings::SettingsMeta>,
    #[serde(
        rename = "updateSettings",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub update_settings: Option<serde_json::Value>,
    #[serde(
        rename = "setSearchEngine",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub set_search_engine: Option<serde_json::Value>,
    #[serde(
        rename = "addCustomEngine",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub add_custom_engine: Option<serde_json::Value>,
    #[serde(
        rename = "updateCustomEngine",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub update_custom_engine: Option<serde_json::Value>,
    #[serde(
        rename = "removeCustomEngine",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub remove_custom_engine: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui: Option<crate::automation::SnapshotUi>,
    #[serde(rename = "windowUi", default, skip_serializing_if = "Option::is_none")]
    pub window_ui: Option<std::collections::HashMap<String, crate::browser::WindowUi>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub find: Option<std::collections::HashMap<String, crate::browser::FindState>>,
    #[serde(
        rename = "setFocusedWindow",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub set_focused_window: Option<serde_json::Value>,
    #[serde(
        rename = "setAppDark",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub set_app_dark: Option<serde_json::Value>,
    #[serde(rename = "openPanel", default, skip_serializing_if = "Option::is_none")]
    pub open_panel: Option<serde_json::Value>,
    #[serde(
        rename = "closePanel",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub close_panel: Option<serde_json::Value>,
    #[serde(
        rename = "setDownloadsOpen",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub set_downloads_open: Option<serde_json::Value>,
    #[serde(rename = "setFind", default, skip_serializing_if = "Option::is_none")]
    pub set_find: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloads: Option<Vec<serde_json::Value>>,
    #[serde(
        rename = "upsertDownload",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub upsert_download: Option<serde_json::Value>,
    #[serde(
        rename = "removeDownload",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub remove_download: Option<serde_json::Value>,
    #[serde(
        rename = "clearDownloads",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub clear_downloads: Option<serde_json::Value>,
    #[serde(
        rename = "forgetDownloads",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub forget_downloads: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peeks: Option<std::collections::HashMap<String, crate::peek::Peek>>,
    #[serde(rename = "openPeek", default, skip_serializing_if = "Option::is_none")]
    pub open_peek: Option<serde_json::Value>,
    #[serde(rename = "closePeek", default, skip_serializing_if = "Option::is_none")]
    pub close_peek: Option<serde_json::Value>,
    #[serde(rename = "peekToTab", default, skip_serializing_if = "Option::is_none")]
    pub peek_to_tab: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hydrate: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CommitPinnedEditParked {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tabs: Option<Vec<crate::browser::ParkedPinsTabsItem>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<crate::browser::ParkedPinsGroupsItem>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub splits: Option<Vec<crate::browser::ClosedWindowSplitsItem>>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ReceivedSend {
    pub key: String,
    pub send: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum OpenTabsMode {
    #[default]
    #[serde(rename = "on")]
    On,
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "retire")]
    Retire,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OpenTabCloses {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetTypeSyncedTypeArg {
    #[default]
    #[serde(rename = "tabs")]
    Tabs,
    #[serde(rename = "settings")]
    Settings,
    #[serde(rename = "bookmarks")]
    Bookmarks,
    #[serde(rename = "history")]
    History,
    #[serde(rename = "extensions")]
    Extensions,
    #[serde(rename = "pinned")]
    Pinned,
    #[serde(rename = "passwords")]
    Passwords,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StopSyncV0 {
    #[serde(
        rename = "deleteData",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub delete_data: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TurnOnSyncHow {
    #[default]
    #[serde(rename = "picked")]
    Picked,
    #[serde(rename = "kept")]
    Kept,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub devices: Vec<SyncedDevice>,
    pub enabled: bool,
    pub error: Option<String>,
    pub folder: Option<String>,
    pub joining: bool,
    #[serde(rename = "lastSyncedAt")]
    pub last_synced_at: Option<Number>,
    pub pending: Number,
    pub profiles: std::collections::HashMap<String, ProfileLink>,
    #[serde(rename = "remoteProfiles")]
    pub remote_profiles: Vec<RemoteProfile>,
    #[serde(rename = "remoteTabs")]
    pub remote_tabs: std::collections::HashMap<String, Vec<RemoteTabs>>,
    pub status: SyncStatus,
    pub types: StateTypes,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `addLinkedProfile` (an exported function that writes a store).
    #[serde(rename = "addLinkedProfile")]
    AddLinkedProfile {
        doc: SyncDoc,
        #[serde(rename = "syncId")]
        sync_id: String,
        record: ProfileRecord,
        #[serde(rename = "shareWith")]
        share_with: Option<String>,
    },
    /// `addRemoteProfile` (an exported function that writes a store).
    #[serde(rename = "addRemoteProfile")]
    AddRemoteProfile {
        #[serde(rename = "syncId")]
        sync_id: String,
    },
    /// `adoptProfileTheme` (an exported function that writes a store).
    #[serde(rename = "adoptProfileTheme")]
    AdoptProfileTheme {
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    /// `appAdapters` (an exported function that writes a store).
    #[serde(rename = "appAdapters")]
    AppAdapters { doc: SyncDoc, retiring: bool },
    /// `applyOpenTabs` (an exported function that writes a store).
    #[serde(rename = "applyOpenTabs")]
    ApplyOpenTabs {
        #[serde(rename = "profileId")]
        profile_id: String,
        visible: std::collections::HashMap<String, serde_json::Value>,
        changed: Vec<String>,
        closed: Vec<String>,
        seen: std::collections::HashMap<String, String>,
    },
    /// `applyPinned` (an exported function that writes a store). The callback parameters `base` are not in the JSON.
    #[serde(rename = "applyPinned")]
    ApplyPinned {
        #[serde(rename = "profileId")]
        profile_id: String,
        visible: std::collections::HashMap<String, serde_json::Value>,
        changed: Vec<String>,
        records: serde_json::Value,
    },
    /// `bookmarksAdapter` (an exported function that writes a store).
    #[serde(rename = "bookmarksAdapter")]
    BookmarksAdapter {
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    /// `chooseSyncFolder` (an exported function that writes a store).
    #[serde(rename = "chooseSyncFolder")]
    ChooseSyncFolder,
    /// `commitPinnedEdit` (an exported function that writes a store).
    #[serde(rename = "commitPinnedEdit")]
    CommitPinnedEdit {
        s: crate::browser::BrowserState,
        #[serde(rename = "profileId")]
        profile_id: String,
        patch: CommitPinnedEditPatch,
        parked: Option<CommitPinnedEditParked>,
    },
    /// `deliverSentTabs` (an exported function that writes a store).
    #[serde(rename = "deliverSentTabs")]
    DeliverSentTabs {
        doc: SyncDoc,
        #[serde(rename = "profileId")]
        profile_id: String,
        arrived: Vec<ReceivedSend>,
    },
    /// `enterRecoveryPhrase` (an exported function that writes a store).
    #[serde(rename = "enterRecoveryPhrase")]
    EnterRecoveryPhrase { folder: String, text: String },
    /// `extractPinned` (an exported function that writes a store). The callback parameters `base`, `queued`, `stamp` are not in the JSON.
    #[serde(rename = "extractPinned")]
    ExtractPinned {
        #[serde(rename = "profileId")]
        profile_id: String,
        now: Number,
    },
    /// `groupLooksAdapter` (an exported function that writes a store).
    #[serde(rename = "groupLooksAdapter")]
    GroupLooksAdapter {
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    /// `historyAdapter` (an exported function that writes a store).
    #[serde(rename = "historyAdapter")]
    HistoryAdapter {
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    /// `linkProfiles` (an exported function that writes a store).
    #[serde(rename = "linkProfiles")]
    LinkProfiles {
        doc: SyncDoc,
        app: serde_json::Value,
    },
    /// `markOpenTabsSynced` (an exported function that writes a store).
    #[serde(rename = "markOpenTabsSynced")]
    MarkOpenTabsSynced {
        doc: SyncDoc,
        #[serde(rename = "profileId")]
        profile_id: String,
        on: bool,
    },
    /// `moveSyncFolder` (an exported function that writes a store).
    #[serde(rename = "moveSyncFolder")]
    MoveSyncFolder { to: String },
    /// `openSyncedTab` (an exported function that writes a store).
    #[serde(rename = "openSyncedTab")]
    OpenSyncedTab {
        choice: String,
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    /// `openTabsAdapters` (an exported function that writes a store). The callback parameters `flag` are not in the JSON.
    #[serde(rename = "openTabsAdapters")]
    OpenTabsAdapters {
        #[serde(rename = "profileId")]
        profile_id: String,
        state: serde_json::Value,
        mode: OpenTabsMode,
        closes: OpenTabCloses,
    },
    /// `pairTheme` (an exported function that writes a store).
    #[serde(rename = "pairTheme")]
    PairTheme {
        app: serde_json::Value,
        #[serde(rename = "profileId")]
        profile_id: String,
        link: ProfileLink,
        #[serde(default)]
        record: Option<ProfileRecord>,
    },
    /// `pinnedAdapter` (an exported function that writes a store).
    #[serde(rename = "pinnedAdapter")]
    PinnedAdapter {
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    /// `pinnedAreaAdapters` (an exported function that writes a store).
    #[serde(rename = "pinnedAreaAdapters")]
    PinnedAreaAdapters {
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    /// `pinnedHomesAdapter` (an exported function that writes a store).
    #[serde(rename = "pinnedHomesAdapter")]
    PinnedHomesAdapter {
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    /// `pinnedOrderAdapter` (an exported function that writes a store).
    #[serde(rename = "pinnedOrderAdapter")]
    PinnedOrderAdapter {
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    /// `pinnedRowsAdapter` (an exported function that writes a store).
    #[serde(rename = "pinnedRowsAdapter")]
    PinnedRowsAdapter {
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    /// `profileAdapters` (an exported function that writes a store).
    #[serde(rename = "profileAdapters")]
    ProfileAdapters {
        doc: SyncDoc,
        #[serde(rename = "profileId")]
        profile_id: String,
        retiring: bool,
        #[serde(rename = "forcePasswords")]
        force_passwords: bool,
    },
    /// `profileOpenTabsAdapter` (an exported function that writes a store). The callback parameters `links`, `removed` are not in the JSON.
    #[serde(rename = "profileOpenTabsAdapter")]
    ProfileOpenTabsAdapter,
    /// `profilesAdapter` (an exported function that writes a store). The callback parameters `links`, `removed`, `records`, `themed` are not in the JSON.
    #[serde(rename = "profilesAdapter")]
    ProfilesAdapter,
    /// `profileSyncAdapters` (an exported function that writes a store). The callback parameters `links`, `records` are not in the JSON.
    #[serde(rename = "profileSyncAdapters")]
    ProfileSyncAdapters,
    /// `profileThemesAdapter` (an exported function that writes a store). The callback parameters `links`, `records`, `removed` are not in the JSON.
    #[serde(rename = "profileThemesAdapter")]
    ProfileThemesAdapter,
    /// `profileTopAppsAdapter` (an exported function that writes a store). The callback parameters `links`, `removed` are not in the JSON.
    #[serde(rename = "profileTopAppsAdapter")]
    ProfileTopAppsAdapter,
    /// `removeRemoteProfile` (an exported function that writes a store).
    #[serde(rename = "removeRemoteProfile")]
    RemoveRemoteProfile {
        #[serde(rename = "syncId")]
        sync_id: String,
    },
    /// `removeSyncedDevice` (an exported function that writes a store).
    #[serde(rename = "removeSyncedDevice")]
    RemoveSyncedDevice { id: String },
    /// `sendTab` (an exported function that writes a store).
    #[serde(rename = "sendTab")]
    SendTab {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(rename = "deviceId")]
        device_id: String,
    },
    /// `sendTabFromMenu` (an exported function that writes a store).
    #[serde(rename = "sendTabFromMenu")]
    SendTabFromMenu {
        choice: String,
        tab: crate::browser::Tab,
    },
    /// `setOpenTabsSynced` (an exported function that writes a store).
    #[serde(rename = "setOpenTabsSynced")]
    SetOpenTabsSynced {
        #[serde(rename = "profileId")]
        profile_id: String,
        on: bool,
    },
    /// `setProfileSynced` (an exported function that writes a store).
    #[serde(rename = "setProfileSynced")]
    SetProfileSynced {
        #[serde(rename = "profileId")]
        profile_id: String,
        enabled: bool,
    },
    /// `setSyncFolder` (an exported function that writes a store).
    #[serde(rename = "setSyncFolder")]
    SetSyncFolder { path: String },
    /// `settleChoices` (an exported function that writes a store).
    #[serde(rename = "settleChoices")]
    SettleChoices {
        #[serde(rename = "profileId")]
        profile_id: String,
        published: std::collections::HashMap<String, serde_json::Value>,
    },
    /// `setTypeSynced` (an exported function that writes a store).
    #[serde(rename = "setTypeSynced")]
    SetTypeSynced {
        #[serde(rename = "typeArg")]
        type_arg: SetTypeSyncedTypeArg,
        enabled: bool,
    },
    /// `startSync` (an exported function that writes a store).
    #[serde(rename = "startSync")]
    StartSync,
    /// `stopSync` (an exported function that writes a store).
    #[serde(rename = "stopSync")]
    StopSync {
        #[serde(default)]
        __0: Option<StopSyncV0>,
    },
    /// `syncNow` (an exported function that writes a store). The callback parameters `stillWanted` are not in the JSON.
    #[serde(rename = "syncNow")]
    SyncNow,
    /// `tileFoldersAdapter` (an exported function that writes a store).
    #[serde(rename = "tileFoldersAdapter")]
    TileFoldersAdapter {
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    /// `turnOnSync` (an exported function that writes a store).
    #[serde(rename = "turnOnSync")]
    TurnOnSync {
        folder: String,
        #[serde(default)]
        how: Option<TurnOnSyncHow>,
    },
}

/// Applies `action` to `app`. Slice 2.31 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.31")
}
