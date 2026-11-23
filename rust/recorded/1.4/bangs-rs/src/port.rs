// Ported from ardia packages/core/src/bangs.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `search::bangs`: it maps the JSON arguments of the recording
//! to typed values and gives the original's JS values and errors. The logic is in
//! `ria_core::search::bangs`. The functions that take or return a `BangTable` stay in TS here;
//! `bangs-harness-rs` records them through `bangs.harness.ts`.

#![forbid(unsafe_code)]

use ria_core::search::bangs::{self, Json};
use serde_json::{Map, Value};

/// The index of an array-index key (`0` to `2^32 - 2`, no leading zero), which JS lists first.
fn array_index(key: &str) -> Option<u64> {
    let digits = !key.is_empty() && key.len() <= 10 && key.bytes().all(|b| b.is_ascii_digit());
    (digits && (key == "0" || !key.starts_with('0')))
        .then(|| key.parse().ok())
        .flatten()
        .filter(|&n| n < u64::from(u32::MAX))
}

/// A parsed value as JS holds it: objects list array-index keys first, ascending.
fn to_value(json: &Json) -> Value {
    match json {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => crate::js::number(*n),
        Json::String(s) => Value::String(s.clone()),
        Json::Array(items) => Value::Array(items.iter().map(to_value).collect()),
        Json::Object(members) => {
            let mut members: Vec<_> = members.iter().collect();
            members.sort_by_key(|(key, _)| array_index(key).unwrap_or(u64::MAX));
            let object: Map<String, Value> = members
                .into_iter()
                .map(|(key, value)| (key.clone(), to_value(value)))
                .collect();
            Value::Object(object)
        }
    }
}

/// Port of `parseLooseJson` (`packages/core/src/bangs.ts:40`).
pub fn parse_loose_json(text: String) -> Result<serde_json::Value, String> {
    bangs::parse_loose_json(&text)
        .map(|json| to_value(&json))
        .map_err(|e| format!("SyntaxError: {e}"))
}

/// Port of `isValidBangTemplate` (`packages/core/src/bangs.ts:71`).
pub fn is_valid_bang_template(template: String) -> Result<serde_json::Value, String> {
    Ok(bangs::is_valid_bang_template(&template).into())
}

/// Port of `staysOnSite` (`packages/core/src/bangs.ts:73`).
pub fn stays_on_site(url: String, template: String) -> Result<bool, String> {
    Ok(bangs::stays_on_site(&url, &template))
}

/// Port of `slimBangs` (`packages/core/src/bangs.ts:83`).
pub fn slim_bangs(json: String) -> Result<Option<String>, String> {
    Ok(bangs::slim_bangs(&json))
}

/// Port of `bangDocument` (`packages/core/src/bangs.ts:175`).
pub fn bang_document(etag: Option<String>, slim: String) -> Result<serde_json::Value, String> {
    Ok(bangs::bang_document(etag.as_deref(), &slim).into())
}

/// Port of `escapeSearchTerms` (`packages/core/src/bangs.ts:222`).
pub fn escape_search_terms(terms: String, in_query: bool) -> Result<String, String> {
    Ok(bangs::escape_search_terms(&terms, in_query))
}

/// Port of `fillBangTemplate` (`packages/core/src/bangs.ts:227`).
pub fn fill_bang_template(template: String, terms: String) -> Result<String, String> {
    Ok(bangs::fill_bang_template(&template, &terms))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objects_list_index_keys_first() {
        let text = r#"{"4294967295":1,"4294967294":2,"01":3,"-1":4,"1":5}"#;
        let keys: Vec<String> = match parse_loose_json(text.into()) {
            Ok(Value::Object(o)) => o.keys().cloned().collect(),
            other => panic!("{other:?}"),
        };
        assert_eq!(keys, ["1", "4294967294", "4294967295", "01", "-1"]);
    }

    /// `JSON.parse` makes `__proto__` an own key; the napi glue's `o[k] = v` drops it (known diff).
    #[test]
    fn proto_stays_an_own_key() {
        let text = r#"{"__proto__":1,"a":{"__proto__":[2]}}"#;
        let expected = serde_json::json!({ "__proto__": 1, "a": { "__proto__": [2] } });
        assert_eq!(parse_loose_json(text.into()), Ok(expected));
    }
}
