// Ported from ardia apps/browser/src/state/profiles/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The logic of `state/profiles`: `profiles.ts`, `actions.ts`, `switcherMark.ts`, `switcherShown.ts`, `idleProfiles.ts`,
//! `profileLoading.ts`, `profileUnloading.ts`, `profileData.ts`, `iconPicker.ts`, `createProfileRequest.ts`,
//! `clearDataRequest.ts`, `profileMenu.ts`, `themeActions.ts`, `engineProfiles.ts`. See `mod.rs` for the changes from Ardia.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde_json::{Value, json};

use super::pending_siblings as sib;
use super::{
    Action, CreateProfileOptions, CreateProfilePreset, CreateProfileRequest, IconPickerAnchor,
    ProfileIconPicker, ProfileMenuPlace, SetProfileLookLook, ShowProfileMenuArg1, State,
    UnloadResult, UpdateProfilePatch,
};
use crate::browser::{BrowserWindow, Profile, ProfileColor, ProfilePresetThemes, ProfileSeenColor};
use crate::kernel::{AppState, Effects, Number};
use crate::platform::PlatformCall;
use crate::settings::{Settings, SettingsUnloadProfilesAfter};
use crate::shared::platform_features::PLATFORM_FEATURES;
use crate::shared::preset_look::{with_own_look, with_preset_chosen};
use crate::shared::profile_look as look;
use crate::{bookmarks, groups, history, pins, settings, tabs, windows};

/// `DEFAULT_PROFILE_ID` (`state/settings/settings.ts:24`).
pub const DEFAULT_PROFILE_ID: &str = "default";

const MINUTE_MS: i64 = 60_000;
const PERSONAL: &str = "";

/// `String.prototype.trim`: it strips U+FEFF and keeps U+0085.
fn js_trim(text: &str) -> &str {
    text.trim_matches(|c: char| (c.is_whitespace() && c != '\u{85}') || c == '\u{feff}')
}

// ---------------------------------------------------------------- profiles.ts

const COLORS: [ProfileColor; 7] = [
    ProfileColor::Blue,
    ProfileColor::Green,
    ProfileColor::Orange,
    ProfileColor::Purple,
    ProfileColor::Red,
    ProfileColor::Yellow,
    ProfileColor::Pink,
];

/// `unusedProfileColor` (`profiles.ts:27`).
pub fn unused_profile_color(profiles: &HashMap<String, Profile>) -> ProfileColor {
    COLORS
        .into_iter()
        .find(|c| !profiles.values().any(|p| p.color == *c))
        .unwrap_or(ProfileColor::Blue)
}

/// `dataIdOf` (`profiles.ts:30`).
pub fn data_id_of(profile: &Profile) -> &str {
    profile.data_id.as_deref().unwrap_or(&profile.id)
}

/// `dataGroups` (`profiles.ts:32`): the groups in the order of their first profile.
pub fn data_groups(s: &State) -> Vec<(String, Vec<String>)> {
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    for id in &s.profile_order {
        let Some(p) = s.profiles.get(id) else {
            continue;
        };
        let key = data_id_of(p);
        match groups.iter_mut().find(|(data_id, _)| data_id == key) {
            Some((_, ids)) => ids.push(id.clone()),
            None => groups.push((key.to_owned(), vec![id.clone()])),
        }
    }
    groups
}

/// `sharingProfiles` (`profiles.ts:43`).
pub fn sharing_profiles(s: &State, id: &str) -> Vec<String> {
    let Some(p) = s.profiles.get(id) else {
        return Vec::new();
    };
    s.profile_order
        .iter()
        .filter(|other| {
            other.as_str() != id
                && s.profiles
                    .get(*other)
                    .is_some_and(|o| data_id_of(o) == data_id_of(p))
        })
        .cloned()
        .collect()
}

fn seen_color(color: ProfileColor) -> ProfileSeenColor {
    serde_json::from_value(serde_json::to_value(color).expect("color")).expect("same names")
}

fn create_profile(app: &mut AppState, options: CreateProfileOptions, now: Option<Number>) {
    let s = app.get::<State>();
    let source = options
        .share_with
        .as_deref()
        .filter(|id| !id.is_empty())
        .and_then(|id| s.profiles.get(id));
    let chosen = options
        .color
        .unwrap_or_else(|| unused_profile_color(&s.profiles));
    let trimmed = js_trim(&options.name);
    let name = if trimmed.is_empty() {
        format!("Profile {}", s.profile_order.len() + 1)
    } else {
        trimmed.to_owned()
    };
    let data_id = source.map(|p| data_id_of(p).to_owned());
    let id = sib::new_id("p");
    let profile = Profile {
        id: id.clone(),
        name,
        color: chosen,
        icon: options.icon,
        created_at: now.unwrap_or_default(),
        preset_themes: Some(ProfilePresetThemes::default()),
        seen_theme: Some(Value::Null),
        seen_color: Some(seen_color(chosen)),
        data_id,
        share_top_apps: options.share_top_apps.unwrap_or(false).then_some(true),
        ..Profile::default()
    };
    let state = app.get_mut::<State>();
    state.profiles.insert(id.clone(), profile);
    state.profile_order.push(id.clone());
    let engine = sib::engine_profile(app, &id);
    sib::ensure_roots(app, &engine);
}

/// `patchedProfile` (`switcherMark.ts:7`).
pub fn patched_profile(profile: &Profile, patch: &UpdateProfilePatch) -> Profile {
    let icon_changed = patch
        .icon
        .as_ref()
        .is_some_and(|icon| *icon != profile.icon);
    let mut next = profile.clone();
    if let Some(name) = &patch.name {
        next.name = name.clone();
    }
    if let Some(icon) = &patch.icon {
        next.icon = icon.clone();
    }
    if let Some(v) = &patch.pinned_collapsed {
        next.pinned_collapsed = *v;
    }
    if let Some(v) = &patch.share_top_apps {
        next.share_top_apps = *v;
    }
    if let Some(v) = &patch.sync_tabs {
        next.sync_tabs = *v;
    }
    if let Some(v) = &patch.password_manager {
        next.password_manager = v.clone();
    }
    let dot = match &patch.switcher_dot {
        Some(v) => *v,
        None => profile.switcher_dot,
    };
    next.switcher_dot = (dot == Some(true) && !icon_changed).then_some(true);
    next
}

