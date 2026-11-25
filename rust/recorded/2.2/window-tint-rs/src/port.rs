// Ported from ardia apps/browser/src/lib/windowTint.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::window_tint`: a tint object crosses as JSON, its
//! numbers as JS numbers. The logic is in `ria_state::shared::window_tint`.
//! Arguments outside the TS types (for example a `color` that is not a colour name) are not covered:
//! the Rust side takes typed values, so the crate records with `--count 0` and only in-type inputs,
//! with the `null` and `undefined` cases where the TS really throws.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, number, string_units};
use ria_state::shared::window_tint as w;
use serde_json::{Value, json};

/// Port of `backdropTint` (`apps/browser/src/lib/windowTint.ts:11`).
pub fn backdrop_tint(color: String, neutral: bool) -> Result<serde_json::Value, String> {
    let t = w::backdrop_tint(&color, neutral);
    Ok(json!({
        "tintColor": t.tint_color,
        "tintAlpha": number(t.tint_alpha),
        "tintLightness": number(t.tint_lightness),
    }))
}

/// Port of `opaqueTint` (`apps/browser/src/lib/windowTint.ts:17`).
pub fn opaque_tint(tint: serde_json::Value, dark: bool) -> Result<serde_json::Value, String> {
    let what = |v: Option<&Value>| match v {
        Some(Value::Null) => Some("null"),
        None => Some("undefined"),
        Some(v) if is_undefined(v) => Some("undefined"),
        Some(_) => None,
    };
    // TS reads `tint.tintAlpha` first, then `parseColor` calls `trim` on `tintColor`.
    if let Some(what) = what(Some(&tint)) {
        return Err(format!(
            "TypeError: Cannot read properties of {what} (reading 'tintAlpha')"
        ));
    }
    let num = |key| tint.get(key).and_then(as_number).unwrap_or(f64::NAN);
    let color = tint.get("tintColor");
    let Some(units) = color.and_then(string_units) else {
        return Err(match what(color) {
            Some(what) => format!("TypeError: Cannot read properties of {what} (reading 'trim')"),
            None => "TypeError: color.trim is not a function".into(),
        });
    };
    let tint_color = String::from_utf16_lossy(&units);
    let tint = w::BackdropTint {
        tint_color,
        tint_alpha: num("tintAlpha"),
        tint_lightness: num("tintLightness"),
    };
    Ok(Value::from(w::opaque_tint(&tint, dark).to_vec()))
}

/// Port of `tintForHue` (`apps/browser/src/lib/windowTint.ts:26`).
pub fn tint_for_hue(swatch: String, grey: Option<bool>) -> Result<String, String> {
    Ok(w::tint_for_hue(&swatch, grey.unwrap_or(false)))
}
