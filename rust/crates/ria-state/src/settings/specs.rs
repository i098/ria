// Ported from ardia apps/browser/src/state/settings/settings.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.
//
// Also ports settingsPatch.ts.

//! The setting table (`SETTING_SPECS`), the checks of a settings patch, and the rules that `updateSettings` applies.
//!
//! Changes from Ardia:
//! - A settings object and a patch are `serde_json` values; a patch is an ordered list of entries, because
//!   `Object.keys(patch)` order decides the order of `settingsMeta.chosen`.
//! - The `schema` and `values` fields of `SettingSpec` are not ported: only the config schema slice reads them.
//! - `noteSettingsWritesRejected` (telemetry) is not called here; `updateSettings` in `mod.rs` calls the telemetry
//!   sibling through the slice 2.4 signatures.

use std::sync::LazyLock;

use ria_core::identity::telemetry_wire::Json;
use ria_core::search::engines::{StoredEngine, custom_engines_from};
use ria_core::search::provider_config::is_provider_entry_list;
use ria_core::search::provider_rows::JsValue;
use serde_json::{Map, Value, json};

use super::SettingsMeta;
use crate::shared::interface_scale::{INTERFACE_SCALE, valid_interface_scale};
use crate::shared::tab_corners::valid_tab_corners;
use crate::shared::window_layout::{DEFAULT_SIDEBAR_WIDTH, PAGE_FRAME, valid_page_frame};

pub const DEFAULT_PROFILE_ID: &str = "default";

/// An ordered list of `(key, value)` entries: a settings patch.
pub type Patch = Vec<(String, Value)>;

/// How a setting value is checked (`SettingSpec.valid`).
#[derive(Clone, Copy)]
enum Valid {
    Flag,
    Text,
    Width,
    Fraction,
    OneOf(&'static [&'static str]),
    StringList,
    RecordOfStringList,
    RecordOfBool,
    CustomEngines,
    ExtensionEngines,
    CleanUp,
    InterfaceScale,
    PageFrame,
    TabCorners,
    CommandBarStart,
    LaunchActions,
    Providers,
    LittleArdiaSize,
    WebsiteEngines,
}

pub struct Spec {
    pub default: Value,
    valid: Valid,
}

fn is_width(v: &Value) -> bool {
    v.as_f64().is_some_and(|n| n.is_finite() && n > 0.0)
}

fn is_string_list(v: &Value) -> bool {
    v.as_array().is_some_and(|a| a.iter().all(Value::is_string))
}

fn has_strings(v: &Value, keys: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|o| keys.iter().all(|k| o.get(*k).is_some_and(Value::is_string)))
}

fn is_record_of(v: &Value, item: impl Fn(&Value) -> bool) -> bool {
    v.as_object().is_some_and(|o| o.values().all(item))
}

/// `Number.isInteger(v) && 0 <= v <= 20` (`commandBarStart.ts:10`).
fn is_limit(v: &Value) -> bool {
    v.as_f64()
        .is_some_and(|n| n.fract() == 0.0 && (0.0..=20.0).contains(&n))
}

fn is_open_tabs(v: &Value) -> bool {
    v.as_object().is_some_and(|o| {
        o.iter().all(|(k, v)| match k.as_str() {
            "limit" => is_limit(v),
            "hidePinned" | "hideTopApps" => v.is_boolean(),
            _ => false,
        })
    })
}

fn is_command_bar_start(v: &Value) -> bool {
    v.as_object()
        .is_some_and(|o| o.iter().all(|(k, v)| k == "openTabs" && is_open_tabs(v)))
}

