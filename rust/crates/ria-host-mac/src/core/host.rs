// Ported from ardia packages/ardiacore/ios/ArdiaHost.h and ArdiaHost.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The class methods of `ArdiaHost` that only go to the engine or hold engine-level state: permission, external
//! app, device chooser, cast, tracing, extension install prompts, the quit-cancelled handler and the host flags.
//!
//! Not here, because they read the tab, window or profile registries that other slices own: `chromeWindows`,
//! `engineInfo`, `displayMediaSources`, the `cli*` calls, `deleteProfileData`, `releaseProfile`, `unloadIdleProfile`,
//! `loadedProfiles`, the `TabWithAppTabId` calls (`stopCapture`, `showAutofillSuggestions`, `shareTabInstead`,
//! `showCastDialog`, `devShowBluetoothChooser`, `actionStates`, `sidePanelURL`, `chromeTabId`) and the press-loop calls.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use block2::RcBlock;
use objc2_foundation::NSString;
use serde_json::Value;

use super::bindings::{Engine, Profile};
use super::convert::to_json;
use super::delegate::Hooks;

/// State of the host that the delegate and the host calls share. It lives on the main thread.
#[derive(Default)]
pub struct State {
    /// `ArdiaLaunchSequence.engineRunning`: true between `engineDidStart` and `engineWillShutDown`.
    pub started: Cell<bool>,
    /// `gInstallPrompts`: the open extension install prompts, by request id.
    pub install_prompts: RefCell<BTreeMap<String, Value>>,
    /// `ArdiaHost.quitCancelledHandler`.
    pub quit_cancelled_handler: RefCell<Option<Box<dyn Fn()>>>,
    /// `gDataDirectory`.
    pub data_directory: RefCell<String>,
    /// `gMenuShortcutsReady`.
    menu_shortcuts_ready: Cell<bool>,
}

impl State {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// `+isStarted`.
    pub fn is_started(&self) -> bool {
        self.started.get()
    }

    /// `+dataDirectory`.
    pub fn data_directory(&self) -> String {
        self.data_directory.borrow().clone()
    }

    pub fn set_quit_cancelled_handler(&self, handler: Option<Box<dyn Fn()>>) {
        *self.quit_cancelled_handler.borrow_mut() = handler;
    }

    /// `+menuShortcutsChanged`.
    pub fn menu_shortcuts_changed(&self) {
        self.menu_shortcuts_ready.set(true);
        self.send_reserved_shortcuts();
    }

    /// `SendReservedShortcuts`: the engine learns about the reserved keys once the engine runs and the menu is ready.
    /// The launch slice calls it again when the keyboard input source changes.
    pub fn send_reserved_shortcuts(&self) {
        if self.started.get() && self.menu_shortcuts_ready.get() {
            Engine::reserved_shortcuts_changed();
        }
    }

    /// `+replayInstallPrompts`.
    pub fn replay_install_prompts(&self, hooks: &dyn Hooks) {
        for prompt in self.install_prompts.borrow().values() {
            hooks.emit("extensions.installPrompt", prompt.clone());
        }
    }

    /// `+resolveExtensionInstallPrompt:accepted:`.
    pub fn resolve_extension_install_prompt(&self, request_id: &str, accepted: bool) {
        self.install_prompts.borrow_mut().remove(request_id);
        Engine::resolve_extension_install_prompt(&NSString::from_str(request_id), accepted);
    }
}

/// `+resolvePermission:result:remember:`.
pub fn resolve_permission(request_id: &str, result: &str, remember: bool) {
    Engine::resolve_permission(&NSString::from_str(request_id), &NSString::from_str(result), remember);
}

/// `+resolveExternalApp:open:remember:`.
pub fn resolve_external_app(request_id: &str, open: bool, remember: bool) {
    Engine::resolve_external_app(&NSString::from_str(request_id), open, remember);
}

/// `+deviceChooser:select:`.
pub fn device_chooser_select(chooser_id: i32, index: i32) {
    Engine::select_device(chooser_id, index);
}

/// `+deviceChooser:action:`: `cancel`, `refresh` or `settings`; another action does nothing.
pub fn device_chooser_action(chooser_id: i32, action: &str) {
    match action {
        "cancel" => Engine::cancel_device_chooser(chooser_id),
        "refresh" => Engine::refresh_device_chooser(chooser_id),
        "settings" => Engine::open_device_chooser_settings(chooser_id),
        _ => {}
    }
}

/// `+castDialog:start:mode:`.
pub fn cast_dialog_start(dialog_id: i32, sink: &str, mode: i32) {
    Engine::start_casting(dialog_id, &NSString::from_str(sink), mode);
}

/// `+castDialog:stop:`.
pub fn cast_dialog_stop(dialog_id: i32, route: &str) {
    Engine::stop_casting(dialog_id, &NSString::from_str(route));
}

/// `+closeCastDialog:`.
pub fn close_cast_dialog(dialog_id: i32) {
    Engine::close_cast_dialog(dialog_id);
}

/// `+watchCastRoutes:` after the profile is resolved by the caller.
pub fn watch_cast_routes(profile: &Profile) {
    profile.watch_cast_routes();
}

/// `+terminateCastRoute:`.
pub fn terminate_cast_route(route: &str) {
    Engine::terminate_cast_route(&NSString::from_str(route));
}

/// `+testExternalLaunches`.
pub fn test_external_launches() -> Value {
    to_json(&Engine::test_external_launches())
}

/// `+testChooserEvents`.
pub fn test_chooser_events() -> Value {
    to_json(&Engine::test_chooser_events())
}

/// `+beginTracing:`.
pub fn begin_tracing(completion: impl Fn(bool) + 'static) {
    Engine::begin_tracing(&RcBlock::new(completion));
}

/// `+endTracing:completion:`: the completion gets the path of the trace, or `None`.
pub fn end_tracing(keep: bool, completion: impl Fn(Option<String>) + 'static) {
    Engine::end_tracing(keep, completion);
}

/// `+isTracing`.
pub fn is_tracing() -> bool {
    Engine::is_tracing()
}
