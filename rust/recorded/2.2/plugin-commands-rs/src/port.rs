// Ported from ardia apps/browser/src/lib/pluginCommands.harness.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_commands`, through `pluginCommands.harness.ts`. The logic is in
//! `ria_state::shared::plugin_commands`.

#![forbid(unsafe_code)]

use ria_core::identity::telemetry_wire::{Json, js_order};
use serde_json::Value;

/// A JS value as `Json`, object keys in JS order; `None` for `undefined`. An `undefined` inside
/// an array is `null`, inside an object it is left out; a string with lone surrogates loses them.
fn json_in(v: &Value) -> Option<Json> {
    if crate::js::is_undefined(v) {
        return None;
    }
    if let Some(n) = crate::js::as_number(v) {
        return Some(Json::Number(n));
    }
    if let Some(units) = crate::js::string_units(v) {
        return Some(Json::String(String::from_utf16_lossy(&units)));
    }
    Some(match v {
        Value::Bool(b) => Json::Bool(*b),
        Value::Array(items) => Json::Array(
            items
                .iter()
                .map(|x| json_in(x).unwrap_or(Json::Null))
                .collect(),
        ),
        Value::Object(o) => {
            let entries: Vec<(String, Json)> = o
                .iter()
                .filter_map(|(k, x)| Some((k.clone(), json_in(x)?)))
                .collect();
            Json::Object(js_order(&entries).into_iter().cloned().collect())
        }
        _ => Json::Null,
    })
}

/// A serialized port value as a JS value: each number as JS gives it.
fn js_numbers(v: Value) -> Value {
    match v {
        Value::Number(n) => n.as_f64().map_or(Value::Null, crate::js::number),
        Value::Array(items) => Value::Array(items.into_iter().map(js_numbers).collect()),
        Value::Object(o) => Value::Object(o.into_iter().map(|(k, x)| (k, js_numbers(x))).collect()),
        v => v,
    }
}

/// A port value as the JS object the TS gives.
fn to_js<T: serde::Serialize>(t: &T) -> Result<Value, String> {
    serde_json::to_value(t)
        .map(js_numbers)
        .map_err(|e| e.to_string())
}
use ria_state::shared::plugin_checks::Check;
use ria_state::shared::plugin_commands as k;
use serde_json::json;

fn out(c: Check, result: Value) -> Result<Value, String> {
    Ok(json!({ "result": result, "problems": c.problems }))
}

/// Port of `splitSidesCase` (`apps/browser/src/lib/pluginCommands.harness.ts:14`).
pub fn split_sides_case() -> Result<Value, String> {
    Ok(json!(k::SPLIT_SIDES))
}

/// Port of `whenOfCase` (`apps/browser/src/lib/pluginCommands.harness.ts:15`).
pub fn when_of_case(path: String, value: Value) -> Result<Value, String> {
    let mut c = Check::default();
    let value = json_in(&value);
    let result = k::when_of(&mut c, &path, value.as_ref())
        .map_or_else(crate::js::undefined, |w| Value::String(w.to_owned()));
    out(c, result)
}

/// Port of `commandCase` (`apps/browser/src/lib/pluginCommands.harness.ts:16`).
pub fn command_case(path: String, value: Value, has_main: bool) -> Result<Value, String> {
    let mut c = Check::default();
    let value = json_in(&value);
    let spec = k::command(&mut c, &path, value.as_ref(), has_main);
    let kinds = spec.as_ref().map_or(Value::Null, |s| {
        let a = &s.action;
        json!([
            k::is_tool_action(a),
            k::is_open_action(a),
            k::is_view_action(a),
            k::is_panel_action(a),
            k::is_handler_action(a)
        ])
    });
    Ok(json!({ "result": to_js(&spec)?, "problems": c.problems, "kinds": kinds }))
}

/// Port of `toolCase` (`apps/browser/src/lib/pluginCommands.harness.ts:21`).
pub fn tool_case(path: String, value: Value) -> Result<Value, String> {
    let mut c = Check::default();
    let value = json_in(&value);
    let result = to_js(&k::tool(&mut c, &path, value.as_ref()))?;
    out(c, result)
}
