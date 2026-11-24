// Ported from ardia apps/browser/src/state/presets/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `presets` (`apps/browser/src/state/presets`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.43 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SetupChoices {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<SetupChoicesPreset>,
    pub settings: SetupChoicesSettings,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<SetupChoicesColor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pins: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetupChoicesPreset {
    #[default]
    #[serde(rename = "chromium")]
    Chromium,
    #[serde(rename = "arc")]
    Arc,
    #[serde(rename = "dia")]
    Dia,
    #[serde(rename = "chrome")]
    Chrome,
    #[serde(rename = "safari")]
    Safari,
    #[serde(rename = "firefox")]
    Firefox,
    #[serde(rename = "zen")]
    Zen,
    #[serde(rename = "brave")]
    Brave,
    #[serde(rename = "edge")]
    Edge,
    #[serde(rename = "helium")]
    Helium,
    #[serde(rename = "vivaldi")]
    Vivaldi,
    #[serde(rename = "opera")]
    Opera,
    #[serde(rename = "fresh")]
    Fresh,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SetupChoicesSettings {
    #[serde(
        rename = "restoreSession",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub restore_session: Option<bool>,
    #[serde(rename = "tabLayout", default, skip_serializing_if = "Option::is_none")]
    pub tab_layout: Option<SetupChoicesSettingsTabLayout>,
    #[serde(
        rename = "sidebarSide",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sidebar_side: Option<SetupChoicesSettingsSidebarSide>,
    #[serde(
        rename = "hideToolbarWhileScrolling",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub hide_toolbar_while_scrolling: Option<bool>,
    #[serde(
        rename = "newTabPosition",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub new_tab_position: Option<SetupChoicesSettingsNewTabPosition>,
    #[serde(
        rename = "newTabButtonPosition",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub new_tab_button_position: Option<SetupChoicesSettingsNewTabButtonPosition>,
    #[serde(
        rename = "newTabOpens",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub new_tab_opens: Option<SetupChoicesSettingsNewTabOpens>,
    #[serde(
        rename = "undoReopensClosedTab",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub undo_reopens_closed_tab: Option<bool>,
    #[serde(
        rename = "shareTabsAcrossWindows",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub share_tabs_across_windows: Option<bool>,
    #[serde(
        rename = "extendWebsiteColor",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extend_website_color: Option<bool>,
    #[serde(
        rename = "cmdClickCreatesTabGroup",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub cmd_click_creates_tab_group: Option<bool>,
    #[serde(
        rename = "peekModifier",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub peek_modifier: Option<SetupChoicesSettingsPeekModifier>,
    #[serde(
        rename = "cleanUpInactiveTabsAfterMinutes",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub clean_up_inactive_tabs_after_minutes: Option<Number>,
    #[serde(
        rename = "balancedTopApps",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub balanced_top_apps: Option<bool>,
    #[serde(
        rename = "bookmarksBar",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bookmarks_bar: Option<SetupChoicesSettingsBookmarksBar>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appearance: Option<SetupChoicesSettingsAppearance>,
    #[serde(
        rename = "addressBar",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub address_bar: Option<SetupChoicesSettingsAddressBar>,
    #[serde(
        rename = "commandBarPlacement",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub command_bar_placement: Option<SetupChoicesSettingsCommandBarPlacement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetupChoicesSettingsTabLayout {
    #[default]
    #[serde(rename = "sidebar")]
    Sidebar,
    #[serde(rename = "top")]
    Top,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetupChoicesSettingsSidebarSide {
    #[default]
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetupChoicesSettingsNewTabPosition {
    #[default]
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "bottom")]
    Bottom,
    #[serde(rename = "nextToCurrent")]
    NextToCurrent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetupChoicesSettingsNewTabButtonPosition {
    #[default]
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "bottom")]
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetupChoicesSettingsNewTabOpens {
    #[default]
    #[serde(rename = "page")]
    Page,
    #[serde(rename = "commandBar")]
    CommandBar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetupChoicesSettingsPeekModifier {
    #[default]
    #[serde(rename = "alt")]
    Alt,
    #[serde(rename = "shift")]
    Shift,
    #[serde(rename = "off")]
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetupChoicesSettingsBookmarksBar {
    #[default]
    #[serde(rename = "newTab")]
    NewTab,
    #[serde(rename = "never")]
    Never,
    #[serde(rename = "always")]
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetupChoicesSettingsAppearance {
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "light")]
    Light,
    #[serde(rename = "dark")]
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetupChoicesSettingsAddressBar {
    #[default]
    #[serde(rename = "toolbar")]
    Toolbar,
    #[serde(rename = "sidebar")]
    Sidebar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetupChoicesSettingsCommandBarPlacement {
    #[default]
    #[serde(rename = "field")]
    Field,
    #[serde(rename = "centred")]
    Centred,
    #[serde(rename = "centredOnClick")]
    CentredOnClick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetupChoicesColor {
    #[default]
    #[serde(rename = "plum")]
    Plum,
    #[serde(rename = "blue")]
    Blue,
    #[serde(rename = "purple")]
    Purple,
    #[serde(rename = "pink")]
    Pink,
    #[serde(rename = "red")]
    Red,
    #[serde(rename = "orange")]
    Orange,
    #[serde(rename = "yellow")]
    Yellow,
    #[serde(rename = "green")]
    Green,
    #[serde(rename = "neutral")]
    Neutral,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `applySetupCode` (an exported function that writes a store).
    #[serde(rename = "applySetupCode")]
    ApplySetupCode {
        code: String,
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `applySetupCodeInOnboarding` (an exported function that writes a store).
    #[serde(rename = "applySetupCodeInOnboarding")]
    ApplySetupCodeInOnboarding { code: String },
}

/// Applies `action` to `app`. Slice 2.43 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.43")
}
