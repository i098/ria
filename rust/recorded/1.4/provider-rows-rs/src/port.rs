// Ported from ardia packages/core/src/providerRows.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `search::provider_rows`: it maps the JSON arguments of the
//! recording to typed values and gives the original's JS errors. The logic is in
//! `ria_core::search::provider_rows`.

#![forbid(unsafe_code)]

use ria_core::search::provider_rows::{
    self, JsValue, PROVIDER_LIMITS, ProviderRowInput, ProviderSection, ProviderSuggestion,
    ShownSuggestion,
};
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

/// A [`JsValue`] back as the JSON of the recording.
fn json(value: &JsValue) -> Value {
    match value {
        JsValue::Undefined => crate::js::undefined(),
        JsValue::Null => Value::Null,
        JsValue::Bool(b) => Value::Bool(*b),
        JsValue::Number(n) => crate::js::number(*n),
        JsValue::String(s) => Value::String(s.clone()),
        JsValue::Array(items) => Value::Array(items.iter().map(json).collect()),
        JsValue::Object(fields) => {
            Value::Object(fields.iter().map(|(k, v)| (k.clone(), json(v))).collect())
        }
    }
}

/// How JS prints a primitive in an error message.
fn shown(value: &JsValue) -> String {
    match value {
        JsValue::Undefined => "undefined".into(),
        JsValue::Null => "null".into(),
        JsValue::Bool(b) => b.to_string(),
        JsValue::Number(n) => crate::js::number_to_string(*n),
        JsValue::String(s) => s.clone(),
        JsValue::Array(_) | JsValue::Object(_) => "[object Object]".into(),
    }
}

/// The JS error for reading `key` of `null` or `undefined`.
fn read_error(value: &JsValue, key: &str) -> String {
    format!(
        "TypeError: Cannot read properties of {} (reading '{key}')",
        shown(value)
    )
}

fn is_nullish(value: &JsValue) -> bool {
    matches!(value, JsValue::Undefined | JsValue::Null)
}

/// A `ProviderSuggestion` as the original builds it; `provider_id` is the section's `id` as given.
fn suggestion(row: &ProviderSuggestion, provider_id: &Value) -> Value {
    let mut o = Map::new();
    o.insert("kind".into(), "provider".into());
    o.insert("providerId".into(), provider_id.clone());
    o.insert("provider".into(), row.provider.clone().into());
    o.insert("title".into(), row.title.clone().into());
    o.insert("url".into(), row.url.clone().into());
    o.insert("icon".into(), row.icon.clone().into());
    if let Some(subtitle) = &row.subtitle {
        o.insert("subtitle".into(), subtitle.clone().into());
    }
    Value::Object(o)
}

/// Port of `providerText` (`packages/core/src/providerRows.ts:29`).
pub fn provider_text(value: Value, max: f64) -> Result<String, String> {
    Ok(provider_rows::provider_text(js_value(&value).as_str(), max))
}

/// Port of `providerUrl` (`packages/core/src/providerRows.ts:36`).
pub fn provider_url(value: Value) -> Result<Option<String>, String> {
    Ok(provider_rows::provider_url(js_value(&value).as_str()).map(str::to_string))
}

/// Port of `providerIcon` (`packages/core/src/providerRows.ts:44`).
pub fn provider_icon(value: Value) -> Result<Option<String>, String> {
    Ok(provider_rows::provider_icon(js_value(&value).as_str()).map(str::to_string))
}

/// Port of `safeProviderRows` (`packages/core/src/providerRows.ts:50`). `providerId` is the
/// section's `id` as given, whatever its type.
pub fn safe_provider_rows(
    section: Value,
    rows: Value,
    limit: Option<f64>,
) -> Result<Vec<Value>, String> {
    let rows = js_value(&rows);
    if !matches!(rows, JsValue::Array(_)) {
        return Ok(vec![]);
    }
    let section = js_value(&section);
    if is_nullish(&section) {
        return Err(read_error(&section, "title"));
    }
    let title = section.get("title").as_str().unwrap_or("");
    let limit = limit.unwrap_or(PROVIDER_LIMITS.rows_per_provider as f64);
    let id = json(section.get("id"));
    Ok(provider_rows::safe_provider_rows("", title, &rows, limit)
        .iter()
        .map(|row| suggestion(row, &id))
        .collect())
}

/// Port of `fillProviderTemplate` (`packages/core/src/providerRows.ts:79`).
pub fn fill_provider_template(template: String, query: String) -> Result<Option<String>, String> {
    Ok(provider_rows::fill_provider_template(&template, &query))
}

