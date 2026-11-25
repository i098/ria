// Ported from ardia apps/browser/src/state/splits/splits.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The split view helpers and the store methods of `splits.ts`.
//!
//! Changes from Ardia:
//! - TypeScript functions take and return one `BrowserState`. Here a `Work` holds a copy of the 4 modules that a split
//!   action reads (`tabs`, `windows`, `groups`, `splits`). `Work::commit` writes back only the modules that differ, so
//!   a failed action changes nothing and the change counters stay exact.
//! - `Object.values(splits)` runs in insertion order in TypeScript. `splits::State.splits` is a `HashMap`, so `splits_in_order`
//!   sorts by id. The result differs only when one tab is in two splits, which `sanitize_splits` removes.
//! - `activated(s, id)` and `apply` run inside the same `set` in TypeScript. Here the caller runs `tabs::Action::Activate`
//!   after `commit`, with the same result for the state. [INFERENCE: `activated` is a sibling that is not ported yet.]
//! - `flatIds`, the `WeakMap` cache of `splitTabIds`, is not ported; the ids are computed each time.
//! - `createSplit` and `openSplitPane` return their result to the caller in Rust; `apply` drops it, as `Action` has no result.
//! - Sibling helpers that are not ported yet are in `pending_siblings.rs`.

use std::collections::{HashMap, HashSet};

use super::lanes::{crosses_lane, is_pinned_row};
use super::pending_siblings::{self as siblings, Placement};
use crate::browser::{
    BrowserWindow, SplitSlot, SplitSlotVariant0, SplitSlotVariant1, SplitView,
    SplitViewOrientation, Tab, TabGroup,
};
use crate::kernel::{AppState, Number};
use crate::shared::platform_features::PLATFORM_FEATURES;
use crate::{groups, tabs, windows};

use super::{State, UpdateSplitPatch};

pub use super::SplitSide;