pub(crate) fn to_json(v: &Value) -> Json {
    match v {
        Value::Null => Json::Null,
        Value::Bool(b) => Json::Bool(*b),
        Value::Number(n) => Json::Number(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => Json::String(s.clone()),
        Value::Array(a) => Json::Array(a.iter().map(to_json).collect()),
        Value::Object(o) => Json::Object(o.iter().map(|(k, v)| (k.clone(), to_json(v))).collect()),
    }
}

fn to_js(v: &Value) -> JsValue {
    match v {
        Value::Null => JsValue::Null,
        Value::Bool(b) => JsValue::Bool(*b),
        Value::Number(n) => JsValue::Number(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => JsValue::String(s.clone()),
        Value::Array(a) => JsValue::Array(a.iter().map(to_js).collect()),
        Value::Object(o) => JsValue::Object(o.iter().map(|(k, v)| (k.clone(), to_js(v))).collect()),
    }
}

impl Valid {
    fn check(self, v: &Value) -> bool {
        match self {
            Self::Flag => v.is_boolean(),
            Self::Text => v.is_string(),
            Self::Width => is_width(v),
            Self::Fraction => v.as_f64().is_some_and(|n| (0.0..=1.0).contains(&n)),
            Self::OneOf(values) => v.as_str().is_some_and(|s| values.contains(&s)),
            Self::StringList => is_string_list(v),
            Self::RecordOfStringList => is_record_of(v, is_string_list),
            Self::RecordOfBool => is_record_of(v, Value::is_boolean),
            Self::CustomEngines => v.as_array().is_some_and(|a| {
                a.iter()
                    .all(|e| has_strings(e, &["id", "name", "keyword", "url"]))
            }),
            Self::ExtensionEngines => v.as_array().is_some_and(|a| {
                a.iter().all(|e| {
                    has_strings(
                        e,
                        &[
                            "profile",
                            "extensionId",
                            "extensionName",
                            "name",
                            "keyword",
                            "url",
                        ],
                    ) && e.get("suggestUrl").is_none_or(Value::is_string)
                        && e.get("isDefault").is_some_and(Value::is_boolean)
                })
            }),
            Self::CleanUp => v.is_null() || is_width(v),
            Self::InterfaceScale => v.as_f64().is_some_and(valid_interface_scale),
            Self::PageFrame => valid_page_frame(&to_json(v)),
            Self::TabCorners => valid_tab_corners(Some(&to_json(v))),
            Self::CommandBarStart => is_command_bar_start(v),
            Self::LaunchActions => is_string_list(v) && v.as_array().is_some_and(|a| a.len() <= 20),
            Self::Providers => is_provider_entry_list(&to_js(v)),
            Self::LittleArdiaSize => v
                .as_array()
                .is_some_and(|a| a.len() == 2 && a.iter().all(is_width)),
            Self::WebsiteEngines => {
                is_record_of(v, |e| matches!(e.as_str(), Some("chromium" | "webkit")))
            }
        }
    }
}

fn spec(default: Value, valid: Valid) -> Spec {
    Spec { default, valid }
}

fn flag(d: bool) -> Spec {
    spec(json!(d), Valid::Flag)
}

fn text(d: &str) -> Spec {
    spec(json!(d), Valid::Text)
}

fn one_of(values: &'static [&'static str], d: &str) -> Spec {
    spec(json!(d), Valid::OneOf(values))
}

fn list(valid: Valid) -> Spec {
    spec(json!([]), valid)
}

fn record(valid: Valid) -> Spec {
    spec(json!({}), valid)
}

/// `SETTING_SPECS` (`settings.ts:50`), in the order of the TypeScript object.
pub static SETTING_SPECS: LazyLock<Vec<(&'static str, Spec)>> = LazyLock::new(|| {
    use Valid::*;
    vec![
        ("warnBeforeQuitting", flag(false)),
        ("warnBeforeClosingWindow", flag(true)),
        ("restoreSession", flag(true)),
        ("separateDownloadFolders", flag(false)),
        ("searchEngine", text("google")),
        ("customSearchUrl", text("")),
        ("customSearchEngines", list(CustomEngines)),
        ("extensionSearchEngines", list(ExtensionEngines)),
        ("searchSuggestions", flag(true)),
        ("commandBarHistory", flag(true)),
        ("showSelectionSearch", flag(true)),
        ("tabLayout", one_of(&["sidebar", "top"], "sidebar")),
        ("sidebarSide", one_of(&["left", "right"], "left")),
        ("autoHideTabs", flag(false)),
        ("hideToolbarWhileScrolling", flag(false)),
        (
            "newTabPosition",
            one_of(&["top", "bottom", "nextToCurrent"], "bottom"),
        ),
        ("newTabButtonPosition", one_of(&["top", "bottom"], "bottom")),
        ("appLinksFirst", flag(false)),
        ("newTabOpens", one_of(&["page", "commandBar"], "page")),
        ("newTabUrl", text("")),
        ("warnBeforeClosingLastTab", flag(true)),
        ("keepTopAppsRunning", flag(false)),
        ("tabReorderHaptics", flag(true)),
        ("swipeBetweenTabs", flag(true)),
        ("undoReopensClosedTab", flag(false)),
        ("warnBeforeMovingTabsToProfile", flag(true)),
        ("shareTabsAcrossWindows", flag(false)),
        ("extendWebsiteColor", flag(true)),
        ("cmdClickCreatesTabGroup", flag(true)),
        ("optShiftClickOpensInGroup", flag(true)),
        ("peekModifier", one_of(&["alt", "shift", "off"], "shift")),
        ("peekLinksToOtherSites", flag(true)),
        (
            "newTabLinksOpenIn",
            one_of(&["tab", "peek", "littleArdia"], "peek"),
        ),
        ("popupsOpenAsTabs", flag(false)),
        ("peekButtons", one_of(&["top", "left", "right"], "top")),
        (
            "splitLinkModifier",
            one_of(&["altShift", "alt"], "altShift"),
        ),
        ("autoGroupMeetingTabs", flag(true)),
        (
            "cleanUpInactiveTabsAfterMinutes",
            spec(Value::Null, CleanUp),
        ),
        ("suggestCleanUp", flag(true)),
        ("mutedSites", list(StringList)),
        ("showUnreadCounts", flag(false)),
        ("showTopAppBadges", flag(false)),
        ("showTabHoverMenu", flag(true)),
        ("showFullUrlInTabHoverMenu", flag(false)),
        ("closedTopAppTiles", flag(true)),
        ("topAppFolders", flag(true)),
        ("unreadHiddenSites", list(StringList)),
        ("neverTranslateSites", record(RecordOfStringList)),
        ("developerModeSites", record(RecordOfBool)),
        ("autoPictureInPicture", flag(true)),
        ("playingNowAcrossProfiles", flag(false)),
        ("sidebarMediaControls", flag(true)),
        ("commandScrollZoom", flag(true)),
        ("blockAutoplayEverywhere", flag(false)),
        ("blockNotificationRequests", flag(false)),
        ("videoUpscaling", flag(true)),
        ("sidebarWidth", spec(json!(DEFAULT_SIDEBAR_WIDTH), Width)),
        ("showSidebarButton", flag(true)),
        ("showClearButton", flag(true)),
        ("tidyTabs", flag(false)),
        ("balancedTopApps", flag(false)),
        ("selectedTabGlow", one_of(&["full", "soft", "off"], "full")),
        (
            "interfaceScale",
            spec(json!(INTERFACE_SCALE.default), InterfaceScale),
        ),
        (
            "sidebarTextSize",
            one_of(&["default", "large", "larger"], "default"),
        ),
        (
            "sidebarTextContrast",
            one_of(&["auto", "standard", "high"], "auto"),
        ),
        (
            "selectedTopApp",
            one_of(&["ring", "fill", "outline"], "ring"),
        ),
        ("elementOutlines", flag(true)),
        ("interfaceStyle", one_of(&["glass", "classic"], "glass")),
        (
            "windowButtons",
            one_of(&["native", "trafficLights", "hidden"], "native"),
        ),
        ("glassTintWithTheme", flag(false)),
        ("glassTintAmount", spec(json!(0.8), Fraction)),
        ("fadeUnloadedTabs", flag(false)),
        ("strikeSleepingTabs", flag(false)),
        ("warnBeforeClosingTabGroup", flag(true)),
        (
            "batterySaverWhen",
            one_of(&["lowBattery", "unplugged", "never"], "lowBattery"),
        ),
        (
            "unloadProfilesAfter",
            one_of(&["never", "5m", "15m", "1h", "2h"], "never"),
        ),
        ("memorySaver", flag(true)),
        (
            "memorySaverMode",
            one_of(&["moderate", "balanced", "maximum"], "balanced"),
        ),
        ("loadProfilesWhenOpened", flag(true)),
        ("keepActiveSites", list(StringList)),
        ("extensionSidePanelWidth", spec(json!(360), Width)),
        ("showFullUrl", flag(false)),
        ("allowJavaScriptFromAppleEvents", flag(false)),
        ("cliControl", flag(false)),
        ("compactCliPill", flag(false)),
        ("showBookmarks", flag(true)),
        (
            "bookmarksBar",
            one_of(&["always", "newTab", "never"], "never"),
        ),
        ("bookmarksImportDismissed", flag(false)),
        ("appearance", one_of(&["auto", "light", "dark"], "auto")),
        ("addressBar", one_of(&["toolbar", "sidebar"], "toolbar")),
        ("compactToolbar", flag(false)),
        ("addressBarButtons", one_of(&["hover", "always"], "hover")),
        ("pageFrame", spec(json!(PAGE_FRAME.default), PageFrame)),
        ("borderlessMode", flag(false)),
        ("tabCornerRadius", spec(Value::Null, TabCorners)),
        ("minimalismMode", flag(false)),
        ("pageFrameZeroInFullScreen", flag(false)),
        ("pageFrameZeroWhenTabsHidden", flag(false)),
        (
            "commandBarPlacement",
            one_of(&["field", "centred", "centredOnClick"], "field"),
        ),
        ("commandBarStart", record(CommandBarStart)),
        ("pressMotion", one_of(&["still", "bounce"], "still")),
        ("folderStyle", one_of(&["profile", "neutral"], "profile")),
        ("liveFolderIcon", one_of(&["folder", "service"], "folder")),
        ("spacesFollowAppearance", flag(true)),
        ("adaptTextToColor", flag(true)),
        ("defaultProfileId", text(DEFAULT_PROFILE_ID)),
        ("showProfileDots", flag(true)),
        ("shortcuts", record(RecordOfStringList)),
        ("mouseShortcuts", record(RecordOfStringList)),
        ("openLinksInLittleArdia", flag(true)),
        ("littleArdiaOpensNewTabLinks", flag(true)),
        ("littleArdiaShortcutFromAnyApp", flag(false)),
        ("littleArdiaSize", spec(json!([900, 640]), LittleArdiaSize)),
        (
            "engineMode",
            one_of(&["automatic", "chromium", "webkit"], "automatic"),
        ),
        ("pageEngine", one_of(&["chromium", "webkit"], "chromium")),
        ("websiteEngines", record(WebsiteEngines)),
        ("protectedVideoInWebKit", flag(true)),
        ("offerWebKitForProtectedVideo", flag(true)),
        ("engineSwitchKeepsPage", flag(true)),
        ("engineSwitchAsksAboutUnsavedWork", flag(true)),
        ("engineSwitchCarriesSiteData", flag(true)),
        ("webKitFraudWarnings", flag(false)),
        ("launchActions", list(LaunchActions)),
        ("omniboxProviders", list(Providers)),
        (
            "readerTextSize",
            one_of(&["small", "medium", "large", "larger", "largest"], "medium"),
        ),
        ("readerFont", one_of(&["serif", "sans"], "serif")),
    ]
});

