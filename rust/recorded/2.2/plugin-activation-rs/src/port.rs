// Ported from ardia apps/browser/src/lib/pluginActivation.harness.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_activation`, through `pluginActivation.harness.ts`. The logic is in
//! `ria_state::shared::plugin_activation`.

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

use ria_state::shared::plugin_activation as k;
use ria_state::shared::plugin_checks::Check;
use ria_state::shared::plugin_contributions::contributes_of;
use serde_json::json;

/// Port of `constantsCase` (`apps/browser/src/lib/pluginActivation.harness.ts:6`).
pub fn constants_case() -> Result<Value, String> {
    Ok(
        json!({ "STARTUP_EVENTS": k::STARTUP_EVENTS, "ACTIVATION_PREFIXES": k::ACTIVATION_PREFIXES }),
    )
}

/// Port of `activationEventProblemCase` (`apps/browser/src/lib/pluginActivation.harness.ts:7`).
pub fn activation_event_problem_case(event: Value) -> Result<Option<String>, String> {
    Ok(k::activation_event_problem(json_in(&event).as_ref()))
}

/// Port of `activationEventsOfCase` (`apps/browser/src/lib/pluginActivation.harness.ts:8`).
pub fn activation_events_of_case(value: Value) -> Result<Value, String> {
    let mut c = Check::default();
    let value = json_in(&value);
    let result = k::activation_events_of(&mut c, value.as_ref());
    Ok(json!({ "result": result, "problems": c.problems }))
}

/// Port of `derivedActivationEventsCase` (`apps/browser/src/lib/pluginActivation.harness.ts:12`).
pub fn derived_activation_events_case(
    contributes: Value,
    main: bool,
    sidecar: bool,
) -> Result<Vec<String>, String> {
    let contributes = json_in(&contributes);
    let k_ = contributes_of(&mut Check::default(), contributes.as_ref(), main);
    Ok(k::derived_activation_events(&k_, main, sidecar))
}

/// Port of `startsEagerlyCase` (`apps/browser/src/lib/pluginActivation.harness.ts:14`).
pub fn starts_eagerly_case(events: Vec<String>) -> Result<bool, String> {
    Ok(k::starts_eagerly(&events))
}

/// Port of `urlActivationHostsCase` (`apps/browser/src/lib/pluginActivation.harness.ts:15`).
pub fn url_activation_hosts_case(events: Vec<String>) -> Result<Vec<String>, String> {
    Ok(k::url_activation_hosts(&events))
}