#[derive(Debug, Clone, Default)]
pub struct OpenPaneOpts {
    pub tab_id: Option<String>,
    pub url: Option<String>,
    pub anchor_tab_id: Option<String>,
    pub side: Option<SplitSide>,
    pub background: bool,
    pub adopt_id: Option<String>,
    pub in_place_of: Option<String>,
    pub opened_for_split: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OpenPaneResult {
    Ok { tab_id: String, split_id: String },
    Max,
    Invalid,
    Unavailable,
}

pub const MAX_SPLIT_PANES: usize = 3;

/// `splitViewAvailable` (`splits.ts:41`).
pub const fn split_view_available() -> bool {
    PLATFORM_FEATURES.split_view
}

/// `splitsWhereUnavailable` (`splits.ts:43`): the patch that clears the splits on a platform without split view.
pub fn splits_where_unavailable(
    splits: &HashMap<String, SplitView>,
    available: bool,
) -> Option<HashMap<String, SplitView>> {
    (!available && !splits.is_empty()).then(HashMap::new)
}

pub fn pane_chunks(ids: &[String]) -> Vec<Vec<String>> {
    ids.chunks(MAX_SPLIT_PANES)
        .filter(|chunk| chunk.len() >= 2)
        .map(<[String]>::to_vec)
        .collect()
}

pub fn slot_tab_ids(slot: &SplitSlot) -> Vec<String> {
    match slot {
        SplitSlot::Other0(s) => vec![s.tab_id.clone()],
        SplitSlot::Other1(s) => vec![s.stack.0.clone(), s.stack.1.clone()],
    }
}

pub fn slot_size(slot: &SplitSlot) -> f64 {
    match slot {
        SplitSlot::Other0(s) => s.size.0,
        SplitSlot::Other1(s) => s.size.0,
    }
}

fn with_size(slot: &SplitSlot, size: f64) -> SplitSlot {
    let mut slot = slot.clone();
    match &mut slot {
        SplitSlot::Other0(s) => s.size = Number(size),
        SplitSlot::Other1(s) => s.size = Number(size),
    }
    slot
}

pub fn split_tab_ids(split: &SplitView) -> Vec<String> {
    split.slots.iter().flat_map(slot_tab_ids).collect()
}

pub fn split_tab_ids_of(split: Option<&SplitView>) -> Option<Vec<String>> {
    split.map(split_tab_ids)
}

pub fn is_stacked(split: &SplitView, tab_id: &str) -> bool {
    split.slots.iter().any(
        |slot| matches!(slot, SplitSlot::Other1(s) if s.stack.0 == tab_id || s.stack.1 == tab_id),
    )
}

pub fn even_slots(ids: &[String]) -> Vec<SplitSlot> {
    let size = Number(1.0 / ids.len() as f64);
    ids.iter()
        .map(|tab_id| {
            SplitSlot::Other0(SplitSlotVariant0 {
                size,
                tab_id: tab_id.clone(),
            })
        })
        .collect()
}

fn evened(slots: &[SplitSlot]) -> Vec<SplitSlot> {
    let size = 1.0 / slots.len() as f64;
    slots.iter().map(|slot| with_size(slot, size)).collect()
}

pub fn map_slot_ids(slots: &[SplitSlot], f: impl Fn(&str) -> String) -> Vec<SplitSlot> {
    slots
        .iter()
        .map(|slot| match slot {
            SplitSlot::Other1(s) => SplitSlot::Other1(SplitSlotVariant1 {
                stack: (f(&s.stack.0), f(&s.stack.1)),
                ..s.clone()
            }),
            SplitSlot::Other0(s) => SplitSlot::Other0(SplitSlotVariant0 {
                tab_id: f(&s.tab_id),
                ..s.clone()
            }),
        })
        .collect()
}

/// `stackAt` (`splits.ts:67`): the index of the first stacked slot, or -1.
pub fn stack_at(split: &SplitView) -> i32 {
    split
        .slots
        .iter()
        .position(|slot| matches!(slot, SplitSlot::Other1(_)))
        .map_or(-1, |i| i as i32)
}

pub fn with_pane_ids(split: &SplitView, ids: &[String]) -> SplitView {
    let mut it = ids.iter().cloned();
    let slots = split
        .slots
        .iter()
        .map(|slot| match slot {
            SplitSlot::Other1(s) => {
                let a = it.next().expect("with_pane_ids: too few ids");
                let b = it.next().expect("with_pane_ids: too few ids");
                SplitSlot::Other1(SplitSlotVariant1 {
                    stack: (a, b),
                    ..s.clone()
                })
            }
            SplitSlot::Other0(s) => SplitSlot::Other0(SplitSlotVariant0 {
                tab_id: it.next().expect("with_pane_ids: too few ids"),
                ..s.clone()
            }),
        })
        .collect();
    SplitView {
        slots,
        ..split.clone()
    }
}

/// The splits in a fixed order (see the module doc).
fn splits_in_order(splits: &HashMap<String, SplitView>) -> Vec<&SplitView> {
    let mut all: Vec<&SplitView> = splits.values().collect();
    all.sort_by(|a, b| a.id.cmp(&b.id));
    all
}

/// `splitOf` (`splits.ts:84`).
pub fn split_of<'a>(
    splits: &'a HashMap<String, SplitView>,
    tab_id: Option<&str>,
) -> Option<&'a SplitView> {
    let tab_id = tab_id.filter(|t| !t.is_empty())?;
    splits_in_order(splits)
        .into_iter()
        .find(|v| split_tab_ids(v).iter().any(|id| id == tab_id))
}

/// `splitRowPanes` (`splits.ts:90`).
pub fn split_row_panes(tabs: &HashMap<String, Tab>, split: Option<&SplitView>) -> Vec<String> {
    let Some(split) = split else { return vec![] };
    let ids = split_tab_ids(split);
    let unpinned: Vec<String> = ids
        .iter()
        .filter(|id| tabs.get(*id).is_some_and(|t| t.pin.is_none()))
        .cloned()
        .collect();
    if unpinned.len() >= 2 {
        return unpinned;
    }
    let rows: Vec<String> = ids
        .into_iter()
        .filter(|id| is_pinned_row(tabs.get(id).and_then(|t| t.pin.as_ref())))
        .collect();
    if rows.len() >= 2 { rows } else { vec![] }
}

/// `withSplitRows` (`splits.ts:98`).
pub fn with_split_rows(
    tabs: &HashMap<String, Tab>,
    splits: &HashMap<String, SplitView>,
    ids: &[String],
) -> Vec<String> {
    let mut out = ids.to_vec();
    for id in ids {
        let panes = split_row_panes(tabs, split_of(splits, Some(id)));
        if panes.contains(id) {
            for pane in panes {
                if !out.contains(&pane) {
                    out.push(pane);
                }
            }
        }
    }
    out
}

/// `withPinnedPanes` (`splits.ts:106`).
pub fn with_pinned_panes(
    tabs: &HashMap<String, Tab>,
    groups: &HashMap<String, TabGroup>,
    splits: &HashMap<String, SplitView>,
    ids: &[String],
) -> Vec<String> {
    let mut out = ids.to_vec();
    for id in ids {
        for pane in split_tab_ids_of(split_of(splits, Some(id))).unwrap_or_default() {
            if siblings::in_pinned_container(tabs, groups, &pane) && !out.contains(&pane) {
                out.push(pane);
            }
        }
    }
    out
}

