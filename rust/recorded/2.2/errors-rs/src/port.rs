// Ported from ardia apps/browser/src/lib/errors.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::errors`. The harness (`errors.harness.ts`) builds an `Error` from a value. The logic is in `ria_state::shared::errors`.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, string_units};
use ria_core::identity::telemetry_wire::Json;
use ria_state::shared::errors::{Thrown, error_text};
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

/// The value the harness throws: `new Error(String(value))` when `as_error`.
fn thrown(value: &Value, as_error: bool) -> Thrown {
    let value = if is_undefined(value) {
        Thrown::Undefined
    } else {
        Thrown::Value(json_of(value))
    };
    if as_error {
        Thrown::Error(error_text(&value))
    } else {
        value
    }
}

/// Port of `errorTextCase` (`apps/browser/src/lib/errors.harness.ts:4`).
pub fn error_text_case(value: Value, as_error: bool) -> Result<String, String> {
    Ok(error_text(&thrown(&value, as_error)))
}
