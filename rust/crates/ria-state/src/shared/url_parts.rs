// Ported from ardia apps/browser/src/lib/urlParts.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! A strict URL splitter: scheme, host, port, path, query and hash, with the host made ASCII
//! (punycode) and checked for spoofing, and IPv6 hosts in canonical form.
//!
//! Changes from Ardia:
//! - `urlParts` takes `&str`; the original also returns `null` for a value that is not a string.
//! - `effectivePort`, `hostWithPort`, `originOf` and `formatUrl` take a [`UrlParts`]; the bridge
//!   crate keeps the JavaScript rules for objects of other shapes.
//! - JavaScript `.` does not match `\n`, `\r`, U+2028 or U+2029, and `\d` is ASCII: the patterns
//!   spell those classes out.
//! - `effectivePort` reads `DEFAULT_PORTS` as a table: for a scheme that names an `Object.prototype`
//!   member (`constructor`) and an empty port, the original returns that function; the port gives `"-1"`.

use regex::Regex;
use ria_core::url::idn::{host_spoof_reason, punycode_encode};
use serde::Serialize;
use std::sync::LazyLock;
use unicode_normalization::UnicodeNormalization;

/// `UrlParts` (`urlParts.ts:3`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UrlParts {
    pub scheme: String,
    pub host: String,
    pub port: String,
    pub path: String,
    pub query: String,
    pub hash: String,
    pub authority: bool,
}

static SHAPE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^([a-zA-Z][a-zA-Z0-9+.-]*):(//([^/?#]*))?([^?#]*)(\?[^#]*)?(#[^\n\r\u{2028}\u{2029}]*)?$",
    )
    .expect("static pattern")
});
static NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9_-]+(?:\.[a-z0-9_-]+)*\.?$").expect("static pattern"));
static DOTTED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9]?[0-9])(\.(25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9]?[0-9])){3}$").expect("static pattern")
});
static NUMERIC_LABEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(0x[0-9a-f]*|[0-9]+)$").expect("static pattern"));
static UNICODE_LABEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\p{L}\p{M}\p{N}-]{1,63}$").expect("static pattern"));

/// `DEFAULT_PORTS` (`urlParts.ts:14`).
pub fn default_port(scheme: &str) -> Option<&'static str> {
    match scheme {
        "http" => Some("80"),
        "https" => Some("443"),
        "ftp" => Some("21"),
        _ => None,
    }
}

