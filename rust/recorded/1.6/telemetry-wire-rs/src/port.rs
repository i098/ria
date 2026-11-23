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

fn object_entries(value: &Value) -> Result<Vec<(String, Json)>, String> {
    match value {
        Value::Object(o) => entries_of(o),
        _ => Err("TypeError: expected an object of attributes".into()),
    }
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
            .map_or(Ok(vec![]), object_entries)?,
    };
    record_value(&wire::event_record(&event, now_ms()))
}

/// Port of `logRecord` (`packages/core/src/telemetryWire.ts:54`).
pub fn log_record(
    level: String,
    message: String,
    attributes: Value,
    at: Option<Value>,
) -> Result<Value, String> {
    let at_ms = match at {
        None => now_ms(),
        Some(Value::Null) => 0.0,
        Some(ref v) if is_undefined(v) => now_ms(),
        Some(Value::Bool(b)) => f64::from(u8::from(b)),
        Some(Value::Number(n)) => n.as_f64().ok_or("TypeError: number out of range")?,
        Some(ref v) if special_number(v).is_some() => special_number(v).unwrap_or(f64::NAN),
        Some(Value::Object(_)) => f64::NAN,
        Some(Value::Array(a)) if a.is_empty() => 0.0,
        Some(_) => return Err("`at` is a string or a list, which `Math.round` coerces".into()),
    };
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
pub fn logs_request(resource: Value, scope: String, records: Vec<Value>) -> Result<String, String> {
    let records = records.iter().map(json_of).collect::<Result<Vec<_>, _>>()?;
    Ok(wire::logs_request(
        &object_entries(&resource)?,
        &scope,
        &records,
    ))
}

/// Port of `outcome` (`packages/core/src/telemetryWire.ts:72`).
pub fn outcome(status: f64, count: f64) -> Result<String, String> {
    Ok(wire::outcome(status, count).as_str().to_owned())
}
