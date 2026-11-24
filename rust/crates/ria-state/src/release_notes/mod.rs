// Ported from ardia apps/browser/src/state/releaseNotes/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `releaseNotes` (`apps/browser/src/state/releaseNotes`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.44 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StatePage {
    #[serde(rename = "windowId")]
    pub window_id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StatePending {
    pub version: String,
    pub since: Number,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub page: Option<StatePage>,
    pub pending: Option<StatePending>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `closeReleaseNotes` (an exported function that writes a store).
    #[serde(rename = "closeReleaseNotes")]
    CloseReleaseNotes,
    /// `openReleaseNotes` (an exported function that writes a store).
    #[serde(rename = "openReleaseNotes")]
    OpenReleaseNotes {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(default)]
        version: Option<serde_json::Value>,
    },
    /// `openReleaseNotesAfterUpdate` (an exported function that writes a store).
    #[serde(rename = "openReleaseNotesAfterUpdate")]
    OpenReleaseNotesAfterUpdate,
    /// `openReleaseNotesPage` (an exported function that writes a store).
    #[serde(rename = "openReleaseNotesPage")]
    OpenReleaseNotesPage {
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `retireReleaseNotes` (an exported function that writes a store).
    #[serde(rename = "retireReleaseNotes")]
    RetireReleaseNotes,
    /// `trackAppVersion` (an exported function that writes a store).
    #[serde(rename = "trackAppVersion")]
    TrackAppVersion,
}

/// Applies `action` to `app`. Slice 2.44 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.44")
}
