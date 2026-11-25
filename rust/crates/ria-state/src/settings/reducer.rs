// Ported from ardia apps/browser/src/state/settings/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.
//
// Sources: actions.ts, settingsWindows.ts, searchState.ts, interfaceScale.ts, interfaceScaleSync.ts, nativeSettings.ts,
// growingRows.ts, paneData.ts and the store actions of settings.ts.

//! The reducer of the `settings` module: `apply` for every `Action`.
//!
//! Changes from Ardia:
//! - The TypeScript files keep several stores (`useSettingsNav`, `useSettingsSearch`, `useInterfaceScale`,
//!   `useSettingsSheet`, `useImportRequest`, `useScrollEnds`) and some module variables. Here the store fields are fields of
//!   `State`, and the module variables are in `State::run`, which is not in the JSON.
//! - `Date.now()` has no source in a reducer. The actions that read the clock carry a `now` field; the caller fills it.
//! - `Action::UpdateSettings.patch` is an `OrderedPatch`, not `onboarding::StateDraft`: `Object.keys(patch)` order decides
//!   `settingsMeta.chosen`, and a wrong-typed value must reach the validator, as it does in Ardia.
//! - A function that returns a value (`addCustomEngine`, `updateCustomEngine`, `takePreferredImportSource`) writes its
//!   effect to the state only; the caller reads the result from the state.
//! - Telemetry notes (`noteSettingsSearch`, `noteSettingsPick`, `noteAutoHideTabsChosen`, ...) are not called: the
//!   telemetry module has no note API in the slice 2.4 skeleton.
//! - Native results arrive as later actions. `startInterfaceScale` cannot read the native scale in the same call, so it
//!   compares the stored scale with `State::scale`, and `refused` is never set (the promise error is not seen here).
//! - Sibling functions that the skeleton does not expose are not called: `toggleTabLayout`, `newTab`,
//!   `setAsDefaultBrowser`, the autofill "asked" document and the telemetry notes. The sequences that
//!   need them are in `rust/replay/settings/PENDING.md`.
//! - `startNativeSettings` does not send `ArdiaSystem.setThinScrollBars`: `PlatformCall` has no variant for it yet, and
//!   the replay test drops that call from the Ardia record.
//! - `browser::State` is empty, so `extensionsProfileId` keeps a given profile without a check, and `updateSettings` does not
//!   clear the per-window `autoHideTabs`.
//! - `paneNeeds` skips the resources that have no `PlatformCall` yet (`loginItem`, `defaultBrowser`, `syncFolder`, the
//!   extension list).

use std::fmt;

use ria_core::search::engines::{
    CustomSearchEngine, EngineInput, ExtensionSearchEngine, SearchEngine, all_engines,
    make_custom_engine, valid_search_template, validate_engine,
};
use ria_core::search::js_text::js_trim;
use serde::de::{DeserializeOwned, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value, json};

use super::specs::{Patch, choose, settled_patch, with_link_keys_apart, writable_patch};
use super::{
    Action, GoToSettingsPanePane, Settings, SettingsMeta, SettingsPlace, SettingsPlacePane, State,
    StateFlash, StatePane, StateReveal, Target,
};
use crate::kernel::{AppState, Effects, Number};
use crate::platform::PlatformCall;
use crate::shared::interface_scale::{
    INTERFACE_SCALE, same_interface_scale, stepped_interface_scale,
};
use crate::shared::js::{js_max, js_min};
use crate::shared::press_motion::{PressMotion, set_press_motion};

const HISTORY_LIMIT: usize = 30;
const NEAR_END: f64 = 600.0;
const SETTINGS_WINDOW_ID: &str = "settings";
const IMPORT_WINDOW_ID: &str = "import";
const APP_ICON_SIZE: f64 = 56.0;
const NATIVE_SETTINGS: [&str; 8] = [
    "appearance",
    "autoPictureInPicture",
    "videoUpscaling",
    "webKitFraudWarnings",
    "popupsOpenAsTabs",
    "blockAutoplayEverywhere",
    "pressMotion",
    "commandScrollZoom",
];

/// A settings patch with the key order of its JSON text.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OrderedPatch(pub Patch);

impl Serialize for OrderedPatch {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_map(self.0.iter().map(|(k, v)| (k, v)))
    }
}

