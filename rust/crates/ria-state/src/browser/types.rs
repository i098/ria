// Ported from ardia apps/browser/src/state/browser/types.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Data types of the browser store. Each type serializes to the JSON that `JSON.stringify` gives for the TypeScript
//! value (plan section 6.5): `T | null` is `Option<T>` that writes `null`, and `x?: T` is `Option<T>` that is absent.
//!
//! Changes from Ardia: `BrowserWindow` is one struct with a `kind` field (types.ts:35-39 is a union of three object
//! types). The fields that only one kind has (`chromeWindow`, `opensBehind`, `linkPointer`, `linkApp`) are optional,
//! and helpers read `kind`. The JSON is the same. `Tab`, `TabGroup`, `Profile` and the closed-item types replace the
//! slice-2.4 skeleton types, which did not write the JSON of the TypeScript objects.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::kernel::Number;

pub type Frame = [Number; 4];

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub color: String,
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub switcher_dot: Option<bool>,
    pub created_at: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset_themes: Option<ProfilePresetThemes>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seen_theme: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seen_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_collapsed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share_top_apps: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_tabs: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password_manager: Option<String>,
}

/// `PresetThemes` (`lib/profileLook.ts`): a theme for each profile color, each one optional.
pub type ProfilePresetThemes = HashMap<String, Value>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowKind {
    #[default]
    Main,
    Private,
    Small,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TabLayout {
    #[default]
    Sidebar,
    Top,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserWindow {
    pub kind: WindowKind,
    pub id: String,
    pub profile_id: String,
    pub tab_ids: Vec<String>,
    pub active_tab_ids: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_layout: Option<TabLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_hide_tabs: Option<bool>,
    pub frame: Option<Frame>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restored_frame: Option<Frame>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximized: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_screen: Option<Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    pub created_at: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chrome_window: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opens_behind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_pointer: Option<[Number; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_app: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabLiveItem {
    pub folder_id: String,
    pub item_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    pub id: String,
    pub window_id: String,
    pub profile_id: String,
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub pin: Option<TabPin>,
    pub muted: bool,
    pub zoom: Number,
    pub custom_title: Option<String>,
    pub custom_icon: Option<String>,
    pub pinned_url: Option<String>,
    pub page: TabPage,
    pub opener_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_opener_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live_item: Option<TabLiveItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chrome_opened: Option<bool>,
    pub created_at: Number,
    pub last_active_at: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restored_at: Option<Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unseen: Option<bool>,
}

/// `AdoptId` is `${kind}:${string}`; the Rust type is the string.
pub type AdoptId = String;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Navigation {
    pub url: String,
    pub seq: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_initiated: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", rename_all_fields = "camelCase")]
pub enum TabPage {
    Idle {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wake: Option<AdoptId>,
    },
    Asleep {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wake: Option<AdoptId>,
    },
    Frozen {
        holder_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wake: Option<AdoptId>,
    },
    Adopted {
        adopt: AdoptId,
        nav: Option<Navigation>,
    },
    Loaded {
        nav: Navigation,
    },
}

impl Default for TabPage {
    fn default() -> Self {
        Self::Idle { wake: None }
    }
}

impl TabPage {
    /// The `kind` string of the TypeScript object.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Idle { .. } => "idle",
            Self::Asleep { .. } => "asleep",
            Self::Frozen { .. } => "frozen",
            Self::Adopted { .. } => "adopted",
            Self::Loaded { .. } => "loaded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "as", rename_all = "lowercase")]
pub enum TabPin {
    Tile {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        folder: Option<TileFolder>,
    },
    Row,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TileFolder {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renamed_at: Option<Number>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabLive {
    pub is_loading: bool,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub playing_audio: bool,
    pub theme_color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosedGroupInfo {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabGroup {
    pub id: String,
    pub window_id: String,
    pub profile_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub collapsed: bool,
    pub pinned: bool,
    pub tab_ids: Vec<String>,
    pub created_at: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_ungroup: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_opener_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shown_tab_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SplitSlot {
    Single {
        size: Number,
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    Stack {
        size: Number,
        stack: [String; 2],
        #[serde(rename = "stackSizes")]
        stack_sizes: [Number; 2],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitView {
    pub id: String,
    pub window_id: String,
    pub orientation: Orientation,
    pub slots: Vec<SplitSlot>,
}

/// `Pick<SplitView, "orientation" | "slots">`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClosedWindowSplitsItem {
    pub orientation: Orientation,
    pub slots: Vec<SplitSlot>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub visits: Number,
    pub last_visit: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visit_times: Option<Vec<Number>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", rename_all_fields = "camelCase")]
pub enum BookmarkNode {
    Url {
        id: String,
        parent_id: String,
        title: String,
        url: String,
        favicon: Option<String>,
        added_at: Number,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sync_key: Option<String>,
    },
    Folder {
        id: String,
        parent_id: Option<String>,
        title: String,
        children: Vec<String>,
        added_at: Number,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sync_key: Option<String>,
    },
}

impl Default for BookmarkNode {
    fn default() -> Self {
        Self::Url {
            id: String::new(),
            parent_id: String::new(),
            title: String::new(),
            url: String::new(),
            favicon: None,
            added_at: Number::default(),
            sync_key: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Bookmarks {
    pub nodes: HashMap<String, BookmarkNode>,
}

/// `Pick<TabSnapshot, ...>`: the saved fields of a closed tab (`TabSnapshot`, types.ts:127-130).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabSnapshot {
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub pin: Option<TabPin>,
    pub muted: bool,
    pub zoom: Number,
    pub custom_title: Option<String>,
    pub custom_icon: Option<String>,
    pub profile_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_active_at: Option<Number>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosedTab {
    pub kind: String,
    pub id: String,
    pub tab: TabSnapshot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close_batch: Option<String>,
    pub window_id: String,
    pub index: Number,
    pub group: Option<ClosedGroupInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parents: Option<Vec<ClosedGroupInfo>>,
    pub closed_at: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_tile: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub small: Option<bool>,
}

/// `Pick<BrowserWindow, "profileId" | "frame" | "restoredFrame" | "maximized" | "tabLayout" | "autoHideTabs">`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosedWindowWindow {
    pub profile_id: String,
    pub frame: Option<Frame>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restored_frame: Option<Frame>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximized: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_layout: Option<TabLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_hide_tabs: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosedWindowTabsItem {
    #[serde(flatten)]
    pub tab: TabSnapshot,
    pub active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosedWindowGroupsItem {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub collapsed: bool,
    pub pinned: bool,
    pub created_at: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_ungroup: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_opener_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub tab_indexes: Vec<Number>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosedWindow {
    pub kind: String,
    pub id: String,
    pub window: ClosedWindowWindow,
    pub tabs: Vec<ClosedWindowTabsItem>,
    pub groups: Vec<ClosedWindowGroupsItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub splits: Option<Vec<ClosedWindowSplitsItem>>,
    pub closed_at: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_window: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_pin: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParkedPinsTabsItem {
    #[serde(flatten)]
    pub tab: TabSnapshot,
    pub id: String,
    pub group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_from: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParkedPinsGroupsItem {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub collapsed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ParkedPins {
    pub tabs: Vec<ParkedPinsTabsItem>,
    pub groups: Vec<ParkedPinsGroupsItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub splits: Option<Vec<ClosedWindowSplitsItem>>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NestedGroupSnapshot {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub collapsed: bool,
    pub parent_id: String,
    pub tab_indexes: Vec<Number>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosedGroupGroup {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub pinned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosedGroup {
    pub kind: String,
    pub id: String,
    pub group: ClosedGroupGroup,
    pub tabs: Vec<TabSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nested: Option<Vec<NestedGroupSnapshot>>,
    pub window_id: String,
    pub index: Number,
    pub closed_at: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindState {
    pub open: bool,
    pub query: String,
    pub count: Option<Number>,
    pub active: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus_request: Option<Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replace: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", rename_all_fields = "camelCase")]
pub enum NewTabTarget {
    List,
    Group { group_id: String },
    Into { tab_id: String },
}

impl Default for NewTabTarget {
    fn default() -> Self {
        Self::List
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelState {
    pub open: bool,
    pub initial_text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_tab: Option<NewTabTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowUi {
    pub panel: PanelState,
    pub downloads_open: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SidebarSide {
    #[default]
    Left,
    Right,
}

/// The data fields of the TypeScript store. The fields of the slices that other folders own keep the types of the
/// slice-2.4 skeleton.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserState {
    pub profiles: HashMap<String, Profile>,
    pub profile_order: Vec<String>,
    pub orphaned_profile_data: Vec<String>,
    pub windows: HashMap<String, BrowserWindow>,
    pub window_order: Vec<String>,
    pub closed_windows: Vec<ClosedWindow>,
    pub parked_pins: HashMap<String, ParkedPins>,
    pub tabs: HashMap<String, Tab>,
    pub closed_tabs: Vec<ClosedTab>,
    pub groups: HashMap<String, TabGroup>,
    pub closed_groups: Vec<ClosedGroup>,
    pub deleted_groups: Vec<ClosedGroup>,
    pub selection: HashMap<String, Vec<String>>,
    pub selection_anchors: HashMap<String, crate::drag::SelectionAnchor>,
    pub pin_choices: HashMap<String, HashMap<String, Value>>,
    pub pin_sync: HashMap<String, HashMap<String, crate::pins::PinSyncEntry>>,
    pub top_apps_merged: String,
    pub private_site_mutes: HashMap<String, HashMap<String, bool>>,
    pub cleaned_tabs: Vec<ClosedTab>,
    pub private_cleaned_tabs: HashMap<String, Vec<ClosedTab>>,
    pub splits: HashMap<String, SplitView>,
    pub history: HashMap<String, crate::history::HistoryView>,
    pub bookmarks: Bookmarks,
    pub bookmarks_ready: HashMap<String, bool>,
    pub settings: crate::settings::Settings,
    pub settings_meta: crate::settings::SettingsMeta,
    pub ui: crate::automation::SnapshotUi,
    pub window_ui: HashMap<String, WindowUi>,
    pub find: HashMap<String, FindState>,
    pub downloads: Vec<Value>,
    pub peeks: HashMap<String, crate::peek::Peek>,
}

/// `HydrateData` (store.ts:51-72).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HydrateData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profiles: Option<HashMap<String, Profile>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_order: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orphaned_profile_data: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows: Option<HashMap<String, BrowserWindow>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_order: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tabs: Option<HashMap<String, Tab>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groups: Option<HashMap<String, TabGroup>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub splits: Option<HashMap<String, SplitView>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_tabs: Option<Vec<ClosedTab>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_windows: Option<Vec<ClosedWindow>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parked_pins: Option<HashMap<String, ParkedPins>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin_choices: Option<HashMap<String, HashMap<String, Value>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin_sync: Option<HashMap<String, HashMap<String, crate::pins::PinSyncEntry>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_apps_merged: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_groups: Option<Vec<ClosedGroup>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_groups: Option<Vec<ClosedGroup>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cleaned_tabs: Option<Vec<ClosedTab>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloads: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<crate::settings::Settings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings_meta: Option<crate::settings::SettingsMeta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_window_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_profile_id: Option<String>,
}