fn update_profile(app: &mut AppState, id: &str, patch: &UpdateProfilePatch) {
    if !app.get::<State>().profiles.contains_key(id) {
        return;
    }
    let state = app.get_mut::<State>();
    let profile = state.profiles.get_mut(id).expect("checked");
    *profile = patched_profile(profile, patch);
}

fn set_profile_look(app: &mut AppState, id: &str, look: &SetProfileLookLook) {
    if !app.get::<State>().profiles.contains_key(id) {
        return;
    }
    let profile = app
        .get_mut::<State>()
        .profiles
        .get_mut(id)
        .expect("checked");
    let before = profile.color;
    if let Some(color) = look.color {
        profile.color = color;
    }
    if let Some(theme) = &look.theme {
        let theme = Some(theme.clone().unwrap_or(Value::Null));
        profile.theme = theme.clone();
        profile.seen_theme = theme;
        profile.seen_color = Some(seen_color(look.color.unwrap_or(before)));
    }
    if let Some(preset_themes) = &look.preset_themes {
        profile.preset_themes = Some(preset_themes.clone());
    }
}

fn reorder_profiles(app: &mut AppState, ids: &[String]) {
    let state = app.get_mut::<State>();
    let known: Vec<String> = ids
        .iter()
        .filter(|id| state.profiles.contains_key(*id))
        .cloned()
        .collect();
    let rest = state
        .profile_order
        .iter()
        .filter(|id| !known.contains(id))
        .cloned();
    state.profile_order = known.iter().cloned().chain(rest).collect();
}

fn set_default_profile(app: &mut AppState, id: &str) {
    if !app.get::<State>().profiles.contains_key(id) {
        return;
    }
    let next = sib::settings_with(
        &app.get::<settings::State>().settings,
        json!({ "defaultProfileId": id }),
    );
    app.get_mut::<settings::State>().settings = next;
}

fn profile_data_deleted(app: &mut AppState, engine: &str) {
    if app
        .get::<State>()
        .orphaned_profile_data
        .iter()
        .any(|e| e == engine)
    {
        app.get_mut::<State>()
            .orphaned_profile_data
            .retain(|e| e != engine);
    }
}

fn window_id(w: &BrowserWindow) -> &str {
    &w.id
}

fn window_profile_id(w: &BrowserWindow) -> &str {
    &w.profile_id
}

fn window_tab_ids(w: &BrowserWindow) -> &[String] {
    &w.tab_ids
}

fn window_active_tab_ids(w: &BrowserWindow) -> &HashMap<String, String> {
    &w.active_tab_ids
}

fn window_active_tab_ids_mut(w: &mut BrowserWindow) -> &mut HashMap<String, String> {
    &mut w.active_tab_ids
}

/// The windows in the order of `windowOrder`, then the rest by id: `Object.values(s.windows)`.
fn windows_in_order(app: &AppState) -> Vec<BrowserWindow> {
    let w = app.get::<windows::State>();
    let mut ids: Vec<&String> = w
        .window_order
        .iter()
        .filter(|id| w.windows.contains_key(*id))
        .collect();
    let mut rest: Vec<&String> = w
        .windows
        .keys()
        .filter(|id| !w.window_order.contains(id))
        .collect();
    rest.sort();
    ids.extend(rest);
    ids.into_iter().map(|id| w.windows[id].clone()).collect()
}

fn delete_profile(app: &mut AppState, id: &str) {
    {
        let s = app.get::<State>();
        if !s.profiles.contains_key(id) || s.profile_order.len() <= 1 {
            return;
        }
    }
    for w in windows_in_order(app) {
        if w.kind == crate::browser::WindowKind::Private
            && sib::incognito_original(window_profile_id(&w)) == id
        {
            sib::close_window(app, window_id(&w));
        }
    }
    sib::hand_pages_out_of_profile(app, id);
    let order: Vec<String> = app
        .get::<State>()
        .profile_order
        .iter()
        .filter(|p| *p != id)
        .cloned()
        .collect();
    let default_id = app
        .get::<settings::State>()
        .settings
        .default_profile_id
        .clone();
    let fallback = if default_id == id {
        order[0].clone()
    } else {
        default_id.clone()
    };
    let mut never_translate =
        serde_json::to_value(&app.get::<settings::State>().settings.never_translate_sites)
            .expect("settings are JSON");
    never_translate.as_object_mut().expect("record").remove(id);
    let next_settings = sib::settings_with(
        &app.get::<settings::State>().settings,
        json!({ "defaultProfileId": fallback, "neverTranslateSites": never_translate }),
    );
    let engine = sib::engine_profile(app, id);
    let engine_left = !order.iter().any(|p| sib::engine_profile(app, p) == engine);
    let known = app.get::<State>().orphaned_profile_data.contains(&engine);
    let orphaned_add = engine_left && !known;

    for w in windows_in_order(app) {
        if window_profile_id(&w) != id {
            continue;
        }
        let other = window_tab_ids(&w).iter().find(|t| {
            app.get::<tabs::State>()
                .tabs
                .get(*t)
                .map(|tab| tab.profile_id.as_str())
                != Some(id)
        });
        if let Some(other) = other {
            let tab_profile = app
                .get::<tabs::State>()
                .tabs
                .get(other)
                .map(|t| t.profile_id.clone());
            let target = tab_profile
                .and_then(|p| window_active_tab_ids(&w).get(&p).cloned())
                .unwrap_or_else(|| other.clone());
            sib::activate_tab(app, &target);
        } else {
            sib::new_background_tab_activated(app, window_id(&w), &fallback);
        }
    }
    let doomed: Vec<String> = app
        .get::<tabs::State>()
        .tabs
        .values()
        .filter(|t| t.profile_id == id)
        .map(|t| t.id.clone())
        .collect();
    sib::remove_tabs(app, &doomed, false);
    for w in app.get_mut::<windows::State>().windows.values_mut() {
        window_active_tab_ids_mut(w).remove(id);
    }
    let roots = if engine_left {
        sib::engine_bookmark_roots(app, &engine)
    } else {
        None
    };
    if let Some(roots) = &roots {
        sib::remove_bookmark_tree(app, roots);
    }
    {
        let state = app.get_mut::<State>();
        state.profiles.remove(id);
        state.profile_order = order;
        if orphaned_add {
            state.orphaned_profile_data.push(engine.clone());
        }
    }
    app.get_mut::<settings::State>().settings = next_settings;
    if engine_left {
        app.get_mut::<history::State>().history.remove(&engine);
        app.get_mut::<bookmarks::State>()
            .bookmarks_ready
            .remove(&engine);
    }
    let g = app.get_mut::<groups::State>();
    g.groups.retain(|_, group| group.profile_id != id);
    g.closed_groups
        .retain(|c| c.tabs.iter().all(|t| t.profile_id != id));
    g.deleted_groups
        .retain(|c| c.tabs.iter().all(|t| t.profile_id != id));
    app.get_mut::<tabs::State>()
        .closed_tabs
        .retain(|c| c.tab.profile_id != id);
    app.get_mut::<pins::State>()
        .cleaned_tabs
        .retain(|c| c.tab.profile_id != id);
    let w = app.get_mut::<windows::State>();
    for closed in &mut w.closed_windows {
        closed.tabs.retain(|t| t.tab.profile_id != id);
    }
    w.closed_windows.retain(|c| !c.tabs.is_empty());
    w.parked_pins.remove(id);
    for w in windows_in_order(app) {
        if sib::view_tab_ids(app, window_id(&w)).is_empty() {
            sib::new_tab(app, window_id(&w));
        }
    }
}

