// Ported from ardia apps/browser/src/state/splits/actions.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The exported split actions (`actions.ts`).
//!
//! Changes from Ardia:
//! - Calls to other stores go through `apply` of the sibling module (`tabs`, `browser`, `app`) with one of its `Action`s.
//! - `activeTabId` (`browser/model.ts:134`) is a private copy. shortcut: delete it when the `browser` folder is ported.
//! - `openSplitPane`, `dropTabIntoSplit` and `dropTabOnRow` return a value to their callers in TypeScript.
//!   The Rust functions return it to the callers in this folder; `apply` drops it.
//! - `showSplitMenu` awaits a native menu and has no `PlatformCall` yet; it is in `pending_siblings.rs`.

use super::lanes::{SplitLane, split_lane, splits_on_row};
use super::pending_siblings as siblings;
use super::splits::{
    self, MAX_SPLIT_PANES, OpenPaneOpts, OpenPaneResult, split_of, split_tab_ids, split_tab_ids_of,
    window_active_tab,
};
use super::{
    DropTabOnRowSide, OpenLinkInSplitAdoptId, OpenPaneOptions, OpenPaneOptionsAdoptId,
    OpenPaneOptionsSide, SplitSide, State,
};
use crate::kernel::{AppState, Effects};
use crate::{app, browser, tabs, windows};

/// `RowSplitDrop` (`actions.ts:63`).
pub struct RowSplitDrop {
    pub adopted: Option<SplitLane>,
}

fn active_tab_id(app: &AppState, window_id: &str) -> Option<String> {
    let w = app.get::<windows::State>().windows.get(window_id)?;
    let id = window_active_tab(w)?;
    (app.get::<tabs::State>().tabs.get(id)?.window_id == window_id).then(|| id.clone())
}

fn adopt_text(id: &OpenPaneOptionsAdoptId) -> &str {
    match id {
        OpenPaneOptionsAdoptId::Other0(s)
        | OpenPaneOptionsAdoptId::Other1(s)
        | OpenPaneOptionsAdoptId::Other2(s)
        | OpenPaneOptionsAdoptId::Other3(s)
        | OpenPaneOptionsAdoptId::Other4(s)
        | OpenPaneOptionsAdoptId::Other5(s) => s,
    }
}

fn link_adopt_text(id: &OpenLinkInSplitAdoptId) -> &str {
    match id {
        OpenLinkInSplitAdoptId::Other0(s)
        | OpenLinkInSplitAdoptId::Other1(s)
        | OpenLinkInSplitAdoptId::Other2(s)
        | OpenLinkInSplitAdoptId::Other3(s)
        | OpenLinkInSplitAdoptId::Other4(s)
        | OpenLinkInSplitAdoptId::Other5(s) => s,
    }
}

/// The adopt id of `AdoptId` (`browser/types.ts`) is a string in a union of 6 literals; the 3 Rust enums hold the same strings.
fn new_tab_adopt(text: &str) -> tabs::NewTabOptionsAdoptId {
    tabs::NewTabOptionsAdoptId::Other0(text.to_string())
}

fn side_of(side: OpenPaneOptionsSide) -> SplitSide {
    match side {
        OpenPaneOptionsSide::Top => SplitSide::Top,
        OpenPaneOptionsSide::Left => SplitSide::Left,
        OpenPaneOptionsSide::Right => SplitSide::Right,
        OpenPaneOptionsSide::Bottom => SplitSide::Bottom,
    }
}

fn pane_opts(o: &OpenPaneOptions) -> OpenPaneOpts {
    OpenPaneOpts {
        tab_id: o.tab_id.clone(),
        url: o.url.clone(),
        anchor_tab_id: o.anchor_tab_id.clone(),
        side: o.side.map(side_of),
        background: o.background.unwrap_or(false),
        adopt_id: o.adopt_id.as_ref().map(|a| adopt_text(a).to_string()),
        in_place_of: o.in_place_of.clone(),
        opened_for_split: o.opened_for_split.unwrap_or(false),
    }
}

fn opts_of(o: &OpenPaneOpts) -> OpenPaneOptions {
    OpenPaneOptions {
        tab_id: o.tab_id.clone(),
        url: o.url.clone(),
        anchor_tab_id: o.anchor_tab_id.clone(),
        side: o.side.map(|s| match s {
            SplitSide::Top => OpenPaneOptionsSide::Top,
            SplitSide::Left => OpenPaneOptionsSide::Left,
            SplitSide::Right => OpenPaneOptionsSide::Right,
            SplitSide::Bottom => OpenPaneOptionsSide::Bottom,
        }),
        background: o.background.then_some(true),
        adopt_id: o
            .adopt_id
            .as_ref()
            .map(|a| OpenPaneOptionsAdoptId::Other0(a.clone())),
        in_place_of: o.in_place_of.clone(),
        opened_for_split: o.opened_for_split.then_some(true),
    }
}

