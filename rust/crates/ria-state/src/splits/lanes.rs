// Ported from ardia apps/browser/src/state/splits/lanes.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Split lanes: where a tab sits in the sidebar (`lanes.ts`).
//!
//! Changes from Ardia: the functions take `Option<&TabPin>` where TypeScript takes `Pick<Tab, "pin">`.
//! `is_pinned_row` is a one-line copy of `isPinnedRow` (`browser/model.ts:92`).
//! shortcut: delete the copy and call the sibling when the `browser` folder is ported.

use std::collections::HashMap;

use crate::browser::{Tab, TabPin};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitLane {
    Row,
    List,
    Tile,
}

pub fn is_pinned_row(pin: Option<&TabPin>) -> bool {
    matches!(pin, Some(TabPin::Row))
}

/// `splitLane` (`lanes.ts:6`).
pub fn split_lane(pin: Option<&TabPin>) -> SplitLane {
    match pin {
        None => SplitLane::List,
        Some(TabPin::Row) => SplitLane::Row,
        Some(TabPin::Tile { .. }) => SplitLane::Tile,
    }
}

/// `crossesLane` (`lanes.ts:8`).
pub fn crosses_lane(anchor: Option<&TabPin>, t: Option<&TabPin>) -> bool {
    let lane = split_lane(anchor);
    let own = split_lane(t);
    lane != SplitLane::Tile && own != SplitLane::Tile && own != lane
}

/// `splitsOnRow` (`lanes.ts:14`).
pub fn splits_on_row(tabs: &HashMap<String, Tab>, tab_id: &str, row_tab_id: &str) -> bool {
    let (Some(tab), Some(row)) = (tabs.get(tab_id), tabs.get(row_tab_id)) else {
        return false;
    };
    tab.id != row.id
        && tab.window_id == row.window_id
        && tab.profile_id == row.profile_id
        && (split_lane(tab.pin.as_ref()) == SplitLane::Tile)
            == (split_lane(row.pin.as_ref()) == SplitLane::Tile)
}
