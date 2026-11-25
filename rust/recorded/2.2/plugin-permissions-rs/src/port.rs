// Ported from ardia apps/browser/src/lib/pluginPermissions.harness.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_permissions`, through `pluginPermissions.harness.ts`. The logic is in
//! `ria_state::shared::plugin_permissions`.

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

/// A serialized port value as a JS value: each number as JS gives it.
fn js_numbers(v: Value) -> Value {
    match v {
        Value::Number(n) => n.as_f64().map_or(Value::Null, crate::js::number),
        Value::Array(items) => Value::Array(items.into_iter().map(js_numbers).collect()),
        Value::Object(o) => Value::Object(o.into_iter().map(|(k, x)| (k, js_numbers(x))).collect()),
        v => v,
    }
}

/// A port value as the JS object the TS gives.
fn to_js<T: serde::Serialize>(t: &T) -> Result<Value, String> {
    serde_json::to_value(t)
        .map(js_numbers)
        .map_err(|e| e.to_string())
}
use ria_state::shared::plugin_checks::Check;
use ria_state::shared::plugin_permissions as k;
use serde_json::json;

/// Port of `constantsCase` (`apps/browser/src/lib/pluginPermissions.harness.ts:6`).
pub fn constants_case() -> Result<Value, String> {
    Ok(json!({
        "PLUGIN_PERMISSIONS": k::PLUGIN_PERMISSIONS,
        "PERMISSION_KEYS": k::PERMISSION_KEYS,
        "TAB_ACCESS": k::TAB_ACCESS,
        "NO_PERMISSIONS": to_js(&k::NO_PERMISSIONS)?,
    }))
}

/// Port of `legacyPermissionsCase` (`apps/browser/src/lib/pluginPermissions.harness.ts:7`).
pub fn legacy_permissions_case(given: Value) -> Result<Value, String> {
    let given = json_in(&given);
    Ok(k::legacy_permissions(given.as_ref()).map_or_else(crate::js::undefined, |v| json_out(&v)))
}

/// Port of `permissionsCase` (`apps/browser/src/lib/pluginPermissions.harness.ts:8`).
pub fn permissions_case(value: Value) -> Result<Value, String> {
    let mut c = Check::default();
    let value = json_in(&value);
    let p = k::permissions_of(&mut c, value.as_ref());
    Ok(json!({
        "result": to_js(&p)?,
        "problems": c.problems,
        "granted": k::granted_permissions(&p),
        "canReadTabs": k::can_read_tabs(&p),
        "canOpenTabs": k::can_open_tabs(&p),
        "hasNetwork": k::has_network(&p),
        "allowedHosts": k::allowed_hosts_of(&p),
    }))
}

/// Port of `missingPermissionsCase` (`apps/browser/src/lib/pluginPermissions.harness.ts:13`).
pub fn missing_permissions_case(caller: Value, callee: Value) -> Result<Vec<String>, String> {
    let (caller, callee) = (json_in(&caller), json_in(&callee));
    let caller = k::permissions_of(&mut Check::default(), caller.as_ref());
    let callee = k::permissions_of(&mut Check::default(), callee.as_ref());
    Ok(k::missing_permissions(&caller, &callee))
}