/// `splitRowHead` (`splits.ts:112`).
pub fn split_row_head(
    tabs: &HashMap<String, Tab>,
    splits: &HashMap<String, SplitView>,
    order: &[String],
    id: &str,
) -> String {
    let panes = split_row_panes(tabs, split_of(splits, Some(id)));
    if panes.iter().any(|p| p == id) {
        order
            .iter()
            .find(|x| panes.contains(x))
            .cloned()
            .unwrap_or_else(|| id.to_string())
    } else {
        id.to_string()
    }
}

/// `removeFromSplits` (`splits.ts:117`).
pub fn remove_from_splits(
    splits: &HashMap<String, SplitView>,
    gone: &HashSet<String>,
) -> HashMap<String, SplitView> {
    let mut next = HashMap::new();
    for v in splits_in_order(splits) {
        if !split_tab_ids(v).iter().any(|id| gone.contains(id)) {
            next.insert(v.id.clone(), v.clone());
            continue;
        }
        let slots: Vec<SplitSlot> = v
            .slots
            .iter()
            .flat_map(|slot| match slot {
                SplitSlot::Other0(s) => (!gone.contains(&s.tab_id)).then(|| slot.clone()),
                SplitSlot::Other1(s) => {
                    let left: Vec<&String> = [&s.stack.0, &s.stack.1]
                        .into_iter()
                        .filter(|id| !gone.contains(*id))
                        .collect();
                    match left.len() {
                        2 => Some(slot.clone()),
                        1 => Some(SplitSlot::Other0(SplitSlotVariant0 {
                            size: s.size,
                            tab_id: left[0].clone(),
                        })),
                        _ => None,
                    }
                }
            })
            .collect();
        if slots.iter().map(|s| slot_tab_ids(s).len()).sum::<usize>() < 2 {
            continue;
        }
        let total: f64 = slots.iter().map(slot_size).sum();
        let slots = if total > 0.0 {
            slots
                .iter()
                .map(|s| with_size(s, slot_size(s) / total))
                .collect()
        } else {
            evened(&slots)
        };
        next.insert(v.id.clone(), SplitView { slots, ..v.clone() });
    }
    next
}

/// `sanitizeSplits` (`splits.ts:136`).
pub fn sanitize_splits(
    splits: &HashMap<String, SplitView>,
    tabs: &HashMap<String, Tab>,
) -> HashMap<String, SplitView> {
    let mut next = HashMap::new();
    let mut used: HashSet<String> = HashSet::new();
    for v in splits_in_order(splits) {
        let ids = split_tab_ids(v);
        let first = ids.first().and_then(|id| tabs.get(id));
        let ok = |id: &String| {
            tabs.get(id).is_some_and(|t| {
                t.window_id == v.window_id
                    && Some(&t.profile_id) == first.map(|f| &f.profile_id)
                    && !used.contains(id)
            })
        };
        let gone: HashSet<String> = ids.iter().filter(|id| !ok(id)).cloned().collect();
        let fixed = if gone.is_empty() {
            Some(v.clone())
        } else {
            remove_from_splits(&HashMap::from([(v.id.clone(), v.clone())]), &gone).remove(&v.id)
        };
        let Some(fixed) = fixed else { continue };
        let fixed_ids = split_tab_ids(&fixed);
        if fixed_ids.len() > MAX_SPLIT_PANES {
            continue;
        }
        used.extend(fixed_ids);
        next.insert(v.id.clone(), fixed);
    }
    next
}

pub fn window_id(w: &BrowserWindow) -> &str {
    match w {
        BrowserWindow::Main { id, .. }
        | BrowserWindow::Private { id, .. }
        | BrowserWindow::Small { id, .. } => id,
    }
}

pub fn window_profile_id(w: &BrowserWindow) -> &str {
    match w {
        BrowserWindow::Main { profile_id, .. }
        | BrowserWindow::Private { profile_id, .. }
        | BrowserWindow::Small { profile_id, .. } => profile_id,
    }
}

pub fn window_tab_ids(w: &BrowserWindow) -> &Vec<String> {
    match w {
        BrowserWindow::Main { tab_ids, .. }
        | BrowserWindow::Private { tab_ids, .. }
        | BrowserWindow::Small { tab_ids, .. } => tab_ids,
    }
}

