// Ported from ardia packages/ardiacore/ios/ArdiaHostEngineDelegate.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The engine delegate (`ArdiaCoreEngineDelegate`), made with `objc2`. Each callback copies its data into JSON
//! and calls a [`Hooks`] method, so no Objective-C object goes past this file except the tab, the profile, the
//! window and the event that the hook needs.
//!
//! Deferred entry rule (plan section 6.4): the hooks of `ria-app` put an item in the action queue and return.
//! The 5 callbacks that need an answer at once (`engineWindowForNewBrowserOfProfile:type:`, the 2 `Reserved` ones and
//! the 2 `BOOL` extension ones) are the only direct entries; their hooks must not call `apply` or the engine.
//!
//! Not ported here, because the unit is owned by another slice: the launch steps of `engineDidStart` (reserved shortcut
//! watch, activation guards, drag fix, content blocker, launch sequence, React Native observer, app delegate) go
//! through [`Hooks::engine_started`]; the popup-window and held-event logic goes through the other hooks.

use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{AllocAnyThread, DefinedClass, define_class, msg_send};
use objc2_app_kit::NSEvent;
use objc2_foundation::{NSArray, NSNotificationCenter, NSString};
use serde_json::{Map, Value, json};

use super::bindings::{Engine, Payload, Profile, Tab, Window};
use super::convert::to_json;
use super::host::State;

/// The parts of Ardia that the delegate calls and that other slices own. Every method runs on the main thread.
pub trait Hooks {
    /// The steps of `engineDidStart` that belong to the launch slice.
    fn engine_started(&self);
    /// `engineWillShutDown`.
    fn engine_shut_down(&self);
    /// `ArdiaEvents emit:data:`.
    fn emit(&self, topic: &str, data: Value);
    /// `ArdiaEvents listens:`: true when the app listens to the topic.
    fn listens(&self, topic: &str) -> bool;
    /// `ardia_host::AppTabIdValue`: the app tab id as a JSON string, or `null` when the tab has none.
    fn app_tab_id_value(&self, tab: Option<&Tab>) -> Value;
    /// `ardia_host::ProfileName`.
    fn profile_name(&self, profile: &Profile) -> String;
    /// The names under which `profile` is loaded (`LoadedProfileNames` filtered by `LoadedProfile(name) == profile`).
    fn names_of_loaded_profile(&self, profile: &Profile) -> Vec<String>;
    /// `ardia_host::AskPermissionInPopupWindow`: true when a popup window took the request.
    fn ask_permission_in_popup_window(&self, tab: &Tab, request: &Value) -> bool;
    /// `ardia_host::HoldUntilHosted`.
    fn hold_until_hosted(&self, tab: &Tab, topic: &str, payload: Value);
    /// `ardia_host::DropPopupWindowPermission`.
    fn drop_popup_window_permission(&self, request_id: &str);
    /// `ardia_host::DropHeldEvent`.
    fn drop_held_event(&self, topic: &str, request_id: &str);
    /// `ardia_host::IsReservedBinding`. Direct entry.
    fn is_reserved_binding(&self, key: &str, unshifted_key: &str, modifiers: usize) -> bool;
    /// `ardia_host::IsReservedKeyEvent`. Direct entry.
    fn is_reserved_key_event(&self, event: &NSEvent) -> bool;
    /// The window for a browser that Chromium makes (`type` is `normal`, `popup` or `app`). Direct entry.
    fn window_for_new_browser(&self, profile: &Profile, kind: &str) -> Option<Retained<Window>>;
}

