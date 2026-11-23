// Ported from ardia packages/core/src/suggestionRows.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `omnibox::suggestion_rows`, through the recording harness
//! `suggestionRows.harness.ts`. The logic is in `ria_core::omnibox::suggestion_rows`; this
//! binding maps JSON values to typed ones and back. The original copies the fields of a row
//! whatever their type, so the rows it prints carry the input values themselves, in the places
//! the Rust row has them.

#![forbid(unsafe_code)]

use ria_core::omnibox::suggestion::{CommandAction, Suggestion};
use ria_core::omnibox::suggestion_rows::{self as rows, Output, PageRow};
use ria_core::search::extension_rows::ExtensionOpen;
use ria_core::search::provider_rows::ProviderSuggestion;
use serde_json::{Map, Value};

use crate::js;

/// A truthy value that is not a string, as a string the core keeps: the binding puts the
/// input value back in its place.
const STAND_IN: &str = "\u{1}";

/// `v[key]`, `undefined` when absent; an error where JS fails to read a property.
fn get(v: &Value, key: &str) -> Result<Value, String> {
    if v.is_null() || js::is_undefined(v) {
        let what = if v.is_null() { "null" } else { "undefined" };
        return Err(format!(
            "TypeError: Cannot read properties of {what} (reading '{key}')"
        ));
    }
    let field = v.as_object().filter(|_| js::string_units(v).is_none());
    Ok(field
        .and_then(|o| o.get(key))
        .cloned()
        .unwrap_or_else(js::undefined))
}

/// A JS string value, lone surrogates replaced.
fn string(v: &Value) -> Option<String> {
    js::string_units(v).map(|u| String::from_utf16_lossy(&u))
}

/// `!!v`.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        Value::Number(n) => n.as_f64().is_some_and(|n| n != 0.0),
        x => match js::as_number(x) {
            Some(n) => n != 0.0 && !n.is_nan(),
            None => !js::is_undefined(x),
        },
    }
}

