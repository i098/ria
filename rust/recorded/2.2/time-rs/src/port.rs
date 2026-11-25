// Ported from ardia apps/browser/src/lib/time.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::time`. The logic is in `ria_state::shared::time`.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, number, number_to_string};
use ria_state::shared::js::js_number;
use ria_state::shared::time::{self as t, DurationUnit};
use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};

/// `Date.now()`; the test hook `RIA_NOW_MS` (see `clock-freeze.mjs`) fixes the clock for the
/// comparison with the original.
fn now_ms() -> f64 {
    if let Some(fixed) = std::env::var("RIA_NOW_MS")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        return fixed;
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_millis() as f64)
}

/// JS `ToNumber` of a value from JSON; arrays go through their `join(",")` text, as `Array.prototype.toString` does.
fn to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => f64::from(u8::from(*b)),
        _ => as_number(v).unwrap_or_else(|| js_number(&to_text(v))),
    }
}

/// JS `String(v)` of a value from JSON.
fn to_text(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|i| {
                if i.is_null() || is_undefined(i) {
                    String::new()
                } else {
                    to_text(i)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        _ => as_number(v).map_or_else(|| "[object Object]".into(), number_to_string),
    }
}

/// Port of `ago` (`apps/browser/src/lib/time.ts:3`).
pub fn ago(at: f64, now: Value) -> Result<String, String> {
    // `Option<Value>` would read `null` as a missing argument; JS defaults only `undefined`.
    let now = if is_undefined(&now) {
        now_ms()
    } else {
        to_number(&now)
    };
    Ok(t::ago(at, now))
}

/// Port of `durationParts` (`apps/browser/src/lib/time.ts:24`).
pub fn duration_parts(minutes: f64) -> Result<Value, String> {
    let (amount, unit) = t::duration_parts(minutes);
    Ok(json!({ "amount": number(amount), "unit": unit.name() }))
}

/// Port of `durationTitle` (`apps/browser/src/lib/time.ts:30`).
pub fn duration_title(minutes: f64) -> Result<String, String> {
    Ok(t::duration_title(minutes))
}

/// Port of `durationMinutes` (`apps/browser/src/lib/time.ts:35`). A unit that is not a
/// `DurationUnit` gives `null`, as Ardia's `NaN` lookup does.
pub fn duration_minutes(amount: String, unit: String) -> Result<Option<f64>, String> {
    let unit = match unit.as_str() {
        "minutes" => DurationUnit::Minutes,
        "hours" => DurationUnit::Hours,
        "days" => DurationUnit::Days,
        _ => return Ok(None),
    };
    Ok(t::duration_minutes(&amount, unit))
}
