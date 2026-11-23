// Ported from ardia packages/ardiacore/ios/ArdiaHost+Launch.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::{Cell, RefCell};
use std::io;
use std::rc::{Rc, Weak};

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSApplication, NSEvent};
use objc2_foundation::NSString;
use ria_engine::call::main_queue;
use serde_json::Value;

use crate::core::bindings::{Engine, Profile, Tab, Window};
use crate::core::delegate::{EngineDelegate, Hooks};
use crate::core::host::State;
use crate::windows::ardia_window_full_screen::configure_acted_transition;

use super::activation::{self, Observers};
use super::early_window::BrowserWindow;
use super::early_window_layouts;
use super::launch_config::LaunchConfig;
use super::launch_sequence::{
    APP_KIT_FINISHED, APP_READY, ENGINE_STARTED, LaunchReason, LaunchSequence,
};
use super::platform;
use super::saved_session::SavedSession;
use super::startup;

#[derive(Debug)]
pub enum LaunchEvent {
    EngineStarted,
    WindowOpened { windows: usize, tabs: usize },
    TabTitle(String),
    EngineWillShutDown,
    Error(String),
}

pub type Observer = Rc<dyn Fn(LaunchEvent)>;

/// Start the engine from the macOS app bundle. Return its exit status after shutdown.
pub fn run(observer: Observer) -> io::Result<i32> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| io::Error::other("launch must run on the main thread"))?;
    objc2::rc::autoreleasepool(|_| run_in_pool(mtm, observer))
}

/// Request an orderly shutdown from the main thread, after the engine starts.
pub fn quit() {
    if MainThreadMarker::new().is_some()
        && let Some(engine) = Engine::shared()
    {
        engine.quit();
    }
}

fn run_in_pool(mtm: MainThreadMarker, observer: Observer) -> io::Result<i32> {
    let executable = std::env::current_exe()?;
    let distribution = platform::signed_for_distribution();
    let home = std::env::home_dir().ok_or_else(|| io::Error::other("home directory not found"))?;
    let env: Vec<_> = std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .collect();
    let config = LaunchConfig::from_environment(
        env.iter().map(|(k, v)| (k.as_str(), v.as_str())),
        &home,
        &std::env::current_dir()?,
        distribution,
    )?;
    let argv = startup::engine_arguments(&config, &executable, std::env::args().skip(1))?;
    if !config.isolated {
        let chromium = config.data_directory().join("Chromium");
        config.check_profile_directory()?;
        ria_engine::domains::browser::native_messaging::sync_hosts(&chromium);
    }
    let session = SavedSession::read(&config);
    platform::load_engine(&executable)?;
    configure_acted_transition(&config);
    let sequence = Rc::new(LaunchSequence::default());
    let runtime = Rc::new_cyclic(|this| Runtime {
        this: this.clone(),
        sequence: sequence.clone(),
        config,
        session,
        observer,
        mtm,
        window: RefCell::default(),
        failed: Cell::new(false),
    });
    let _observers = Observers::new(&sequence, runtime.config.background);
    let weak = Rc::downgrade(&runtime);
    sequence.after(ENGINE_STARTED | APP_KIT_FINISHED, move |_| {
        // Do not call the engine from within an engine or AppKit callback.
        main_queue::post(Box::new(move || {
            if let Some(runtime) = weak.upgrade() {
                runtime.open_initial_window();
            }
        }));
    });
    let delegate = EngineDelegate::new(runtime.clone(), State::new(), None);
    runtime.config.check_profile_directory()?;
    // SAFETY: the engine is loaded, mtm proves the thread, and the delegate and argv live until return.
    let code = unsafe { delegate.run(&argv) };
    if code == 0 && runtime.failed.get() {
        Ok(1)
    } else {
        Ok(code)
    }
}

struct Runtime {
    this: Weak<Self>,
    sequence: Rc<LaunchSequence>,
    config: LaunchConfig,
    session: Option<SavedSession>,
    observer: Observer,
    mtm: MainThreadMarker,
    window: RefCell<Option<BrowserWindow>>,
    failed: Cell<bool>,
}

