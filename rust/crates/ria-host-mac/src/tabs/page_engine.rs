// Ported from ardia packages/ardiacore/ios/PageEngine.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! `PageEngine`, `ChromiumPageEngine` and the owner lookup of `PageEngines`.
//!
//! The data calls of a page engine are `ria_engine::domains::data::host::PageEngine`; the calls that go to both
//! engines (`clearBrowsingData`, `deleteProfileData`, `externalAppAllowances`, `removeExternalAppAllowance`) are in
//! `ria_engine::domains::data::page_engines`. This file adds the two transfer calls and the Chromium engine.
//! `WebKitPageEngine` belongs to slice 5.1.

use std::rc::Rc;

use ria_engine::call::main_queue;
use ria_engine::domains::data::host::{PageEngine as DataEngine, ServiceDone};
use serde_json::Value;

use crate::core::host;
use crate::core::services::Services;

/// The page engine as the page views need it: the data calls and the transfer calls.
pub trait PageEngine: DataEngine {
    fn prepare_transfer(&self, key: &str);
    fn release_transfer(&self, key: &str);
}

/// The live page views and the parked tabs, which `ArdiaWebView.prepareTransfer:` and `.releaseTransfer:` reach.
pub trait LiveViews {
    /// A live view that has a tab and this transfer key stops giving up its tab (`_life.handingOff = YES`).
    fn begin_handing_off(&self, key: &str);
    /// Every live view with this key stops handing off, and the parked tab of the key is released.
    fn release_transfer(&self, key: &str);
}

type Deleted = Box<dyn FnOnce(Vec<String>)>;
type DeleteNamed = Box<dyn Fn(&str, Deleted)>;

/// What `deleteProfileData` needs from other slices: `ProfileData.deleteDefault` and `ArdiaHost.deleteProfileData:`.
pub struct ProfileDeletion {
    pub default_profile: Box<dyn Fn(Deleted)>,
    pub named: DeleteNamed,
}

pub struct ChromiumPageEngine {
    services: Services,
    views: Rc<dyn LiveViews>,
    deletion: ProfileDeletion,
}

impl ChromiumPageEngine {
    pub fn new(services: Services, views: Rc<dyn LiveViews>, deletion: ProfileDeletion) -> Self {
        Self {
            services,
            views,
            deletion,
        }
    }
}

impl DataEngine for ChromiumPageEngine {
    fn owns(&self, _: &str) -> bool {
        true
    }

    fn resolve_permission(&self, id: &str, result: &str, remember: bool) {
        host::resolve_permission(id, result, remember);
    }

    fn resolve_external_app(&self, id: &str, open: bool, remember: bool) {
        host::resolve_external_app(id, open, remember);
    }

    fn download_command(&self, action: &str, id: &str) {
        self.services.download_command(action, id);
    }

    fn external_app_allowances(&self, profile: &str, done: Box<dyn FnOnce(Vec<Value>)>) {
        self.services.external_app_allowances(
            profile,
            Box::new(|reply| {
                done(match reply {
                    Ok(Value::Array(list)) => list,
                    _ => Vec::new(),
                });
            }),
        );
    }

    fn remove_external_app_allowance(&self, profile: &str, origin: &str, scheme: &str) {
        self.services
            .remove_external_app_allowance(profile, origin, scheme);
    }

    fn site_settings(&self, profile: &str, origin: &str, done: ServiceDone) {
        self.services.site_settings(profile, origin, done);
    }

    fn set_site_setting(&self, profile: &str, origin: &str, kind: &str, value: &str) {
        self.services.set_site_setting(value, profile, origin, kind);
    }

    fn clear_site_data(&self, profile: &str, origin: &str, done: ServiceDone) {
        self.services.clear_site_data(profile, origin, done);
    }

    fn clear_browsing_data(
        &self,
        profile: &str,
        types: &[String],
        since: f64,
        done: Box<dyn FnOnce(Option<String>)>,
    ) {
        let types: Vec<&str> = types.iter().map(String::as_str).collect();
        self.services
            .clear_browsing_data(profile, &types, since, done);
    }

    fn delete_profile_data(&self, profile: &str, done: Deleted) {
        if profile.is_empty() {
            (self.deletion.default_profile)(done);
        } else {
            (self.deletion.named)(profile, done);
        }
    }
}

impl PageEngine for ChromiumPageEngine {
    fn prepare_transfer(&self, key: &str) {
        if key.is_empty() {
            return;
        }
        let (views, key) = (self.views.clone(), key.to_owned());
        main_queue::post(Box::new(move || views.begin_handing_off(&key)));
    }

    fn release_transfer(&self, key: &str) {
        if key.is_empty() {
            return;
        }
        let (views, key) = (self.views.clone(), key.to_owned());
        main_queue::post(Box::new(move || views.release_transfer(&key)));
    }
}

/// `PageEngines.owner(of:)`: the first engine that owns the id. WebKit comes first, and Chromium owns what is left.
pub fn owner_of<'a>(
    web_kit: &'a Rc<dyn PageEngine>,
    chromium: &'a Rc<dyn PageEngine>,
    id: &str,
) -> &'a Rc<dyn PageEngine> {
    [web_kit, chromium]
        .into_iter()
        .find(|engine| engine.owns(id))
        .unwrap_or(chromium)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    struct Prefix(&'static str);

    impl DataEngine for Prefix {
        fn owns(&self, id: &str) -> bool {
            id.starts_with(self.0)
        }
        fn resolve_permission(&self, _: &str, _: &str, _: bool) {}
        fn resolve_external_app(&self, _: &str, _: bool, _: bool) {}
        fn download_command(&self, _: &str, _: &str) {}
        fn external_app_allowances(&self, _: &str, _: Box<dyn FnOnce(Vec<Value>)>) {}
        fn remove_external_app_allowance(&self, _: &str, _: &str, _: &str) {}
        fn site_settings(&self, _: &str, _: &str, _: ServiceDone) {}
        fn set_site_setting(&self, _: &str, _: &str, _: &str, _: &str) {}
        fn clear_site_data(&self, _: &str, _: &str, _: ServiceDone) {}
        fn clear_browsing_data(
            &self,
            _: &str,
            _: &[String],
            _: f64,
            _: Box<dyn FnOnce(Option<String>)>,
        ) {
        }
        fn delete_profile_data(&self, _: &str, _: Deleted) {}
    }

    impl PageEngine for Prefix {
        fn prepare_transfer(&self, _: &str) {}
        fn release_transfer(&self, _: &str) {}
    }

    #[test]
    fn webkit_ids_go_to_webkit_and_the_rest_to_chromium() {
        let web_kit: Rc<dyn PageEngine> = Rc::new(Prefix("wk-"));
        let chromium: Rc<dyn PageEngine> = Rc::new(Prefix(""));
        let seen = RefCell::new(Vec::new());
        for id in ["wk-1", "7", ""] {
            let owner = owner_of(&web_kit, &chromium, id);
            seen.borrow_mut().push(Rc::ptr_eq(owner, &web_kit));
        }
        assert_eq!(*seen.borrow(), [true, false, false]);
    }
}