fn spec_of(key: &str) -> Option<&'static Spec> {
    SETTING_SPECS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, s)| s)
}

/// `validSettingValue` (`settings.ts:155`); an unknown key is not valid.
pub fn valid_setting_value(key: &str, value: &Value) -> bool {
    spec_of(key).is_some_and(|s| s.valid.check(value))
}

/// `DEFAULT_SETTINGS` (`settings.ts:197`).
pub fn default_settings_map() -> Map<String, Value> {
    SETTING_SPECS
        .iter()
        .map(|(k, s)| ((*k).to_owned(), s.default.clone()))
        .collect()
}

fn set_entry(patch: &mut Patch, key: &str, value: Value) {
    match patch.iter_mut().find(|(k, _)| k == key) {
        Some(entry) => entry.1 = value,
        None => patch.push((key.to_owned(), value)),
    }
}

fn entry<'a>(patch: &'a Patch, key: &str) -> Option<&'a Value> {
    patch.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn in_patch(patch: &Patch, key: &str) -> bool {
    patch.iter().any(|(k, _)| k == key)
}

/// `customEnginesFrom` (`engines.ts:98`) over a JSON value: each engine with a valid template, as a fresh object.
pub fn custom_engines_value(raw: &Value) -> Value {
    let Some(items) = raw.as_array() else {
        return json!([]);
    };
    let get = |e: &'_ Value, k: &str| e.get(k).and_then(Value::as_str).map(str::to_owned);
    let stored: Vec<[Option<String>; 4]> = items
        .iter()
        .map(|e| ["id", "name", "keyword", "url"].map(|k| get(e, k)))
        .collect();
    let view: Vec<StoredEngine> = stored
        .iter()
        .map(|[id, name, keyword, url]| StoredEngine {
            id: id.as_deref(),
            name: name.as_deref(),
            keyword: keyword.as_deref(),
            url: url.as_deref(),
        })
        .collect();
    Value::Array(
        custom_engines_from(&view)
            .into_iter()
            .map(|e| json!({ "id": e.id, "name": e.name, "keyword": e.keyword, "url": e.url }))
            .collect(),
    )
}

