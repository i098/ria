// Ported from ardia apps/browser/src/lib/pluginWebUrl.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_web_url`: the result is `{ url, host }` or
//! `{ problem }`. The logic is in `ria_state::shared::plugin_web_url`.

#![forbid(unsafe_code)]

use ria_core::identity::telemetry_wire::{Json, js_order};
use ria_state::shared::plugin_web_url::{PluginWebUrl, plugin_web_url as check};
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

/// Port of `pluginWebUrl` (`apps/browser/src/lib/pluginWebUrl.ts:19`).
pub fn plugin_web_url(raw: Value) -> Result<Value, String> {
    Ok(match check(&json_or_null(&raw)) {
        PluginWebUrl::Url { url, host } => json!({ "url": url, "host": host }),
        PluginWebUrl::Problem(problem) => json!({ "problem": problem }),
    })
}
