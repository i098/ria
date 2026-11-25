// Ported from ardia apps/browser/src/lib/savedDocument.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::saved_document`. The harness (`savedDocument.harness.ts`) builds each migration from data. The logic is in `ria_state::shared::saved_document`.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, number, string_units};
use ria_core::identity::telemetry_wire::set_entry;
use ria_core::identity::telemetry_wire::{Json, js_order};
use ria_state::shared::saved_document::{
    Migrations, SavedObject, parse_saved, unchanged, upgraded, versioned_json,
};
use serde_json::Value;
use serde_json::json;
use std::cell::RefCell;
use std::rc::Rc;

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

/// The entries of `{ ...value }`.
fn spread(value: &Json) -> Vec<(String, Json)> {
    match value {
        Json::Object(entries) => js_order(entries).into_iter().cloned().collect(),
        Json::Array(items) => items
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v.clone()))
            .collect(),
        Json::String(s) => s
            .encode_utf16()
            .enumerate()
            .map(|(i, u)| (i.to_string(), Json::String(String::from_utf16_lossy(&[u]))))
            .collect(),
        _ => Vec::new(),
    }
}

/// `migrationsOf` of the harness: each step logs its key to `calls`. A step of `null` or `undefined`
/// throws in the harness (`step.value`); it sets `threw` and gives no object.
fn migrations_of(
    steps: &Value,
    calls: &Rc<RefCell<Vec<String>>>,
    threw: &Rc<RefCell<Option<&'static str>>>,
) -> Migrations {
    let entries: Vec<(String, Value)> = match steps {
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v.clone()))
            .collect(),
        // `Object.entries("ab")`: one step for each UTF-16 code unit.
        _ if string_units(steps).is_some() || steps.is_string() => {
            let units = string_units(steps)
                .unwrap_or_else(|| steps.as_str().unwrap_or_default().encode_utf16().collect());
            units
                .iter()
                .enumerate()
                .map(|(i, u)| {
                    (
                        i.to_string(),
                        Value::String(String::from_utf16_lossy(&[*u])),
                    )
                })
                .collect()
        }
        Value::Object(o)
            if as_number(steps).is_none()
                && string_units(steps).is_none()
                && !is_undefined(steps) =>
        {
            o.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        }
        _ => Vec::new(),
    };
    let mut migrations = Migrations::new();
    for (key, step) in entries {
        // Only a canonical integer key is ever looked up (`String(at)`).
        let Some(at) = key.parse::<u64>().ok().filter(|n| n.to_string() == key) else {
            continue;
        };
        let (calls, threw) = (calls.clone(), threw.clone());
        migrations.insert(
            at,
            Box::new(move |doc: SavedObject| {
                calls.borrow_mut().push(key.clone());
                let plain = as_number(&step).is_none()
                    && string_units(&step).is_none()
                    && !is_undefined(&step);
                match &step {
                    Value::String(s) if s == "unchanged" => Json::Object(unchanged(doc)),
                    Value::Object(o) if plain => match o.get("merge") {
                        Some(merge) => {
                            let mut out = doc;
                            for (k, v) in spread(&json_of(merge)) {
                                set_entry(&mut out, &k, v);
                            }
                            Json::Object(out)
                        }
                        None => o.get("value").map_or(Json::Null, json_of),
                    },
                    Value::Null => {
                        *threw.borrow_mut() = Some("null");
                        Json::Null
                    }
                    _ if is_undefined(&step) => {
                        *threw.borrow_mut() = Some("undefined");
                        Json::Null
                    }
                    _ => Json::Null,
                }
            }),
        );
    }
    migrations
}

fn result(doc: Option<SavedObject>, calls: &Rc<RefCell<Vec<String>>>) -> Value {
    json!({ "result": doc.map_or(Value::Null, |doc| value_of(&Json::Object(doc))), "calls": *calls.borrow() })
}

/// Port of `upgradedCase` (`apps/browser/src/lib/savedDocument.harness.ts:18`).
pub fn upgraded_case(
    doc: Value,
    version: f64,
    steps: Value,
    field: Option<String>,
) -> Result<Value, String> {
    let (calls, threw) = Default::default();
    let migrations = migrations_of(&steps, &calls, &threw);
    let doc = upgraded(
        json_of(&doc),
        version,
        &migrations,
        field.as_deref().unwrap_or("version"),
    );
    match *threw.borrow() {
        Some(what) => Err(format!(
            "TypeError: Cannot read properties of {what} (reading 'value')"
        )),
        None => Ok(result(doc, &calls)),
    }
}

/// Port of `parseSavedCase` (`apps/browser/src/lib/savedDocument.harness.ts:23`).
pub fn parse_saved_case(
    json: Option<String>,
    version: f64,
    steps: Value,
    field: Option<String>,
) -> Result<Value, String> {
    let (calls, threw) = Default::default();
    let migrations = migrations_of(&steps, &calls, &threw);
    let doc = parse_saved(
        json.as_deref(),
        version,
        &migrations,
        field.as_deref().unwrap_or("version"),
    );
    Ok(result(doc, &calls))
}

/// Port of `versionedJsonCase` (`apps/browser/src/lib/savedDocument.harness.ts:28`).
pub fn versioned_json_case(
    data: Value,
    version: f64,
    field: Option<String>,
    space: Option<f64>,
) -> Result<String, String> {
    Ok(versioned_json(
        &json_of(&data),
        version,
        field.as_deref().unwrap_or("version"),
        space,
    ))
}

/// Port of `unchangedCase` (`apps/browser/src/lib/savedDocument.harness.ts:30`).
pub fn unchanged_case(doc: Value) -> Result<Value, String> {
    Ok(match json_of(&doc) {
        Json::Object(entries) => value_of(&Json::Object(unchanged(entries))),
        _ => doc,
    })
}
