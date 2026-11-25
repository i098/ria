// Ported from ardia apps/browser/src/state/app/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `app` (`apps/browser/src/state/app`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.10 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

mod control;
mod install;

pub use control::parse_control_caller;
pub use install::describe_command_install;

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
    #[serde(skip)]
    pub(crate) control: control::Control,
    #[serde(skip)]
    pub(crate) local: Local,
}

/// The module variables of the TypeScript files that are not store fields.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Local {
    pub(crate) toast_seq: u64,
    pub(crate) running: bool,
    pub(crate) auto_first_install: bool,
    pub(crate) saved: Vec<(String, ria_core::identity::telemetry_wire::Json)>,
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
    InstallOrUpgradeCommand {
        #[serde(default)]
        reads: InstallReads,
    },
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
    /// The `engine.cliControl` event (`onControlChanged` in `programControl.ts`).
    #[serde(rename = "controlChanged")]
    ControlChanged { server: serde_json::Value },
    /// The result of `setControl` or `controlStatus`; `error` is the text of a rejected call.
    #[serde(rename = "controlCallSettled")]
    ControlCallSettled {
        request: Number,
        #[serde(default)]
        server: serde_json::Value,
        #[serde(default)]
        error: Option<String>,
    },
    /// The end of the 3000 ms hold of the control indicator (`INDICATOR_HOLD_MS`).
    #[serde(rename = "indicatorHoldElapsed")]
    IndicatorHoldElapsed,
    /// One `onInstallLine` event of `installCommand`.
    #[serde(rename = "commandInstallLine")]
    CommandInstallLine { line: String },
    /// The result of `installCommand`.
    #[serde(rename = "commandInstallFinished")]
    CommandInstallFinished(CommandInstallFinished),
}

/// The synchronous reads that `installOrUpgradeCommand` makes, taken by the host (see `install.rs`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstallReads {
    #[serde(default, rename = "installedVersion")]
    pub installed_version: Option<String>,
    /// The text of `ardia-cli.json`, or `None` when the file is absent.
    #[serde(default)]
    pub saved: Option<String>,
    #[serde(default, rename = "appVersion")]
    pub app_version: Option<String>,
    #[serde(default, rename = "canInstallUnattended")]
    pub can_install_unattended: bool,
    #[serde(default)]
    pub now: Number,
}

/// The result of `installCommand`, with the reads that follow it.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CommandInstallFinished {
    pub ok: bool,
    #[serde(default, rename = "binPath")]
    pub bin_path: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default, rename = "installedVersion")]
    pub installed_version: Option<String>,
    #[serde(default)]
    pub now: Number,
}

/// Applies `action` to `app`.
///
/// Changes from Ardia: the `Start*` actions only register listeners and timers of the host (`ria-app` owns them),
/// and `watchSystemContrast` ends with the first `onHighContrast` event, so they write no state except the
/// `started` and `watching` guards. `startAppIntegration`, `startNativeSync`, `startScripting` and
/// `startNotificationResponses` wire other folders and write no store of this one.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    match action {
        Action::CloseAnchoredUi => {
            use crate::{bookmarks, extensions, profiles, sidebar, tabs};
            tabs::apply(app, tabs::Action::CloseEveryPopover, fx);
            extensions::apply(app, extensions::Action::CloseExtensionPopup, fx);
            bookmarks::apply(app, bookmarks::Action::CloseFolderMenus { arg0: None }, fx);
            sidebar::apply(app, sidebar::Action::CloseFolderPanel, fx);
            sidebar::apply(app, sidebar::Action::DismissHover, fx);
            tabs::apply(app, tabs::Action::DismissStripHover, fx);
            profiles::apply(app, profiles::Action::CloseProfileIconPicker, fx);
        }
        Action::ShowToast {
            window_id,
            title,
            message,
            options,
        } => show_toast(app, window_id, title, message, options),
        Action::HideToast { window_id, id } => {
            let shown = app
                .get::<State>()
                .toasts
                .get(&window_id)
                .and_then(Option::as_ref);
            if shown.is_some_and(|t| t.id == id) {
                app.get_mut::<State>().toasts.insert(window_id, None);
            }
        }
        Action::HideKeyedToast { window_id, key } => {
            let shown = app
                .get::<State>()
                .toasts
                .get(&window_id)
                .and_then(Option::as_ref);
            if shown.is_some_and(|t| t.key.as_deref() == Some(key.as_str())) {
                app.get_mut::<State>().toasts.insert(window_id, None);
            }
        }
        Action::NoteControlCaller { caller } => control::note_control_caller(app, caller),
        Action::NoteSocketRun { delta } => control::note_socket_run(app, delta.0 as i64),
        Action::NoteSocketServing { serving } => control::note_socket_serving(app, serving),
        Action::NoteTabReady { tab_id } => control::note_tab_ready(app, fx, &tab_id),
        Action::SetControlFromSettings { on } | Action::SetProgramControl { on } => {
            control::set_program_control(app, fx, on)
        }
        Action::StopProgramControl => control::set_program_control(app, fx, false),
        Action::StartProgramControl => control::start_program_control(app, fx),
        Action::ControlChanged { server } => control::control_changed(app, fx, server),
        Action::ControlCallSettled {
            request,
            server,
            error,
        } => control::control_call_settled(app, fx, request.0 as i64, server, error),
        Action::IndicatorHoldElapsed => control::indicator_hold_elapsed(app),
        Action::InstallCommandInBackground { reason } => {
            install::install_command_in_background(app, fx, reason)
        }
        Action::InstallOrUpgradeCommand { reads } => {
            install::install_or_upgrade_command(app, fx, reads)
        }
        Action::CommandInstallLine { line } => install::command_install_line(app, &line),
        Action::CommandInstallFinished(done) => install::command_install_finished(app, fx, done),
        Action::WatchSystemContrast => app.get_mut::<State>().control.watching = true,
        Action::ShowAppMenu { .. }
        | Action::StartAppIntegration
        | Action::StartNativeSync
        | Action::StartNotificationResponses
        | Action::StartScripting => {}
    }
}