impl<'de> Deserialize<'de> for OrderedPatch {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Entries;
        impl<'de> Visitor<'de> for Entries {
            type Value = OrderedPatch;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<OrderedPatch, A::Error> {
                let mut patch: Patch = Vec::new();
                while let Some((key, value)) = map.next_entry::<String, Value>()? {
                    // A repeated key keeps its first place and takes the last value, as `JSON.parse` does.
                    match patch.iter_mut().find(|(k, _)| *k == key) {
                        Some(entry) => entry.1 = value,
                        None => patch.push((key, value)),
                    }
                }
                Ok(OrderedPatch(patch))
            }
        }
        deserializer.deserialize_map(Entries)
    }
}

/// The module variables of the TypeScript files (see the module doc).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Run {
    /// `armed` of `growingRows.ts`, inverted so that the default is armed.
    scroll_disarmed: bool,
    /// `lastContent` of `growingRows.ts`; `None` is `-1`.
    last_content: Option<f64>,
    /// `engineCounter` of `settings.ts`.
    engine_counter: u64,
    /// `startNativeSettings` ran.
    native_started: bool,
    /// `sent` of `nativeSettings.ts`.
    native_sent: Vec<(String, Value)>,
    /// `startInterfaceScale` ran.
    scale_started: bool,
    /// `asked` of `interfaceScaleSync.ts`.
    asked: Option<f64>,
    /// `refused` of `interfaceScaleSync.ts`.
    refused: Option<f64>,
    /// The cache of `resources.ts`, keyed by the JSON of `[name, ...args]`.
    resources: Vec<(String, Value)>,
}

