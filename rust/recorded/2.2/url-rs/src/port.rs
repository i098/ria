// Ported from ardia apps/browser/src/lib/url.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::url`: it reads the options object of `urlTrail` with
//! the JavaScript destructuring rules. The logic is in `ria_state::shared::url`.

#![forbid(unsafe_code)]

use crate::js::is_undefined;
use ria_state::shared::url as u;
use serde_json::{Value, json};

fn type_error(message: &str) -> String {
    format!("TypeError: {message}")
}

/// Port of `withoutFragment` (`apps/browser/src/lib/url.ts:3`).
pub fn without_fragment(url: String) -> Result<Value, String> {
    Ok(json!(u::without_fragment(&url)))
}

/// Port of `withoutUserInfo` (`apps/browser/src/lib/url.ts:5`).
pub fn without_user_info(url: String) -> Result<Value, String> {
    Ok(json!(u::without_user_info(&url)))
}

/// Port of `samePage` (`apps/browser/src/lib/url.ts:7`).
pub fn same_page(a: String, b: String) -> Result<Value, String> {
    Ok(json!(u::same_page(&a, &b)))
}

/// Port of `isWebPageUrl` (`apps/browser/src/lib/url.ts:9`).
pub fn is_web_page_url(url: String) -> Result<Value, String> {
    Ok(json!(u::is_web_page_url(&url)))
}

/// Port of `displayHostOf` (`apps/browser/src/lib/url.ts:11`).
pub fn display_host_of(url: String) -> Result<Value, String> {
    Ok(json!(u::display_host_of(&url)))
}

/// Port of `pageHostLabel` (`apps/browser/src/lib/url.ts:13`).
pub fn page_host_label(url: String) -> Result<Value, String> {
    Ok(json!(u::page_host_label(&url).map_err(type_error)?))
}

/// JavaScript truthiness of a property read from a structured value.
fn truthy_property(object: &Value, key: &str) -> bool {
    match object.as_object().and_then(|o| o.get(key)) {
        None | Some(Value::Null) => false,
        Some(v) if is_undefined(v) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Number(n)) => n.as_f64() != Some(0.0),
        Some(Value::Object(o)) => o
            .get("$num")
            .and_then(Value::as_str)
            .is_none_or(|n| n != "NaN" && n != "-0"),
        Some(Value::Array(_)) => true,
    }
}

/// Port of `urlTrail` (`apps/browser/src/lib/url.ts:15`).
pub fn url_trail(url: String, title: String, arg2: Value) -> Result<String, String> {
    if arg2.is_null() {
        return Err(type_error(
            "Cannot destructure property 'hovered' of 'object null' as it is null.",
        ));
    }
    if is_undefined(&arg2) {
        return Err(type_error(
            "Cannot destructure property 'hovered' of 'undefined' as it is undefined.",
        ));
    }
    u::url_trail(
        &url,
        &title,
        truthy_property(&arg2, "hovered"),
        truthy_property(&arg2, "showFullUrl"),
    )
    .map_err(type_error)
}

/// Port of `hostOrOrigin` (`apps/browser/src/lib/url.ts:33`).
pub fn host_or_origin(origin: String) -> Result<Value, String> {
    Ok(json!(u::host_or_origin(&origin)))
}

/// Port of `originForSecurityDisplay` (`apps/browser/src/lib/url.ts:35`).
pub fn origin_for_security_display(origin: String) -> Result<String, String> {
    Ok(u::origin_for_security_display(&origin))
}
