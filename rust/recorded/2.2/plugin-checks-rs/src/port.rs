// Ported from ardia apps/browser/src/lib/pluginChecks.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_checks`, through `pluginChecks.harness.ts`:
//! each check case gives the result and the problems it added; `matching` takes a pattern by
//! its export name. The logic is in `ria_state::shared::plugin_checks`.

#![forbid(unsafe_code)]

use crate::js::number;
use regex::Regex;
use ria_core::identity::telemetry_wire::{Json, js_order};
use ria_state::shared::errors::js_string;
use ria_state::shared::plugin_checks::{self as k, Check, Key};
use serde_json::{Map, Value, json};

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

/// `Json` back as a JS value.
fn json_out(v: &Json) -> Value {
    match v {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => crate::js::number(*n),
        Json::String(s) => Value::String(s.clone()),
        Json::Array(items) => Value::Array(items.iter().map(json_out).collect()),
        Json::Object(entries) => Value::Object(
            entries
                .iter()
                .map(|(k, x)| (k.clone(), json_out(x)))
                .collect(),
        ),
    }
}

fn out(c: Check, result: Value) -> Result<Value, String> {
    Ok(json!({ "result": result, "problems": c.problems }))
}

fn text_out(text: Option<&str>) -> Value {
    text.map_or(Value::Null, |t| Value::String(t.to_owned()))
}

/// `LIMITS` as the TS object, in its key order.
fn limits() -> Map<String, Value> {
    let l = k::LIMITS;
    let pair = |p: [f64; 2]| json!([number(p[0]), number(p[1])]);
    let entries = [
        ("name", json!(l.name)),
        ("title", json!(l.title)),
        ("description", json!(l.description)),
        ("pageTitle", json!(l.page_title)),
        ("rows", json!(l.rows)),
        ("rowText", json!(l.row_text)),
        ("choices", pair(l.choices)),
        ("commands", json!(l.commands)),
        ("keywords", json!(l.keywords)),
        ("keyword", json!(l.keyword)),
        ("pluginKeywords", json!(l.plugin_keywords)),
        ("example", json!(l.example)),
        ("telemetryItems", json!(l.telemetry_items)),
        ("telemetryItem", json!(l.telemetry_item)),
        ("tools", json!(l.tools)),
        ("toolTimeout", pair(l.tool_timeout)),
        ("argv", json!(l.argv)),
        ("argBytes", json!(l.arg_bytes)),
        ("memoryMB", pair(l.memory_mb)),
        ("programs", json!(l.programs)),
        ("afterUpdate", pair(l.after_update)),
        ("hooks", json!(l.hooks)),
        ("menuItems", json!(l.menu_items)),
        ("toolbar", json!(l.toolbar)),
        ("sidebarViews", json!(l.sidebar_views)),
        ("panels", json!(l.panels)),
        ("omnibox", json!(l.omnibox)),
        ("themes", json!(l.themes)),
        ("activationEvents", json!(l.activation_events)),
        ("allowedHosts", json!(l.allowed_hosts)),
        ("when", json!(l.when)),
        ("tooltip", json!(l.tooltip)),
    ];
    entries
        .into_iter()
        .map(|(key, v)| (key.to_owned(), v))
        .collect()
}

/// `limitOf` of the harness: a count limit by its `LIMITS` name, else `LIMITS.name`.
fn limit(name: &str) -> usize {
    limits()
        .get(name)
        .and_then(Value::as_u64)
        .map_or(k::LIMITS.name, |n| n as usize)
}

fn pattern(name: &str) -> &'static Regex {
    match name {
        "KEY" => &k::KEY,
        "VERSION" => &k::VERSION,
        "SYMBOL" => &k::SYMBOL,
        "KEYWORD" => &k::KEYWORD,
        "UNSAFE_TEXT" => &k::UNSAFE_TEXT,
        _ => &k::NAME,
    }
}

/// Port of `constantsCase` (`apps/browser/src/lib/pluginChecks.harness.ts:18`).
pub fn constants_case() -> Result<Value, String> {
    Ok(json!({
        "LIMITS": limits(),
        "DEFAULT_SIDECAR_MB": number(k::DEFAULT_SIDECAR_MB),
        "DEFAULT_MAIN_MB": number(k::DEFAULT_MAIN_MB),
        "NAME_RULE": k::NAME_RULE,
        "KEY_RULE": k::KEY_RULE,
        "SYMBOL_RULE": k::SYMBOL_RULE,
        "NAME_PATTERN": k::NAME_PATTERN,
        "KEY_PATTERN": k::KEY_PATTERN,
        "VERSION_PATTERN": k::VERSION_PATTERN,
        "SYMBOL_PATTERN": k::SYMBOL_PATTERN,
        "KEYWORD_PATTERN": k::KEYWORD_PATTERN,
    }))
}

/// Port of `patternCase` (`apps/browser/src/lib/pluginChecks.harness.ts:19`).
pub fn pattern_case(name: String, value: String) -> Result<bool, String> {
    Ok(pattern(&name).is_match(&value))
}

/// Port of `atCase` (`apps/browser/src/lib/pluginChecks.harness.ts:20`).
/// A key outside `string | number` is turned to text as JS does, and an empty `path` gives it back.
pub fn at_case(path: String, key: Value) -> Result<Value, String> {
    if let Some(i) = crate::js::as_number(&key) {
        return Ok(Value::String(k::at(&path, Key::Index(i))));
    }
    if path.is_empty() {
        return Ok(key);
    }
    let name = json_in(&key).map_or("undefined".to_owned(), |v| js_string(&v));
    Ok(Value::String(k::at(&path, name.as_str())))
}

