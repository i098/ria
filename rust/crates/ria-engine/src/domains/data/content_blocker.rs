// Ported from ardia packages/ardiacore/ios/ArdiaContentBlocker.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The content blocker: the bundled uBlock Origin as a component extension of each data profile. The default profile
//! holds the settings; every other profile follows it. Also ports `ArdiaContentBlocker.h` and
//! `ArdiaContentBlockerInternal.h`: the shared state lives in `ContentBlocker`, the host behind `BlockerHost`.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use std::time::Duration;

use serde_json::{Value, json};

use super::content_blocker_calls::{Op, script_with};
use super::content_blocker_lite::lite_settings_pending;
use super::content_blocker_page::{Done, ExtensionScripts, Page};
use super::host::{CoreProfile, Profile};
use super::profiles::original_profile_name;

pub const EXTENSION_ID: &str = "fajbdaomknpkicfjcpipdmcpoamdmilh";
pub const LITE_ID: &str = "bnjeokpoejhioagiokhkhmdogkhbnbki";
const BUNDLED_NAME: &str = "ublock";
pub const LITE_READER_NAME: &str = "ublock-lite-settings";
pub const LITE_MOVED_MARKER: &str = "uBlock Origin Lite settings moved";
/// The whitelist entry that turns uBlock Origin off everywhere.
pub const OFF_DIRECTIVE: &str = "/^(?:ardia-blocking-off|)/";
/// `kProfile`: the default profile, which holds the settings.
const PROFILE: &str = "";

const SETTLE_AFTER_LAUNCH: Duration = Duration::from_millis(1500);
const INHERIT_ATTEMPTS: u32 = 6;

/// What the content blocker needs from the host, besides the pages (`ExtensionScripts`). Main thread only.
pub trait BlockerHost {
    /// `ArdiaHost.isStarted`.
    fn is_started(&self) -> bool;
    /// `ardia_host::LoadedProfile`.
    fn loaded_profile(&self, name: &str) -> Option<Profile>;
    fn is_deleted_profile(&self, name: &str) -> bool;
    fn is_private_profile(&self, name: &str) -> bool;
    /// `ardia_host::WithProfile`.
    fn with_profile(&self, name: &str, done: Box<dyn FnOnce(Option<Profile>)>);
    /// `NSBundle.mainBundle.resourcePath`; the extensions are in its `Extensions` folder.
    fn resource_path(&self) -> Option<PathBuf>;
    /// `ArdiaHost.dataDirectory`.
    fn data_directory(&self) -> PathBuf;
    /// `-[ArdiaCoreProfile unloadComponentExtension:]`.
    fn unload_component_extension(&self, profile: &Profile, extension_id: &str);
    /// Posts the `ArdiaContentBlockerDidChange` notification with `change` as its user info.
    fn did_change(&self, change: Value);
    /// `ArdiaEvents emit:data:`.
    fn emit(&self, name: &str, data: Value);
    /// A monotonic clock for the deadlines of calls.
    fn now(&self) -> Duration;
    /// `crate::call::main_queue::after`.
    fn after(&self, delay: Duration, work: Box<dyn FnOnce()>);
    /// `crate::call::main_queue::post`.
    fn post(&self, work: Box<dyn FnOnce()>);
}

type ErrorDone = Box<dyn FnOnce(Option<String>)>;
type HiddenDone = Box<dyn FnOnce(Option<Vec<String>>, Option<String>)>;

#[derive(Default)]
pub(super) struct State {
    loaded_profiles: Vec<String>,
    loaded_into: HashMap<String, Weak<dyn CoreProfile>>,
    pub(super) pages: HashMap<String, Rc<Page>>,
    pub(super) queues: HashMap<String, VecDeque<Rc<Op>>>,
    pub(super) busy: HashSet<String>,
    pub(super) uses: HashMap<String, u64>,
    pub(super) fail_next_message: bool,
    delay_next_load: Duration,
    default_settled: bool,
    launched: bool,
    settled_waiters: Vec<Box<dyn FnOnce()>>,
    extension_path: Option<Option<PathBuf>>,
}

pub(super) struct Inner {
    pub(super) host: Rc<dyn BlockerHost>,
    pub(super) scripts: Rc<dyn ExtensionScripts>,
    pub(super) state: RefCell<State>,
}