/// `canonicalIpv6` (`urlParts.ts:19`): the compressed lower-case form, or `None` for text that is not
/// an IPv6 address in lower case.
pub fn canonical_ipv6(text: &str) -> Option<String> {
    if text.is_empty()
        || !text
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f' | b':'))
        || text.matches("::").count() > 1
    {
        return None;
    }
    let compressed = text.contains("::");
    let (head, tail) = if compressed {
        text.split_once("::").expect("contains ::")
    } else {
        (text, "")
    };
    fn split(s: &str) -> Vec<&str> {
        if s.is_empty() {
            Vec::new()
        } else {
            s.split(':').collect()
        }
    }
    let (left, right) = (split(head), split(tail));
    let missing = 8 - left.len() as isize - right.len() as isize;
    if if compressed {
        missing < 1
    } else {
        left.len() != 8
    } {
        return None;
    }
    let fill = if compressed { missing as usize } else { 0 };
    let groups: Vec<&str> = left
        .iter()
        .copied()
        .chain(std::iter::repeat_n("0", fill))
        .chain(right.iter().copied())
        .collect();
    if groups.iter().any(|g| g.is_empty() || g.len() > 4) {
        return None;
    }
    let values: Vec<String> = groups
        .iter()
        .map(|g| {
            format!(
                "{:x}",
                u16::from_str_radix(g, 16).expect("1 to 4 hex digits")
            )
        })
        .collect();
    let (mut best, mut best_length) = (None, 1);
    let mut i = 0;
    while i < 8 {
        if values[i] != "0" {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < 8 && values[j] == "0" {
            j += 1;
        }
        if j - i > best_length {
            (best, best_length) = (Some(i), j - i);
        }
        i = j;
    }
    Some(match best {
        None => values.join(":"),
        Some(best) => format!(
            "{}::{}",
            values[..best].join(":"),
            values[best + best_length..].join(":")
        ),
    })
}

/// `DEVIATIONS` (`urlParts.ts:46`).
fn has_deviation(s: &str) -> bool {
    s.contains([
        '\u{00df}', '\u{03c2}', '\u{200c}', '\u{200d}', '\u{3002}', '\u{ff0e}', '\u{ff61}',
    ])
}

/// `asciiHost` (`urlParts.ts:49`): the host with each Unicode label in punycode, or `None` when a
/// label is not allowed or the result looks like a spoof.
pub fn ascii_host(host: &str) -> Option<String> {
    if host.is_ascii() {
        return Some(host.to_string());
    }
    let lower = host.nfc().collect::<String>().to_lowercase();
    if has_deviation(&lower) || lower.nfkc().collect::<String>() != lower {
        return None;
    }
    let labels = lower
        .split('.')
        .map(|label| {
            if label.is_ascii() {
                return Some(label.to_string());
            }
            if !UNICODE_LABEL.is_match(label) || label.starts_with('-') || label.ends_with('-') {
                return None;
            }
            let encoded = format!("xn--{}", punycode_encode(label));
            (encoded.encode_utf16().count() <= 63).then_some(encoded)
        })
        .collect::<Option<Vec<_>>>()?;
    let ascii = labels.join(".");
    host_spoof_reason(&ascii).is_none().then_some(ascii)
}

/// `plainHost` (`urlParts.ts:64`).
pub fn plain_host(given: &str) -> Option<String> {
    let raw = if given.starts_with('[') {
        given.to_string()
    } else {
        ascii_host(given)?
    };
    if raw.starts_with('[') {
        if !raw.ends_with(']') || raw.len() < 2 {
            return None;
        }
        let inner = &raw[1..raw.len() - 1];
        return (canonical_ipv6(inner).as_deref() == Some(inner)).then_some(raw);
    }
    let host = raw.to_lowercase();
    if !NAME.is_match(&host) {
        return None;
    }
    let last = host
        .strip_suffix('.')
        .unwrap_or(&host)
        .rsplit('.')
        .next()
        .unwrap_or_default();
    if NUMERIC_LABEL.is_match(last) {
        return DOTTED.is_match(&host).then_some(host);
    }
    Some(host)
}

/// The index of the `:` before the port: after the `]` of an IPv6 host (`urlParts.ts:91`).
pub(super) fn port_colon(host_and_port: &str) -> Option<usize> {
    let from = if host_and_port.starts_with('[') {
        host_and_port.find(']').unwrap_or(0)
    } else {
        0
    };
    host_and_port[from..].find(':').map(|i| from + i)
}

/// `urlParts` (`urlParts.ts:79`).
pub fn url_parts(href: &str) -> Option<UrlParts> {
    if href.contains(|c: char| c <= ' ' || c == '\u{7f}' || c == '\\') {
        return None;
    }
    let found = SHAPE.captures(href)?;
    let scheme = found[1].to_lowercase();
    let authority = found.get(2).is_some();
    let web = scheme == "http" || scheme == "https";
    let mut host = String::new();
    let mut port = String::new();
    if authority {
        let host_and_port = &found[3];
        if host_and_port.contains(['@', '%']) {
            return None;
        }
        let colon = port_colon(host_and_port);
        let raw_host = colon.map_or(host_and_port, |c| &host_and_port[..c]);
        if let Some(c) = colon {
            port = host_and_port[c + 1..].to_string();
            let b = port.as_bytes();
            if b.is_empty()
                || b.len() > 5
                || b[0] == b'0'
                || !b.iter().all(u8::is_ascii_digit)
                || port.parse::<u32>().ok()? > 65535
            {
                return None;
            }
            if Some(port.as_str()) == default_port(&scheme) {
                port.clear();
            }
        }
        if !raw_host.is_empty() {
            host = plain_host(raw_host)?;
        }
    }
    if web && (!authority || host.is_empty()) {
        return None;
    }
    let path = match found.get(4).map_or("", |m| m.as_str()) {
        "" if authority => "/",
        p => p,
    };
    Some(UrlParts {
        scheme,
        host,
        port,
        path: path.to_string(),
        query: found.get(5).map_or("", |m| m.as_str()).to_string(),
        hash: found.get(6).map_or("", |m| m.as_str()).to_string(),
        authority,
    })
}

/// `effectivePort` (`urlParts.ts:109`): the port, else the scheme's default port, else `"-1"`.
pub fn effective_port(parts: &UrlParts) -> String {
    if !parts.port.is_empty() {
        return parts.port.clone();
    }
    default_port(&parts.scheme).unwrap_or("-1").to_string()
}

/// `hostWithPort` (`urlParts.ts:111`).
pub fn host_with_port(parts: &UrlParts) -> String {
    if parts.port.is_empty() {
        parts.host.clone()
    } else {
        format!("{}:{}", parts.host, parts.port)
    }
}

/// `originOf` (`urlParts.ts:113`).
pub fn origin_of(parts: &UrlParts) -> String {
    format!("{}://{}", parts.scheme, host_with_port(parts))
}

/// `formatUrl` (`urlParts.ts:115`).
pub fn format_url(parts: &UrlParts) -> String {
    let start = if parts.authority {
        origin_of(parts)
    } else {
        format!("{}:", parts.scheme)
    };
    format!("{start}{}{}{}", parts.path, parts.query, parts.hash)
}

/// `webUrl` (`urlParts.ts:117`): the parts of an `http` or `https` URL.
pub fn web_url(href: &str) -> Option<UrlParts> {
    url_parts(href).filter(|p| p.scheme == "http" || p.scheme == "https")
}

/// `minimizedUrl` (`urlParts.ts:122`): the URL up to its first `?` or `#`.
pub fn minimized_url(url: &str) -> String {
    url.find(['?', '#'])
        .map_or(url, |cut| &url[..cut])
        .to_string()
}
