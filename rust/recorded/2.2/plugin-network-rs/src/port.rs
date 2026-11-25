// Ported from ardia apps/browser/src/lib/pluginNetwork.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_network`: it reads `unknown` arguments as
//! `Json`. The logic is in `ria_state::shared::plugin_network`.

#![forbid(unsafe_code)]

use ria_core::identity::telemetry_wire::{Json, js_order};
use ria_state::shared::plugin_network as n;
use serde_json::Value;

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

/// Port of `normalizedHost` (`apps/browser/src/lib/pluginNetwork.ts:8`).
pub fn normalized_host(host: String) -> Result<String, String> {
    Ok(n::normalized_host(&host))
}

/// Port of `hostPatternProblem` (`apps/browser/src/lib/pluginNetwork.ts:21`).
pub fn host_pattern_problem(pattern: Value) -> Result<Option<String>, String> {
    Ok(n::host_pattern_problem(&json_or_null(&pattern)).map(str::to_owned))
}

/// Port of `isHostPattern` (`apps/browser/src/lib/pluginNetwork.ts:38`).
pub fn is_host_pattern(pattern: Value) -> Result<bool, String> {
    Ok(n::is_host_pattern(&json_or_null(&pattern)))
}

/// Port of `matchesAllowedHost` (`apps/browser/src/lib/pluginNetwork.ts:40`).
pub fn matches_allowed_host(host: String, allowed_hosts: Vec<String>) -> Result<bool, String> {
    let hosts: Vec<&str> = allowed_hosts.iter().map(String::as_str).collect();
    Ok(n::matches_allowed_host(&host, &hosts))
}

/// Port of `hostPatternCovers` (`apps/browser/src/lib/pluginNetwork.ts:54`).
pub fn host_pattern_covers(broader: String, narrower: String) -> Result<bool, String> {
    Ok(n::host_pattern_covers(&broader, &narrower))
}
