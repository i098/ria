// Ria original, no Ardia source.

//! What the data domains need from the host. `ArdiaCoreProfile`, `ArdiaCoreEngine` and `ArdiaEvents` belong to slice 3.5
//! (`ria-host-mac`); the domains reach them through these traits, so this crate stays free of Objective-C and tests run
//! on any system. Everything runs on the main thread, so the traits are not `Send` and callbacks are `'static`.

use std::rc::Rc;

use serde_json::Value;

/// `ArdiaCoreProfile` (`ArdiaCore.h`): one loaded Chromium profile.
pub trait CoreProfile {
    /// The profile folder that every engine call takes (`ArdiaCoreProfile.path`).
    fn path(&self) -> String;
    /// The Chromium folder name, for example `Default` or `Profile work` (`ArdiaCoreProfile.name`).
    fn directory_name(&self) -> String;
    fn off_the_record(&self) -> bool;
    fn destroyed(&self) -> bool;

    fn passwords_offered(&self) -> bool;
    fn set_passwords_offered(&self, on: bool);
    fn addresses_autofilled(&self) -> bool;
    fn set_addresses_autofilled(&self, on: bool);
    fn cards_autofilled(&self) -> bool;
    fn set_cards_autofilled(&self, on: bool);
    fn preload_pages(&self) -> bool;
    fn set_preload_pages(&self, on: bool);

    /// `fetchIcon:maxBytes:completion:`: the bytes of the icon, or `None`.
    fn fetch_icon(&self, url: &str, max_bytes: usize, done: Box<dyn FnOnce(Option<Vec<u8>>)>);
    /// `loadComponentExtension:`: the id of the loaded extension, or an empty string when it did not load.
    fn load_component_extension(&self, path: &str) -> String;
    /// `holdNavigationsUntilRulesetsOf:`.
    fn hold_navigations_until_rulesets_of(&self, extension_id: &str);
    /// `releaseNavigationHoldReady`.
    fn release_navigation_hold_ready(&self);
    /// `releaseNavigationHold:`.
    fn release_navigation_hold(&self, reason: &str);
}

pub type Profile = Rc<dyn CoreProfile>;

/// `ArdiaCoreEngine` (`ArdiaCore.h`), the part that the profile code uses.
pub trait CoreEngine {
    /// `defaultProfile`, when it is loaded.
    fn default_profile(&self) -> Option<Profile>;
    /// `loadProfile:completion:` with a Chromium folder name.
    fn load_profile(&self, directory: &str, done: Box<dyn FnOnce(Option<Profile>)>);
    /// `offTheRecordProfileFor:completion:`: the private profile, or an error text.
    fn off_the_record_profile_for(
        &self,
        base: &Profile,
        done: Box<dyn FnOnce(Option<Profile>, Option<String>)>,
    );
    /// `releaseProfile:`.
    fn release_profile(&self, profile: &Profile);
}

/// `ArdiaEvents emit:data:`: sends an event to the app. The mover rewrite of `downloads.changed` is the job of the caller.
pub trait EventSink {
    fn emit(&self, name: &str, data: Value);
}

/// What runs when a profile is loaded (`ArdiaProfiles.mm:WithProfile`): the content blocker and the download watch.
pub trait ProfileLoadHooks {
    fn profile_loaded(&self, name: &str);
}

/// The answer of a service: a value (`Null` for nothing) or an error text. It settles once.
pub type ServiceDone = Box<dyn FnOnce(Result<Value, String>)>;

/// `ArdiaServices` and the other native helpers that the call tables use (`ArdiaServices.mm`, `ArdiaHost`,
/// `ArdiaWebView`, `WebKitHub`): they belong to slice 3.5 and later. `op` is the name of the Swift method, for example
/// `listPasswords`; `args` is a JSON object with the arguments of the method, in the order and under the names of
/// `ArdiaServices.h`. A host settles `done` once, on the main thread.
pub trait Services {
    fn run(&self, op: &str, profile: &str, args: Value, done: ServiceDone);
}

/// `PageEngine` of `PageEngine.swift`: what one page engine (Chromium or WebKit) does for the data calls.
pub trait PageEngine {
    fn owns(&self, id: &str) -> bool;
    fn resolve_permission(&self, id: &str, result: &str, remember: bool);
    fn resolve_external_app(&self, id: &str, open: bool, remember: bool);
    fn download_command(&self, action: &str, id: &str);
    fn external_app_allowances(&self, profile: &str, done: Box<dyn FnOnce(Vec<Value>)>);
    fn remove_external_app_allowance(&self, profile: &str, origin: &str, scheme: &str);
    fn site_settings(&self, profile: &str, origin: &str, done: ServiceDone);
    fn set_site_setting(&self, profile: &str, origin: &str, kind: &str, value: &str);
    fn clear_site_data(&self, profile: &str, origin: &str, done: ServiceDone);
    fn clear_browsing_data(
        &self,
        profile: &str,
        types: &[String],
        since: f64,
        done: Box<dyn FnOnce(Option<String>)>,
    );
    fn delete_profile_data(&self, profile: &str, done: Box<dyn FnOnce(Vec<String>)>);
}
