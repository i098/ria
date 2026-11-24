// Ported from ardia apps/browser/src/state/sharedFolders/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `sharedFolders` (`apps/browser/src/state/sharedFolders`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.41 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SharedFolder {
    pub path: String,
    pub base: Vec<BaseEntry>,
    #[serde(rename = "addedAt")]
    pub added_at: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum BaseEntry {
    #[serde(rename = "link")]
    Link {
        key: String,
        file: String,
        name: String,
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    #[serde(rename = "dir")]
    Dir {
        key: String,
        file: String,
        name: String,
        children: Vec<Box<BaseEntry>>,
    },
}

impl Default for BaseEntry {
    fn default() -> Self {
        Self::Link {
            key: Default::default(),
            file: Default::default(),
            name: Default::default(),
            url: Default::default(),
            id: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SharedFolderSync {
    pub state: SharedFolderSyncState,
    pub waiting: Number,
    pub error: Option<String>,
    pub at: Number,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SharedFolderSyncState {
    #[default]
    #[serde(rename = "missing")]
    Missing,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "syncing")]
    Syncing,
    #[serde(rename = "synced")]
    Synced,
    #[serde(rename = "paused")]
    Paused,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SetFolderSyncPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<SetFolderSyncPatchState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiting: Option<Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Number>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetFolderSyncPatchState {
    #[default]
    #[serde(rename = "missing")]
    Missing,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "syncing")]
    Syncing,
    #[serde(rename = "synced")]
    Synced,
    #[serde(rename = "paused")]
    Paused,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub folders: std::collections::HashMap<String, std::collections::HashMap<String, SharedFolder>>,
    pub sync: std::collections::HashMap<String, SharedFolderSync>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `addSharedFolder` (an exported function that writes a store).
    #[serde(rename = "addSharedFolder")]
    AddSharedFolder {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `bindFolder` (an exported function that writes a store).
    #[serde(rename = "bindFolder")]
    BindFolder {
        #[serde(rename = "profileId")]
        profile_id: String,
        key: String,
        path: String,
        #[serde(default)]
        identity: Option<serde_json::Value>,
    },
    /// `bindFolderIfUnbound` (an exported function that writes a store).
    #[serde(rename = "bindFolderIfUnbound")]
    BindFolderIfUnbound {
        #[serde(rename = "profileId")]
        profile_id: String,
        key: String,
        folder: serde_json::Value,
    },
    /// `bindingAt` (an exported function that writes a store).
    #[serde(rename = "bindingAt")]
    BindingAt {
        #[serde(rename = "profileId")]
        profile_id: String,
        found: serde_json::Value,
    },
    /// `runSharedFolderChoice` (an exported function that writes a store).
    #[serde(rename = "runSharedFolderChoice")]
    RunSharedFolderChoice {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "groupId")]
        group_id: String,
        choice: String,
    },
    /// `setFolderBase` (an exported function that writes a store).
    #[serde(rename = "setFolderBase")]
    SetFolderBase {
        #[serde(rename = "profileId")]
        profile_id: String,
        key: String,
        base: Vec<BaseEntry>,
    },
    /// `setFolderSync` (an exported function that writes a store).
    #[serde(rename = "setFolderSync")]
    SetFolderSync {
        #[serde(rename = "profileId")]
        profile_id: String,
        key: String,
        patch: SetFolderSyncPatch,
    },
    /// `shareFolder` (an exported function that writes a store).
    #[serde(rename = "shareFolder")]
    ShareFolder {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "groupId")]
        group_id: String,
    },
    /// `startSharedFolders` (an exported function that writes a store).
    #[serde(rename = "startSharedFolders")]
    StartSharedFolders,
    /// `stopSyncingGroup` (an exported function that writes a store).
    #[serde(rename = "stopSyncingGroup")]
    StopSyncingGroup {
        #[serde(rename = "groupId")]
        group_id: String,
    },
    /// `syncAllFolders` (an exported function that writes a store).
    #[serde(rename = "syncAllFolders")]
    SyncAllFolders,
    /// `syncFolderAfterNow` (an exported function that writes a store). The callback parameters `allowed` are not in the JSON.
    #[serde(rename = "syncFolderAfterNow")]
    SyncFolderAfterNow {
        #[serde(rename = "profileId")]
        profile_id: String,
        key: String,
    },
    /// `syncFolderNow` (an exported function that writes a store). The callback parameters `allowed` are not in the JSON.
    #[serde(rename = "syncFolderNow")]
    SyncFolderNow {
        #[serde(rename = "profileId")]
        profile_id: String,
        key: String,
    },
    /// `unbindFolder` (an exported function that writes a store).
    #[serde(rename = "unbindFolder")]
    UnbindFolder {
        #[serde(rename = "profileId")]
        profile_id: String,
        key: String,
    },
    /// `upgradeFolder` (an exported function that writes a store).
    #[serde(rename = "upgradeFolder")]
    UpgradeFolder {
        #[serde(rename = "profileId")]
        profile_id: String,
        key: String,
        was: SharedFolder,
        found: serde_json::Value,
    },
    /// `upgradeLegacy` (an exported function that writes a store).
    #[serde(rename = "upgradeLegacy")]
    UpgradeLegacy {
        #[serde(rename = "profileId")]
        profile_id: String,
        key: String,
    },
}

/// Applies `action` to `app`. Slice 2.41 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.41")
}
