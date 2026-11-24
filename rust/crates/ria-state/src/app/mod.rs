// Ported from ardia apps/browser/src/state/app/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `app` (`apps/browser/src/state/app`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.10 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ControlCaller {
    pub name: String,
    pub app: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateInstalling {
    #[default]
    #[serde(rename = "settings")]
    Settings,
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "upgrade")]
    Upgrade,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Toast {
    pub id: Number,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<ToastAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sticky: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ToastAction {
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ToastOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<ToastAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sticky: Option<bool>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(rename = "binPath")]
    pub bin_path: Option<String>,
    pub caller: Option<ControlCaller>,
    pub error: Option<String>,
    #[serde(rename = "installedVersion")]
    pub installed_version: Option<String>,
    pub installing: Option<StateInstalling>,
    pub on: bool,
    pub server: serde_json::Value,
    pub shown: bool,
    pub step: Option<String>,
    pub toasts: std::collections::HashMap<String, Option<Toast>>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `closeAnchoredUi` (an exported function that writes a store).
    #[serde(rename = "closeAnchoredUi")]
    CloseAnchoredUi,
    /// `hideKeyedToast` (an exported function that writes a store).
    #[serde(rename = "hideKeyedToast")]
    HideKeyedToast {
        #[serde(rename = "windowId")]
        window_id: String,
        key: String,
    },
    /// `hideToast` (an exported function that writes a store).
    #[serde(rename = "hideToast")]
    HideToast {
        #[serde(rename = "windowId")]
        window_id: String,
        id: Number,
    },
    /// `installCommandInBackground` (an exported function that writes a store).
    #[serde(rename = "installCommandInBackground")]
    InstallCommandInBackground {
        reason: crate::telemetry::CliInstallReason,
    },
    /// `installOrUpgradeCommand` (an exported function that writes a store).
    #[serde(rename = "installOrUpgradeCommand")]
    InstallOrUpgradeCommand,
    /// `noteControlCaller` (an exported function that writes a store).
    #[serde(rename = "noteControlCaller")]
    NoteControlCaller { caller: ControlCaller },
    /// `noteSocketRun` (an exported function that writes a store).
    #[serde(rename = "noteSocketRun")]
    NoteSocketRun { delta: Number },
    /// `noteSocketServing` (an exported function that writes a store).
    #[serde(rename = "noteSocketServing")]
    NoteSocketServing { serving: bool },
    /// `noteTabReady` (an exported function that writes a store).
    #[serde(rename = "noteTabReady")]
    NoteTabReady {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `setControlFromSettings` (an exported function that writes a store).
    #[serde(rename = "setControlFromSettings")]
    SetControlFromSettings { on: bool },
    /// `setProgramControl` (an exported function that writes a store).
    #[serde(rename = "setProgramControl")]
    SetProgramControl { on: bool },
    /// `showAppMenu` (an exported function that writes a store).
    #[serde(rename = "showAppMenu")]
    ShowAppMenu {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `showToast` (an exported function that writes a store).
    #[serde(rename = "showToast")]
    ShowToast {
        #[serde(rename = "windowId")]
        window_id: String,
        title: String,
        #[serde(default)]
        message: Option<String>,
        #[serde(default)]
        options: Option<ToastOptions>,
    },
    /// `startAppIntegration` (an exported function that writes a store).
    #[serde(rename = "startAppIntegration")]
    StartAppIntegration,
    /// `startNativeSync` (an exported function that writes a store).
    #[serde(rename = "startNativeSync")]
    StartNativeSync,
    /// `startNotificationResponses` (an exported function that writes a store).
    #[serde(rename = "startNotificationResponses")]
    StartNotificationResponses,
    /// `startProgramControl` (an exported function that writes a store).
    #[serde(rename = "startProgramControl")]
    StartProgramControl,
    /// `startScripting` (an exported function that writes a store).
    #[serde(rename = "startScripting")]
    StartScripting,
    /// `stopProgramControl` (an exported function that writes a store).
    #[serde(rename = "stopProgramControl")]
    StopProgramControl,
    /// `watchSystemContrast` (an exported function that writes a store).
    #[serde(rename = "watchSystemContrast")]
    WatchSystemContrast,
}

/// Applies `action` to `app`. Slice 2.10 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.10")
}