// ---------------------------------------------------------------- actions.ts

fn choose_icon(app: &mut AppState, id: &str, icon: Option<String>) {
    let patch = UpdateProfilePatch {
        icon: Some(icon),
        switcher_dot: Some(None),
        ..Default::default()
    };
    update_profile(app, id, &patch);
}

fn set_share_top_apps(app: &mut AppState, id: &str, share: bool) {
    let patch = UpdateProfilePatch {
        share_top_apps: Some(share.then_some(true)),
        ..Default::default()
    };
    update_profile(app, id, &patch);
}

fn commit_profile_name(app: &mut AppState, id: &str, text: &str) -> bool {
    let name = js_trim(text);
    let Some(profile) = app.get::<State>().profiles.get(id) else {
        return false;
    };
    if name.is_empty() {
        return false;
    }
    if name != profile.name {
        let patch = UpdateProfilePatch {
            name: Some(name.to_owned()),
            ..Default::default()
        };
        update_profile(app, id, &patch);
    }
    true
}

// ------------------------------------------------- switcherMark.ts, switcherShown.ts

/// `switcherIconOf` (`switcherMark.ts:5`).
pub fn switcher_icon_of(profile: Option<&Profile>) -> Option<&str> {
    profile
        .filter(|p| p.switcher_dot != Some(true))
        .and_then(|p| p.icon.as_deref())
}

/// `LEGACY_SWITCHER_ICONS_KEY` (`switcherMark.ts:13`).
pub const LEGACY_SWITCHER_ICONS_KEY: &str = "profileSwitcherIcons";

/// `profilesKeepingLegacyDots` (`switcherMark.ts:15`).
pub fn profiles_keeping_legacy_dots(
    profiles: HashMap<String, Profile>,
    saved_settings: &Value,
) -> HashMap<String, Profile> {
    let keeps = match saved_settings.as_object() {
        Some(fields) => fields
            .get(LEGACY_SWITCHER_ICONS_KEY)
            .is_some_and(|v| *v != Value::Bool(true)),
        None => false,
    };
    if !keeps {
        return profiles;
    }
    profiles
        .into_iter()
        .map(|(id, p)| {
            let p = if p.icon.is_some() {
                Profile {
                    switcher_dot: Some(true),
                    ..p
                }
            } else {
                p
            };
            (id, p)
        })
        .collect()
}

/// `profileSwitcherShown` (`switcherShown.ts:3`).
pub fn profile_switcher_shown(
    show_profile_dots: bool,
    profile_count: usize,
    incognito: bool,
) -> bool {
    show_profile_dots && !incognito && profile_count > 1
}

/// `profileNames` (`createProfileRequest.ts:34`).
pub fn profile_names(names: &[String]) -> String {
    if names.len() <= 2 {
        return names.join(" & ");
    }
    format!("{}, & {} more", names[..2].join(", "), names.len() - 2)
}

// ------------------------------------------------------------ idleProfiles.ts

/// `profileUnloadDelay` (`idleProfiles.ts:13`), in ms.
pub fn profile_unload_delay(settings: &Settings) -> Option<i64> {
    if !(PLATFORM_FEATURES.unloads_idle_profiles && settings.memory_saver) {
        return None;
    }
    match settings.unload_profiles_after {
        SettingsUnloadProfilesAfter::Never => None,
        SettingsUnloadProfilesAfter::V5m => Some(5 * MINUTE_MS),
        SettingsUnloadProfilesAfter::V15m => Some(15 * MINUTE_MS),
        SettingsUnloadProfilesAfter::V1h => Some(60 * MINUTE_MS),
        SettingsUnloadProfilesAfter::V2h => Some(120 * MINUTE_MS),
    }
}

/// `idleProfiles()` (`idleProfiles.ts:15`): the idle book of the loaded profiles.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IdleProfiles {
    idle_since: BTreeMap<String, i64>,
    shown: BTreeSet<String>,
}

impl IdleProfiles {
    /// `noteShown`.
    pub fn note_shown(&mut self, in_use: &BTreeSet<String>, now: i64) {
        for profile in &self.shown {
            if !in_use.contains(profile) {
                self.idle_since.insert(profile.clone(), now);
            }
        }
        for profile in in_use {
            self.idle_since.remove(profile);
        }
        self.shown = in_use.clone();
    }