/// `ArdiaContentBlocker`. Cheap to clone; every clone shares one state. Main thread only.
#[derive(Clone)]
pub struct ContentBlocker(pub(super) Rc<Inner>);

/// `Category`: the category of a list from its id and uBlock Origin group.
pub fn category(list_id: &str, group: &str) -> &'static str {
    match group {
        "custom" => "custom",
        "regions" => "regional",
        _ if list_id.contains("cookie") => "cookies",
        "privacy" => "trackers",
        _ if list_id == "ublock-privacy" => "trackers",
        "malware" => "security",
        _ if list_id == "ublock-badware" => "security",
        "annoyances" | "social" => "annoyances",
        _ => "ads",
    }
}

/// The version in a `manifest.json`: `version_name`, else `version`, else empty.
pub fn manifest_version(manifest: &str) -> String {
    let manifest: Value = serde_json::from_str(manifest).unwrap_or(Value::Null);
    ["version_name", "version"]
        .iter()
        .find_map(|key| manifest.get(key).and_then(Value::as_str))
        .unwrap_or_default()
        .to_owned()
}

fn count(value: Option<&Value>) -> f64 {
    value.and_then(Value::as_f64).unwrap_or(0.0)
}

fn truthy(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|n| n != 0.0),
        _ => false,
    }
}

/// `Strings`: the strings of a JSON array.
fn strings(value: Option<Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .into_iter()
            .filter_map(|item| item.as_str().map(str::to_owned))
            .collect(),
        _ => Vec::new(),
    }
}

/// The state that `State` gives from what the `STATE` script read, and the change it publishes (`None` when the read
/// failed).
pub fn shape_state(
    read: Option<&Value>,
    error: Option<&str>,
    version: &str,
) -> (Value, Option<Value>) {
    let Some(read_lists) = read.and_then(|r| r.get("lists")).and_then(Value::as_array) else {
        let state = json!({
            "enabled": false, "lists": [], "allowedHosts": [], "version": version,
            "stats": {"ready": false, "enabled": false, "parseMs": 0, "lookups": 0, "averageLookupMicros": 0,
                      "maxLookupMicros": 0, "error": error.unwrap_or("unavailable")},
        });
        return (state, None);
    };
    let read = read.unwrap_or(&Value::Null);
    let lists: Vec<Value> = read_lists
        .iter()
        .filter_map(|l| {
            let id = l.get("id")?.as_str()?;
            let group = l.get("group").and_then(Value::as_str).unwrap_or("");
            Some(json!({
                "id": id,
                "title": l.get("title").and_then(Value::as_str).unwrap_or(id),
                "category": category(id, group),
                "enabled": truthy(l.get("enabled")),
                "defaultOn": truthy(l.get("defaultOn")),
                "filters": count(l.get("filters")),
                "failed": truthy(l.get("failed")),
            }))
        })
        .collect();
    let allowed: Vec<Value> = read
        .get("allowedHosts")
        .and_then(Value::as_array)
        .map(|hosts| hosts.iter().filter(|h| h.is_string()).cloned().collect())
        .unwrap_or_default();
    let on = truthy(read.get("enabled"));
    let chosen: Vec<Value> = lists
        .iter()
        .filter(|l| l["enabled"] == true)
        .map(|l| l["id"].clone())
        .collect();
    let publish = json!({"enabled": on, "lists": chosen, "allowedHosts": allowed});
    let state = json!({
        "enabled": on,
        "version": version,
        "lists": lists,
        "allowedHosts": allowed,
        "stats": {
            "ready": true, "enabled": on, "networkFilters": count(read.get("networkFilters")),
            "cosmeticFilters": count(read.get("cosmeticFilters")), "parseMs": 0, "lookups": 0,
            "averageLookupMicros": 0, "maxLookupMicros": 0,
        },
    });
    (state, Some(publish))
}

const READ: &str = "Promise.all([lists(), trusted()]).then(([l, t]) => ({lists: selected(l).sort(), imported: imported(l), whitelist: [...t.whitelist].sort(), autoUpdate: l.autoUpdate !== false}))";

