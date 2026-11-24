// Ported from ardia apps/browser/src/state/drag/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `drag` (`apps/browser/src/state/drag`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.32 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SelectionAnchor {
    #[serde(rename = "tabId")]
    pub tab_id: String,
    #[serde(rename = "shownTabId")]
    pub shown_tab_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "phase")]
pub enum TabDrag {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "tab")]
    Tab {
        #[serde(rename = "tabId")]
        tab_id: Option<String>,
        #[serde(rename = "tabIds")]
        tab_ids: Vec<String>,
        #[serde(rename = "windowId")]
        window_id: String,
        x: Number,
        y: Number,
        lifted: Option<bool>,
        #[serde(rename = "onPage")]
        on_page: bool,
        target: Option<DropTarget>,
        outside: bool,
        #[serde(rename = "overWindow")]
        over_window: Option<String>,
        #[serde(rename = "overPoint")]
        over_point: Option<(Number, Number)>,
        insert: Option<Insert>,
        session: TabDragSession,
    },
    #[serde(rename = "external")]
    External {
        owner: TabDragExternalOwner,
        #[serde(rename = "overWindow")]
        over_window: String,
        #[serde(rename = "overPoint")]
        over_point: (Number, Number),
        insert: Option<Insert>,
    },
}

impl Default for TabDrag {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DropTarget {
    #[serde(rename = "tabId")]
    pub tab_id: String,
    pub side: crate::splits::SplitSide,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Insert {
    #[serde(rename = "windowId")]
    pub window_id: String,
    #[serde(rename = "beforeId")]
    pub before_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TabDragSession {
    pub seq: Number,
    #[serde(rename = "liveFrame")]
    pub live_frame: Option<(Number, Number, Number, Number)>,
    pub pointer: (Number, Number),
    pub picture: Option<Picture>,
    #[serde(rename = "sentShape")]
    pub sent_shape: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Picture {
    pub grab: (Number, Number),
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TabDragExternalOwner {
    #[serde(rename = "windowId")]
    pub window_id: String,
    pub token: Number,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Drop {
    #[serde(rename = "tabs")]
    Tabs {
        placement: crate::pins::TabPlacement,
    },
    #[serde(rename = "group")]
    Group {
        #[serde(rename = "groupId")]
        group_id: String,
        placement: crate::pins::GroupPlacement,
    },
    #[serde(rename = "live")]
    Live,
    #[serde(rename = "split")]
    Split {
        #[serde(rename = "targetId")]
        target_id: String,
        side: SplitDropSide,
    },
    #[serde(rename = "folder")]
    Folder {
        #[serde(rename = "ontoId")]
        onto_id: String,
        #[serde(rename = "beforeId", default, skip_serializing_if = "Option::is_none")]
        before_id: Option<String>,
    },
    #[serde(rename = "none")]
    None,
}

impl Default for Drop {
    fn default() -> Self {
        Self::Tabs {
            placement: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SplitDropSide {
    #[default]
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StatePhase {
    #[default]
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "external")]
    External,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DroppedEntry {
    #[serde(rename = "file")]
    File { url: String },
    #[serde(rename = "url")]
    Url { url: String },
    #[serde(rename = "text")]
    Text { text: String },
}

impl Default for DroppedEntry {
    fn default() -> Self {
        Self::File {
            url: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickMods {
    #[serde(rename = "metaKey", default, skip_serializing_if = "Option::is_none")]
    pub meta_key: Option<bool>,
    #[serde(rename = "shiftKey", default, skip_serializing_if = "Option::is_none")]
    pub shift_key: Option<bool>,
    #[serde(rename = "altKey", default, skip_serializing_if = "Option::is_none")]
    pub alt_key: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DragSource {
    pub kind: DragSourceKind,
    #[serde(rename = "groupId", default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(rename = "folderId", default, skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DragSourceKind {
    #[default]
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "live")]
    Live,
    #[serde(rename = "group")]
    Group,
    #[serde(rename = "split")]
    Split,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "zone")]
pub enum DropSpot {
    #[serde(rename = "tiles")]
    Tiles {
        #[serde(rename = "beforeId")]
        before_id: Option<String>,
    },
    #[serde(rename = "pins")]
    Pins {
        #[serde(rename = "beforeId")]
        before_id: Option<String>,
    },
    #[serde(rename = "pinnedRows")]
    PinnedRows {
        #[serde(rename = "beforeId")]
        before_id: Option<String>,
    },
    #[serde(rename = "list")]
    List {
        #[serde(rename = "beforeId")]
        before_id: Option<String>,
    },
    #[serde(rename = "group")]
    Group {
        #[serde(rename = "groupId")]
        group_id: String,
        #[serde(rename = "beforeId")]
        before_id: Option<String>,
    },
    #[serde(rename = "split")]
    Split {
        #[serde(rename = "targetId")]
        target_id: String,
        side: SplitDropSide,
    },
    #[serde(rename = "folder")]
    Folder {
        #[serde(rename = "ontoId")]
        onto_id: String,
        #[serde(rename = "beforeId", default, skip_serializing_if = "Option::is_none")]
        before_id: Option<String>,
    },
}

impl Default for DropSpot {
    fn default() -> Self {
        Self::Tiles {
            before_id: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Slot {
    #[serde(rename = "tabIds")]
    pub tab_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Lane {
    pub pinned: bool,
    #[serde(rename = "groupId")]
    pub group_id: Option<String>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub phase: StatePhase,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `beginExternalDrag` (an exported function that writes a store).
    #[serde(rename = "beginExternalDrag")]
    BeginExternalDrag {
        #[serde(rename = "windowId")]
        window_id: String,
        point: (Number, Number),
        token: Number,
    },
    /// `beginTabDrag` (an exported function that writes a store).
    #[serde(rename = "beginTabDrag")]
    BeginTabDrag {
        #[serde(rename = "tabId")]
        tab_id: Option<String>,
        #[serde(rename = "tabIds", default)]
        tab_ids: Option<Vec<String>>,
    },
    /// `cancelTabDrag` (an exported function that writes a store).
    #[serde(rename = "cancelTabDrag")]
    CancelTabDrag,
    /// `dropTabsOutside` (an exported function that writes a store). The callback parameters `onNewWindow` are not in the JSON.
    #[serde(rename = "dropTabsOutside")]
    DropTabsOutside {
        #[serde(rename = "tabIds")]
        tab_ids: Vec<String>,
        source: String,
        x: Number,
        y: Number,
        #[serde(default)]
        live: Option<(Number, Number, Number, Number)>,
    },
    /// `endExternalDrag` (an exported function that writes a store).
    #[serde(rename = "endExternalDrag")]
    EndExternalDrag {
        #[serde(rename = "windowId")]
        window_id: String,
        point: Option<(Number, Number)>,
        token: Number,
    },
    /// `endTabDrag` (an exported function that writes a store).
    #[serde(rename = "endTabDrag")]
    EndTabDrag,
    /// `moveExternalDrag` (an exported function that writes a store).
    #[serde(rename = "moveExternalDrag")]
    MoveExternalDrag {
        #[serde(rename = "windowId")]
        window_id: String,
        point: (Number, Number),
        token: Number,
    },
    /// `openDroppedInTab` (an exported function that writes a store).
    #[serde(rename = "openDroppedInTab")]
    OpenDroppedInTab {
        #[serde(rename = "tabId")]
        tab_id: String,
        items: Vec<DroppedEntry>,
    },
    /// `openDroppedUrls` (an exported function that writes a store).
    #[serde(rename = "openDroppedUrls")]
    OpenDroppedUrls {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "profileId")]
        profile_id: String,
        placement: Option<crate::pins::TabPlacement>,
        urls: Vec<String>,
    },
    /// `setDropTarget` (an exported function that writes a store).
    #[serde(rename = "setDropTarget")]
    SetDropTarget { offered: Option<DropTarget> },
    /// `setInsert` (an exported function that writes a store).
    #[serde(rename = "setInsert")]
    SetInsert {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "beforeId")]
        before_id: Option<String>,
    },
    /// `setOnPage` (an exported function that writes a store).
    #[serde(rename = "setOnPage")]
    SetOnPage {
        #[serde(rename = "onPage")]
        on_page: bool,
    },
    /// `updateTabDrag` (an exported function that writes a store).
    #[serde(rename = "updateTabDrag")]
    UpdateTabDrag {
        x: Number,
        y: Number,
        #[serde(default)]
        lifted: Option<bool>,
    },
}

/// Applies `action` to `app`. Slice 2.32 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.32")
}
