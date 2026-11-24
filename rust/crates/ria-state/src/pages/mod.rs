// Ported from ardia apps/browser/src/state/pages/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `pages` (`apps/browser/src/state/pages`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.24 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct IsInternalTabT {
    pub url: String,
    pub page: crate::browser::TabPage,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OpenInternalPageOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(rename = "windowId")]
    pub window_id: Option<String>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `closePersonalizePanelOf` (an exported function that writes a store).
    #[serde(rename = "closePersonalizePanelOf")]
    ClosePersonalizePanelOf {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `isAppPage` (an exported function that writes a store).
    #[serde(rename = "isAppPage")]
    IsAppPage { url: Option<String> },
    /// `isInternalTab` (an exported function that writes a store).
    #[serde(rename = "isInternalTab")]
    IsInternalTab { t: IsInternalTabT },
    /// `openChromeSettings` (an exported function that writes a store).
    #[serde(rename = "openChromeSettings")]
    OpenChromeSettings,
    /// `openInternalPage` (an exported function that writes a store).
    #[serde(rename = "openInternalPage")]
    OpenInternalPage {
        page: serde_json::Value,
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
        #[serde(default)]
        options: Option<OpenInternalPageOptions>,
    },
    /// `setPersonalizePanel` (an exported function that writes a store).
    #[serde(rename = "setPersonalizePanel")]
    SetPersonalizePanel {
        #[serde(rename = "windowId")]
        window_id: Option<String>,
    },
}

/// Applies `action` to `app`. Slice 2.24 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.24")
}