const FOLLOW: &str = concat!(
    "Promise.all([lists(), trusted()]).then(([l, t]) => {",
    "  const want = %@;",
    "  if (!Array.isArray(want.lists) || !Array.isArray(want.whitelist)) throw new Error('no settings to copy');",
    "  const mine = imported(l);",
    "  const wanted = Array.isArray(want.imported) ? want.imported : mine;",
    "  const toImport = wanted.filter((u) => !mine.includes(u));",
    "  const toRemove = mine.filter((u) => !wanted.includes(u));",
    "  const importsDiffer = toImport.length > 0 || toRemove.length > 0;",
    "  let written = Promise.resolve(null);",
    "  if (importsDiffer)",
    "    written = written.then(() => send({what: 'applyFilterListSelection', toImport: toImport.join('\\n'), toRemove}));",
    "  if (importsDiffer || !sameSet(selected(l), want.lists))",
    "    written = written.then(() => applyLists(want.lists));",
    "  if (!sameSet(t.whitelist, want.whitelist))",
    "    written = written.then(() => send({what: 'setWhitelist', whitelist: want.whitelist.join('\\n')}));",
    "  if ((l.autoUpdate !== false) !== (want.autoUpdate !== false))",
    "    written = written.then(() => send({what: 'userSettings', name: 'autoUpdate', value: want.autoUpdate !== false}));",
    "  return written.then(() => null);",
    "})",
);

const STATE: &str = concat!(
    "Promise.all([lists(), trusted(), fetch('/assets/assets.json').then((r) => r.json())]).then(([l, t, assets]) => {",
    "  const langs = new Set([chrome.i18n.getUILanguage(), ...(navigator.languages || [])].map((x) => (/^[a-z]+/.exec(x) || [])[0]).filter(Boolean));",
    "  const forLanguages = (lang) => typeof lang === 'string' && lang.split(/\\s+/).some((x) => langs.has(x));",
    "  const onByDefault = (id) => !!assets[id] && (assets[id].off !== true || forLanguages(assets[id].lang));",
    "  return {",
    "    lists: Object.entries(l.available)",
    "      .filter(([id, list]) => list && list.content === 'filters' && list.group !== 'user')",
    "      .map(([id, list]) => ({id, title: list.title || id, group: list.group || '', enabled: list.off !== true,",
    "        defaultOn: list.group !== 'custom' && onByDefault(id), filters: list.entryCount || 0,",
    "        failed: !!(l.cache && l.cache[id] && l.cache[id].error !== undefined)})),",
    "    networkFilters: l.netFilterCount || 0,",
    "    cosmeticFilters: l.cosmeticFilterCount || 0,",
    "    enabled: !t.whitelist.includes(offDirective),",
    "    allowedHosts: t.whitelist.filter((d) => !d.includes('/') && !d.startsWith('#') && !(t.whitelistDefault || []).includes(d)),",
    "  };",
    "})",
);

const HIDING_RULES: &str = concat!(
    "const hidingRule = /^[a-z0-9.-]+##[^+^\\s][^\\n{}]*$/;",
    "const savedHidingRule = /^[a-z0-9.-]+##(?![+^\\s])(?!.*\\/\\*)[^\\0-\\x1f\\x7f\\x85\\u2028\\u2029{}]+$/;",
    "const userFilters = () => chrome.extension.getBackgroundPage().µBlock.loadUserFilters().then((d) => {",
    "  if (!d || d.error || typeof d.content !== 'string') throw new Error(`user filters: ${d && d.error || 'unreadable'}`);",
    "  return d.content;",
    "});",
);

const READ_HIDDEN: &str = "userFilters().then((content) => content.split('\\n').map((l) => l.trim()).filter((l) => hidingRule.test(l)))";

const HIDE: &str = concat!(
    "Promise.resolve(%@).then((want) => {",
    "  for (const f of want) {",
    "    if (typeof f !== 'string' || f.length > 1024 || !savedHidingRule.test(f)) throw new Error('not a hiding rule');",
    "    document.createDocumentFragment().querySelector(f.slice(f.indexOf('##') + 2));",
    "  }",
    "  return userFilters().then((content) => {",
    "    const have = new Set(content.split('\\n').map((l) => l.trim()));",
    "    const fresh = [...new Set(want)].filter((f) => !have.has(f));",
    "    const ublock = chrome.extension.getBackgroundPage().µBlock;",
    "    if (!ublock.selectedFilterLists.includes(ublock.userFiltersPath))",
    "      ublock.applyFilterListSelection({toSelect: [ublock.userFiltersPath], merge: true});",
    "    return fresh.length ? ublock.appendUserFilters(fresh.join('\\n'), {}) : null;",
    "  });",
    "}).then(() => null)",
);

