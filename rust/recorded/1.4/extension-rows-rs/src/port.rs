// Ported from ardia packages/core/src/extensionRows.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `search::extension_rows`: it maps the JSON arguments of the
//! recording to typed values and gives the original's JS errors. The logic is in
//! `ria_core::search::extension_rows`.

#![forbid(unsafe_code)]

use ria_core::fuzzy;
use ria_core::search::extension_rows::{
    self as core, ExtensionCandidate, ExtensionOpen, Suggestion,
};
use ria_core::search::js_text::js_trim;
use serde_json::{Value, json};

use crate::js;

/// `!!v`.
fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Number(n)) => n.as_f64().is_some_and(|n| n != 0.0),
        Some(x) => match js::as_number(x) {
            Some(n) => n != 0.0 && !n.is_nan(),
            None => !js::is_undefined(x),
        },
    }
}

/// One extension as `matchActions` reads it (its title), with the JS errors of `fuzzy.ts`.
///
/// shortcut: `id`, `icon`, `action` and `options` are read leniently (a non-string `id` reads as
/// "", a non-string `icon` as null) and `keywords` is not read; upgrade when a caller passes such values.
fn candidate(v: &Value) -> Result<ExtensionCandidate, String> {
    let Some(o) = v.as_object().filter(|_| !js::is_undefined(v)) else {
        return Err(if v.is_null() {
            "TypeError: Cannot read properties of null (reading 'title')".into()
        } else {
            "TypeError: Cannot read properties of undefined (reading 'toLowerCase')".into()
        });
    };
    let title = match o.get("title") {
        Some(Value::String(s)) => s.clone(),
        None | Some(Value::Null) => {
            return Err(
                "TypeError: Cannot read properties of undefined (reading 'toLowerCase')".into(),
            );
        }
        Some(_) => return Err("TypeError: text.toLowerCase is not a function".into()),
    };
    Ok(ExtensionCandidate {
        id: o
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
        title,
        icon: o.get("icon").and_then(Value::as_str).map(Into::into),
        action: truthy(o.get("action")),
        options: truthy(o.get("options")),
    })
}

fn rows_json(rows: &core::ExtensionRows) -> Value {
    let list: Vec<Value> = rows
        .rows
        .iter()
        .map(
            |Suggestion::Extension {
                 id,
                 title,
                 icon,
                 open,
             }| {
                let open = match open {
                    ExtensionOpen::Action => "action",
                    ExtensionOpen::Options => "options",
                };
                json!({ "kind": "extension", "id": id, "title": title, "icon": icon, "open": open })
            },
        )
        .collect();
    json!({ "rows": list, "exact": rows.exact })
}

/// Port of `extensionSuggestions` (`packages/core/src/extensionRows.ts:18`); the JS
/// `slice(0, limit)` of `matchActions` is applied here.
pub fn extension_suggestions(
    query: String,
    extensions: Value,
    limit: Option<f64>,
) -> Result<Value, String> {
    let none = || json!({ "rows": [], "exact": false });
    let items = match &extensions {
        Value::Null => {
            return Err("TypeError: Cannot read properties of null (reading 'length')".into());
        }
        v if js::is_undefined(v) => {
            return Err("TypeError: Cannot read properties of undefined (reading 'length')".into());
        }
        Value::Array(items) if !items.is_empty() => items,
        // A string has a length; `matchActions` then fails on it as a WeakMap key.
        Value::String(s) if !s.is_empty() => {
            return match fuzzy::normalize_query(js_trim(&query)) {
                Some(_) => Err("TypeError: Invalid value used as weak map key".into()),
                None => Ok(none()),
            };
        }
        _ => return Ok(none()),
    };
    if fuzzy::normalize_query(js_trim(&query)).is_none() {
        return Ok(none());
    }
    let candidates = items.iter().map(candidate).collect::<Result<Vec<_>, _>>()?;
    let n = fuzzy::match_actions(js_trim(&query), &candidates, usize::MAX).len() as f64;
    let end = match limit.unwrap_or(core::DEFAULT_LIMIT as f64).trunc() {
        l if l.is_nan() => 0.0,
        l if l < 0.0 => (n + l).max(0.0),
        l => l.min(n),
    } as usize;
    Ok(rows_json(&core::extension_suggestions(
        &query,
        &candidates,
        end,
    )))
}