/// The tab that `activate` of the `tabs` store makes active.
fn activate(app: &mut AppState, id: &str, fx: &mut Effects) {
    tabs::apply(app, tabs::Action::Activate { id: id.to_string() }, fx);
}

/// `openSplitPane` (`actions.ts:15`).
pub fn open_split_pane(
    app: &mut AppState,
    window_id: &str,
    options: &OpenPaneOptions,
    fx: &mut Effects,
) -> Option<String> {
    let o = pane_opts(options);
    let result = splits::open_split_pane(app, window_id, &o);
    match result {
        OpenPaneResult::Ok { tab_id, .. } => {
            if !o.background {
                activate(app, &tab_id, fx);
            }
            Some(tab_id)
        }
        OpenPaneResult::Unavailable => {
            if o.url.as_deref().is_some_and(|u| !u.is_empty()) {
                open_as_tab(app, window_id, &o, fx)
            } else {
                None
            }
        }
        OpenPaneResult::Max => {
            app::apply(
                app,
                app::Action::ShowToast {
                    window_id: window_id.to_string(),
                    title: "Cannot Add New Pane".to_string(),
                    message: Some(format!(
                        "Split View can show up to {MAX_SPLIT_PANES} tabs at once."
                    )),
                    options: None,
                },
                fx,
            );
            None
        }
        OpenPaneResult::Invalid => None,
    }
}

/// `focusSplitPane` (`actions.ts:26`).
pub fn focus_split_pane(app: &mut AppState, window_id: &str, delta: i64, fx: &mut Effects) {
    let before = active_tab_id(app, window_id);
    if let Some(pane) = splits::pane_to_focus(app, window_id, delta) {
        activate(app, &pane, fx);
    }
    if active_tab_id(app, window_id) != before {
        siblings::return_focus_to_page(window_id);
    }
}

/// `openAsTab` (`actions.ts:32`).
fn open_as_tab(
    app: &mut AppState,
    window_id: &str,
    o: &OpenPaneOpts,
    fx: &mut Effects,
) -> Option<String> {
    let anchor = o
        .anchor_tab_id
        .as_deref()
        .filter(|a| !a.is_empty())
        .and_then(|a| app.get::<tabs::State>().tabs.get(a))
        .cloned();
    if !app.get::<windows::State>().windows.contains_key(window_id) {
        return None;
    }
    let options = tabs::NewTabOptions {
        url: o.url.clone(),
        adopt_id: o.adopt_id.as_deref().map(new_tab_adopt),
        background: o.background.then_some(true),
        opener_id: anchor.as_ref().map(|a| a.id.clone()),
        profile_id: anchor.map(|a| a.profile_id),
        gated: Some(true),
        ..Default::default()
    };
    tabs::apply(
        app,
        tabs::Action::NewTab {
            window_id: window_id.to_string(),
            options: Some(options),
        },
        fx,
    );
    // shortcut: `newTab` returns the id of the new tab, but `Action` has no result; the id is not needed here.
    None
}

/// `openNewTabInSplit` (`actions.ts:41`).
pub fn open_new_tab_in_split(app: &mut AppState, window_id: &str, fx: &mut Effects) {
    open_split_pane(app, window_id, &OpenPaneOptions::default(), fx);
}

/// `openInSplit` (`actions.ts:43`).
pub fn open_in_split(app: &mut AppState, tab_id: &str, side: SplitSide, fx: &mut Effects) {
    let Some(tab) = app.get::<tabs::State>().tabs.get(tab_id) else {
        return;
    };
    let window_id = tab.window_id.clone();
    let Some(active) = active_tab_id(app, &window_id) else {
        return;
    };
    if active == tab_id {
        return;
    }
    let options = opts_of(&OpenPaneOpts {
        tab_id: Some(tab_id.to_string()),
        anchor_tab_id: Some(active),
        side: Some(side),
        ..Default::default()
    });
    open_split_pane(app, &window_id, &options, fx);
}

