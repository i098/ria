// Ported from ardia apps/browser/src/lib/presetLook.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::preset_look`: profiles and themes cross as JSON, numbers
//! as JS numbers. A profile keeps its other fields; a colour name that is not a `ProfileColor` is an
//! error here (TS takes any string), and such a key of `presetThemes` is left out.
//! `trackPress` stays in TS; `preset-look-harness-rs` replays it. The logic is in
//! `ria_state::shared::preset_look`.
//! Arguments outside the TS types (for example a `color` that is not a colour name) are not covered:
//! the Rust side takes typed values, so the crate records with `--count 0` and only in-type inputs,
//! with the `null` and `undefined` cases where the TS really throws.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, number, undefined};
use ria_state::shared::preset_look as p;
use ria_state::shared::profile_look::{
    PresetLookFields, PresetThemes, ProfileColor, ProfileTheme, ThemeColor, ThemeLook, ThemeMode,
    ThemeStop, ThemeTexture,
};
use serde_json::{Map, Value, json};

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

/// A theme as JSON, the keys in the order `presetStartingTheme` writes them.
fn look_fields(l: &ThemeLook, mode: Option<ThemeMode>) -> Map<String, Value> {
    let stops = l
        .stops
        .iter()
        .map(|s| json!({"x": number(s.x), "y": number(s.y), "color": color_value(&s.color)}));
    let mut m = Map::new();
    m.insert("stops".into(), Value::Array(stops.collect()));
    m.insert("intensity".into(), number(l.intensity));
    if let Some(mode) = mode {
        m.insert("mode".into(), mode.name().into());
    }
    m.insert("grain".into(), number(l.grain));
    m.insert("texture".into(), l.texture.name().into());
    m.insert("translucency".into(), number(l.translucency));
    if let Some(a) = l.angle {
        m.insert("angle".into(), number(a));
    }
    m
}

fn theme_value(t: &ProfileTheme) -> Value {
    let mut m = look_fields(&t.look, Some(t.mode));
    if let Some(d) = &t.dark {
        m.insert("dark".into(), Value::Object(look_fields(d, None)));
    }
    Value::Object(m)
}

/// `v ?? null` as a theme: `None` for `null` and `undefined`.
fn maybe_theme(v: Option<&Value>) -> Option<ProfileTheme> {
    v.filter(|t| !t.is_null() && !is_undefined(t)).map(theme_of)
}

fn theme_or_null(t: Option<&ProfileTheme>) -> Value {
    t.map_or(Value::Null, theme_value)
}

fn color_of_value(v: Option<&Value>) -> Result<ProfileColor, String> {
    v.and_then(Value::as_str)
        .and_then(ProfileColor::from_name)
        .ok_or_else(|| "TypeError: not a ProfileColor".into())
}

fn looks_of(v: &Value) -> PresetThemes {
    v.as_object()
        .map(|o| {
            o.iter()
                .filter_map(|(k, t)| Some((ProfileColor::from_name(k)?, theme_of(t))))
                .collect()
        })
        .unwrap_or_default()
}

fn looks_value(looks: &PresetThemes) -> Value {
    Value::Object(
        looks
            .iter()
            .map(|(c, t)| (c.name().into(), theme_value(t)))
            .collect(),
    )
}

/// The `TypeError` of `v[key]` when `v` is `null` or `undefined`.
fn readable(v: &Value, key: &str) -> Result<(), String> {
    let what = match v {
        Value::Null => "null",
        _ if is_undefined(v) => "undefined",
        _ => return Ok(()),
    };
    Err(format!(
        "TypeError: Cannot read properties of {what} (reading '{key}')"
    ))
}

/// The fields of `profile`; `first` is the property TS reads first.
fn fields_of(profile: &Value, first: &str) -> Result<PresetLookFields, String> {
    readable(profile, first)?;
    Ok(PresetLookFields {
        color: color_of_value(profile.get("color"))?,
        theme: maybe_theme(profile.get("theme")),
        preset_themes: profile
            .get("presetThemes")
            .filter(|v| !v.is_null() && !is_undefined(v))
            .map(looks_of),
    })
}

fn point_of(v: &Value) -> (f64, f64) {
    (num(v, "x"), num(v, "y"))
}

/// Port of `presetStartingTheme` (`apps/browser/src/lib/presetLook.ts:3`).
pub fn preset_starting_theme(swatch: String) -> Result<serde_json::Value, String> {
    Ok(theme_value(&p::preset_starting_theme(&swatch)))
}

/// Port of `presetThemeOf` (`apps/browser/src/lib/presetLook.ts:10`).
pub fn preset_theme_of(
    profile: serde_json::Value,
    color: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let t = p::preset_theme_of(
        &fields_of(&profile, "color")?,
        color_of_value(Some(&color))?,
    );
    Ok(t.as_ref().map(theme_value))
}

/// Port of `snapToStart` (`apps/browser/src/lib/presetLook.ts:31`).
pub fn snap_to_start(value: f64, start: f64, track_pixels: f64) -> Result<f64, String> {
    Ok(p::snap_to_start(value, start, track_pixels))
}

/// Port of `snapPointToStart` (`apps/browser/src/lib/presetLook.ts:35`): a point that does not snap is
/// given back as it came.
pub fn snap_point_to_start(
    point: serde_json::Value,
    start: serde_json::Value,
    pad_pixels: f64,
) -> Result<serde_json::Value, String> {
    readable(&point, "x")?;
    readable(&start, "x")?;
    let ((px, py), (sx, sy)) = (point_of(&point), point_of(&start));
    let (x, y) = p::snap_point_to_start((px, py), (sx, sy), pad_pixels);
    // TS gives a new `{x, y}` when it snaps, even to an equal point: the snap test, not the value, decides.
    let near = |a: f64, b: f64| (a - b).abs() * pad_pixels <= 0.5;
    let kept = !(near(px, sx) && near(py, sy));
    Ok(match kept {
        true => point,
        false => json!({"x": number(x), "y": number(y)}),
    })
}

