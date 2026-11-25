// Ported from ardia apps/browser/src/lib/minimalismLook.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Minimalism mode's look: shorter sidebar rows, smaller corners and fewer chrome parts, unless the
//! theme set its own.
//!
//! Changes from Ardia: not ported: the `WeakMap` cache, which only keeps the object identity of the
//! result; `minimalism_look_of` gives an equal look on each call. `MINIMALISM_METRICS` is a
//! `RowMetrics` and `MINIMALISM_RADII` a `MinimalismRadii`. `hidden` keeps the `Set` rule: a part
//! that the look already hides is not added again.

use ria_core::identity::telemetry_wire::Json;

use super::tab_corners::radius_at;
use super::theme_look::{RADII, ROW_METRICS, RowMetrics, ThemeLook};

/// `MINIMALISM_METRICS` (`minimalismLook.ts:3`).
pub const MINIMALISM_METRICS: RowMetrics = RowMetrics {
    row_height: 28.0,
    row_gap: 2.0,
    pinned_height: 34.0,
};

/// The type of `MINIMALISM_RADII` (`minimalismLook.ts:4`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MinimalismRadii {
    pub chrome: f64,
    pub pinned: f64,
}

/// `MINIMALISM_RADII` (`minimalismLook.ts:4`).
pub const MINIMALISM_RADII: MinimalismRadii = MinimalismRadii {
    chrome: 9.0,
    pinned: 9.0,
};

/// `MINIMALISM_HIDDEN` (`minimalismLook.ts:5`).
pub const MINIMALISM_HIDDEN: [&str; 5] = [
    "addressField",
    "navButtons",
    "sidebarButton",
    "downloads",
    "profile",
];

/// `unlessThemed` (`minimalismLook.ts:9`): ours where the theme kept the base value.
fn unless_themed(theirs: f64, ours: f64, base: f64) -> f64 {
    if theirs == base { ours } else { theirs }
}

/// `minimalismLookOf` (`minimalismLook.ts:11`).
pub fn minimalism_look_of(look: &ThemeLook) -> ThemeLook {
    let (chrome, pinned) = (radius_at("chrome"), radius_at("pinned"));
    let mut radii = look.radii;
    radii[chrome] = unless_themed(look.radii[chrome], MINIMALISM_RADII.chrome, RADII[chrome].1);
    radii[pinned] = unless_themed(look.radii[pinned], MINIMALISM_RADII.pinned, RADII[pinned].1);
    let mut hidden = look.hidden.clone();
    for part in MINIMALISM_HIDDEN {
        if !hidden
            .iter()
            .any(|h| matches!(h, Json::String(s) if s == part))
        {
            hidden.push(Json::String(part.into()));
        }
    }
    let m = look.metrics;
    ThemeLook {
        metrics: RowMetrics {
            row_height: unless_themed(
                m.row_height,
                MINIMALISM_METRICS.row_height,
                ROW_METRICS.row_height,
            ),
            row_gap: unless_themed(m.row_gap, MINIMALISM_METRICS.row_gap, ROW_METRICS.row_gap),
            pinned_height: unless_themed(
                m.pinned_height,
                MINIMALISM_METRICS.pinned_height,
                ROW_METRICS.pinned_height,
            ),
        },
        radii,
        hidden,
        ..look.clone()
    }
}
