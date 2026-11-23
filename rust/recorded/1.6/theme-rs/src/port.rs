// Ported from ardia packages/core/src/theme.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `identity::theme`: it reads the JSON arguments of the
//! recording as JS does (`undefined` fields, `ToNumber`, spreads that keep unknown keys) and
//! gives the original's JS errors. The logic is in `ria_core::identity::theme`.

#![forbid(unsafe_code)]

use crate::js;
use ria_core::identity::theme::{
    self as t, ProfileTheme, RawColor, RawLook, RawStop, RawTheme, ThemeColor, ThemeLook,
    ThemeLookSource, ThemeMode, ThemeStop, ThemeTexture,
};
use serde_json::{Map, Value, json};

type Res<T> = Result<T, String>;

/// `v[key]` as JS reads it: `None` for `undefined` (missing, or `v` not an object).
fn get<'v>(v: &'v Value, key: &str) -> Option<&'v Value> {
    v.as_object()?.get(key).filter(|x| !js::is_undefined(x))
}

fn is_nullish(v: &Value) -> bool {
    v.is_null() || js::is_undefined(v)
}

fn read_error(v: Option<&Value>, prop: &str) -> String {
    let of = if v.is_some_and(Value::is_null) {
        "null"
    } else {
        "undefined"
    };
    format!("TypeError: Cannot read properties of {of} (reading '{prop}')")
}

/// `v.prop` throws on `null` and `undefined`.
fn object<'v>(v: &'v Value, prop: &str) -> Res<&'v Value> {
    if is_nullish(v) {
        Err(read_error(Some(v), prop))
    } else {
        Ok(v)
    }
}

fn is_js_space(c: char) -> bool {
    matches!(c, '\t'..='\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}'
        | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

/// JS `Number(string)`.
fn string_to_number(s: &str) -> f64 {
    let s = s.trim_matches(is_js_space);
    let radix = |digits: &str, r: u32| {
        if digits.is_empty() {
            return f64::NAN;
        }
        digits
            .chars()
            .try_fold(0.0, |n, c| {
                Some(n * f64::from(r) + f64::from(c.to_digit(r)?))
            })
            .unwrap_or(f64::NAN)
    };
    match s {
        "" => 0.0,
        "Infinity" | "+Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        _ if s.starts_with("0x") || s.starts_with("0X") => radix(&s[2..], 16),
        _ if s.starts_with("0o") || s.starts_with("0O") => radix(&s[2..], 8),
        _ if s.starts_with("0b") || s.starts_with("0B") => radix(&s[2..], 2),
        // Rust also reads `inf` and `nan`: only decimal-literal characters go to `parse`.
        _ if s
            .bytes()
            .all(|b| b.is_ascii_digit() || b"+-.eE".contains(&b)) =>
        {
            s.parse().unwrap_or(f64::NAN)
        }
        _ => f64::NAN,
    }
}

/// JS `ToNumber`; `None` is `undefined`.
fn to_number(v: Option<&Value>) -> f64 {
    let Some(v) = v else { return f64::NAN };
    if let Some(n) = js::as_number(v) {
        return n;
    }
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::String(s) => string_to_number(s),
        // An array goes through its string: `[]` is "", `[x]` is `String(x)`, more have a comma.
        Value::Array(items) => match items.as_slice() {
            [] => 0.0,
            [x] => match x {
                Value::Null => 0.0,
                Value::Bool(_) => f64::NAN,
                _ if js::is_undefined(x) => 0.0,
                _ => to_number(Some(x)) + 0.0,
            },
            _ => f64::NAN,
        },
        _ => f64::NAN,
    }
}

fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => !s.is_empty(),
        Some(x) => js::as_number(x).map_or(!js::is_undefined(x), |n| n != 0.0 && !n.is_nan()),
    }
}

/// `typeof v === "object" && v`: an object or an array.
fn is_object(v: &Value) -> bool {
    match v {
        Value::Array(_) => true,
        Value::Object(_) => {
            js::as_number(v).is_none() && js::string_units(v).is_none() && !js::is_undefined(v)
        }
        _ => false,
    }
}

