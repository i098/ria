// Ported from ardia apps/browser/src/state/omnibox/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `omnibox` (`apps/browser/src/state/omnibox`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.15 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CommandBarStart {
    #[serde(rename = "openTabs", default, skip_serializing_if = "Option::is_none")]
    pub open_tabs: Option<CommandBarStartOpenTabs>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CommandBarStartOpenTabs {
    #[serde(
        rename = "hidePinned",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub hide_pinned: Option<bool>,
    #[serde(
        rename = "hideTopApps",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub hide_top_apps: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<Number>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OmniboxProvider {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ClosePanelToHeroBarReason {
    #[default]
    #[serde(rename = "escape")]
    Escape,
    #[serde(rename = "closeKey")]
    CloseKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Disposition {
    #[default]
    #[serde(rename = "newTab")]
    NewTab,
    #[serde(rename = "newWindow")]
    NewWindow,
    #[serde(rename = "split")]
    Split,
    #[serde(rename = "current")]
    Current,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProviderApprovalSource {}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ShowUrlBarMenuTab {
    pub id: String,
    pub url: String,
    pub title: String,
    #[serde(rename = "windowId")]
    pub window_id: String,
    #[serde(
        rename = "customTitle",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_title: Option<String>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub list: Vec<OmniboxProvider>,
    pub revision: Number,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `cancelPanel` (an exported function that writes a store).
    #[serde(rename = "cancelPanel")]
    CancelPanel {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `closeNewTabBar` (an exported function that writes a store).
    #[serde(rename = "closeNewTabBar")]
    CloseNewTabBar {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `closePanelToHeroBar` (an exported function that writes a store).
    #[serde(rename = "closePanelToHeroBar")]
    ClosePanelToHeroBar {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(default)]
        reason: Option<ClosePanelToHeroBarReason>,
    },
    /// `openBarFromAddress` (an exported function that writes a store).
    #[serde(rename = "openBarFromAddress")]
    OpenBarFromAddress {
        #[serde(rename = "windowId")]
        window_id: String,
        text: String,
    },
    /// `openEmptySidebarField` (an exported function that writes a store).
    #[serde(rename = "openEmptySidebarField")]
    OpenEmptySidebarField {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `openFromBar` (an exported function that writes a store).
    #[serde(rename = "openFromBar")]
    OpenFromBar {
        url: String,
        #[serde(rename = "tabId")]
        tab_id: String,
        disposition: Disposition,
    },
    /// `openFromNewTabBar` (an exported function that writes a store).
    #[serde(rename = "openFromNewTabBar")]
    OpenFromNewTabBar {
        #[serde(rename = "windowId")]
        window_id: String,
        target: crate::browser::NewTabTarget,
        url: String,
    },
    /// `registerOmniboxProvider` (an exported function that writes a store).
    #[serde(rename = "registerOmniboxProvider")]
    RegisterOmniboxProvider { provider: OmniboxProvider },
    /// `runBarAction` (an exported function that writes a store).
    #[serde(rename = "runBarAction")]
    RunBarAction {
        id: String,
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `runBarExtension` (an exported function that writes a store).
    #[serde(rename = "runBarExtension")]
    RunBarExtension {
        s: serde_json::Value,
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `setProviderApprovalSource` (an exported function that writes a store).
    #[serde(rename = "setProviderApprovalSource")]
    SetProviderApprovalSource { next: ProviderApprovalSource },
    /// `showNewTabBar` (an exported function that writes a store).
    #[serde(rename = "showNewTabBar")]
    ShowNewTabBar {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(default)]
        target: Option<crate::browser::NewTabTarget>,
    },
    /// `showOnceOverEmptyCard` (an exported function that writes a store).
    #[serde(rename = "showOnceOverEmptyCard")]
    ShowOnceOverEmptyCard {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `showUrlBarMenu` (an exported function that writes a store).
    #[serde(rename = "showUrlBarMenu")]
    ShowUrlBarMenu { tab: ShowUrlBarMenuTab },
    /// `startNewTabBar` (an exported function that writes a store).
    #[serde(rename = "startNewTabBar")]
    StartNewTabBar,
    /// `switchFromBar` (an exported function that writes a store).
    #[serde(rename = "switchFromBar")]
    SwitchFromBar {
        #[serde(rename = "targetTabId")]
        target_tab_id: String,
        url: String,
        #[serde(rename = "barTabId")]
        bar_tab_id: String,
        disposition: Disposition,
    },
    /// `toggleNewTabBar` (an exported function that writes a store).
    #[serde(rename = "toggleNewTabBar")]
    ToggleNewTabBar {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(default)]
        target: Option<crate::browser::NewTabTarget>,
    },
}

/// Applies `action` to `app`. Slice 2.15 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.15")
}
