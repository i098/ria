// Ported from ardia apps/browser/src/state/pins/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `pins` (`apps/browser/src/state/pins`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.13 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PinSyncEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub g: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w: Option<Number>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TabPlacement {
    pub pinned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row: Option<bool>,
    #[serde(rename = "beforeId", default, skip_serializing_if = "Option::is_none")]
    pub before_id: Option<String>,
    #[serde(rename = "groupId", default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<bool>,
    #[serde(
        rename = "tileFolder",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub tile_folder: Option<crate::browser::TileFolder>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GroupPlacement {
    pub pinned: bool,
    #[serde(rename = "beforeId", default, skip_serializing_if = "Option::is_none")]
    pub before_id: Option<String>,
    #[serde(rename = "parentId", default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Bridge {
    #[serde(rename = "copyId")]
    pub copy_id: String,
    pub uri: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CleanUpTabsOptions {
    #[serde(
        rename = "inactiveForMs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub inactive_for_ms: Option<Number>,
    #[serde(
        rename = "allProfiles",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub all_profiles: Option<bool>,
    #[serde(
        rename = "hiddenProfilesUpTo",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub hidden_profiles_up_to: Option<Number>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RestoreCleanedEntryIds {
    String(String),
    Other1(Vec<String>),
}

impl Default for RestoreCleanedEntryIds {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClearedTabs {
    #[serde(rename = "windowId")]
    pub window_id: String,
    pub entries: Vec<crate::browser::ClosedTab>,
    #[serde(rename = "closedEntryIds")]
    pub closed_entry_ids: Vec<String>,
    #[serde(rename = "tabIds")]
    pub tab_ids: Vec<String>,
    pub groups: Vec<crate::browser::TabGroup>,
    pub fingerprint: String,
    #[serde(rename = "profileId")]
    pub profile_id: String,
    #[serde(rename = "remoteDeletions")]
    pub remote_deletions: Number,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TidyScope {
    #[default]
    #[serde(rename = "tabs")]
    Tabs,
    #[serde(rename = "pins")]
    Pins,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub bridges: std::collections::HashMap<String, Bridge>,
    #[serde(rename = "cleanedTabs")]
    pub cleaned_tabs: Vec<crate::browser::ClosedTab>,
    pub frames: std::collections::HashMap<String, String>,
    #[serde(rename = "pinChoices")]
    pub pin_choices:
        std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>>,
    #[serde(rename = "pinSync")]
    pub pin_sync:
        std::collections::HashMap<String, std::collections::HashMap<String, PinSyncEntry>>,
    #[serde(rename = "privateCleanedTabs")]
    pub private_cleaned_tabs: std::collections::HashMap<String, Vec<crate::browser::ClosedTab>>,
    pub selection: std::collections::HashMap<String, Vec<String>>,
    #[serde(rename = "selectionAnchors")]
    pub selection_anchors: std::collections::HashMap<String, crate::drag::SelectionAnchor>,
    #[serde(rename = "topAppsMerged")]
    pub top_apps_merged: String,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `cleanUpTabs` (an exported function that writes a store).
    #[serde(rename = "cleanUpTabs")]
    CleanUpTabs {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(default)]
        options: Option<CleanUpTabsOptions>,
    },
    /// `clearUnpinnedTabs` (a store method).
    #[serde(rename = "clearUnpinnedTabs")]
    ClearUnpinnedTabs {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `clickTab` (a store method).
    #[serde(rename = "clickTab")]
    ClickTab {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(default)]
        mods: Option<crate::drag::ClickMods>,
    },
    /// `closeAbandonedNewTabs` (an exported function that writes a store).
    #[serde(rename = "closeAbandonedNewTabs")]
    CloseAbandonedNewTabs,
    /// `dropTabs` (an exported function that writes a store).
    #[serde(rename = "dropTabs")]
    DropTabs {
        ids: Vec<String>,
        drop: crate::drag::Drop,
    },
    /// `handOffPinPage` (an exported function that writes a store). The callback parameters `commit` are not in the JSON.
    #[serde(rename = "handOffPinPage")]
    HandOffPinPage {
        #[serde(rename = "copyId")]
        copy_id: String,
    },
    /// `noteOrderChoices` (an exported function that writes a store).
    #[serde(rename = "noteOrderChoices")]
    NoteOrderChoices {
        s: crate::browser::BrowserState,
        prev: crate::browser::BrowserState,
    },
    /// `pinTabs` (an exported function that writes a store).
    #[serde(rename = "pinTabs")]
    PinTabs {
        ids: Vec<String>,
        pinned: bool,
        #[serde(default)]
        row: Option<bool>,
    },
    /// `placeTabs` (an exported function that writes a store).
    #[serde(rename = "placeTabs")]
    PlaceTabs {
        ids: Vec<String>,
        placement: TabPlacement,
    },
    /// `removeClosedPin` (a store method).
    #[serde(rename = "removeClosedPin")]
    RemoveClosedPin { id: String },
    /// `requestPinHandOff` (an exported function that writes a store). The callback parameters `mayCommit` are not in the JSON.
    #[serde(rename = "requestPinHandOff")]
    RequestPinHandOff {
        #[serde(rename = "copyId")]
        copy_id: String,
    },
    /// `restoreCleaned` (an exported function that writes a store).
    #[serde(rename = "restoreCleaned")]
    RestoreCleaned {
        #[serde(rename = "entryIds", default)]
        entry_ids: Option<RestoreCleanedEntryIds>,
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `returnToPinnedUrl` (an exported function that writes a store).
    #[serde(rename = "returnToPinnedUrl")]
    ReturnToPinnedUrl {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `setPinnedUrl` (an exported function that writes a store).
    #[serde(rename = "setPinnedUrl")]
    SetPinnedUrl {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(default)]
        url: Option<String>,
    },
    /// `setSelection` (an exported function that writes a store).
    #[serde(rename = "setSelection")]
    SetSelection {
        #[serde(rename = "windowId")]
        window_id: String,
        ids: Vec<String>,
    },
    /// `startPinHandoff` (an exported function that writes a store).
    #[serde(rename = "startPinHandoff")]
    StartPinHandoff,
    /// `undoClearTabs` (a store method).
    #[serde(rename = "undoClearTabs")]
    UndoClearTabs { cleared: ClearedTabs },
}

/// Applies `action` to `app`. Slice 2.13 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.13")
}