/// `checkedSettingsPatch` (`settingsPatch.ts:5`): the valid entries and the rejected keys.
pub fn checked_patch(patch: &Patch) -> (Patch, Vec<String>) {
    let rejected: Vec<String> = patch
        .iter()
        .filter(|(k, v)| !valid_setting_value(k, v))
        .map(|(k, _)| k.clone())
        .collect();
    let kept = patch
        .iter()
        .filter(|(k, _)| !rejected.contains(k))
        .cloned()
        .collect();
    (kept, rejected)
}

/// `writableSettingsPatch` (`settings.ts:233`).
pub fn writable_patch(raw: &Patch) -> (Patch, Vec<String>) {
    let (mut patch, rejected) = checked_patch(raw);
    if let Some((_, v)) = patch.iter_mut().find(|(k, _)| k == "customSearchEngines") {
        *v = custom_engines_value(v);
    }
    (patch, rejected)
}

/// `withLinkKeysApart` (`settings.ts:174`).
pub fn with_link_keys_apart(settings: &Map<String, Value>, patch: Patch) -> Patch {
    let get = |key: &str| {
        entry(&patch, key)
            .or_else(|| settings.get(key))
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    if get("peekModifier").as_deref() != Some("alt")
        || get("splitLinkModifier").as_deref() != Some("alt")
    {
        return patch;
    }
    let mut out = patch;
    if in_patch(&out, "splitLinkModifier") && !in_patch(&out, "peekModifier") {
        set_entry(&mut out, "peekModifier", json!("shift"));
    } else {
        set_entry(&mut out, "splitLinkModifier", json!("altShift"));
    }
    out
}

/// `settledPatch` (`settings.ts:243`).
pub fn settled_patch(settings: &Map<String, Value>, raw: &Patch) -> Patch {
    let (mut patch, _) = checked_patch(raw);
    let on = |key: &str| {
        entry(&patch, key)
            .or_else(|| settings.get(key))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    };
    if on("pageFrameZeroInFullScreen")
        && !on("pageFrameZeroWhenTabsHidden")
        && !in_patch(&patch, "pageFrameZeroWhenTabsHidden")
    {
        patch.push(("pageFrameZeroWhenTabsHidden".to_owned(), json!(true)));
    }
    patch
}

/// `choose` (`settings.ts:250`), without the two telemetry notes: the caller makes them.
pub fn choose(
    settings: &Map<String, Value>,
    meta: &SettingsMeta,
    patch: &Patch,
    inferring: bool,
) -> (Map<String, Value>, SettingsMeta, Patch) {
    let patch = settled_patch(settings, patch);
    let mut next = settings.clone();
    for (k, v) in &patch {
        next.insert(k.clone(), v.clone());
    }
    let added: Vec<String> = patch
        .iter()
        .map(|(k, _)| k.clone())
        .filter(|k| !meta.chosen.contains(k))
        .collect();
    let left: Vec<String> = if inferring {
        let mut all = meta.inferred.clone();
        for (k, _) in &patch {
            if !all.contains(k) {
                all.push(k.clone());
            }
        }
        all
    } else {
        meta.inferred
            .iter()
            .filter(|k| !in_patch(&patch, k))
            .cloned()
            .collect()
    };
    let new_meta = if !added.is_empty() || left.len() != meta.inferred.len() {
        let mut chosen = meta.chosen.clone();
        chosen.extend(added);
        SettingsMeta {
            chosen,
            inferred: left,
        }
    } else {
        meta.clone()
    };
    (next, new_meta, patch)
}

/// `parseSettings` (`settings.ts:199`).
pub fn parse_settings(raw: &Value) -> Map<String, Value> {
    let empty = Map::new();
    let saved = raw.as_object().unwrap_or(&empty);
    let mut settings: Map<String, Value> = SETTING_SPECS
        .iter()
        .map(|(k, s)| {
            let v = saved
                .get(*k)
                .filter(|v| s.valid.check(v))
                .cloned()
                .unwrap_or_else(|| s.default.clone());
            ((*k).to_owned(), v)
        })
        .collect();
    let engines = custom_engines_value(&settings["customSearchEngines"]);
    settings.insert("customSearchEngines".into(), engines);
    if saved.get("hideBookmarksBarImport") == Some(&json!(true)) {
        settings.insert("bookmarksImportDismissed".into(), json!(true));
    }
    for (k, v) in with_link_keys_apart(&settings, Vec::new()) {
        settings.insert(k, v);
    }
    settings
}

/// `parseSettingsMeta` (`settings.ts:213`).
pub fn parse_settings_meta(raw: &Value) -> SettingsMeta {
    let list = |k: &str| {
        raw.get(k)
            .filter(|v| is_string_list(v))
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    };
    SettingsMeta {
        chosen: list("chosen"),
        inferred: list("inferred"),
    }
}
