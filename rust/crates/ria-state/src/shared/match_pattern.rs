// Ported from ardia apps/browser/src/lib/matchPattern.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! WebExtension match patterns (`<all_urls>`, `scheme://host/path`): parse them and test URLs.
//!
//! Changes from Ardia:
//! - `parseMatchPattern` gives `Err(message)` in place of `{ error: message }`.
//! - `globMatches` compares UTF-16 code units, as the original does.

use serde::Serialize;

use super::url::decode_uri;
use super::url_parts::{effective_port, port_colon, url_parts};

/// `MatchSpec` (`matchPattern.ts:3`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MatchSpec {
    pub all: bool,
    pub schemes: Vec<String>,
    pub host: String,
    pub sub: bool,
    pub port: String,
    pub file: bool,
    pub path: String,
}

/// `SCRIPT_SCHEMES` (`matchPattern.ts:5`).
pub const SCRIPT_SCHEMES: [&str; 4] = ["http", "https", "file", "ftp"];
/// `WILDCARD_SCHEMES` (`matchPattern.ts:6`).
pub const WILDCARD_SCHEMES: [&str; 2] = ["http", "https"];

/// `checkPort` (`matchPattern.ts:8`): the problem with the port, if any.
pub fn check_port(scheme: &str, port: &str) -> Option<&'static str> {
    if port == "*" {
        return None;
    }
    if scheme == "*" {
        return Some("a pattern with scheme * can't name a port");
    }
    if port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) {
        return Some("the port is a number or *");
    }
    if port.len() > 1 && port.starts_with('0') {
        return Some("write the port without leading zeros");
    }
    if port.parse::<f64>().is_ok_and(|p| p > 65535.0) {
        return Some("the port must be 0 to 65535");
    }
    None
}

/// `parseMatchPattern` (`matchPattern.ts:17`).
pub fn parse_match_pattern(pattern: &str) -> Result<MatchSpec, String> {
    let strings = |s: &[&str]| s.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    if pattern == "<all_urls>" {
        return Ok(MatchSpec {
            all: true,
            schemes: strings(&SCRIPT_SCHEMES),
            host: String::new(),
            sub: true,
            port: "*".into(),
            file: false,
            path: "/*".into(),
        });
    }
    if pattern.contains('\0') {
        return Err("contains a null character".into());
    }
    let Some((scheme, rest)) = pattern.split_once("://") else {
        return Err("a match pattern is <all_urls> or scheme://host/path".into());
    };
    if scheme != "*" && !SCRIPT_SCHEMES.contains(&scheme) {
        return Err(format!(
            "match patterns only match *, {} URLs",
            SCRIPT_SCHEMES.join(", ")
        ));
    }
    let schemes = if scheme == "*" {
        strings(&WILDCARD_SCHEMES)
    } else {
        vec![scheme.to_string()]
    };
    if rest.is_empty() {
        return Err("the host is empty".into());
    }
    if scheme == "file" {
        let Some(slash) = rest.find('/') else {
            return Err("add a path, like file:///*".into());
        };
        let host = &rest[..slash];
        if !host.is_empty() && host != "localhost" {
            return Err("file patterns have no host (file:///path)".into());
        }
        return Ok(MatchSpec {
            all: false,
            schemes,
            host: String::new(),
            sub: false,
            port: "*".into(),
            file: true,
            path: rest[slash..].to_string(),
        });
    }
    let end = match rest.find('/') {
        Some(0) => return Err("the host is empty".into()),
        None => return Err("add a path, like /*".into()),
        Some(end) => end,
    };
    let host_and_port = &rest[..end];
    let colon = port_colon(host_and_port);
    let mut host = colon
        .map_or(host_and_port, |c| &host_and_port[..c])
        .to_string();
    let port = colon.map_or("*", |c| &host_and_port[c + 1..]);
    if let Some(problem) = check_port(scheme, port) {
        return Err(problem.into());
    }
    if host.is_empty() {
        return Err("the host is empty".into());
    }
    let mut sub = false;
    if host == "*" {
        (sub, host) = (true, String::new());
    } else if let Some(rest) = host.strip_prefix("*.") {
        if rest.is_empty() {
            return Err("the host is empty".into());
        }
        (sub, host) = (true, rest.to_string());
    }
    if host.contains('*') {
        return Err("* only works as the whole host or a *. prefix".into());
    }
    if host.contains('%') {
        return Err("hosts can't have %".into());
    }
    if !host.is_empty() && !host.starts_with('[') {
        let Some(parsed) = url_parts(&format!("http://{host}/")) else {
            return Err("the host has characters a hostname can't have".into());
        };
        host = parsed.host;
    }
    Ok(MatchSpec {
        all: false,
        schemes,
        host: host.to_lowercase(),
        sub,
        port: port.to_string(),
        file: false,
        path: rest[end..].to_string(),
    })
}

/// `globMatches` (`matchPattern.ts:60`): `*` matches any run of code units, other units match themselves.
pub fn glob_matches(glob: &str, text: &str) -> bool {
    let (glob, text): (Vec<u16>, Vec<u16>) =
        (glob.encode_utf16().collect(), text.encode_utf16().collect());
    let star_unit = u16::from(b'*');
    let (mut g, mut t, mut star, mut resume) = (0, 0, None, 0);
    while t < text.len() {
        if g < glob.len() && glob[g] != star_unit && glob[g] == text[t] {
            g += 1;
            t += 1;
        } else if g < glob.len() && glob[g] == star_unit {
            star = Some(g);
            g += 1;
            resume = t;
        } else if let Some(s) = star {
            g = s + 1;
            resume += 1;
            t = resume;
        } else {
            return false;
        }
    }
    while glob.get(g) == Some(&star_unit) {
        g += 1;
    }
    g == glob.len()
}

/// `isIp` (`matchPattern.ts:81`): an IPv6 host or four dotted numbers.
pub fn is_ip(host: &str) -> bool {
    host.starts_with('[') || {
        let parts: Vec<&str> = host.split('.').collect();
        parts.len() == 4
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    }
}

/// `hostMatches` (`matchPattern.ts:83`).
pub fn host_matches(spec: &MatchSpec, given: &str) -> bool {
    let host = given.trim_end_matches('.');
    let pattern = spec.host.trim_end_matches('.');
    if host == pattern {
        return true;
    }
    if spec.sub && pattern.is_empty() {
        return true;
    }
    if !spec.sub || is_ip(host) {
        return false;
    }
    host.len() > pattern.len() + 1 && host.ends_with(&format!(".{pattern}"))
}

/// `matchesUrl` (`matchPattern.ts:92`).
pub fn matches_url(spec: &MatchSpec, href: &str) -> bool {
    let Some(url) = url_parts(href) else {
        return false;
    };
    if !spec.schemes.contains(&url.scheme) {
        return false;
    }
    if spec.all {
        return true;
    }
    if !spec.file && !host_matches(spec, &url.host) {
        return false;
    }
    if spec.port != "*" && spec.port != effective_port(&url) {
        return false;
    }
    let path = format!("{}{}", url.path, url.query);
    if spec.path == format!("{path}/*") {
        return true;
    }
    let decoded = decode_uri(&path).unwrap_or_else(|| path.clone());
    glob_matches(&spec.path, &path) || glob_matches(&spec.path, &decoded)
}

/// `anyMatches` (`matchPattern.ts:107`).
pub fn any_matches(specs: &[MatchSpec], href: &str) -> bool {
    specs.iter().any(|spec| matches_url(spec, href))
}
