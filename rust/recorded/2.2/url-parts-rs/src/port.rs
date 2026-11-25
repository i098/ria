// Ported from ardia apps/browser/src/lib/urlParts.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::url_parts`. A well-formed `UrlParts` object goes to
//! the Rust functions; any other value follows the JavaScript property and string rules here,
//! so a recorded call with a malformed argument gives the same value or `TypeError`.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, number_to_string, string_units, undefined};
use ria_state::shared::url_parts::{self as p, UrlParts};
use serde_json::{Value, json};

fn parts_of(v: &Value) -> Option<UrlParts> {
    let o = v.as_object()?;
    let s = |k: &str| o.get(k)?.as_str().map(str::to_string);
    Some(UrlParts {
        scheme: s("scheme")?,
        host: s("host")?,
        port: s("port")?,
        path: s("path")?,
        query: s("query")?,
        hash: s("hash")?,
        authority: o.get("authority")?.as_bool()?,
    })
}

/// `value[key]`: a `TypeError` on `null` and `undefined`, own properties of plain objects, else `undefined`.
fn get(v: &Value, key: &str) -> Result<Value, String> {
    if v.is_null() || is_undefined(v) {
        let what = if v.is_null() { "null" } else { "undefined" };
        return Err(format!(
            "TypeError: Cannot read properties of {what} (reading '{key}')"
        ));
    }
    let plain = v
        .as_object()
        .filter(|_| as_number(v).is_none() && string_units(v).is_none());
    Ok(plain
        .and_then(|o| o.get(key))
        .cloned()
        .unwrap_or_else(undefined))
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        Value::Number(n) => n.as_f64() != Some(0.0),
        _ if is_undefined(v) => false,
        _ => as_number(v).is_none_or(|n| n != 0.0 && !n.is_nan()),
    }
}

/// `String(value)` for a template literal.
fn text(v: &Value) -> String {
    if let Some(units) = string_units(v) {
        return String::from_utf16_lossy(&units);
    }
    if let Some(n) = as_number(v) {
        return number_to_string(n);
    }
    match v {
        _ if is_undefined(v) => "undefined".into(),
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Array(items) => items
            .iter()
            .map(|i| {
                if i.is_null() || is_undefined(i) {
                    String::new()
                } else {
                    text(i)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        _ => "[object Object]".into(),
    }
}

fn js_host_with_port(parts: &Value) -> Result<Value, String> {
    let port = get(parts, "port")?;
    if truthy(&port) {
        Ok(json!(format!(
            "{}:{}",
            text(&get(parts, "host")?),
            text(&port)
        )))
    } else {
        get(parts, "host")
    }
}

fn js_origin_of(parts: &Value) -> Result<String, String> {
    let scheme = get(parts, "scheme")?;
    Ok(format!(
        "{}://{}",
        text(&scheme),
        text(&js_host_with_port(parts)?)
    ))
}

/// Port of `canonicalIpv6` (`apps/browser/src/lib/urlParts.ts:19`).
pub fn canonical_ipv6(text: String) -> Result<Option<String>, String> {
    Ok(p::canonical_ipv6(&text))
}

/// Port of `asciiHost` (`apps/browser/src/lib/urlParts.ts:49`).
pub fn ascii_host(host: String) -> Result<Option<String>, String> {
    Ok(p::ascii_host(&host))
}

/// Port of `urlParts` (`apps/browser/src/lib/urlParts.ts:79`).
pub fn url_parts(href: String) -> Result<Option<Value>, String> {
    Ok(p::url_parts(&href).map(|parts| json!(parts)))
}

/// Port of `effectivePort` (`apps/browser/src/lib/urlParts.ts:109`).
pub fn effective_port(parts: Value) -> Result<Value, String> {
    if let Some(typed) = parts_of(&parts) {
        return Ok(json!(p::effective_port(&typed)));
    }
    let port = get(&parts, "port")?;
    if truthy(&port) {
        return Ok(port);
    }
    Ok(json!(
        p::default_port(&text(&get(&parts, "scheme")?)).unwrap_or("-1")
    ))
}

/// Port of `hostWithPort` (`apps/browser/src/lib/urlParts.ts:111`).
pub fn host_with_port(parts: Value) -> Result<Value, String> {
    match parts_of(&parts) {
        Some(typed) => Ok(json!(p::host_with_port(&typed))),
        None => js_host_with_port(&parts),
    }
}

/// Port of `originOf` (`apps/browser/src/lib/urlParts.ts:113`).
pub fn origin_of(parts: Value) -> Result<Value, String> {
    match parts_of(&parts) {
        Some(typed) => Ok(json!(p::origin_of(&typed))),
        None => Ok(json!(js_origin_of(&parts)?)),
    }
}

/// Port of `formatUrl` (`apps/browser/src/lib/urlParts.ts:115`).
pub fn format_url(parts: Value) -> Result<Value, String> {
    if let Some(typed) = parts_of(&parts) {
        return Ok(json!(p::format_url(&typed)));
    }
    let start = if truthy(&get(&parts, "authority")?) {
        js_origin_of(&parts)?
    } else {
        format!("{}:", text(&get(&parts, "scheme")?))
    };
    Ok(json!(format!(
        "{start}{}{}{}",
        text(&get(&parts, "path")?),
        text(&get(&parts, "query")?),
        text(&get(&parts, "hash")?)
    )))
}

/// Port of `webUrl` (`apps/browser/src/lib/urlParts.ts:117`).
pub fn web_url(href: String) -> Result<Option<Value>, String> {
    Ok(p::web_url(&href).map(|parts| json!(parts)))
}

/// Port of `minimizedUrl` (`apps/browser/src/lib/urlParts.ts:122`).
pub fn minimized_url(url: String) -> Result<String, String> {
    Ok(p::minimized_url(&url))
}
