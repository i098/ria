// Ported from ardia apps/browser/src/state/media/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `media` (`apps/browser/src/state/media`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.30 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Entry {
    pub uri: Option<String>,
    pub at: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TabShare {
    #[serde(rename = "capturedTabId")]
    pub captured_tab_id: String,
    pub origin: String,
    pub dismissed: std::collections::HashMap<String, bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SkipOrTrackDirection {
    #[default]
    #[serde(rename = "back")]
    Back,
    #[serde(rename = "ahead")]
    Ahead,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub dialogs: std::collections::HashMap<String, serde_json::Value>,
    pub dismissed: std::collections::HashMap<String, bool>,
    #[serde(rename = "displayRequests")]
    pub display_requests: std::collections::HashMap<String, serde_json::Value>,
    pub entries: std::collections::HashMap<String, Entry>,
    #[serde(rename = "leftPaused")]
    pub left_paused: std::collections::HashMap<String, bool>,
    pub pip: std::collections::HashMap<String, String>,
    #[serde(rename = "pipOpen")]
    pub pip_open: std::collections::HashMap<String, serde_json::Value>,
    pub routes: std::collections::HashMap<String, Vec<serde_json::Value>>,
    pub sessions: std::collections::HashMap<String, serde_json::Value>,
    #[serde(rename = "sharedTabs")]
    pub shared_tabs: std::collections::HashMap<String, bool>,
    #[serde(rename = "tabShares")]
    pub tab_shares: std::collections::HashMap<String, TabShare>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `answerDisplayMedia` (an exported function that writes a store).
    #[serde(rename = "answerDisplayMedia")]
    AnswerDisplayMedia {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(rename = "sourceId")]
        source_id: Option<String>,
        #[serde(default)]
        audio: Option<bool>,
        #[serde(default)]
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `cancelDisplayMediaOnNavigation` (an exported function that writes a store).
    #[serde(rename = "cancelDisplayMediaOnNavigation")]
    CancelDisplayMediaOnNavigation,
    /// `dismissPlayer` (an exported function that writes a store).
    #[serde(rename = "dismissPlayer")]
    DismissPlayer {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `dismissShareBar` (an exported function that writes a store).
    #[serde(rename = "dismissShareBar")]
    DismissShareBar {
        capturer: String,
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `exitPictureInPicture` (an exported function that writes a store).
    #[serde(rename = "exitPictureInPicture")]
    ExitPictureInPicture {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `noteTabShare` (an exported function that writes a store).
    #[serde(rename = "noteTabShare")]
    NoteTabShare {
        #[serde(rename = "capturerTabId")]
        capturer_tab_id: String,
        #[serde(rename = "capturedTabId")]
        captured_tab_id: String,
        origin: String,
    },
    /// `requestDisplayMedia` (an exported function that writes a store).
    #[serde(rename = "requestDisplayMedia")]
    RequestDisplayMedia {
        #[serde(rename = "tabId")]
        tab_id: String,
        request: serde_json::Value,
    },
    /// `seekTo` (an exported function that writes a store).
    #[serde(rename = "seekTo")]
    SeekTo {
        #[serde(rename = "tabId")]
        tab_id: String,
        session: serde_json::Value,
        seconds: Number,
    },
    /// `setNowPlaying` (an exported function that writes a store).
    #[serde(rename = "setNowPlaying")]
    SetNowPlaying {
        #[serde(rename = "tabId")]
        tab_id: String,
        state: serde_json::Value,
    },
    /// `setPictureInPictureState` (an exported function that writes a store).
    #[serde(rename = "setPictureInPictureState")]
    SetPictureInPictureState {
        #[serde(rename = "tabId")]
        tab_id: String,
        state: serde_json::Value,
    },
    /// `shareInstead` (an exported function that writes a store).
    #[serde(rename = "shareInstead")]
    ShareInstead {
        capturer: String,
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `shareTab` (an exported function that writes a store).
    #[serde(rename = "shareTab")]
    ShareTab {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(rename = "sharedTabId")]
        shared_tab_id: String,
        audio: bool,
    },
    /// `skipBy` (an exported function that writes a store).
    #[serde(rename = "skipBy")]
    SkipBy {
        #[serde(rename = "tabId")]
        tab_id: String,
        session: serde_json::Value,
        seconds: Number,
    },
    /// `skipOrTrack` (an exported function that writes a store).
    #[serde(rename = "skipOrTrack")]
    SkipOrTrack {
        #[serde(rename = "tabId")]
        tab_id: String,
        session: serde_json::Value,
        direction: SkipOrTrackDirection,
    },
    /// `startCast` (an exported function that writes a store).
    #[serde(rename = "startCast")]
    StartCast,
    /// `startMedia` (an exported function that writes a store).
    #[serde(rename = "startMedia")]
    StartMedia,
    /// `startTabShareCleanup` (an exported function that writes a store).
    #[serde(rename = "startTabShareCleanup")]
    StartTabShareCleanup,
}

/// Applies `action` to `app`. Slice 2.30 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.30")
}
