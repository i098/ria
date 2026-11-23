// Ported from ardia packages/core/src/telemetryWire.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `identity::telemetry_wire`: it maps the JSON arguments of
//! the recording to typed values. The logic is in `ria_core`.

#![forbid(unsafe_code)]

use ria_core::identity::telemetry_wire::{
    self as wire, Json, LogLevel, OtlpLogRecord, QueuedEvent,
};
use serde_json::{Map, Value};
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

/// A JSON value of the recording as `Json`. A property that is `undefined` in JS is dropped
/// from an object and is `null` in an array, as `JSON.stringify` does.
fn json_of(value: &Value) -> Result<Json, String> {
    Ok(match value {
        Value::Null => Json::Null,
        Value::Bool(b) => Json::Bool(*b),
        Value::Number(n) => Json::Number(n.as_f64().ok_or("TypeError: number out of range")?),
        Value::String(s) => Json::String(s.clone()),
        Value::Array(items) => Json::Array(
            items
                .iter()
                .map(|v| {
                    if is_undefined(v) {
                        Ok(Json::Null)
                    } else {
                        json_of(v)
                    }
                })
                .collect::<Result<_, _>>()?,
        ),
        Value::Object(_) if is_undefined(value) => Json::Null,
        Value::Object(_) if special_number(value).is_some() => {
            Json::Number(special_number(value).unwrap_or(f64::NAN))
        }
        Value::Object(o) if o.contains_key("$utf16") => {
            return Err("a lone surrogate, which a Rust String cannot hold".into());
        }
        Value::Object(o) => Json::Object(entries_of(o)?),
    })
}

/// `NaN`, the infinities and `-0` cross as `{"$num": "..."}`.
fn special_number(value: &Value) -> Option<f64> {
    match value.get("$num")?.as_str()? {
        "NaN" => Some(f64::NAN),
        "Infinity" => Some(f64::INFINITY),
        "-Infinity" => Some(f64::NEG_INFINITY),
        "-0" => Some(-0.0),
        _ => None,
    }
}

fn is_undefined(value: &Value) -> bool {
    value.get("$undefined") == Some(&Value::Bool(true))
}

fn entries_of(object: &Map<String, Value>) -> Result<Vec<(String, Json)>, String> {
    let mut entries = Vec::new();
    for (key, value) in object {
        if !is_undefined(value) {
            wire::set_entry(&mut entries, key, json_of(value)?);
        }
    }
    Ok(entries)
}

/// `Object.entries(value)` of an `attributes` argument. A string gives one entry for each UTF-16
/// unit, an array one for each index, and a number or a boolean none; `null` and `undefined` throw.
fn object_entries(value: &Value) -> Result<Vec<(String, Json)>, String> {
    const NOT_OBJECT: &str = "TypeError: Cannot convert undefined or null to object";
    const SURROGATE: &str = "a lone surrogate, which a Rust String cannot hold";
    match value {
        Value::Object(_) if is_undefined(value) => Err(NOT_OBJECT.into()),
        Value::Object(_) if special_number(value).is_some() => Ok(vec![]),
        Value::Object(o) if o.contains_key("$utf16") => Err(SURROGATE.into()),
        Value::Object(o) => entries_of(o),
        Value::Null => Err(NOT_OBJECT.into()),
        Value::Bool(_) | Value::Number(_) => Ok(vec![]),
        Value::String(s) => {
            let mut entries = Vec::new();
            for (i, c) in s.chars().enumerate() {
                if c.len_utf16() != 1 {
                    return Err(SURROGATE.into());
                }
                entries.push((i.to_string(), Json::String(c.to_string())));
            }
            Ok(entries)
        }
        Value::Array(items) => {
            let mut entries = Vec::new();
            for (i, v) in items.iter().enumerate() {
                let json = if is_undefined(v) {
                    Json::Null
                } else {
                    json_of(v)?
                };
                entries.push((i.to_string(), json));
            }
            Ok(entries)
        }
    }
}

/// JS `ToString` of a value inside an array that `ToNumber` joins with commas. An object gives
/// `[object Object]`; `null` and `undefined` give an empty string; a lone surrogate never parses.
fn join_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => wire::js_number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items.iter().map(join_text).collect::<Vec<_>>().join(","),
        Value::Object(_) if is_undefined(value) => String::new(),
        Value::Object(_) => match special_number(value) {
            Some(n) => wire::js_number_to_string(n),
            None if value.get("$utf16").is_some() => "\u{FFFD}".into(),
            None => "[object Object]".into(),
        },
    }
}