    /// The call of the function `ready`: the loaded profiles that have been idle for `delay_ms`.
    pub fn ready(
        &mut self,
        loaded: &[String],
        in_use: &BTreeSet<String>,
        now: i64,
        delay_ms: Option<i64>,
    ) -> Vec<String> {
        let Some(delay) = delay_ms else {
            self.idle_since.clear();
            self.shown.clear();
            return Vec::new();
        };
        self.note_shown(in_use, now);
        let mut profiles: Vec<&String> = Vec::new();
        for p in loaded {
            if !profiles.contains(&p) {
                profiles.push(p);
            }
        }
        self.idle_since.retain(|p, _| profiles.contains(&p));
        let mut due = Vec::new();
        for profile in profiles {
            if in_use.contains(profile) {
                continue;
            }
            let since = *self.idle_since.entry(profile.clone()).or_insert(now);
            if now - since >= delay {
                due.push(profile.clone());
            }
        }
        due
    }
}

// ---------------------------------------------------------- profileLoading.ts

fn profile_opened(s: &State, engine: &str) -> bool {
    !s.unloaded.contains_key(engine) && (!s.deferred || s.opened.contains_key(engine))
}

fn engine_profiles(app: &AppState) -> Vec<String> {
    let mut ids: Vec<&String> = app.get::<State>().profiles.keys().collect();
    ids.sort();
    let mut engines: Vec<String> = Vec::new();
    for id in ids {
        if sib::is_incognito_profile(id) {
            continue;
        }
        let engine = sib::engine_profile(app, id);
        if !engines.contains(&engine) {
            engines.push(engine);
        }
    }
    engines
}

fn shown_engine_profiles(app: &AppState) -> Vec<String> {
    let mut shown: Vec<String> = Vec::new();
    for w in windows_in_order(app) {
        let pid = window_profile_id(&w);
        let id = if sib::is_incognito_profile(pid) {
            sib::incognito_original(pid)
        } else {
            pid.to_owned()
        };
        if app.get::<State>().profiles.contains_key(&id) {
            let engine = sib::engine_profile(app, &id);
            if !shown.contains(&engine) {
                shown.push(engine);
            }
        }
    }
    shown
}

fn open_profile(app: &mut AppState, engine: &str) {
    if profile_opened(app.get::<State>(), engine) {
        return;
    }
    let was_unloaded = app.get::<State>().unloaded.contains_key(engine);
    let s = app.get_mut::<State>();
    s.opened.insert(engine.to_owned(), true);
    s.unloaded.remove(engine);
    if was_unloaded {
        sib::note_unloaded_profile_reopened(app);
    }
}

fn mark_profile_unloaded(app: &mut AppState, engine: &str) {
    if !profile_opened(app.get::<State>(), engine) {
        return;
    }
    let s = app.get_mut::<State>();
    s.opened.remove(engine);
    s.unloaded.insert(engine.to_owned(), true);
}

fn start_profile_loading(app: &mut AppState, fx: &mut Effects) {
    let deferred = app
        .get::<settings::State>()
        .settings
        .load_profiles_when_opened
        && app.get::<State>().work.lazy_profiles;
    let shown = shown_engine_profiles(app);
    let s = app.get_mut::<State>();
    s.deferred = deferred;
    s.opened = if deferred {
        shown.into_iter().map(|e| (e, true)).collect()
    } else {
        HashMap::new()
    };
    s.unloaded = HashMap::new();
    if deferred {
        fx.push(PlatformCall::ProfilesLoaded {});
    }
}

// -------------------------------------------------------- profileUnloading.ts

/// `closedByUnload` (`profileUnloading.ts:36`).
pub fn closed_by_unload(app: &mut AppState, tab_id: &str) -> bool {
    let Some(tab) = app.get::<tabs::State>().tabs.get(tab_id) else {
        return false;
    };
    let profile_id = tab.profile_id.clone();
    if sib::has_page(app, tab_id) {
        app.get_mut::<State>().work.slept_by_unload.remove(tab_id);
    }
    let engine = sib::engine_profile(app, &profile_id);
    let work = &app.get::<State>().work;
    work.unloading_now.contains(&engine) || work.slept_by_unload.contains(tab_id)
}

fn unloadable(profile: &str) -> bool {
    profile != PERSONAL && !sib::is_incognito_profile(profile)
}

fn apply_unload(app: &mut AppState, profile: &str, result: &UnloadResult) -> bool {
    let shown = shown_engine_profiles(app).iter().any(|p| p == profile);
    for id in &result.tabs {
        if !app.get::<tabs::State>().tabs.contains_key(id) || !sib::has_page(app, id) {
            continue;
        }
        sib::idle_tab_page(app, id);
        app.get_mut::<State>()
            .work
            .slept_by_unload
            .insert(id.clone());
    }
    if result.status == "busy" {
        return false;
    }
    if shown {
        open_profile(app, profile);
        return false;
    }
    if !profile_opened(app.get::<State>(), profile) {
        return false;
    }
    app.get_mut::<State>()
        .work
        .unloaded_here
        .insert(profile.to_owned());
    mark_profile_unloaded(app, profile);
    sib::note_profile_unloaded(app);
    true
}

fn wake_shown(app: &mut AppState, ids: &[String]) {
    let active: BTreeSet<String> = windows_in_order(app)
        .iter()
        .filter_map(|w| window_active_tab_ids(w).get(window_profile_id(w)).cloned())
        .collect();
    for id in ids {
        if app.get::<tabs::State>().tabs.contains_key(id)
            && active.contains(id)
            && sib::needs_page(app, id)
        {
            sib::wake_tab_page(app, id);
        }
    }
}

/// Starts `unload(profile, settled)`; `true` when it waits for the platform call.
fn start_unload(app: &mut AppState, fx: &mut Effects, profile: &str, settled: bool) -> bool {
    if !unloadable(profile)
        || profile_unload_delay(&app.get::<settings::State>().settings).is_none()
        || shown_engine_profiles(app).iter().any(|p| p == profile)
    {
        return false;
    }
    let work = &mut app.get_mut::<State>().work;
    work.unloading_now.insert(profile.to_owned());
    work.unload_settled = settled;
    fx.push(PlatformCall::ProfilesUnloadIdle {
        profile: profile.to_owned(),
    });
    true
}