const RESTORE: &str = concat!(
    "Promise.resolve(new Set(%@)).then((drop) => userFilters().then((content) => {",
    "  const lines = content.split('\\n');",
    "  const kept = lines.filter((l) => !drop.has(l.trim()));",
    "  if (kept.length === lines.length) return null;",
    "  const ublock = chrome.extension.getBackgroundPage().µBlock;",
    "  return ublock.saveUserFilters(kept.join('\\n')).then(() => ublock.loadFilterLists());",
    "})).then(() => null)",
);

const SET_ENABLED: &str = concat!(
    "trusted().then((t) => {",
    "  const on = %@;",
    "  const kept = t.whitelist.filter((d) => d !== offDirective);",
    "  return send({what: 'setWhitelist', whitelist: (on ? kept : kept.concat([offDirective])).join('\\n')});",
    "}).then(() => send({what: 'userSettings', name: 'autoUpdate', value: %@}))",
);

const SET_LIST: &str = concat!(
    "lists().then((l) => {",
    "  const [id, on] = [%@, %@];",
    "  if (!l.available[id]) throw new Error(`no list ${id}`);",
    "  const keep = selected(l).filter((x) => x !== id);",
    "  return applyLists(on ? keep.concat([id]) : keep);",
    "})",
);

const ADD_LIST: &str = concat!(
    "lists().then((before) => {",
    "  const url = %@;",
    "  if (!/^https?:\\/\\/[^\\s\\/]+\\S*$/i.test(url) || url.length > 2048) throw new Error('not a list address');",
    "  const needle = url.replace(/^https?:/i, '');",
    "  const findId = (l) => l.available[url] ? url : Object.keys(l.available).find((k) =>",
    "    [].concat(l.available[k] && l.available[k].contentURL || []).some((u) => typeof u === 'string' && u.endsWith(needle)));",
    "  const had = findId(before);",
    "  return applySelection({toImport: url}, Infinity).then(lists).then((l) => {",
    "    const id = findId(l);",
    "    if (!id) throw new Error('uBlock Origin turned the list down');",
    "    const entry = l.available[id];",
    "    const cache = l.cache && l.cache[id];",
    "    return {id, custom: entry.group === 'custom', existed: !!had && before.available[had].off !== true,",
    "      filters: entry.entryCount || 0, failed: !!(cache && cache.error !== undefined)};",
    "  });",
    "})",
);

const REMOVE_LIST: &str = concat!(
    "lists().then((l) => {",
    "  const id = %@;",
    "  if (!l.available[id] || l.available[id].group !== 'custom') throw new Error(`no added list ${id}`);",
    "  return applySelection({toRemove: [id]});",
    "})",
);

const IS_ALLOWED: &str = "trusted().then((t) => t.whitelist.some((d) => covers(d, %@)))";

const SET_ALLOWED: &str = concat!(
    "trusted().then((t) => {",
    "  const [allowed, host] = [%@, %@];",
    "  const kept = t.whitelist.filter((d) => allowed ? d !== host : !covers(d, host));",
    "  return send({what: 'setWhitelist', whitelist: (allowed ? kept.concat([host]) : kept).join('\\n')});",
    "})",
);

const USER_FILTERS: &str = "send({what: 'readUserFilters'}).then((u) => ({content: typeof u.content === 'string' ? u.content : '', enabled: u.enabled !== false, trusted: !!u.trusted}))";

/// `WithHidingRules`: `body` with the hiding rule helpers in scope.
pub fn with_hiding_rules(body: &str) -> String {
    format!("(() => {{ {HIDING_RULES} return {body}; }})()")
}

impl ContentBlocker {
    pub fn new(host: Rc<dyn BlockerHost>, scripts: Rc<dyn ExtensionScripts>) -> Self {
        Self(Rc::new(Inner {
            host,
            scripts,
            state: RefCell::default(),
        }))
    }

    pub(super) fn is_loaded(&self, profile: &str) -> bool {
        self.0
            .state
            .borrow()
            .loaded_profiles
            .iter()
            .any(|p| p == profile)
    }

    fn launching(&self) -> bool {
        !self.0.state.borrow().launched
    }

