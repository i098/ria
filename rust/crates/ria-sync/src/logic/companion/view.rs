// Ported from ardia packages/sync/src/companion/view.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The view of the phone app: devices, profiles with their tabs, pins, bookmarks and history,
//! search and look, built from the synced scopes. `serde_json::to_value(&view)` written by
//! `js::stringify` is the text of `JSON.stringify(view)`.
//!
//! Changes from Ardia:
//! - Record values are JSON values; `undefined` is `None`. A record timestamp is always a
//!   string, so the `typeof stamp !== "string"` check of `spaceOrder` never skips.
//! - `sendTargets` asks `seen` for its own keys; Ardia's `in` also finds the names of
//!   `Object.prototype` (`toString`, ...), which no device id is.
//! - The sort comparators are total orders: a tie of the last fields keeps the input order
//!   (Ardia returns 1, which V8's stable sort also treats as "keep" for arrays shorter than
//!   64 items), and a `NaN` time sorts by `f64::total_cmp` (Ardia's comparator is not
//!   consistent there).
//! - `localeCompare` is an approximation of the ICU root (`en-US`) collation: ASCII control
//!   characters are ignored, white space and punctuation sort before digits and digits before
//!   letters in ICU's ASCII order, letters compare without case first, and then lowercase sorts
//!   before uppercase. Characters past ASCII sort after all ASCII by their lowercase code point:
//!   ICU orders accented and non-Latin letters differently (for example `é` next to `e`).
//! - `bookmarkTree` builds each folder with recursion to depth 100, not with a stack of
//!   shared folder objects; the tree is the same.

use super::super::folders::folder_children;
use super::super::js::cmp_utf16;
use super::super::scope_keys::visible;
use super::super::scope_state::ScopeState;
use super::adapters::last_visit_of;
use super::legacy::{from_legacy, from_legacy_url};
use super::theme::{ViewTheme, theme_of, truthy};
use indexmap::{IndexMap, IndexSet};
use serde::Serialize;
use serde_json::Value;
use std::cmp::Ordering;

pub use super::theme::{ViewThemeLook, ViewThemeStop, ViewThemeTexture};

/// Port of `MAX_HISTORY_ITEMS` (`packages/sync/src/companion/view.ts:10`).
pub const MAX_HISTORY_ITEMS: usize = 2_000;
const MAX_BOOKMARK_DEPTH: usize = 100;

/// Port of `Kind` (`packages/sync/src/companion/view.ts:13`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Mac,
    Phone,
}

/// Port of `ViewDevice` (`packages/sync/src/companion/view.ts:16`).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewDevice {
    pub id: String,
    pub name: String,
    pub kind: Kind,
    pub last_seen: Option<f64>,
    pub is_current: bool,
    pub can_receive_tabs: bool,
}

/// One tab of `ViewDeviceTabs`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ViewTab {
    pub url: String,
    pub title: String,
}

/// Port of `ViewDeviceTabs` (`packages/sync/src/companion/view.ts:17`).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewDeviceTabs {
    pub device_id: String,
    pub device_name: String,
    pub kind: Kind,
    pub tabs: Vec<ViewTab>,
}

/// Port of `ViewPinnedTab` (`packages/sync/src/companion/view.ts:18`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ViewPinnedTab {
    pub key: String,
    pub title: String,
    pub url: String,
    pub icon: Option<String>,
}

/// Port of `ViewPinnedTile` (`packages/sync/src/companion/view.ts:19`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ViewPinnedTile {
    pub key: String,
    pub title: String,
    pub url: String,
    pub icon: Option<String>,
    pub folder: Option<Vec<ViewPinnedTab>>,
}

/// Port of `ViewPinnedSection` (`packages/sync/src/companion/view.ts:20`).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewPinnedSection {
    Tab(ViewPinnedTab),
    Group(ViewPinnedGroup),
}

/// Port of `ViewPinnedGroup` (`packages/sync/src/companion/view.ts:21`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ViewPinnedGroup {
    pub key: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub items: Vec<ViewPinnedSection>,
}

/// The `pinned` field of `ViewProfile`.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ViewPinned {
    pub tiles: Vec<ViewPinnedTile>,
    pub sections: Vec<ViewPinnedSection>,
}

/// The `root` of a `ViewBookmark`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BookmarkRoot {
    Bar,
    Other,
}

/// Port of `ViewBookmark` (`packages/sync/src/companion/view.ts:22`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ViewBookmark {
    pub key: String,
    pub title: String,
    pub url: Option<String>,
    pub added: f64,
    pub root: Option<BookmarkRoot>,
    pub children: Option<Vec<ViewBookmark>>,
}

