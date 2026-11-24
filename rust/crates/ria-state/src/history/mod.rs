// Ported from ardia apps/browser/src/state/history/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `history` (`apps/browser/src/state/history`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.27 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HistoryView {
    pub list: Vec<crate::browser::HistoryEntry>,
    pub floor: Number,
    pub ready: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ImportHistoryEntriesItem {
    pub url: String,
    pub title: String,
    #[serde(
        rename = "visitTimes",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub visit_times: Option<Vec<Number>>,
    pub visits: Number,
    #[serde(rename = "lastVisit")]
    pub last_visit: Number,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub history: std::collections::HashMap<String, HistoryView>,
    #[serde(rename = "tabId")]
    pub tab_id: Option<String>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `clearHistory` (a store method).
    #[serde(rename = "clearHistory")]
    ClearHistory {
        #[serde(rename = "profileId")]
        profile_id: String,
        #[serde(default)]
        since: Option<Number>,
        #[serde(default)]
        until: Option<Number>,
    },
    /// `importCleanedTabs` (a store method).
    #[serde(rename = "importCleanedTabs")]
    ImportCleanedTabs {
        entries: Vec<crate::browser::ClosedTab>,
    },
    /// `importHistory` (a store method).
    #[serde(rename = "importHistory")]
    ImportHistory {
        #[serde(rename = "profileId")]
        profile_id: String,
        entries: Vec<ImportHistoryEntriesItem>,
    },
    /// `openHistoryRowMenu` (an exported function that writes a store). The callback parameters `onMoreFromSite`, `onRemove` are not in the JSON.
    #[serde(rename = "openHistoryRowMenu")]
    OpenHistoryRowMenu {
        entries: Vec<crate::bookmarks::Link>,
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `reloadEngineHistory` (an exported function that writes a store).
    #[serde(rename = "reloadEngineHistory")]
    ReloadEngineHistory { engine: String },
    /// `removeHistory` (a store method).
    #[serde(rename = "removeHistory")]
    RemoveHistory {
        #[serde(rename = "profileId")]
        profile_id: String,
        urls: Vec<String>,
    },
    /// `showHistory` (an exported function that writes a store).
    #[serde(rename = "showHistory")]
    ShowHistory {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `startHistory` (an exported function that writes a store).
    #[serde(rename = "startHistory")]
    StartHistory,
    /// `takeHistorySearchFocusRequest` (an exported function that writes a store).
    #[serde(rename = "takeHistorySearchFocusRequest")]
    TakeHistorySearchFocusRequest,
}

/// Applies `action` to `app`. Slice 2.27 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.27")
}
