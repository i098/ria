// Ported from ardia packages/core/src/suggest.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `omnibox::suggest`, through the recording harness
//! `suggest.harness.ts`: it maps the JSON source and options to typed ones, types the texts in
//! turn on one pool and one narrowing chain, as the harness does on one source, and prints the
//! rows in the field order of the original. The logic is in `ria_core::omnibox::suggest`.

#![forbid(unsafe_code)]

use ria_core::omnibox::candidate_pool::Pool;
use ria_core::omnibox::suggest::{Narrowing, build_suggestions};
use ria_core::omnibox::suggestion::{
    BookmarkRow, CommandAction, HistoryRow, Preference, SourceTab, SuggestEngine, SuggestOptions,
    Suggestion, SuggestionResult, SuggestionSource,
};
use ria_core::search::bangs::{bang_table, find_bang};
use ria_core::search::engines::RemoteSite;
use ria_core::search::extension_rows::{ExtensionCandidate, ExtensionOpen};
use ria_core::search::js_text::js_trim;
use ria_core::search::provider_rows::{JsValue, ProviderSection};
use ria_core::search::site_search::{ScopeKind, SearchScope};
use serde_json::{Map, Value};

use crate::js;

/// A JS string value, lone surrogates replaced.
fn string(v: &Value) -> Option<String> {
    js::string_units(v).map(|u| String::from_utf16_lossy(&u))
}

fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.get(key).filter(|v| !v.is_null() && !js::is_undefined(v))
}

fn text(v: &Value, key: &str) -> String {
    get(v, key).and_then(string).unwrap_or_default()
}

fn opt_text(v: &Value, key: &str) -> Option<String> {
    get(v, key).and_then(string)
}

fn number(v: &Value, key: &str) -> f64 {
    get(v, key).and_then(js::as_number).unwrap_or(f64::NAN)
}

fn flag(v: &Value, key: &str) -> bool {
    get(v, key).and_then(Value::as_bool).unwrap_or(false)
}

fn list<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    get(v, key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn texts(v: &Value, key: &str) -> Vec<String> {
    list(v, key).iter().filter_map(string).collect()
}

fn js_value(v: &Value) -> JsValue {
    if js::is_undefined(v) {
        return JsValue::Undefined;
    }
    if let Some(n) = js::as_number(v) {
        return JsValue::Number(n);
    }
    if let Some(s) = string(v) {
        return JsValue::String(s);
    }
    match v {
        Value::Bool(b) => JsValue::Bool(*b),
        Value::Array(items) => JsValue::Array(items.iter().map(js_value).collect()),
        Value::Object(fields) => JsValue::Object(
            fields
                .iter()
                .map(|(k, v)| (k.clone(), js_value(v)))
                .collect(),
        ),
        _ => JsValue::Null,
    }
}

fn source_of(v: &Value) -> SuggestionSource {
    SuggestionSource {
        tabs: list(v, "tabs")
            .iter()
            .map(|t| SourceTab {
                id: text(t, "id"),
                url: text(t, "url"),
                title: text(t, "title"),
                custom_title: opt_text(t, "customTitle"),
                favicon: opt_text(t, "favicon"),
            })
            .collect(),
        history: list(v, "history")
            .iter()
            .map(|h| HistoryRow {
                url: text(h, "url"),
                title: text(h, "title"),
                favicon: opt_text(h, "favicon"),
                visits: number(h, "visits"),
                last_visit: number(h, "lastVisit"),
            })
            .collect(),
        bookmarks: list(v, "bookmarks")
            .iter()
            .map(|b| BookmarkRow {
                url: text(b, "url"),
                title: text(b, "title"),
                favicon: opt_text(b, "favicon"),
            })
            .collect(),
    }
}

fn scope_of(v: &Value) -> Option<SearchScope> {
    let kind = match get(v, "kind")?.as_str()? {
        "engine" => ScopeKind::Engine,
        "site" => ScopeKind::Site,
        "history" => ScopeKind::History,
        "provider" => ScopeKind::Provider,
        _ => return None,
    };
    Some(SearchScope {
        kind,
        name: text(v, "name"),
        host: text(v, "host"),
        url: text(v, "url"),
        engine_id: opt_text(v, "engineId"),
        provider_id: opt_text(v, "providerId"),
    })
}

fn options_of<'a>(
    v: &Value,
    bangs: Option<&'a ria_core::search::bangs::BangTable>,
) -> SuggestOptions<'a> {
    let defaults = SuggestOptions::default();
    SuggestOptions {
        limit: get(v, "limit")
            .and_then(js::as_number)
            .map_or(defaults.limit, |n| n as usize),
        now: get(v, "now").and_then(js::as_number),
        engine: get(v, "engine").map(|e| SuggestEngine {
            name: text(e, "name"),
            url: text(e, "url"),
        }),
        preference: match get(v, "preference").and_then(Value::as_str) {
            Some("search") => Preference::Search,
            _ => Preference::Website,
        },
        remote: texts(v, "remote"),
        remote_sites: list(v, "remoteSites")
            .iter()
            .map(|s| RemoteSite {
                url: text(s, "url"),
                title: text(s, "title"),
            })
            .collect(),
        actions: list(v, "actions")
            .iter()
            .map(|a| CommandAction {
                id: text(a, "id"),
                title: text(a, "title"),
                keywords: texts(a, "keywords"),
                icon: opt_text(a, "icon"),
                hint: opt_text(a, "hint"),
            })
            .collect(),
        extensions: list(v, "extensions")
            .iter()
            .map(|x| ExtensionCandidate {
                id: text(x, "id"),
                title: text(x, "title"),
                icon: opt_text(x, "icon"),
                action: flag(x, "action"),
                options: flag(x, "options"),
            })
            .collect(),
        current_tab_id: opt_text(v, "currentTabId"),
        current_url: opt_text(v, "currentUrl"),
        scope: get(v, "scope").and_then(scope_of),
        prevent_inline: flag(v, "preventInline"),
        bangs,
        providers: list(v, "providers")
            .iter()
            .map(|p| ProviderSection {
                id: text(p, "id"),
                title: text(p, "title"),
                rows: p.get("rows").map_or(JsValue::Undefined, js_value),
            })
            .collect(),
    }
}

