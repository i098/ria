// Ported from ardia apps/browser/src/state/browser/{model,structure,changes}.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of those files, with changes.

//! Pure helpers of the browser store: ids, incognito profiles, tab and window lookups, labels (`model.ts`), the
//! group and split indexes (`structure.ts`), and the changed-id list (`changes.ts`).
//!
//! Changes from Ardia:
//! - `Date.now()` is the `now` argument of `make_tab`, and `newId` and `navigationTo` use process counters.
//! - `structure.ts` caches each index in a `WeakMap` keyed by the object identity of the record. Rust has no object
//!   identity, so each call builds the index again.
//! - `changedIds` (`changes.ts`) compares object identity. `changed_ids` compares values, so it can list one id more
//!   than Ardia when an action replaces an object with an equal one.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;

use super::pending_siblings::split_tab_ids;
use super::types::*;
use crate::kernel::Number;
use crate::shared::single_line::single_line;

const DEFAULT_PROFILE_ID: &str = "default";

static COUNTER: AtomicU64 = AtomicU64::new(0);
static NAV_SEQ: AtomicU64 = AtomicU64::new(0);
static LAUNCH: LazyLock<String> = LazyLock::new(|| {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    base36(nanos / 1_000_000) + &base36(nanos % 1_679_616)
});