/// The loop of `checkIdleProfiles` over the due profiles.
fn unload_next(app: &mut AppState, fx: &mut Effects) {
    loop {
        let next = app.get_mut::<State>().work.check_queue.pop_front();
        let Some(profile) = next else {
            app.get_mut::<State>().work.checking = false;
            return;
        };
        if start_unload(app, fx, &profile, false) {
            return;
        }
    }
}

fn shown_set(app: &AppState) -> BTreeSet<String> {
    shown_engine_profiles(app).into_iter().collect()
}

fn check_idle_profiles(app: &mut AppState, fx: &mut Effects, now: Option<Number>) {
    let now = now.map_or(0, |n| n.0 as i64);
    let delay = profile_unload_delay(&app.get::<settings::State>().settings);
    if delay.is_none() {
        app.get_mut::<State>()
            .work
            .idle
            .ready(&[], &BTreeSet::new(), now, None);
        return;
    }
    if app.get::<State>().work.checking || engine_profiles(app).len() < 2 {
        return;
    }
    let work = &mut app.get_mut::<State>().work;
    work.checking = true;
    work.check_now = now;
    fx.push(PlatformCall::ProfilesLoaded {});
}

fn idle_profiles_loaded(app: &mut AppState, fx: &mut Effects, loaded: Option<Vec<String>>) {
    let Some(loaded) = loaded else {
        sib::warn_for(app, "[profiles] couldn't unload idle profiles");
        app.get_mut::<State>().work.checking = false;
        return;
    };
    let loaded: Vec<String> = loaded
        .into_iter()
        .filter(|p| unloadable(p) && profile_opened(app.get::<State>(), p))
        .collect();
    let shown = shown_set(app);
    let delay = profile_unload_delay(&app.get::<settings::State>().settings);
    let work = &mut app.get_mut::<State>().work;
    let due = work.idle.ready(&loaded, &shown, work.check_now, delay);
    work.check_queue = due.into();
    unload_next(app, fx);
}

fn unload_idle_result(
    app: &mut AppState,
    fx: &mut Effects,
    profile: &str,
    result: Option<UnloadResult>,
) {
    app.get_mut::<State>().work.unloading_now.remove(profile);
    let Some(result) = result else {
        sib::warn_for(app, "[profiles] couldn't unload idle profiles");
        let work = &mut app.get_mut::<State>().work;
        work.checking = false;
        work.check_queue.clear();
        return;
    };
    if result.held_by.as_deref() != Some("sleeping") || app.get::<State>().work.unload_settled {
        apply_unload(app, profile, &result);
        unload_next(app, fx);
    }
}

fn start_profile_unloading(app: &mut AppState) {
    let delay = profile_unload_delay(&app.get::<settings::State>().settings);
    let shown = shown_set(app);
    let work = &mut app.get_mut::<State>().work;
    match delay {
        None => {
            work.idle.ready(&[], &BTreeSet::new(), 0, None);
        }
        Some(_) => work.idle.note_shown(&shown, work.check_now),
    }
}

// ------------------------------------------------------------- profileData.ts

fn next_cleanup(app: &mut AppState) {
    let state = app.get_mut::<State>();
    let next = state
        .orphaned_profile_data
        .iter()
        .find(|e| !state.work.cleanup_failed.contains(*e))
        .cloned();
    state.work.cleanup_running = next.is_some();
    state.work.cleanup_engine = next;
}

fn drain_profile_data(app: &mut AppState) {
    if !app.get::<State>().work.cleanup_running {
        next_cleanup(app);
    }
}

fn profile_data_cleanup_tick(app: &mut AppState, fx: &mut Effects) {
    let Some(engine) = app.get_mut::<State>().work.cleanup_engine.take() else {
        return;
    };
    let ids: Vec<String> = app.get::<State>().profiles.keys().cloned().collect();
    if ids.iter().any(|id| sib::engine_profile(app, id) == engine) {
        profile_data_deleted(app, &engine);
        next_cleanup(app);
        return;
    }
    sib::forget_site_data_profile(app, &engine);
    fx.push(PlatformCall::ProfilesDeleteData { profile: engine });
}

fn profile_data_cleanup_result(app: &mut AppState, engine: &str, remaining: &[String]) {
    sib::note_profile_data_deleted(app, remaining.is_empty());
    if remaining.is_empty() {
        profile_data_deleted(app, engine);
    } else {
        app.get_mut::<State>()
            .work
            .cleanup_failed
            .insert(engine.to_owned());
        let label = if engine.is_empty() {
            "original profile"
        } else {
            engine
        };
        sib::warn_for(
            app,
            &format!(
                "Couldn't delete profile data ({label}): {}",
                remaining.join(", ")
            ),
        );
    }
    next_cleanup(app);
}

// ------------------------------------------- iconPicker.ts, createProfileRequest.ts

fn measured_anchor(view: &Value) -> Option<IconPickerAnchor> {
    let n = |k: &str| view.get(k).and_then(Value::as_f64).map(Number);
    let (x, y, width, height) = (n("x")?, n("y")?, n("width")?, n("height")?);
    (width.0 != 0.0).then_some(IconPickerAnchor {
        x,
        y,
        width,
        height,
    })
}

fn open_profile_icon_picker(
    app: &mut AppState,
    profile_id: &str,
    window_id: &str,
    place: ProfileMenuPlace,
    view: &Value,
) {
    let anchor = measured_anchor(view);
    if app.get::<State>().profiles.contains_key(profile_id) {
        app.get_mut::<State>().picker = Some(ProfileIconPicker {
            profile_id: profile_id.to_owned(),
            window_id: window_id.to_owned(),
            place,
            anchor,
        });
    }
}

fn profile_icon_picker_in(s: &State, window_id: &str) -> bool {
    s.picker
        .as_ref()
        .is_some_and(|p| p.window_id == window_id && s.profiles.contains_key(&p.profile_id))
}

