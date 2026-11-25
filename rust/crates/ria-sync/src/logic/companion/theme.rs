// Ported from ardia packages/sync/src/companion/theme.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The theme of a profile, read from the `pth:` records of the app scope or the profile record.
//!
//! Changes from Ardia:
//! - The inputs are JSON values (`Option<&Value>`, `None` for `undefined`), not any JS value,
//!   so a number is always finite; the `Number.isFinite` checks stay.

use super::super::js::{js_max, js_min};
use super::super::scope_state::ScopeState;
use serde::Serialize;
use serde_json::{Map, Value};

/// The color of a `ViewThemeStop`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ViewThemeColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

/// Port of `ViewThemeStop` (`packages/sync/src/companion/theme.ts:3`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ViewThemeStop {
    pub color: ViewThemeColor,
    pub x: f64,
    pub y: f64,
}

/// Port of `ViewThemeTexture` (`packages/sync/src/companion/theme.ts:4`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewThemeTexture {
    Grain,
    Sand,
    Denim,
    Tweed,
}

/// Port of `ViewThemeLook` (`packages/sync/src/companion/theme.ts:5`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ViewThemeLook {
    pub stops: Vec<ViewThemeStop>,
    pub intensity: f64,
    pub grain: f64,
    pub texture: ViewThemeTexture,
    pub translucency: f64,
    pub angle: f64,
}

/// The `mode` of a `ViewTheme`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewThemeMode {
    Auto,
    Light,
    Dark,
}

/// Port of `ViewTheme` (`packages/sync/src/companion/theme.ts:6`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ViewTheme {
    pub colors: Vec<String>,
    #[serde(rename = "darkColors")]
    pub dark_colors: Option<Vec<String>>,
    pub mode: ViewThemeMode,
    pub look: ViewThemeLook,
    pub dark: Option<ViewThemeLook>,
}

/// JS truthiness of a JSON value (`undefined` is `None`).
pub(crate) fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|n| n != 0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(_) | Value::Object(_)) => true,
    }
}

/// `undefined` for JSON `null` too: the left side of `??`.
fn defined(v: Option<&Value>) -> Option<&Value> {
    v.filter(|v| !v.is_null())
}

/// `typeof v === "number" && Number.isFinite(v)`.
fn finite(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64).filter(|n| n.is_finite())
}

fn clamp01(n: f64) -> f64 {
    js_min(1.0, js_max(0.0, n))
}

/// `Math.round` (ties go up).
fn round(x: f64) -> f64 {
    let floor = x.floor();
    if x - floor >= 0.5 { floor + 1.0 } else { floor }
}

/// Port of `hex` (`packages/sync/src/companion/theme.ts:8`).
fn hex(n: Option<&Value>) -> String {
    let v = finite(n).map_or(0.0, clamp01);
    format!("{:02X}", round(v * 255.0) as u8)
}

/// The color of a stop when its `r`, `g` and `b` are finite numbers under 4 in size.
fn stop_color(stop: &Value) -> Option<&Map<String, Value>> {
    let c = stop.get("color")?.as_object()?;
    ["r", "g", "b"]
        .iter()
        .all(|k| finite(c.get(*k)).is_some_and(|n| n.abs() < 4.0))
        .then_some(c)
}

/// The `stops` array of a look object.
fn stops_of(look: Option<&Value>) -> Option<&Vec<Value>> {
    look?.as_object()?.get("stops")?.as_array()
}

/// Port of `colorsOf` (`packages/sync/src/companion/theme.ts:13`).
fn colors_of(look: Option<&Value>) -> Option<Vec<String>> {
    let mut out = Vec::new();
    for stop in stops_of(look)?.iter().take(16) {
        if out.len() >= 3 {
            break;
        }
        let Some(c) = stop_color(stop) else { continue };
        let alpha = finite(c.get("a")).map_or(1.0, clamp01);
        let a = if alpha < 1.0 {
            hex(c.get("a"))
        } else {
            String::new()
        };
        out.push(format!(
            "#{}{}{}{a}",
            hex(c.get("r")),
            hex(c.get("g")),
            hex(c.get("b"))
        ));
    }
    (!out.is_empty()).then_some(out)
}

const DEFAULT_ANGLE: f64 = 135.0;

/// Port of `unit` (`packages/sync/src/companion/theme.ts:28`).
fn unit(v: Option<&Value>, fallback: f64) -> f64 {
    finite(v).map_or(fallback, clamp01)
}