    /// `StartPreparing`: finds the bundled extension once and deletes the copies of old builds.
    fn extension_path(&self) -> Option<PathBuf> {
        if let Some(path) = &self.0.state.borrow().extension_path {
            return path.clone();
        }
        let bundled = self
            .0
            .host
            .resource_path()
            .map(|r| r.join("Extensions").join(BUNDLED_NAME));
        let path = bundled.filter(|b| b.join("manifest.json").exists());
        self.0.state.borrow_mut().extension_path = Some(path.clone());
        if let Some(parent) = self.0.host.data_directory().parent() {
            let old_copies = parent.join("Built-in Extensions");
            std::thread::spawn(move || {
                if old_copies.exists() {
                    let _ = std::fs::remove_dir_all(old_copies);
                }
            });
        }
        path
    }

    /// `ardia_host::PrepareContentBlocker`.
    pub fn prepare(&self) {
        self.extension_path();
    }

    /// `ardia_host::LoadContentBlocker`: runs when a profile loads.
    pub fn load(&self, profile: &str) {
        if !self.0.host.is_private_profile(profile) {
            self.load_held(profile);
        }
    }

    pub(super) fn forget_if_gone(&self, profile: &str) {
        if !self.is_loaded(profile) {
            return;
        }
        let now = self.0.host.loaded_profile(profile);
        let into = self
            .0
            .state
            .borrow()
            .loaded_into
            .get(profile)
            .and_then(Weak::upgrade);
        if let (Some(now), Some(into)) = (&now, &into)
            && Rc::ptr_eq(now, into)
            && !self.0.host.is_deleted_profile(profile)
        {
            return;
        }
        let page = {
            let mut state = self.0.state.borrow_mut();
            state.loaded_profiles.retain(|p| p != profile);
            state.loaded_into.remove(profile);
            state.pages.remove(profile)
        };
        if let Some(page) = page {
            page.fail("profile gone");
        }
    }

    fn forget_gone_profiles(&self) {
        let loaded = self.0.state.borrow().loaded_profiles.clone();
        for profile in loaded {
            self.forget_if_gone(&profile);
        }
    }

    /// `LoadIntoProfile`: loads uBlock Origin into `profile` once.
    pub(super) fn load_into_profile(&self, profile: &str, then: ErrorDone) {
        let path = self.extension_path();
        if self.is_loaded(profile) {
            return then(None);
        }
        let (blocker, owned) = (self.clone(), profile.to_owned());
        self.0.host.with_profile(
            profile,
            Box::new(move |p| {
                if blocker.is_loaded(&owned) {
                    return then(None);
                }
                let (Some(p), Some(path)) = (p, path) else {
                    return then(Some("not loaded".into()));
                };
                let loaded = p.load_component_extension(&path.to_string_lossy());
                if loaded != EXTENSION_ID {
                    eprintln!("[blocker] component load failed ({loaded})");
                    return then(Some("not loaded".into()));
                }
                let mut state = blocker.0.state.borrow_mut();
                state.loaded_profiles.push(owned.clone());
                state.loaded_into.insert(owned, Rc::downgrade(&p));
                drop(state);
                then(None);
            }),
        );
    }

    fn when_default_settled(&self, then: Box<dyn FnOnce()>) {
        if self.0.state.borrow().default_settled {
            return then();
        }
        self.0.state.borrow_mut().settled_waiters.push(then);
    }

    fn default_settled(&self) {
        let waiters = {
            let mut state = self.0.state.borrow_mut();
            state.default_settled = true;
            state.launched = true;
            std::mem::take(&mut state.settled_waiters)
        };
        for waiter in waiters {
            waiter();
        }
    }