fn request_create_profile(
    app: &mut AppState,
    window_id: Option<String>,
    preset: Option<CreateProfilePreset>,
) {
    let preset = preset.unwrap_or_default();
    let known = window_id
        .as_deref()
        .is_some_and(|id| !id.is_empty() && app.get::<windows::State>().windows.contains_key(id));
    let s = app.get_mut::<State>();
    if known {
        s.request = Some(CreateProfileRequest { window_id, preset });
    } else {
        s.sheet = Some(CreateProfileRequest {
            window_id: None,
            preset,
        });
    }
}

// ------------------------------------------------------------- profileMenu.ts

/// `ProfileMenuState` (`profileMenu.ts:20`).
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileMenuState {
    pub place: ProfileMenuPlace,
    pub has_icon: bool,
    pub colors: Vec<super::ProfileColorChoice>,
    pub color: Option<ProfileColor>,
    pub is_default: bool,
    pub only: bool,
    pub shares_top_apps: Option<bool>,
}

/// `profileMenuItems` (`profileMenu.ts:30`).
pub fn profile_menu_items(m: &ProfileMenuState) -> Vec<Value> {
    let item =
        |id: &str, title: &str, symbol: &str| json!({ "id": id, "title": title, "symbol": symbol });
    let colors: Vec<Value> = m
        .colors
        .iter()
        .map(|c| json!({ "value": { "color": c.id }, "title": c.title, "swatch": c.swatch, "checked": Some(c.id) == m.color }))
        .collect();
    let mut icon = item(
        "icon",
        if m.has_icon {
            "Change Icon…"
        } else {
            "Add Icon…"
        },
        "photo",
    );
    icon["title"] = json!(if m.has_icon {
        "Change Icon…"
    } else {
        "Add Icon…"
    });
    let mut color = item("color", "Color", "paintpalette");
    color["children"] = Value::Array(colors);
    let top_apps = m.shares_top_apps.map(|checked| {
        let mut v = item("topApps", "Share Top Apps", "square.grid.2x2");
        v["checked"] = json!(checked);
        v
    });
    let place_items = if m.place == ProfileMenuPlace::Sidebar {
        vec![Some(item("settings", "Profile Settings…", "gearshape"))]
    } else {
        let mut default = item("default", "Make Default Profile", "star");
        default["enabled"] = json!(!m.is_default);
        vec![
            Some(item("details", "Show Profile Details", "info.circle")),
            Some(default),
        ]
    };
    let mut delete = item("delete", "Delete Profile…", "trash");
    delete["enabled"] = json!(!m.only);
    sib::menu_sections(vec![
        vec![
            Some(item("rename", "Rename…", "pencil")),
            Some(icon),
            Some(color),
        ],
        vec![top_apps],
        place_items,
        vec![Some(delete)],
    ])
}

fn rename_profile(app: &mut AppState, fx: &mut Effects, profile_id: &str, window_id: Option<&str>) {
    let Some(profile) = app.get::<State>().profiles.get(profile_id) else {
        return;
    };
    let mut options = json!({
        "title": "Rename Profile",
        "value": profile.name,
        "placeholder": "Profile name",
        "confirmTitle": "Rename",
    });
    if let Some(window_id) = window_id {
        options["windowId"] = json!(window_id);
    }
    fx.push(PlatformCall::DialogsPrompt { options });
}

fn show_profile_menu(app: &mut AppState, profile_id: &str, arg1: &ShowProfileMenuArg1) {
    let s = app.get::<State>();
    let Some(profile) = s.profiles.get(profile_id) else {
        return;
    };
    let sharing = !sharing_profiles(s, profile_id).is_empty();
    let items = profile_menu_items(&ProfileMenuState {
        place: arg1.place,
        has_icon: profile.icon.is_some(),
        colors: arg1.colors.clone(),
        color: Some(profile.color),
        is_default: app.get::<settings::State>().settings.default_profile_id == profile_id,
        only: s.profile_order.len() <= 1,
        shares_top_apps: sharing.then_some(profile.share_top_apps == Some(true)),
    });
    sib::choose_from_menu(app, items);
}

fn profile_menu_chosen(
    app: &mut AppState,
    fx: &mut Effects,
    profile_id: &str,
    arg1: &ShowProfileMenuArg1,
    choice: Option<Value>,
) {
    let Some(choice) = choice else { return };
    if let Some(color) = choice.get("color") {
        let color: ProfileColor = serde_json::from_value(color.clone()).expect("a profile colour");
        return choose_preset_color(app, profile_id, color);
    }
    match choice.as_str() {
        Some("icon") => {
            open_profile_icon_picker(app, profile_id, &arg1.window_id, arg1.place, &arg1.anchor)
        }
        Some("rename") => rename_profile(app, fx, profile_id, Some(&arg1.window_id)),
        Some("topApps") => {
            let shares = app
                .get::<State>()
                .profiles
                .get(profile_id)
                .is_some_and(|p| p.share_top_apps == Some(true));
            set_share_top_apps(app, profile_id, !shares);
        }
        Some("settings") => settings::apply(
            app,
            settings::Action::OpenSettings {
                pane: Some(settings::OpenSettingsPane::Profiles),
                profile_id: Some(profile_id.to_owned()),
                detail: None,
            },
            fx,
        ),
        Some("details") => settings::apply(
            app,
            settings::Action::GoToSettingsPane {
                pane: settings::GoToSettingsPanePane::Profiles,
                profile_id: Some(profile_id.to_owned()),
                detail: None,
            },
            fx,
        ),
        Some("default") => set_default_profile(app, profile_id),
        Some("delete") => crate::browser::apply(
            app,
            crate::browser::Action::DeleteProfile {
                profile_id: profile_id.to_owned(),
                window_id: Some(arg1.window_id.clone()),
            },
            fx,
        ),
        _ => {}
    }
}

// ----------------------------------------------------------- themeActions.ts

fn shared_color(color: ProfileColor) -> look::ProfileColor {
    let name = serde_json::to_value(color).expect("colour");
    look::ProfileColor::from_name(name.as_str().expect("name")).expect("same names")
}

fn browser_color(color: look::ProfileColor) -> ProfileColor {
    serde_json::from_value(json!(color.name())).expect("same names")
}

fn num(n: f64) -> Value {
    if n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0 {
        json!(n as i64)
    } else {
        json!(n)
    }
}

