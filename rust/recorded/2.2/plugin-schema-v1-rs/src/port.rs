// Ported from ardia apps/browser/src/lib/pluginSchemaV1.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_schema_v1`. The logic is in `ria_state::shared::plugin_schema_v1`.

#![forbid(unsafe_code)]

use crate::js::number;
use ria_core::identity::telemetry_wire::{Json, js_order};
use ria_state::shared::plugin_schema_v1 as v1;
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

/// Port of `legacyPluginBody` (`apps/browser/src/lib/pluginSchemaV1.ts:6`).
pub fn legacy_plugin_body(event_names: Vec<String>) -> Result<Value, String> {
    Ok(value_of(&v1::legacy_plugin_body(event_names.as_slice())))
}
