// Ported from ardia packages/core/src/providerConfig.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `search::provider_config`: it maps the JSON arguments of the
//! recording to typed values and gives the original's JS errors. The logic is in
//! `ria_core::search::provider_config`.

#![forbid(unsafe_code)]

use ria_core::search::provider_config::{self, ProviderEntry};
use ria_core::search::provider_rows::JsValue;
use serde_json::{Map, Value};

/// A JSON argument as the `unknown` value the TS code sees. Lone surrogates become U+FFFD.
fn js_value(value: &Value) -> JsValue {
    if crate::js::is_undefined(value) {
        return JsValue::Undefined;
    }
    if let Some(n) = crate::js::as_number(value) {
        return JsValue::Number(n);
    }
    if let Some(units) = crate::js::string_units(value) {
        return JsValue::String(String::from_utf16_lossy(&units));
    }
    match value {
        Value::Bool(b) => JsValue::Bool(*b),
        Value::Array(items) => JsValue::Array(items.iter().map(js_value).collect()),
        Value::Object(fields) => JsValue::Object(
            fields
                .iter()
                .map(|(k, v)| (k.clone(), js_value(v)))
                .collect(),
        ),
        // Numbers and strings are handled above.
        _ => JsValue::Null,
    }
}

/// JS `String(value)`, as a template literal prints it.
fn js_string(value: &JsValue) -> String {
    match value {
        JsValue::Undefined => "undefined".into(),
        JsValue::Null => "null".into(),
        JsValue::Bool(b) => b.to_string(),
        JsValue::Number(n) => crate::js::number_to_string(*n),
        JsValue::String(s) => s.clone(),
        JsValue::Array(items) => items
            .iter()
            .map(|v| match v {
                JsValue::Undefined | JsValue::Null => String::new(),
                v => js_string(v),
            })
            .collect::<Vec<_>>()
            .join(","),
        JsValue::Object(_) => "[object Object]".into(),
    }
}

/// Port of `isProviderKeyword` (`packages/core/src/providerConfig.ts:32`).
pub fn is_provider_keyword(value: Value) -> Result<bool, String> {
    Ok(js_value(&value)
        .as_str()
        .is_some_and(provider_config::is_provider_keyword))
}

/// Port of `isRemoteEndpoint` (`packages/core/src/providerConfig.ts:34`).
pub fn is_remote_endpoint(template: String) -> Result<bool, String> {
    Ok(provider_config::is_remote_endpoint(&template))
}

/// Port of `readProviderEntries` (`packages/core/src/providerConfig.ts:91`). Each entry is its
/// input object as given, with `rows` narrowed to the rows kept, as the original's spread does.
pub fn read_provider_entries(value: Value, name: Option<Value>) -> Result<Value, String> {
    let name = match name.as_ref().map(js_value) {
        None | Some(JsValue::Undefined) => "omniboxProviders".into(),
        Some(name) => js_string(&name),
    };
    let read = provider_config::read_provider_entries(&js_value(&value), &name);
    let items = value.as_array().map(Vec::as_slice).unwrap_or_default();
    let entries = read
        .entries
        .iter()
        .zip(&read.origins)
        .map(|(entry, origin)| {
            let Some(Value::Object(fields)) = items.get(origin.item) else {
                return Value::Null;
            };
            let mut out = fields.clone();
            if entry.rows.is_some() {
                let rows = fields.get("rows").and_then(Value::as_array);
                let kept = origin
                    .rows
                    .iter()
                    .filter_map(|&i| rows.and_then(|r| r.get(i)).cloned())
                    .collect();
                out.insert("rows".into(), Value::Array(kept));
            }
            Value::Object(out)
        })
        .collect();
    let mut out = Map::new();
    out.insert("entries".into(), Value::Array(entries));
    out.insert("problems".into(), read.problems.into());
    Ok(Value::Object(out))
}