/// Port of `ViewHistory` (`packages/sync/src/companion/view.ts:23`).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewHistory {
    pub url: String,
    pub title: String,
    pub visits: f64,
    pub last_visit: f64,
}

/// Port of `ViewProfile` (`packages/sync/src/companion/view.ts:24`).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewProfile {
    pub sync_id: String,
    pub name: String,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub is_default: bool,
    pub theme: Option<ViewTheme>,
    pub send_targets: Vec<String>,
    pub device_tabs: Vec<ViewDeviceTabs>,
    pub pinned: ViewPinned,
    pub bookmarks: Vec<ViewBookmark>,
    pub history: Vec<ViewHistory>,
}

/// One custom engine of `ViewSearch`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ViewCustomEngine {
    pub id: String,
    pub name: String,
    pub keyword: String,
    pub url: String,
}

/// Port of `ViewSearch` (`packages/sync/src/companion/view.ts:37`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ViewSearch {
    #[serde(rename = "engineId")]
    pub engine_id: String,
    #[serde(rename = "customURL")]
    pub custom_url: Option<String>,
    #[serde(rename = "customEngines")]
    pub custom_engines: Vec<ViewCustomEngine>,
}

/// Port of `ViewLook` (`packages/sync/src/companion/view.ts:38`).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewLook {
    pub interface_style: Option<&'static str>,
    pub appearance: Option<&'static str>,
}

/// Port of `View` (`packages/sync/src/companion/view.ts:39`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct View {
    pub devices: Vec<ViewDevice>,
    pub profiles: Vec<ViewProfile>,
    pub search: Option<ViewSearch>,
    pub look: Option<ViewLook>,
}

/// Port of `str` (`packages/sync/src/companion/view.ts:41`) with the fallback `""`.
fn str_of(v: Option<&Value>) -> String {
    v.and_then(Value::as_str).unwrap_or_default().to_owned()
}