pub fn window_tab_ids_mut(w: &mut BrowserWindow) -> &mut Vec<String> {
    match w {
        BrowserWindow::Main { tab_ids, .. }
        | BrowserWindow::Private { tab_ids, .. }
        | BrowserWindow::Small { tab_ids, .. } => tab_ids,
    }
}

pub fn window_active_tab(w: &BrowserWindow) -> Option<&String> {
    match w {
        BrowserWindow::Main {
            active_tab_ids,
            profile_id,
            ..
        }
        | BrowserWindow::Private {
            active_tab_ids,
            profile_id,
            ..
        }
        | BrowserWindow::Small {
            active_tab_ids,
            profile_id,
            ..
        } => active_tab_ids.get(profile_id),
    }
}

/// A copy of the 4 modules that a split action reads and writes.
#[derive(Debug, Clone, Default)]
pub struct Work {
    pub tabs: HashMap<String, Tab>,
    pub windows: HashMap<String, BrowserWindow>,
    pub groups: HashMap<String, TabGroup>,
    pub splits: HashMap<String, SplitView>,
}

impl Work {
    pub fn read(app: &AppState) -> Self {
        Self {
            tabs: app.get::<tabs::State>().tabs.clone(),
            windows: app.get::<windows::State>().windows.clone(),
            groups: app.get::<groups::State>().groups.clone(),
            splits: app.get::<State>().splits.clone(),
        }
    }

    /// Writes each module that differs from `app`.
    pub fn commit(self, app: &mut AppState) {
        if self.tabs != app.get::<tabs::State>().tabs {
            app.get_mut::<tabs::State>().tabs = self.tabs;
        }
        if self.windows != app.get::<windows::State>().windows {
            app.get_mut::<windows::State>().windows = self.windows;
        }
        if self.groups != app.get::<groups::State>().groups {
            app.get_mut::<groups::State>().groups = self.groups;
        }
        if self.splits != app.get::<State>().splits {
            app.get_mut::<State>().splits = self.splits;
        }
    }

    fn tab_pin(&self, id: &str) -> Option<&crate::browser::TabPin> {
        self.tabs.get(id).and_then(|t| t.pin.as_ref())
    }

    fn sync_groups(&mut self, window_id: &str) {
        if let Some(window) = self.windows.get(window_id) {
            self.groups = siblings::sync_group_order(&self.groups, window);
        }
    }
}

/// `Array.prototype.indexOf`.
fn index_of(v: &[String], id: &str) -> isize {
    v.iter().position(|x| x == id).map_or(-1, |i| i as isize)
}

/// `Array.prototype.splice` for a start index that may be -1.
fn splice(v: &mut Vec<String>, start: isize, delete: usize, items: Vec<String>) {
    let len = v.len() as isize;
    let at = if start < 0 {
        (len + start).max(0)
    } else {
        start.min(len)
    } as usize;
    let end = (at + delete).min(v.len());
    v.splice(at..end, items);
}

fn gather_pinned(w: &mut Work, anchor_id: &str, members: &[String]) {
    let newcomers: Vec<String> = members
        .iter()
        .filter(|id| {
            *id != anchor_id
                && w.tabs
                    .get(*id)
                    .is_some_and(|t| !is_pinned_row(t.pin.as_ref()))
        })
        .cloned()
        .collect();
    let settled = split_tab_ids_of(split_of(&w.splits, Some(anchor_id)))
        .unwrap_or_default()
        .into_iter()
        .filter(|id| !newcomers.contains(id) && is_pinned_row(w.tab_pin(id)))
        .count();
    if newcomers.is_empty() || settled < 2 {
        return;
    }
    siblings::placing(
        w,
        &newcomers,
        Placement {
            pinned: true,
            row: true,
            before_id: anchor_id.to_string(),
        },
    );
    let wid = w.tabs[anchor_id].window_id.clone();
    let block: Vec<String> = members
        .iter()
        .filter(|id| *id == anchor_id || newcomers.contains(id))
        .cloned()
        .collect();
    let win = w
        .windows
        .get_mut(&wid)
        .expect("gather_pinned: window of the anchor");
    let tab_ids = window_tab_ids_mut(win);
    tab_ids.retain(|id| !newcomers.contains(id));
    let at = index_of(tab_ids, anchor_id);
    splice(tab_ids, at, 1, block);
}

