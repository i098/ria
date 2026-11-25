// Ported from ardia apps/browser/src/lib/pluginPageBar.harness.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_page_bar`, through `pluginPageBar.harness.ts`. The logic is in
//! `ria_state::shared::plugin_page_bar`.

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
use ria_state::shared::plugin_page_bar as k;
use serde_json::json;

/// Port of `constantsCase` (`apps/browser/src/lib/pluginPageBar.harness.ts:6`).
pub fn constants_case() -> Result<Value, String> {
    let actions: serde_json::Map<String, Value> = k::HOST_ACTIONS
        .iter()
        .map(|a| {
            (
                a.name.to_owned(),
                json!({ "title": a.title, "consent": a.consent }),
            )
        })
        .collect();
    let l = k::PAGE_BAR_LIMITS;
    Ok(json!({
        "HOST_ACTIONS": actions,
        "HOST_ACTION_NAMES": k::HOST_ACTION_NAMES,
        "BUTTON_DIVIDER": k::BUTTON_DIVIDER,
        "PAGE_BAR_LOOKS": k::PAGE_BAR_LOOKS,
        "PAGE_BAR_LIMITS": { "siteSwitches": l.site_switches, "buttons": l.buttons, "switchTitle": l.switch_title },
    }))
}

/// Port of `isHostActionCase` (`apps/browser/src/lib/pluginPageBar.harness.ts:7`).
pub fn is_host_action_case(value: Value) -> Result<bool, String> {
    Ok(k::is_host_action(json_in(&value).as_ref()))
}

/// Port of `pageBarCase` (`apps/browser/src/lib/pluginPageBar.harness.ts:8`).
pub fn page_bar_case(switches: Value, bar: Value, rows: Value) -> Result<Value, String> {
    let mut c = Check::default();
    let (switches, bar) = (json_in(&switches), json_in(&bar));
    fn text<'a>(r: &'a Value, key: &str) -> Option<&'a str> {
        r.get(key).and_then(Value::as_str)
    }
    let rows: Vec<k::RowRef> = rows
        .as_array()
        .and_then(|rows| {
            let row = |r| {
                Some(k::RowRef {
                    key: text(r, "key")?,
                    r#type: text(r, "type")?,
                })
            };
            rows.iter().map(row).collect()
        })
        .ok_or("TypeError: rows must be a list of { key, type } text")?;
    let read = k::site_switches_of(&mut c, switches.as_ref());
    let spec = k::page_bar_of(&mut c, bar.as_ref(), &read, &rows);
    let consent = k::page_bar_consent(&read, spec.as_ref());
    Ok(
        json!({ "switches": to_js(&read)?, "bar": to_js(&spec)?, "problems": c.problems, "consent": to_js(&consent)? }),
    )
}

/// Port of `switchOriginCase` (`apps/browser/src/lib/pluginPageBar.harness.ts:14`).
pub fn switch_origin_case(url: String) -> Result<Option<String>, String> {
    Ok(k::switch_origin(&url))
}

/// Port of `isLocalOriginCase` (`apps/browser/src/lib/pluginPageBar.harness.ts:15`).
pub fn is_local_origin_case(origin: String) -> Result<bool, String> {
    Ok(k::is_local_origin(&origin))
}
