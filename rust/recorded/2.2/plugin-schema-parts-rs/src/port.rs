// Ported from ardia apps/browser/src/lib/pluginSchemaParts.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_schema_parts`. The logic is in `ria_state::shared::plugin_schema_parts`.

#![forbid(unsafe_code)]

use crate::js::{is_undefined, number, string_units};
use ria_core::identity::telemetry_wire::{Json, js_order};
use ria_state::shared::plugin_schema_parts as p;
use serde_json::Value;

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

/// Port of `textOf` (`apps/browser/src/lib/pluginSchemaParts.ts:7`).
pub fn text_of(max: f64, description: Option<String>) -> Result<Value, String> {
    Ok(value_of(&p::text_of(max, description.as_deref())))
}

/// Port of `closed` (`apps/browser/src/lib/pluginSchemaParts.ts:8`). `properties` goes in unchanged, so
/// an `undefined` inside it stays. `required` is any JS value: `required.length` decides, as in the TS.
pub fn closed(properties: Value, required: Value) -> Result<Value, String> {
    let strings: Option<Vec<String>> = match &required {
        _ if is_undefined(&required) => Some(Vec::new()),
        Value::Array(items) => items
            .iter()
            .map(|i| i.as_str().map(str::to_owned))
            .collect(),
        _ => None,
    };
    let mut out = value_of(&p::closed(Json::Null, &strings.clone().unwrap_or_default()));
    out["properties"] = properties;
    if strings.is_none() {
        match &required {
            Value::Null => {
                return Err("TypeError: Cannot read properties of null (reading 'length')".into());
            }
            Value::Array(items) if !items.is_empty() => out["required"] = required,
            _ if string_units(&required).is_some_and(|u| !u.is_empty()) => {
                out["required"] = required
            }
            _ => {}
        }
    }
    Ok(out)
}

/// Port of `pluginPath` (`apps/browser/src/lib/pluginSchemaParts.ts:17`).
pub fn plugin_path(description: Option<String>) -> Result<Value, String> {
    Ok(value_of(&p::plugin_path(description.as_deref())))
}

/// Port of `rowSchema` (`apps/browser/src/lib/pluginSchemaParts.ts:20`).
pub fn row_schema(types: Vec<String>) -> Result<Value, String> {
    Ok(value_of(&p::row_schema(&types)))
}
