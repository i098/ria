// Ported from ardia apps/browser/src/lib/tabCorners.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The Tab corners setting: the radius of the sidebar rows, chosen with a slider or left to the
//! theme.
//!
//! Changes from Ardia: `validTabCorners` takes `Option<&Json>`, `None` for `undefined`.
//! `with_tab_corners` gives a `Cow`: the look itself (borrowed) where Ardia returns the same object.
//! `tabCornersControl` returns a `TabCornersControl` struct.

use std::borrow::Cow;

use ria_core::identity::telemetry_wire::{Json, js_number_to_string};

use super::js::{js_max, js_min};
use super::theme_look::{RADII, ThemeLook};

/// The type of `TAB_CORNERS` (`tabCorners.ts:3`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabCorners {
    pub min: f64,
    pub max: f64,
}

/// `TAB_CORNERS` (`tabCorners.ts:3`).
pub const TAB_CORNERS: TabCorners = TabCorners {
    min: 0.0,
    max: 20.0,
};

/// The place of `radii.chrome` in `ThemeLook.radii`.
pub(crate) fn radius_at(name: &str) -> usize {
    RADII
        .iter()
        .position(|(n, _)| *n == name)
        .expect("a RADII name")
}

/// `validTabCorners` (`tabCorners.ts:5`): `null` or a whole number in range.
pub fn valid_tab_corners(value: Option<&Json>) -> bool {
    match value {
        Some(Json::Null) => true,
        Some(Json::Number(n)) => {
            n.is_finite() && n.fract() == 0.0 && *n >= TAB_CORNERS.min && *n <= TAB_CORNERS.max
        }
        _ => false,
    }
}

/// `withTabCorners` (`tabCorners.ts:8`).
pub fn with_tab_corners(look: &ThemeLook, corners: Option<f64>) -> Cow<'_, ThemeLook> {
    let chrome = radius_at("chrome");
    match corners {
        Some(corners) if corners != look.radii[chrome] => {
            let mut radii = look.radii;
            radii[chrome] = corners;
            Cow::Owned(ThemeLook {
                radii,
                ..look.clone()
            })
        }
        _ => Cow::Borrowed(look),
    }
}

/// The result of `tabCornersControl` (`tabCorners.ts:14`).
#[derive(Clone, Debug, PartialEq)]
pub struct TabCornersControl {
    pub position: f64,
    pub label: String,
    pub follows_theme: bool,
}

/// `tabCornersControl` (`tabCorners.ts:13`). `chosen` `None` is `null`: the theme decides.
pub fn tab_corners_control(in_force: f64, chosen: Option<f64>) -> TabCornersControl {
    TabCornersControl {
        position: js_min(1.0, js_max(0.0, in_force / TAB_CORNERS.max)),
        label: if in_force == 0.0 {
            "Square".into()
        } else {
            format!("{} pt", js_number_to_string(in_force))
        },
        follows_theme: chosen.is_none(),
    }
}
