// Ported from ardia apps/browser/src/lib/fallback.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::fallback`. The harness (`fallback.harness.ts`) builds an `Error` from a value. The logic is in `ria_state::shared::fallback`.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, string_units};
use ria_core::identity::telemetry_wire::Json;
use ria_state::shared::errors::{Thrown, error_text};
use ria_state::shared::fallback::{PasswordAction, or_warn, or_warn_text, password_failure};
use serde_json::Value;
use serde_json::json;

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

/// The value the harness throws: `new Error(String(value))` when `as_error`.
fn thrown(value: &Value, as_error: bool) -> Thrown {
    let value = if is_undefined(value) {
        Thrown::Undefined
    } else {
        Thrown::Value(json_of(value))
    };
    if as_error {
        Thrown::Error(error_text(&value))
    } else {
        value
    }
}

/// Port of `passwordFailureCase` (`apps/browser/src/lib/fallback.harness.ts:7`).
pub fn password_failure_case(value: Value, as_error: bool) -> Result<String, String> {
    Ok(password_failure(&thrown(&value, as_error))
        .as_str()
        .to_owned())
}

/// Port of `orWarnCase` (`apps/browser/src/lib/fallback.harness.ts:11`).
pub fn or_warn_case(
    fallback: Value,
    action: Option<String>,
    value: Value,
    as_error: bool,
) -> Result<Value, String> {
    let action = match action.as_deref() {
        None => None,
        Some("browser import") => Some(PasswordAction::BrowserImport),
        Some("csv import") => Some(PasswordAction::CsvImport),
        Some("settings edit") => Some(PasswordAction::SettingsEdit),
        Some("settings delete") => Some(PasswordAction::SettingsDelete),
        Some("sync delete") => Some(PasswordAction::SyncDelete),
        Some(other) => return Ok(json!({ "error": format!("not a PasswordAction: {other}") })),
    };
    let error = thrown(&value, as_error);
    let result = or_warn(fallback, action)(&error);
    Ok(json!({ "result": result, "warned": [or_warn_text(action, &error)] }))
}