/// What the delegate keeps for its callbacks.
pub struct Ivars {
    hooks: Rc<dyn Hooks>,
    state: Rc<State>,
    page_script: Option<String>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "RiaHostEngineDelegate"]
    #[ivars = Ivars]
    pub struct EngineDelegate;

    impl EngineDelegate {
        #[unsafe(method(engineDidStart))]
        fn engine_did_start(&self) {
            let ivars = self.ivars();
            if let Some(engine) = Engine::shared() {
                eprintln!("[ardiacore] engine started (Chromium {})", engine.chromium_version());
                if let Some(script) = &ivars.page_script {
                    engine.set_page_script(Some(&NSString::from_str(script)));
                }
            }
            ivars.state.started.set(true);
            ivars.hooks.engine_started();
        }

        #[unsafe(method(engineWillShutDown))]
        fn engine_will_shut_down(&self) {
            let ivars = self.ivars();
            ivars.state.started.set(false);
            ivars.hooks.engine_shut_down();
        }

        #[unsafe(method(engineQuitCancelled))]
        fn engine_quit_cancelled(&self) {
            eprintln!("[ardiacore] quit cancelled");
            let name = NSString::from_str("ArdiaHostQuitCancelled");
            unsafe { NSNotificationCenter::defaultCenter().postNotificationName_object(&name, None) };
            if let Some(handler) = &*self.ivars().state.quit_cancelled_handler.borrow() {
                handler();
            }
        }

        #[unsafe(method(engine:permissionRequest:tab:))]
        fn permission_request(&self, _engine: &Engine, request: &Payload, tab: Option<&Tab>) {
            let hooks = &self.ivars().hooks;
            let mut payload = object(request);
            if let Some(tab) = tab {
                if hooks.ask_permission_in_popup_window(tab, &Value::Object(payload.clone())) {
                    return;
                }
            }
            payload.insert("tabId".into(), hooks.app_tab_id_value(tab));
            match tab {
                Some(tab) if payload["tabId"].is_null() => hooks.hold_until_hosted(tab, "permissions.requested", Value::Object(payload)),
                _ => hooks.emit("permissions.requested", Value::Object(payload)),
            }
        }

        #[unsafe(method(engine:deviceChooser:tab:))]
        fn device_chooser(&self, _engine: &Engine, chooser: &Payload, tab: Option<&Tab>) {
            let hooks = &self.ivars().hooks;
            let mut payload = object(chooser);
            payload.insert("tabId".into(), hooks.app_tab_id_value(tab));
            if hooks.listens("media") {
                hooks.emit("media.deviceChooser", Value::Object(payload));
            } else if opened(&payload) {
                Engine::cancel_device_chooser(id(&payload));
            }
        }

        #[unsafe(method(engine:castDialog:tab:))]
        fn cast_dialog(&self, _engine: &Engine, dialog: &Payload, tab: Option<&Tab>) {
            let hooks = &self.ivars().hooks;
            let mut payload = object(dialog);
            payload.insert("tabId".into(), hooks.app_tab_id_value(tab));
            if hooks.listens("media") {
                hooks.emit("media.castDialog", Value::Object(payload));
            } else if opened(&payload) {
                Engine::close_cast_dialog(id(&payload));
            }
        }

        #[unsafe(method(engine:castRoutes:profile:))]
        fn cast_routes(&self, _engine: &Engine, routes: &NSArray<AnyObject>, profile: &Profile) {
            let hooks = &self.ivars().hooks;
            if !hooks.listens("media") {
                return;
            }
            let mut names = Vec::new();
            if profile.off_the_record() {
                names = hooks.names_of_loaded_profile(profile);
            }
            if names.is_empty() {
                names.push(hooks.profile_name(profile));
            }
            let routes = to_json(routes);
            let routes = if routes.is_array() { routes } else { json!([]) };
            for name in names {
                hooks.emit("media.castRoutes", json!({"profile": name, "routes": routes}));
            }
        }

        #[unsafe(method(engine:extensionSidePanel:tab:))]
        fn extension_side_panel(&self, _engine: &Engine, panel: &Payload, tab: Option<&Tab>) {
            let hooks = &self.ivars().hooks;
            let mut payload = object(panel);
            payload.insert("tabId".into(), hooks.app_tab_id_value(tab));
            hooks.emit("extensions.sidePanel", Value::Object(payload));
        }

        #[unsafe(method(engine:extensionActionPopup:tab:))]
        fn extension_action_popup(&self, _engine: &Engine, popup: &Payload, tab: Option<&Tab>) -> bool {
            let hooks = &self.ivars().hooks;
            let mut payload = object(popup);
            if hooks.listens("extensions") && tab.is_some() {
                payload.insert("tabId".into(), hooks.app_tab_id_value(tab));
                hooks.emit("extensions.actionPopup", Value::Object(payload));
                true
            } else {
                let why = if tab.is_some() { "the app isn't listening yet" } else { "no tab" };
                eprintln!("[ardiacore] action.openPopup for {}: {why}", payload.get("extensionId").unwrap_or(&Value::Null));
                false
            }
        }

        #[unsafe(method(engineIsReservedKey:unshiftedKey:modifiers:))]
        fn is_reserved_key(&self, key: &NSString, unshifted_key: &NSString, modifiers: usize) -> bool {
            self.ivars().hooks.is_reserved_binding(&key.to_string(), &unshifted_key.to_string(), modifiers)
        }

        #[unsafe(method(engineIsReservedKeyEvent:))]
        fn is_reserved_key_event(&self, event: &NSEvent) -> bool {
            // `NSEventTypeKeyDown` is 10.
            event.r#type().0 == 10 && self.ivars().hooks.is_reserved_key_event(event)
        }

        #[unsafe(method(engine:extensionActionShortcut:tab:))]
        fn extension_action_shortcut(&self, _engine: &Engine, shortcut: &Payload, tab: Option<&Tab>) -> bool {
            let hooks = &self.ivars().hooks;
            if hooks.listens("extensions") && tab.is_some() {
                let mut payload = object(shortcut);
                payload.insert("tabId".into(), hooks.app_tab_id_value(tab));
                hooks.emit("extensions.actionShortcut", Value::Object(payload));
                true
            } else {
                false
            }
        }

        #[unsafe(method(engine:extensionInstallPrompt:profile:tab:))]
        fn extension_install_prompt(&self, _engine: &Engine, prompt: &Payload, profile: &Profile, tab: Option<&Tab>) {
            let ivars = self.ivars();
            let mut payload = object(prompt);
            payload.insert("tabId".into(), ivars.hooks.app_tab_id_value(tab));
            payload.insert("profile".into(), Value::String(ivars.hooks.profile_name(profile)));
            let request = payload.get("requestId").and_then(Value::as_str).unwrap_or("").to_owned();
            ivars.state.install_prompts.borrow_mut().insert(request, Value::Object(payload.clone()));
            ivars.hooks.emit("extensions.installPrompt", Value::Object(payload));
        }

        #[unsafe(method(engine:extensionInstallFailed:tab:))]
        fn extension_install_failed(&self, _engine: &Engine, message: &NSString, tab: Option<&Tab>) {
            let hooks = &self.ivars().hooks;
            hooks.emit(
                "extensions.installFailed",
                json!({"message": message.to_string(), "tabId": hooks.app_tab_id_value(tab)}),
            );
        }

        #[unsafe(method(engine:permissionRequestDismissed:))]
        fn permission_request_dismissed(&self, _engine: &Engine, request_id: &NSString) {
            let hooks = &self.ivars().hooks;
            let request = request_id.to_string();
            hooks.drop_popup_window_permission(&request);
            hooks.drop_held_event("permissions.requested", &request);
            hooks.emit("permissions.dismissed", json!({"id": request}));
        }

        #[unsafe(method_id(engineWindowForNewBrowserOfProfile:type:))]
        fn window_for_new_browser(&self, profile: &Profile, kind: &NSString) -> Option<Retained<Window>> {
            self.ivars().hooks.window_for_new_browser(profile, &kind.to_string())
        }
    }
);

