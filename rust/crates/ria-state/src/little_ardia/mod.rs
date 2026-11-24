// Ported from ardia apps/browser/src/state/littleArdia/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `littleArdia` (`apps/browser/src/state/littleArdia`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.36 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OpenExternalUrlsSource {
    #[serde(rename = "bundleId")]
    pub bundle_id: Option<String>,
    #[serde(rename = "appName")]
    pub app_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OpenLittleArdiaAdoptId {
    Other0(String),
    Other1(String),
    Other2(String),
    Other3(String),
    Other4(String),
    Other5(String),
}

impl Default for OpenLittleArdiaAdoptId {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `closeBlankLittleArdiaOnEscape` (an exported function that writes a store).
    #[serde(rename = "closeBlankLittleArdiaOnEscape")]
    CloseBlankLittleArdiaOnEscape {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `closeLittleArdia` (an exported function that writes a store).
    #[serde(rename = "closeLittleArdia")]
    CloseLittleArdia {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(default)]
        arg1: Option<crate::windows::CloseGuard>,
    },
    /// `closeLittleArdiaForAppLink` (an exported function that writes a store).
    #[serde(rename = "closeLittleArdiaForAppLink")]
    CloseLittleArdiaForAppLink {
        #[serde(rename = "windowId")]
        window_id: Option<String>,
    },
    /// `closeLittleArdiaOnEscape` (an exported function that writes a store).
    #[serde(rename = "closeLittleArdiaOnEscape")]
    CloseLittleArdiaOnEscape {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(rename = "pressedIn", default)]
        pressed_in: Option<crate::telemetry::EscapePressedIn>,
    },
    /// `cycleLittleArdiaProfile` (an exported function that writes a store).
    #[serde(rename = "cycleLittleArdiaProfile")]
    CycleLittleArdiaProfile {
        #[serde(rename = "windowId")]
        window_id: String,
        delta: Number,
    },
    /// `moveLittleArdiaToProfile` (an exported function that writes a store).
    #[serde(rename = "moveLittleArdiaToProfile")]
    MoveLittleArdiaToProfile {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    /// `openExternalUrls` (an exported function that writes a store).
    #[serde(rename = "openExternalUrls")]
    OpenExternalUrls {
        given: Vec<String>,
        #[serde(default)]
        pointer: Option<(Number, Number)>,
        #[serde(default)]
        source: Option<OpenExternalUrlsSource>,
        #[serde(rename = "returnTo", default)]
        return_to: Option<String>,
    },
    /// `openInMainWindow` (an exported function that writes a store).
    #[serde(rename = "openInMainWindow")]
    OpenInMainWindow {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `openLittleArdia` (an exported function that writes a store).
    #[serde(rename = "openLittleArdia")]
    OpenLittleArdia {
        #[serde(default)]
        url: Option<String>,
        #[serde(rename = "profileId", default)]
        profile_id: Option<String>,
        #[serde(rename = "adoptId", default)]
        adopt_id: Option<OpenLittleArdiaAdoptId>,
        #[serde(rename = "linkPointer", default)]
        link_pointer: Option<(Number, Number)>,
        #[serde(rename = "linkApp", default)]
        link_app: Option<String>,
    },
    /// `openLittleArdiaProfileMenu` (an exported function that writes a store).
    #[serde(rename = "openLittleArdiaProfileMenu")]
    OpenLittleArdiaProfileMenu {
        #[serde(rename = "windowId")]
        window_id: String,
    },
}

/// Applies `action` to `app`. Slice 2.36 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.36")
}