/// Port of `optStr` (`packages/sync/src/companion/view.ts:42`).
fn opt_str(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// Port of `kindOf` (`packages/sync/src/companion/view.ts:43`).
fn kind_of(v: Option<&Value>) -> Kind {
    if v.and_then(Value::as_str) == Some("phone") {
        Kind::Phone
    } else {
        Kind::Mac
    }
}

/// A slot that `byPosThenKey` sorts.
#[derive(Clone)]
struct Slot {
    key: String,
    pos: String,
    group: bool,
}

/// Port of `byPosThenKey` (`packages/sync/src/companion/view.ts:44`).
fn by_pos_then_key(a: &Slot, z: &Slot) -> Ordering {
    cmp_utf16(&a.pos, &z.pos).then_with(|| cmp_utf16(&a.key, &z.key))
}

/// `z - a` as an order: a later time first; `NaN` by `total_cmp`, so the order stays total.
fn later_first(a: f64, z: f64) -> Ordering {
    z.partial_cmp(&a).unwrap_or_else(|| z.total_cmp(&a))
}

/// An object or an array: `v && typeof v === "object"`.
fn is_object(v: &Value) -> bool {
    v.is_object() || v.is_array()
}

/// The smallest string in UTF-16 order: `[...].sort()[0]`.
fn first_sorted(items: &[String]) -> String {
    items
        .iter()
        .min_by(|a, z| cmp_utf16(a, z))
        .cloned()
        .unwrap_or_default()
}

/// Port of `liveProfiles` (`packages/sync/src/companion/view.ts:46`).
pub fn live_profiles(app: Option<&ScopeState>) -> IndexMap<String, Value> {
    let Some(app) = app else {
        return IndexMap::new();
    };
    visible(app, "prof:")
        .into_iter()
        .filter(|(_, v)| is_object(v))
        .map(|(k, v)| (k[5..].to_owned(), v))
        .collect()
}

/// Port of `defaultProfileId` (`packages/sync/src/companion/view.ts:53`).
pub fn default_profile_id(profiles: &IndexMap<String, Value>) -> Option<String> {
    let mut ids: Vec<&String> = profiles.keys().collect();
    ids.sort_by(|a, z| cmp_utf16(a, z));
    ids.iter()
        .find(|id| profiles[id.as_str()].get("d") == Some(&Value::Bool(true)))
        .or(ids.first())
        .map(|id| (*id).clone())
}

/// Port of `dataOwner` (`packages/sync/src/companion/view.ts:58`).
pub fn data_owner(app: Option<&ScopeState>, sync_id: &str) -> String {
    let Some(app) = app else {
        return sync_id.to_owned();
    };
    let profiles = live_profiles(Some(app));
    let mut path = vec![sync_id.to_owned()];
    let mut at = sync_id.to_owned();
    loop {
        let owner = app
            .records
            .get(&format!("share:{at}"))
            .and_then(|r| r.v.get("o"))
            .and_then(Value::as_str);
        let Some(owner) = owner.filter(|o| *o != at && profiles.contains_key(*o)) else {
            return at;
        };
        if let Some(loop_at) = path.iter().position(|p| p == owner) {
            return first_sorted(&path[loop_at..]);
        }
        path.push(owner.to_owned());
        at = owner.to_owned();
    }
}

/// Port of `deviceTabs` (`packages/sync/src/companion/view.ts:72`).
fn device_tabs(
    scope: Option<&ScopeState>,
    me: &str,
    devices: &IndexMap<String, ViewDevice>,
) -> Vec<ViewDeviceTabs> {
    let Some(scope) = scope else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (k, value) in visible(scope, "tabs:") {
        let device_id = &k[5..];
        let Some(device) = devices.get(device_id) else {
            continue;
        };
        let Some(list) = value.get("tabs").and_then(Value::as_array) else {
            continue;
        };
        if device_id == me {
            continue;
        }
        let mut seen = IndexSet::new();
        let mut tabs = Vec::new();
        for t in list {
            let url = from_legacy_url(&str_of(t.get("u")));
            let title = str_of(t.get("t"));
            if url.is_empty() || !seen.insert(format!("{url}\n{title}")) {
                continue;
            }
            tabs.push(ViewTab { url, title });
        }
        if tabs.is_empty() {
            continue;
        }
        let device_name = [device.name.clone(), str_of(value.get("n"))]
            .into_iter()
            .find(|n| !n.is_empty())
            .unwrap_or_else(|| "Unknown Device".into());
        let kind = if truthy(value.get("k")) {
            kind_of(value.get("k"))
        } else {
            device.kind
        };
        out.push(ViewDeviceTabs {
            device_id: device_id.to_owned(),
            device_name,
            kind,
            tabs,
        });
    }
    let seen_at = |id: &str| devices.get(id).and_then(|d| d.last_seen).unwrap_or(0.0);
    out.sort_by(|a, z| {
        later_first(seen_at(&a.device_id), seen_at(&z.device_id))
            .then_with(|| cmp_utf16(&a.device_id, &z.device_id))
    });
    out
}

/// Port of `sendTargets` (`packages/sync/src/companion/view.ts:95`).
fn send_targets(
    scope: Option<&ScopeState>,
    me: &str,
    devices: &IndexMap<String, ViewDevice>,
) -> Vec<String> {
    let Some(scope) = scope else {
        return Vec::new();
    };
    let mut out: Vec<(f64, &String)> = devices
        .values()
        .filter(|d| d.id != me && d.can_receive_tabs)
        .filter_map(|d| scope.seen.get(&d.id).map(|at| (*at, &d.id)))
        .collect();
    out.sort_by(|(a, a_id), (z, z_id)| later_first(*a, *z).then_with(|| cmp_utf16(a_id, z_id)));
    out.into_iter().map(|(_, id)| id.clone()).collect()
}

/// Port of `pinnedArea` (`packages/sync/src/companion/view.ts:106`).
pub fn pinned_area(scope: Option<&ScopeState>) -> ViewPinned {
    let Some(scope) = scope else {
        return ViewPinned::default();
    };
    let records = |prefix: &str| visible(scope, prefix);
    let objects = |prefix: &str| -> IndexMap<String, Value> {
        records(prefix)
            .into_iter()
            .filter(|(_, v)| is_object(v))
            .map(|(k, v)| (k[prefix.len()..].to_owned(), v))
            .collect()
    };
    let pins: IndexMap<String, Value> = objects("pin:t:")
        .into_iter()
        .map(|(k, v)| (k, from_legacy(&v)))
        .collect();
    let group_values = objects("pin:g:");
    let nest_values = objects("pinn:g:");
    let rows: IndexSet<String> = records("pinrow:")
        .into_iter()
        .filter(|(_, v)| v.as_f64() == Some(1.0))
        .map(|(k, _)| k[7..].to_owned())
        .collect();
    let order = records("pord:g:");
    let all: IndexSet<String> = group_values
        .keys()
        .chain(nest_values.keys())
        .cloned()
        .collect();
    let mut parent: IndexMap<String, Option<String>> = all
        .iter()
        .map(|id| {
            let p = nest_values
                .get(id)
                .and_then(|v| v.get("p"))
                .and_then(Value::as_str);
            (
                id.clone(),
                p.filter(|p| p != id && all.contains(*p)).map(str::to_owned),
            )
        })
        .collect();
    // `parent.get(at) ?? null` read as a loop condition: `""` and `null` both stop.
    let up = |parent: &IndexMap<String, Option<String>>, at: &str| {
        parent.get(at).cloned().flatten().filter(|p| !p.is_empty())
    };
    let mut starts: Vec<&String> = all.iter().collect();
    starts.sort_by(|a, z| cmp_utf16(a, z));
    for start in starts {
        let mut path: Vec<String> = Vec::new();
        let mut at = Some(start.clone()).filter(|s| !s.is_empty());
        while let Some(here) = at {
            if let Some(loop_at) = path.iter().position(|p| *p == here) {
                parent.insert(first_sorted(&path[loop_at..]), None);
                break;
            }
            at = up(&parent, here.as_str());
            path.push(here);
        }
    }
    let chain_up = |id: &str| -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut g = Some(id.to_owned()).filter(|g| !g.is_empty());
        while let Some(here) = g {
            if out.contains(&here) {
                break;
            }
            g = up(&parent, here.as_str());
            out.push(here);
        }
        out
    };
    let mut group_of: IndexMap<String, Option<String>> = pins
        .iter()
        .map(|(key, v)| {
            (
                key.clone(),
                v.get("g")
                    .and_then(Value::as_str)
                    .filter(|g| all.contains(*g))
                    .map(str::to_owned),
            )
        })
        .collect();
    for (k, v) in records("pinn:t:") {
        let key = &k[7..];
        let Some(inner) = v.get("g").and_then(Value::as_str) else {
            continue;
        };
        let Some(g) = group_of
            .get(key)
            .cloned()
            .flatten()
            .filter(|g| !g.is_empty())
        else {
            continue;
        };
        if !all.contains(inner) {
            continue;
        }
        if chain_up(inner).last() == chain_up(g.as_str()).last() {
            group_of.insert(key.to_owned(), Some(inner.to_owned()));
        }
    }
    let tab_of = |key: &str| -> ViewPinnedTab {
        let v = pins.get(key);
        let field = |f: &str| v.and_then(|v| v.get(f));
        ViewPinnedTab {
            key: key.to_owned(),
            title: opt_str(field("t")).unwrap_or_else(|| str_of(field("ti"))),
            url: str_of(field("u")),
            icon: opt_str(field("i")),
        }
    };
    let pos = |v: Option<&Value>, key: &str| {
        v.and_then(|v| v.get("pos"))
            .and_then(Value::as_str)
            .map_or_else(|| format!("\u{FFFF}{key}"), str::to_owned)
    };
    let grouped = |k: &str| group_of.get(k).cloned().flatten();
    let mut area_slots: Vec<Slot> = pins
        .keys()
        .filter(|k| group_of.get(k.as_str()).is_some_and(Option::is_none))
        .map(|key| Slot {
            key: key.clone(),
            pos: pos(pins.get(key), key.as_str()),
            group: false,
        })
        .chain(
            all.iter()
                .filter(|id| up(&parent, id.as_str()).is_none())
                .map(|key| Slot {
                    key: key.clone(),
                    pos: pos(group_values.get(key), key.as_str()),
                    group: true,
                }),
        )
        .collect();
    area_slots.sort_by(by_pos_then_key);

    let mut kids: IndexMap<String, Vec<(bool, String)>> = IndexMap::new();
    let mut placed: IndexSet<String> = IndexSet::new();
    let mut members: Vec<(Slot, Vec<String>)> = pins
        .keys()
        .filter_map(|key| {
            let g = grouped(key.as_str()).filter(|g| !g.is_empty())?;
            let mut chain = chain_up(g.as_str());
            chain.reverse();
            Some((
                Slot {
                    key: key.clone(),
                    pos: pos(pins.get(key), key.as_str()),
                    group: false,
                },
                chain,
            ))
        })
        .collect();
    members.sort_by(|(a, _), (z, _)| by_pos_then_key(a, z));
    for (m, chain) in &members {
        for (i, g) in chain.iter().enumerate() {
            if i == 0 || !placed.insert(g.clone()) {
                continue;
            }
            kids.entry(chain[i - 1].clone())
                .or_default()
                .push((true, g.clone()));
        }
        if let Some(leaf) = chain.last() {
            kids.entry(leaf.clone())
                .or_default()
                .push((false, m.key.clone()));
        }
    }
    let group_look = |id: &str| -> Option<Value> {
        let look = if up(&parent, id).is_some() {
            nest_values.get(id).or(group_values.get(id))
        } else {
            group_values.get(id).or(nest_values.get(id))
        };
        look.cloned()
    };
    fn group_view(
        id: &str,
        group_look: &dyn Fn(&str) -> Option<Value>,
        kids: &IndexMap<String, Vec<(bool, String)>>,
        tab_of: &dyn Fn(&str) -> ViewPinnedTab,
    ) -> ViewPinnedGroup {
        let look = group_look(id);
        let field = |f: &str| look.as_ref().and_then(|v| v.get(f));
        ViewPinnedGroup {
            key: id.to_owned(),
            name: str_of(field("n")),
            icon: opt_str(field("i")),
            color: opt_str(field("c")),
            items: kids
                .get(id)
                .into_iter()
                .flatten()
                .map(|(group, key)| {
                    if *group {
                        ViewPinnedSection::Group(group_view(key, group_look, kids, tab_of))
                    } else {
                        ViewPinnedSection::Tab(tab_of(key.as_str()))
                    }
                })
                .collect(),
        }
    }

    let mut folder_of: IndexMap<String, String> = IndexMap::new();
    for (k, v) in records("tilefold:") {
        if k.starts_with("tilefold:n:") {
            continue;
        }
        if let Some(f) = v.get("f").and_then(Value::as_str).filter(|f| !f.is_empty()) {
            folder_of.insert(k[9..].to_owned(), f.to_owned());
        }
    }
    let folder_name = |id: &str| {
        str_of(
            scope
                .records
                .get(&format!("tilefold:n:{id}"))
                .and_then(|r| r.v.get("n")),
        )
    };
    let tile_keys: Vec<&String> = area_slots
        .iter()
        .filter(|s| !s.group && !rows.contains(&s.key))
        .map(|s| &s.key)
        .collect();
    let mut folder_members: IndexMap<&String, Vec<&String>> = IndexMap::new();
    for key in &tile_keys {
        if let Some(f) = folder_of.get(*key) {
            folder_members.entry(f).or_default().push(*key);
        }
    }
    let mut tiles = Vec::new();
    let mut emitted: IndexSet<&String> = IndexSet::new();
    for key in &tile_keys {
        let f = folder_of.get(*key);
        let in_folder = f
            .and_then(|f| folder_members.get(&f))
            .map_or(&[][..], Vec::as_slice);
        let Some(f) = f.filter(|_| in_folder.len() >= 2) else {
            let t = tab_of(key.as_str());
            tiles.push(ViewPinnedTile {
                key: t.key,
                title: t.title,
                url: t.url,
                icon: t.icon,
                folder: None,
            });
            continue;
        };
        if !emitted.insert(f) {
            continue;
        }
        let tabs: Vec<ViewPinnedTab> = in_folder.iter().map(|k| tab_of(k.as_str())).collect();
        let name = folder_name(f.as_str());
        tiles.push(ViewPinnedTile {
            key: format!("tilefold:{f}"),
            title: if name.is_empty() {
                "Folder".into()
            } else {
                name
            },
            url: tabs[0].url.clone(),
            icon: None,
            folder: Some(tabs),
        });
    }

    let blocks: Vec<&Slot> = area_slots
        .iter()
        .filter(|s| {
            if s.group {
                kids.get(&s.key).is_some_and(|k| !k.is_empty())
            } else {
                rows.contains(&s.key)
            }
        })
        .collect();
    let row_keys: IndexSet<&String> = blocks.iter().filter(|b| !b.group).map(|b| &b.key).collect();
    // `undefined` (no record) is `None`; a record without a string `a` is `Some(None)`.
    let anchor_of = |group: &str| -> Option<Option<String>> {
        order
            .get(&format!("pord:g:{group}"))
            .map(|v| v.get("a").and_then(Value::as_str).map(str::to_owned))
    };
    let mut ordered = blocks.clone();
    if blocks
        .iter()
        .any(|b| b.group && anchor_of(b.key.as_str()).is_some())
    {
        let mut placed_after: IndexMap<Option<String>, Vec<&Slot>> = IndexMap::new();
        let mut last = Vec::new();
        for g in blocks.iter().filter(|b| b.group) {
            match anchor_of(g.key.as_str()) {
                Some(a) if a.as_ref().is_none_or(|a| row_keys.contains(a)) => {
                    placed_after.entry(a).or_default().push(*g)
                }
                _ => last.push(*g),
            }
        }
        let after = |a: Option<String>| placed_after.get(&a).cloned().unwrap_or_default();
        ordered = after(None);
        for r in blocks.iter().filter(|b| !b.group) {
            ordered.push(*r);
            ordered.extend(after(Some(r.key.clone())));
        }
        ordered.extend(last);
    }
    let sections = ordered
        .iter()
        .map(|b| {
            if b.group {
                ViewPinnedSection::Group(group_view(&b.key, &group_look, &kids, &tab_of))
            } else {
                ViewPinnedSection::Tab(tab_of(b.key.as_str()))
            }
        })
        .collect();
    ViewPinned { tiles, sections }
}

