// Ported from ardia apps/browser/src/lib/color.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::color`: it gives the channels back as JS numbers
//! (`NaN` and `-0` cross as `$num`). The logic is in `ria_state::shared::color`.

#![forbid(unsafe_code)]

use crate::js::number;
use ria_state::shared::color as c;
use serde_json::{Value, json};

fn channels(v: &[f64]) -> Value {
    Value::Array(v.iter().map(|x| number(*x)).collect())
}

/// Port of `isHexColor` (`apps/browser/src/lib/color.ts:5`).
pub fn is_hex_color(color: String) -> Result<Value, String> {
    Ok(json!(c::is_hex_color(&color)))
}

/// Port of `isRgbFunction` (`apps/browser/src/lib/color.ts:7`).
pub fn is_rgb_function(color: String) -> Result<Value, String> {
    Ok(json!(c::is_rgb_function(&color)))
}

/// Port of `parseColor` (`apps/browser/src/lib/color.ts:9`).
pub fn parse_color(color: String) -> Result<Value, String> {
    Ok(channels(&c::parse_color(&color)))
}

/// Port of `toHex` (`apps/browser/src/lib/color.ts:21`).
pub fn to_hex(channels: Vec<f64>) -> Result<Value, String> {
    Ok(json!(c::to_hex(&channels)))
}

/// Port of `mix` (`apps/browser/src/lib/color.ts:24`).
pub fn mix(a: String, b: String, t: f64) -> Result<String, String> {
    Ok(c::mix(&a, &b, t))
}

/// Port of `over` (`apps/browser/src/lib/color.ts:30`).
pub fn over(top: String, under: String) -> Result<String, String> {
    Ok(c::over(&top, &under))
}

/// Port of `colorDistance` (`apps/browser/src/lib/color.ts:38`).
pub fn color_distance(a: String, b: String) -> Result<f64, String> {
    Ok(c::color_distance(&a, &b))
}

/// Port of `luminance` (`apps/browser/src/lib/color.ts:44`).
pub fn luminance(color: String) -> Result<f64, String> {
    Ok(c::luminance(&color))
}

/// Port of `contrastRatio` (`apps/browser/src/lib/color.ts:53`).
pub fn contrast_ratio(a: String, b: String) -> Result<f64, String> {
    Ok(c::contrast_ratio(&a, &b))
}

/// Port of `isLight` (`apps/browser/src/lib/color.ts:58`).
pub fn is_light(color: String) -> Result<bool, String> {
    Ok(c::is_light(&color))
}

/// Port of `hslToRgb` (`apps/browser/src/lib/color.ts:63`).
pub fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Result<Value, String> {
    Ok(channels(&c::hsl_to_rgb(h, s, l)))
}

/// Port of `rgbToHsl` (`apps/browser/src/lib/color.ts:74`). The arguments are not `r`, `g`, `b`:
/// the generated wire code names its reader `r`.
pub fn rgb_to_hsl(red: f64, green: f64, blue: f64) -> Result<Value, String> {
    Ok(channels(&c::rgb_to_hsl(red, green, blue)))
}