/// `{ ...v }`.
fn spread(v: &Value) -> Map<String, Value> {
    match v {
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(i, x)| (i.to_string(), x.clone()))
            .collect(),
        _ if is_object(v) => v.as_object().cloned().unwrap_or_default(),
        _ => match js::string_units(v) {
            Some(units) => units
                .iter()
                .enumerate()
                .map(|(i, u)| (i.to_string(), js::utf16_string(&[*u])))
                .collect(),
            None => Map::new(),
        },
    }
}

fn color_of(v: &Value) -> ThemeColor {
    let n = |k| to_number(get(v, k));
    ThemeColor {
        r: n("r"),
        g: n("g"),
        b: n("b"),
        a: n("a"),
    }
}

/// A color argument: reading `r` throws on `null` and `undefined`.
fn color(v: Option<&Value>) -> Res<ThemeColor> {
    match v {
        Some(v) if !is_nullish(v) => Ok(color_of(v)),
        _ => Err(read_error(v, "r")),
    }
}

fn stop_of(v: &Value) -> ThemeStop {
    ThemeStop {
        color: get(v, "color").map_or(color_of(&Value::Null), color_of),
        x: to_number(get(v, "x")),
        y: to_number(get(v, "y")),
    }
}

fn raw_stops<'v>(theme: &'v Value, method: &str) -> Res<&'v Vec<Value>> {
    get(theme, "stops")
        .and_then(Value::as_array)
        .ok_or_else(|| read_error(None, method))
}

/// `TEXTURE_STRENGTH[texture]` finds a strength only for a texture name (or an array of one).
fn texture_key(v: Option<&Value>) -> Option<ThemeTexture> {
    match v? {
        Value::String(s) => ThemeTexture::from_name(s),
        Value::Array(items) if items.len() == 1 => texture_key(items.first()),
        _ => None,
    }
}

fn look(v: &Value) -> ThemeLook {
    let n = |k| to_number(get(v, k));
    ThemeLook {
        stops: get(v, "stops")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |s| s.iter().map(stop_of).collect()),
        intensity: n("intensity"),
        grain: n("grain"),
        texture: texture_key(get(v, "texture")).unwrap_or(ThemeTexture::Grain),
        translucency: n("translucency"),
        angle: get(v, "angle")
            .filter(|a| !a.is_null())
            .map(|a| to_number(Some(a))),
    }
}

/// `{ ...light, ...darkLook }` with the light angle dropped: the look JS shows when dark.
fn dark_merged(theme: &Value, dark: &Value) -> Map<String, Value> {
    let mut m = spread(theme);
    m.shift_remove("dark");
    m.shift_remove("angle");
    m.extend(spread(dark));
    m
}

fn profile(v: &Value) -> ProfileTheme {
    ProfileTheme {
        look: look(v),
        // `themeDark` and `shownLook` treat any mode but "auto" and "dark" as light.
        mode: match get(v, "mode").and_then(Value::as_str) {
            Some("auto") => ThemeMode::Auto,
            Some("dark") => ThemeMode::Dark,
            _ => ThemeMode::Light,
        },
        dark: get(v, "dark")
            .filter(|d| truthy(Some(d)))
            .map(|d| look(&Value::Object(dark_merged(v, d)))),
    }
}

/// A look `themeArea` paints: `first` is the property read first, so the one a `null` fails on.
fn painted(theme: &Value, first: &str) -> Res<ThemeLook> {
    object(theme, first)?;
    if raw_stops(theme, "map")?.is_empty() {
        return Err(read_error(None, "r"));
    }
    Ok(look(theme))
}

fn source_of(source: &str) -> ThemeLookSource {
    match source {
        "own" => ThemeLookSource::Own,
        "utility" => ThemeLookSource::Utility,
        _ => ThemeLookSource::App,
    }
}

fn color_json(c: &ThemeColor) -> Value {
    json!({"r": js::number(c.r), "g": js::number(c.g), "b": js::number(c.b), "a": js::number(c.a)})
}

fn rgb_json(c: [f64; 3]) -> Value {
    Value::Array(c.map(js::number).to_vec())
}