/// Port of `isProviderEntryList` (`packages/core/src/providerConfig.ts:110`).
pub fn is_provider_entry_list(value: Value) -> Result<bool, String> {
    Ok(provider_config::is_provider_entry_list(&js_value(&value)))
}

/// A string field of an entry as `providerConfigs` reads it: `None` when nullish.
fn text_field(e: &JsValue, key: &str, error: &str) -> Result<Option<String>, String> {
    match e.get(key) {
        JsValue::Undefined | JsValue::Null => Ok(None),
        JsValue::String(s) => Ok(Some(s.clone())),
        _ => Err(format!("TypeError: {error}")),
    }
}

/// One entry of `providerConfigs` as typed input, with the JS errors of the reads the original
/// makes, in its order.
fn entry(e: &JsValue) -> Result<ProviderEntry, String> {
    if matches!(e, JsValue::Undefined | JsValue::Null) {
        let what = if *e == JsValue::Null {
            "null"
        } else {
            "undefined"
        };
        return Err(format!(
            "TypeError: Cannot read properties of {what} (reading 'keyword')"
        ));
    }
    let keyword = text_field(e, "keyword", "e.keyword?.toLowerCase is not a function")?;
    let title = match e.get("title") {
        JsValue::String(s) => Some(s.clone()),
        t if t.is_truthy() => return Err("TypeError: e.title.trim is not a function".into()),
        _ => None,
    };
    let endpoint_error = "(e.url ?? e.suggest ?? \"\").split is not a function";
    let (mut url, mut suggest) = (None, None);
    if title.as_deref().is_none_or(str::is_empty) {
        url = text_field(e, "url", endpoint_error)?;
        if is_nullish_or_missing(e, "url") {
            suggest = text_field(e, "suggest", endpoint_error)?;
        }
    }
    Ok(ProviderEntry {
        keyword,
        title,
        url,
        suggest,
        ..ProviderEntry::default()
    })
}

fn is_nullish_or_missing(e: &JsValue, key: &str) -> bool {
    matches!(e.get(key), JsValue::Undefined | JsValue::Null)
}

/// Port of `providerConfigs` (`packages/core/src/providerConfig.ts:119`). `rows`, `url` and
/// `suggest` are the entry's own values, whatever their type, as the original passes them on.
pub fn provider_configs(entries: Value) -> Result<Vec<Value>, String> {
    let list = match js_value(&entries) {
        JsValue::Array(_) => entries.as_array().cloned().unwrap_or_default(),
        JsValue::Null => {
            return Err("TypeError: Cannot read properties of null (reading 'map')".into());
        }
        JsValue::Undefined => {
            return Err("TypeError: Cannot read properties of undefined (reading 'map')".into());
        }
        _ => return Err("TypeError: entries.map is not a function".into()),
    };
    let values: Vec<JsValue> = list.iter().map(js_value).collect();
    let typed = values.iter().map(entry).collect::<Result<Vec<_>, _>>()?;
    let configs = provider_config::provider_configs(&typed);
    Ok(configs
        .iter()
        .zip(&list)
        .zip(&values)
        .map(|((config, raw), e)| {
            let own = |key: &str| raw.as_object().and_then(|o| o.get(key)).cloned();
            let mut o = Map::new();
            o.insert("id".into(), config.id.clone().into());
            o.insert("title".into(), config.title.clone().into());
            if let Some(keyword) = &config.keyword {
                o.insert("keyword".into(), keyword.clone().into());
            }
            o.insert("global".into(), config.global.into());
            let rows = own("rows")
                .filter(|_| !is_nullish_or_missing(e, "rows"))
                .unwrap_or_else(|| Value::Array(vec![]));
            o.insert("rows".into(), rows);
            for key in ["url", "suggest"] {
                if e.get(key).is_truthy()
                    && let Some(v) = own(key)
                {
                    o.insert(key.into(), v);
                }
            }
            Value::Object(o)
        })
        .collect())
}
