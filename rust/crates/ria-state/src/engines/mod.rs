// Ported from ardia apps/browser/src/state/engines/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `engines` (`apps/browser/src/state/engines`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.21 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StatePending {
    #[default]
    #[serde(rename = "automatic")]
    Automatic,
    #[serde(rename = "chromium")]
    Chromium,
    #[serde(rename = "webkit")]
    Webkit,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Swap {
    pub from: serde_json::Value,
    pub to: serde_json::Value,
    pub scroll: Option<SwapScroll>,
    pub zoom: bool,
    #[serde(rename = "startedAt")]
    pub started_at: Number,
    pub consent: Option<crate::telemetry::ConsentToken>,
    pub watching: bool,
    pub visible: bool,
    pub deadline: Option<Number>,
    pub held: HeldLive,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SwapScroll {
    pub x: Number,
    pub y: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HeldLive {
    #[serde(rename = "isLoading", default, skip_serializing_if = "Option::is_none")]
    pub is_loading: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<Number>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MoveOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<serde_json::Value>,
    #[serde(rename = "checkWork", default, skip_serializing_if = "Option::is_none")]
    pub check_work: Option<bool>,
    #[serde(rename = "mayWrite", default, skip_serializing_if = "Option::is_none")]
    pub may_write: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetEngineModeMode {
    #[default]
    #[serde(rename = "automatic")]
    Automatic,
    #[serde(rename = "chromium")]
    Chromium,
    #[serde(rename = "webkit")]
    Webkit,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SetEngineModeOptions {
    #[serde(rename = "windowId", default, skip_serializing_if = "Option::is_none")]
    pub window_id: Option<String>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub pending: Option<StatePending>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `acceptEngineOffer` (an exported function that writes a store).
    #[serde(rename = "acceptEngineOffer")]
    AcceptEngineOffer {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `clearEngineOffer` (an exported function that writes a store).
    #[serde(rename = "clearEngineOffer")]
    ClearEngineOffer {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `dropSwap` (an exported function that writes a store).
    #[serde(rename = "dropSwap")]
    DropSwap {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(default)]
        only: Option<Swap>,
    },
    /// `followSiteChoice` (an exported function that writes a store).
    #[serde(rename = "followSiteChoice")]
    FollowSiteChoice {
        #[serde(rename = "tabId")]
        tab_id: String,
        engine: serde_json::Value,
        url: String,
        #[serde(rename = "previousUrl")]
        previous_url: String,
    },
    /// `goBackAcrossEngines` (an exported function that writes a store). The callback parameters `mayAct`, `ask` are not in the JSON.
    #[serde(rename = "goBackAcrossEngines")]
    GoBackAcrossEngines {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `holdPageForSwap` (an exported function that writes a store). The callback parameters `stillWanted` are not in the JSON.
    #[serde(rename = "holdPageForSwap")]
    HoldPageForSwap {
        #[serde(rename = "tabId")]
        tab_id: String,
        from: serde_json::Value,
        to: serde_json::Value,
        url: String,
        #[serde(rename = "keepScroll")]
        keep_scroll: bool,
    },
    /// `offerWebKit` (an exported function that writes a store).
    #[serde(rename = "offerWebKit")]
    OfferWebKit {
        #[serde(rename = "tabId")]
        tab_id: String,
        __1: serde_json::Value,
    },
    /// `openPageIn` (an exported function that writes a store).
    #[serde(rename = "openPageIn")]
    OpenPageIn {
        #[serde(rename = "tabId")]
        tab_id: String,
        engine: serde_json::Value,
        #[serde(default)]
        options: Option<MoveOptions>,
    },
    /// `persistEngine` (an exported function that writes a store).
    #[serde(rename = "persistEngine")]
    PersistEngine {
        #[serde(rename = "tabId")]
        tab_id: String,
        engine: serde_json::Value,
    },
    /// `setEngineMode` (an exported function that writes a store).
    #[serde(rename = "setEngineMode")]
    SetEngineMode {
        mode: SetEngineModeMode,
        #[serde(default)]
        options: Option<SetEngineModeOptions>,
    },
    /// `setWebsiteEngine` (an exported function that writes a store).
    #[serde(rename = "setWebsiteEngine")]
    SetWebsiteEngine {
        url: String,
        engine: serde_json::Value,
    },
    /// `stayOnPage` (an exported function that writes a store).
    #[serde(rename = "stayOnPage")]
    StayOnPage {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(default)]
        remember: Option<bool>,
    },
    /// `togglePageEngine` (an exported function that writes a store). The callback parameters `mayWrite` are not in the JSON.
    #[serde(rename = "togglePageEngine")]
    TogglePageEngine {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `watchFirstPaint` (an exported function that writes a store).
    #[serde(rename = "watchFirstPaint")]
    WatchFirstPaint {
        #[serde(rename = "tabId")]
        tab_id: String,
        engine: serde_json::Value,
        visible: bool,
    },
}

/// Applies `action` to `app`. Slice 2.21 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.21")
}
