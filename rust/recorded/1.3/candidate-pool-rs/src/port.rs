// Ported from ardia packages/core/src/candidatePool.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `omnibox::candidate_pool`, through the recording harness
//! `candidatePool.harness.ts`: it maps the JSON rows to typed rows, gives an error where the
//! original throws, and prints pools as the harness does. The logic is in
//! `ria_core::omnibox::candidate_pool`.

#![forbid(unsafe_code)]

use ria_core::omnibox::candidate_pool::{self as core, Candidate, Pool};
use ria_core::omnibox::suggestion::{BookmarkRow, HistoryRow, SourceTab};
use ria_core::url::page_text::PageText;
use serde_json::{Map, Value, json};

use crate::js;

const NOT_STRING: &str = "TypeError: a page row field is not a string";

/// A JS string value, lone surrogates replaced.
fn string(v: &Value) -> Option<String> {
    js::string_units(v).map(|u| String::from_utf16_lossy(&u))
}

/// `row[key]`: an error where JS fails to read a property of null or undefined.
fn field<'a>(row: &'a Value, key: &str) -> Result<Option<&'a Value>, String> {
    if row.is_null() || js::is_undefined(row) {
        return Err(format!(
            "TypeError: Cannot read properties of {row} (reading '{key}')"
        ));
    }
    Ok(row.as_object().and_then(|o| o.get(key)))
}

/// A field `pageText` reads as a string.
fn text_field(row: &Value, key: &str) -> Result<String, String> {
    field(row, key)?
        .and_then(string)
        .ok_or_else(|| NOT_STRING.to_string())
}

fn favicon(row: &Value) -> Result<Option<String>, String> {
    Ok(field(row, "favicon")?.and_then(string))
}

fn number(row: &Value, key: &str) -> Result<f64, String> {
    Ok(field(row, key)?.and_then(js::as_number).unwrap_or(f64::NAN))
}

/// The rows of a list. A `for...of` needs an array; the index loops of `poolFor` and
/// `tabsOver` (`lenient`) read an object without `length` as no rows. Other values fail, as
/// they do as `WeakMap` keys.
fn rows(v: &Value, lenient: bool) -> Result<&[Value], String> {
    let marker = js::is_undefined(v) || js::as_number(v).is_some() || js::string_units(v).is_some();
    match v {
        Value::Array(items) => Ok(items),
        Value::Object(_) if lenient && !marker => Ok(&[]),
        _ => Err("TypeError: rows is not iterable".into()),
    }
}

fn history(v: &Value, lenient: bool) -> Result<Vec<HistoryRow>, String> {
    rows(v, lenient)?
        .iter()
        .map(|r| {
            Ok(HistoryRow {
                url: text_field(r, "url")?,
                title: text_field(r, "title")?,
                favicon: favicon(r)?,
                visits: number(r, "visits")?,
                last_visit: number(r, "lastVisit")?,
            })
        })
        .collect()
}

fn bookmarks(v: &Value, lenient: bool) -> Result<Vec<BookmarkRow>, String> {
    rows(v, lenient)?
        .iter()
        .map(|r| {
            Ok(BookmarkRow {
                url: text_field(r, "url")?,
                title: text_field(r, "title")?,
                favicon: favicon(r)?,
            })
        })
        .collect()
}

/// `source.bookmarks ?? NO_BOOKMARKS`.
fn source_bookmarks(source: &Value, lenient: bool) -> Result<Vec<BookmarkRow>, String> {
    match field(source, "bookmarks")? {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(v) if js::is_undefined(v) => Ok(Vec::new()),
        Some(v) => bookmarks(v, lenient),
    }
}

fn source_history(source: &Value, lenient: bool) -> Result<Vec<HistoryRow>, String> {
    history(
        field(source, "history")?.unwrap_or(&js::undefined()),
        lenient,
    )
}

fn pool(history: &[HistoryRow], bookmarks: &[BookmarkRow]) -> Result<Pool, String> {
    Pool::new(history, bookmarks).map_err(|e| format!("TypeError: {e}"))
}

