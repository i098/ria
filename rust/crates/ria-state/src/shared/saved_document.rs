// Ported from ardia apps/browser/src/lib/savedDocument.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Saved JSON documents with a version field, upgraded step by step when read.
//!
//! Changes from Ardia:
//! - Documents are `ria_core::identity::telemetry_wire::Json`; `SavedObject` is the entries of an object.
//! - `Migrations` maps a version to its step. A step gets the document by value and cannot throw: a step
//!   that fails returns a value that is not an object, which reads as `None`. `parseSaved` in TS also
//!   catches a step that throws.
//! - Rust has no default arguments: callers pass `&Migrations::new()`, `"version"` and `None` (`space`).
//! - `versionedJson` spreads an object, array or string `data`; any other `data` spreads as nothing, as
//!   `null` does. A string with a character outside the BMP spreads into lone surrogates in JS; here each
//!   half is U+FFFD.
//! - `space` between 0 and 1 gives several lines with no indent, as V8 does (the spec gives one line).
//! - `JSON.parse` is `ria_core::search::bangs::parse_json`. Deep nesting recurses on the stack; JS
//!   `JSON.parse` takes any depth. shortcut: recursive conversion, upgrade when saved documents nest deeply.

use ria_core::identity::telemetry_wire::{Json, js_order, set_entry};
use ria_core::search::bangs::{Json as Parsed, parse_json};
use std::collections::HashMap;
use std::mem::take;

use super::json_schema::json_stringify;

/// `SavedObject` (`savedDocument.ts:3`).
pub type SavedObject = Vec<(String, Json)>;
/// One upgrade step: the document at version `n` to the document at `n + 1`.
pub type Migration = Box<dyn Fn(SavedObject) -> Json>;
/// `Migrations` (`savedDocument.ts:4`).
pub type Migrations = HashMap<u64, Migration>;

/// `unchanged` (`savedDocument.ts:6`): the step that changes nothing.
pub fn unchanged(doc: SavedObject) -> SavedObject {
    doc
}

/// `upgraded` (`savedDocument.ts:10`): `doc` at `version`, after each step from its saved version;
/// `None` for a document that is not an object, a bad or newer version, a missing step, or a step that
/// gives no object.
pub fn upgraded(
    doc: Json,
    version: f64,
    migrations: &Migrations,
    field: &str,
) -> Option<SavedObject> {
    let Json::Object(doc) = doc else { return None };
    let mut at = 0.0;
    if let Some((_, saved)) = doc.iter().find(|(key, _)| key == field) {
        let &Json::Number(saved) = saved else {
            return None;
        };
        if !is_safe_integer(saved) || saved < 0.0 {
            return None;
        }
        at = saved;
    }
    if at > version {
        return None;
    }
    let mut current = doc;
    while at < version {
        let migration = migrations.get(&(at as u64))?;
        let Json::Object(next) = migration(current) else {
            return None;
        };
        at += 1.0;
        // `{ ...next, [field]: at }`.
        current = js_order(&next).into_iter().cloned().collect();
        set_entry(&mut current, field, Json::Number(at));
    }
    Some(current)
}

/// `Number.isSafeInteger`.
fn is_safe_integer(n: f64) -> bool {
    n.fract() == 0.0 && n.abs() <= 9_007_199_254_740_991.0
}

/// `parseSaved` (`savedDocument.ts:30`): `upgraded` of the parsed text; `None` for no text or bad JSON.
pub fn parse_saved(
    json: Option<&str>,
    version: f64,
    migrations: &Migrations,
    field: &str,
) -> Option<SavedObject> {
    let mut parsed = parse_json(json.filter(|j| !j.is_empty())?).ok()?;
    upgraded(from_parsed(&mut parsed), version, migrations, field)
}

/// A parsed value as `Json`, taking its strings.
fn from_parsed(value: &mut Parsed) -> Json {
    match value {
        Parsed::Null => Json::Null,
        Parsed::Bool(b) => Json::Bool(*b),
        Parsed::Number(n) => Json::Number(*n),
        Parsed::String(s) => Json::String(take(s)),
        Parsed::Array(items) => Json::Array(items.iter_mut().map(from_parsed).collect()),
        Parsed::Object(entries) => Json::Object(
            entries
                .iter_mut()
                .map(|(k, v)| (take(k), from_parsed(v)))
                .collect(),
        ),
    }
}

/// `versionedJson` (`savedDocument.ts:39`): `data` with `field` set to `version`, as JSON. A `field` already
/// in `data` keeps its place. `space` is the indent of `JSON.stringify`.
pub fn versioned_json(data: &Json, version: f64, field: &str, space: Option<f64>) -> String {
    let mut entries: Vec<(String, Json)> = match data {
        Json::Object(entries) => js_order(entries).into_iter().cloned().collect(),
        Json::Array(items) => items
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v.clone()))
            .collect(),
        // `{ ..."ab" }` is `{ "0": "a", "1": "b" }`, one entry for each UTF-16 code unit.
        Json::String(s) => s
            .encode_utf16()
            .enumerate()
            .map(|(i, u)| (i.to_string(), Json::String(String::from_utf16_lossy(&[u]))))
            .collect(),
        _ => Vec::new(),
    };
    set_entry(&mut entries, field, Json::Number(version));
    // V8 writes several lines for any `space` above 0, indented by its whole part, at most 10.
    let gap = space
        .filter(|s| *s > 0.0)
        .map(|s| " ".repeat(s.min(10.0) as usize));
    json_stringify(&Json::Object(entries), gap.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    // `savedDocument.test.mjs`: a `__proto__` key is plain data. The napi bridge cannot show this, because
    // setting `__proto__` on a JS object sets its prototype.
    #[test]
    fn proto_key_is_data() {
        let doc = parse_saved(
            Some(r#"{"__proto__":{"polluted":true},"a":1}"#),
            0.0,
            &Migrations::new(),
            "version",
        );
        assert_eq!(
            Json::Object(doc.unwrap()).stringify(),
            r#"{"__proto__":{"polluted":true},"a":1}"#
        );
    }
}
