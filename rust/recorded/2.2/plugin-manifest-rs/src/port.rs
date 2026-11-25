// Ported from ardia apps/browser/src/lib/pluginManifest.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_manifest`, through `pluginManifest.harness.ts`. The logic
//! is in `ria_state::shared::plugin_manifest`.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, number, string_units, utf16_string};
use ria_core::identity::telemetry_wire::{Json, js_order};
use ria_state::shared::decode::Fields;
use ria_state::shared::plugin_manifest::{
    MANIFEST_VERSION, check_manifest, contributions_of, default_values, network_label,
    read_manifest,
};
use serde_json::{Value, json};

/// The keys `checkManifest` reads from `raw`.
const READ_KEYS: [&str; 19] = [
    "$schema",
    "manifest",
    "name",
    "title",
    "description",
    "version",
    "requires",
    "icon",
    "main",
    "memoryMB",
    "sidecar",
    "activationEvents",
    "permissions",
    "contributes",
    "alwaysOn",
    "official",
    "autoInstall",
    "keywords",
    "telemetry",
];

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

/// `raw` as the fields `checkManifest` sees: `Object.keys` for `only`, and the members it reads. An
/// `undefined` member it reads is left out; one it does not read stays, as `null`, for `only` to report.
fn raw_fields(raw: &Value) -> Result<Fields, String> {
    if raw.is_null() || is_undefined(raw) {
        return Err("TypeError: Cannot convert undefined or null to object".into());
    }
    if as_number(raw).is_some() || raw.is_boolean() {
        return Ok(Fields::new());
    }
    let indexes = |n: usize| (0..n).map(|i| (i.to_string(), Json::Null)).collect();
    if let Some(units) = string_units(raw) {
        return Ok(indexes(units.len()));
    }
    Ok(match raw {
        Value::String(s) => indexes(s.encode_utf16().count()),
        Value::Array(items) => indexes(items.len()),
        Value::Object(o) => o
            .iter()
            .filter(|(k, v)| !is_undefined(v) || !READ_KEYS.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), json_of(v)))
            .collect(),
        _ => Fields::new(),
    })
}

/// `Json` as a JS value, object keys in JS order.
fn value_of(j: &Json) -> Value {
    match j {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => number(*n),
        Json::String(s) => Value::String(s.clone()),
        Json::Array(items) => Value::Array(items.iter().map(value_of).collect()),
        Json::Object(entries) => Value::Object(
            js_order(entries)
                .into_iter()
                .map(|(k, v)| (k.clone(), value_of(v)))
                .collect(),
        ),
    }
}

/// A serialized result with its numbers in the wire form (`1024`, not `1024.0`).
fn wire(value: Value) -> Value {
    match value {
        Value::Number(n) => number(n.as_f64().unwrap_or(f64::NAN)),
        Value::Array(items) => Value::Array(items.into_iter().map(wire).collect()),
        Value::Object(o) => Value::Object(o.into_iter().map(|(k, v)| (k, wire(v))).collect()),
        v => v,
    }
}

fn to_value(value: &impl serde::Serialize) -> Result<Value, String> {
    serde_json::to_value(value)
        .map(wire)
        .map_err(|e| e.to_string())
}

/// Port of `checkManifestCase` (`apps/browser/src/lib/pluginManifest.harness.ts:6`).
pub fn check_manifest_case(raw: Value, declared_manifest: Option<f64>) -> Result<Value, String> {
    let declared = declared_manifest.unwrap_or(MANIFEST_VERSION);
    let mut out = to_value(&check_manifest(&raw_fields(&raw)?, declared))?;
    // serde gives `null` for a NaN `declaredManifest`.
    if let Some(m) = out.get_mut("manifest").filter(|m| m.is_object()) {
        m["declaredManifest"] = number(declared);
    }
    Ok(out)
}

/// The `JSON.parse` problem as V8 writes it, in UTF-16 units: its bad token or quote can cut an emoji in two,
/// which a Rust `String` (and so the problem `ria_state` gives) holds as U+FFFD. The bad token is a lone
/// surrogate there, so this finds the surrogate position whose V8 message reads as `problem`.
fn parse_problem_units(text: &str, problem: &str) -> Option<Value> {
    const PREFIX: &str = "plugin.json isn't valid JSON (Unexpected token '";
    if !problem.starts_with(PREFIX) || !problem.contains('\u{FFFD}') {
        return None;
    }
    let units: Vec<u16> = text.encode_utf16().collect();
    let len = units.len();
    let s = |t: &str| t.encode_utf16().collect::<Vec<u16>>();
    (0..len)
        .filter(|&pos| (0xD800..0xE000).contains(&units[pos]))
        .map(|pos| {
            // V8 `JsonParser::ReportUnexpectedToken`, as `ria_core::search::bangs::parse_json` (context 10).
            let (head, range, tail) = if len <= 20 {
                ("\"", 0..len, "\"")
            } else if pos < 10 {
                ("\"", 0..pos + 10, "\"...")
            } else if pos < len - 10 {
                ("...\"", pos - 10..pos + 10, "\"...")
            } else {
                ("...\"", pos - 10..len, "\"")
            };
            [
                s(PREFIX),
                vec![units[pos]],
                s("', "),
                s(head),
                units[range].to_vec(),
                s(tail),
                s(" is not valid JSON)"),
            ]
            .concat()
        })
        .find(|built| String::from_utf16_lossy(built) == problem)
        .map(|built| utf16_string(&built))
}

/// Port of `readManifestCase` (`apps/browser/src/lib/pluginManifest.harness.ts:4`).
pub fn read_manifest_case(text: String) -> Result<Value, String> {
    let mut out = to_value(&read_manifest(&text))?;
    if let Some(Value::Array(problems)) = out.get_mut("problems") {
        for p in problems.iter_mut() {
            if let Some(units) = p.as_str().and_then(|p| parse_problem_units(&text, p)) {
                *p = units;
            }
        }
    }
    Ok(out)
}

/// Port of `manifestUsesCase` (`apps/browser/src/lib/pluginManifest.harness.ts:10`).
pub fn manifest_uses_case(text: String) -> Result<Value, String> {
    Ok(match read_manifest(&text).manifest {
        None => Value::Null,
        Some(m) => json!({
            "contributions": contributions_of(&m),
            "defaults": value_of(&Json::Object(default_values(&m))),
            "network": network_label(&m.permissions),
        }),
    })
}