fn base36(mut n: u128) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if n == 0 {
        return "0".into();
    }
    let mut out = Vec::new();
    while n > 0 {
        out.push(DIGITS[(n % 36) as usize]);
        n /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// `newId` (`model.ts:12`): `<prefix>-<launch>-<counter in base 36>`.
pub fn new_id(prefix: &str) -> String {
    let launch = &*LAUNCH;
    format!("{prefix}-{launch}-{}", base36(u128::from(COUNTER.fetch_add(1, Ordering::Relaxed) + 1)))
}

/// `pinKeyOf` (`model.ts:14`).
pub fn pin_key_of<'a>(id: &'a str, pin_key: Option<&'a str>) -> &'a str {
    pin_key.unwrap_or(id)
}

/// `familyKeyOf` (`model.ts:15`).
pub fn family_key_of(t: &Tab) -> String {
    format!("{}\u{1}{}", t.profile_id, pin_key_of(&t.id, t.pin_key.as_deref()))
}

/// `navigationTo` (`model.ts:18`).
pub fn navigation_to(url: &str, user_initiated: bool) -> Navigation {
    Navigation {
        url: url.to_string(),
        seq: Number((NAV_SEQ.fetch_add(1, Ordering::Relaxed) + 1) as f64),
        user_initiated: user_initiated.then_some(true),
    }
}

/// `incognitoProfileId` (`model.ts:20`).
pub fn incognito_profile_id(window_id: &str, original_profile_id: &str) -> String {
    format!("incognito:{window_id}@{original_profile_id}")
}

/// `isIncognitoProfile` (`model.ts:21`).
pub fn is_incognito_profile(profile_id: &str) -> bool {
    profile_id.starts_with("incognito:")
}

/// `incognitoOriginal` (`model.ts:22`). `indexOf` counts UTF-16 units and `slice` counts from the match, so a byte
/// index gives the same text.
pub fn incognito_original(profile_id: &str) -> String {
    profile_id.find('@').map_or_else(|| DEFAULT_PROFILE_ID.to_string(), |at| profile_id[at + 1..].to_string())
}

/// `engineProfile` (`model.ts:27`).
pub fn engine_profile(profiles: &HashMap<String, Profile>, profile_id: &str) -> String {
    if is_incognito_profile(profile_id) {
        let head = profile_id.find('@').map_or(profile_id, |at| &profile_id[..at]);
        return format!("{head}@{}", engine_profile(profiles, &incognito_original(profile_id)));
    }
    let id = profiles.get(profile_id).and_then(|p| p.data_id.as_deref()).unwrap_or(profile_id);
    if id == DEFAULT_PROFILE_ID { String::new() } else { id.to_string() }
}

/// `privateSession` (`model.ts:36`).
pub fn private_session(profiles: &HashMap<String, Profile>, profile_id: &str) -> Option<String> {
    is_incognito_profile(profile_id).then(|| engine_profile(profiles, &incognito_original(profile_id)))
}

fn incognito_profile() -> Profile {
    Profile { id: "incognito".into(), name: "Incognito".into(), color: "neutral".into(), icon: None, created_at: Number(0.0), ..Default::default() }
}

/// `IDLE_LIVE` (`model.ts:41`).
pub fn idle_live() -> TabLive {
    TabLive::default()
}

/// `makeTab` (`model.ts:49`). `id` is `newId("tab")` in Ardia; `now` is `Date.now()`.
pub fn make_tab(id: String, window_id: &str, profile_id: &str, url: &str, snapshot: Option<&TabSnapshot>, now: f64) -> Tab {
    let mut tab = Tab {
        id,
        window_id: window_id.into(),
        profile_id: profile_id.into(),
        url: url.into(),
        title: String::new(),
        favicon: None,
        pin: None,
        muted: false,
        zoom: Number(1.0),
        custom_title: None,
        custom_icon: None,
        pinned_url: None,
        opener_id: None,
        ..Default::default()
    };
    if let Some(snap) = snapshot {
        tab.url = snap.url.clone();
        tab.title = snap.title.clone();
        tab.favicon = snap.favicon.clone();
        tab.pin = snap.pin.clone();
        tab.muted = snap.muted;
        tab.zoom = snap.zoom;
        tab.custom_title = snap.custom_title.clone();
        tab.custom_icon = snap.custom_icon.clone();
        tab.profile_id = snap.profile_id.clone();
        if snap.pinned_url.is_some() {
            tab.pinned_url = snap.pinned_url.clone();
        }
        tab.engine = snap.engine.clone();
    }
    tab.page = if url.is_empty() { TabPage::Idle { wake: None } } else { TabPage::Loaded { nav: navigation_to(url, false) } };
    tab.created_at = Number(now);
    tab.last_active_at = snapshot.and_then(|s| s.last_active_at).unwrap_or(Number(now));
    // `snapshot?.lastActiveAt` is falsy for 0.
    if snapshot.and_then(|s| s.last_active_at).is_some_and(|n| n.0 != 0.0 && !n.0.is_nan()) {
        tab.restored_at = Some(Number(now));
    }
    tab
}

/// `byRecentUse` (`model.ts:73`): the order of `a` against `b`.
pub fn by_recent_use(a: &Tab, b: &Tab) -> std::cmp::Ordering {
    let key = i64::from(a.unseen.unwrap_or(false)) - i64::from(b.unseen.unwrap_or(false));
    if key != 0 {
        return key.cmp(&0);
    }
    (b.last_active_at.0).partial_cmp(&a.last_active_at.0).unwrap_or(std::cmp::Ordering::Equal)
}

/// `snapshotTab` (`model.ts:76`).
pub fn snapshot_tab(t: &Tab) -> TabSnapshot {
    TabSnapshot {
        url: t.url.clone(),
        title: t.title.clone(),
        favicon: t.favicon.clone(),
        pin: t.pin.clone(),
        muted: t.muted,
        zoom: t.zoom,
        custom_title: t.custom_title.clone(),
        custom_icon: t.custom_icon.clone(),
        pinned_url: t.pinned_url.clone(),
        engine: t.engine.clone(),
        profile_id: t.profile_id.clone(),
        last_active_at: Some(t.last_active_at),
    }
}

/// `isPinnedRow` (`model.ts:93`).
pub fn is_pinned_row(t: Option<&Tab>) -> bool {
    matches!(t.and_then(|t| t.pin.as_ref()), Some(TabPin::Row))
}

/// `samePin` (`model.ts:94`). Two tile pins are the same when `folder` is the same object in Ardia; Rust compares values.
pub fn same_pin(a: Option<&TabPin>, b: Option<&TabPin>) -> bool {
    match (a, b) {
        (None, None) | (Some(TabPin::Row), Some(TabPin::Row)) => true,
        (Some(TabPin::Tile { folder: x }), Some(TabPin::Tile { folder: y })) => x == y,
        _ => false,
    }
}

/// `pinnedSectionTitle` (`model.ts:96`).
pub fn pinned_section_title(profiles: &HashMap<String, Profile>, tab: Option<&Tab>) -> String {
    let name = tab.and_then(|t| profiles.get(&t.profile_id)).map_or("Profile", |p| p.name.as_str());
    format!("Pinned in {name}")
}

/// `togglePinTitle` (`model.ts:97`).
pub fn toggle_pin_title(tab: Option<&Tab>) -> &'static str {
    if is_pinned_row(tab) {
        "Unpin"
    } else if tab.is_some_and(|t| t.pin.is_some()) {
        "Unpin from Top Apps"
    } else {
        "Move to Top Apps"
    }
}

