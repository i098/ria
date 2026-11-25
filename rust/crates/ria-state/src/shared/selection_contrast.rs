// Ported from ardia apps/browser/src/lib/selectionContrast.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Selection fills and lines that stay visible on their backdrops: a fill whose Oklab lightness is
//! too close to a backdrop's is replaced by an overlay of a fallback colour, raised in 1 % steps.
//!
//! Changes from Ardia: `Math.min(...list)` is a fold of `js_min` from `Infinity` (the value of
//! `Math.min()` for no backdrops), and `Math.round` is `js_round`.

use super::color::{over, parse_color, to_hex};
use super::js::{js_min, js_round};
use super::profile_look::{ThemeColor, oklab};

/// `MIN_SELECTION_LIGHTNESS` (`selectionContrast.ts:4`).
pub const MIN_SELECTION_LIGHTNESS: f64 = 0.06;

const ALPHA_STEP: f64 = 0.01;
const MAX_FALLBACK_ALPHA: f64 = 0.5;
const MAX_LINE_ALPHA: f64 = 0.3;

/// `lightness` (`selectionContrast.ts:10`): the Oklab lightness of an opaque colour.
fn lightness(color: &str) -> f64 {
    let [r, g, b, _] = parse_color(color);
    oklab(&ThemeColor {
        r: r / 255.0,
        g: g / 255.0,
        b: b / 255.0,
        a: 1.0,
    })[0]
}

/// `selectionLightnessDelta` (`selectionContrast.ts:15`): the smallest lightness step from a backdrop to
/// `fill` drawn over it.
pub fn selection_lightness_delta(fill: &str, backdrops: &[String]) -> f64 {
    backdrops.iter().fold(f64::INFINITY, |min, backdrop| {
        js_min(
            min,
            (lightness(&over(fill, backdrop)) - lightness(backdrop)).abs(),
        )
    })
}

/// `overlayAt` (`selectionContrast.ts:19`).
fn overlay_at(color: &str, alpha: f64) -> String {
    let [r, g, b, _] = parse_color(color);
    to_hex(&[r, g, b, js_round(alpha * 100.0) * 2.55])
}

/// `lifted` (`selectionContrast.ts:24`).
fn lifted(fill: &str, fallback: &str, backdrops: &[String], cap: f64) -> String {
    let passes =
        |color: &str| selection_lightness_delta(color, backdrops) >= MIN_SELECTION_LIGHTNESS;
    if backdrops.is_empty() || passes(fill) {
        return fill.to_owned();
    }
    let mut alpha = ALPHA_STEP;
    while alpha <= cap + 1e-9 {
        let shown = overlay_at(fallback, alpha);
        if passes(&shown) {
            return shown;
        }
        alpha += ALPHA_STEP;
    }
    overlay_at(fallback, cap)
}

/// `visibleSelection` (`selectionContrast.ts:34`).
pub fn visible_selection(fill: &str, fallback: &str, backdrops: &[String]) -> String {
    lifted(fill, fallback, backdrops, MAX_FALLBACK_ALPHA)
}

/// `visibleLine` (`selectionContrast.ts:36`).
pub fn visible_line(line: &str, ink: &str, backdrops: &[String]) -> String {
    lifted(line, ink, backdrops, MAX_LINE_ALPHA)
}