fn text_json(t: &PageText) -> Value {
    let mut o = Map::new();
    o.insert("url".into(), t.url.as_str().into());
    o.insert("key".into(), t.key.as_str().into());
    o.insert("shown".into(), t.shown.as_str().into());
    o.insert("shownLower".into(), t.shown_lower.as_str().into());
    o.insert("titleLower".into(), t.title_lower.as_str().into());
    o.insert("titleWords".into(), t.title_words.as_str().into());
    Value::Object(o)
}

fn candidate_json(c: &Candidate) -> Value {
    let mut o = Map::new();
    o.insert("url".into(), c.url.as_str().into());
    o.insert("title".into(), c.title.as_str().into());
    if let Some(p) = &c.page_title {
        o.insert("pageTitle".into(), p.as_str().into());
    }
    if c.renamed {
        o.insert("renamed".into(), true.into());
    }
    o.insert("favicon".into(), c.favicon.clone().into());
    o.insert("visits".into(), js::number(c.visits));
    o.insert("lastVisit".into(), js::number(c.last_visit));
    o.insert("logVisits".into(), js::number(c.log_visits));
    if c.fresh {
        o.insert("fresh".into(), true.into());
    }
    o.insert("bookmarked".into(), c.bookmarked.into());
    if let Some(t) = &c.tab_id {
        o.insert("tabId".into(), t.as_str().into());
    }
    o.insert("text".into(), text_json(&c.text));
    Value::Object(o)
}

/// The harness's `view`: the list, and `byKey.get` / `byKey.has` of each key.
fn view(pool: &Pool, keys: &[String]) -> Value {
    let list: Vec<Value> = pool.list.iter().map(candidate_json).collect();
    let by_key: Vec<Value> = keys
        .iter()
        .map(|k| pool.by_key(k).map_or(Value::Null, candidate_json))
        .collect();
    let has: Vec<bool> = keys.iter().map(|k| pool.has_key(k)).collect();
    json!({ "list": list, "byKey": by_key, "has": has })
}

/// Port of `poolGrid` (`packages/core/src/candidatePool.harness.ts:14`). A pool reads its
/// lists only when it is built, so no list is read without a pair.
pub fn pool_grid(
    histories: Vec<Value>,
    bookmark_lists: Value,
    keys: Vec<String>,
) -> Result<Value, String> {
    if histories.is_empty() {
        return Ok(Value::Array(Vec::new()));
    }
    let Value::Array(bookmark_lists) = bookmark_lists else {
        return Err("TypeError: bookmarkLists.map is not a function".into());
    };
    let mut out = Vec::new();
    for h in &histories {
        for b in &bookmark_lists {
            let b = if b.is_null() || js::is_undefined(b) {
                Vec::new()
            } else {
                bookmarks(b, true)?
            };
            out.push(view(&pool(&history(h, true)?, &b)?, &keys));
        }
    }
    Ok(Value::Array(out))
}

/// Port of `preparedPool` (`packages/core/src/candidatePool.harness.ts:19`): `Pool::new` is
/// `prepareSuggestions` to the end, whatever the deadline.
pub fn prepared_pool(source: Value, _deadline: f64, keys: Vec<String>) -> Result<Value, String> {
    let b = source_bookmarks(&source, false)?;
    let h = source_history(&source, false)?;
    Ok(view(&pool(&h, &b)?, &keys))
}

/// A tab row; `None` for one `tabsOver` skips (no URL, or the current tab).
fn tab(t: &Value, current_tab_id: Option<&str>) -> Result<Option<SourceTab>, String> {
    let url = field(t, "url")?;
    let id = field(t, "id")?.and_then(string);
    let skipped = match url {
        None | Some(Value::Null | Value::Bool(false)) => true,
        Some(Value::String(s)) => s.is_empty(),
        Some(v) => js::is_undefined(v) || js::as_number(v).is_some_and(|n| n == 0.0 || n.is_nan()),
    };
    if skipped || id.as_deref() == current_tab_id {
        return Ok(None);
    }
    let custom_title = match field(t, "customTitle")? {
        None | Some(Value::Null) => None,
        Some(v) => Some(string(v).ok_or_else(|| NOT_STRING.to_string())?),
    };
    Ok(Some(SourceTab {
        id: id.unwrap_or_default(),
        url: text_field(t, "url")?,
        title: text_field(t, "title")?,
        custom_title,
        favicon: favicon(t)?,
    }))
}