/// `pinSection` (`model.ts:100`).
pub fn pin_section(t: Option<&Tab>) -> &'static str {
    match t.and_then(|t| t.pin.as_ref()) {
        Some(TabPin::Tile { .. }) => "tile",
        Some(TabPin::Row) => "row",
        None => "none",
    }
}

/// `pinnedGroupMembers` (`structure.ts:7`).
pub fn pinned_group_members(groups: &HashMap<String, TabGroup>) -> HashSet<&str> {
    groups.values().filter(|g| g.pinned).flat_map(|g| g.tab_ids.iter().map(String::as_str)).collect()
}

/// `inPinnedContainer` (`model.ts:103`).
pub fn in_pinned_container(tabs: &HashMap<String, Tab>, groups: &HashMap<String, TabGroup>, id: &str) -> bool {
    let Some(tab) = tabs.get(id) else { return false };
    tab.pin.is_some() || pinned_group_members(groups).contains(id)
}

/// `viewTabIds` (`model.ts:109`).
pub fn view_tab_ids(s: &BrowserState, window_id: &str, profile_id: Option<&str>) -> Vec<String> {
    let Some(w) = s.windows.get(window_id) else { return vec![] };
    let profile = profile_id.unwrap_or(&w.profile_id);
    w.tab_ids.iter().filter(|id| s.tabs.get(*id).is_some_and(|t| t.profile_id == profile)).cloned().collect()
}

/// `pinnedItemsRemain` (`model.ts:115`).
pub fn pinned_items_remain(s: &BrowserState, id: &str) -> bool {
    let Some(w) = s.tabs.get(id).and_then(|t| s.windows.get(&t.window_id)) else { return false };
    if view_tab_ids(s, &w.id, None).iter().any(|t| t != id && in_pinned_container(&s.tabs, &s.groups, t)) {
        return true;
    }
    s.groups.values().any(|g| g.pinned && g.window_id == w.id && g.profile_id == w.profile_id)
}

/// `closesWindow` (`model.ts:122`).
pub fn closes_window(s: &BrowserState, id: &str) -> bool {
    let Some(tab) = s.tabs.get(id) else { return false };
    let Some(w) = s.windows.get(&tab.window_id) else { return false };
    if tab.profile_id != w.profile_id || in_pinned_container(&s.tabs, &s.groups, id) {
        return false;
    }
    view_tab_ids(s, &w.id, None).iter().all(|t| t == id || s.tabs[t].page.kind() == "asleep")
}

/// `showsEmptyWindow` (`model.ts:129`).
pub fn shows_empty_window(s: &BrowserState, window_id: &str) -> bool {
    let active = active_tab_id(s, window_id, None);
    if active.as_ref().is_some_and(|a| !s.tabs[a].url.is_empty()) {
        return false;
    }
    view_tab_ids(s, window_id, None)
        .iter()
        .all(|id| Some(id) == active.as_ref() || s.tabs[id].url.is_empty() || s.tabs[id].page.kind() == "asleep")
}

/// `activeTabId` (`model.ts:135`).
pub fn active_tab_id(s: &BrowserState, window_id: &str, profile_id: Option<&str>) -> Option<String> {
    let w = s.windows.get(window_id)?;
    let id = w.active_tab_ids.get(profile_id.unwrap_or(&w.profile_id))?;
    (s.tabs.get(id)?.window_id == window_id).then(|| id.clone())
}