fn look_value(l: &look::ThemeLook) -> serde_json::Map<String, Value> {
    let stops = l.stops.iter().map(|s| {
        json!({ "x": num(s.x), "y": num(s.y), "color": { "r": num(s.color.r), "g": num(s.color.g), "b": num(s.color.b), "a": num(s.color.a) } })
    });
    let mut out = serde_json::Map::new();
    out.insert("stops".into(), Value::Array(stops.collect()));
    out.insert("intensity".into(), num(l.intensity));
    out.insert("grain".into(), num(l.grain));
    out.insert("texture".into(), json!(l.texture.name()));
    out.insert("translucency".into(), num(l.translucency));
    if let Some(a) = l.angle {
        out.insert("angle".into(), num(a));
    }
    out
}

fn theme_value(theme: &look::ProfileTheme) -> Value {
    let mut out = look_value(&theme.look);
    out.insert("mode".into(), json!(theme.mode.name()));
    if let Some(dark) = &theme.dark {
        out.insert("dark".into(), Value::Object(look_value(dark)));
    }
    Value::Object(out)
}

fn look_from(v: &Value) -> Option<look::ThemeLook> {
    let f = |k: &str| v.get(k)?.as_f64();
    let stops = v
        .get("stops")?
        .as_array()?
        .iter()
        .map(|s| {
            let c = s.get("color")?;
            let g = |k: &str| c.get(k)?.as_f64();
            Some(look::ThemeStop {
                color: look::ThemeColor {
                    r: g("r")?,
                    g: g("g")?,
                    b: g("b")?,
                    a: g("a")?,
                },
                x: s.get("x")?.as_f64()?,
                y: s.get("y")?.as_f64()?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(look::ThemeLook {
        stops,
        intensity: f("intensity")?,
        grain: f("grain")?,
        texture: look::ThemeTexture::from_name(v.get("texture")?.as_str()?)?,
        translucency: f("translucency")?,
        angle: f("angle"),
    })
}

/// The stored theme (a sanitized `ProfileTheme` as JSON), or `None` for `null` and for a value that is not a theme.
fn theme_from(v: &Value) -> Option<look::ProfileTheme> {
    Some(look::ProfileTheme {
        look: look_from(v)?,
        mode: look::ThemeMode::from_name(v.get("mode")?.as_str()?)?,
        dark: v.get("dark").and_then(look_from),
    })
}

fn preset_slot(p: &mut ProfilePresetThemes, color: ProfileColor) -> &mut Option<Value> {
    match color {
        ProfileColor::Plum => &mut p.plum,
        ProfileColor::Blue => &mut p.blue,
        ProfileColor::Purple => &mut p.purple,
        ProfileColor::Pink => &mut p.pink,
        ProfileColor::Red => &mut p.red,
        ProfileColor::Orange => &mut p.orange,
        ProfileColor::Yellow => &mut p.yellow,
        ProfileColor::Green => &mut p.green,
        ProfileColor::Neutral => &mut p.neutral,
    }
}

const ALL_COLORS: [ProfileColor; 9] = [
    ProfileColor::Plum,
    ProfileColor::Blue,
    ProfileColor::Purple,
    ProfileColor::Pink,
    ProfileColor::Red,
    ProfileColor::Orange,
    ProfileColor::Yellow,
    ProfileColor::Green,
    ProfileColor::Neutral,
];

fn preset_fields(profile: &Profile) -> look::PresetLookFields {
    let preset_themes = profile.preset_themes.clone().map(|mut p| {
        ALL_COLORS
            .into_iter()
            .filter_map(|c| {
                let theme = theme_from(preset_slot(&mut p, c).as_ref()?)?;
                Some((shared_color(c), theme))
            })
            .collect()
    });
    look::PresetLookFields {
        color: shared_color(profile.color),
        theme: profile.theme.as_ref().and_then(theme_from),
        preset_themes,
    }
}

fn own_look(profile: &Profile, theme: Option<look::ProfileTheme>) -> SetProfileLookLook {
    let own = with_own_look(&preset_fields(profile), theme);
    let mut preset_themes = ProfilePresetThemes::default();
    for (color, theme) in &own.preset_themes {
        *preset_slot(&mut preset_themes, browser_color(*color)) = Some(theme_value(theme));
    }
    SetProfileLookLook {
        color: None,
        theme: Some(own.theme.as_ref().map(theme_value)),
        preset_themes: Some(preset_themes),
    }
}

fn set_profile_theme(app: &mut AppState, profile_id: &str, theme: Option<&Value>) {
    let Some(profile) = app.get::<State>().profiles.get(profile_id).cloned() else {
        return;
    };
    sib::adopt_profile_theme(app, profile_id);
    let look = own_look(&profile, theme.and_then(theme_from));
    set_profile_look(app, profile_id, &look);
}

fn choose_preset_color(app: &mut AppState, profile_id: &str, color: ProfileColor) {
    let Some(profile) = app.get::<State>().profiles.get(profile_id).cloned() else {
        return;
    };
    let Some(choice) = with_preset_chosen(&preset_fields(&profile), shared_color(color)) else {
        return;
    };
    sib::adopt_profile_theme(app, profile_id);
    let look = SetProfileLookLook {
        color: Some(browser_color(choice.color)),
        theme: Some(choice.theme.as_ref().map(theme_value)),
        preset_themes: None,
    };
    set_profile_look(app, profile_id, &look);
}

fn reset_preset_theme(app: &mut AppState, profile_id: &str) {
    let Some(profile) = app.get::<State>().profiles.get(profile_id).cloned() else {
        return;
    };
    if profile.theme.as_ref().is_none_or(Value::is_null) {
        return;
    }
    sib::adopt_profile_theme(app, profile_id);
    set_profile_look(app, profile_id, &own_look(&profile, None));
    sib::note_theme_preset(app, "reset");
}

// ------------------------------------------------------------------- reducer

pub(super) fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    match action {
        Action::ApplyUnload { profile, result } => {
            apply_unload(app, &profile, &result);
        }
        Action::CheckIdleProfiles { now } => check_idle_profiles(app, fx, now),
        Action::ChooseIcon { profile_id, icon } => choose_icon(app, &profile_id, icon),
        Action::ChoosePresetColor { profile_id, color } => {
            choose_preset_color(app, &profile_id, color)
        }
        Action::ChooseProfileIcon { arg0, icon } => {
            choose_icon(app, &arg0.profile_id, icon);
            app.get_mut::<State>().picker = None;
        }
        Action::CloseProfileIconPicker => app.get_mut::<State>().picker = None,
        Action::CloseProfileIconPickerIfShown { picker } => {
            if app.get::<State>().picker.as_ref() == Some(&picker) {
                app.get_mut::<State>().picker = None;
            }
        }
        Action::CloseProfileIconPickerIn { window_id } => {
            if profile_icon_picker_in(app.get::<State>(), &window_id) {
                app.get_mut::<State>().picker = None;
            }
        }
        Action::CommitProfileName { profile_id, text } => {
            commit_profile_name(app, &profile_id, &text);
        }
        Action::CreateProfile { options, now } => create_profile(app, options, now),
        Action::DeleteProfile { id } => delete_profile(app, &id),
        Action::EndCreateProfileRequest { .. } => app.get_mut::<State>().request = None,
        Action::ImportArcTheme { profile_id } => sib::import_arc_theme(app, &profile_id),
        Action::MarkProfileUnloaded { engine } => mark_profile_unloaded(app, &engine),
        Action::OpenProfile { engine } => open_profile(app, &engine),
        Action::OpenProfileIconPicker {
            profile_id,
            window_id,
            place,
            anchor_view,
        } => {
            open_profile_icon_picker(app, &profile_id, &window_id, place, &anchor_view);
        }
        Action::ProfileDataDeleted { engine_id } => profile_data_deleted(app, &engine_id),
        Action::RenameProfile {
            profile_id,
            window_id,
        } => {
            rename_profile(app, fx, &profile_id, window_id.as_deref());
        }
        Action::ReorderProfiles { ids } => reorder_profiles(app, &ids),
        Action::RequestClearData { window_id } => app.get_mut::<State>().window_id = window_id,
        Action::RequestCreateProfile { window_id, preset } => {
            request_create_profile(app, window_id, preset)
        }
        Action::ResetPresetTheme { profile_id } => reset_preset_theme(app, &profile_id),
        Action::SetDefaultProfile { id } => set_default_profile(app, &id),
        Action::SetProfileLook { id, look } => set_profile_look(app, &id, &look),
        Action::SetProfileTheme { profile_id, theme } => {
            set_profile_theme(app, &profile_id, theme.as_ref())
        }
        Action::SetShareTopApps { profile_id, share } => {
            set_share_top_apps(app, &profile_id, share)
        }
        Action::ShowProfileMenu { profile_id, arg1 } => show_profile_menu(app, &profile_id, &arg1),
        Action::StartProfileDataCleanup => {
            app.get_mut::<State>().work.cleanup_failed.clear();
            drain_profile_data(app);
        }
        Action::StartProfileLoading => start_profile_loading(app, fx),
        Action::StartProfileUnloading => start_profile_unloading(app),
        Action::TakeCreateProfileSheet => {
            if app.get::<State>().sheet.is_some() {
                app.get_mut::<State>().sheet = None;
            }
        }
        Action::UpdateProfile { id, patch } => update_profile(app, &id, &patch),
        Action::LoadedProfilesResult { engines } => match engines {
            Some(engines) => engines.iter().for_each(|e| open_profile(app, e)),
            None => app.get_mut::<State>().deferred = false,
        },
        Action::IdleProfilesLoaded { loaded } => idle_profiles_loaded(app, fx, loaded),
        Action::UnloadIdleResult { profile, result } => {
            unload_idle_result(app, fx, &profile, result)
        }
        Action::UnloadSettled { profile } => {
            if !start_unload(app, fx, &profile, true) {
                unload_next(app, fx);
            }
        }
        Action::WakeShown { ids } => wake_shown(app, &ids),
        Action::DrainProfileData => drain_profile_data(app),
        Action::ProfileDataCleanupTick => profile_data_cleanup_tick(app, fx),
        Action::ProfileDataCleanupResult { engine, remaining } => {
            profile_data_cleanup_result(app, &engine, &remaining);
        }
        Action::RenamePrompted { profile_id, name } => {
            if let Some(name) = name {
                commit_profile_name(app, &profile_id, &name);
            }
        }
        Action::ProfileMenuChosen {
            profile_id,
            arg1,
            choice,
        } => {
            profile_menu_chosen(app, fx, &profile_id, &arg1, choice);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(id: &str, color: ProfileColor) -> Profile {
        Profile {
            id: id.into(),
            name: id.into(),
            color,
            ..Profile::default()
        }
    }

    #[test]
    fn name_is_trimmed_like_js() {
        assert_eq!(js_trim("\u{feff} a \u{85}"), "a \u{85}");
        assert_eq!(
            profile_names(&["a".into(), "b".into(), "c".into(), "d".into()]),
            "a, b, & 2 more"
        );
    }

    #[test]
    fn unused_color_skips_taken() {
        let mut m = HashMap::new();
        m.insert("a".to_owned(), profile("a", ProfileColor::Blue));
        assert_eq!(unused_profile_color(&m), ProfileColor::Green);
    }

    #[test]
    fn dot_drops_when_icon_changes() {
        let p = Profile {
            switcher_dot: Some(true),
            ..profile("a", ProfileColor::Blue)
        };
        let patch = UpdateProfilePatch {
            icon: Some(Some("x".into())),
            ..Default::default()
        };
        assert_eq!(patched_profile(&p, &patch).switcher_dot, None);
        assert_eq!(
            patched_profile(&p, &UpdateProfilePatch::default()).switcher_dot,
            Some(true)
        );
    }

    #[test]
    fn idle_profile_is_ready_after_delay() {
        let mut idle = IdleProfiles::default();
        let none = BTreeSet::new();
        assert!(idle.ready(&["p".into()], &none, 0, Some(100)).is_empty());
        assert_eq!(
            idle.ready(&["p".into()], &none, 100, Some(100)),
            vec!["p".to_owned()]
        );
    }
}