/// `showToast`: a toast with the key of the shown toast keeps its `id`.
fn show_toast(
    app: &mut AppState,
    window_id: String,
    title: String,
    message: Option<String>,
    options: Option<ToastOptions>,
) {
    let options = options.unwrap_or_default();
    let current = app
        .get::<State>()
        .toasts
        .get(&window_id)
        .and_then(Option::as_ref);
    let kept = options
        .key
        .as_ref()
        .and_then(|key| current.filter(|t| t.key.as_ref() == Some(key)))
        .map(|t| t.id);
    let st = app.get_mut::<State>();
    let id = kept.unwrap_or_else(|| {
        st.local.toast_seq += 1;
        Number(st.local.toast_seq as f64)
    });
    let toast = Toast {
        id,
        title,
        message,
        icon: options.icon,
        action: options.action,
        key: options.key,
        sticky: options.sticky,
    };
    st.toasts.insert(window_id, Some(toast));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(app: &mut AppState, action: Action) {
        apply(app, action, &mut Effects::default());
    }

    fn show(app: &mut AppState, title: &str, key: Option<&str>) {
        let options = key.map(|k| ToastOptions {
            key: Some(k.into()),
            ..Default::default()
        });
        run(
            app,
            Action::ShowToast {
                window_id: "w".into(),
                title: title.into(),
                message: None,
                options,
            },
        );
    }

    fn id(app: &AppState) -> f64 {
        app.get::<State>().toasts["w"].as_ref().unwrap().id.0
    }

    #[test]
    fn a_keyed_toast_keeps_its_id_and_a_hide_with_another_id_does_nothing() {
        let mut app = AppState::default();
        show(&mut app, "a", Some("k"));
        show(&mut app, "b", Some("k"));
        assert_eq!(id(&app), 1.0);
        show(&mut app, "c", None);
        assert_eq!(id(&app), 2.0);
        let before = app.changes::<State>();
        run(
            &mut app,
            Action::HideToast {
                window_id: "w".into(),
                id: Number(1.0),
            },
        );
        assert_eq!(app.changes::<State>(), before);
        run(
            &mut app,
            Action::HideToast {
                window_id: "w".into(),
                id: Number(2.0),
            },
        );
        assert!(app.get::<State>().toasts["w"].is_none());
    }

    #[test]
    fn the_indicator_hold_ends_only_when_asked_and_a_connection_cancels_it() {
        let mut app = AppState::default();
        let step = |app: &mut AppState, a| run(app, a);
        step(&mut app, Action::NoteSocketServing { serving: true });
        step(&mut app, Action::NoteSocketRun { delta: Number(1.0) });
        assert!(app.get::<State>().shown);
        step(
            &mut app,
            Action::NoteSocketRun {
                delta: Number(-1.0),
            },
        );
        assert!(app.get::<State>().shown);
        step(&mut app, Action::NoteSocketRun { delta: Number(1.0) });
        step(&mut app, Action::IndicatorHoldElapsed);
        assert!(app.get::<State>().shown);
        step(
            &mut app,
            Action::NoteSocketRun {
                delta: Number(-1.0),
            },
        );
        step(&mut app, Action::IndicatorHoldElapsed);
        assert!(!app.get::<State>().shown);
    }
}