/// `activeTab` (`model.ts:141`).
pub fn active_tab<'a>(s: &'a BrowserState, window_id: &str) -> Option<&'a Tab> {
    s.tabs.get(&active_tab_id(s, window_id, None)?)
}

/// `profileFor` (`model.ts:146`).
pub fn profile_for(s: &BrowserState, profile_id: &str) -> Profile {
    if is_incognito_profile(profile_id) {
        return incognito_profile();
    }
    s.profiles.get(profile_id).or_else(|| s.profiles.get(&s.settings.default_profile_id)).cloned().unwrap_or_else(incognito_profile)
}

/// `bookmarkProfileId` (`model.ts:150`).
pub fn bookmark_profile_id(s: &BrowserState, window: Option<&BrowserWindow>) -> String {
    let id = window.map(|w| if w.kind == WindowKind::Private { incognito_original(&w.profile_id) } else { w.profile_id.clone() });
    match id {
        Some(id) if s.profiles.contains_key(&id) => id,
        _ => s.settings.default_profile_id.clone(),
    }
}

/// `resolveWindowId` (`model.ts:155`).
pub fn resolve_window_id(s: &BrowserState, id: Option<&str>) -> Option<String> {
    if let Some(id) = id.filter(|id| s.windows.contains_key(*id)) {
        return Some(id.to_string());
    }
    let main = |w: &&String| s.windows.get(*w).is_some_and(|w| w.kind != WindowKind::Small);
    s.ui.focus_order.iter().find(main).or_else(|| s.window_order.iter().find(main)).cloned()
}

fn layout_of(l: crate::settings::SettingsTabLayout) -> TabLayout {
    match l {
        crate::settings::SettingsTabLayout::Sidebar => TabLayout::Sidebar,
        crate::settings::SettingsTabLayout::Top => TabLayout::Top,
    }
}

/// `tabLayoutOf` (`model.ts:161`).
pub fn tab_layout_of(s: &BrowserState, window_id: Option<&str>) -> TabLayout {
    let id = resolve_window_id(s, window_id);
    id.and_then(|id| s.windows[&id].tab_layout).unwrap_or_else(|| layout_of(s.settings.tab_layout))
}

/// `autoHidesTabsIn` (`model.ts:166`).
pub fn auto_hides_tabs_in(s: &BrowserState, window_id: Option<&str>) -> bool {
    window_id.and_then(|w| s.windows.get(w)).and_then(|w| w.auto_hide_tabs).unwrap_or(s.settings.auto_hide_tabs)
}

/// `tabsShownIn` (`model.ts:170`).
pub fn tabs_shown_in(s: &BrowserState, window_id: Option<&str>) -> bool {
    !auto_hides_tabs_in(s, window_id) && !window_id.and_then(|w| s.windows.get(w)).is_some_and(|w| w.kind == WindowKind::Small)
}

/// `sidebarSideOf` (`model.ts:177`).
pub fn sidebar_side_of(s: &BrowserState, window_id: Option<&str>) -> SidebarSide {
    if resolve_window_id(s, window_id).is_some_and(|id| s.windows[&id].kind == WindowKind::Small) {
        return SidebarSide::Left;
    }
    if tab_layout_of(s, window_id) == TabLayout::Sidebar {
        match s.settings.sidebar_side {
            crate::settings::SettingsSidebarSide::Left => SidebarSide::Left,
            crate::settings::SettingsSidebarSide::Right => SidebarSide::Right,
        }
    } else {
        SidebarSide::Left
    }
}

/// `without` (`model.ts:190`).
pub fn without<T: Clone>(map: &HashMap<String, T>, ids: &[String]) -> HashMap<String, T> {
    let mut next = map.clone();
    for id in ids {
        next.remove(id);
    }
    next
}