fn opt(o: &mut Map<String, Value>, key: &str, v: &Option<String>) {
    o.insert(key.into(), v.clone().into());
}

/// A row as the original builds it, fields in its order.
fn row_json(s: &Suggestion) -> Value {
    let mut o = Map::new();
    o.insert("kind".into(), s.kind().into());
    match s {
        Suggestion::Page {
            url,
            title,
            favicon,
            tab_id,
            bookmarked,
            visited,
        } => {
            o.insert("url".into(), url.as_str().into());
            o.insert("title".into(), title.as_str().into());
            opt(&mut o, "favicon", favicon);
            if let Some(t) = tab_id {
                o.insert("tabId".into(), t.as_str().into());
            }
            if *bookmarked {
                o.insert("bookmarked".into(), true.into());
            }
            if *visited {
                o.insert("visited".into(), true.into());
            }
        }
        Suggestion::Search {
            query,
            url,
            engine,
            suggested,
            bang,
        } => {
            o.insert("query".into(), query.as_str().into());
            o.insert("url".into(), url.as_str().into());
            o.insert("engine".into(), engine.as_str().into());
            if *suggested {
                o.insert("suggested".into(), true.into());
            }
            if *bang {
                o.insert("bang".into(), true.into());
            }
        }
        Suggestion::Calc {
            expression,
            value,
            url,
        } => {
            o.insert("expression".into(), expression.as_str().into());
            o.insert("value".into(), value.as_str().into());
            o.insert("url".into(), url.as_str().into());
        }
        Suggestion::Action {
            id,
            title,
            icon,
            hint,
        } => {
            o.insert("id".into(), id.as_str().into());
            o.insert("title".into(), title.as_str().into());
            if let Some(i) = icon {
                o.insert("icon".into(), i.as_str().into());
            }
            if let Some(h) = hint {
                o.insert("hint".into(), h.as_str().into());
            }
        }
        Suggestion::Create {
            id,
            title,
            url,
            site,
        } => {
            o.insert("id".into(), id.as_str().into());
            o.insert("title".into(), title.as_str().into());
            o.insert("url".into(), url.as_str().into());
            o.insert("site".into(), site.as_str().into());
        }
        Suggestion::Extension {
            id,
            title,
            icon,
            open,
        } => {
            o.insert("id".into(), id.as_str().into());
            o.insert("title".into(), title.as_str().into());
            opt(&mut o, "icon", icon);
            let open = match open {
                ExtensionOpen::Action => "action",
                ExtensionOpen::Options => "options",
            };
            o.insert("open".into(), open.into());
        }
        Suggestion::Provider(p) => {
            o.insert("providerId".into(), p.provider_id.as_str().into());
            o.insert("provider".into(), p.provider.as_str().into());
            o.insert("title".into(), p.title.as_str().into());
            o.insert("url".into(), p.url.as_str().into());
            opt(&mut o, "icon", &p.icon);
            if let Some(s) = &p.subtitle {
                o.insert("subtitle".into(), s.as_str().into());
            }
        }
    }
    Value::Object(o)
}

