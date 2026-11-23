// Ported from ardia packages/core/src/siteSearch.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `search::site_search`: it reads the JSON arguments of the
//! recording as JS would (property reads, defaults, the JS errors of null and missing values)
//! into typed values, and writes the results in the shape the original returns. The logic is in
//! `ria_core::search::site_search`.
//!
//! shortcut: fields of engines, sites, providers and key states are read leniently: a missing or
//! non-string field reads as "" (or as absent for optional ones) where the JS would see
//! `undefined`, and sites and engines that are not objects are skipped. Upgrade when a caller
//! passes such values.

#![forbid(unsafe_code)]

use ria_core::search::engines::{EngineExtension, SearchEngine};
use ria_core::search::host::host_of;
use ria_core::search::site_search::{
    self as core, KeywordProvider, ScopeKind, ScopeSources, SearchScope, SiteSearch, SiteSearchKey,
    SiteSearchKeyModifiers, SiteSearchKeyState,
};
use serde_json::{Map, Value, json};

use crate::js;

/// "null" or "undefined" for those values: reading a property of them throws.
fn nullish(v: &Value) -> Option<&'static str> {
    match v {
        Value::Null => Some("null"),
        _ if js::is_undefined(v) => Some("undefined"),
        _ => None,
    }
}

fn read_error(of: &str, key: &str) -> String {
    format!("TypeError: Cannot read properties of {of} (reading '{key}')")
}

/// A plain object (not an array, nor a marker of `undefined` and other JS values).
fn object(v: &Value) -> Option<&Map<String, Value>> {
    v.as_object().filter(|o| {
        !o.keys()
            .next()
            .is_some_and(|k| o.len() == 1 && k.starts_with('$'))
    })
}

/// `v[key]`, `None` for `undefined`, with the JS error when `v` is null or undefined.
fn get<'a>(v: &'a Value, key: &str) -> Result<Option<&'a Value>, String> {
    if let Some(n) = nullish(v) {
        return Err(read_error(n, key));
    }
    Ok(object(v)
        .and_then(|o| o.get(key))
        .filter(|x| !js::is_undefined(x)))
}

/// A named property of an object, read leniently (see the module shortcut).
fn field<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    object(v).and_then(|o| o.get(key)).and_then(Value::as_str)
}

fn text(v: &Value, key: &str) -> String {
    field(v, key).unwrap_or_default().into()
}

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