/// `tabLabel` (`model.ts:196`): `title` and `custom_title` are the strings of the tab, `url` is its URL.
pub fn tab_label(url: &str, title: &str, custom_title: Option<&str>) -> String {
    if let Some(custom) = custom_title.filter(|c| !c.is_empty()) {
        return custom.to_string();
    }
    if url.is_empty() {
        return "New Tab".into();
    }
    if let Ok(Some(app)) = ria_core::url::app_urls::app_url_origin(url) {
        return app;
    }
    let fallback = if title.is_empty() { url } else { title };
    let Ok(u) = url::Url::parse(url) else { return fallback.to_string() };
    if u.scheme() == "file" {
        if !title.is_empty() {
            return title.to_string();
        }
        let last = u.path().rsplit('/').next().unwrap_or("");
        return percent_decode(last);
    }
    let host = ria_core::url::idn::display_host(u.host_str().unwrap_or(""));
    let host = host.strip_prefix("www.").unwrap_or(&host);
    if host.is_empty() { fallback.to_string() } else { host.to_string() }
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes.get(i + 1..i + 3).filter(|_| bytes[i] == b'%').and_then(|h| std::str::from_utf8(h).ok()).and_then(|h| u8::from_str_radix(h, 16).ok());
        if let Some(b) = hex {
            out.push(b);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn url_label(url: &str) -> String {
    let rest = match url.find("://") {
        Some(at) if at > 0 && url[..at].chars().next().is_some_and(|c| c.is_ascii_alphabetic()) && url[..at].chars().all(|c| c.is_ascii_alphanumeric() || "_+.-".contains(c)) => &url[at + 3..],
        _ => url,
    };
    let rest = rest.strip_prefix("www.").unwrap_or(rest);
    rest.strip_suffix('/').unwrap_or(rest).to_string()
}

/// `tabTitle` (`model.ts:216`).
pub fn tab_title(url: &str, title: &str, custom_title: Option<&str>) -> String {
    let own = single_line(custom_title.filter(|c| !c.is_empty()).unwrap_or(title));
    if !own.is_empty() {
        return own;
    }
    if url.is_empty() { "New Tab".into() } else { url_label(url) }
}

/// `tabTooltip` (`model.ts:218`).
pub fn tab_tooltip(url: &str, title: &str, custom_title: Option<&str>) -> String {
    if url.is_empty() { tab_title(url, title, custom_title) } else { format!("{}\n{url}", tab_title(url, title, custom_title)) }
}

/// `plural` (`model.ts:227`).
pub fn plural(n: usize, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

/// `windowTitle` (`model.ts:231`).
pub fn window_title(s: &BrowserState, window_id: &str) -> String {
    let Some(w) = s.windows.get(window_id) else { return String::new() };
    let label = active_tab(s, window_id).map_or_else(|| "New Tab".into(), |t| tab_label(&t.url, &t.title, t.custom_title.as_deref()));
    let others = view_tab_ids(s, window_id, None).len() as i64 - 1;
    let more = if others > 0 { format!(" & {}", plural(others as usize, "Tab")) } else { String::new() };
    format!("{} — {label}{more}", profile_for(s, &w.profile_id).name)
}

/// `groupIndex` (`structure.ts:19`): the first top-level group of each tab.
pub fn group_index(groups: &HashMap<String, TabGroup>) -> HashMap<&str, &TabGroup> {
    let mut index = HashMap::new();
    for g in groups.values().filter(|g| g.parent_id.is_none()) {
        for id in &g.tab_ids {
            index.entry(id.as_str()).or_insert(g);
        }
    }
    index
}

/// `splitIndex` (`structure.ts:31`): the splits that hold each tab.
pub fn split_index(splits: &HashMap<String, SplitView>) -> HashMap<String, Vec<&SplitView>> {
    let mut index: HashMap<String, Vec<&SplitView>> = HashMap::new();
    for v in splits.values() {
        for id in split_tab_ids(v) {
            let list = index.entry(id).or_default();
            if !list.contains(&v) {
                list.push(v);
            }
        }
    }
    index
}

/// `stableList` (`structure.ts:46`): Ardia returns the old array when it equals the new one; Rust returns the list.
pub fn stable_list(_previous: Option<Vec<String>>, next: Vec<String>) -> Vec<String> {
    next
}

/// `changedIds` (`changes.ts:17`): the ids whose value differs in `next` and `prev`, then the ids that `next` lacks.
pub fn changed_ids<T: PartialEq>(next: &HashMap<String, T>, prev: &HashMap<String, T>) -> Vec<String> {
    let mut ids: Vec<String> = next.iter().filter(|(id, v)| prev.get(*id) != Some(*v)).map(|(id, _)| id.clone()).collect();
    ids.extend(prev.keys().filter(|id| !next.contains_key(*id)).cloned());
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(id: &str, window: &str, profile: &str) -> Tab {
        Tab { id: id.into(), window_id: window.into(), profile_id: profile.into(), ..Default::default() }
    }

    fn state() -> BrowserState {
        let mut s = BrowserState::default();
        s.windows.insert(
            "w".into(),
            BrowserWindow { id: "w".into(), profile_id: "p".into(), tab_ids: vec!["a".into(), "b".into()], active_tab_ids: [("p".to_string(), "a".to_string())].into(), ..Default::default() },
        );
        s.tabs.insert("a".into(), tab("a", "w", "p"));
        s.tabs.insert("b".into(), tab("b", "w", "p"));
        s
    }

    #[test]
    fn incognito_ids_round_trip() {
        let id = incognito_profile_id("w1", "work");
        assert!(is_incognito_profile(&id));
        assert_eq!(incognito_original(&id), "work");
        assert_eq!(incognito_original("incognito:w1"), "default");
        let mut profiles = HashMap::new();
        profiles.insert("work".to_string(), Profile { id: "work".into(), data_id: Some("d1".into()), ..Default::default() });
        assert_eq!(engine_profile(&profiles, &id), "incognito:w1@d1");
        assert_eq!(engine_profile(&profiles, "default"), "");
    }

    #[test]
    fn labels_follow_the_url_rules() {
        assert_eq!(tab_label("", "", None), "New Tab");
        assert_eq!(tab_label("https://www.example.com/x", "T", None), "example.com");
        assert_eq!(tab_label("not a url", "", None), "not a url");
        assert_eq!(tab_label("file:///a/b%20c.txt", "", None), "b c.txt");
        assert_eq!(tab_title("https://www.example.com/", "", None), "example.com");
        assert_eq!(tab_tooltip("https://a.b/", "Té", None), "Té\nhttps://a.b/");
        assert_eq!(plural(1, "Tab"), "1 Tab");
        assert_eq!(plural(2, "Tab"), "2 Tabs");
    }

    #[test]
    fn window_lookups_use_the_active_profile() {
        let s = state();
        assert_eq!(active_tab_id(&s, "w", None).as_deref(), Some("a"));
        assert_eq!(view_tab_ids(&s, "w", None), vec!["a", "b"]);
        assert!(shows_empty_window(&s, "w"));
        assert_eq!(window_title(&s, "w"), "Incognito — New Tab & 1 Tab");
    }

    #[test]
    fn changed_ids_lists_changed_added_and_removed() {
        let prev: HashMap<String, i32> = [("a".into(), 1), ("b".into(), 2)].into();
        let next: HashMap<String, i32> = [("a".into(), 1), ("b".into(), 3), ("c".into(), 4)].into();
        let mut ids = changed_ids(&next, &prev);
        ids.sort();
        assert_eq!(ids, vec!["b", "c"]);
        assert_eq!(changed_ids(&prev, &next).len(), 2);
    }

    #[test]
    fn make_tab_marks_restored_only_with_a_nonzero_last_active() {
        let snap = TabSnapshot { last_active_at: Some(Number(5.0)), ..Default::default() };
        let t = make_tab("t".into(), "w", "p", "https://a.b/", Some(&snap), 9.0);
        assert_eq!(t.restored_at, Some(Number(9.0)));
        assert_eq!(t.last_active_at, Number(5.0));
        assert!(matches!(t.page, TabPage::Loaded { .. }));
        let zero = TabSnapshot { last_active_at: Some(Number(0.0)), ..Default::default() };
        assert_eq!(make_tab("t".into(), "w", "p", "", Some(&zero), 9.0).restored_at, None);
    }
}