/// Port of `bookmarkTree` (`packages/sync/src/companion/view.ts:229`).
pub fn bookmark_tree(scope: Option<&ScopeState>) -> Vec<ViewBookmark> {
    let Some(scope) = scope else {
        return Vec::new();
    };
    let mut values: IndexMap<String, Value> = IndexMap::new();
    for (k, b) in visible(scope, "bm:") {
        let kind = b.get("k").and_then(Value::as_str);
        if k != "bm:bar" && k != "bm:other" && is_object(&b) && matches!(kind, Some("u" | "f")) {
            values.insert(k[3..].to_owned(), from_legacy(&b));
        }
    }
    let is_folder =
        |v: Option<&Value>| v.and_then(|v| v.get("k")).and_then(Value::as_str) == Some("f");
    let mut parent_of: IndexMap<String, String> = values
        .iter()
        .map(|(key, v)| {
            let p = v.get("p").and_then(Value::as_str).unwrap_or("other");
            let p = if p == "bar" || p == "other" || is_folder(values.get(p)) {
                p
            } else {
                "other"
            };
            (key.clone(), p.to_owned())
        })
        .collect();
    let mut keys: Vec<String> = values.keys().cloned().collect();
    keys.sort_by(|a, z| cmp_utf16(a, z));
    let children = folder_children(&mut parent_of, &keys, "other");
    let sorted = |parent: &str| -> Vec<String> {
        let mut slots: Vec<Slot> = children
            .get(parent)
            .into_iter()
            .flatten()
            .map(|key| Slot {
                key: key.clone(),
                pos: str_of(values.get(key).and_then(|v| v.get("pos"))),
                group: false,
            })
            .collect();
        slots.sort_by(by_pos_then_key);
        slots.into_iter().map(|s| s.key).collect()
    };
    let node = |key: &str, root: Option<BookmarkRoot>| -> ViewBookmark {
        let v = values.get(key);
        let field = |f: &str| v.and_then(|v| v.get(f));
        let folder = is_folder(v);
        ViewBookmark {
            key: key.to_owned(),
            title: str_of(field("t")),
            url: if folder {
                None
            } else {
                Some(str_of(field("u")))
            },
            added: field("a")
                .and_then(Value::as_f64)
                .filter(|a| a.is_finite())
                .unwrap_or(0.0),
            root,
            children: folder.then(Vec::new),
        }
    };
    fn fill(
        folder: &mut ViewBookmark,
        depth: usize,
        sorted: &dyn Fn(&str) -> Vec<String>,
        node: &dyn Fn(&str, Option<BookmarkRoot>) -> ViewBookmark,
    ) {
        let Some(children) = folder.children.as_mut() else {
            return;
        };
        if depth >= MAX_BOOKMARK_DEPTH {
            return;
        }
        for k in sorted(folder.key.as_str()) {
            let mut child = node(k.as_str(), None);
            fill(&mut child, depth + 1, sorted, node);
            children.push(child);
        }
    }
    let mut tree: Vec<ViewBookmark> = sorted("bar")
        .iter()
        .map(|k| node(k.as_str(), Some(BookmarkRoot::Bar)))
        .chain(
            sorted("other")
                .iter()
                .map(|k| node(k.as_str(), Some(BookmarkRoot::Other))),
        )
        .collect();
    for folder in &mut tree {
        fill(folder, 1, &sorted, &node);
    }
    tree
}