fn gather_lane(w: &mut Work, anchor_id: &str, members: &[String]) {
    let Some(anchor) = w.tabs.get(anchor_id) else {
        return;
    };
    if !w.windows.contains_key(&anchor.window_id) {
        return;
    }
    let wid = anchor.window_id.clone();
    if let Some(pin) = &anchor.pin {
        if is_pinned_row(Some(pin)) {
            gather_pinned(w, anchor_id, members);
        }
        return;
    }
    let mut moving: Vec<String> = vec![];
    for id in members {
        if id != anchor_id
            && w.tabs.get(id).is_some_and(|t| t.pin.is_none())
            && !moving.contains(id)
        {
            moving.push(id.clone());
        }
    }
    if moving.is_empty() {
        return;
    }
    let block: Vec<String> = members
        .iter()
        .filter(|id| *id == anchor_id || moving.contains(id))
        .cloned()
        .collect();
    let win = w
        .windows
        .get_mut(&wid)
        .expect("gather_lane: window checked above");
    let tab_ids = window_tab_ids_mut(win);
    tab_ids.retain(|id| !moving.contains(id));
    let at = index_of(tab_ids, anchor_id);
    splice(tab_ids, at, 1, block);
    let moving_set: HashSet<String> = moving.iter().cloned().collect();
    let mut groups = siblings::leave_groups(&w.groups, &moving_set, None);
    if let Some(group) = siblings::innermost_group_of(&w.groups, anchor_id)
        && let Some(g) = groups.get_mut(&group)
    {
        g.tab_ids.extend(moving);
    }
    w.groups = siblings::sync_group_order(&groups, &w.windows[&wid]);
}

fn beside_anchor(w: &mut Work, anchor_id: &str, ids: &[String]) {
    let Some(wid) = w.tabs.get(anchor_id).map(|t| t.window_id.clone()) else {
        return;
    };
    let Some(win) = w.windows.get_mut(&wid) else {
        return;
    };
    if ids.is_empty() {
        return;
    }
    let tab_ids = window_tab_ids_mut(win);
    tab_ids.retain(|id| !ids.contains(id));
    let at = index_of(tab_ids, anchor_id) + 1;
    splice(tab_ids, at, 0, ids.to_vec());
    w.sync_groups(&wid);
}

/// `gather` (`splits.ts:194`): puts the members next to the anchor, and adopts the tabs that cross a lane.
pub fn gather(w: &mut Work, anchor_id: &str, members: &[String], joining: &[String]) {
    let crossing: Vec<String> = match w.tabs.get(anchor_id) {
        Some(anchor) => members
            .iter()
            .filter(|id| {
                *id != anchor_id
                    && joining.contains(id)
                    && w.tabs
                        .get(*id)
                        .is_some_and(|t| crosses_lane(anchor.pin.as_ref(), t.pin.as_ref()))
            })
            .cloned()
            .collect(),
        None => vec![],
    };
    if !crossing.is_empty() {
        let pinned = is_pinned_row(w.tab_pin(anchor_id));
        siblings::placing(
            w,
            &crossing,
            Placement {
                pinned,
                row: true,
                before_id: anchor_id.to_string(),
            },
        );
    }
    gather_lane(w, anchor_id, members);
    beside_anchor(w, anchor_id, &crossing);
}

fn placed_after_split(w: &mut Work, split: &SplitView, tab_id: &str) {
    if !w.windows.contains_key(&split.window_id) || w.tab_pin(tab_id).is_some() {
        return;
    }
    let win = w
        .windows
        .get_mut(&split.window_id)
        .expect("placed_after_split: window checked above");
    let tab_ids = window_tab_ids_mut(win);
    tab_ids.retain(|id| id != tab_id);
    let last = split_tab_ids(split)
        .iter()
        .map(|id| index_of(tab_ids, id))
        .max()
        .unwrap_or(-1);
    splice(tab_ids, last + 1, 0, vec![tab_id.to_string()]);
    w.sync_groups(&split.window_id.clone());
}

