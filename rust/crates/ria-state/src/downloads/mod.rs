// Ported from ardia apps/browser/src/state/downloads/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `downloads` (`apps/browser/src/state/downloads`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.34 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub downloads: Vec<serde_json::Value>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `clearDownloads` (a store method).
    #[serde(rename = "clearDownloads")]
    ClearDownloads {
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `downloadMenu` (an exported function that writes a store).
    #[serde(rename = "downloadMenu")]
    DownloadMenu {
        d: serde_json::Value,
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `forgetDownloadProgress` (an exported function that writes a store).
    #[serde(rename = "forgetDownloadProgress")]
    ForgetDownloadProgress { ids: Vec<String> },
    /// `forgetDownloads` (a store method).
    #[serde(rename = "forgetDownloads")]
    ForgetDownloads { profile: String },
    /// `removeDownload` (a store method).
    #[serde(rename = "removeDownload")]
    RemoveDownload { id: String },
    /// `showDownloadProgress` (an exported function that writes a store).
    #[serde(rename = "showDownloadProgress")]
    ShowDownloadProgress { d: serde_json::Value },
    /// `upsertDownload` (a store method).
    #[serde(rename = "upsertDownload")]
    UpsertDownload { d: serde_json::Value },
}

/// Applies `action` to `app`. Slice 2.34 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.34")
}