/// Port of `historyList` (`packages/sync/src/companion/view.ts:265`).
pub fn history_list(scope: Option<&ScopeState>) -> Vec<ViewHistory> {
    let Some(scope) = scope else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (k, h) in visible(scope, "h:") {
        let Some(vt) = h
            .get("vt")
            .and_then(Value::as_array)
            .filter(|vt| !vt.is_empty())
        else {
            continue;
        };
        let visits = h
            .get("n")
            .and_then(Value::as_f64)
            .unwrap_or(vt.len() as f64);
        out.push(ViewHistory {
            url: from_legacy_url(&k[2..]),
            title: str_of(h.get("t")),
            visits,
            last_visit: last_visit_of(Some(&h)),
        });
    }
    out.sort_by(|a, z| {
        later_first(a.last_visit, z.last_visit).then_with(|| cmp_utf16(&a.url, &z.url))
    });
    out.truncate(MAX_HISTORY_ITEMS);
    out
}

/// Port of `settingOf` (`packages/sync/src/companion/view.ts:276`).
fn setting_of(app: &ScopeState, name: &str) -> Option<Value> {
    let record = &app.records.get(&format!("set:{name}"))?.v;
    if !is_object(record) {
        return None;
    }
    record.get("v").map(from_legacy)
}

/// Port of `oneOf` (`packages/sync/src/companion/view.ts:281`).
fn one_of(value: Option<&Value>, choices: &[&'static str]) -> Option<&'static str> {
    let value = value?.as_str()?;
    choices.iter().find(|c| **c == value).copied()
}

/// Port of `lookOf` (`packages/sync/src/companion/view.ts:285`).
fn look_of(app: &ScopeState) -> Option<ViewLook> {
    let interface_style = one_of(
        setting_of(app, "interfaceStyle").as_ref(),
        &["glass", "classic"],
    );
    let appearance = one_of(
        setting_of(app, "appearance").as_ref(),
        &["auto", "light", "dark"],
    );
    (interface_style.is_some() || appearance.is_some()).then_some(ViewLook {
        interface_style,
        appearance,
    })
}

/// Port of `searchOf` (`packages/sync/src/companion/view.ts:291`).
fn search_of(app: &ScopeState) -> Option<ViewSearch> {
    let engine = setting_of(app, "searchEngine");
    let custom_url = setting_of(app, "customSearchUrl");
    let custom = setting_of(app, "customSearchEngines");
    if engine.is_none() && custom_url.is_none() && custom.is_none() {
        return None;
    }
    let field = |e: &Value, f: &str| e.get(f).and_then(Value::as_str).map(str::to_owned);
    let custom_engines = custom
        .as_ref()
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|e| {
            Some(ViewCustomEngine {
                id: field(e, "id")?,
                name: field(e, "name")?,
                keyword: field(e, "keyword")?,
                url: field(e, "url")?,
            })
        })
        .collect();
    Some(ViewSearch {
        engine_id: opt_str(engine.as_ref()).unwrap_or_else(|| "google".into()),
        custom_url: opt_str(custom_url.as_ref()),
        custom_engines,
    })
}

/// Port of `spaceOrder` (`packages/sync/src/companion/view.ts:310`).
fn space_order(app: &ScopeState, me: &str) -> Vec<String> {
    let mut newest: Option<(&str, String, Vec<String>)> = None;
    for (k, v) in visible(app, "dev:") {
        let id = &k[4..];
        let Some(stamp) = app.records.get(&k).map(|r| r.h.as_str()) else {
            continue;
        };
        let Some(listed) = v.get("o").and_then(Value::as_array) else {
            continue;
        };
        if id == me {
            continue;
        }
        let order: Vec<String> = listed
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect::<IndexSet<_>>()
            .into_iter()
            .collect();
        if order.is_empty() {
            continue;
        }
        let newer = newest
            .as_ref()
            .is_none_or(|(s, n, _)| match cmp_utf16(stamp, s) {
                Ordering::Greater => true,
                Ordering::Equal => cmp_utf16(id, n).is_lt(),
                Ordering::Less => false,
            });
        if newer {
            newest = Some((stamp, id.to_owned(), order));
        }
    }
    newest.map(|(_, _, order)| order).unwrap_or_default()
}

/// ICU's root order of the printable ASCII characters (`Intl.Collator("en-US")`); letters
/// appear once, in lowercase.
const ASCII_ORDER: &str =
    "\t\n\u{b}\u{c}\r _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$0123456789abcdefghijklmnopqrstuvwxyz";

/// The primary weight of a character, `None` for an ignorable one.
fn primary(c: char) -> Option<u32> {
    let lower = c.to_lowercase().next().unwrap_or(c);
    if lower.is_ascii() {
        return ASCII_ORDER.find(lower).map(|at| at as u32);
    }
    Some(ASCII_ORDER.len() as u32 + lower as u32)
}

/// `a.localeCompare(z)` with the default collation (Node: `en-US`, the root rules), approximated.
fn locale_compare() -> impl Fn(&str, &str) -> Ordering {
    |a, z| {
        let weights = |s: &str| {
            s.chars()
                .filter_map(|c| primary(c).map(|w| (w, c.is_uppercase())))
                .collect::<Vec<_>>()
        };
        let (a, z) = (weights(a), weights(z));
        let first = a.iter().map(|w| w.0).cmp(z.iter().map(|w| w.0));
        first.then_with(|| a.iter().map(|w| w.1).cmp(z.iter().map(|w| w.1)))
    }
}

/// Port of `buildView` (`packages/sync/src/companion/view.ts:326`).
pub fn build_view(scopes: &IndexMap<String, ScopeState>, me: &str) -> View {
    let Some(app) = scopes.get("app") else {
        return View {
            devices: Vec::new(),
            profiles: Vec::new(),
            search: None,
            look: None,
        };
    };
    let mut last_seen: IndexMap<&String, f64> = IndexMap::new();
    for scope in scopes.values() {
        for (device, at) in &scope.seen {
            let known = last_seen.get(device).copied().unwrap_or(0.0);
            last_seen.insert(device, super::super::js::js_max(*at, known));
        }
    }
    let mut devices: IndexMap<String, ViewDevice> = IndexMap::new();
    for (k, d) in visible(app, "dev:") {
        let id = k[4..].to_owned();
        let name = opt_str(d.get("n")).unwrap_or_else(|| "Unknown Device".into());
        let can_receive_tabs = d
            .get("f")
            .and_then(Value::as_array)
            .is_some_and(|f| f.iter().any(|x| x.as_str() == Some("send")));
        let device = ViewDevice {
            last_seen: last_seen.get(&id).copied(),
            is_current: id == me,
            id: id.clone(),
            name,
            kind: kind_of(d.get("k")),
            can_receive_tabs,
        };
        devices.insert(id, device);
    }
    let mut device_list: Vec<ViewDevice> = devices.values().cloned().collect();
    device_list.sort_by(|a, z| {
        z.is_current
            .cmp(&a.is_current)
            .then_with(|| later_first(a.last_seen.unwrap_or(0.0), z.last_seen.unwrap_or(0.0)))
            .then_with(|| cmp_utf16(&a.id, &z.id))
    });
    let profiles = live_profiles(Some(app));
    let default_id = default_profile_id(&profiles);
    let mut profile_list: Vec<ViewProfile> = profiles
        .iter()
        .map(|(sync_id, p)| {
            let own = scopes.get(&format!("p:{sync_id}"));
            let data = scopes.get(&format!("p:{}", data_owner(Some(app), sync_id)));
            ViewProfile {
                sync_id: sync_id.clone(),
                name: opt_str(p.get("n")).unwrap_or_else(|| "Profile".into()),
                color: opt_str(p.get("c")),
                icon: opt_str(p.get("i")),
                is_default: default_id.as_ref() == Some(sync_id),
                theme: theme_of(app, sync_id, p.get("th")),
                send_targets: send_targets(own, me, &devices),
                device_tabs: device_tabs(own, me, &devices),
                pinned: pinned_area(own),
                bookmarks: bookmark_tree(data),
                history: history_list(data),
            }
        })
        .collect();
    let rank: IndexMap<String, usize> = space_order(app, me)
        .into_iter()
        .enumerate()
        .map(|(i, id)| (id, i))
        .collect();
    let placed = |p: &ViewProfile| rank.get(&p.sync_id).copied().unwrap_or(rank.len());
    let compare_names = locale_compare();
    profile_list.sort_by(|a, z| {
        placed(a)
            .cmp(&placed(z))
            .then_with(|| z.is_default.cmp(&a.is_default))
            .then_with(|| compare_names(&a.name, &z.name))
            .then_with(|| cmp_utf16(&a.sync_id, &z.sync_id))
    });
    View {
        devices: device_list,
        profiles: profile_list,
        search: search_of(app),
        look: look_of(app),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_compare_matches_node_on_ascii() {
        // Order from `[...].sort((a, b) => a.localeCompare(b))` in Node 26 (en-US).
        let node = ["a b", "a-b", "A-b", "ab", "aB", "Ab", "AB", "ab-"];
        let mut names = ["ab", "aB", "Ab", "AB", "a-b", "ab-", "a b", "A-b"];
        let compare = locale_compare();
        names.sort_by(|a, z| compare(a, z));
        assert_eq!(names, node);
        assert_eq!(compare("a\u{1}b", "ab"), Ordering::Equal);
        assert_eq!(compare("$", "0"), Ordering::Less);
        assert_eq!(compare("9", "a"), Ordering::Less);
    }
}
