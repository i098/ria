// Ported from ardia apps/browser/src/lib/unread.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::unread`: `Unread` crosses as `{ kind, label }`.
//! The logic is in `ria_state::shared::unread`.

#![forbid(unsafe_code)]

use ria_state::shared::unread::{self as u, Unread};
use serde_json::{Value, json};

fn to_js(unread: Option<Unread>) -> Option<Value> {
    unread.map(|v| match v {
        Unread::Count { label } => json!({ "kind": "count", "label": label }),
        Unread::Dot => json!({ "kind": "dot" }),
    })
}

/// An `Unread` from JS; any other value, falsy or of another shape, never matches a title's mark.
fn from_js(value: &Value) -> Option<Unread> {
    match (value.get("kind")?.as_str()?, value.get("label")) {
        ("dot", _) => Some(Unread::Dot),
        ("count", Some(Value::String(label))) => Some(Unread::Count {
            label: label.clone(),
        }),
        _ => None,
    }
}

/// Port of `unreadCount` (`apps/browser/src/lib/unread.ts:43`).
pub fn unread_count(n: f64, more: Option<bool>) -> Result<Option<Value>, String> {
    Ok(to_js(u::unread_count(n, more.unwrap_or(false))))
}

/// Port of `titleWithoutUnread` (`apps/browser/src/lib/unread.ts:51`).
pub fn title_without_unread(title: String, unread: Option<Value>) -> Result<String, String> {
    Ok(u::title_without_unread(
        &title,
        unread.as_ref().and_then(from_js).as_ref(),
    ))
}

/// Port of `hostOfOrigin` (`apps/browser/src/lib/unread.ts:60`).
pub fn host_of_origin(origin: String) -> Result<String, String> {
    Ok(u::host_of_origin(&origin))
}

/// Port of `unreadInTitle` (`apps/browser/src/lib/unread.ts:78`).
pub fn unread_in_title(raw: String, host: Option<String>) -> Result<Option<Value>, String> {
    Ok(to_js(u::unread_in_title(
        &raw,
        host.as_deref().unwrap_or(""),
    )))
}

/// Port of `originOf` (`apps/browser/src/lib/unread.ts:91`).
pub fn origin_of(url: String) -> Result<String, String> {
    Ok(u::origin_of(&url))
}

/// Port of `faviconIsBadgeNamed` (`apps/browser/src/lib/unread.ts:103`).
pub fn favicon_is_badge_named(url: String) -> Result<Value, String> {
    Ok(json!(u::favicon_is_badge_named(&url)))
}

/// Port of `faviconIsOrdinary` (`apps/browser/src/lib/unread.ts:105`).
pub fn favicon_is_ordinary(url: String) -> Result<Value, String> {
    Ok(json!(u::favicon_is_ordinary(&url)))
}

/// Port of `faviconLooksBadged` (`apps/browser/src/lib/unread.ts:107`).
pub fn favicon_looks_badged(home: String, now: String) -> Result<bool, String> {
    Ok(u::favicon_looks_badged(&home, &now))
}
