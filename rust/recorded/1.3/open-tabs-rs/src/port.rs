// Ported from ardia packages/core/src/openTabs.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `omnibox::open_tabs`: it maps the JSON tabs to typed rows and
//! gives an error where the original throws. The logic is in `ria_core::omnibox::open_tabs`.

#![forbid(unsafe_code)]

use ria_core::omnibox::open_tabs::{self as core, OpenTabRow};
use ria_core::omnibox::suggestion::Suggestion;
use serde_json::{Map, Value};

use crate::js;

/// A JS string value, lone surrogates replaced.
fn string(v: &Value) -> Option<String> {
    js::string_units(v).map(|u| String::from_utf16_lossy(&u))
}

/// `slice(0, limit)` end for `n` rows.
fn slice_end(limit: f64, n: usize) -> usize {
    let n = n as f64;
    let end = match limit.trunc() {
        l if l.is_nan() => 0.0,
        l if l < 0.0 => (n + l).max(0.0),
        l => l.min(n),
    };
    end as usize
}

/// Port of `recentTabSuggestions` (`packages/core/src/openTabs.ts:5`).
pub fn recent_tab_suggestions(tabs: Vec<Value>, options: Value) -> Result<Vec<Value>, String> {
    if options.is_null() {
        return Err(
            "TypeError: Cannot destructure property 'exclude' of 'options' as it is null.".into(),
        );
    }
    let get = |key: &str| options.get(key);
    let exclude = get("exclude").and_then(string);
    let limit = match get("limit") {
        None => core::DEFAULT_LIMIT as f64,
        Some(v) if js::is_undefined(v) => core::DEFAULT_LIMIT as f64,
        Some(v) => js::as_number(v).unwrap_or(f64::NAN),
    };
    let rows: Vec<OpenTabRow> = tabs
        .iter()
        .map(|t| OpenTabRow {
            id: t.get("id").and_then(string).unwrap_or_default(),
            url: t.get("url").and_then(string).unwrap_or_default(),
            title: t.get("title").and_then(string).unwrap_or_default(),
            favicon: t.get("favicon").and_then(string),
            last_active_at: t
                .get("lastActiveAt")
                .and_then(js::as_number)
                .unwrap_or(f64::NAN),
        })
        .collect();
    let all = core::recent_tab_suggestions(&rows, exclude.as_deref(), usize::MAX);
    let end = slice_end(limit, all.len());
    Ok(all[..end]
        .iter()
        .map(|s| {
            let mut o = Map::new();
            if let Suggestion::Page {
                url,
                title,
                favicon,
                tab_id,
                ..
            } = s
            {
                o.insert("kind".into(), "page".into());
                o.insert("url".into(), url.as_str().into());
                o.insert("title".into(), title.as_str().into());
                o.insert("favicon".into(), favicon.clone().into());
                o.insert("tabId".into(), tab_id.clone().into());
            }
            Value::Object(o)
        })
        .collect())
}
