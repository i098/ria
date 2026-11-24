// Ported from ardia apps/browser/src/state/peek/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `peek` (`apps/browser/src/state/peek`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.19 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Peek {
    #[serde(rename = "tabId")]
    pub tab_id: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "sourcePage")]
    pub source_page: String,
    pub from: Option<PeekPoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landing: Option<SiteLanding>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PeekPoint {
    pub x: Number,
    pub y: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SiteLanding {
    pub host: String,
    pub until: Number,
    pub then: SiteLandingThen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SiteLandingThen {
    #[default]
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "source")]
    Source,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PeekOptions {
    #[serde(rename = "adoptId", default, skip_serializing_if = "Option::is_none")]
    pub adopt_id: Option<PeekOptionsAdoptId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landing: Option<SiteLanding>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PeekOptionsAdoptId {
    Other0(String),
    Other1(String),
    Other2(String),
    Other3(String),
    Other4(String),
    Other5(String),
}

impl Default for PeekOptionsAdoptId {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PeekToTabOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activate: Option<bool>,
    #[serde(rename = "sourceId", default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub peeks: std::collections::HashMap<String, Peek>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `closePeek` (a store method).
    #[serde(rename = "closePeek")]
    ClosePeek {
        #[serde(rename = "sourceTabId")]
        source_tab_id: String,
    },
    /// `closePeekOnEscape` (an exported function that writes a store).
    #[serde(rename = "closePeekOnEscape")]
    ClosePeekOnEscape {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `dismissPeek` (an exported function that writes a store).
    #[serde(rename = "dismissPeek")]
    DismissPeek {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `endPeekToolbarPreview` (an exported function that writes a store).
    #[serde(rename = "endPeekToolbarPreview")]
    EndPeekToolbarPreview {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `finishPeek` (an exported function that writes a store).
    #[serde(rename = "finishPeek")]
    FinishPeek {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `holdPeekDonor` (an exported function that writes a store).
    #[serde(rename = "holdPeekDonor")]
    HoldPeekDonor {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(rename = "sourceId")]
        source_id: String,
    },
    /// `notePeekRecipient` (an exported function that writes a store).
    #[serde(rename = "notePeekRecipient")]
    NotePeekRecipient {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `openPeek` (a store method).
    #[serde(rename = "openPeek")]
    OpenPeek {
        #[serde(rename = "sourceTabId")]
        source_tab_id: String,
        url: String,
        #[serde(default)]
        from: Option<PeekPoint>,
        #[serde(default)]
        options: Option<PeekOptions>,
    },
    /// `openPeekAsTab` (an exported function that writes a store).
    #[serde(rename = "openPeekAsTab")]
    OpenPeekAsTab {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `openPeekInSplit` (an exported function that writes a store).
    #[serde(rename = "openPeekInSplit")]
    OpenPeekInSplit {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `peekToTab` (a store method).
    #[serde(rename = "peekToTab")]
    PeekToTab {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(default)]
        options: Option<PeekToTabOptions>,
    },
    /// `previewPeekToolbar` (an exported function that writes a store).
    #[serde(rename = "previewPeekToolbar")]
    PreviewPeekToolbar {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(rename = "sourceId")]
        source_id: String,
    },
    /// `promotePeekNow` (an exported function that writes a store).
    #[serde(rename = "promotePeekNow")]
    PromotePeekNow {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `releasePeekDonor` (an exported function that writes a store).
    #[serde(rename = "releasePeekDonor")]
    ReleasePeekDonor {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `runPeekCommand` (an exported function that writes a store).
    #[serde(rename = "runPeekCommand")]
    RunPeekCommand {
        command: String,
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `startPeeks` (an exported function that writes a store).
    #[serde(rename = "startPeeks")]
    StartPeeks,
}

/// Applies `action` to `app`. Slice 2.19 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.19")
}
