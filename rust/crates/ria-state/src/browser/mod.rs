// Ported from ardia apps/browser/src/state/browser/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `browser` (`apps/browser/src/state/browser`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.5 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TabGroup {
    pub id: String,
    #[serde(rename = "windowId")]
    pub window_id: String,
    #[serde(rename = "profileId")]
    pub profile_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: serde_json::Value,
    pub collapsed: bool,
    pub pinned: bool,
    #[serde(rename = "tabIds")]
    pub tab_ids: Vec<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Number,
    #[serde(
        rename = "autoUngroup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub auto_ungroup: Option<bool>,
    #[serde(
        rename = "pinnedOpenerId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub pinned_opener_id: Option<String>,
    #[serde(rename = "pinKey", default, skip_serializing_if = "Option::is_none")]
    pub pin_key: Option<String>,
    #[serde(rename = "parentId", default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(
        rename = "shownTabIds",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub shown_tab_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Tab {
    pub id: String,
    #[serde(rename = "windowId")]
    pub window_id: String,
    #[serde(rename = "profileId")]
    pub profile_id: String,
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub pin: Option<TabPin>,
    pub muted: bool,
    pub zoom: Number,
    #[serde(rename = "customTitle")]
    pub custom_title: Option<String>,
    #[serde(rename = "customIcon")]
    pub custom_icon: Option<String>,
    #[serde(rename = "pinnedUrl")]
    pub pinned_url: Option<String>,
    pub page: TabPage,
    #[serde(rename = "openerId")]
    pub opener_id: Option<String>,
    #[serde(
        rename = "linkOpenerId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub link_opener_id: Option<String>,
    #[serde(rename = "liveItem", default, skip_serializing_if = "Option::is_none")]
    pub live_item: Option<TabLiveItem>,
    #[serde(rename = "pinKey", default, skip_serializing_if = "Option::is_none")]
    pub pin_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<serde_json::Value>,
    #[serde(
        rename = "chromeOpened",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub chrome_opened: Option<bool>,
    #[serde(rename = "createdAt")]
    pub created_at: Number,
    #[serde(rename = "lastActiveAt")]
    pub last_active_at: Number,
    #[serde(
        rename = "restoredAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub restored_at: Option<Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unseen: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "as")]
pub enum TabPin {
    #[serde(rename = "tile")]
    Tile {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        folder: Option<TileFolder>,
    },
    #[serde(rename = "row")]
    Row,
}

impl Default for TabPin {
    fn default() -> Self {
        Self::Tile {
            folder: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TileFolder {
    pub id: String,
    pub name: String,
    #[serde(rename = "renamedAt", default, skip_serializing_if = "Option::is_none")]
    pub renamed_at: Option<Number>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TabPage {
    #[serde(rename = "idle")]
    Idle {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wake: Option<TabPageIdleWake>,
    },
    #[serde(rename = "asleep")]
    Asleep {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wake: Option<TabPageAsleepWake>,
    },
    #[serde(rename = "frozen")]
    Frozen {
        #[serde(rename = "holderId")]
        holder_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wake: Option<TabPageFrozenWake>,
    },
    #[serde(rename = "adopted")]
    Adopted {
        adopt: TabPageAdoptedAdopt,
        nav: Option<Navigation>,
    },
    #[serde(rename = "loaded")]
    Loaded { nav: Navigation },
}

impl Default for TabPage {
    fn default() -> Self {
        Self::Idle {
            wake: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TabPageIdleWake {
    Other0(String),
    Other1(String),
    Other2(String),
    Other3(String),
    Other4(String),
    Other5(String),
}

impl Default for TabPageIdleWake {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TabPageAsleepWake {
    Other0(String),
    Other1(String),
    Other2(String),
    Other3(String),
    Other4(String),
    Other5(String),
}

impl Default for TabPageAsleepWake {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TabPageFrozenWake {
    Other0(String),
    Other1(String),
    Other2(String),
    Other3(String),
    Other4(String),
    Other5(String),
}

impl Default for TabPageFrozenWake {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TabPageAdoptedAdopt {
    Other0(String),
    Other1(String),
    Other2(String),
    Other3(String),
    Other4(String),
    Other5(String),
}

impl Default for TabPageAdoptedAdopt {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Navigation {
    pub url: String,
    pub seq: Number,
    #[serde(
        rename = "userInitiated",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub user_initiated: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TabLiveItem {
    #[serde(rename = "folderId")]
    pub folder_id: String,
    #[serde(rename = "itemId")]
    pub item_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub color: ProfileColor,
    pub icon: Option<String>,
    #[serde(
        rename = "switcherDot",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub switcher_dot: Option<bool>,
    #[serde(rename = "createdAt")]
    pub created_at: Number,
    #[serde(rename = "dataId", default, skip_serializing_if = "Option::is_none")]
    pub data_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<serde_json::Value>,
    #[serde(
        rename = "presetThemes",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub preset_themes: Option<ProfilePresetThemes>,
    #[serde(rename = "seenTheme", default, skip_serializing_if = "Option::is_none")]
    pub seen_theme: Option<serde_json::Value>,
    #[serde(rename = "seenColor", default, skip_serializing_if = "Option::is_none")]
    pub seen_color: Option<ProfileSeenColor>,
    #[serde(
        rename = "pinnedCollapsed",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub pinned_collapsed: Option<bool>,
    #[serde(
        rename = "shareTopApps",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub share_top_apps: Option<bool>,
    #[serde(rename = "syncTabs", default, skip_serializing_if = "Option::is_none")]
    pub sync_tabs: Option<bool>,
    #[serde(
        rename = "passwordManager",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub password_manager: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ProfileColor {
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
pub struct ProfilePresetThemes {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plum: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blue: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purple: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pink: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub red: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orange: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yellow: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub green: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub neutral: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ProfileSeenColor {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum BrowserWindow {
    #[serde(rename = "main")]
    Main {
        id: String,
        #[serde(rename = "profileId")]
        profile_id: String,
        #[serde(rename = "tabIds")]
        tab_ids: Vec<String>,
        #[serde(rename = "activeTabIds")]
        active_tab_ids: std::collections::HashMap<String, String>,
        #[serde(rename = "tabLayout", default, skip_serializing_if = "Option::is_none")]
        tab_layout: Option<BrowserWindowMainTabLayout>,
        #[serde(
            rename = "autoHideTabs",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        auto_hide_tabs: Option<bool>,
        frame: Option<(Number, Number, Number, Number)>,
        #[serde(
            rename = "restoredFrame",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        restored_frame: Option<(Number, Number, Number, Number)>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        maximized: Option<bool>,
        #[serde(
            rename = "fullScreen",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        full_screen: Option<Number>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display: Option<String>,
        #[serde(rename = "createdAt")]
        created_at: Number,
        #[serde(
            rename = "chromeWindow",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        chrome_window: Option<String>,
        #[serde(
            rename = "opensBehind",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        opens_behind: Option<String>,
    },
    #[serde(rename = "private")]
    Private {
        id: String,
        #[serde(rename = "profileId")]
        profile_id: String,
        #[serde(rename = "tabIds")]
        tab_ids: Vec<String>,
        #[serde(rename = "activeTabIds")]
        active_tab_ids: std::collections::HashMap<String, String>,
        #[serde(rename = "tabLayout", default, skip_serializing_if = "Option::is_none")]
        tab_layout: Option<BrowserWindowPrivateTabLayout>,
        #[serde(
            rename = "autoHideTabs",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        auto_hide_tabs: Option<bool>,
        frame: Option<(Number, Number, Number, Number)>,
        #[serde(
            rename = "restoredFrame",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        restored_frame: Option<(Number, Number, Number, Number)>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        maximized: Option<bool>,
        #[serde(
            rename = "fullScreen",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        full_screen: Option<Number>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display: Option<String>,
        #[serde(rename = "createdAt")]
        created_at: Number,
        #[serde(
            rename = "chromeWindow",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        chrome_window: Option<String>,
    },
    #[serde(rename = "small")]
    Small {
        id: String,
        #[serde(rename = "profileId")]
        profile_id: String,
        #[serde(rename = "tabIds")]
        tab_ids: Vec<String>,
        #[serde(rename = "activeTabIds")]
        active_tab_ids: std::collections::HashMap<String, String>,
        #[serde(rename = "tabLayout", default, skip_serializing_if = "Option::is_none")]
        tab_layout: Option<BrowserWindowSmallTabLayout>,
        #[serde(
            rename = "autoHideTabs",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        auto_hide_tabs: Option<bool>,
        frame: Option<(Number, Number, Number, Number)>,
        #[serde(
            rename = "restoredFrame",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        restored_frame: Option<(Number, Number, Number, Number)>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        maximized: Option<bool>,
        #[serde(
            rename = "fullScreen",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        full_screen: Option<Number>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display: Option<String>,
        #[serde(rename = "createdAt")]
        created_at: Number,
        #[serde(
            rename = "linkPointer",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        link_pointer: Option<(Number, Number)>,
        #[serde(rename = "linkApp", default, skip_serializing_if = "Option::is_none")]
        link_app: Option<String>,
    },
}

impl Default for BrowserWindow {
    fn default() -> Self {
        Self::Main {
            id: Default::default(),
            profile_id: Default::default(),
            tab_ids: Default::default(),
            active_tab_ids: Default::default(),
            tab_layout: Default::default(),
            auto_hide_tabs: Default::default(),
            frame: Default::default(),
            restored_frame: Default::default(),
            maximized: Default::default(),
            full_screen: Default::default(),
            display: Default::default(),
            created_at: Default::default(),
            chrome_window: Default::default(),
            opens_behind: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BrowserWindowMainTabLayout {
    #[default]
    #[serde(rename = "sidebar")]
    Sidebar,
    #[serde(rename = "top")]
    Top,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BrowserWindowPrivateTabLayout {
    #[default]
    #[serde(rename = "sidebar")]
    Sidebar,
    #[serde(rename = "top")]
    Top,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BrowserWindowSmallTabLayout {
    #[default]
    #[serde(rename = "sidebar")]
    Sidebar,
    #[serde(rename = "top")]
    Top,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BrowserState {
    pub profiles: std::collections::HashMap<String, Profile>,
    #[serde(rename = "profileOrder")]
    pub profile_order: Vec<String>,
    #[serde(rename = "orphanedProfileData")]
    pub orphaned_profile_data: Vec<String>,
    pub windows: std::collections::HashMap<String, BrowserWindow>,
    #[serde(rename = "windowOrder")]
    pub window_order: Vec<String>,
    #[serde(rename = "closedWindows")]
    pub closed_windows: Vec<ClosedWindow>,
    #[serde(rename = "parkedPins")]
    pub parked_pins: std::collections::HashMap<String, ParkedPins>,
    pub tabs: std::collections::HashMap<String, Tab>,
    #[serde(rename = "closedTabs")]
    pub closed_tabs: Vec<ClosedTab>,
    pub groups: std::collections::HashMap<String, TabGroup>,
    #[serde(rename = "closedGroups")]
    pub closed_groups: Vec<ClosedGroup>,
    #[serde(rename = "deletedGroups")]
    pub deleted_groups: Vec<ClosedGroup>,
    pub selection: std::collections::HashMap<String, Vec<String>>,
    #[serde(rename = "selectionAnchors")]
    pub selection_anchors: std::collections::HashMap<String, crate::drag::SelectionAnchor>,
    #[serde(rename = "pinChoices")]
    pub pin_choices:
        std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>>,
    #[serde(rename = "pinSync")]
    pub pin_sync: std::collections::HashMap<
        String,
        std::collections::HashMap<String, crate::pins::PinSyncEntry>,
    >,
    #[serde(rename = "topAppsMerged")]
    pub top_apps_merged: String,
    #[serde(rename = "privateSiteMutes")]
    pub private_site_mutes:
        std::collections::HashMap<String, std::collections::HashMap<String, bool>>,
    #[serde(rename = "cleanedTabs")]
    pub cleaned_tabs: Vec<ClosedTab>,
    #[serde(rename = "privateCleanedTabs")]
    pub private_cleaned_tabs: std::collections::HashMap<String, Vec<ClosedTab>>,
    pub splits: std::collections::HashMap<String, SplitView>,
    pub history: std::collections::HashMap<String, crate::history::HistoryView>,
    pub bookmarks: Bookmarks,
    #[serde(rename = "bookmarksReady")]
    pub bookmarks_ready: std::collections::HashMap<String, bool>,
    pub settings: crate::settings::Settings,
    #[serde(rename = "settingsMeta")]
    pub settings_meta: crate::settings::SettingsMeta,
    pub ui: crate::automation::SnapshotUi,
    #[serde(rename = "windowUi")]
    pub window_ui: std::collections::HashMap<String, WindowUi>,
    pub find: std::collections::HashMap<String, FindState>,
    pub downloads: Vec<serde_json::Value>,
    pub peeks: std::collections::HashMap<String, crate::peek::Peek>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClosedWindow {
    pub kind: String,
    pub id: String,
    pub window: ClosedWindowWindow,
    pub tabs: Vec<ClosedWindowTabsItem>,
    pub groups: Vec<ClosedWindowGroupsItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub splits: Option<Vec<ClosedWindowSplitsItem>>,
    #[serde(rename = "closedAt")]
    pub closed_at: Number,
    #[serde(
        rename = "lastWindow",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_window: Option<bool>,
    #[serde(rename = "activePin", default, skip_serializing_if = "Option::is_none")]
    pub active_pin: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClosedWindowWindow {
    #[serde(rename = "profileId")]
    pub profile_id: String,
    #[serde(rename = "tabLayout", default, skip_serializing_if = "Option::is_none")]
    pub tab_layout: Option<ClosedWindowWindowTabLayout>,
    #[serde(
        rename = "autoHideTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub auto_hide_tabs: Option<bool>,
    pub frame: Option<(Number, Number, Number, Number)>,
    #[serde(
        rename = "restoredFrame",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub restored_frame: Option<(Number, Number, Number, Number)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximized: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ClosedWindowWindowTabLayout {
    #[default]
    #[serde(rename = "sidebar")]
    Sidebar,
    #[serde(rename = "top")]
    Top,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClosedWindowTabsItem {
    #[serde(rename = "profileId")]
    pub profile_id: String,
    pub url: String,
    pub pin: Option<ClosedWindowTabsItemPin>,
    pub title: String,
    pub favicon: Option<String>,
    pub muted: bool,
    pub zoom: Number,
    #[serde(rename = "customTitle")]
    pub custom_title: Option<String>,
    #[serde(rename = "customIcon")]
    pub custom_icon: Option<String>,
    #[serde(
        rename = "lastActiveAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_active_at: Option<Number>,
    #[serde(rename = "pinnedUrl", default, skip_serializing_if = "Option::is_none")]
    pub pinned_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<serde_json::Value>,
    pub active: bool,
    #[serde(rename = "tabId", default, skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "as")]
pub enum ClosedWindowTabsItemPin {
    #[serde(rename = "tile")]
    Tile {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        folder: Option<TileFolder>,
    },
    #[serde(rename = "row")]
    Row,
}

impl Default for ClosedWindowTabsItemPin {
    fn default() -> Self {
        Self::Tile {
            folder: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClosedWindowGroupsItem {
    #[serde(rename = "profileId")]
    pub profile_id: String,
    pub id: String,
    pub name: String,
    pub color: serde_json::Value,
    pub icon: Option<String>,
    pub collapsed: bool,
    pub pinned: bool,
    #[serde(rename = "pinKey", default, skip_serializing_if = "Option::is_none")]
    pub pin_key: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Number,
    #[serde(
        rename = "autoUngroup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub auto_ungroup: Option<bool>,
    #[serde(
        rename = "pinnedOpenerId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub pinned_opener_id: Option<String>,
    #[serde(rename = "parentId", default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(rename = "tabIndexes")]
    pub tab_indexes: Vec<Number>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClosedWindowSplitsItem {
    pub slots: Vec<SplitSlot>,
    pub orientation: ClosedWindowSplitsItemOrientation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SplitSlot {
    Other0(SplitSlotVariant0),
    Other1(SplitSlotVariant1),
}

impl Default for SplitSlot {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SplitSlotVariant0 {
    pub size: Number,
    #[serde(rename = "tabId")]
    pub tab_id: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SplitSlotVariant1 {
    pub size: Number,
    pub stack: (String, String),
    #[serde(rename = "stackSizes")]
    pub stack_sizes: (Number, Number),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ClosedWindowSplitsItemOrientation {
    #[default]
    #[serde(rename = "horizontal")]
    Horizontal,
    #[serde(rename = "vertical")]
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ParkedPins {
    pub tabs: Vec<ParkedPinsTabsItem>,
    pub groups: Vec<ParkedPinsGroupsItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub splits: Option<Vec<ClosedWindowSplitsItem>>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ParkedPinsTabsItem {
    #[serde(rename = "profileId")]
    pub profile_id: String,
    pub url: String,
    pub pin: Option<ParkedPinsTabsItemPin>,
    pub title: String,
    pub favicon: Option<String>,
    pub muted: bool,
    pub zoom: Number,
    #[serde(rename = "customTitle")]
    pub custom_title: Option<String>,
    #[serde(rename = "customIcon")]
    pub custom_icon: Option<String>,
    #[serde(
        rename = "lastActiveAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_active_at: Option<Number>,
    #[serde(rename = "pinnedUrl", default, skip_serializing_if = "Option::is_none")]
    pub pinned_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<serde_json::Value>,
    pub id: String,
    #[serde(rename = "groupId")]
    pub group_id: Option<String>,
    #[serde(
        rename = "restoreFrom",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub restore_from: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "as")]
pub enum ParkedPinsTabsItemPin {
    #[serde(rename = "tile")]
    Tile {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        folder: Option<TileFolder>,
    },
    #[serde(rename = "row")]
    Row,
}

impl Default for ParkedPinsTabsItemPin {
    fn default() -> Self {
        Self::Tile {
            folder: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ParkedPinsGroupsItem {
    pub id: String,
    pub name: String,
    pub color: serde_json::Value,
    pub icon: Option<String>,
    pub collapsed: bool,
    #[serde(rename = "parentId", default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClosedTab {
    pub kind: String,
    pub id: String,
    pub tab: TabSnapshot,
    #[serde(rename = "tabId", default, skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
    #[serde(
        rename = "restoreFrom",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub restore_from: Option<String>,
    #[serde(
        rename = "closeBatch",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub close_batch: Option<String>,
    #[serde(rename = "windowId")]
    pub window_id: String,
    pub index: Number,
    pub group: Option<ClosedGroupInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parents: Option<Vec<ClosedGroupInfo>>,
    #[serde(rename = "closedAt")]
    pub closed_at: Number,
    #[serde(
        rename = "pinnedTile",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub pinned_tile: Option<bool>,
    #[serde(rename = "pinKey", default, skip_serializing_if = "Option::is_none")]
    pub pin_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub small: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TabSnapshot {
    #[serde(rename = "profileId")]
    pub profile_id: String,
    pub url: String,
    pub pin: Option<TabSnapshotPin>,
    pub title: String,
    pub favicon: Option<String>,
    pub muted: bool,
    pub zoom: Number,
    #[serde(rename = "customTitle")]
    pub custom_title: Option<String>,
    #[serde(rename = "customIcon")]
    pub custom_icon: Option<String>,
    #[serde(
        rename = "lastActiveAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_active_at: Option<Number>,
    #[serde(rename = "pinnedUrl", default, skip_serializing_if = "Option::is_none")]
    pub pinned_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "as")]
pub enum TabSnapshotPin {
    #[serde(rename = "tile")]
    Tile {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        folder: Option<TileFolder>,
    },
    #[serde(rename = "row")]
    Row,
}

impl Default for TabSnapshotPin {
    fn default() -> Self {
        Self::Tile {
            folder: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClosedGroupInfo {
    pub id: String,
    pub name: String,
    pub color: serde_json::Value,
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClosedGroup {
    pub kind: String,
    pub id: String,
    pub group: ClosedGroupGroup,
    pub tabs: Vec<TabSnapshot>,
    #[serde(rename = "tabIds", default, skip_serializing_if = "Option::is_none")]
    pub tab_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nested: Option<Vec<NestedGroupSnapshot>>,
    #[serde(rename = "windowId")]
    pub window_id: String,
    pub index: Number,
    #[serde(rename = "closedAt")]
    pub closed_at: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClosedGroupGroup {
    pub id: String,
    pub name: String,
    pub color: serde_json::Value,
    pub icon: Option<String>,
    pub pinned: bool,
    #[serde(rename = "parentId", default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct NestedGroupSnapshot {
    pub id: String,
    pub name: String,
    pub color: serde_json::Value,
    pub icon: Option<String>,
    pub collapsed: bool,
    #[serde(rename = "parentId")]
    pub parent_id: String,
    #[serde(rename = "tabIndexes")]
    pub tab_indexes: Vec<Number>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SplitView {
    pub id: String,
    #[serde(rename = "windowId")]
    pub window_id: String,
    pub orientation: SplitViewOrientation,
    pub slots: Vec<SplitSlot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SplitViewOrientation {
    #[default]
    #[serde(rename = "horizontal")]
    Horizontal,
    #[serde(rename = "vertical")]
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub visits: Number,
    #[serde(rename = "lastVisit")]
    pub last_visit: Number,
    #[serde(
        rename = "visitTimes",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub visit_times: Option<Vec<Number>>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Bookmarks {
    pub nodes: std::collections::HashMap<String, BookmarkNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum BookmarkNode {
    #[serde(rename = "url")]
    Url {
        id: String,
        #[serde(rename = "parentId")]
        parent_id: String,
        title: String,
        url: String,
        favicon: Option<String>,
        #[serde(rename = "addedAt")]
        added_at: Number,
        #[serde(rename = "syncKey", default, skip_serializing_if = "Option::is_none")]
        sync_key: Option<String>,
    },
    #[serde(rename = "folder")]
    Folder {
        id: String,
        #[serde(rename = "parentId")]
        parent_id: Option<String>,
        title: String,
        children: Vec<String>,
        #[serde(rename = "addedAt")]
        added_at: Number,
        #[serde(rename = "syncKey", default, skip_serializing_if = "Option::is_none")]
        sync_key: Option<String>,
    },
}

impl Default for BookmarkNode {
    fn default() -> Self {
        Self::Url {
            id: Default::default(),
            parent_id: Default::default(),
            title: Default::default(),
            url: Default::default(),
            favicon: Default::default(),
            added_at: Default::default(),
            sync_key: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WindowUi {
    pub panel: PanelState,
    #[serde(rename = "downloadsOpen")]
    pub downloads_open: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PanelState {
    pub open: bool,
    #[serde(rename = "initialText")]
    pub initial_text: String,
    #[serde(rename = "newTab", default, skip_serializing_if = "Option::is_none")]
    pub new_tab: Option<PanelStateNewTab>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<PanelStatePlacement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum PanelStateNewTab {
    #[serde(rename = "list")]
    List,
    #[serde(rename = "group")]
    Group {
        #[serde(rename = "groupId")]
        group_id: String,
    },
    #[serde(rename = "into")]
    Into {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
}

impl Default for PanelStateNewTab {
    fn default() -> Self {
        Self::List
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PanelStatePlacement {
    #[default]
    #[serde(rename = "field")]
    Field,
    #[serde(rename = "centred")]
    Centred,
    #[serde(rename = "centredNoField")]
    CentredNoField,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FindState {
    pub open: bool,
    pub query: String,
    pub count: Option<Number>,
    pub active: Number,
    #[serde(
        rename = "focusRequest",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub focus_request: Option<Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replace: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SidebarSide {
    #[default]
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HydrateData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groups: Option<std::collections::HashMap<String, TabGroup>>,
    #[serde(
        rename = "parkedPins",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub parked_pins: Option<std::collections::HashMap<String, ParkedPins>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tabs: Option<std::collections::HashMap<String, Tab>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profiles: Option<std::collections::HashMap<String, Profile>>,
    #[serde(
        rename = "profileOrder",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub profile_order: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloads: Option<Vec<serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<crate::settings::Settings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows: Option<std::collections::HashMap<String, BrowserWindow>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub splits: Option<std::collections::HashMap<String, SplitView>>,
    #[serde(
        rename = "orphanedProfileData",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub orphaned_profile_data: Option<Vec<String>>,
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
    pub closed_windows: Option<Vec<ClosedWindow>>,
    #[serde(
        rename = "closedTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub closed_tabs: Option<Vec<ClosedTab>>,
    #[serde(
        rename = "closedGroups",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub closed_groups: Option<Vec<ClosedGroup>>,
    #[serde(
        rename = "deletedGroups",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub deleted_groups: Option<Vec<ClosedGroup>>,
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
        rename = "cleanedTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub cleaned_tabs: Option<Vec<ClosedTab>>,
    #[serde(
        rename = "settingsMeta",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub settings_meta: Option<crate::settings::SettingsMeta>,
    #[serde(
        rename = "focusedWindowId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub focused_window_id: Option<String>,
    #[serde(
        rename = "lastProfileId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_profile_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum NewTabTarget {
    #[serde(rename = "list")]
    List,
    #[serde(rename = "group")]
    Group {
        #[serde(rename = "groupId")]
        group_id: String,
    },
    #[serde(rename = "into")]
    Into {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
}

impl Default for NewTabTarget {
    fn default() -> Self {
        Self::List
    }
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `closeGroup` (an exported function that writes a store). The callback parameters `mayWrite` are not in the JSON.
    #[serde(rename = "closeGroup")]
    CloseGroup {
        #[serde(rename = "groupId")]
        group_id: String,
    },
    /// `closeTab` (an exported function that writes a store). The callback parameters `mayWrite` are not in the JSON.
    #[serde(rename = "closeTab")]
    CloseTab {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `cycleProfile` (an exported function that writes a store). The callback parameters `mayWrite` are not in the JSON.
    #[serde(rename = "cycleProfile")]
    CycleProfile {
        #[serde(rename = "windowId")]
        window_id: String,
        delta: Number,
    },
    /// `deleteProfile` (an exported function that writes a store). The callback parameters `mayDelete`, `ask` are not in the JSON.
    #[serde(rename = "deleteProfile")]
    DeleteProfile {
        #[serde(rename = "profileId")]
        profile_id: String,
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `dropTabsInProfile` (an exported function that writes a store).
    #[serde(rename = "dropTabsInProfile")]
    DropTabsInProfile {
        #[serde(rename = "tabIds")]
        tab_ids: Vec<String>,
        target: String,
        drop: crate::drag::Drop,
        home: String,
        #[serde(rename = "onIcon")]
        on_icon: bool,
    },
    /// `focus` (an exported function that writes a store).
    #[serde(rename = "focus")]
    Focus {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `hydrate` (a store method).
    #[serde(rename = "hydrate")]
    Hydrate { data: HydrateData },
    /// `moveGroupToProfile` (an exported function that writes a store).
    #[serde(rename = "moveGroupToProfile")]
    MoveGroupToProfile {
        #[serde(rename = "groupId")]
        group_id: String,
        target: Option<String>,
    },
    /// `moveGroupToWindow` (an exported function that writes a store).
    #[serde(rename = "moveGroupToWindow")]
    MoveGroupToWindow {
        #[serde(rename = "groupId")]
        group_id: String,
        target: Option<String>,
    },
    /// `moveTabsToProfile` (an exported function that writes a store). The callback parameters `mayWrite` are not in the JSON.
    #[serde(rename = "moveTabsToProfile")]
    MoveTabsToProfile {
        #[serde(rename = "tabIds")]
        tab_ids: Vec<String>,
        target: Option<String>,
    },
    /// `moveTabToProfile` (an exported function that writes a store). The callback parameters `mayWrite` are not in the JSON.
    #[serde(rename = "moveTabToProfile")]
    MoveTabToProfile {
        #[serde(rename = "tabId")]
        tab_id: String,
        target: Option<String>,
    },
    /// `moveTabToWindow` (an exported function that writes a store).
    #[serde(rename = "moveTabToWindow")]
    MoveTabToWindow {
        #[serde(rename = "tabId")]
        tab_id: String,
        target: Option<String>,
    },
    /// `openProfileMenu` (an exported function that writes a store).
    #[serde(rename = "openProfileMenu")]
    OpenProfileMenu {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `openUrls` (an exported function that writes a store).
    #[serde(rename = "openUrls")]
    OpenUrls {
        urls: Vec<String>,
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `openWindow` (an exported function that writes a store).
    #[serde(rename = "openWindow")]
    OpenWindow {
        #[serde(default)]
        options: Option<crate::windows::CreateWindowOptions>,
    },
    /// `setSiteMuted` (an exported function that writes a store).
    #[serde(rename = "setSiteMuted")]
    SetSiteMuted {
        #[serde(rename = "tabId")]
        tab_id: String,
        muted: bool,
    },
    /// `switchProfile` (an exported function that writes a store). The callback parameters `mayWrite` are not in the JSON.
    #[serde(rename = "switchProfile")]
    SwitchProfile {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "profileId")]
        profile_id: String,
        #[serde(default)]
        animated: Option<bool>,
    },
    /// `switchToTab` (an exported function that writes a store).
    #[serde(rename = "switchToTab")]
    SwitchToTab {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `toggleMute` (an exported function that writes a store).
    #[serde(rename = "toggleMute")]
    ToggleMute {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `toggleSiteMute` (an exported function that writes a store).
    #[serde(rename = "toggleSiteMute")]
    ToggleSiteMute {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
}

/// Applies `action` to `app`. Slice 2.5 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.5")
}