/// `String(v)` for the values a scope host can hold.
fn js_string(v: Option<&Value>) -> String {
    match v {
        None => "undefined".into(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) => "null".into(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Some(x) => js::as_number(x).map_or_else(|| "[object Object]".into(), js::number_to_string),
    }
}

fn scope_json(s: &SearchScope) -> Value {
    let kind = match s.kind {
        ScopeKind::Engine => "engine",
        ScopeKind::Site => "site",
        ScopeKind::History => "history",
        ScopeKind::Provider => "provider",
    };
    let mut o = json!({ "kind": kind, "name": s.name, "host": s.host, "url": s.url });
    if let Some(id) = &s.engine_id {
        o["engineId"] = id.clone().into();
    }
    if let Some(id) = &s.provider_id {
        o["providerId"] = id.clone().into();
    }
    o
}

fn search_engine(v: &Value) -> SearchEngine {
    let extension = object(v)
        .and_then(|o| o.get("extension"))
        .filter(|x| object(x).is_some());
    SearchEngine {
        id: text(v, "id"),
        name: text(v, "name"),
        keyword: text(v, "keyword"),
        url: text(v, "url"),
        suggest_url: field(v, "suggestUrl").map(Into::into),
        custom: truthy(object(v).and_then(|o| o.get("custom"))),
        extension: extension.map(|x| EngineExtension {
            id: text(x, "id"),
            name: text(x, "name"),
        }),
    }
}

fn site(v: &Value) -> SiteSearch {
    SiteSearch {
        host: text(v, "host"),
        name: text(v, "name"),
        url: text(v, "url"),
        keyword: field(v, "keyword").map(Into::into),
    }
}

/// The providers `providerScope` reads before its `find` throws, and that error.
fn providers_of(
    query: &str,
    providers: Option<&Value>,
) -> Result<(Vec<KeywordProvider>, Option<String>), String> {
    let items = match providers {
        None => return Ok((vec![], None)),
        Some(Value::Null) => return Err(read_error("null", "length")),
        Some(Value::Array(items)) => items,
        // A string has a length but no `find`.
        Some(Value::String(s)) if !s.is_empty() && core::is_scope_word(query) => {
            return Err("TypeError: providers.find is not a function".into());
        }
        Some(_) => return Ok((vec![], None)),
    };
    if !core::is_scope_word(query) {
        return Ok((vec![], None));
    }
    let mut typed = Vec::new();
    for x in items {
        let keyword = match get(x, "keyword") {
            Err(e) => return Ok((typed, Some(e))),
            Ok(Some(Value::String(k))) => k.clone(),
            Ok(None) => return Ok((typed, Some(read_error("undefined", "toLowerCase")))),
            Ok(Some(Value::Null)) => return Ok((typed, Some(read_error("null", "toLowerCase")))),
            Ok(Some(_)) => {
                return Ok((
                    typed,
                    Some("TypeError: x.keyword.toLowerCase is not a function".into()),
                ));
            }
        };
        typed.push(KeywordProvider {
            id: text(x, "id"),
            title: text(x, "title"),
            keyword,
            host: field(x, "host").map(Into::into),
            url: field(x, "url").map(Into::into),
        });
    }
    Ok((typed, None))
}

fn provider_scope_of(text: &str, providers: Option<&Value>) -> Result<Option<SearchScope>, String> {
    let (typed, fail) = providers_of(text, providers)?;
    match (core::provider_scope(text, &typed), fail) {
        (None, Some(e)) => Err(e),
        (scope, _) => Ok(scope),
    }
}

/// `findScope(text, sources)` with the defaults and errors of its destructuring.
fn find_scope_of(text: &str, sources: &Value) -> Result<Option<SearchScope>, String> {
    if let Some(n) = nullish(sources) {
        return Err(format!(
            "TypeError: Cannot destructure property 'engines' of '{}' as it is {n}.",
            if n == "null" { "object null" } else { n }
        ));
    }
    let prop = |key| {
        object(sources)
            .and_then(|o| o.get(key))
            .filter(|x| !js::is_undefined(x))
    };
    if let Some(scope) = provider_scope_of(text, prop("providers"))? {
        return Ok(Some(scope));
    }
    let Some(t) = core::find_scope_text(text) else {
        return Ok(None);
    };
    let Some(Value::Array(engines)) = prop("engines") else {
        return Err("TypeError: engines is not iterable".into());
    };
    let engines: Vec<SearchEngine> = engines
        .iter()
        .filter(|e| object(e).is_some())
        .map(search_engine)
        .collect();
    let host = Some(host_of(&format!("https://{t}")))
        .filter(|h| !h.is_empty())
        .unwrap_or(t);
    // The engines decide alone first: the sites and hosts are read only when no engine matches.
    let none: [SiteSearch; 0] = [];
    let engine_only = ScopeSources {
        engines: &engines,
        sites: Some(&none),
        hosts: &[],
        providers: &[],
    };
    if let Some(scope) = core::find_scope(text, &engine_only) {
        return Ok(Some(scope));
    }
    let sites = match prop("sites") {
        None => None,
        Some(Value::Array(items)) => Some(
            items
                .iter()
                .filter(|s| object(s).is_some())
                .map(site)
                .collect::<Vec<_>>(),
        ),
        Some(_) => return Err("TypeError: sites is not iterable".into()),
    };
    let sites_only = ScopeSources {
        sites: sites.as_deref(),
        ..engine_only
    };
    if let Some(scope) = core::find_scope(text, &sites_only) {
        return Ok(Some(scope));
    }
    let hosts: Vec<String> = match prop("hosts") {
        None => vec![],
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(Into::into)
            .collect(),
        // `String.prototype.includes` finds a part of the string.
        Some(Value::String(s)) => s
            .contains(&host)
            .then(|| host.clone())
            .into_iter()
            .collect(),
        Some(v) if host.contains('.') => {
            return Err(match v {
                Value::Null => read_error("null", "includes"),
                _ => "TypeError: hosts.includes is not a function".into(),
            });
        }
        Some(_) => vec![],
    };
    Ok(core::find_scope(
        text,
        &ScopeSources {
            hosts: &hosts,
            ..sites_only
        },
    ))
}

fn out(scope: Option<SearchScope>) -> Option<Value> {
    scope.as_ref().map(scope_json)
}

/// Port of `providerScope` (`packages/core/src/siteSearch.ts:50`).
pub fn provider_scope(text: String, providers: Value) -> Result<Option<Value>, String> {
    let providers = Some(&providers).filter(|p| !js::is_undefined(p));
    Ok(out(provider_scope_of(&text, providers)?))
}

/// Port of `findScope` (`packages/core/src/siteSearch.ts:67`).
pub fn find_scope(text: String, arg1: Value) -> Result<Option<Value>, String> {
    Ok(out(find_scope_of(&text, &arg1)?))
}

/// `template` of `searchUrlFor`, with the JS errors of `template.includes`.
fn template(v: Option<&Value>) -> Result<&str, String> {
    match v {
        None => Err(read_error("undefined", "includes")),
        Some(Value::Null) => Err(read_error("null", "includes")),
        Some(Value::String(s)) => Ok(s),
        Some(_) => Err("TypeError: template.includes is not a function".into()),
    }
}

/// Port of `scopedSearchUrl` (`packages/core/src/siteSearch.ts:95`).
pub fn scoped_search_url(
    scope: Value,
    query: String,
    default_engine: Value,
) -> Result<String, String> {
    let kind = match get(&scope, "kind")?.and_then(Value::as_str) {
        Some("history") => ScopeKind::History,
        Some("provider") => ScopeKind::Provider,
        _ => ScopeKind::Site,
    };
    let url = get(&scope, "url")?;
    let by_default = kind == ScopeKind::History || (kind == ScopeKind::Provider && !truthy(url));
    let typed = SearchScope {
        kind,
        name: String::new(),
        host: if kind == ScopeKind::History {
            js_string(get(&scope, "host")?)
        } else {
            String::new()
        },
        url: if by_default {
            String::new()
        } else {
            template(url)?.into()
        },
        engine_id: None,
        provider_id: None,
    };
    let default_url = if by_default {
        template(get(&default_engine, "url")?)?
    } else {
        ""
    };
    Ok(core::scoped_search_url(&typed, &query, default_url))
}

/// Port of `offeredSiteSearch` (`packages/core/src/siteSearch.ts:101`).
pub fn offered_site_search(
    typed: String,
    completion: String,
    sources: Value,
) -> Result<Option<Value>, String> {
    if !core::is_scope_word(&typed) {
        return Ok(None);
    }
    if let Some(scope) = provider_scope_of(&typed, get(&sources, "providers")?)? {
        return Ok(out(Some(scope)));
    }
    Ok(out(find_scope_of(
        &format!("{typed}{completion}"),
        &sources,
    )?))
}

/// Port of `keywordBySpace` (`packages/core/src/siteSearch.ts:124`).
pub fn keyword_by_space(typed: String, next: String) -> Result<Option<Value>, String> {
    Ok(core::keyword_by_space(&typed, &next)
        .map(|k| json!({ "keyword": k.keyword, "from": k.from })))
}

/// Port of `asksForBackspace` (`packages/core/src/siteSearch.ts:129`).
pub fn asks_for_backspace(scope: Option<Value>) -> Result<bool, String> {
    let present = SearchScope {
        kind: ScopeKind::Site,
        name: String::new(),
        host: String::new(),
        url: String::new(),
        engine_id: None,
        provider_id: None,
    };
    Ok(core::asks_for_backspace(
        truthy(scope.as_ref()).then_some(&present),
    ))
}

/// Port of `siteSearchKey` (`packages/core/src/siteSearch.ts:133`).
pub fn site_search_key(
    key: String,
    modifiers: Value,
    state: Value,
) -> Result<Option<Value>, String> {
    if let Some(n) = nullish(&modifiers) {
        let of = if n == "null" { "null" } else { "undefined" };
        return Err(format!(
            "TypeError: Cannot destructure property 'shiftKey' of 'modifiers' as it is {of}."
        ));
    }
    let flag = |k| truthy(object(&modifiers).and_then(|o| o.get(k)));
    let m = SiteSearchKeyModifiers {
        shift: flag("shiftKey"),
        alt: flag("altKey"),
        meta: flag("metaKey"),
        ctrl: flag("ctrlKey"),
    };
    if core::scope_key(&key, &m).is_none() {
        return Ok(None);
    }
    // A scope stands in for each truthy one of the state: the original's object is returned.
    let present = SearchScope {
        kind: ScopeKind::Site,
        name: String::new(),
        host: String::new(),
        url: String::new(),
        engine_id: None,
        provider_id: None,
    };
    let scope = get(&state, "scope")?;
    let offered = get(&state, "offered")?;
    let scope_from = get(&state, "scopeFrom")?;
    let (typed, completion) = (text(&state, "typed"), text(&state, "completion"));
    let s = SiteSearchKeyState {
        typed: &typed,
        completion: &completion,
        scope: truthy(scope).then_some(&present),
        offered: truthy(offered).then_some(&present),
        scope_from: "",
    };
    Ok(core::site_search_key(&key, &m, &s).map(|k| match k {
        SiteSearchKey::Enter { from, .. } => {
            json!({ "kind": "enter", "scope": offered.cloned().unwrap_or_else(js::undefined), "from": from })
        }
        SiteSearchKey::Leave { .. } => {
            json!({ "kind": "leave", "text": scope_from.cloned().unwrap_or_else(js::undefined) })
        }
        SiteSearchKey::Accept { text } => json!({ "kind": "accept", "text": text }),
    }))
}