impl Runtime {
    fn open_initial_window(&self) {
        if !self.sequence.engine_running() {
            return;
        }
        let Some(engine) = Engine::shared() else {
            return self.fail("engine did not become available");
        };
        if self.config.check_profile_directory().is_err() {
            return self.fail("launch profile directory changed");
        }
        let directory = self
            .session
            .as_ref()
            .and_then(|session| session.launch_profile_directory(self.config.data_directory()));
        let restore_hint = directory.is_some();
        let weak = self.this.clone();
        let name = NSString::from_str(directory.as_deref().unwrap_or("Default"));
        engine.load_profile(&name, move |profile| {
            let weak = weak.clone();
            later(move || {
                if let Some(runtime) = weak.upgrade()
                    && runtime.sequence.engine_running()
                {
                    let usable = |profile: &Retained<Profile>| {
                        !profile.off_the_record() && !profile.destroyed()
                    };
                    if let Some(profile) = profile.filter(usable) {
                        runtime.profile_ready(&profile, restore_hint);
                    } else if restore_hint
                        && let Some(profile) =
                            Engine::shared().map(|engine| engine.default_profile())
                        && usable(&profile)
                    {
                        runtime.profile_ready(&profile, false);
                    } else {
                        runtime.fail("could not load launch profile");
                    }
                }
            });
        });
    }

    fn profile_ready(&self, profile: &Profile, restore_hint: bool) {
        let reason = self.sequence.reason();
        let show = reason != LaunchReason::Hidden
            && !NSApplication::sharedApplication(self.mtm).isHidden();
        let session = self.session.as_ref().filter(|_| restore_hint);
        let layout = session
            .filter(|_| show && reason == LaunchReason::Normal)
            .and_then(|session| {
                early_window_layouts::read(&self.config, session, activation::dark(self.mtm))
            });
        self.sequence.reach(APP_READY);
        let url = session
            .and_then(|session| session.launch_tab.as_ref())
            .map_or("about:blank", |tab| tab.url.as_str());
        let window = BrowserWindow::open(
            profile,
            url,
            layout,
            show,
            self.config.background,
            &self.observer,
        );
        *self.window.borrow_mut() = Some(window);
    }

    fn fail(&self, message: &str) {
        self.failed.set(true);
        (self.observer)(LaunchEvent::Error(message.to_owned()));
        quit();
    }
}

impl Hooks for Runtime {
    fn engine_started(&self) {
        self.sequence.reach(ENGINE_STARTED);
        (self.observer)(LaunchEvent::EngineStarted);
    }

    fn engine_shut_down(&self) {
        self.sequence.engine_shutdown();
        (self.observer)(LaunchEvent::EngineWillShutDown);
    }

    fn emit(&self, _topic: &str, _data: Value) {}
    fn listens(&self, _topic: &str) -> bool {
        false
    }
    fn app_tab_id_value(&self, _tab: Option<&Tab>) -> Value {
        Value::Null
    }
    fn profile_name(&self, _profile: &Profile) -> String {
        String::new()
    }
    fn names_of_loaded_profile(&self, _profile: &Profile) -> Vec<String> {
        Vec::new()
    }
    fn ask_permission_in_popup_window(&self, _tab: &Tab, _request: &Value) -> bool {
        false
    }
    fn hold_until_hosted(&self, _tab: &Tab, _topic: &str, _payload: Value) {}
    fn drop_popup_window_permission(&self, _request_id: &str) {}
    fn drop_held_event(&self, _topic: &str, _request_id: &str) {}
    fn is_reserved_binding(&self, _key: &str, _unshifted_key: &str, _modifiers: usize) -> bool {
        false
    }
    fn is_reserved_key_event(&self, _event: &NSEvent) -> bool {
        false
    }
    fn window_for_new_browser(&self, _profile: &Profile, _kind: &str) -> Option<Retained<Window>> {
        None
    }
}

/// Schedule a main-thread action after the current engine callback returns.
pub fn later(action: impl FnOnce() + 'static) {
    main_queue::post(Box::new(action));
}

/// Schedule a deadline on the main thread.
pub fn after(delay: std::time::Duration, action: impl FnOnce() + 'static) {
    main_queue::after(delay, Box::new(action));
}
