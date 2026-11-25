// Ported from ardia apps/browser/src/lib/pluginMigrations.harness.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_migrations`, through `pluginMigrations.harness.ts`. The logic is in
//! `ria_state::shared::plugin_migrations`.

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

/// `Json` back as a JS value.
fn json_out(v: &Json) -> Value {
    match v {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => crate::js::number(*n),
        Json::String(s) => Value::String(s.clone()),
        Json::Array(items) => Value::Array(items.iter().map(json_out).collect()),
        Json::Object(entries) => Value::Object(
            js_order(entries)
                .into_iter()
                .map(|(k, x)| (k.clone(), json_out(x)))
                .collect(),
        ),
    }
}

use ria_state::shared::plugin_migrations as k;

/// The `Record<string, unknown>` argument of a case.
fn doc_in(doc: &Value) -> Result<Vec<(String, Json)>, String> {
    match json_in(doc) {
        Some(Json::Object(doc)) => Ok(doc),
        _ => Err("TypeError: doc is not an object".into()),
    }
}

/// Port of `contributesFromV1Case` (`apps/browser/src/lib/pluginMigrations.harness.ts:4`).
pub fn contributes_from_v1_case(doc: Value) -> Result<Value, String> {
    Ok(json_out(&Json::Object(k::contributes_from_v1(doc_in(
        &doc,
    )?))))
}

/// Port of `migrationCase` (`apps/browser/src/lib/pluginMigrations.harness.ts:5`).
pub fn migration_case(version: f64, doc: Value) -> Result<Value, String> {
    // JS reads the arguments before it checks that the step is a function.
    let doc = doc_in(&doc)?;
    let migrations = k::manifest_migrations();
    let step = (version >= 0.0 && version.fract() == 0.0)
        .then(|| migrations.get(&(version as u64)))
        .flatten();
    let Some(step) = step else {
        return Err("TypeError: MANIFEST_MIGRATIONS[version] is not a function".into());
    };
    Ok(json_out(&step(doc)))
}

/// Port of `v1ProblemCase` (`apps/browser/src/lib/pluginMigrations.harness.ts:6`).
pub fn v1_problem_case(problem: Value) -> Result<String, String> {
    let Some(units) = crate::js::string_units(&problem) else {
        return Err("TypeError: problem is not a string".into());
    };
    Ok(k::v1_problem(&String::from_utf16_lossy(&units)))
}