    /// `FollowDefault`: copies the settings of the default profile to `profiles` (all loaded profiles when `None`).
    fn follow_default(&self, profiles: Option<Vec<String>>, done: ErrorDone) {
        if !self.0.state.borrow().default_settled && self.0.host.is_started() {
            let blocker = self.clone();
            return self
                .when_default_settled(Box::new(move || blocker.follow_default(profiles, done)));
        }
        if profiles.is_none() {
            self.forget_gone_profiles();
        }
        let mut profiles =
            profiles.unwrap_or_else(|| self.0.state.borrow().loaded_profiles.clone());
        profiles.retain(|p| !p.is_empty());
        if profiles.is_empty() {
            return done(None);
        }
        let blocker = self.clone();
        self.call(
            PROFILE,
            READ.to_owned(),
            Box::new(move |settings, error| {
                if error.is_some() {
                    return done(error);
                }
                let Some(settings) = settings.filter(Value::is_object) else {
                    return done(Some("no settings to copy".into()));
                };
                let left = Rc::new(std::cell::Cell::new(profiles.len()));
                let failed = Rc::new(RefCell::new(None::<String>));
                let done = Rc::new(std::cell::Cell::new(Some(done)));
                for profile in profiles {
                    let (b, left, failed, done) =
                        (blocker.clone(), left.clone(), failed.clone(), done.clone());
                    blocker.call(
                        &profile.clone(),
                        script_with(FOLLOW, std::slice::from_ref(&settings)),
                        Box::new(move |_, error| {
                            let gone = b.0.host.loaded_profile(&profile).is_none()
                                || b.0.host.is_deleted_profile(&profile);
                            if !gone && failed.borrow().is_none() {
                                *failed.borrow_mut() = error;
                            }
                            left.set(left.get() - 1);
                            if left.get() == 0 {
                                let failed = failed.take();
                                let done = done.take();
                                b.0.host.post(Box::new(move || {
                                    if let Some(done) = done {
                                        done(failed);
                                    }
                                }));
                            }
                        }),
                        false,
                    );
                }
            }),
            true,
        );
    }

    /// `Inherit`: a new profile takes the default settings, retrying with backoff; `first` gets the first result.
    fn inherit(&self, profile: String, attempt: u32, first: Option<ErrorDone>) {
        let blocker = self.clone();
        self.follow_default(
            Some(vec![profile.clone()]),
            Box::new(move |error| {
                if let Some(first) = first {
                    first(error.clone());
                }
                let host = &blocker.0.host;
                let gone = host.loaded_profile(&profile).is_none()
                    || host.is_deleted_profile(&profile)
                    || !blocker.is_loaded(&profile);
                if error.is_none() || gone || attempt + 1 >= INHERIT_ATTEMPTS {
                    if let (Some(error), false) = (&error, gone) {
                        eprintln!("[blocker] profile \"{profile}\" didn't take the default profile's settings: {error}");
                    }
                    return;
                }
                let again = blocker.clone();
                host.after(
                    Duration::from_secs(1 << attempt),
                    Box::new(move || again.inherit(profile, attempt + 1, None)),
                );
            }),
        );
    }

    /// `LoadHeld`: loads uBlock Origin into `profile`, holding the navigations of a first load until it is ready.
    fn load_held(&self, profile: &str) {
        self.forget_gone_profiles();
        let host = self.0.host.clone();
        let p = host.loaded_profile(profile);
        let is_default = profile.is_empty();
        let first_load = p.is_some() && !self.is_loaded(profile);
        let hold = first_load
            && if is_default {
                p.as_ref().is_some_and(lite_settings_pending)
            } else {
                !self.launching()
            };
        if hold && let Some(p) = &p {
            p.hold_navigations_until_rulesets_of(EXTENSION_ID);
        }
        let release: Rc<dyn Fn(Option<String>)> = Rc::new(move |why| {
            let Some(p) = p.as_ref().filter(|_| hold) else {
                return;
            };
            match why {
                None => p.release_navigation_hold_ready(),
                Some(why) => p.release_navigation_hold(&why),
            }
        });
        let (blocker, profile) = (self.clone(), profile.to_owned());
        let load = move || {
            let then_blocker = blocker.clone();
            let then_profile = profile.clone();
            blocker.load_into_profile(
                &profile,
                Box::new(move |error| {
                    let (blocker, profile) = (then_blocker, then_profile);
                    if let Some(error) = error {
                        if is_default {
                            blocker.default_settled();
                        }
                        return release(Some(format!("the content blocker didn't load: {error}")));
                    }
                    if !is_default {
                        let b = blocker.clone();
                        return blocker.when_default_settled(Box::new(move || {
                            b.inherit(
                                profile,
                                0,
                                Some(Box::new(move |e| {
                                    release(e.map(|e| {
                                        format!("the default profile's settings didn't apply: {e}")
                                    }))
                                })),
                            )
                        }));
                    }
                    let b = blocker.clone();
                    let settle = move || {
                        let host = b.0.host.clone();
                        let current = host.loaded_profile(&profile);
                        if !hold && current.as_ref().is_some_and(|c| !lite_settings_pending(c)) {
                            return b.default_settled();
                        }
                        let (b2, profile2) = (b.clone(), profile.clone());
                        b.call(
                            &profile,
                            "null".into(),
                            Box::new(move |_, ready_error| {
                                match (ready_error, host.loaded_profile(&profile2)) {
                                    (Some(e), _) => {
                                        b2.default_settled();
                                        release(Some(format!("uBlock Origin isn't ready: {e}")))
                                    }
                                    (None, None) => {
                                        b2.default_settled();
                                        release(None)
                                    }
                                    (None, Some(loaded)) => {
                                        let b3 = b2.clone();
                                        b2.move_lite_settings(
                                            &profile2,
                                            &loaded,
                                            Box::new(move || {
                                                b3.default_settled();
                                                release(None)
                                            }),
                                        )
                                    }
                                }
                            }),
                            true,
                        );
                    };
                    if hold || !blocker.launching() {
                        return settle();
                    }
                    blocker.0.host.after(SETTLE_AFTER_LAUNCH, Box::new(settle));
                }),
            );
        };
        let delay = std::mem::take(&mut self.0.state.borrow_mut().delay_next_load);
        if !delay.is_zero() {
            host.after(delay, Box::new(load));
        } else {
            load();
        }
    }