fn with_pane(
    existing: Option<&SplitView>,
    anchor_id: &str,
    tab_id: &str,
    side: SplitSide,
    window_id: &str,
) -> Option<SplitView> {
    let horizontal = matches!(side, SplitSide::Left | SplitSide::Right);
    let before = matches!(side, SplitSide::Left | SplitSide::Top);
    let orientation = if horizontal {
        SplitViewOrientation::Horizontal
    } else {
        SplitViewOrientation::Vertical
    };
    let tab = tab_id.to_string();
    let anchor = anchor_id.to_string();
    let Some(existing) = existing else {
        let ids = if before { [tab, anchor] } else { [anchor, tab] };
        return Some(SplitView {
            id: siblings::new_id("split"),
            window_id: window_id.to_string(),
            orientation,
            slots: even_slots(&ids),
        });
    };
    if split_tab_ids(existing).len() >= MAX_SPLIT_PANES {
        return None;
    }
    let mut slots = existing.slots.clone();
    let k = slots
        .iter()
        .position(|slot| slot_tab_ids(slot).contains(&anchor))?;
    if (existing.orientation == SplitViewOrientation::Horizontal) == horizontal {
        slots.insert(
            if before { k } else { k + 1 },
            SplitSlot::Other0(SplitSlotVariant0 {
                size: Number(0.0),
                tab_id: tab,
            }),
        );
        return Some(SplitView {
            slots: evened(&slots),
            ..existing.clone()
        });
    }
    match slots[k].clone() {
        SplitSlot::Other1(slot) => {
            if slots.len() > 1 {
                return None;
            }
            let mut ids = vec![slot.stack.0, slot.stack.1];
            let at = (index_of(&ids, &anchor) + if before { 0 } else { 1 }) as usize;
            ids.insert(at, tab);
            Some(SplitView {
                orientation,
                slots: even_slots(&ids),
                ..existing.clone()
            })
        }
        SplitSlot::Other0(slot) => {
            let stack = if before { (tab, anchor) } else { (anchor, tab) };
            slots[k] = SplitSlot::Other1(SplitSlotVariant1 {
                size: slot.size,
                stack,
                stack_sizes: (Number(0.5), Number(0.5)),
            });
            Some(SplitView {
                slots,
                ..existing.clone()
            })
        }
    }
}

/// `createSplit` (`splits.ts:243`). Returns the id of the new split, or "".
pub fn create_split(
    app: &mut AppState,
    tab_ids: &[String],
    orientation: SplitViewOrientation,
) -> String {
    if !split_view_available() {
        return String::new();
    }
    let mut w = Work::read(app);
    let mut seen = HashSet::new();
    let tabs: Vec<&Tab> = tab_ids
        .iter()
        .filter(|id| seen.insert(*id))
        .filter_map(|id| w.tabs.get(id))
        .collect();
    let Some(first) = tabs.first() else {
        return String::new();
    };
    if tabs.len() < 2 || tabs.len() > MAX_SPLIT_PANES {
        return String::new();
    }
    if tabs
        .iter()
        .any(|t| t.window_id != first.window_id || t.profile_id != first.profile_id)
    {
        return String::new();
    }
    let first_id = first.id.clone();
    let window_id = first.window_id.clone();
    let ids: Vec<String> = tabs.iter().map(|t| t.id.clone()).collect();
    let mut splits = remove_from_splits(&w.splits, &ids.iter().cloned().collect());
    let split = SplitView {
        id: siblings::new_id("split"),
        window_id,
        orientation,
        slots: even_slots(&ids),
    };
    let split_id = split.id.clone();
    splits.insert(split_id.clone(), split);
    w.splits = splits;
    gather(&mut w, &first_id, &ids, &ids);
    w.commit(app);
    split_id
}

