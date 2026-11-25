// Ported from ardia apps/browser/src/state/browser/settle.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The settle loop: after a write, each step may change the state again, until a round changes nothing.
//!
//! Changes from Ardia: a step returns the whole new state in place of a patch (`patched(..)` of settle.ts:20-26 is
//! inside the sibling function), and a round has changed when a step returns `Some`. Ardia also compares object
//! identity (`out === s`); a Rust step returns `None` for no change.

use super::pending_siblings::*;
use super::types::BrowserState;

pub const SETTLE_ROUNDS: usize = 8;

/// The steps in the order of `SETTLE_STEPS` (settle.ts:28-46).
pub const SETTLE_STEPS: [(&str, Step); 17] = [
    ("splitsWhereUnavailable", splits_where_unavailable),
    ("forgetClosedWindowMutes", forget_closed_window_mutes),
    ("forgetClosedWindowCleanUps", forget_closed_window_clean_ups),
    ("forgetMovedAnchors", forget_moved_anchors),
    ("settleNesting", settle_nesting),
    ("settleShownTabs", settle_shown_tabs),
    ("livePinTables", live_pin_tables),
    ("settlePeeks", settle_peeks),
    ("settlePinHomes", settle_pin_homes),
    ("unshareTabs", unshare_tabs),
    ("pagesIntoSplits", pages_into_splits),
    ("settleSharedPins", settle_shared_pins),
    ("settleTileFolders", settle_tile_folders),
    ("mirrorPins", mirror_pins),
    ("mirrorTopApps", mirror_top_apps),
    ("mirrorTabs", mirror_tabs),
    ("pinnedTabsLeaveLive", pinned_tabs_leave_live),
];

/// `SettleDidNotConverge` (settle.ts:50): the steps that still changed in the last round, and the state then.
#[derive(Debug, Clone, PartialEq)]
pub struct SettleDidNotConverge {
    pub steps: Vec<String>,
    pub settled: Box<BrowserState>,
}

impl std::fmt::Display for SettleDidNotConverge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The store didn't settle in {SETTLE_ROUNDS} rounds; still changing: {}",
            self.steps.join(", ")
        )
    }
}

impl std::error::Error for SettleDidNotConverge {}

/// `settle` (settle.ts:60).
pub fn settle(
    next: BrowserState,
    prev: &BrowserState,
) -> Result<BrowserState, SettleDidNotConverge> {
    settle_with(next, prev, &SETTLE_STEPS)
}

/// `settle` with the steps as an argument, as the `steps` parameter of Ardia.
pub fn settle_with(
    next: BrowserState,
    prev: &BrowserState,
    steps: &[(&str, Step)],
) -> Result<BrowserState, SettleDidNotConverge> {
    let mut s = next;
    let mut base = prev.clone();
    for round in 0..SETTLE_ROUNDS {
        let input = s.clone();
        let mut changed = Vec::new();
        for (name, step) in steps {
            if let Some(out) = step(&s, &base) {
                s = out;
                changed.push((*name).to_string());
            }
        }
        if changed.is_empty() {
            return Ok(s);
        }
        if round == SETTLE_ROUNDS - 1 {
            return Err(SettleDidNotConverge {
                steps: changed,
                settled: Box::new(s),
            });
        }
        base = input;
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::Number;

    fn quiet(_: &BrowserState, _: &BrowserState) -> Option<BrowserState> {
        None
    }

    fn count_up(s: &BrowserState, _: &BrowserState) -> Option<BrowserState> {
        let mut next = s.clone();
        let n = next.top_apps_merged.parse::<u32>().unwrap_or(0);
        (n < 3).then(|| {
            next.top_apps_merged = (n + 1).to_string();
            next
        })
    }

    fn forever(s: &BrowserState, _: &BrowserState) -> Option<BrowserState> {
        let mut next = s.clone();
        next.top_apps_merged.push('x');
        Some(next)
    }

    #[test]
    fn runs_rounds_until_no_step_changes() {
        let out = settle_with(
            BrowserState::default(),
            &BrowserState::default(),
            &[("quiet", quiet), ("count", count_up)],
        )
        .unwrap();
        assert_eq!(out.top_apps_merged, "3");
    }

    #[test]
    fn the_eighth_changing_round_is_an_error_that_names_the_steps() {
        let err = settle_with(
            BrowserState::default(),
            &BrowserState::default(),
            &[("forever", forever)],
        )
        .unwrap_err();
        assert_eq!(err.steps, vec!["forever"]);
        assert_eq!(err.settled.top_apps_merged.len(), SETTLE_ROUNDS);
        assert!(err.to_string().ends_with("still changing: forever"));
        let _ = Number(0.0);
    }
}