/// Port of `parseProviderReply` (`packages/core/src/providerRows.ts:85`).
pub fn parse_provider_reply(body: Value, template: Option<String>) -> Result<Vec<Value>, String> {
    Ok(
        provider_rows::parse_provider_reply(&js_value(&body), template.as_deref())
            .iter()
            .map(json)
            .collect(),
    )
}

/// `(row[key] ?? "").toLowerCase()` as far as it can fail: the string, or the JS error.
fn row_text(row: &JsValue, key: &str) -> Result<String, String> {
    match row.get(key) {
        JsValue::Undefined | JsValue::Null => Ok(String::new()),
        JsValue::String(s) => Ok(s.clone()),
        _ => Err(format!(
            "TypeError: (row.{key} ?? \"\").toLowerCase is not a function"
        )),
    }
}

/// Port of `matchProviderRows` (`packages/core/src/providerRows.ts:104`): the original rows, in
/// the order the port ranks them.
pub fn match_provider_rows(rows: Value, text: String) -> Result<Vec<Value>, String> {
    let has_tokens = text
        .split(ria_core::search::js_text::is_js_space)
        .any(|t| !t.is_empty());
    let list = match (js_value(&rows), &rows) {
        (JsValue::Array(_), Value::Array(list)) => list,
        (JsValue::String(s), _) if !has_tokens => {
            return Ok(s.chars().map(|c| Value::String(c.into())).collect());
        }
        _ if !has_tokens => return Err("TypeError: rows is not iterable".into()),
        (v, _) if is_nullish(&v) => return Err(read_error(&v, "forEach")),
        _ => return Err("TypeError: rows.forEach is not a function".into()),
    };
    let typed = list
        .iter()
        .map(|row| {
            let row = js_value(row);
            if is_nullish(&row) {
                return Err(read_error(&row, "title"));
            }
            let title = row_text(&row, "title")?;
            let subtitle = row_text(&row, "subtitle")?;
            Ok(ProviderRowInput {
                title,
                subtitle: Some(subtitle),
                url: row_text(&row, "url")?,
                icon: None,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(provider_rows::match_provider_rows(&typed, &text)
        .into_iter()
        .filter_map(|hit| typed.iter().position(|r| std::ptr::eq(r, hit)))
        .map(|i| list[i].clone())
        .collect())
}

/// The items `for (const x of value)` visits: an array's items or a string's code points.
fn iterate(value: &JsValue, name: &str) -> Result<Vec<JsValue>, String> {
    match value {
        JsValue::Array(items) => Ok(items.clone()),
        JsValue::String(s) => Ok(s.chars().map(|c| JsValue::String(c.into())).collect()),
        _ => Err(format!("TypeError: {name} is not iterable")),
    }
}

/// Port of `globalProviderRows` (`packages/core/src/providerRows.ts:119`). Each section's
/// `id` crosses to the port as its index and comes back as given, whatever its type.
pub fn global_provider_rows(sections: Value, shown_rows: Value) -> Result<Vec<Value>, String> {
    let shown_rows = iterate(&js_value(&shown_rows), "shown")?;
    let mut shown_typed = Vec::new();
    for s in &shown_rows {
        let JsValue::Object(fields) = s else {
            if let JsValue::Array(_) = s {
                continue;
            }
            return Err(format!(
                "TypeError: Cannot use 'in' operator to search for 'url' in {}",
                shown(s)
            ));
        };
        if !fields.iter().any(|(k, _)| k == "url") {
            continue;
        }
        let kind = s.get("kind").as_str().unwrap_or("");
        if kind == "search" {
            continue;
        }
        match s.get("url") {
            JsValue::String(url) => shown_typed.push(ShownSuggestion {
                kind,
                url: Some(url),
            }),
            url if is_nullish(url) => return Err(read_error(url, "replace")),
            _ => return Err("TypeError: url.replace is not a function".into()),
        }
    }
    let sections = iterate(&js_value(&sections), "sections")?;
    let mut ids = Vec::new();
    let mut typed = Vec::new();
    for (i, section) in sections.iter().enumerate() {
        if is_nullish(section) {
            return Err(read_error(section, "rows"));
        }
        ids.push(json(section.get("id")));
        typed.push(ProviderSection {
            id: i.to_string(),
            title: section.get("title").as_str().unwrap_or("").to_string(),
            rows: section.get("rows").clone(),
        });
    }
    Ok(provider_rows::global_provider_rows(&typed, &shown_typed)
        .iter()
        .map(|row| {
            let id = row
                .provider_id
                .parse::<usize>()
                .ok()
                .and_then(|i| ids.get(i))
                .unwrap_or(&Value::Null);
            suggestion(row, id)
        })
        .collect())
}
