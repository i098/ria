// Ported from ardia apps/browser/src/lib/decode.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::decode`. Values cross as `Json` (see `json_of`). The logic is in `ria_state::shared::decode`.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, number, string_units};
use ria_core::identity::telemetry_wire::{Json, js_order};
use ria_state::shared::decode as d;
use serde_json::Value;

/// A JS value as `Json`, as `JSON.stringify` reads it: an `undefined` member is left out, an `undefined`
/// item is `null`.
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

/// A fallback of any JS value.
struct Out(Value);

impl From<&str> for Out {
    fn from(s: &str) -> Self {
        Out(Value::String(s.to_owned()))
    }
}

impl From<f64> for Out {
    fn from(n: f64) -> Self {
        Out(number(n))
    }
}

/// Port of `isFields` (`apps/browser/src/lib/decode.ts:3`).
pub fn is_fields(v: Value) -> Result<bool, String> {
    Ok(d::is_fields(&json_of(&v)))
}

/// Port of `isString` (`apps/browser/src/lib/decode.ts:4`).
pub fn is_string(v: Value) -> Result<bool, String> {
    Ok(d::is_string(&json_of(&v)))
}

/// Port of `isNumber` (`apps/browser/src/lib/decode.ts:5`).
pub fn is_number(v: Value) -> Result<bool, String> {
    Ok(d::is_number(&json_of(&v)))
}

/// Port of `isStringList` (`apps/browser/src/lib/decode.ts:6`).
pub fn is_string_list(v: Value) -> Result<bool, String> {
    Ok(d::is_string_list(&json_of(&v)))
}

/// Port of `stringOr` (`apps/browser/src/lib/decode.ts:8`).
pub fn string_or(v: Value, fallback: Value) -> Result<Value, String> {
    Ok(d::string_or(&json_of(&v), Out(fallback)).0)
}

/// Port of `numberOr` (`apps/browser/src/lib/decode.ts:9`).
pub fn number_or(v: Value, fallback: Value) -> Result<Value, String> {
    Ok(d::number_or(&json_of(&v), Out(fallback)).0)
}

/// Port of `listOf` (`apps/browser/src/lib/decode.ts:10`).
pub fn list_of(v: Value) -> Result<Vec<Value>, String> {
    Ok(d::list_of(&json_of(&v)).iter().map(value_of).collect())
}
