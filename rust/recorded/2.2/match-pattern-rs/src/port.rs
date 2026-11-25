// Ported from ardia apps/browser/src/lib/matchPattern.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::match_pattern`. A well-formed `MatchSpec` goes to the
//! Rust functions; any other value follows the JavaScript property rules here, so a recorded call
//! with a malformed spec gives the same result or `TypeError`. The logic is in `ria_state::shared::match_pattern`.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, string_units, undefined};
use ria_state::shared::match_pattern::{self as m, MatchSpec};
use ria_state::shared::url::decode_uri;
use ria_state::shared::url_parts::{UrlParts, effective_port, url_parts};
use serde_json::{Value, json};

fn type_error(message: String) -> String {
    format!("TypeError: {message}")
}

fn spec_of(v: &Value) -> Option<MatchSpec> {
    let o = v.as_object()?;
    let s = |k: &str| o.get(k)?.as_str().map(str::to_string);
    let b = |k: &str| o.get(k)?.as_bool();
    Some(MatchSpec {
        all: b("all")?,
        schemes: o
            .get("schemes")?
            .as_array()?
            .iter()
            .map(|s| s.as_str().map(str::to_string))
            .collect::<Option<_>>()?,
        host: s("host")?,
        sub: b("sub")?,
        port: s("port")?,
        file: b("file")?,
        path: s("path")?,
    })
}

fn nullish(v: &Value) -> Option<&'static str> {
    if v.is_null() {
        Some("null")
    } else if is_undefined(v) {
        Some("undefined")
    } else {
        None
    }
}

/// `value[key]`: a `TypeError` on `null` and `undefined`, own properties of plain objects, else `undefined`.
fn get(v: &Value, key: &str) -> Result<Value, String> {
    if let Some(what) = nullish(v) {
        return Err(type_error(format!(
            "Cannot read properties of {what} (reading '{key}')"
        )));
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

/// `globMatches` over a string or an array, whose items are compared with `===` to single code units.
fn js_glob(glob: &Value, text: &str) -> Result<bool, String> {
    if let Some(units) = string_units(glob) {
        return Ok(m::glob_matches(&String::from_utf16_lossy(&units), text));
    }
    if let Some(what) = nullish(glob) {
        let key = if text.is_empty() { "0" } else { "length" };
        return Err(type_error(format!(
            "Cannot read properties of {what} (reading '{key}')"
        )));
    }
    let Value::Array(items) = glob else {
        return Ok(false);
    };
    let unit = |v: &Value| {
        v.as_str().and_then(|s| {
            (s.encode_utf16().count() == 1)
                .then(|| s.encode_utf16().next())
                .flatten()
        })
    };
    let glob: Vec<Option<u16>> = items.iter().map(unit).collect();
    let text: Vec<u16> = text.encode_utf16().collect();
    let star = Some(u16::from(b'*'));
    let (mut g, mut t, mut back, mut resume) = (0, 0, None, 0);
    while t < text.len() {
        if g < glob.len() && glob[g] != star && glob[g] == Some(text[t]) {
            (g, t) = (g + 1, t + 1);
        } else if g < glob.len() && glob[g] == star {
            (back, resume) = (Some(g), t);
            g += 1;
        } else if let Some(s) = back {
            g = s + 1;
            resume += 1;
            t = resume;
        } else {
            return Ok(false);
        }
    }
    while glob.get(g).is_some_and(|u| *u == star) {
        g += 1;
    }
    Ok(g == glob.len())
}

/// `matchesUrl` for a spec that is not a well-formed `MatchSpec`.
fn js_matches_url(spec: &Value, url: &UrlParts) -> Result<bool, String> {
    let schemes = get(spec, "schemes")?;
    let listed = if let Some(units) = string_units(&schemes) {
        String::from_utf16_lossy(&units).contains(&url.scheme)
    } else if let Some(what) = nullish(&schemes) {
        return Err(type_error(format!(
            "Cannot read properties of {what} (reading 'includes')"
        )));
    } else if let Value::Array(items) = &schemes {
        items
            .iter()
            .any(|s| s.as_str() == Some(url.scheme.as_str()))
    } else {
        return Err(type_error("spec.schemes.includes is not a function".into()));
    };
    if !listed {
        return Ok(false);
    }
    if truthy(&get(spec, "all")?) {
        return Ok(true);
    }
    if !truthy(&get(spec, "file")?) {
        let host = get(spec, "host")?;
        let Some(pattern) = string_units(&host) else {
            return Err(type_error(match nullish(&host) {
                Some(what) => format!("Cannot read properties of {what} (reading 'replace')"),
                None => "spec.host.replace is not a function".into(),
            }));
        };
        let typed = MatchSpec {
            all: false,
            schemes: Vec::new(),
            host: String::from_utf16_lossy(&pattern),
            sub: truthy(&get(spec, "sub")?),
            port: "*".into(),
            file: false,
            path: String::new(),
        };
        if !m::host_matches(&typed, &url.host) {
            return Ok(false);
        }
    }
    let port = get(spec, "port")?;
    if port.as_str() != Some("*") && port.as_str() != Some(effective_port(url).as_str()) {
        return Ok(false);
    }
    let path = format!("{}{}", url.path, url.query);
    let spec_path = get(spec, "path")?;
    if spec_path.as_str() == Some(format!("{path}/*").as_str()) {
        return Ok(true);
    }
    let decoded = decode_uri(&path).unwrap_or_else(|| path.clone());
    Ok(js_glob(&spec_path, &path)? || js_glob(&spec_path, &decoded)?)
}

/// Port of `parseMatchPattern` (`apps/browser/src/lib/matchPattern.ts:17`).
pub fn parse_match_pattern(pattern: String) -> Result<Value, String> {
    Ok(match m::parse_match_pattern(&pattern) {
        Ok(spec) => json!(spec),
        Err(error) => json!({ "error": error }),
    })
}

/// Port of `globMatches` (`apps/browser/src/lib/matchPattern.ts:60`).
pub fn glob_matches(glob: String, text: String) -> Result<bool, String> {
    Ok(m::glob_matches(&glob, &text))
}

/// Port of `matchesUrl` (`apps/browser/src/lib/matchPattern.ts:92`).
pub fn matches_url(spec: Value, href: String) -> Result<bool, String> {
    if let Some(typed) = spec_of(&spec) {
        return Ok(m::matches_url(&typed, &href));
    }
    match url_parts(&href) {
        Some(url) => js_matches_url(&spec, &url),
        None => Ok(false),
    }
}

/// Port of `anyMatches` (`apps/browser/src/lib/matchPattern.ts:107`).
pub fn any_matches(specs: Value, href: String) -> Result<Value, String> {
    if let Some(what) = nullish(&specs) {
        return Err(type_error(format!(
            "Cannot read properties of {what} (reading 'some')"
        )));
    }
    let Value::Array(items) = specs else {
        return Err(type_error("specs.some is not a function".into()));
    };
    if let Some(typed) = items.iter().map(spec_of).collect::<Option<Vec<_>>>() {
        return Ok(json!(m::any_matches(&typed, &href)));
    }
    for spec in items {
        if matches_url(spec, href.clone())? {
            return Ok(json!(true));
        }
    }
    Ok(json!(false))
}