/// `openSplitPane` (`splits.ts:259`).
pub fn open_split_pane(app: &mut AppState, window_id: &str, o: &OpenPaneOpts) -> OpenPaneResult {
    use OpenPaneResult::{Invalid, Max};
    if !split_view_available() {
        return OpenPaneResult::Unavailable;
    }
    let mut s = Work::read(app);
    let w = s.windows.get(window_id);
    let anchor_id = o
        .anchor_tab_id
        .clone()
        .or_else(|| w.and_then(|w| window_active_tab(w).cloned()));
    let anchor = anchor_id
        .filter(|a| !a.is_empty())
        .and_then(|a| s.tabs.get(&a))
        .cloned();
    let (Some(_), Some(anchor)) = (w, anchor) else {
        return Invalid;
    };
    if anchor.window_id != window_id {
        return Invalid;
    }
    let existing = split_of(&s.splits, Some(&anchor.id)).cloned();
    let replaced = o
        .in_place_of
        .as_deref()
        .filter(|p| {
            !p.is_empty()
                && *p != anchor.id
                && existing
                    .as_ref()
                    .is_some_and(|e| split_tab_ids(e).iter().any(|id| id == p))
        })
        .map(str::to_string);
    if existing
        .as_ref()
        .is_some_and(|e| split_tab_ids(e).len() >= MAX_SPLIT_PANES)
        && replaced.is_none()
    {
        return Max;
    }

    let tab_id = match o.tab_id.as_deref().filter(|t| !t.is_empty()) {
        Some(tab_id) => {
            let Some(t) = s.tabs.get(tab_id) else {
                return Invalid;
            };
            if tab_id == anchor.id || t.window_id != window_id || t.profile_id != anchor.profile_id
            {
                return Invalid;
            }
            s.splits = remove_from_splits(&s.splits, &HashSet::from([tab_id.to_string()]));
            tab_id.to_string()
        }
        None => {
            let url = o.url.clone().filter(|u| !u.is_empty());
            let new_tab = siblings::NewTab {
                adopt_id: url.as_ref().and(o.adopt_id.clone()),
                opener_id: url.as_ref().map(|_| anchor.id.clone()),
                url,
                profile_id: anchor.profile_id.clone(),
            };
            siblings::with_new_tab(&mut s, window_id, new_tab)
        }
    };
    let joining: Vec<String> =
        if o.tab_id.as_deref().is_some_and(|t| !t.is_empty()) && !o.opened_for_split {
            vec![tab_id.clone()]
        } else {
            vec![]
        };
    let current = existing.as_ref().and_then(|e| s.splits.get(&e.id)).cloned();
    let split_id;
    if let (Some(replaced), Some(current)) = (&replaced, &current) {
        let ids: Vec<String> = split_tab_ids(current)
            .into_iter()
            .map(|id| if id == *replaced { tab_id.clone() } else { id })
            .collect();
        let split = with_pane_ids(current, &ids);
        gather(&mut s, &anchor.id, &split_tab_ids(&split), &joining);
        s.splits.insert(split.id.clone(), split.clone());
        placed_after_split(&mut s, &split, replaced);
        split_id = split.id;
    } else {
        let side = o.side.unwrap_or(SplitSide::Right);
        let Some(split) = with_pane(current.as_ref(), &anchor.id, &tab_id, side, window_id) else {
            return Max;
        };
        s.splits.insert(split.id.clone(), split.clone());
        gather(&mut s, &anchor.id, &split_tab_ids(&split), &joining);
        split_id = split.id;
    }
    s.commit(app);
    OpenPaneResult::Ok { tab_id, split_id }
}

/// `replaceSplitPane` (`splits.ts:304`). Returns the tab to activate, if the pane was replaced.
pub fn replace_split_pane(app: &mut AppState, pane_tab_id: &str, tab_id: &str) -> Option<String> {
    let mut s = Work::read(app);
    let split = split_of(&s.splits, Some(pane_tab_id))?.clone();
    let pane = s.tabs.get(pane_tab_id)?;
    let tab = s.tabs.get(tab_id)?;
    if split_tab_ids(&split).iter().any(|id| id == tab_id)
        || tab.window_id != pane.window_id
        || tab.profile_id != pane.profile_id
    {
        return None;
    }
    s.splits = remove_from_splits(&s.splits, &HashSet::from([tab_id.to_string()]));
    gather(
        &mut s,
        pane_tab_id,
        &[pane_tab_id.to_string(), tab_id.to_string()],
        &[pane_tab_id.to_string(), tab_id.to_string()],
    );
    let current = s.splits[&split.id].clone();
    let ids: Vec<String> = split_tab_ids(&current)
        .into_iter()
        .map(|id| {
            if id == pane_tab_id {
                tab_id.to_string()
            } else {
                id
            }
        })
        .collect();
    s.splits
        .insert(split.id.clone(), with_pane_ids(&current, &ids));
    siblings::remove_tabs(&mut s, &[pane_tab_id.to_string()], false);
    s.commit(app);
    Some(tab_id.to_string())
}

/// `removeTabFromSplit` (`splits.ts:318`).
pub fn remove_tab_from_split(app: &mut AppState, tab_id: &str) {
    let mut s = Work::read(app);
    let Some(split) = split_of(&s.splits, Some(tab_id)).cloned() else {
        return;
    };
    s.splits = remove_from_splits(&s.splits, &HashSet::from([tab_id.to_string()]));
    if s.splits.contains_key(&split.id) {
        let rest: Vec<String> = split_tab_ids(&split)
            .into_iter()
            .filter(|id| id != tab_id)
            .collect();
        let pinned = s.tab_pin(tab_id).is_some();
        let win = s
            .windows
            .get_mut(&split.window_id)
            .expect("remove_tab_from_split: window of the split");
        let mut tab_ids: Vec<String> = window_tab_ids(win)
            .iter()
            .filter(|id| *id != tab_id)
            .cloned()
            .collect();
        let last = rest
            .iter()
            .map(|id| index_of(&tab_ids, id))
            .max()
            .unwrap_or(-1);
        splice(&mut tab_ids, last + 1, 0, vec![tab_id.to_string()]);
        if !pinned {
            *window_tab_ids_mut(win) = tab_ids;
        }
        s.sync_groups(&split.window_id);
    }
    s.commit(app);
}

