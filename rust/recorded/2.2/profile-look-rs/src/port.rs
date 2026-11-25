// Ported from ardia apps/browser/src/lib/profileLook.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::profile_look`: themes cross as JSON (keys in the order
//! `presetStartingTheme` writes), numbers as JS numbers. The logic is in `ria_state::shared::profile_look`.
//! Arguments outside the TS types (for example a `color` that is not a colour name) are not covered:
//! the Rust side takes typed values, so the crate records with `--count 0` and only in-type inputs,
//! with the `null` and `undefined` cases where the TS really throws.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, number};
use ria_state::shared::profile_look::{
    self as p, ProfileTheme, ThemeColor, ThemeLook, ThemeLookSource, ThemeMode, ThemeStop,
    ThemeTexture,
};
use serde_json::{Value, json};

fn num(v: &Value, key: &str) -> f64 {
    v.get(key).and_then(as_number).unwrap_or(f64::NAN)
}

fn color_of(v: &Value) -> ThemeColor {
    ThemeColor {
        r: num(v, "r"),
        g: num(v, "g"),
        b: num(v, "b"),
        a: num(v, "a"),
    }
}

fn color_value(c: &ThemeColor) -> Value {
    json!({"r": number(c.r), "g": number(c.g), "b": number(c.b), "a": number(c.a)})
}

/// A `ThemeLook` from JSON: a number left out is `NaN`, a texture left out is grain.
fn look_of(v: &Value) -> ThemeLook {
    let stop = |s: &Value| ThemeStop {
        color: color_of(s.get("color").unwrap_or(&Value::Null)),
        x: num(s, "x"),
        y: num(s, "y"),
    };
    ThemeLook {
        stops: v
            .get("stops")
            .and_then(Value::as_array)
            .map(|s| s.iter().map(stop).collect())
            .unwrap_or_default(),
        intensity: num(v, "intensity"),
        grain: num(v, "grain"),
        texture: v
            .get("texture")
            .and_then(Value::as_str)
            .and_then(ThemeTexture::from_name)
            .unwrap_or(ThemeTexture::Grain),
        translucency: num(v, "translucency"),
        angle: v.get("angle").and_then(as_number),
    }
}

/// A `ProfileTheme` from JSON; `dark` is read when it is an object.
fn theme_of(v: &Value) -> ProfileTheme {
    ProfileTheme {
        look: look_of(v),
        mode: v
            .get("mode")
            .and_then(Value::as_str)
            .and_then(ThemeMode::from_name)
            .unwrap_or(ThemeMode::Auto),
        dark: v
            .get("dark")
            .filter(|d| d.is_object() && !is_undefined(d))
            .map(look_of),
    }
}

/// `!!v[key]`.
fn truthy(v: &Value, key: &str) -> bool {
    match v.get(key) {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => !s.is_empty(),
        Some(x) => as_number(x).map_or(!is_undefined(x), |n| n != 0.0 && !n.is_nan()),
    }
}

/// The `TypeError` of `v[key]` when `v` is `null` or `undefined` (`None` is a missing member).
fn readable(v: Option<&Value>, key: &str) -> Result<(), String> {
    let what = match v {
        Some(Value::Null) => "null",
        None => "undefined",
        Some(v) if is_undefined(v) => "undefined",
        Some(_) => return Ok(()),
    };
    Err(format!(
        "TypeError: Cannot read properties of {what} (reading '{key}')"
    ))
}

/// Port of `hexColor` (`apps/browser/src/lib/profileLook.ts:70`).
pub fn hex_color(hex: String) -> Result<serde_json::Value, String> {
    Ok(color_value(&p::hex_color(&hex)))
}

/// Port of `spaceLookSource` (`apps/browser/src/lib/profileLook.ts:75`).
pub fn space_look_source(settings: serde_json::Value) -> Result<String, String> {
    readable(Some(&settings), "spacesFollowAppearance")?;
    let source = p::space_look_source(truthy(&settings, "spacesFollowAppearance"));
    Ok(match source {
        ThemeLookSource::Own => "own",
        ThemeLookSource::App => "app",
        ThemeLookSource::Utility => "utility",
    }
    .into())
}

/// Port of `themeBackdrop` (`apps/browser/src/lib/profileLook.ts:79`).
pub fn theme_backdrop(theme: serde_json::Value, dark: bool) -> Result<serde_json::Value, String> {
    readable(Some(&theme), "stops")?;
    readable(theme.get("stops"), "slice")?;
    let b = p::theme_backdrop(&theme_of(&theme), dark);
    Ok(json!({
        "themeColors": b.theme_colors.iter().map(|c| number(*c)).collect::<Vec<_>>(),
        "themeOpacity": number(b.theme_opacity),
        "themeBaseAlpha": number(b.theme_base_alpha),
        "themeGrain": number(b.theme_grain),
        "themeTexture": b.theme_texture.name(),
        "themeAppearance": b.theme_appearance.name(),
        "themeAngle": number(b.theme_angle),
    }))
}
