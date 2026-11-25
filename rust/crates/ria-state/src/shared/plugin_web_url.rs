// Ported from ardia apps/browser/src/lib/pluginWebUrl.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The check of a web address a plugin asks to open: http or https with a plain ASCII host,
//! in canonical form, or the problem with it.
//!
//! Changes from Ardia:
//! - `raw: unknown` is `&Json`. The result is the enum [`PluginWebUrl`].
//! - A `&str` cannot hold a lone surrogate, so the `catch` branch ("has text that isn't valid
//!   Unicode", from `encodeURIComponent`) cannot happen and is not ported.
//! - `raw.length` counts UTF-16 code units. `\s` is the JS whitespace set and `\d` is `[0-9]`;
//!   the `i` flag on `^https?://$` is an ASCII case-insensitive compare.

use regex::Regex;
use ria_core::identity::telemetry_wire::Json;
use ria_core::search::js_text::{encode_uri_component, is_js_space};
use std::sync::LazyLock;

use super::url_parts::{canonical_ipv6, format_url, url_parts};

/// `MAX_OPEN_URL_LENGTH` (`pluginWebUrl.ts:3`).
pub const MAX_OPEN_URL_LENGTH: usize = 2048;

static AUTHORITY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)^([A-Za-z][A-Za-z0-9+.-]*://)([^/?#]*)(.*)$").expect("static pattern")
});
static NON_ASCII: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^\x00-\x7f]+").expect("static pattern"));

/// `PluginWebUrl` (`pluginWebUrl.ts:8`).
#[derive(Debug, Clone, PartialEq)]
pub enum PluginWebUrl {
    Url { url: String, host: String },
    Problem(String),
}

/// `withCanonicalHost` (`pluginWebUrl.ts:10`): `raw` with a bracketed IPv6 host in canonical form.
fn with_canonical_host(raw: &str, authority: &str) -> Option<String> {
    if !authority.starts_with('[') {
        return Some(raw.to_owned());
    }
    let close = authority.find(']')?;
    let canonical = canonical_ipv6(&authority[1..close].to_lowercase())?;
    let found = AUTHORITY.captures(raw)?;
    Some(format!(
        "{}[{canonical}]{}{}",
        &found[1],
        &authority[close + 1..],
        &found[3]
    ))
}

fn problem(text: &str) -> PluginWebUrl {
    PluginWebUrl::Problem(text.to_owned())
}

/// `pluginWebUrl` (`pluginWebUrl.ts:19`).
pub fn plugin_web_url(raw: &Json) -> PluginWebUrl {
    let Json::String(raw) = raw else {
        return problem("isn't text");
    };
    if raw.encode_utf16().count() > MAX_OPEN_URL_LENGTH {
        return PluginWebUrl::Problem(format!("is over {MAX_OPEN_URL_LENGTH} characters"));
    }
    if raw
        .chars()
        .any(|ch| matches!(ch, '\0'..='\x1f' | '\x7f'..='\u{9f}'))
    {
        return problem("has a control character");
    }
    if raw.chars().any(is_js_space) {
        return problem("has a space; percent-encode it as %20");
    }
    if raw.contains('\\') {
        return problem("has a backslash");
    }
    let found = AUTHORITY.captures(raw);
    let Some(found) = found.filter(|f| {
        let scheme = &f[1];
        scheme.eq_ignore_ascii_case("http://") || scheme.eq_ignore_ascii_case("https://")
    }) else {
        return problem("isn't an http or https address with a host");
    };
    let authority = &found[2];
    if !authority.is_ascii() {
        return problem(
            "has a host that isn't ASCII; send an international name as punycode (xn--)",
        );
    }
    if authority.contains('@') {
        return problem("has credentials");
    }
    let parts = with_canonical_host(raw, authority)
        .and_then(|plain| url_parts(&plain))
        .filter(|p| p.authority && !p.host.is_empty());
    let Some(parts) = parts else {
        return problem("isn't a valid address (check the host and that the port is 1 to 65535)");
    };
    let formatted = format_url(&parts);
    let url = NON_ASCII.replace_all(&formatted, |run: &regex::Captures| {
        encode_uri_component(&run[0])
    });
    PluginWebUrl::Url {
        url: url.into_owned(),
        host: parts.host,
    }
}
