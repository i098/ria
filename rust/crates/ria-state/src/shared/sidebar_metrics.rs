// Ported from ardia apps/browser/src/lib/sidebarMetrics.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The sizes of the sidebar's rows (from the look in force) and of its groups.
//! The file exports no function (only constants and getters), so it has no bridge crate and no
//! recorded cases.
//!
//! Changes from Ardia: the getters of `SIDEBAR_ROW` are methods: `SIDEBAR_ROW.height()`, `.gap()`
//! and `.pinned_height()`.

use super::active_look::active_look;

/// The type of `SIDEBAR_ROW` (`sidebarMetrics.ts:3`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SidebarRow;

impl SidebarRow {
    /// `SIDEBAR_ROW.height` (`sidebarMetrics.ts:4`).
    pub fn height(self) -> f64 {
        active_look().metrics.row_height
    }

    /// `SIDEBAR_ROW.gap` (`sidebarMetrics.ts:7`).
    pub fn gap(self) -> f64 {
        active_look().metrics.row_gap
    }

    /// `SIDEBAR_ROW.pinnedHeight` (`sidebarMetrics.ts:10`).
    pub fn pinned_height(self) -> f64 {
        active_look().metrics.pinned_height
    }
}

/// `SIDEBAR_ROW` (`sidebarMetrics.ts:3`).
pub const SIDEBAR_ROW: SidebarRow = SidebarRow;

/// The type of `SIDEBAR_GROUP` (`sidebarMetrics.ts:15`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SidebarGroup {
    pub indent: f64,
    pub border: f64,
    pub radius: f64,
    pub inset: f64,
}

/// `SIDEBAR_GROUP` (`sidebarMetrics.ts:15`).
pub const SIDEBAR_GROUP: SidebarGroup = SidebarGroup {
    indent: 16.0,
    border: 0.5,
    radius: 12.0,
    inset: 3.0,
};

/// `GROUP_MEMBERS_PAD_BOTTOM` (`sidebarMetrics.ts:17`).
pub const GROUP_MEMBERS_PAD_BOTTOM: f64 = SIDEBAR_GROUP.inset - SIDEBAR_GROUP.border;
