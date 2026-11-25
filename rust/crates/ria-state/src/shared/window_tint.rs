// Ported from ardia apps/browser/src/lib/windowTint.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The tint of the window backdrop: its colour and strength, the opaque colours an inactive
//! window shows in its place, and the tint of a swatch's hue.
//!
//! Changes from Ardia: `grey = false` is a plain `bool`; `Math.min`, `Math.max` are `js_min`,
//! `js_max` to keep `NaN`.

use super::color::{hsl_to_rgb, parse_color, rgb_to_hsl, to_hex};
use super::js::{js_max, js_min};

/// `BackdropTint` (`windowTint.ts:3`).
#[derive(Clone, Debug, PartialEq)]
pub struct BackdropTint {
    pub tint_color: String,
    pub tint_alpha: f64,
    pub tint_lightness: f64,
}

const LIGHTNESS: f64 = 0.25;

/// `backdropTint` (`windowTint.ts:11`).
pub fn backdrop_tint(color: &str, neutral: bool) -> BackdropTint {
    BackdropTint {
        tint_color: color.to_owned(),
        tint_alpha: if neutral { 0.12 } else { 0.36 },
        tint_lightness: LIGHTNESS,
    }
}

/// `INACTIVE_FILL.dark` (`windowTint.ts:15`).
const INACTIVE_FILL_DARK: [f64; 3] = [40.7, 33.1, 31.8];
/// `INACTIVE_FILL.light` (`windowTint.ts:15`).
const INACTIVE_FILL_LIGHT: [f64; 3] = [235.0, 231.0, 230.0];

/// `opaqueTint` (`windowTint.ts:17`): the top and bottom colours of the tint over the inactive fill.
pub fn opaque_tint(tint: &BackdropTint, dark: bool) -> [String; 2] {
    let a = tint.tint_alpha * if dark { 0.5 } else { 0.75 };
    let under = if dark {
        INACTIVE_FILL_DARK.map(|f| 0.6 * f)
    } else {
        INACTIVE_FILL_LIGHT.map(|f| 0.8 * 255.0 + 0.2 * f)
    };
    let [r, g, b, _] = parse_color(&tint.tint_color);
    let top = [r, g, b];
    let bottom = lighten(top, tint.tint_lightness);
    let over = |c: [f64; 3]| to_hex(&[0, 1, 2].map(|i| a * c[i] + (1.0 - a) * under[i]));
    [over(top), over(bottom)]
}

/// `tintForHue` (`windowTint.ts:26`).
pub fn tint_for_hue(swatch: &str, grey: bool) -> String {
    let [r, g, b, _] = parse_color(swatch);
    let [h, ..] = hsl([r, g, b]);
    to_hex(&from_hsl(h, if grey { 0.0 } else { 0.36 }, 0.527))
}

/// `hsl` (`windowTint.ts:31`).
fn hsl([r, g, b]: [f64; 3]) -> [f64; 3] {
    rgb_to_hsl(r / 255.0, g / 255.0, b / 255.0)
}

/// `fromHsl` (`windowTint.ts:32`).
fn from_hsl(h: f64, s: f64, l: f64) -> [f64; 3] {
    hsl_to_rgb(h, s, l).map(|v| v * 255.0)
}

/// `lighten` (`windowTint.ts:34`).
fn lighten(c: [f64; 3], delta: f64) -> [f64; 3] {
    let [h, s, l] = hsl(c);
    from_hsl(h, s, js_min(1.0, js_max(0.0, l + delta)))
}
