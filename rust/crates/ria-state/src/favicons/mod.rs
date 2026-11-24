// Ported from ardia apps/browser/src/state/favicons/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `favicons` (`apps/browser/src/state/favicons`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.26 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Icons {
    pub pages: std::collections::HashMap<String, Option<String>>,
    pub srcs: std::collections::HashMap<String, String>,
    pub appearances: std::collections::HashMap<String, (String, String)>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct KnownFavicon {
    pub page: String,
    pub icon: String,
    pub png: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LaunchFavicons {
    pub images: Vec<String>,
    pub profiles: std::collections::HashMap<String, LaunchFaviconsProfilesValue>,
    pub themes: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LaunchFaviconsProfilesValue {
    pub pages: std::collections::HashMap<String, Number>,
    pub srcs: std::collections::HashMap<String, Number>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Watcher {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    #[serde(rename = "profileId", default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known: Option<bool>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(rename = "byUri")]
    pub by_uri: std::collections::HashMap<String, Option<String>>,
    pub profiles: std::collections::HashMap<String, Icons>,
    pub themes: std::collections::HashMap<String, serde_json::Value>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `adoptFavicons` (an exported function that writes a store).
    #[serde(rename = "adoptFavicons")]
    AdoptFavicons {
        #[serde(rename = "profileId")]
        profile_id: String,
        known: Vec<KnownFavicon>,
    },
    /// `askFaviconTheme` (an exported function that writes a store).
    #[serde(rename = "askFaviconTheme")]
    AskFaviconTheme { uri: String },
    /// `dropOutlivedFavicon` (an exported function that writes a store).
    #[serde(rename = "dropOutlivedFavicon")]
    DropOutlivedFavicon {
        #[serde(rename = "tabId")]
        tab_id: String,
        url: String,
        #[serde(rename = "isLoading")]
        is_loading: bool,
    },
    /// `faviconFailed` (an exported function that writes a store).
    #[serde(rename = "faviconFailed")]
    FaviconFailed {
        #[serde(rename = "profileId")]
        profile_id: String,
        uri: String,
    },
    /// `noteFavicon` (an exported function that writes a store).
    #[serde(rename = "noteFavicon")]
    NoteFavicon {
        #[serde(rename = "tabId")]
        tab_id: String,
        src: String,
    },
    /// `pruneEngineFavicons` (an exported function that writes a store).
    #[serde(rename = "pruneEngineFavicons")]
    PruneEngineFavicons { engine: String },
    /// `seedFavicons` (an exported function that writes a store).
    #[serde(rename = "seedFavicons")]
    SeedFavicons { seed: LaunchFavicons },
    /// `startFavicons` (an exported function that writes a store).
    #[serde(rename = "startFavicons")]
    StartFavicons,
    /// `watchMissingFavicon` (an exported function that writes a store).
    #[serde(rename = "watchMissingFavicon")]
    WatchMissingFavicon { w: Watcher },
}

/// Applies `action` to `app`. Slice 2.26 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.26")
}
