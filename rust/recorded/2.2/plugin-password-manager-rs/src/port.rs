// Ported from ardia apps/browser/src/lib/pluginPasswordManager.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_password_manager`, through
//! `pluginPasswordManager.harness.ts`: each case gives the result and the problems it added.

#![forbid(unsafe_code)]

use ria_core::identity::telemetry_wire::{Json, js_order};
use ria_state::shared::plugin_password_manager as p;
use serde_json::{Value, json};

/// A JS value as `Json`, object keys in JS order; `None` for `undefined`. An `undefined` inside
/// an array or object is `null`; a string with lone surrogates loses them (`known-diffs.jsonl`).
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
        Value::Array(items) => Json::Array(items.iter().map(json_or_null).collect()),
        Value::Object(o) => {
            let entries: Vec<(String, Json)> = o
                .iter()
                .map(|(k, x)| (k.clone(), json_or_null(x)))
                .collect();
            Json::Object(js_order(&entries).into_iter().cloned().collect())
        }
        _ => Json::Null,
    })
}

fn json_or_null(v: &Value) -> Json {
    json_in(v).unwrap_or(Json::Null)
}

/// Port of `passwordManagerOfCase` (`apps/browser/src/lib/pluginPasswordManager.harness.ts:4`).
pub fn password_manager_of_case(value: Value) -> Result<Value, String> {
    let mut problems = Vec::new();
    let result = p::password_manager_of(&mut problems, json_in(&value).as_ref());
    Ok(json!({ "result": result, "problems": problems }))
}