    fn bundled_version(&self) -> String {
        self.extension_path()
            .and_then(|path| std::fs::read_to_string(path.join("manifest.json")).ok())
            .map(|manifest| manifest_version(&manifest))
            .unwrap_or_default()
    }

    /// `state:`: the lists, allowed hosts and stats of the default profile.
    pub fn state(&self, completion: Box<dyn FnOnce(Value)>) {
        if !self.0.state.borrow().default_settled && self.0.host.is_started() {
            let blocker = self.clone();
            return self.when_default_settled(Box::new(move || blocker.state(completion)));
        }
        let blocker = self.clone();
        self.call(
            PROFILE,
            STATE.to_owned(),
            Box::new(move |read, error| {
                let (state, publish) =
                    shape_state(read.as_ref(), error.as_deref(), &blocker.bundled_version());
                if let Some(publish) = publish {
                    blocker.0.host.did_change(publish);
                }
                completion(state);
            }),
            true,
        );
    }

    fn emit_changed(&self) {
        let blocker = self.clone();
        self.state(Box::new(move |state| {
            blocker
                .0
                .host
                .emit("contentBlocker.changed", state["stats"].clone())
        }));
    }

    /// `Changed`: other profiles follow before `completion` runs.
    fn changed(&self, error: Option<String>, completion: ErrorDone) {
        if error.is_some() {
            return completion(error);
        }
        let blocker = self.clone();
        self.follow_default(
            None,
            Box::new(move |follow_error| {
                blocker.emit_changed();
                completion(follow_error);
            }),
        );
    }

    /// `ListsChanged`: `completion` runs first; other profiles follow in the background.
    fn lists_changed(&self, error: Option<String>, completion: ErrorDone) {
        if error.is_some() {
            return completion(error);
        }
        completion(None);
        self.emit_changed();
        self.follow_default(
            None,
            Box::new(|e| {
                if let Some(e) = e {
                    eprintln!("[blocker] the other profiles didn't take the list change: {e}");
                }
            }),
        );
    }

    fn call_default(
        &self,
        script: String,
        then: impl FnOnce(&Self, Option<Value>, Option<String>) + 'static,
    ) {
        let blocker = self.clone();
        self.call(
            PROFILE,
            script,
            Box::new(move |v, e| then(&blocker, v, e)),
            true,
        );
    }

    pub fn set_enabled(&self, enabled: bool, completion: ErrorDone) {
        self.call_default(
            script_with(SET_ENABLED, &[enabled.into(), enabled.into()]),
            move |b, _, error| {
                if error.is_none() {
                    b.0.host.did_change(json!({"enabled": enabled}));
                }
                b.changed(error, completion);
            },
        );
    }