/// JS `StringToNumber`: white space is trimmed, an empty string is 0, and `0x`, `0o` and `0b`
/// give integers; anything else that is not a decimal literal is NaN.
fn string_to_number(text: &str) -> f64 {
    let is_space = |c: char| c.is_whitespace() || c == '\u{FEFF}';
    let s = text.trim_matches(is_space);
    if s.is_empty() {
        return 0.0;
    }
    let radix = match s.get(..2) {
        Some("0x" | "0X") => 16,
        Some("0o" | "0O") => 8,
        Some("0b" | "0B") => 2,
        _ => 0,
    };
    if radix != 0 {
        let digits = &s[2..];
        if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
            return f64::NAN;
        }
        // shortcut: digits past 128 bits add up in f64, so the last bit can differ from JS.
        return u128::from_str_radix(digits, radix).map_or_else(
            |_| {
                digits.chars().fold(0.0, |n, c| {
                    n * f64::from(radix) + f64::from(c.to_digit(radix).unwrap_or(0))
                })
            },
            |n| n as f64,
        );
    }
    let (sign, rest) = match s.as_bytes()[0] {
        b'-' => (-1.0, &s[1..]),
        b'+' => (1.0, &s[1..]),
        _ => (1.0, s),
    };
    if rest == "Infinity" {
        return sign * f64::INFINITY;
    }
    let (mantissa, exponent) = rest
        .split_once(['e', 'E'])
        .map_or((rest, None), |(m, e)| (m, Some(e)));
    let (int, frac) = mantissa
        .split_once('.')
        .map_or((mantissa, None), |(i, f)| (i, Some(f)));
    let all_digits = |t: &str| t.bytes().all(|b| b.is_ascii_digit());
    let mantissa_ok = all_digits(int)
        && frac.is_none_or(all_digits)
        && !(int.is_empty() && frac.is_none_or(str::is_empty));
    let exponent_ok = exponent.is_none_or(|e| {
        let digits = e.strip_prefix(['+', '-']).unwrap_or(e);
        !digits.is_empty() && all_digits(digits)
    });
    if !mantissa_ok || !exponent_ok {
        return f64::NAN;
    }
    sign * rest.parse::<f64>().unwrap_or(f64::NAN)
}

/// `ToNumber` of the `at` argument of `logRecord`; `undefined` is the default `Date.now()`.
fn at_ms(at: &Value) -> Result<f64, String> {
    Ok(match at {
        Value::Null => 0.0,
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::Number(n) => n.as_f64().ok_or("TypeError: number out of range")?,
        Value::String(s) => string_to_number(s),
        Value::Array(_) => string_to_number(&join_text(at)),
        Value::Object(_) if is_undefined(at) => now_ms(),
        Value::Object(_) => special_number(at).unwrap_or(f64::NAN),
    })
}

fn string_field(object: &Map<String, Value>, key: &str) -> Result<String, String> {
    match object.get(key) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(format!("TypeError: `{key}` is not a string")),
    }
}

fn record_value(record: &OtlpLogRecord) -> Result<Value, String> {
    serde_json::from_str(&record.to_json().stringify()).map_err(|e| e.to_string())
}

/// Port of `eventRecord` (`packages/core/src/telemetryWire.ts:43`).
pub fn event_record(e: Value) -> Result<Value, String> {
    let object = e.as_object().ok_or("TypeError: event is not an object")?;
    let event = QueuedEvent {
        uuid: string_field(object, "uuid")?,
        event: string_field(object, "event")?,
        distinct_id: string_field(object, "distinct_id")?,
        timestamp: string_field(object, "timestamp")?,
        properties: object
            .get("properties")
            .filter(|v| !v.is_null() && !is_undefined(v)) // `...null` spreads nothing
            .map_or(Ok(vec![]), object_entries)?,
    };
    record_value(&wire::event_record(&event, now_ms()))
}

/// Port of `logRecord` (`packages/core/src/telemetryWire.ts:54`).
pub fn log_record(
    level: String,
    message: String,
    attributes: Value,
    at: Value,
) -> Result<Value, String> {
    let at_ms = at_ms(&at)?;
    let level = if level == "error" {
        LogLevel::Error
    } else {
        LogLevel::Warn
    };
    record_value(&wire::log_record(
        level,
        &message,
        &object_entries(&attributes)?,
        at_ms,
    ))
}

/// Port of `logsRequest` (`packages/core/src/telemetryWire.ts:64`).
pub fn logs_request(resource: Value, scope: String, records: Value) -> Result<String, String> {
    // `logRecords` takes any JSON value; `undefined` leaves the key out.
    let records = if is_undefined(&records) {
        None
    } else {
        Some(json_of(&records)?)
    };
    Ok(wire::logs_request_of(
        &object_entries(&resource)?,
        &scope,
        records,
    ))
}

/// Port of `outcome` (`packages/core/src/telemetryWire.ts:72`).
pub fn outcome(status: f64, count: f64) -> Result<String, String> {
    Ok(wire::outcome(status, count).as_str().to_owned())
}
