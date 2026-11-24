// Ported from ardia apps/browser/src/state/airTraffic/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `airTraffic` (`apps/browser/src/state/airTraffic`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.39 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AirTrafficConfig {
    pub rules: Vec<AirTrafficRule>,
    pub fallback: Option<AirTrafficTarget>,
    #[serde(
        rename = "fallbackChosen",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub fallback_chosen: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AirTrafficRule {
    #[serde(rename = "profileId")]
    pub profile_id: Option<String>,
    pub open: AirTrafficRuleOpen,
    #[serde(rename = "groupKey", default, skip_serializing_if = "Option::is_none")]
    pub group_key: Option<String>,
    pub id: String,
    pub enabled: bool,
    pub from: Option<SourceApp>,
    pub when: LinkCondition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AirTrafficRuleOpen {
    #[default]
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "window")]
    Window,
    #[serde(rename = "littleArdia")]
    LittleArdia,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SourceApp {
    #[serde(rename = "bundleId")]
    pub bundle_id: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum LinkCondition {
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "site")]
    Site { host: String },
    #[serde(rename = "pattern")]
    Pattern { pattern: String },
    #[serde(rename = "contains")]
    Contains { text: String },
    #[serde(rename = "equals")]
    Equals { text: String },
}

impl Default for LinkCondition {
    fn default() -> Self {
        Self::Any
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AirTrafficTarget {
    #[serde(rename = "profileId")]
    pub profile_id: Option<String>,
    pub open: AirTrafficTargetOpen,
    #[serde(rename = "groupKey", default, skip_serializing_if = "Option::is_none")]
    pub group_key: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AirTrafficTargetOpen {
    #[default]
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "window")]
    Window,
    #[serde(rename = "littleArdia")]
    LittleArdia,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RuleDraft {
    #[serde(rename = "groupKey", default, skip_serializing_if = "Option::is_none")]
    pub group_key: Option<String>,
    #[serde(rename = "profileId")]
    pub profile_id: Option<String>,
    pub open: RuleDraftOpen,
    pub enabled: bool,
    pub from: Option<SourceApp>,
    pub when: LinkCondition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RuleDraftOpen {
    #[default]
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "window")]
    Window,
    #[serde(rename = "littleArdia")]
    LittleArdia,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct NoteLinkSourceSource {
    #[serde(rename = "bundleId")]
    pub bundle_id: Option<String>,
    #[serde(rename = "appName")]
    pub app_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UpdateAirTrafficRulePatch {
    #[serde(rename = "groupKey", default, skip_serializing_if = "Option::is_none")]
    pub group_key: Option<String>,
    #[serde(rename = "profileId", default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<UpdateAirTrafficRulePatchOpen>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<SourceApp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<UpdateAirTrafficRulePatchWhen>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum UpdateAirTrafficRulePatchOpen {
    #[default]
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "window")]
    Window,
    #[serde(rename = "littleArdia")]
    LittleArdia,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum UpdateAirTrafficRulePatchWhen {
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "site")]
    Site { host: String },
    #[serde(rename = "pattern")]
    Pattern { pattern: String },
    #[serde(rename = "contains")]
    Contains { text: String },
    #[serde(rename = "equals")]
    Equals { text: String },
}

impl Default for UpdateAirTrafficRulePatchWhen {
    fn default() -> Self {
        Self::Any
    }
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub config: AirTrafficConfig,
    pub loaded: bool,
    #[serde(rename = "recentApps")]
    pub recent_apps: Vec<SourceApp>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `addAirTrafficRule` (an exported function that writes a store).
    #[serde(rename = "addAirTrafficRule")]
    AddAirTrafficRule {
        draft: RuleDraft,
        #[serde(default)]
        id: Option<String>,
    },
    /// `currentAirTraffic` (an exported function that writes a store).
    #[serde(rename = "currentAirTraffic")]
    CurrentAirTraffic,
    /// `loadAirTraffic` (an exported function that writes a store).
    #[serde(rename = "loadAirTraffic")]
    LoadAirTraffic,
    /// `moveAirTrafficRule` (an exported function that writes a store).
    #[serde(rename = "moveAirTrafficRule")]
    MoveAirTrafficRule { id: String, step: Number },
    /// `noteLinkSource` (an exported function that writes a store).
    #[serde(rename = "noteLinkSource")]
    NoteLinkSource {
        source: Option<NoteLinkSourceSource>,
    },
    /// `openInRouteGroup` (an exported function that writes a store).
    #[serde(rename = "openInRouteGroup")]
    OpenInRouteGroup {
        url: String,
        #[serde(rename = "profileId")]
        profile_id: String,
        #[serde(rename = "groupKey")]
        group_key: String,
    },
    /// `placeAirTrafficRule` (an exported function that writes a store).
    #[serde(rename = "placeAirTrafficRule")]
    PlaceAirTrafficRule { id: String, index: Number },
    /// `previewLink` (an exported function that writes a store).
    #[serde(rename = "previewLink")]
    PreviewLink {
        typed: String,
        source: Option<NoteLinkSourceSource>,
    },
    /// `removeAirTrafficRule` (an exported function that writes a store).
    #[serde(rename = "removeAirTrafficRule")]
    RemoveAirTrafficRule { id: String },
    /// `resetAirTrafficForTests` (an exported function that writes a store).
    #[serde(rename = "resetAirTrafficForTests")]
    ResetAirTrafficForTests {
        #[serde(default)]
        config: Option<AirTrafficConfig>,
        #[serde(rename = "recentApps", default)]
        recent_apps: Option<Vec<SourceApp>>,
    },
    /// `setAirTrafficFallback` (an exported function that writes a store).
    #[serde(rename = "setAirTrafficFallback")]
    SetAirTrafficFallback { given: Option<AirTrafficTarget> },
    /// `tryAddAirTrafficRule` (an exported function that writes a store).
    #[serde(rename = "tryAddAirTrafficRule")]
    TryAddAirTrafficRule {
        draft: RuleDraft,
        #[serde(default)]
        id: Option<String>,
    },
    /// `updateAirTrafficRule` (an exported function that writes a store).
    #[serde(rename = "updateAirTrafficRule")]
    UpdateAirTrafficRule {
        id: String,
        patch: UpdateAirTrafficRulePatch,
    },
}

/// Applies `action` to `app`. Slice 2.39 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.39")
}
