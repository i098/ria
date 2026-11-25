// Ported from ardia apps/browser/src/lib/pluginSchema.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_schema`: `rules` crosses as a JS value and is read as
//! the TS reads it (`rules.decideEvents ?? []`, then a spread of each list). The logic is in
//! `ria_state::shared::plugin_schema`.

#![forbid(unsafe_code)]

use crate::js::{is_undefined, number, string_units};
use ria_core::identity::telemetry_wire::{Json, js_order};
use ria_state::shared::plugin_schema::{self as s, PluginRules};
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

/// `[...value]` of a list of strings: an array, or a string spread into its code points.
fn spread(name: &str, value: Option<&Value>) -> Result<Vec<String>, String> {
    let not_iterable = || format!("TypeError: {name} is not iterable");
    let Some(value) = value.filter(|v| !is_undefined(v)) else {
        return Err(not_iterable());
    };
    if let Some(units) = string_units(value).filter(|_| !value.is_array()) {
        return Ok(String::from_utf16_lossy(&units)
            .chars()
            .map(String::from)
            .collect());
    }
    match value {
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::String(s) => Ok(s.clone()),
                _ => Err(format!(
                    "TypeError: {name} holds a value that is not a string"
                )),
            })
            .collect(),
        _ => Err(not_iterable()),
    }
}

/// `f` of `event_names` and the JS value `rules`.
fn with_rules<T>(
    event_names: &[String],
    rules: &Value,
    f: impl FnOnce(&[&str], &PluginRules) -> T,
) -> Result<T, String> {
    if rules.is_null() || is_undefined(rules) {
        let what = if rules.is_null() { "null" } else { "undefined" };
        return Err(format!(
            "TypeError: Cannot read properties of {what} (reading 'decideEvents')"
        ));
    }
    let field = |key: &str| {
        rules
            .as_object()
            .filter(|_| string_units(rules).is_none())
            .and_then(|o| o.get(key))
    };
    let decide = field("decideEvents").filter(|v| !v.is_null() && !is_undefined(v));
    let untabbed = spread("rules.untabbedEvents", field("untabbedEvents"))?;
    let approved = spread("rules.approvedSettings", field("approvedSettings"))?;
    let decide = decide
        .map(|d| spread("decideEvents", Some(d)))
        .transpose()?;
    fn refs(list: &[String]) -> Vec<&str> {
        list.iter().map(String::as_str).collect()
    }
    let (untabbed, approved) = (refs(&untabbed), refs(&approved));
    let decide = decide.as_deref().map(refs);
    let rules = PluginRules {
        untabbed_events: &untabbed,
        approved_settings: &approved,
        decide_events: decide.as_deref(),
    };
    Ok(f(&refs(event_names), &rules))
}

/// Port of `pluginSchema` (`apps/browser/src/lib/pluginSchema.ts:113`).
pub fn plugin_schema(event_names: Vec<String>, rules: Value) -> Result<Value, String> {
    with_rules(&event_names, &rules, |e, r| {
        value_of(&s::plugin_schema(e, r))
    })
}

/// Port of `pluginSchemaJson` (`apps/browser/src/lib/pluginSchema.ts:127`).
pub fn plugin_schema_json(event_names: Vec<String>, rules: Value) -> Result<Value, String> {
    with_rules(&event_names, &rules, |e, r| {
        Value::String(s::plugin_schema_json(e, r))
    })
}
