// Ported from ardia packages/sync/src/canon.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The canonical JSON of a value: object keys sorted, so equal values give equal text.
//!
//! Changes from Ardia: none. (`serde_json::Value` has no `undefined`, so the skip of
//! `undefined` members in `sortKeys` has nothing to skip.)

use super::js::{cmp_utf16, stringify};
use serde_json::{Map, Value};

fn sort_keys(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(sort_keys).collect()),
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_by(|a, b| cmp_utf16(a, b));
            let mut out = Map::new();
            for key in keys {
                out.insert(key.clone(), sort_keys(&map[key]));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

/// Port of `canonical` (`packages/sync/src/canon.ts:1`).
pub fn canonical(value: &Value) -> String {
    stringify(&sort_keys(value))
}