/// `separateSplit` (`splits.ts:335`).
pub fn separate_split(app: &mut AppState, split_id: &str) {
    app.get_mut::<State>().splits.remove(split_id);
}

/// `updateSplit` (`splits.ts:342`).
pub fn update_split(app: &mut AppState, id: &str, patch: &UpdateSplitPatch) {
    let Some(split) = app.get::<State>().splits.get(id) else {
        return;
    };
    let slots = split
        .slots
        .iter()
        .enumerate()
        .map(|(i, slot)| {
            let size = match &patch.sizes {
                Some(sizes) if sizes.len() == split.slots.len() => sizes[i].0,
                _ => slot_size(slot),
            };
            let mut slot = with_size(slot, size);
            if let (SplitSlot::Other1(s), Some(stack_sizes)) = (&mut slot, patch.stack_sizes) {
                s.stack_sizes = stack_sizes;
            }
            slot
        })
        .collect();
    let split = SplitView {
        slots,
        ..split.clone()
    };
    app.get_mut::<State>().splits.insert(id.to_string(), split);
}

/// `flipSplit` (`splits.ts:356`).
pub fn flip_split(app: &mut AppState, split_id: &str) {
    let Some(split) = app.get::<State>().splits.get(split_id) else {
        return;
    };
    let mut split = split.clone();
    split.slots.reverse();
    app.get_mut::<State>()
        .splits
        .insert(split_id.to_string(), split);
}

/// `toggleSplitOrientation` (`splits.ts:364`).
pub fn toggle_split_orientation(app: &mut AppState, split_id: &str) {
    let Some(split) = app.get::<State>().splits.get(split_id) else {
        return;
    };
    let mut split = split.clone();
    split.orientation = match split.orientation {
        SplitViewOrientation::Horizontal => SplitViewOrientation::Vertical,
        SplitViewOrientation::Vertical => SplitViewOrientation::Horizontal,
    };
    app.get_mut::<State>()
        .splits
        .insert(split_id.to_string(), split);
}

/// `movePane` (`splits.ts:373`).
pub fn move_pane(app: &mut AppState, tab_id: &str, delta: i64) {
    let Some(split) = split_of(&app.get::<State>().splits, Some(tab_id)) else {
        return;
    };
    let mut ids = split_tab_ids(split);
    let i = index_of(&ids, tab_id) as i64;
    let j = i + delta;
    if j < 0 || j >= ids.len() as i64 {
        return;
    }
    ids.swap(i as usize, j as usize);
    let split = with_pane_ids(split, &ids);
    app.get_mut::<State>()
        .splits
        .insert(split.id.clone(), split);
}

/// The tab that `focusPane` (`splits.ts:386`) activates, if the active tab is in a split.
pub fn pane_to_focus(app: &AppState, window_id: &str, delta: i64) -> Option<String> {
    let w = app.get::<windows::State>().windows.get(window_id)?;
    let active = window_active_tab(w)?;
    let split = split_of(&app.get::<State>().splits, Some(active))?;
    let panes = split_tab_ids(split);
    let i = index_of(&panes, active) as i64;
    let n = panes.len() as i64;
    Some(panes[(i + delta + n).rem_euclid(n) as usize].clone())
}

/// `toggleTabLayout` (`splits.ts:397`).
pub fn toggle_tab_layout(app: &mut AppState, window_id: &str) {
    use crate::browser::{
        BrowserWindowMainTabLayout as M, BrowserWindowPrivateTabLayout as P,
        BrowserWindowSmallTabLayout as S,
    };
    if !app.get::<windows::State>().windows.contains_key(window_id) {
        return;
    }
    let to_sidebar = siblings::tab_layout_of(app, window_id) != "sidebar";
    match app.get_mut::<windows::State>().windows.get_mut(window_id) {
        Some(BrowserWindow::Main { tab_layout, .. }) => {
            *tab_layout = Some(if to_sidebar { M::Sidebar } else { M::Top })
        }
        Some(BrowserWindow::Private { tab_layout, .. }) => {
            *tab_layout = Some(if to_sidebar { P::Sidebar } else { P::Top })
        }
        Some(BrowserWindow::Small { tab_layout, .. }) => {
            *tab_layout = Some(if to_sidebar { S::Sidebar } else { S::Top })
        }
        None => {}
    }
    siblings::choose_tab_layout(app, if to_sidebar { "sidebar" } else { "top" });
}