/// Sets `key` to `n` unless it already holds that number, so equal numbers keep their JSON.
fn put_number(m: &mut Map<String, Value>, key: &str, n: f64) {
    let same = m
        .get(key)
        .and_then(js::as_number)
        .is_some_and(|o| o.to_bits() == n.to_bits());
    if !same {
        m.insert(key.into(), js::number(n));
    }
}

/// `{ ...s, color: { ...s.color, r, g, b } }`.
fn stop_with_rgb(s: &Value, c: &ThemeColor) -> Value {
    let mut stop = spread(s);
    let mut col = get(s, "color").map(spread).unwrap_or_default();
    put_number(&mut col, "r", c.r);
    put_number(&mut col, "g", c.g);
    put_number(&mut col, "b", c.b);
    stop.insert("color".into(), Value::Object(col));
    Value::Object(stop)
}

/// `{ ...padColor(..), a: s.color.a }`.
fn padded_color(c: &ThemeColor, s: &Value) -> Value {
    let a = get(s, "color")
        .and_then(|c| get(c, "a"))
        .cloned()
        .unwrap_or_else(js::undefined);
    json!({"r": js::number(c.r), "g": js::number(c.g), "b": js::number(c.b), "a": a})
}

/// `{ ...theme, stops }`.
fn with_stops(theme: &Value, stops: Vec<Value>) -> Value {
    let mut out = spread(theme);
    out.insert("stops".into(), Value::Array(stops));
    Value::Object(out)
}

fn look_json(l: &ThemeLook) -> Map<String, Value> {
    let mut m = Map::new();
    let stops = l.stops.iter().map(
        |s| json!({"color": color_json(&s.color), "x": js::number(s.x), "y": js::number(s.y)}),
    );
    m.insert("stops".into(), Value::Array(stops.collect()));
    m.insert("intensity".into(), js::number(l.intensity));
    m.insert("grain".into(), js::number(l.grain));
    m.insert("texture".into(), l.texture.name().into());
    m.insert("translucency".into(), js::number(l.translucency));
    if let Some(a) = l.angle {
        m.insert("angle".into(), js::number(a));
    }
    m
}

/// Port of `padColor` (`packages/core/src/theme.ts:80`).
pub fn pad_color(
    x: f64,
    y: f64,
    lightness: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let l = if js::is_undefined(&lightness) {
        t::PAD_LIGHTNESS
    } else {
        to_number(Some(&lightness))
    };
    Ok(color_json(&t::pad_color(x, y, l)))
}

/// Port of `padPosition` (`packages/core/src/theme.ts:89`).
pub fn pad_position(color: serde_json::Value) -> Result<serde_json::Value, String> {
    let (x, y) = t::pad_position(&self::color(Some(&color))?);
    Ok(json!({"x": js::number(x), "y": js::number(y)}))
}

/// Port of `oklab` (`packages/core/src/theme.ts:98`).
pub fn oklab(c: serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(rgb_json(t::oklab(&color(Some(&c))?)))
}

/// Port of `fromOklab` (`packages/core/src/theme.ts:110`): the argument is destructured as an iterable.
pub fn from_oklab(arg0: serde_json::Value) -> Result<serde_json::Value, String> {
    let items: Vec<f64> = if let Value::Array(items) = &arg0 {
        items
            .iter()
            .map(|x| to_number(Some(x).filter(|x| !js::is_undefined(x))))
            .collect()
    } else if let Some(units) = js::string_units(&arg0) {
        char::decode_utf16(units)
            .map(|c| c.map_or(f64::NAN, |c| string_to_number(&c.to_string())))
            .collect()
    } else {
        let what = match &arg0 {
            Value::Null => "object null".to_string(),
            Value::Bool(b) => format!("boolean {b}"),
            _ if js::is_undefined(&arg0) => "undefined".to_string(),
            _ => match js::as_number(&arg0) {
                Some(n) => format!("number {}", js::number_to_string(n)),
                None => "object".to_string(),
            },
        };
        return Err(format!(
            "TypeError: {what} is not iterable (cannot read property Symbol(Symbol.iterator))"
        ));
    };
    let at = |i: usize| items.get(i).copied().unwrap_or(f64::NAN);
    // `L + 0.39.. * A + 0.21.. * B` concatenates when `L` is a string; the later `L - ..` do not.
    // shortcut: only the NaN of a non-numeric concatenation is kept, a numeric one ("1" + "0" + "0") is not.
    let first = match &arg0 {
        Value::Array(items) => items.first().and_then(Value::as_str).map(str::to_string),
        _ => js::string_units(&arg0)
            .and_then(|u| char::decode_utf16(u).next()?.ok())
            .map(String::from),
    };
    let l = match first {
        Some(l) => {
            let joined = format!(
                "{l}{}{}",
                js::number_to_string(0.3963377774 * at(1)),
                js::number_to_string(0.2158037573 * at(2))
            );
            if string_to_number(&joined).is_nan() {
                f64::NAN
            } else {
                at(0)
            }
        }
        None => at(0),
    };
    Ok(rgb_json(t::from_oklab([l, at(1), at(2)])))
}