/// Port of `isUntouched` (`apps/browser/src/lib/presetLook.ts:39`).
pub fn is_untouched(theme: serde_json::Value, start: serde_json::Value) -> Result<bool, String> {
    readable(&theme, "dark")?;
    Ok(p::is_untouched(&theme_of(&theme), &theme_of(&start)))
}

/// Port of `sameTheme` (`apps/browser/src/lib/presetLook.ts:43`).
pub fn same_theme(
    a: Option<serde_json::Value>,
    b: Option<serde_json::Value>,
) -> Result<bool, String> {
    Ok(p::same_theme(
        maybe_theme(a.as_ref()).as_ref(),
        maybe_theme(b.as_ref()).as_ref(),
    ))
}

/// Port of `withOwnLook` (`apps/browser/src/lib/presetLook.ts:51`).
pub fn with_own_look(
    profile: serde_json::Value,
    theme: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let own = p::with_own_look(
        &fields_of(&profile, "presetThemes")?,
        maybe_theme(Some(&theme)),
    );
    // TS gives `theme` back as it came, so `undefined` stays `undefined`.
    let theme = match is_undefined(&theme) {
        true => theme,
        false => theme_or_null(own.theme.as_ref()),
    };
    Ok(json!({"theme": theme, "presetThemes": looks_value(&own.preset_themes)}))
}

/// Port of `withPresetChosen` (`apps/browser/src/lib/presetLook.ts:58`).
pub fn with_preset_chosen(
    profile: serde_json::Value,
    color: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let choice = p::with_preset_chosen(
        &fields_of(&profile, "color")?,
        color_of_value(Some(&color))?,
    );
    Ok(choice.map(|c| json!({"color": c.color.name(), "theme": theme_or_null(c.theme.as_ref())})))
}

/// Port of `withSyncedColor` (`apps/browser/src/lib/presetLook.ts:65`).
pub fn with_synced_color(
    profile: serde_json::Value,
    color: serde_json::Value,
    synced: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let synced_theme = Some(&synced)
        .filter(|s| !is_undefined(s))
        .map(|s| maybe_theme(Some(s)));
    let out = p::with_synced_color(
        &fields_of(&profile, "color")?,
        color_of_value(Some(&color))?,
        synced_theme,
    );
    Ok(out.map(|s| {
        let theme = s
            .theme
            .map_or_else(undefined, |t| theme_or_null(t.as_ref()));
        json!({"color": s.color.name(), "theme": theme})
    }))
}

/// Port of `editedPresetCount` (`apps/browser/src/lib/presetLook.ts:69`).
pub fn edited_preset_count(profile: serde_json::Value) -> Result<f64, String> {
    Ok(p::edited_preset_count(&fields_of(&profile, "presetThemes")?) as f64)
}

fn seeded_of(profile: &Value) -> Result<p::SeededProfile, String> {
    let seen_theme = profile.get("seenTheme").filter(|v| !is_undefined(v));
    let seen_color = profile.get("seenColor").filter(|v| !is_undefined(v));
    Ok(p::SeededProfile {
        fields: fields_of(profile, "theme")?,
        seen_theme: seen_theme.map(|v| maybe_theme(Some(v))),
        seen_color: seen_color.map(|v| color_of_value(Some(v))).transpose()?,
    })
}

/// The profile `seeded` with the fields `withOwnLooksSeeded` writes, its other fields as they came.
fn seeded_value(mut profile: Value, seeded: &p::SeededProfile) -> Value {
    let Some(m) = profile.as_object_mut() else {
        return profile;
    };
    if let Some(looks) = &seeded.fields.preset_themes {
        m.insert("presetThemes".into(), looks_value(looks));
    }
    if let Some(t) = &seeded.seen_theme {
        m.insert("seenTheme".into(), theme_or_null(t.as_ref()));
    }
    if let Some(c) = seeded.seen_color {
        m.insert("seenColor".into(), c.name().into());
    }
    profile
}

/// Port of `withOwnLooksSeeded` (`apps/browser/src/lib/presetLook.ts:73`).
pub fn with_own_looks_seeded(profile: serde_json::Value) -> Result<serde_json::Value, String> {
    let seeded = p::with_own_looks_seeded(&seeded_of(&profile)?);
    Ok(seeded_value(profile, &seeded))
}

/// Port of `profilesWithOwnLooks` (`apps/browser/src/lib/presetLook.ts:84`).
pub fn profiles_with_own_looks(profiles: serde_json::Value) -> Result<serde_json::Value, String> {
    if profiles.is_null() || is_undefined(&profiles) {
        return Err("TypeError: Cannot convert undefined or null to object".into());
    }
    let Value::Object(entries) = profiles else {
        return Ok(json!({}));
    };
    let list = entries
        .iter()
        .map(|(id, v)| Ok((id.clone(), seeded_of(v)?)))
        .collect::<Result<Vec<_>, String>>()?;
    let out = p::profiles_with_own_looks(&list);
    Ok(Value::Object(
        entries
            .into_iter()
            .zip(out)
            .map(|((id, v), (_, s))| (id, seeded_value(v, &s)))
            .collect(),
    ))
}

/// Port of `trackValueAt` (`apps/browser/src/lib/presetLook.ts:100`).
pub fn track_value_at(
    page_x: f64,
    track_x: f64,
    track_pixels: f64,
    snap_to: Option<f64>,
) -> Result<Option<f64>, String> {
    Ok(p::track_value_at(page_x, track_x, track_pixels, snap_to))
}
