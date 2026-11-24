// Ported from ardia apps/browser/src/state/commands/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `commands` (`apps/browser/src/state/commands`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.11 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MenuBar {
    pub listed: Vec<String>,
    pub fixed: Vec<ShortcutSource>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ShortcutSource {
    pub id: String,
    pub keys: Vec<Shortcut>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Shortcut {
    pub key: String,
    pub modifiers: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CtrlTabPreset {
    #[default]
    #[serde(rename = "step")]
    Step,
    #[serde(rename = "switcher")]
    Switcher,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ReportProblemSource {
    #[default]
    #[serde(rename = "menu")]
    Menu,
    #[serde(rename = "check_in")]
    CheckIn,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RunCommandArg1 {
    #[serde(
        rename = "strictWindow",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub strict_window: Option<bool>,
    #[serde(rename = "tabId", default, skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
    #[serde(rename = "mayWrite", default, skip_serializing_if = "Option::is_none")]
    pub may_write: Option<serde_json::Value>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar: Option<MenuBar>,
    pub revision: Number,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `chooseCtrlTab` (an exported function that writes a store).
    #[serde(rename = "chooseCtrlTab")]
    ChooseCtrlTab { preset: CtrlTabPreset },
    /// `exportBookmarksFromWindow` (an exported function that writes a store).
    #[serde(rename = "exportBookmarksFromWindow")]
    ExportBookmarksFromWindow {
        #[serde(rename = "windowId")]
        window_id: Option<String>,
    },
    /// `followChromeCtrlTab` (an exported function that writes a store).
    #[serde(rename = "followChromeCtrlTab")]
    FollowChromeCtrlTab,
    /// `menuState` (an exported function that writes a store). The callback parameters `liveOfTab` are not in the JSON.
    #[serde(rename = "menuState")]
    MenuState { s: crate::browser::BrowserState },
    /// `reportProblem` (an exported function that writes a store).
    #[serde(rename = "reportProblem")]
    ReportProblem {
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
        #[serde(default)]
        source: Option<ReportProblemSource>,
    },
    /// `runCommand` (an exported function that writes a store).
    #[serde(rename = "runCommand")]
    RunCommand {
        arg0: serde_json::Value,
        #[serde(default)]
        arg1: Option<RunCommandArg1>,
    },
    /// `runLittleArdiaCommand` (an exported function that writes a store).
    #[serde(rename = "runLittleArdiaCommand")]
    RunLittleArdiaCommand {
        command: String,
        arg: Option<String>,
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `setCtrlTab` (an exported function that writes a store).
    #[serde(rename = "setCtrlTab")]
    SetCtrlTab { preset: CtrlTabPreset },
    /// `setMenuBarShortcuts` (an exported function that writes a store).
    #[serde(rename = "setMenuBarShortcuts")]
    SetMenuBarShortcuts { offered: Vec<serde_json::Value> },
    /// `toggleDownloadsPanel` (an exported function that writes a store).
    #[serde(rename = "toggleDownloadsPanel")]
    ToggleDownloadsPanel {
        #[serde(rename = "windowId")]
        window_id: String,
        open: bool,
    },
    /// `trackedMenuState` (an exported function that writes a store).
    #[serde(rename = "trackedMenuState")]
    TrackedMenuState { s: crate::browser::BrowserState },
}

/// Applies `action` to `app`. Slice 2.11 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.11")
}
