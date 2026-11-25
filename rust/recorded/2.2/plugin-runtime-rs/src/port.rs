// Ported from ardia apps/browser/src/lib/pluginRuntime.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_runtime`, through `pluginRuntime.harness.ts`:
//! each case gives the result and the problems the check added. The logic is in `ria_state`.

#![forbid(unsafe_code)]

use ria_core::identity::telemetry_wire::{Json, js_order};
use ria_state::shared::plugin_checks::Check;
use ria_state::shared::plugin_runtime as p;
use serde::Serialize;
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

fn out<T: Serialize>(c: Check, result: Option<T>) -> Result<Value, String> {
    let result = serde_json::to_value(result).map_err(|e| e.to_string())?;
    Ok(json!({ "result": result, "problems": c.problems }))
}

/// Port of `sidecarOfCase` (`apps/browser/src/lib/pluginRuntime.harness.ts:10`).
pub fn sidecar_of_case(path: String, value: Value) -> Result<Value, String> {
    let mut c = Check::default();
    let result = p::sidecar_of(&mut c, &path, json_in(&value).as_ref());
    out(c, result)
}

/// Port of `mainOfCase` (`apps/browser/src/lib/pluginRuntime.harness.ts:11`).
pub fn main_of_case(entry: Value, memory: Value) -> Result<Value, String> {
    let mut c = Check::default();
    let result = p::main_of(&mut c, json_in(&entry).as_ref(), json_in(&memory).as_ref());
    out(c, result)
}

/// Port of `programCase` (`apps/browser/src/lib/pluginRuntime.harness.ts:12`).
pub fn program_case(path: String, value: Value) -> Result<Value, String> {
    let mut c = Check::default();
    let result = p::program(&mut c, &path, json_in(&value).as_ref());
    out(c, result)
}
