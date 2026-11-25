// Ported from ardia apps/browser/src/state/browser/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `browser` (`apps/browser/src/state/browser`).
//!
//! `State` is the data of the TypeScript store, `Action` has one variant for each exported store action, and `apply`
//! is the reducer. Files: `types` (types.ts), `model` (model.ts, structure.ts, changes.ts), `settle` (settle.ts),
//! `store` (`hydrate` of store.ts), `actions` (actions.ts). `pending_siblings` has the stubs of the sibling functions
//! that these files call.
//!
//! Changes from Ardia:
//! - `State` is the whole composed store (`BrowserState`), as in Ardia where one zustand store holds every slice. The
//!   sibling `State` types of the 2.4 skeleton keep their own copies until gate G2 joins them.
//! - The subscription code of zustand and React has no data and no Rust form: `transaction.ts` (depth and the held
//!   listeners), `keyedWatch.ts`, `hooks.ts`, `recordReads.ts`, `current.ts`, `actionsApart.ts` and
//!   `foreignWriteHooks.ts`. The kernel change counters (plan section 6.6) wake the UI in place of them.
//! - `deleteProfile`, `closeTab`, `closeGroup`, the profile moves and `openProfileMenu` await a dialog or a menu in
//!   Ardia. The second action of each is not written; the arm of `run` stops with `todo!()` after its first guard.

use serde::{Deserialize, Serialize};

use crate::kernel::{AppState, Effects, Number};

mod actions;
mod model;
mod pending_siblings;
mod settle;
mod store;
mod types;

pub use actions::*;
pub use model::*;
pub use settle::*;
pub use store::hydrate;
pub use types::*;

