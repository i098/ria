// Ported from ardia apps/browser/src/lib/pluginRows.harness.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_rows`, through `pluginRows.harness.ts`. The logic is in
//! `ria_state::shared::plugin_rows`.

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
use ria_state::shared::plugin_rows as k;
use serde_json::json;

/// Port of `rowTypesCase` (`apps/browser/src/lib/pluginRows.harness.ts:14`).
pub fn row_types_case() -> Result<Value, String> {
    Ok(json!(k::ROW_TYPES))
}

/// Port of `rowCase` (`apps/browser/src/lib/pluginRows.harness.ts:15`).
pub fn row_case(path: String, value: Value) -> Result<Value, String> {
    let mut c = Check::default();
    let value = json_in(&value);
    let result = to_js(&k::row(&mut c, &path, value.as_ref()))?;
    Ok(json!({ "result": result, "problems": c.problems }))
}

/// Port of `settingsOfCase` (`apps/browser/src/lib/pluginRows.harness.ts:16`).
pub fn settings_of_case(path: String, value: Value) -> Result<Value, String> {
    let mut c = Check::default();
    let value = json_in(&value);
    let s = k::settings_of(&mut c, &path, value.as_ref());
    let keys = |rows: Vec<&k::RowSpec>| rows.iter().map(|r| r.key.clone()).collect::<Vec<_>>();
    Ok(
        json!({ "result": to_js(&s)?, "problems": c.problems, "stored": keys(k::stored_rows(&s)), "secret": keys(k::secret_rows(&s)) }),
    )
}

/// Port of `rowValueCase` (`apps/browser/src/lib/pluginRows.harness.ts:21`).
pub fn row_value_case(raw: Value, value: Value) -> Result<Value, String> {
    let raw = json_in(&raw);
    let value = json_in(&value);
    match k::row(&mut Check::default(), "row", raw.as_ref()) {
        Some(r) => to_js(&k::row_value(&r, value.as_ref())),
        None => Ok(Value::Null),
    }
}