/// Port of `colorHex` (`packages/core/src/theme.ts:121`).
pub fn color_hex(color: serde_json::Value) -> Result<String, String> {
    Ok(t::color_hex(&self::color(Some(&color))?))
}

/// Port of `hexColor` (`packages/core/src/theme.ts:125`).
pub fn hex_color(hex: String) -> Result<serde_json::Value, String> {
    Ok(color_json(&t::hex_color(&hex)))
}

/// Port of `mixOklab` (`packages/core/src/theme.ts:132`).
pub fn mix_oklab(
    a: serde_json::Value,
    b: serde_json::Value,
    t: f64,
) -> Result<serde_json::Value, String> {
    let (a, b) = (color(Some(&a))?, color(Some(&b))?);
    Ok(rgb_json(self::t::mix_oklab(&a, &b, t)))
}

/// Port of `lookAngle` (`packages/core/src/theme.ts:137`).
pub fn look_angle(theme: serde_json::Value) -> Result<f64, String> {
    object(&theme, "angle")?;
    Ok(t::look_angle(look(&theme).angle))
}

/// Port of `themeArea` (`packages/core/src/theme.ts:156`).
pub fn theme_area(
    theme: serde_json::Value,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<serde_json::Value, String> {
    Ok(rgb_json(t::theme_area(
        &painted(&theme, "stops")?,
        x,
        y,
        width,
        height,
    )))
}

/// Port of `themeDark` (`packages/core/src/theme.ts:166`).
pub fn theme_dark(theme: serde_json::Value, app_dark: bool) -> Result<bool, String> {
    object(&theme, "mode")?;
    Ok(t::theme_dark(&profile(&theme), app_dark))
}

/// `resolveTheme` on the JSON, as its spreads do: `ria_core` `resolve_theme` is the typed form.
fn resolved_json(theme: &Value, dark: bool) -> Res<Map<String, Value>> {
    if is_nullish(theme) {
        let what = if theme.is_null() { "null" } else { "undefined" };
        return Err(format!(
            "TypeError: Cannot destructure property 'dark' of 'theme' as it is {what}."
        ));
    }
    match get(theme, "dark") {
        Some(d) if dark && truthy(Some(d)) => Ok(dark_merged(theme, d)),
        _ => {
            let mut m = spread(theme);
            m.shift_remove("dark");
            Ok(m)
        }
    }
}

/// Port of `resolveTheme` (`packages/core/src/theme.ts:170`).
pub fn resolve_theme(theme: serde_json::Value, dark: bool) -> Result<serde_json::Value, String> {
    resolved_json(&theme, dark).map(Value::Object)
}

/// Port of `darkenedLook` (`packages/core/src/theme.ts:177`).
pub fn darkened_look(
    theme: serde_json::Value,
    keep: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let keep = if js::is_undefined(&keep) {
        t::DARKENED_LOOK_KEEP
    } else {
        to_number(Some(&keep))
    };
    object(&theme, "stops")?;
    let raw = raw_stops(&theme, "map")?;
    let dark = t::darkened_look(&profile(&theme), keep);
    Ok(with_stops(
        &theme,
        raw.iter()
            .zip(&dark.look.stops)
            .map(|(s, d)| stop_with_rgb(s, &d.color))
            .collect(),
    ))
}

/// Port of `requestedDarkFor` (`packages/core/src/theme.ts:181`).
pub fn requested_dark_for(
    theme: serde_json::Value,
    app_dark: bool,
    source: String,
) -> Result<bool, String> {
    let source = source_of(&source);
    if source == ThemeLookSource::Own {
        object(&theme, "mode")?;
    }
    Ok(t::requested_dark_for(&profile(&theme), app_dark, source))
}

/// Port of `shownLook` (`packages/core/src/theme.ts:185`).
pub fn shown_look(
    theme: serde_json::Value,
    app_dark: bool,
    source: String,
) -> Result<serde_json::Value, String> {
    let source = source_of(&source);
    if source == ThemeLookSource::Own {
        object(&theme, "mode")?;
    }
    let p = profile(&theme);
    let requested_dark = t::requested_dark_for(&p, app_dark, source);
    let resolved = Value::Object(resolved_json(&theme, requested_dark)?);
    // The original measures the resolved look (which throws without stops) in these cases.
    let light_only = p.mode != ThemeMode::Dark && p.dark.is_none();
    if (source != ThemeLookSource::Own && requested_dark && light_only)
        || source != ThemeLookSource::Utility
    {
        painted(&resolved, "stops")?;
    }
    let shown = t::shown_look(&p, app_dark, source);
    let raw = raw_stops(&resolved, "map")
        .map(Vec::as_slice)
        .unwrap_or_default();
    let stops: Vec<Value> = raw
        .iter()
        .zip(&shown.look.look.stops)
        .map(|(s, d)| stop_with_rgb(s, &d.color))
        .collect();
    let look = if stops.is_empty() {
        resolved
    } else {
        with_stops(&resolved, stops)
    };
    Ok(json!({"look": look, "requestedDark": shown.requested_dark, "dark": shown.dark}))
}

/// Port of `arcLayerAlpha` (`packages/core/src/theme.ts:194`).
pub fn arc_layer_alpha(intensity: f64) -> Result<f64, String> {
    Ok(t::arc_layer_alpha(intensity))
}

/// Port of `themeLayerAlpha` (`packages/core/src/theme.ts:198`).
pub fn theme_layer_alpha(theme: serde_json::Value) -> Result<f64, String> {
    object(&theme, "translucency")?;
    Ok(t::theme_layer_alpha(&look(&theme)))
}

/// Port of `themeUnderlayAlpha` (`packages/core/src/theme.ts:202`).
pub fn theme_underlay_alpha(theme: serde_json::Value, dark: bool) -> Result<f64, String> {
    object(&theme, "intensity")?;
    Ok(t::theme_underlay_alpha(&look(&theme), dark))
}

/// Port of `themeGrainAlpha` (`packages/core/src/theme.ts:207`).
pub fn theme_grain_alpha(theme: serde_json::Value) -> Result<f64, String> {
    object(&theme, "grain")?;
    // Another texture has no strength: `clamp01(grain) * undefined`.
    Ok(match texture_key(get(&theme, "texture")) {
        Some(_) => t::theme_grain_alpha(&look(&theme)),
        None => f64::NAN,
    })
}

/// Port of `flatColors` (`packages/core/src/theme.ts:211`).
pub fn flat_colors(theme: serde_json::Value) -> Result<Vec<f64>, String> {
    object(&theme, "stops")?;
    raw_stops(&theme, "slice")?;
    Ok(t::flat_colors(&look(&theme)))
}

/// Port of `themeAccent` (`packages/core/src/theme.ts:215`).
pub fn theme_accent(theme: serde_json::Value) -> Result<String, String> {
    Ok(t::theme_accent(&painted(&theme, "stops")?))
}

/// Port of `themePalette` (`packages/core/src/theme.ts:219`).
pub fn theme_palette(theme: serde_json::Value) -> Result<Vec<String>, String> {
    object(&theme, "stops")?;
    raw_stops(&theme, "map")?;
    Ok(t::theme_palette(&look(&theme)))
}

/// Port of `backdropUnderColor` (`packages/core/src/theme.ts:233`).
pub fn backdrop_under_color(underlay_alpha: f64, dark: bool) -> Result<String, String> {
    Ok(t::backdrop_under_color(underlay_alpha, dark))
}

/// Port of `themeUnderColor` (`packages/core/src/theme.ts:238`).
pub fn theme_under_color(theme: serde_json::Value, dark: bool) -> Result<String, String> {
    object(&theme, "intensity")?;
    Ok(t::theme_under_color(&look(&theme), dark))
}

/// Port of `opaqueThemeTint` (`packages/core/src/theme.ts:248`).
pub fn opaque_theme_tint(
    theme: serde_json::Value,
    dark: bool,
) -> Result<serde_json::Value, String> {
    Ok(json!(t::opaque_theme_tint(
        &painted(&theme, "translucency")?,
        dark
    )))
}

/// Port of `themeLuminance` (`packages/core/src/theme.ts:257`).
pub fn theme_luminance(theme: serde_json::Value, dark: bool) -> Result<f64, String> {
    Ok(t::theme_luminance(&painted(&theme, "translucency")?, dark))
}

/// Port of `readableDark` (`packages/core/src/theme.ts:263`).
pub fn readable_dark(theme: serde_json::Value, dark: bool) -> Result<bool, String> {
    Ok(t::readable_dark(&painted(&theme, "translucency")?, dark))
}

/// Port of `stopLightness` (`packages/core/src/theme.ts:270`).
pub fn stop_lightness(stop: serde_json::Value) -> Result<f64, String> {
    object(&stop, "color")?;
    color(get(&stop, "color"))?;
    Ok(t::stop_lightness(&stop_of(&stop)))
}

/// Port of `withStopAt` (`packages/core/src/theme.ts:274`).
pub fn with_stop_at(
    theme: serde_json::Value,
    index: f64,
    x: f64,
    y: f64,
) -> Result<serde_json::Value, String> {
    object(&theme, "stops")?;
    let raw = raw_stops(&theme, "map")?;
    let moved = t::with_stop_at(&look(&theme), index, x, y);
    let stops = raw.iter().zip(&moved.stops).enumerate().map(|(i, (s, m))| {
        if i as f64 == index {
            json!({"x": js::number(m.x), "y": js::number(m.y), "color": padded_color(&m.color, s)})
        } else {
            s.clone()
        }
    });
    Ok(with_stops(&theme, stops.collect()))
}

/// Port of `withStopLightness` (`packages/core/src/theme.ts:279`).
pub fn with_stop_lightness(
    theme: serde_json::Value,
    index: f64,
    lightness: f64,
) -> Result<serde_json::Value, String> {
    object(&theme, "stops")?;
    let raw = raw_stops(&theme, "map")?;
    let lit = t::with_stop_lightness(&look(&theme), index, lightness);
    let stops = raw.iter().zip(&lit.stops).enumerate().map(|(i, (s, m))| {
        if i as f64 == index {
            let mut out = spread(s);
            out.insert("color".into(), padded_color(&m.color, s));
            Value::Object(out)
        } else {
            s.clone()
        }
    });
    Ok(with_stops(&theme, stops.collect()))
}

/// Port of `withAngle` (`packages/core/src/theme.ts:284`).
pub fn with_angle(theme: serde_json::Value, degrees: f64) -> Result<serde_json::Value, String> {
    let mut out = spread(&theme);
    let angle = t::with_angle(&look(&theme), degrees)
        .angle
        .unwrap_or(f64::NAN);
    out.insert("angle".into(), js::number(angle));
    Ok(Value::Object(out))
}

/// Port of `withAddedStop` (`packages/core/src/theme.ts:288`).
pub fn with_added_stop(theme: serde_json::Value) -> Result<serde_json::Value, String> {
    object(&theme, "stops")?;
    let raw = raw_stops(&theme, "length")?;
    let added = t::with_added_stop(&look(&theme));
    match added.stops.get(raw.len()) {
        None => Ok(theme),
        Some(s) => {
            let mut stops = raw.clone();
            stops.push(
                json!({"x": js::number(s.x), "y": js::number(s.y), "color": color_json(&s.color)}),
            );
            Ok(with_stops(&theme, stops))
        }
    }
}

/// Port of `withoutStop` (`packages/core/src/theme.ts:297`). The kept stops are the input's own
/// JSON, so this filters them here, as `ria_core` `without_stop` does on typed stops.
pub fn without_stop(theme: serde_json::Value, index: f64) -> Result<serde_json::Value, String> {
    object(&theme, "stops")?;
    let raw = raw_stops(&theme, "length")?;
    if raw.len() <= 1 {
        return Ok(theme);
    }
    let kept = raw
        .iter()
        .enumerate()
        .filter(|(i, _)| *i as f64 != index)
        .map(|(_, s)| s.clone());
    Ok(with_stops(&theme, kept.collect()))
}

/// Port of `withTexture` (`packages/core/src/theme.ts:302`).
pub fn with_texture(
    theme: serde_json::Value,
    texture: serde_json::Value,
) -> Result<serde_json::Value, String> {
    object(&theme, "grain")?;
    let grain = get(&theme, "grain");
    let mut out = spread(&theme);
    out.insert("texture".into(), texture);
    let grain = match grain {
        g if truthy(g) => g.cloned().unwrap_or_else(js::undefined),
        g => js::number(t::grain_or_starting(to_number(g))),
    };
    out.insert("grain".into(), grain);
    Ok(Value::Object(out))
}

/// Port of `startingTheme` (`packages/core/src/theme.ts:306`).
pub fn starting_theme(swatch: String) -> Result<serde_json::Value, String> {
    let theme = t::starting_theme(&swatch);
    let l = &theme.look;
    let stops = l.stops.iter().map(
        |s| json!({"x": js::number(s.x), "y": js::number(s.y), "color": color_json(&s.color)}),
    );
    Ok(json!({
        "stops": stops.collect::<Vec<_>>(),
        "intensity": js::number(l.intensity),
        "mode": theme.mode.name(),
        "grain": js::number(l.grain),
        "texture": l.texture.name(),
        "translucency": js::number(l.translucency),
    }))
}

fn raw_color(v: &Value) -> RawColor {
    let n = |k| get(v, k).and_then(js::as_number);
    RawColor {
        r: n("r"),
        g: n("g"),
        b: n("b"),
        a: n("a"),
    }
}

fn raw_look(v: &Value) -> RawLook {
    let n = |k| get(v, k).and_then(js::as_number);
    let raw_stop = |s: &Value| RawStop {
        color: get(s, "color").filter(|c| is_object(c)).map(raw_color),
        x: get(s, "x").and_then(js::as_number),
        y: get(s, "y").and_then(js::as_number),
    };
    RawLook {
        stops: get(v, "stops")
            .and_then(Value::as_array)
            .map(|s| s.iter().map(raw_stop).collect()),
        intensity: n("intensity"),
        grain: n("grain"),
        texture: get(v, "texture")
            .and_then(Value::as_str)
            .and_then(ThemeTexture::from_name),
        translucency: n("translucency"),
        angle: n("angle"),
    }
}

/// Port of `sanitizeTheme` (`packages/core/src/theme.ts:347`).
pub fn sanitize_theme(v: serde_json::Value) -> Result<Option<serde_json::Value>, String> {
    if !is_object(&v) {
        return Ok(None);
    }
    let raw = RawTheme {
        look: raw_look(&v),
        mode: get(&v, "mode")
            .and_then(Value::as_str)
            .and_then(ThemeMode::from_name),
        dark: get(&v, "dark").filter(|d| is_object(d)).map(raw_look),
    };
    Ok(t::sanitize_theme(&raw).map(|theme| {
        let mut out = look_json(&theme.look);
        out.insert("mode".into(), theme.mode.name().into());
        if let Some(dark) = &theme.dark {
            out.insert("dark".into(), Value::Object(look_json(dark)));
        }
        Value::Object(out)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_number_follows_js() {
        assert_eq!(to_number(Some(&json!(" 0x1f "))), 31.0);
        assert_eq!(to_number(Some(&json!(["7"]))), 7.0);
        assert!(to_number(Some(&json!("inf"))).is_nan());
        assert_eq!(to_number(Some(&Value::Null)), 0.0);
    }
}
