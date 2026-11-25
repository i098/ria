// Ported from ardia apps/browser/src/lib/pluginContributions.harness.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_contributions`, through `pluginContributions.harness.ts`. The logic is in
//! `ria_state::shared::plugin_contributions`.

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
use ria_state::shared::plugin_contributions as k;
use serde_json::json;

/// Port of `constantsCase` (`apps/browser/src/lib/pluginContributions.harness.ts:6`).
pub fn constants_case() -> Result<Value, String> {
    Ok(json!({
        "MENU_SLOTS": k::MENU_SLOTS,
        "PANEL_MODES": k::PANEL_MODES,
        "CONTRIBUTES_KEYS": k::CONTRIBUTES_KEYS,
        "PAGE_PANEL_ID": k::PAGE_PANEL_ID,
        "emptyMenus": to_js(&k::empty_menus())?,
    }))
}

/// Port of `contributesCase` (`apps/browser/src/lib/pluginContributions.harness.ts:7`).
pub fn contributes_case(value: Value, has_main: bool) -> Result<Value, String> {
    let mut c = Check::default();
    let value = json_in(&value);
    let result = k::contributes_of(&mut c, value.as_ref(), has_main);
    Ok(json!({
        "result": to_js(&result)?,
        "problems": c.problems,
        "pagePanel": to_js(&k::page_panel_of(&result.panels))?,
        "viewIds": k::view_ids_of(&result),
    }))
}
