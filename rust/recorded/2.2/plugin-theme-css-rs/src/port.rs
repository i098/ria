// Ported from ardia apps/browser/src/lib/pluginThemeCss.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_theme_css`. `Object.entries` of `null` or
//! `undefined` throws a `TypeError`; of any other non-object it has no role names.

#![forbid(unsafe_code)]

use ria_core::identity::telemetry_wire::{Json, js_order};
use ria_state::shared::decode::Fields;
use ria_state::shared::plugin_theme_css as t;
use serde_json::{Value, json};

/// A JS value as `Json`, object keys in JS order; `None` for `undefined`. An `undefined` inside
/// an array or object is `null`; a string with lone surrogates loses them (`known-diffs.jsonl`).
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
        Value::Array(items) => Json::Array(items.iter().map(json_or_null).collect()),
        Value::Object(o) => {
            let entries: Vec<(String, Json)> = o
                .iter()
                .map(|(k, x)| (k.clone(), json_or_null(x)))
                .collect();
            Json::Object(js_order(&entries).into_iter().cloned().collect())
        }
        _ => Json::Null,
    })
}

fn json_or_null(v: &Value) -> Json {
    json_in(v).unwrap_or(Json::Null)
}

/// `Object.entries(roles)` as `Fields`.
fn roles_of(roles: Option<Json>) -> Result<Fields, String> {
    match roles {
        None | Some(Json::Null) => {
            Err("TypeError: Cannot convert undefined or null to object".into())
        }
        Some(Json::Object(entries)) => Ok(entries),
        Some(_) => Ok(Fields::new()),
    }
}

/// Port of `cssRoleName` (`apps/browser/src/lib/pluginThemeCss.ts:8`).
pub fn css_role_name(role: String) -> Result<Value, String> {
    Ok(json!(t::css_role_name(&role)))
}

/// Port of `cssRoleValue` (`apps/browser/src/lib/pluginThemeCss.ts:10`).
pub fn css_role_value(value: Value) -> Result<Option<String>, String> {
    Ok(t::css_role_value(&json_or_null(&value)))
}

/// Port of `pluginThemeVariables` (`apps/browser/src/lib/pluginThemeCss.ts:18`).
pub fn plugin_theme_variables(roles: Value) -> Result<Vec<Value>, String> {
    let roles = roles_of(json_in(&roles))?;
    Ok(t::plugin_theme_variables(&roles)
        .into_iter()
        .map(|(name, value)| json!([name, value]))
        .collect())
}

/// Port of `buildPluginThemeCss` (`apps/browser/src/lib/pluginThemeCss.ts:31`).
pub fn build_plugin_theme_css(theme: Value) -> Result<String, String> {
    let Some(Json::Object(theme)) = json_in(&theme) else {
        return Err("TypeError: Cannot read properties of the theme".into());
    };
    let field = |key: &str| theme.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());
    let scheme = match field("scheme") {
        Some(Json::String(s)) => s,
        _ => String::new(),
    };
    let roles = roles_of(field("roles"))?;
    Ok(t::build_plugin_theme_css(&t::PluginThemeRoles {
        scheme,
        roles,
    }))
}