impl EngineDelegate {
    /// Makes the delegate. `page_script` is the text of `page_script.js` (Ardia reads it from the bundle); the
    /// delegate gives it to the engine at `engineDidStart`.
    pub fn new(hooks: Rc<dyn Hooks>, state: Rc<State>, page_script: Option<String>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(Ivars { hooks, state, page_script });
        unsafe { msg_send![super(this), init] }
    }

    /// The delegate as the plain object that `+[ArdiaCoreEngine runWithArgc:argv:delegate:]` takes.
    pub fn as_object(&self) -> &AnyObject {
        self
    }

    /// `[ArdiaCoreEngine runWithArgc:argv:delegate:]` with `argv`. Returns the exit code at quit.
    ///
    /// # Safety
    /// The engine framework must be loaded, and the call must be on the main thread.
    pub unsafe fn run(&self, argv: &[std::ffi::CString]) -> i32 {
        let mut pointers: Vec<*const std::ffi::c_char> = argv.iter().map(|a| a.as_ptr()).collect();
        pointers.push(std::ptr::null());
        unsafe { Engine::run(argv.len() as i32, pointers.as_ptr(), self.as_object()) }
    }
}

fn object(payload: &Payload) -> Map<String, Value> {
    match to_json(payload) {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

fn opened(payload: &Map<String, Value>) -> bool {
    payload.get("open").and_then(Value::as_bool).unwrap_or(false)
}

fn id(payload: &Map<String, Value>) -> i32 {
    payload.get("id").and_then(Value::as_i64).unwrap_or(0) as i32
}