/// Port of `onlyCase` (`apps/browser/src/lib/pluginChecks.harness.ts:21`).
pub fn only_case(path: String, doc: Value, keys: Vec<String>) -> Result<Value, String> {
    let mut c = Check::default();
    // `Object.keys` of an array or a string gives its indexes; of other primitives, none.
    let indexes = |n: usize| (0..n).map(|i| (i.to_string(), Json::Null)).collect();
    let doc = match json_in(&doc) {
        Some(Json::Object(fields)) => fields,
        Some(Json::Array(items)) => indexes(items.len()),
        Some(Json::String(s)) => indexes(s.encode_utf16().count()),
        None | Some(Json::Null) => {
            return Err("TypeError: Cannot convert undefined or null to object".into());
        }
        _ => Vec::new(),
    };
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    k::only(&mut c, &path, &doc, &keys);
    out(c, crate::js::undefined())
}

/// Port of `textCase` (`apps/browser/src/lib/pluginChecks.harness.ts:22`).
pub fn text_case(
    path: String,
    value: Value,
    limit_name: String,
    required: bool,
) -> Result<Value, String> {
    let mut c = Check::default();
    let value = json_in(&value);
    let result = text_out(k::text(
        &mut c,
        &path,
        value.as_ref(),
        limit(&limit_name),
        required,
    ));
    out(c, result)
}

/// Port of `matchingCase` (`apps/browser/src/lib/pluginChecks.harness.ts:23`).
pub fn matching_case(
    path: String,
    value: Value,
    pattern_name: String,
    rule: String,
    required: bool,
) -> Result<Value, String> {
    let mut c = Check::default();
    let value = json_in(&value);
    let found = k::matching(
        &mut c,
        &path,
        value.as_ref(),
        pattern(&pattern_name),
        &rule,
        required,
    );
    let result = text_out(found);
    out(c, result)
}

/// Port of `listCase` (`apps/browser/src/lib/pluginChecks.harness.ts:24`).
pub fn list_case(path: String, value: Value, limit_name: String) -> Result<Value, String> {
    let mut c = Check::default();
    let value = json_in(&value);
    let items = k::list(&mut c, &path, value.as_ref(), limit(&limit_name));
    let result = Value::Array(items.iter().map(json_out).collect());
    out(c, result)
}

/// Port of `objectCase` (`apps/browser/src/lib/pluginChecks.harness.ts:25`).
pub fn object_case(path: String, value: Value) -> Result<Value, String> {
    let mut c = Check::default();
    let value = json_in(&value);
    let result = k::object(&mut c, &path, value.as_ref())
        .map_or(Value::Null, |doc| json_out(&Json::Object(doc.clone())));
    out(c, result)
}

/// Port of `integerCase` (`apps/browser/src/lib/pluginChecks.harness.ts:26`).
pub fn integer_case(
    path: String,
    value: Value,
    min: f64,
    max: f64,
    fallback: f64,
) -> Result<Value, String> {
    let mut c = Check::default();
    let value = json_in(&value);
    let result = k::integer(&mut c, &path, value.as_ref(), [min, max], fallback);
    out(c, crate::js::number(result))
}

/// Port of `flagCase` (`apps/browser/src/lib/pluginChecks.harness.ts:27`).
pub fn flag_case(path: String, value: Value, fallback: bool) -> Result<Value, String> {
    let mut c = Check::default();
    let value = json_in(&value);
    let result = k::flag(&mut c, &path, value.as_ref(), fallback);
    out(c, json!(result))
}

/// Port of `uniqueCase` (`apps/browser/src/lib/pluginChecks.harness.ts:28`).
/// A name outside `string | null` is turned to text as JS does.
pub fn unique_case(path: String, names: Vec<Value>, what: String) -> Result<Value, String> {
    let mut c = Check::default();
    // shortcut: `undefined` and the text "undefined" count as one name; JS keeps them apart.
    let names = names.iter().map(|v| match json_in(v) {
        Some(Json::Null) => None,
        None => Some("undefined".to_owned()),
        Some(v) => Some(js_string(&v)),
    });
    k::unique(&mut c, &path, names, &what);
    out(c, crate::js::undefined())
}

/// Port of `symbolOfCase` (`apps/browser/src/lib/pluginChecks.harness.ts:29`).
pub fn symbol_of_case(path: String, value: Value, fallback: String) -> Result<Value, String> {
    let mut c = Check::default();
    let value = json_in(&value);
    let result = k::symbol_of(&mut c, &path, value.as_ref(), &fallback).to_owned();
    out(c, Value::String(result))
}

/// Port of `isPluginPathCase` (`apps/browser/src/lib/pluginChecks.harness.ts:30`).
pub fn is_plugin_path_case(value: String) -> Result<bool, String> {
    Ok(k::is_plugin_path(&value))
}

/// Port of `isArgumentCase` (`apps/browser/src/lib/pluginChecks.harness.ts:31`).
pub fn is_argument_case(arg: Value) -> Result<bool, String> {
    Ok(json_in(&arg).is_some_and(|a| k::is_argument(&a)))
}

/// Port of `pluginFileCase` (`apps/browser/src/lib/pluginChecks.harness.ts:32`).
pub fn plugin_file_case(
    path: String,
    value: Value,
    rule: String,
    extension: Option<String>,
) -> Result<Value, String> {
    let mut c = Check::default();
    let value = json_in(&value);
    let found = k::plugin_file(&mut c, &path, value.as_ref(), &rule, extension.as_deref());
    let result = text_out(found);
    out(c, result)
}