    pub fn set_list_enabled(&self, list_id: &str, enabled: bool, completion: ErrorDone) {
        let id = list_id.to_owned();
        self.call_default(
            script_with(SET_LIST, &[list_id.into(), enabled.into()]),
            move |b, _, error| {
                if error.is_none() {
                    b.0.host
                        .did_change(json!({"list": id, "listEnabled": enabled}));
                }
                b.lists_changed(error, completion);
            },
        );
    }

    pub fn add_list(&self, url: &str, completion: Done) {
        self.call_default(
            script_with(ADD_LIST, &[url.into()]),
            move |b, added, error| {
                let added = added.filter(Value::is_object);
                let error = error.or_else(|| added.is_none().then(|| "no answer".to_owned()));
                if let (None, Some(added)) = (&error, &added) {
                    b.0.host
                        .did_change(json!({"list": added["id"], "listEnabled": true}));
                }
                b.lists_changed(
                    error,
                    Box::new(move |e| match e {
                        Some(e) => completion(None, Some(e)),
                        None => completion(added, None),
                    }),
                );
            },
        );
    }

    pub fn remove_list(&self, list_id: &str, completion: ErrorDone) {
        let id = list_id.to_owned();
        self.call_default(
            script_with(REMOVE_LIST, &[list_id.into()]),
            move |b, _, error| {
                if error.is_none() {
                    b.0.host
                        .did_change(json!({"list": id, "listEnabled": false}));
                }
                b.lists_changed(error, completion);
            },
        );
    }

    pub fn is_allowed(&self, host: &str, completion: Box<dyn FnOnce(bool)>) {
        self.call_default(
            script_with(IS_ALLOWED, &[host.to_lowercase().into()]),
            move |_, allowed, error| {
                completion(
                    error.is_none() && allowed.as_ref().and_then(Value::as_bool) == Some(true),
                )
            },
        );
    }

    pub fn set_allowed(&self, allowed: bool, host: &str, completion: ErrorDone) {
        let host = host.to_lowercase();
        let script = script_with(SET_ALLOWED, &[allowed.into(), host.clone().into()]);
        self.call_default(script, move |b, _, error| {
            if error.is_none() {
                b.0.host
                    .did_change(json!({"host": host, "allowed": allowed}));
            }
            b.changed(error, completion);
        });
    }

    pub fn user_filters(&self, completion: Done) {
        self.call_default(USER_FILTERS.to_owned(), move |_, value, error| {
            let value = value.filter(Value::is_object);
            let error = error.or_else(|| value.is_none().then(|| "no user filters".to_owned()));
            completion(value, error)
        });
    }

    pub fn hidden_elements(&self, profile: &str, completion: HiddenDone) {
        self.call(
            &original_profile_name(profile),
            with_hiding_rules(READ_HIDDEN),
            Box::new(move |filters, error| match error {
                Some(error) => completion(None, Some(error)),
                None => completion(Some(strings(filters)), None),
            }),
            true,
        );
    }

    pub fn hide_elements(&self, filters: &[String], profile: &str, completion: ErrorDone) {
        if self.0.host.is_private_profile(profile) {
            return completion(Some(
                "a private window keeps its hidden elements for the session".into(),
            ));
        }
        if filters.is_empty() {
            return completion(None);
        }
        self.edit_user_filters(profile, script_with(HIDE, &[filters.into()]), completion);
    }

    pub fn restore_elements(&self, filters: &[String], profile: &str, completion: ErrorDone) {
        if filters.is_empty() {
            return completion(None);
        }
        let profile = original_profile_name(profile);
        self.edit_user_filters(
            &profile,
            script_with(RESTORE, &[filters.into()]),
            completion,
        );
    }

    fn edit_user_filters(&self, profile: &str, script: String, completion: ErrorDone) {
        let blocker = self.clone();
        self.call(
            profile,
            with_hiding_rules(&script),
            Box::new(move |_, error| {
                if error.is_none() {
                    blocker.0.host.did_change(json!({"userFilters": true}));
                }
                completion(error);
            }),
            true,
        );
    }

    pub fn dev_fail_next_message(&self) {
        self.0.state.borrow_mut().fail_next_message = true;
    }

    pub(super) fn dev_delay_next_load(&self, delay: Duration) {
        self.0.state.borrow_mut().delay_next_load = delay;
    }
}

#[cfg(test)]
#[path = "content_blocker_tests.rs"]
pub(crate) mod tests;
