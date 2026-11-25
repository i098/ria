// Ported from ardia apps/browser/src/lib/jsonSchema.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::json_schema`. The logic is in `ria_state::shared::json_schema`.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, string_units};
use ria_core::identity::telemetry_wire::Json;
use ria_state::shared::json_schema::schema_json as schema_text;
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

/// Port of `schemaJson` (`apps/browser/src/lib/jsonSchema.ts:27`).
pub fn schema_json(schema: Value) -> Result<String, String> {
    Ok(schema_text(&json_of(&schema)))
}
