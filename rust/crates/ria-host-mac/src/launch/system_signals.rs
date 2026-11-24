// Ported from ardia packages/shell/ios/SystemSignalsModule.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The product host of the `ArdiaSystemSignals` functions that reach the download mover:
//! `setApprovedDownloadFolders` and `moveCompletedDownload` (`SystemSignalsModule.swift:100-108`), which JS calls at
//! `decideHooks.ts:116`.
//!
//! Changes from Ardia:
//! - Ardia has an Expo module that JS calls. Ria has no page layer yet, so [`SystemSignals::call`] takes the
//!   function name and its positional arguments as JSON, as the module bridge would pass them. No page layer calls it until issue 101.
//! - `ArdiaEvents.emit` has no consumer in the app yet. [`ProductHost::emit`] hands each event to the emit
//!   callback that the launch gives to [`SystemSignals::start`].
//! - The other module functions (`watchIdle`, `holdsIncomingLinks`, ...) are not here; each answers `Err`.

use std::cell::RefCell;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2_foundation::NSString;
use ria_engine::call::{ActionQueue, Engine as CallEngine, main_queue};
use serde_json::Value;

use crate::core::bindings::{Engine, Profile};
use crate::core::services::{Reply, ServiceHost, Services};

thread_local! {
    static CURRENT: RefCell<Option<Rc<SystemSignals>>> = const { RefCell::new(None) };
}

/// The running app's system-signals host, after `engineDidStart` and before `engineWillShutDown`.
pub fn current() -> Option<Rc<SystemSignals>> {
    CURRENT.with_borrow(Clone::clone)
}

type Emit = Rc<dyn Fn(&str, Value)>;

/// What `ArdiaServices` reads from the rest of the host, for the app: one profile per directory name, no private
/// profiles and no app tabs yet.
struct ProductHost {
    emit: Emit,
}

impl ServiceHost for ProductHost {
    fn is_private(&self, _profile: &str) -> bool {
        false
    }
    fn original_name(&self, profile: &str) -> String {
        profile.to_owned()
    }
    fn with_profile(&self, name: &str, done: Box<dyn FnOnce(Option<Retained<Profile>>)>) {
        let Some(engine) = Engine::shared() else {
            return done(None);
        };
        if name.is_empty() {
            return done(Some(engine.default_profile()));
        }
        // The engine's completion is a `Fn`; `done` runs once.
        let done = RefCell::new(Some(done));
        engine.load_profile(&NSString::from_str(name), move |profile| {
            if let Some(done) = done.take() {
                done(profile);
            }
        });
    }
    fn emit(&self, topic: &str, data: Value) {
        (self.emit)(topic, data);
    }
    fn app_tab_ids(&self, _chrome_tab_id: i64) -> Vec<Value> {
        Vec::new()
    }
    fn forget_early_window_pictures(&self) {}
}

pub struct SystemSignals {
    services: Services,
}

impl SystemSignals {
    /// Call on `engineDidStart`, on the main thread. Makes the call engine and the services, and starts the
    /// `downloads.changed` feed that the mover reads.
    pub(super) fn start(emit: Emit) {
        let queue = ActionQueue::new();
        let weak = Rc::downgrade(&queue);
        queue.set_waker(move || {
            let weak = weak.clone();
            main_queue::post(Box::new(move || _ = weak.upgrade().map(|q| q.drain())));
        });
        let engine = CallEngine::new(queue);
        engine.did_start();
        let services = Services::new(engine, Rc::new(ProductHost { emit }));
        services.watch_download_events();
        CURRENT.set(Some(Rc::new(Self { services })));
    }

    /// Call on `engineWillShutDown`.
    pub(super) fn stop() {
        CURRENT.set(None);
    }

    /// `ArdiaServices watchDownloads:` for a loaded profile: emits its listed downloads and starts its events.
    pub(super) fn watch_downloads(&self, profile: &str) {
        self.services.watch_downloads(profile);
    }

    /// One function of the module. `args` is the positional argument array of the JS call. `reply` runs once: the
    /// synchronous function `setApprovedDownloadFolders` answers `null` at once.
    pub fn call(&self, name: &str, args: &Value, reply: Reply) {
        match (name, args.as_array().map(Vec::as_slice)) {
            ("setApprovedDownloadFolders", Some([folders])) => match strings(folders) {
                Some(folders) => {
                    self.services.set_approved_download_folders(&folders);
                    reply(Ok(Value::Null));
                }
                None => reply(Err("folders must be an array of strings".into())),
            },
            ("moveCompletedDownload", Some([Value::String(path), Value::String(folder)])) => {
                self.services.move_completed_download(path, folder, reply);
            }
            ("setApprovedDownloadFolders" | "moveCompletedDownload", _) => {
                reply(Err(format!("{name}: bad arguments")));
            }
            _ => reply(Err(format!("{name}: not a system signal of this host"))),
        }
    }
}

fn strings(value: &Value) -> Option<Vec<String>> {
    value
        .as_array()?
        .iter()
        .map(|v| v.as_str().map(str::to_owned))
        .collect()
}