/// Port of `degrees` (`packages/sync/src/companion/theme.ts:29`).
fn degrees(v: Option<&Value>) -> Option<f64> {
    finite(v).map(|v| ((v % 360.0) + 360.0) % 360.0)
}

/// Port of `themeLookOf` (`packages/sync/src/companion/theme.ts:31`).
fn theme_look_of(look: Option<&Value>, angle: Option<&Value>) -> Option<ViewThemeLook> {
    let raw_stops = stops_of(look)?;
    let t = look?;
    let mut stops = Vec::new();
    for s in raw_stops.iter().take(16) {
        if stops.len() >= 3 {
            break;
        }
        let Some(c) = stop_color(s) else { continue };
        let n = |k: &str| finite(c.get(k)).unwrap_or_default();
        stops.push(ViewThemeStop {
            color: ViewThemeColor {
                r: n("r"),
                g: n("g"),
                b: n("b"),
                a: unit(c.get("a"), 1.0),
            },
            x: unit(s.get("x"), 0.5),
            y: unit(s.get("y"), 0.5),
        });
    }
    if stops.is_empty() {
        return None;
    }
    let texture = match t.get("texture").and_then(Value::as_str) {
        Some("sand") => ViewThemeTexture::Sand,
        Some("denim") => ViewThemeTexture::Denim,
        Some("tweed") => ViewThemeTexture::Tweed,
        _ => ViewThemeTexture::Grain,
    };
    Some(ViewThemeLook {
        stops,
        intensity: unit(t.get("intensity"), 0.6),
        grain: unit(t.get("grain"), 0.0),
        texture,
        translucency: unit(t.get("translucency"), 1.0),
        angle: degrees(angle)
            .or_else(|| degrees(t.get("angle")))
            .unwrap_or(DEFAULT_ANGLE),
    })
}

/// Port of `themeOf` (`packages/sync/src/companion/theme.ts:53`).
pub fn theme_of(
    app: &ScopeState,
    sync_id: &str,
    profile_theme: Option<&Value>,
) -> Option<ViewTheme> {
    let record = app.records.get(&format!("pth:{sync_id}")).map(|r| &r.v);
    let has_record = truthy(record);
    let theme = match record {
        Some(r) if has_record => {
            if r.get("preset") == Some(&Value::Bool(true)) {
                None
            } else {
                r.get("t")
            }
        }
        _ => profile_theme,
    };
    let colors = colors_of(theme)?;
    let angles = if has_record {
        app.records
            .get(&format!("pth:{sync_id}:angle"))
            .map(|r| &r.v)
    } else {
        None
    };
    let angle_a = angles.and_then(|a| a.get("a"));
    let t = theme?;
    let look = theme_look_of(theme, angle_a)?;
    let light_angle = degrees(angle_a).or_else(|| degrees(t.get("angle")));
    let light_value = light_angle
        .and_then(serde_json::Number::from_f64)
        .map(Value::Number);
    let dark_look = t.get("dark");
    let dark_angle = defined(angles.and_then(|a| a.get("d")))
        .or_else(|| defined(dark_look.and_then(|d| d.get("angle"))))
        .or(light_value.as_ref());
    let dark = theme_look_of(dark_look, dark_angle);
    let mode = match t.get("mode").and_then(Value::as_str) {
        Some("light") => ViewThemeMode::Light,
        Some("dark") => ViewThemeMode::Dark,
        _ => ViewThemeMode::Auto,
    };
    Some(ViewTheme {
        colors,
        dark_colors: colors_of(dark_look),
        mode,
        look,
        dark,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn hex_rounds_half_up_and_clamps() {
        assert_eq!(hex(Some(&json!(0.5))), "80");
        assert_eq!(hex(Some(&json!(-2))), "00");
        // `Math.round(0.5)` is 1: Node gives `#01` here too.
        assert_eq!(hex(Some(&json!(0.49999999999999994 / 255.0))), "01");
        assert_eq!(hex(None), "00");
    }

    #[test]
    fn degrees_wraps_negative_angles() {
        assert_eq!(degrees(Some(&json!(-90))), Some(270.0));
        assert_eq!(degrees(Some(&json!(720.5))), Some(0.5));
        assert_eq!(degrees(Some(&json!("9"))), None);
    }
}