/// `openLinkInSplit` (`actions.ts:51`).
pub fn open_link_in_split(
    app: &mut AppState,
    source_tab_id: &str,
    url: &str,
    adopt_id: Option<&OpenLinkInSplitAdoptId>,
    fx: &mut Effects,
) {
    let Some(source) = app.get::<tabs::State>().tabs.get(source_tab_id) else {
        return;
    };
    let window_id = source.window_id.clone();
    let panes = split_tab_ids_of(split_of(&app.get::<State>().splits, Some(source_tab_id)));
    let target = panes.as_ref().and_then(|panes| {
        let next = panes
            .iter()
            .position(|p| p == source_tab_id)
            .and_then(|i| panes.get(i + 1))
            .filter(|n| !n.is_empty());
        if next.is_some() || panes.len() >= MAX_SPLIT_PANES {
            next.or(panes.last()).cloned()
        } else {
            None
        }
    });
    let in_place_of = target.filter(|t| t != source_tab_id);
    let options = OpenPaneOptions {
        anchor_tab_id: Some(source_tab_id.to_string()),
        url: Some(url.to_string()),
        side: Some(OpenPaneOptionsSide::Right),
        background: Some(true),
        adopt_id: adopt_id.map(|a| OpenPaneOptionsAdoptId::Other0(link_adopt_text(a).to_string())),
        in_place_of,
        ..Default::default()
    };
    open_split_pane(app, &window_id, &options, fx);
}

/// `dropTabIntoSplit` (`actions.ts:63`).
pub fn drop_tab_into_split(
    app: &mut AppState,
    tab_id: &str,
    target_tab_id: &str,
    side: SplitSide,
    fx: &mut Effects,
) -> Option<String> {
    let target = app.get::<tabs::State>().tabs.get(target_tab_id)?;
    if tab_id == target_tab_id {
        return None;
    }
    let window_id = target.window_id.clone();
    let options = opts_of(&OpenPaneOpts {
        tab_id: Some(tab_id.to_string()),
        anchor_tab_id: Some(target_tab_id.to_string()),
        side: Some(side),
        ..Default::default()
    });
    open_split_pane(app, &window_id, &options, fx)
}

/// `dropTabOnRow` (`actions.ts:70`).
pub fn drop_tab_on_row(
    app: &mut AppState,
    tab_id: &str,
    row_tab_id: &str,
    side: DropTabOnRowSide,
    fx: &mut Effects,
) -> Option<RowSplitDrop> {
    let all = &app.get::<tabs::State>().tabs;
    let (tab, row) = (all.get(tab_id)?, all.get(row_tab_id)?);
    if !splits_on_row(all, tab_id, row_tab_id) {
        return None;
    }
    let row_lane = split_lane(row.pin.as_ref());
    let tab_lane = split_lane(tab.pin.as_ref());
    let split = split_of(&app.get::<State>().splits, Some(row_tab_id));
    let panes = split_tab_ids_of(split);
    if panes
        .as_ref()
        .is_some_and(|p| p.iter().any(|id| id == tab_id))
    {
        return None;
    }
    let row_panes: Option<Vec<String>> = panes.map(|p| {
        p.into_iter()
            .filter(|id| {
                all.get(id)
                    .is_some_and(|t| split_lane(t.pin.as_ref()) == row_lane)
            })
            .collect()
    });
    let left = matches!(side, DropTabOnRowSide::Left);
    let anchor = match row_panes.as_ref().filter(|r| !r.is_empty()) {
        Some(r) => if left { r.first() } else { r.last() }
            .cloned()
            .unwrap_or_default(),
        None => row_tab_id.to_string(),
    };
    let stacked = split.is_some_and(|s| s.orientation == crate::browser::Orientation::Vertical);
    let drop_side = match (stacked, left, side) {
        (true, true, _) => SplitSide::Top,
        (true, false, _) => SplitSide::Bottom,
        (false, _, DropTabOnRowSide::Left) => SplitSide::Left,
        (false, _, DropTabOnRowSide::Right) => SplitSide::Right,
    };
    drop_tab_into_split(app, tab_id, &anchor, drop_side, fx)?;
    let now = app
        .get::<tabs::State>()
        .tabs
        .get(tab_id)
        .map(|t| split_lane(t.pin.as_ref()));
    Some(RowSplitDrop {
        adopted: now.filter(|n| *n != tab_lane),
    })
}

/// `closePane` (`actions.ts:84`).
pub fn close_pane(app: &mut AppState, tab_id: &str, fx: &mut Effects) {
    browser::apply(
        app,
        browser::Action::CloseTab {
            tab_id: tab_id.to_string(),
        },
        fx,
    );
}

/// `closeSplitView` (`actions.ts:88`).
pub fn close_split_view(app: &mut AppState, split_id: &str, fx: &mut Effects) {
    let Some(split) = app.get::<State>().splits.get(split_id) else {
        return;
    };
    let ids = split_tab_ids(split);
    siblings::note_split("splitViewCloses");
    tabs::apply(app, tabs::Action::CloseTabs { ids, options: None }, fx);
}