/// The data fields of the TypeScript store.
pub type State = BrowserState;

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `closeGroup`. The callback parameter `mayWrite` is not in the JSON.
    #[serde(rename = "closeGroup")]
    CloseGroup {
        #[serde(rename = "groupId")]
        group_id: String,
    },
    /// `closeTab`. The callback parameter `mayWrite` is not in the JSON.
    #[serde(rename = "closeTab")]
    CloseTab {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `cycleProfile`. The callback parameter `mayWrite` is not in the JSON.
    #[serde(rename = "cycleProfile")]
    CycleProfile {
        #[serde(rename = "windowId")]
        window_id: String,
        delta: Number,
    },
    /// `deleteProfile`. The callback parameters `mayDelete`, `ask` are not in the JSON.
    #[serde(rename = "deleteProfile")]
    DeleteProfile {
        #[serde(rename = "profileId")]
        profile_id: String,
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `dropTabsInProfile`.
    #[serde(rename = "dropTabsInProfile")]
    DropTabsInProfile {
        #[serde(rename = "tabIds")]
        tab_ids: Vec<String>,
        target: String,
        drop: crate::drag::Drop,
        home: String,
        #[serde(rename = "onIcon")]
        on_icon: bool,
    },
    /// `focus`.
    #[serde(rename = "focus")]
    Focus {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `hydrate` (a store method).
    #[serde(rename = "hydrate")]
    Hydrate { data: HydrateData },
    /// `moveGroupToProfile`.
    #[serde(rename = "moveGroupToProfile")]
    MoveGroupToProfile {
        #[serde(rename = "groupId")]
        group_id: String,
        target: Option<String>,
    },
    /// `moveGroupToWindow`.
    #[serde(rename = "moveGroupToWindow")]
    MoveGroupToWindow {
        #[serde(rename = "groupId")]
        group_id: String,
        target: Option<String>,
    },
    /// `moveTabsToProfile`. The callback parameter `mayWrite` is not in the JSON.
    #[serde(rename = "moveTabsToProfile")]
    MoveTabsToProfile {
        #[serde(rename = "tabIds")]
        tab_ids: Vec<String>,
        target: Option<String>,
    },
    /// `moveTabToProfile`. The callback parameter `mayWrite` is not in the JSON.
    #[serde(rename = "moveTabToProfile")]
    MoveTabToProfile {
        #[serde(rename = "tabId")]
        tab_id: String,
        target: Option<String>,
    },
    /// `moveTabToWindow`.
    #[serde(rename = "moveTabToWindow")]
    MoveTabToWindow {
        #[serde(rename = "tabId")]
        tab_id: String,
        target: Option<String>,
    },
    /// `openProfileMenu`.
    #[serde(rename = "openProfileMenu")]
    OpenProfileMenu {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `openUrls`.
    #[serde(rename = "openUrls")]
    OpenUrls {
        urls: Vec<String>,
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `openWindow`.
    #[serde(rename = "openWindow")]
    OpenWindow {
        #[serde(default)]
        options: Option<crate::windows::CreateWindowOptions>,
    },
    /// `setSiteMuted`.
    #[serde(rename = "setSiteMuted")]
    SetSiteMuted {
        #[serde(rename = "tabId")]
        tab_id: String,
        muted: bool,
    },
    /// `switchProfile`. The callback parameter `mayWrite` is not in the JSON.
    #[serde(rename = "switchProfile")]
    SwitchProfile {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "profileId")]
        profile_id: String,
        #[serde(default)]
        animated: Option<bool>,
    },
    /// `switchToTab`.
    #[serde(rename = "switchToTab")]
    SwitchToTab {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `toggleMute`.
    #[serde(rename = "toggleMute")]
    ToggleMute {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `toggleSiteMute`.
    #[serde(rename = "toggleSiteMute")]
    ToggleSiteMute {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
}

/// The reducer. After an action that changed the state, `settle` runs on it, as the transaction middleware of
/// transaction.ts:30-38 runs `settle` when a write is published. A `SettleDidNotConverge` keeps the settled state
/// (store.ts:81-85 does the same outside tests).
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let prev = app.get::<State>().clone();
    run(app.get_mut::<State>(), action, fx);
    if *app.get::<State>() == prev {
        return;
    }
    let current = std::mem::take(app.get_mut::<State>());
    *app.get_mut::<State>() = settle(current, &prev).unwrap_or_else(|e| {
        eprintln!("{e}");
        *e.settled
    });
}

fn run(s: &mut State, action: Action, fx: &mut Effects) {
    match action {
        Action::Focus { window_id } => actions::focus(s, fx, &window_id),
        Action::SwitchToTab { tab_id } => actions::switch_to_tab(s, fx, &tab_id),
        Action::ToggleMute { tab_id } => actions::toggle_mute(s, &tab_id),
        Action::ToggleSiteMute { tab_id } => actions::toggle_site_mute(s, &tab_id),
        Action::SetSiteMuted { tab_id, muted } => {
            pending_siblings::set_site_muted(s, &tab_id, muted)
        }
        Action::OpenWindow { options } => {
            pending_siblings::create_window(s, options.as_ref());
        }
        Action::OpenUrls { urls, window_id } => actions::open_urls(s, &urls, window_id.as_deref()),
        Action::MoveGroupToWindow { group_id, target } => {
            actions::move_group_to_window_action(s, fx, &group_id, target.as_deref())
        }
        Action::MoveTabToWindow { tab_id, target } => {
            actions::move_tab_to_window(s, fx, &tab_id, target.as_deref())
        }
        Action::SwitchProfile {
            window_id,
            profile_id,
            animated,
        } => actions::switch_profile_action(s, &window_id, &profile_id, animated.unwrap_or(false)),
        Action::CycleProfile { window_id, delta } => {
            actions::cycle_profile(s, &window_id, delta.0 as i64)
        }
        Action::Hydrate { data } => *s = store::hydrate(s, data),
        // The actions that await a dialog: the guard of the first leg is written, the dialog and the second action are not.
        Action::CloseTab { tab_id } => {
            if s.tabs
                .get(&tab_id)
                .is_some_and(|t| s.windows.contains_key(&t.window_id))
            {
                todo!("slice 2.5: closeTab awaits a dialog (second action not written)");
            }
        }
        Action::CloseGroup { group_id } => {
            if s.groups
                .get(&group_id)
                .is_some_and(|g| s.windows.contains_key(&g.window_id))
            {
                todo!("slice 2.5: closeGroup awaits a dialog (second action not written)");
            }
        }
        Action::DeleteProfile { profile_id, .. } => {
            if s.profiles.contains_key(&profile_id) && s.profile_order.len() > 1 {
                todo!("slice 2.5: deleteProfile awaits a dialog (second action not written)");
            }
        }
        Action::MoveTabsToProfile { tab_ids, .. } | Action::DropTabsInProfile { tab_ids, .. } => {
            if tab_ids.iter().any(|id| s.tabs.contains_key(id)) {
                todo!("slice 2.5: the profile move awaits a dialog (second action not written)");
            }
        }
        Action::MoveTabToProfile { tab_id, .. } => {
            if s.tabs.contains_key(&tab_id) {
                todo!("slice 2.5: moveTabToProfile awaits a dialog (second action not written)");
            }
        }
        Action::MoveGroupToProfile { group_id, .. } => {
            if s.groups.contains_key(&group_id) {
                todo!("slice 2.5: moveGroupToProfile awaits a dialog (second action not written)");
            }
        }
        Action::OpenProfileMenu { .. } => {
            todo!("slice 2.5: openProfileMenu awaits a menu choice (second action not written)")
        }
    }
}