fn convert<A: Serialize, B: DeserializeOwned + Default>(a: &A) -> B {
    serde_json::to_value(a)
        .ok()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

fn text_of<A: Serialize>(a: &A) -> String {
    match serde_json::to_value(a) {
        Ok(Value::String(s)) => s,
        _ => String::new(),
    }
}

fn settings_map(s: &State) -> Map<String, Value> {
    match serde_json::to_value(&s.settings) {
        Ok(Value::Object(m)) => m,
        _ => Map::new(),
    }
}

fn put_settings(s: &mut State, map: Map<String, Value>, meta: SettingsMeta) {
    if let Ok(next) = serde_json::from_value::<Settings>(Value::Object(map)) {
        s.settings = next;
    }
    s.settings_meta = meta;
}

fn build_id(now: f64, counter: u64) -> String {
    fn base36(mut n: u64) -> String {
        if n == 0 {
            return "0".into();
        }
        let mut out = Vec::new();
        while n > 0 {
            out.push(char::from_digit((n % 36) as u32, 36).unwrap_or('0'));
            n /= 36;
        }
        out.iter().rev().collect()
    }
    let ms = if now.is_finite() && now >= 0.0 {
        now as u64
    } else {
        0
    };
    format!("engine-{}-{}", base36(ms), base36(counter))
}

/// `searchEngines` (`settings.ts:119`).
fn search_engines(settings: &Map<String, Value>) -> Vec<SearchEngine> {
    let text = |e: &Value, k: &str| {
        e.get(k)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let list = |key: &str| {
        settings
            .get(key)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    let custom: Vec<CustomSearchEngine> = list("customSearchEngines")
        .iter()
        .map(|e| CustomSearchEngine {
            id: text(e, "id"),
            name: text(e, "name"),
            keyword: text(e, "keyword"),
            url: text(e, "url"),
        })
        .collect();
    let extension: Vec<ExtensionSearchEngine> = list("extensionSearchEngines")
        .iter()
        .map(|e| ExtensionSearchEngine {
            profile: text(e, "profile"),
            extension_id: text(e, "extensionId"),
            extension_name: text(e, "extensionName"),
            name: text(e, "name"),
            keyword: text(e, "keyword"),
            url: text(e, "url"),
            suggest_url: e
                .get("suggestUrl")
                .and_then(Value::as_str)
                .map(str::to_owned),
            is_default: e.get("isDefault").and_then(Value::as_bool).unwrap_or(false),
        })
        .collect();
    let mut engines = all_engines(&custom, &extension);
    let legacy = settings
        .get("customSearchUrl")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if valid_search_template(legacy) {
        let at = engines
            .iter()
            .position(|e| e.extension.is_some())
            .unwrap_or(engines.len());
        engines.insert(
            at,
            SearchEngine {
                id: "custom".into(),
                name: "Custom".into(),
                keyword: "custom".into(),
                url: legacy.to_owned(),
                suggest_url: None,
                custom: true,
                extension: None,
            },
        );
    }
    engines
}

fn engine_input(input: &Value) -> EngineInput {
    let text = |k: &str| {
        input
            .get(k)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    EngineInput {
        name: text("name"),
        keyword: text("keyword"),
        url: text("url"),
    }
}

fn engine_json(e: &CustomSearchEngine) -> Value {
    json!({ "id": e.id, "name": e.name, "keyword": e.keyword, "url": e.url })
}

fn stored_engines(settings: &Map<String, Value>) -> Vec<Value> {
    settings
        .get("customSearchEngines")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// `updateSettings` (`settings.ts:318`).
fn update_settings(
    app: &mut AppState,
    raw: Patch,
    from_sync: bool,
    inferred: bool,
    fx: &mut Effects,
) {
    let (patch, _rejected) = writable_patch(&raw);
    let (current, meta) = {
        let s = app.get::<State>();
        (settings_map(s), s.settings_meta.clone())
    };
    let auto_hide = patch
        .iter()
        .find(|(k, _)| k == "autoHideTabs")
        .map(|(_, v)| v);
    if auto_hide.is_some_and(|v| Some(v) != current.get("autoHideTabs")) {
        forget_window_auto_hide(app);
    }
    let (next, meta) = if from_sync {
        let mut next = current.clone();
        for (k, v) in settled_patch(&current, &patch) {
            next.insert(k, v);
        }
        (next, meta)
    } else {
        let (next, meta, _) = choose(
            &current,
            &meta,
            &with_link_keys_apart(&current, patch),
            inferred,
        );
        (next, meta)
    };
    let s = app.get_mut::<State>();
    put_settings(s, next, meta);
    follow_settings(s, fx);
}

/// `windowsFollowingAutoHideTabs` (`settings.ts:306`) has nothing to clear yet: `browser::State` is still empty, so the
/// windows are not in `AppState`. The browser slice must clear `autoHideTabs` of every window when this runs.
fn forget_window_auto_hide(_app: &mut AppState) {}

/// The two subscriptions that follow `settings`: `startNativeSettings` and `startInterfaceScale`.
fn follow_settings(s: &mut State, fx: &mut Effects) {
    if s.run.scale_started {
        follow_scale(s, fx);
    }
    if s.run.native_started {
        sync_native(s, fx);
    }
}

fn stored_scale_wins(s: &State) -> bool {
    s.settings_meta.chosen.iter().any(|k| k == "interfaceScale")
        || s.settings.interface_scale.0 != INTERFACE_SCALE.default
}

fn push_scale(s: &mut State, scale: f64, fx: &mut Effects) {
    s.run.asked = Some(scale);
    s.run.refused = None;
    fx.push(PlatformCall::AppSetInterfaceScale {
        scale: Number(scale),
    });
}

/// The subscription of `startInterfaceScale` (`interfaceScaleSync.ts:34`).
fn follow_scale(s: &mut State, fx: &mut Effects) {
    if !stored_scale_wins(s) {
        return;
    }
    let wanted = s.settings.interface_scale.0;
    let retry = s
        .run
        .refused
        .is_some_and(|r| same_interface_scale(wanted, r));
    if retry || !same_interface_scale(wanted, s.run.asked.unwrap_or(s.scale.0)) {
        push_scale(s, wanted, fx);
    }
}

/// `sync` of `startNativeSettings` (`nativeSettings.ts:28`).
fn sync_native(s: &mut State, fx: &mut Effects) {
    let map = settings_map(s);
    for key in NATIVE_SETTINGS {
        let value = map.get(key).cloned().unwrap_or(Value::Null);
        match s.run.native_sent.iter_mut().find(|(k, _)| k == key) {
            Some((_, sent)) if *sent == value => continue,
            Some((_, sent)) => *sent = value.clone(),
            None => s.run.native_sent.push((key.to_owned(), value.clone())),
        }
        let on = value.as_bool().unwrap_or(false);
        match key {
            "appearance" => fx.push(PlatformCall::ShellSetAppearance { mode: value }),
            "autoPictureInPicture" => {
                fx.push(PlatformCall::WebViewSetAutoPictureInPicture { enabled: on })
            }
            "videoUpscaling" => fx.push(PlatformCall::WebViewSetVideoUpscaling { enabled: on }),
            "webKitFraudWarnings" => fx.push(PlatformCall::WebKitSetFraudWarnings { enabled: on }),
            "popupsOpenAsTabs" => fx.push(PlatformCall::WebViewSetPopupsOpenAsTabs { enabled: on }),
            "blockAutoplayEverywhere" => {
                fx.push(PlatformCall::WebViewSetBlockAutoplayEverywhere { enabled: on })
            }
            "commandScrollZoom" => {
                fx.push(PlatformCall::WebViewSetCommandScrollZoom { enabled: on })
            }
            _ => set_press_motion(if value.as_str() == Some("bounce") {
                PressMotion::Bounce
            } else {
                PressMotion::Still
            }),
        }
    }
}

fn place_of(s: &State) -> SettingsPlace {
    SettingsPlace {
        pane: convert(&s.pane),
        profile_id: s.profile_id.clone(),
        detail: s.detail.clone(),
    }
}

/// `goToSettingsPane` (`settingsWindows.ts:28`).
fn go_to_pane(s: &mut State, pane: StatePane, profile_id: Option<String>, detail: Option<String>) {
    if s.pane == pane && s.profile_id == profile_id && s.detail == detail {
        return;
    }
    let current = place_of(s);
    s.back.push(current);
    if s.back.len() > HISTORY_LIMIT {
        s.back.drain(..s.back.len() - HISTORY_LIMIT);
    }
    s.pane = pane;
    s.profile_id = profile_id;
    s.detail = detail;
    s.forward.clear();
}

/// `extensionsProfileId` (`extensions/store.ts:66`). The profiles and windows are not in `AppState` yet (`browser::State` is
/// empty), so a given profile is kept as it is and the window lookups are skipped.
fn extensions_profile_id(app: &AppState, profile: Option<&str>) -> String {
    match profile.filter(|p| !p.is_empty()) {
        Some(p) => p.to_owned(),
        None => app.get::<State>().settings.default_profile_id.clone(),
    }
}

/// `settingsProfileFor` (`extensions/store.ts:73`).
fn settings_profile_for(app: &AppState, pane: StatePane) -> Option<String> {
    let s = app.get::<State>();
    if pane != StatePane::Profiles {
        return None;
    }
    let current = if s.pane == StatePane::Profiles {
        s.profile_id.as_deref()
    } else {
        None
    };
    Some(extensions_profile_id(app, current))
}

fn section_key(pane: StatePane, section: &str) -> String {
    format!("{}\u{0}{}", text_of(&pane), section)
}

/// `dropSettingsSearch` (`searchState.ts:46`).
fn drop_search(s: &mut State) {
    s.query.clear();
    s.selected = Number(0.0);
    s.keyboard_move = Number(0.0);
    s.reveal = None;
    s.flash = None;
    s.preview_section = None;
    s.previewing = false;
}

fn reveal_at(id: &str, now: Number) -> Option<StateReveal> {
    Some(StateReveal {
        id: id.to_owned(),
        at: now,
    })
}

/// `showWithoutHistory` (`searchState.ts:107`).
fn show_without_history(app: &mut AppState, pane: StatePane) {
    let profile_id = settings_profile_for(app, pane);
    let s = app.get_mut::<State>();
    if s.pane != pane || s.profile_id != profile_id || s.detail.is_some() {
        s.pane = pane;
        s.profile_id = profile_id;
        s.detail = None;
        s.forward.clear();
    }
}

/// `jumpToSetting` (`searchState.ts:89`).
fn jump_to(app: &mut AppState, target: &Target, now: Number) {
    let pane: StatePane = convert(&target.pane);
    let key = section_key(pane, &target.section);
    let previewing = {
        let s = app.get_mut::<State>();
        if !s.revealed_sections.contains(&key) {
            s.revealed_sections.push(key);
        }
        s.previewing
    };
    if previewing {
        show_without_history(app, pane);
    } else {
        let profile_id = settings_profile_for(app, pane);
        go_to_pane(app.get_mut::<State>(), pane, profile_id, None);
    }
    let s = app.get_mut::<State>();
    s.reveal = reveal_at(&target.id, now);
    s.previewing = false;
}

/// `previewSetting` (`searchState.ts:98`).
fn preview(app: &mut AppState, target: &Target, now: Number) {
    let pane: StatePane = convert(&target.pane);
    {
        let s = app.get::<State>();
        if js_trim(&s.query).is_empty() {
            return;
        }
        if s.reveal.as_ref().is_some_and(|r| r.id == target.id) && s.previewing && s.pane == pane {
            return;
        }
    }
    if app.get::<State>().previewing {
        show_without_history(app, pane);
    } else {
        let profile_id = settings_profile_for(app, pane);
        go_to_pane(app.get_mut::<State>(), pane, profile_id, None);
    }
    let s = app.get_mut::<State>();
    s.preview_section = Some(section_key(pane, &target.section));
    s.previewing = true;
    s.reveal = reveal_at(&target.id, now);
}

fn menu_commands() -> Target {
    Target {
        id: "shortcuts.menuCommands".into(),
        pane: convert(&StatePane::Shortcuts),
        section: "Keyboard".into(),
    }
}

fn open_window(fx: &mut Effects, id: &str, kind: &str, title: &str) {
    fx.push(PlatformCall::AppWindowsOpenWindow {
        id: id.to_owned(),
        options: json!({ "kind": kind, "title": title }),
    });
}

fn open_settings(
    app: &mut AppState,
    pane: Option<StatePane>,
    profile_id: Option<String>,
    detail: Option<String>,
    fx: &mut Effects,
) {
    if let Some(pane) = pane {
        let s = app.get_mut::<State>();
        drop_search(s);
        go_to_pane(s, pane, profile_id, detail);
    }
    open_window(fx, SETTINGS_WINDOW_ID, "settings", "Settings");
}

fn shortcut_entry(
    shortcuts: Option<&Value>,
    id: &str,
    combo: Option<&crate::commands::Shortcut>,
) -> Value {
    let mut next = shortcuts
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    match combo {
        Some(c) => {
            let mut keys = vec![Value::String(c.key.clone())];
            keys.extend(c.modifiers.iter().cloned());
            next.insert(id.to_owned(), Value::Array(keys));
        }
        None => {
            next.remove(id);
        }
    }
    Value::Object(next)
}

fn set_one(app: &mut AppState, key: &str, value: Value, fx: &mut Effects) {
    update_settings(app, vec![(key.to_owned(), value)], false, false, fx);
}

fn resource_key(name: &str, args: &[Value]) -> String {
    let mut all = vec![Value::String(name.to_owned())];
    all.extend(args.iter().cloned());
    Value::Array(all).to_string()
}

/// The platform call that fetches a resource of `paneData.ts`.
fn fetch_call(name: &str, args: &[Value]) -> Option<PlatformCall> {
    let profile = || {
        args.first()
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    Some(match name {
        "updater" => PlatformCall::UpdaterUpdaterState {},
        "appIcons" => PlatformCall::AppAppIcons {
            size: Number(APP_ICON_SIZE),
        },
        "contentBlocker" => PlatformCall::ContentBlockerState {},
        "downloadSettings" => PlatformCall::DownloadsSettings { profile: profile() },
        "appLinks" => PlatformCall::PermissionsExternalAppAllowances { profile: profile() },
        "zoomLevels" => PlatformCall::SitesZoomLevels { profile: profile() },
        "preloadPages" => PlatformCall::SitesPreloadPages { profile: profile() },
        "passwordAutofill" => PlatformCall::PasswordsAutofillEnabled { profile: profile() },
        "addresses" => PlatformCall::AutofillAddresses { profile: profile() },
        "cards" => PlatformCall::AutofillCards { profile: profile() },
        "autofillEnabled" => PlatformCall::AutofillSettings { profile: profile() },
        "menuShortcuts" => PlatformCall::MenusMenuShortcuts {},
        "engineInfo" => PlatformCall::EngineInfo {},
        _ => return None,
    })
}

/// `paneNeeds` (`paneData.ts:84`), without the resources that have no platform call.
fn pane_needs(
    app: &AppState,
    pane: StatePane,
    profile_id: Option<&str>,
) -> Vec<(&'static str, Vec<Value>)> {
    let profile = profile_id.map_or_else(
        || app.get::<State>().settings.default_profile_id.clone(),
        str::to_owned,
    );
    let p = || vec![Value::String(profile.clone())];
    match pane {
        StatePane::General => vec![
            ("updater", vec![]),
            ("downloadSettings", p()),
            ("contentBlocker", vec![]),
            ("engineInfo", vec![]),
        ],
        StatePane::Profiles => vec![
            ("downloadSettings", p()),
            ("preloadPages", p()),
            ("passwordAutofill", p()),
            ("addresses", p()),
            ("cards", p()),
            ("autofillEnabled", p()),
        ],
        StatePane::Appearance => vec![("appIcons", vec![])],
        StatePane::Automations => vec![("appLinks", p())],
        StatePane::Shortcuts => vec![("menuShortcuts", vec![])],
        _ => vec![],
    }
}

fn load(name: &str, args: &[Value], fx: &mut Effects) {
    if let Some(call) = fetch_call(name, args) {
        fx.push(call);
    }
}

/// `apply` for the `settings` module.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    match action {
        Action::UpdateSettings { patch, options } => {
            let o = options.unwrap_or_default();
            update_settings(
                app,
                patch.0,
                o.from_sync.unwrap_or(false),
                o.inferred.unwrap_or(false),
                fx,
            );
        }
        Action::SetSearchEngine { id } => {
            if search_engines(&settings_map(app.get::<State>()))
                .iter()
                .any(|e| e.id == id)
            {
                set_one(app, "searchEngine", json!(id), fx);
            }
        }
        Action::AddCustomEngine { input, now } => {
            let current = settings_map(app.get::<State>());
            let input = engine_input(&input);
            if validate_engine(&input, &search_engines(&current), None).is_some() {
                return;
            }
            let run = &mut app.get_mut::<State>().run;
            run.engine_counter += 1;
            let engine = make_custom_engine(&input, &build_id(now.0, run.engine_counter));
            let mut all = stored_engines(&current);
            all.push(engine_json(&engine));
            set_one(app, "customSearchEngines", Value::Array(all), fx);
        }
        Action::UpdateCustomEngine { id, input } => {
            let current = settings_map(app.get::<State>());
            let stored = stored_engines(&current);
            let has = |e: &Value| e.get("id").and_then(Value::as_str) == Some(id.as_str());
            if !stored.iter().any(has) {
                return;
            }
            let input = engine_input(&input);
            if validate_engine(&input, &search_engines(&current), Some(&id)).is_some() {
                return;
            }
            let made = engine_json(&make_custom_engine(&input, &id));
            let next = stored
                .iter()
                .map(|e| if has(e) { made.clone() } else { e.clone() })
                .collect();
            set_one(app, "customSearchEngines", Value::Array(next), fx);
        }
        Action::RemoveCustomEngine { id } => {
            let current = settings_map(app.get::<State>());
            let kept: Vec<Value> = stored_engines(&current)
                .into_iter()
                .filter(|e| e.get("id").and_then(Value::as_str) != Some(id.as_str()))
                .collect();
            let mut patch: Patch = vec![("customSearchEngines".into(), Value::Array(kept))];
            if current.get("searchEngine").and_then(Value::as_str) == Some(id.as_str()) {
                patch.push(("searchEngine".into(), json!("google")));
            }
            update_settings(app, patch, false, false, fx);
        }
        Action::SetShortcutOverride { id, combo } => {
            let next = shortcut_entry(
                settings_map(app.get::<State>()).get("shortcuts"),
                &id,
                combo.as_ref(),
            );
            set_one(app, "shortcuts", next, fx);
        }
        Action::SetMouseShortcut { id, button } => {
            let next = shortcut_entry(
                settings_map(app.get::<State>()).get("mouseShortcuts"),
                &id,
                button.as_ref(),
            );
            set_one(app, "mouseShortcuts", next, fx);
        }
        Action::ChooseAddressBar { address_bar } => {
            set_one(app, "addressBar", json!(text_of(&address_bar)), fx)
        }
        Action::ChooseScrollToolbar {
            hide_toolbar_while_scrolling,
        } => {
            set_one(
                app,
                "hideToolbarWhileScrolling",
                json!(hide_toolbar_while_scrolling),
                fx,
            );
        }
        Action::ChooseCommandBarPlacement {
            command_bar_placement,
        } => {
            set_one(
                app,
                "commandBarPlacement",
                json!(text_of(&command_bar_placement)),
                fx,
            );
        }
        Action::ChooseSidebarSide { side } => {
            set_one(app, "sidebarSide", json!(text_of(&side)), fx)
        }
        Action::ToggleSidebarSide => {
            let right = settings_map(app.get::<State>())
                .get("sidebarSide")
                .and_then(Value::as_str)
                == Some("right");
            set_one(
                app,
                "sidebarSide",
                json!(if right { "left" } else { "right" }),
                fx,
            );
        }
        Action::ChooseInterfaceScale { scale } => set_one(
            app,
            "interfaceScale",
            json!(stepped_interface_scale(scale.0)),
            fx,
        ),
        // The windows follow `settings.tabLayout` through the browser module's `toggleTabLayout`.
        Action::ChooseTabLayout { layout } => {
            set_one(app, "tabLayout", json!(text_of(&layout)), fx)
        }
        Action::SwitchTabLayout { .. } | Action::ToggleTabLayout { .. } => {}
        Action::GoToSettingsPane {
            pane,
            profile_id,
            detail,
        } => {
            go_to_pane(
                app.get_mut::<State>(),
                convert::<GoToSettingsPanePane, StatePane>(&pane),
                profile_id,
                detail,
            );
        }
        Action::ReplaceSettingsPlace { profile_id, detail } => {
            let s = app.get_mut::<State>();
            let pane: SettingsPlacePane = convert(&s.pane);
            let same = s.back.last().is_some_and(|p| {
                p.pane == pane && p.profile_id == profile_id && p.detail == detail
            });
            if same {
                s.back.pop();
            }
            s.profile_id = profile_id;
            s.detail = detail;
            s.forward.clear();
        }
        Action::SettingsBack => {
            let s = app.get_mut::<State>();
            if let Some(prev) = s.back.pop() {
                let current = place_of(s);
                s.forward.insert(0, current);
                s.pane = convert(&prev.pane);
                s.profile_id = prev.profile_id;
                s.detail = prev.detail;
            }
        }
        Action::SettingsForward => {
            let s = app.get_mut::<State>();
            if !s.forward.is_empty() {
                let next = s.forward.remove(0);
                let current = place_of(s);
                s.back.push(current);
                s.pane = convert(&next.pane);
                s.profile_id = next.profile_id;
                s.detail = next.detail;
            }
        }
        Action::OpenSettings {
            pane,
            profile_id,
            detail,
        } => {
            open_settings(app, pane.map(|p| convert(&p)), profile_id, detail, fx);
        }
        Action::OpenSiteSettings { origin, profile_id } => {
            open_settings(
                app,
                Some(StatePane::Profiles),
                Some(profile_id),
                Some(format!("site:{origin}")),
                fx,
            );
        }
        Action::OpenImport { browser_id } => {
            app.get_mut::<State>().source = browser_id;
            open_window(
                fx,
                IMPORT_WINDOW_ID,
                "import",
                "Import from Another Browser",
            );
        }
        Action::TakePreferredImportSource => app.get_mut::<State>().source = None,
        // `newTab` and the window order are the browser module's, which is still empty.
        Action::OpenUrlFromSettings { .. } => {}
        Action::SetSettingsQuery { query } => {
            let s = app.get_mut::<State>();
            let blank = js_trim(&query).is_empty();
            s.query = query;
            s.selected = Number(0.0);
            s.keyboard_move = Number(0.0);
            if blank {
                s.previewing = false;
            }
        }
        Action::DropSettingsSearch { .. } | Action::SettingsSearchHost => {
            drop_search(app.get_mut::<State>())
        }
        Action::FocusSettingsSearch { now } => app.get_mut::<State>().focus_request = now,
        Action::MoveSettingsSelection { step, count } => {
            let s = app.get_mut::<State>();
            s.selected = Number(js_max(0.0, js_min(count.0 - 1.0, s.selected.0 + step.0)));
            s.keyboard_move = Number(s.keyboard_move.0 + 1.0);
        }
        Action::SelectSettingsResult { index } => app.get_mut::<State>().selected = index,
        Action::ClearSettingsReveal => app.get_mut::<State>().reveal = None,
        Action::FlashSetting { id, now } => {
            let s = app.get_mut::<State>();
            s.reveal = None;
            s.flash = Some(StateFlash { id, at: now });
        }
        Action::SetShortcutsQuery { shortcuts_query } => {
            app.get_mut::<State>().shortcuts_query = shortcuts_query
        }
        Action::JumpToSetting { arg0, now } => jump_to(app, &arg0, now),
        Action::PreviewSetting { arg0, now } => preview(app, &arg0, now),
        Action::RevealSetting { target, now, .. } => jump_to(app, &target, now),
        Action::RevealCommandKeys { keys, now, .. } => {
            app.get_mut::<State>().shortcuts_query = keys;
            jump_to(app, &menu_commands(), now);
        }
        Action::PreviewCommandKeys { keys, now } => {
            app.get_mut::<State>().shortcuts_query = keys;
            preview(app, &menu_commands(), now);
        }
        Action::NoteSettingsScroll {
            offset,
            viewport,
            content,
        } => {
            let s = app.get_mut::<State>();
            if s.run.last_content != Some(content.0) {
                s.run.last_content = Some(content.0);
                s.run.scroll_disarmed = false;
            }
            if offset.0 + viewport.0 < content.0 - NEAR_END {
                s.run.scroll_disarmed = false;
                return;
            }
            if s.run.scroll_disarmed {
                return;
            }
            s.run.scroll_disarmed = true;
            s.reached = Number(s.reached.0 + 1.0);
        }
        Action::ShowSettingsSheet { sheet } => app.get_mut::<State>().sheet = sheet,
        Action::CloseSettingsSheet => app.get_mut::<State>().sheet = Value::Null,
        Action::AppliedInterfaceScale { scale } => {
            let s = app.get_mut::<State>();
            if scale != s.scale {
                s.scale = scale;
            }
        }
        Action::StartInterfaceScale => start_interface_scale(app.get_mut::<State>(), fx),
        Action::StartNativeSettings => {
            let s = app.get_mut::<State>();
            start_interface_scale(s, fx);
            s.run.native_started = true;
            sync_native(s, fx);
        }
        Action::Load { r, args, arg2: _ } => {
            load(&r.name, args.as_array().map_or(&[][..], Vec::as_slice), fx)
        }
        Action::LoadAll { needed } => {
            for (r, args) in &needed {
                load(&r.name, args, fx);
            }
        }
        Action::PreparePane { pane, profile_id } => {
            for (name, args) in pane_needs(app, convert(&pane), profile_id.as_deref()) {
                load(name, &args, fx);
            }
        }
        Action::Store { r, args, value } => {
            let args: Vec<Value> = args.as_array().cloned().unwrap_or_default();
            let key = resource_key(&r.name, &args);
            let cache = &mut app.get_mut::<State>().run.resources;
            match cache.iter_mut().find(|(k, _)| *k == key) {
                Some(entry) => entry.1 = value,
                None => cache.push((key, value)),
            }
        }
        Action::SetPreloadPagesFor { profile, on } => {
            fx.push(PlatformCall::SitesSetPreloadPages { profile, on })
        }
        Action::SetPasswordAutofillFor { profile, on } => {
            fx.push(PlatformCall::PasswordsSetAutofillEnabled {
                profile,
                enabled: on,
            });
        }
        Action::SetAutofillFor { profile, kind, on } => {
            fx.push(PlatformCall::AutofillSetSettings {
                profile,
                settings: json!({ text_of(&kind): on }),
            });
        }
        Action::SetDownloadsFor { patch, profiles } => {
            let patch = serde_json::to_value(&patch).unwrap_or_default();
            for profile in profiles {
                fx.push(PlatformCall::DownloadsSetSettings {
                    patch: patch.clone(),
                    profile,
                });
            }
        }
        Action::SetUpdaterPrefs { patch } => {
            if let Some(on) = patch.automatic_checks {
                fx.push(PlatformCall::UpdaterSetAutomaticUpdateChecks { on });
            }
            if let Some(on) = patch.automatic_downloads {
                fx.push(PlatformCall::UpdaterSetAutomaticUpdateDownloads { on });
            }
            if let Some(channel) = patch.channel {
                fx.push(PlatformCall::UpdaterSetUpdateChannel { channel });
            }
        }
        Action::FollowUpdater => fx.push(PlatformCall::UpdaterUpdaterState {}),
        Action::ChooseAppIcon { id } => fx.push(PlatformCall::AppSetAppIcon { id }),
        Action::MakeDefaultBrowser => {}
    }
}

/// `startInterfaceScale` (`interfaceScaleSync.ts:27`); the native scale comes back as `AppliedInterfaceScale`.
fn start_interface_scale(s: &mut State, fx: &mut Effects) {
    if s.run.scale_started {
        return;
    }
    s.run.scale_started = true;
    fx.push(PlatformCall::AppInterfaceScale {});
    if stored_scale_wins(s) && !same_interface_scale(s.settings.interface_scale.0, s.scale.0) {
        let wanted = s.settings.interface_scale.0;
        push_scale(s, wanted, fx);
    }
}
