// Ported from ardia apps/browser/src/lib/themeFile.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::theme_file`: palettes, themes and schemas cross as
//! JSON. A palette value that is not a `RoleValue` (`null`, an object, an array that is not two
//! strings) is left out of the palette. The logic is in `ria_state::shared::theme_file`.
//! Arguments outside the TS types (for example a `color` that is not a colour name) are not covered:
//! the Rust side takes typed values, so the crate records with `--count 0` and only in-type inputs,
//! with the `null` and `undefined` cases where the TS really throws.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, number, string_units};
use ria_core::identity::telemetry_wire::Json;
use ria_state::shared::theme_file::{self as f, RoleKind, RolePalette, RolePalettes, RoleValue};
use ria_state::shared::theme_look::{LOOK_KEYS, LookBlocks};
use serde_json::{Map, Value, json};

/// A JS value as `Json`: an `undefined` member is left out, an `undefined` item is `null`.
fn json_of(v: &Value) -> Json {
    if let Some(n) = as_number(v) {
        return Json::Number(n);
    }
    if let Some(units) = string_units(v) {
        return Json::String(String::from_utf16_lossy(&units));
    }
    match v {
        Value::Bool(b) => Json::Bool(*b),
        Value::String(s) => Json::String(s.clone()),
        Value::Array(items) => Json::Array(items.iter().map(json_of).collect()),
        Value::Object(o) if !is_undefined(v) => Json::Object(
            o.iter()
                .filter(|(_, v)| !is_undefined(v))
                .map(|(k, v)| (k.clone(), json_of(v)))
                .collect(),
        ),
        _ => Json::Null,
    }
}

/// `Json` as a JS value.
fn value_of(j: &Json) -> Value {
    match j {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => number(*n),
        Json::String(s) => Value::String(s.clone()),
        Json::Array(items) => Value::Array(items.iter().map(value_of).collect()),
        Json::Object(o) => Value::Object(o.iter().map(|(k, v)| (k.clone(), value_of(v))).collect()),
    }
}

fn blocks_value(b: &LookBlocks) -> Value {
    Value::Object(
        LOOK_KEYS
            .iter()
            .filter_map(|key| Some(((*key).to_owned(), value_of(b.get(key)?))))
            .collect(),
    )
}

fn role_value_of(v: &Json) -> Option<RoleValue> {
    match v {
        Json::String(s) => Some(RoleValue::Color(s.clone())),
        Json::Number(n) => Some(RoleValue::Number(*n)),
        Json::Bool(b) => Some(RoleValue::Bool(*b)),
        Json::Array(items) => match items.as_slice() {
            [Json::String(a), Json::String(b)] => Some(RoleValue::Pair([a.clone(), b.clone()])),
            _ => None,
        },
        _ => None,
    }
}

fn palette_of(v: Option<&Value>) -> RolePalette {
    match v.map(json_of) {
        Some(Json::Object(o)) => o
            .iter()
            .filter_map(|(k, v)| Some((k.clone(), role_value_of(v)?)))
            .collect(),
        _ => Vec::new(),
    }
}

fn palettes_of(v: &Value) -> RolePalettes {
    RolePalettes {
        dark: palette_of(v.get("dark")),
        light: palette_of(v.get("light")),
    }
}

fn palette_value(p: &RolePalette) -> Value {
    Value::Object(
        p.iter()
            .map(|(k, v)| (k.clone(), value_of(&v.to_json())))
            .collect(),
    )
}

/// Port of `themeColor` (`apps/browser/src/lib/themeFile.ts:23`).
pub fn theme_color(value: serde_json::Value) -> Result<Option<String>, String> {
    Ok(f::theme_color(&json_of(&value)))
}

/// Port of `roleKind` (`apps/browser/src/lib/themeFile.ts:45`): any array is a pair here, as in TS.
pub fn role_kind(
    role: String,
    value: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let value = match json_of(&value) {
        Json::Array(_) => Some(RoleValue::Pair([String::new(), String::new()])),
        j => role_value_of(&j),
    };
    Ok(value
        .and_then(|v| f::role_kind(&role, &v))
        .map(|kind| match kind {
            RoleKind::Color => json!({"kind": "color"}),
            RoleKind::Pair => json!({"kind": "pair"}),
            RoleKind::Number { minimum, maximum } => {
                let mut m = Map::new();
                m.insert("kind".into(), "number".into());
                if let Some(min) = minimum {
                    m.insert("minimum".into(), number(min));
                }
                if let Some(max) = maximum {
                    m.insert("maximum".into(), number(max));
                }
                Value::Object(m)
            }
        }))
}

/// Port of `parseThemeFile` (`apps/browser/src/lib/themeFile.ts:69`).
pub fn parse_theme_file(
    file: String,
    text: String,
    defaults: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let parsed = f::parse_theme_file(&file, &text, &palettes_of(&defaults));
    let roles = parsed.roles.map_or(
        Value::Null,
        |r| json!({"dark": palette_value(&r.dark), "light": palette_value(&r.light)}),
    );
    let look = parsed.look.as_ref().map_or(Value::Null, blocks_value);
    Ok(json!({"roles": roles, "look": look, "problems": parsed.problems}))
}

/// Port of `mergedPalettes` (`apps/browser/src/lib/themeFile.ts:102`).
pub fn merged_palettes(
    defaults: serde_json::Value,
    roles: Option<serde_json::Value>,
    revision: f64,
) -> Result<serde_json::Value, String> {
    if defaults.is_null() || is_undefined(&defaults) {
        let what = if defaults.is_null() {
            "null"
        } else {
            "undefined"
        };
        return Err(format!(
            "TypeError: Cannot read properties of {what} (reading 'dark')"
        ));
    }
    let roles = roles
        .filter(|r| !r.is_null() && !is_undefined(r))
        .map(|r| palettes_of(&r));
    let p = f::merged_palettes(&palettes_of(&defaults), roles.as_ref(), revision);
    Ok(
        json!({"revision": number(p.revision), "dark": palette_value(&p.dark), "light": palette_value(&p.light)}),
    )
}

/// Port of `themeSchema` (`apps/browser/src/lib/themeFile.ts:123`).
pub fn theme_schema(defaults: serde_json::Value) -> Result<serde_json::Value, String> {
    if defaults.is_null() || is_undefined(&defaults) {
        let what = if defaults.is_null() {
            "null"
        } else {
            "undefined"
        };
        return Err(format!(
            "TypeError: Cannot read properties of {what} (reading 'dark')"
        ));
    }
    let palette = |mode| {
        defaults
            .get(mode)
            .filter(|p| !p.is_null() && !is_undefined(p))
    };
    if palette("dark").is_none() || palette("light").is_none() {
        return Err("TypeError: Cannot convert undefined or null to object".into());
    }
    Ok(value_of(&f::theme_schema(&palettes_of(&defaults))))
}