/// `String(v)`, as a template literal prints it.
fn js_string(v: &Value) -> String {
    if let Some(s) = string(v) {
        return s;
    }
    if js::is_undefined(v) {
        return "undefined".into();
    }
    if let Some(n) = js::as_number(v) {
        return js::number_to_string(n);
    }
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Array(items) => items
            .iter()
            .map(|x| {
                if x.is_null() || js::is_undefined(x) {
                    String::new()
                } else {
                    js_string(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        _ => "[object Object]".into(),
    }
}

/// `Number(v)`, as far as `visits > 0` needs.
fn to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::String(s) => {
            let s = s.trim();
            if s.is_empty() {
                0.0
            } else {
                s.parse().unwrap_or(f64::NAN)
            }
        }
        Value::Array(items) if items.is_empty() => 0.0,
        Value::Array(items) if items.len() == 1 => to_number(&Value::String(js_string(&items[0]))),
        x => js::as_number(x).unwrap_or(f64::NAN),
    }
}

/// A string the core keeps for a value of which only its truthiness counts.
fn truthy_text(v: &Value) -> Option<String> {
    truthy(v).then(|| string(v).unwrap_or_else(|| STAND_IN.into()))
}

/// A URL `pageKey` or `withRootSlash` reads: it calls `url.replace`.
fn url_text(v: &Value) -> Result<String, String> {
    string(v).ok_or_else(|| "TypeError: url.replace is not a function".to_string())
}

/// The fields of a row that `suggestionKey` reads, as a `Suggestion`; `None` for a value whose
/// `kind` is none of the kinds, whose key is `undefined`.
fn suggestion(v: &Value) -> Result<Option<Suggestion>, String> {
    let field = |key| get(v, key);
    let text = |key| field(key).map(|x| js_string(&x));
    Ok(Some(match string(&field("kind")?).as_deref() {
        Some("page") => {
            let tab_id = truthy_text(&field("tabId")?);
            Suggestion::Page {
                url: url_text(&field("url")?)?,
                title: String::new(),
                favicon: None,
                tab_id,
                bookmarked: false,
                visited: false,
            }
        }
        Some("search") => Suggestion::Search {
            query: String::new(),
            url: text("url")?,
            engine: String::new(),
            suggested: false,
            bang: false,
        },
        Some("calc") => Suggestion::Calc {
            expression: text("expression")?,
            value: String::new(),
            url: String::new(),
        },
        Some("action") => Suggestion::Action {
            id: text("id")?,
            title: String::new(),
            icon: None,
            hint: None,
        },
        Some("create") => rows::create_row(&text("id")?, "", "", ""),
        Some("extension") => {
            let open = match text("open")?.as_str() {
                "action" => ExtensionOpen::Action,
                "options" => ExtensionOpen::Options,
                other => {
                    return Err(format!(
                        "shortcut: extension rows open only an action or options, not {other:?}"
                    ));
                }
            };
            Suggestion::Extension {
                id: text("id")?,
                title: String::new(),
                icon: None,
                open,
            }
        }
        Some("provider") => Suggestion::Provider(ProviderSuggestion {
            provider_id: text("providerId")?,
            provider: String::new(),
            title: String::new(),
            url: url_text(&field("url")?)?,
            icon: None,
            subtitle: None,
        }),
        _ => return Ok(None),
    }))
}

/// The JSON of a row the core built, with the input value of each field it has.
fn row_json(s: &Suggestion, raw: &[(&str, &Value)]) -> Value {
    let mut o = Map::new();
    o.insert("kind".into(), s.kind().into());
    let mut put = |key: &str, present: bool| {
        if present && let Some((_, v)) = raw.iter().find(|(k, _)| *k == key) {
            o.insert(key.into(), (*v).clone());
        }
    };
    match s {
        Suggestion::Page {
            tab_id,
            bookmarked,
            visited,
            ..
        } => {
            put("url", true);
            put("title", true);
            put("favicon", true);
            put("tabId", tab_id.is_some());
            if *bookmarked {
                o.insert("bookmarked".into(), true.into());
            }
            if *visited {
                o.insert("visited".into(), true.into());
            }
        }
        Suggestion::Action { icon, hint, .. } => {
            put("id", true);
            put("title", true);
            put("icon", icon.is_some());
            put("hint", hint.is_some());
        }
        _ => {
            for key in ["id", "title", "url", "site"] {
                put(key, true);
            }
        }
    }
    Value::Object(o)
}

/// Port of `pageSuggestionOf` (`packages/core/src/suggestionRows.harness.ts:7`): an undefined
/// `tabId` takes the row's own.
pub fn page_suggestion_of(c: Value, tab_id: Value) -> Result<Value, String> {
    let tab_id = if js::is_undefined(&tab_id) {
        get(&c, "tabId")?
    } else {
        tab_id
    };
    let url = get(&c, "url")?;
    let title = get(&c, "title")?;
    let favicon = get(&c, "favicon")?;
    let (url_s, title_s, favicon_s) = (js_string(&url), js_string(&title), string(&favicon));
    let row = PageRow {
        url: &url_s,
        title: &title_s,
        favicon: favicon_s.as_deref(),
        visits: to_number(&get(&c, "visits")?),
        bookmarked: truthy(&get(&c, "bookmarked")?),
    };
    let s = rows::page_suggestion(row, truthy_text(&tab_id).as_deref());
    Ok(row_json(
        &s,
        &[
            ("url", &url),
            ("title", &title),
            ("favicon", &favicon),
            ("tabId", &tab_id),
        ],
    ))
}

/// Port of `suggestionKeyOf` (`packages/core/src/suggestionRows.harness.ts:11`).
pub fn suggestion_key_of(s: Value) -> Result<Value, String> {
    Ok(suggestion(&s)?.map_or_else(js::undefined, |s| rows::suggestion_key(&s).into()))
}

/// The rows of a spread or a `for...of` over `v`: an array's items, a string's code points.
fn iterate(v: &Value) -> Result<Vec<Value>, String> {
    if let Some(units) = js::string_units(v) {
        return Ok(char::decode_utf16(units)
            .map(|c| match c {
                Ok(c) => Value::String(c.into()),
                Err(e) => js::utf16_string(&[e.unpaired_surrogate()]),
            })
            .collect());
    }
    match v {
        Value::Array(items) => Ok(items.clone()),
        _ => Err("TypeError: Spread syntax requires an iterable".into()),
    }
}

/// Port of `outputItems` (`packages/core/src/suggestionRows.harness.ts:15`). The original stops
/// at `items.length >= limit`: a fractional limit rounds up, and NaN never stops. The rows kept
/// are the input rows themselves, as in JS.
pub fn output_items(limit: f64, batches: Value) -> Result<Vec<Value>, String> {
    let limit = if limit.is_nan() {
        usize::MAX
    } else {
        limit.max(0.0).ceil() as usize
    };
    let mut out = Output::new(limit);
    let mut kept = Vec::new();
    for batch in iterate(&batches)? {
        for row in iterate(&batch)? {
            // The key of a row of no kind is `undefined`: one such row is kept, as one row of a
            // calc expression no input has.
            let s = suggestion(&row)?.unwrap_or_else(|| Suggestion::Calc {
                expression: "\u{0}undefined".into(),
                value: String::new(),
                url: String::new(),
            });
            if out.push(s) {
                kept.push(row);
            }
        }
    }
    Ok(kept)
}

/// Port of `createRowOf` (`packages/core/src/suggestionRows.harness.ts:21`).
pub fn create_row_of(c: Value) -> Result<Value, String> {
    let raw = [
        ("id", get(&c, "id")?),
        ("title", get(&c, "title")?),
        ("url", get(&c, "url")?),
        ("site", get(&c, "site")?),
    ];
    let [id, title, url, site] = raw.each_ref().map(|(_, v)| js_string(v));
    let s = rows::create_row(&id, &title, &url, &site);
    let raw: Vec<(&str, &Value)> = raw.iter().map(|(k, v)| (*k, v)).collect();
    Ok(row_json(&s, &raw))
}

/// Port of `actionRowOf` (`packages/core/src/suggestionRows.harness.ts:25`).
pub fn action_row_of(a: Value) -> Result<Value, String> {
    let id = get(&a, "id")?;
    let title = get(&a, "title")?;
    let icon = get(&a, "icon")?;
    let hint = get(&a, "hint")?;
    let action = CommandAction {
        id: js_string(&id),
        title: js_string(&title),
        keywords: Vec::new(),
        icon: truthy_text(&icon),
        hint: truthy_text(&hint),
    };
    Ok(row_json(
        &rows::action_row(&action),
        &[
            ("id", &id),
            ("title", &title),
            ("icon", &icon),
            ("hint", &hint),
        ],
    ))
}

/// Port of `dedupeSuggestionsOf` (`packages/core/src/suggestionRows.harness.ts:29`).
pub fn dedupe_suggestions_of(remote: Vec<String>, query: String) -> Result<Vec<String>, String> {
    Ok(rows::dedupe_suggestions(&remote, &query)
        .into_iter()
        .map(str::to_string)
        .collect())
}
