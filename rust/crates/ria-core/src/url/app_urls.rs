// Ported from ardia packages/core/src/appUrls.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Conversion between `ardia://` app URLs, `chrome://` engine URLs and `about:` aliases.

use super::{is_line_terminator, strip_prefix_ci};

const APP_SCHEME: &str = "ardia";
const APP_PAGE_HOSTS: [&str; 3] = ["history", "bookmarks", "downloads"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppPageHost {
    History,
    Bookmarks,
    Downloads,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppUrlRoute {
    Page(AppPageHost),
    Settings { section: Option<String> },
    NewTab,
    Engine { url: String },
}

struct Split<'a> {
    view_source: &'static str,
    host: String,
    rest: &'a str,
}

/// `split(url, /^(view-source:)?<scheme>:(?:\/\/)?/i)`.
fn split<'a>(url: &'a str, scheme: &str) -> Option<Split<'a>> {
    let (view_source, s) = match strip_prefix_ci(url, "view-source:") {
        Some(after) => ("view-source:", after),
        None => ("", url),
    };
    let s = strip_prefix_ci(s, scheme)?;
    let s = s.strip_prefix(':')?;
    let after = s.strip_prefix("//").unwrap_or(s);
    let host_len = after.find(['/', '?', '#']).unwrap_or(after.len());
    Some(Split {
        view_source,
        host: after[..host_len].to_lowercase(),
        rest: &after[host_len..],
    })
}

/// `about:` followed by `blank` or `srcdoc`, then the end or `?` or `#`.
fn is_about_page(url: &str) -> bool {
    let Some(s) = strip_prefix_ci(url, "about:") else {
        return false;
    };
    ["blank", "srcdoc"].iter().any(|name| {
        strip_prefix_ci(s, name).is_some_and(|r| r.is_empty() || r.starts_with(['?', '#']))
    })
}

/// `/^about:([a-z][a-z0-9-]*)/i`: the alias and the length of the match.
fn about_alias(url: &str) -> Option<(&str, usize)> {
    let s = strip_prefix_ci(url, "about:")?;
    let b = s.as_bytes();
    if !b.first()?.is_ascii_alphabetic() {
        return None;
    }
    let len = b
        .iter()
        .position(|c| !(c.is_ascii_alphanumeric() || *c == b'-'))
        .unwrap_or(b.len());
    Some((&s[..len], "about:".len() + len))
}

/// Port of `toAppUrl` (`appUrls.ts:19`).
pub fn to_app_url(url: &str) -> String {
    if let Some(parts) = split(url, APP_SCHEME).or_else(|| split(url, "chrome")) {
        return if parts.host.is_empty() {
            url.to_string()
        } else {
            format!(
                "{}{APP_SCHEME}://{}{}",
                parts.view_source, parts.host, parts.rest
            )
        };
    }
    if is_about_page(url) {
        return url.to_string();
    }
    match about_alias(url) {
        Some((alias, len)) => format!("{APP_SCHEME}://{}{}", alias.to_lowercase(), &url[len..]),
        None => url.to_string(),
    }
}

/// Port of `toEngineUrl` (`appUrls.ts:27`).
pub fn to_engine_url(url: &str) -> String {
    match split(url, APP_SCHEME) {
        Some(parts) if !parts.host.is_empty() => {
            format!("{}chrome://{}{}", parts.view_source, parts.host, parts.rest)
        }
        _ => url.to_string(),
    }
}

/// `!!url && /^ardia:\/\/[^/?#]/i.test(url)`.
fn is_app_url(url: &str) -> bool {
    strip_prefix_ci(url, "ardia://")
        .is_some_and(|r| r.starts_with(|c| !matches!(c, '/' | '?' | '#')))
}

struct AppUrlParts {
    host: String,
    path: String,
}

/// The original throws this `TypeError` when the fragment holds a line terminator:
/// `/^([^?#]*)(\?[^#]*)?(#.*)?$/` does not match, and `m[1]` reads `null`.
pub const LINE_TERMINATOR_IN_HASH: &str = "Cannot read properties of null (reading '1')";

/// `appUrlParts` (`appUrls.ts:34`). `Ok(None)` for a URL that is not an app URL.
fn app_url_parts(url: &str) -> Result<Option<AppUrlParts>, &'static str> {
    if !is_app_url(url) {
        return Ok(None);
    }
    let Some(parts) = split(url, APP_SCHEME) else {
        return Ok(None);
    };
    let path_len = parts.rest.find(['?', '#']).unwrap_or(parts.rest.len());
    let after_path = &parts.rest[path_len..];
    let hash = match after_path.strip_prefix('?') {
        Some(query) => &query[query.find('#').unwrap_or(query.len())..],
        None => after_path,
    };
    if hash.contains(is_line_terminator) {
        return Err(LINE_TERMINATOR_IN_HASH);
    }
    Ok(Some(AppUrlParts {
        host: parts.host,
        path: parts.rest[..path_len].to_string(),
    }))
}

/// Port of `appUrlOrigin` (`appUrls.ts:41`).
pub fn app_url_origin(url: &str) -> Result<Option<String>, &'static str> {
    Ok(app_url_parts(&to_app_url(url))?.map(|p| format!("{APP_SCHEME}://{}", p.host)))
}

/// Port of `appUrlRoute` (`appUrls.ts:52`).
pub fn app_url_route(url: &str) -> Result<Option<AppUrlRoute>, &'static str> {
    let Some(parts) = app_url_parts(url)? else {
        return Ok(None);
    };
    Ok(Some(match parts.host.as_str() {
        "history" => AppUrlRoute::Page(AppPageHost::History),
        "bookmarks" => AppUrlRoute::Page(AppPageHost::Bookmarks),
        "downloads" => AppUrlRoute::Page(AppPageHost::Downloads),
        "newtab" => AppUrlRoute::NewTab,
        "settings" => AppUrlRoute::Settings {
            section: parts
                .path
                .split('/')
                .find(|s| !s.is_empty())
                .map(str::to_string),
        },
        _ => AppUrlRoute::Engine {
            url: to_engine_url(url),
        },
    }))
}

impl AppPageHost {
    pub fn as_str(self) -> &'static str {
        APP_PAGE_HOSTS[self as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_terminator_in_hash_is_an_error_like_the_original() {
        assert_eq!(app_url_origin("ardia://x#a\nb"), Err(LINE_TERMINATOR_IN_HASH));
        assert!(app_url_route("ardia://x?q#h\u{2028}i").is_err());
        assert_eq!(app_url_origin("ardia://x#ab").unwrap().as_deref(), Some("ardia://x"));
        assert_eq!(app_url_route("https://x#a\nb"), Ok(None));
    }
}