fn result_json(r: &SuggestionResult) -> Value {
    let mut o = Map::new();
    o.insert(
        "items".into(),
        Value::Array(r.items.iter().map(row_json).collect()),
    );
    o.insert("completion".into(), r.completion.as_str().into());
    if let Some(via) = &r.via {
        let via = via
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().into()))
            .collect();
        o.insert("via".into(), Value::Object(via));
    }
    Value::Object(o)
}

/// Whether `buildSuggestions(raw)` reads `source.history`: after its early returns for an empty
/// text, a provider scope, a `?` search and a bang with no query.
fn reads_source(raw: &str, options: &SuggestOptions) -> bool {
    let query = js_trim(raw);
    if query.is_empty()
        || options
            .scope
            .as_ref()
            .is_some_and(|s| s.kind == ScopeKind::Provider)
    {
        return false;
    }
    if options.scope.is_some() {
        return true;
    }
    match find_bang(query, options.bangs) {
        Some(bang) => !bang.query.is_empty(),
        None => !query.starts_with('?'),
    }
}

/// The `TypeError` JS throws where `poolFor` reads `source`: a property of `null`, or a
/// primitive as a `WeakMap` key (`undefined` history; bookmarks other than `null`/`undefined`).
fn source_error(source: &Value) -> Option<String> {
    if source.is_null() || js::is_undefined(source) {
        let what = if source.is_null() {
            "null"
        } else {
            "undefined"
        };
        return Some(format!(
            "TypeError: Cannot read properties of {what} (reading 'history')"
        ));
    }
    let object = |v: Option<&Value>| {
        v.is_some_and(|v| {
            matches!(v, Value::Array(_) | Value::Object(_))
                && !js::is_undefined(v)
                && js::as_number(v).is_none()
                && js::string_units(v).is_none()
        })
    };
    let field = |key: &str| source.as_object().and_then(|o| o.get(key));
    let bookmarks = field("bookmarks");
    let bookmarks_ok =
        bookmarks.is_none_or(|b| b.is_null() || js::is_undefined(b)) || object(bookmarks);
    (!object(field("history")) || !bookmarks_ok)
        .then(|| "TypeError: Invalid value used as weak map key".to_string())
}

/// Port of `suggestTyped` (`packages/core/src/suggest.harness.ts:10`).
pub fn suggest_typed(
    raws: Vec<String>,
    source: serde_json::Value,
    options: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    if options.is_null() || js::is_undefined(&options) {
        let what = if options.is_null() {
            "null"
        } else {
            "undefined"
        };
        return Err(format!(
            "TypeError: Cannot read properties of {what} (reading 'bangs')"
        ));
    }
    let table = get(&options, "bangs").and_then(string).map(bang_table);
    let options = options_of(&options, table.as_ref());
    if raws.iter().any(|raw| reads_source(raw, &options))
        && let Some(e) = source_error(&source)
    {
        return Err(e);
    }
    let source = source_of(&source);
    let mut pool =
        Pool::new(&source.history, &source.bookmarks).map_err(|e| format!("TypeError: {e}"))?;
    let mut narrowing = Narrowing::default();
    raws.iter()
        .map(|raw| {
            build_suggestions(raw, &source, &options, &mut pool, &mut narrowing)
                .map(|r| result_json(&r))
                .map_err(|e| format!("TypeError: {e}"))
        })
        .collect()
}