/// UTF-16 order, as JS compares strings.
fn by_text(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// Port of `tabsOverView` (`packages/core/src/candidatePool.harness.ts:26`).
pub fn tabs_over_view(source: Value, current_tab_id: Option<String>) -> Result<Value, String> {
    let b = source_bookmarks(&source, true)?;
    let h = source_history(&source, true)?;
    let pool = pool(&h, &b)?;
    let tabs = rows(field(&source, "tabs")?.unwrap_or(&js::undefined()), true)?
        .iter()
        .filter_map(|t| tab(t, current_tab_id.as_deref()).transpose())
        .collect::<Result<Vec<_>, _>>()?;
    let over = core::tabs_over(&pool, &tabs, current_tab_id.as_deref())
        .map_err(|e| format!("TypeError: {e}"))?;
    let mut tab_ids: Vec<(&String, &String)> = over.tab_ids.iter().collect();
    tab_ids.sort_by(|a, b| by_text(a.0, b.0));
    let mut renamed: Vec<&String> = over.renamed.iter().collect();
    renamed.sort_by(|a, b| by_text(a, b));
    let extra: Vec<Value> = over.extra.iter().map(candidate_json).collect();
    Ok(json!({ "tabIds": tab_ids, "extra": extra, "renamed": renamed }))
}

/// The rows of a list, each with its URL; `exactRows` reads each row's `url`.
fn url_rows(v: &Value) -> Result<(&[Value], Vec<HistoryRow>), String> {
    let list = rows(v, false)?;
    let typed = list
        .iter()
        .map(|r| {
            let url = field(r, "url")?
                .and_then(string)
                .ok_or_else(|| NOT_STRING.to_string())?;
            Ok(HistoryRow {
                url,
                title: String::new(),
                favicon: None,
                visits: 0.0,
                last_visit: 0.0,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok((list, typed))
}

/// The input row `found` stands for, or null.
fn row_of(list: &[Value], typed: &[HistoryRow], found: Option<&HistoryRow>) -> Value {
    found
        .and_then(|f| typed.iter().position(|r| std::ptr::eq(r, f)))
        .map_or(Value::Null, |i| list[i].clone())
}

/// Port of `savedVisits` (`packages/core/src/candidatePool.harness.ts:33`).
pub fn saved_visits(history: Value, urls: Vec<String>) -> Result<Value, String> {
    if urls.is_empty() {
        return Ok(Value::Array(Vec::new()));
    }
    let (list, typed) = url_rows(&history)?;
    Ok(urls
        .iter()
        .map(|u| row_of(list, &typed, core::saved_visit(&typed, u)))
        .collect())
}

/// Port of `savedBookmarks` (`packages/core/src/candidatePool.harness.ts:37`).
pub fn saved_bookmarks(bookmarks: Value, urls: Vec<String>) -> Result<Value, String> {
    if urls.is_empty() {
        return Ok(Value::Array(Vec::new()));
    }
    let (list, typed) = url_rows(&bookmarks)?;
    let marks: Vec<BookmarkRow> = typed
        .iter()
        .map(|r| BookmarkRow {
            url: r.url.clone(),
            title: String::new(),
            favicon: None,
        })
        .collect();
    Ok(urls
        .iter()
        .map(|u| {
            let found = core::saved_bookmark(&marks, u)
                .and_then(|f| marks.iter().position(|r| std::ptr::eq(r, f)));
            found.map_or(Value::Null, |i| list[i].clone())
        })
        .collect())
}
