// Ported from ardia apps/browser/src/lib/url.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! URL text helpers for the toolbar and page labels: fragments, user info, hosts and the trail
//! after the host. Also the JavaScript `decodeURIComponent` and `decodeURI` that the sibling ports use.
//!
//! Changes from Ardia:
//! - `urlTrail` takes `hovered` and `show_full_url` as two `bool`s, not one object.
//! - `pageHostLabel` and `urlTrail` give `Err` with the `TypeError` message where `appUrlOrigin` throws.
//! - `urlTrail` slices `file://` URLs in UTF-16 units; a cut through a surrogate pair gives U+FFFD,
//!   because a Rust `String` cannot hold a lone surrogate.
//! - `originForSecurityDisplay` parses with the `url` crate (WHATWG) in place of `new URL`.

use regex::Regex;
use ria_core::omnibox::omnibox::{breadcrumb, host_of, url_for_display};
use ria_core::url::app_urls::app_url_origin;
use ria_core::url::idn::display_host;
use std::sync::LazyLock;

// `[a-z]` with the `i` flag and no `u` flag matches ASCII letters only.
static USER_INFO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([a-zA-Z][a-zA-Z0-9+.-]*://)[^@/?#]*@").expect("static pattern")
});

fn starts_with_ci(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

/// `withoutFragment` (`url.ts:3`): `replace(/#.*$/, "")`. `.` stops at line terminators, so the cut
/// is at the first `#` after the last line terminator.
pub fn without_fragment(url: &str) -> String {
    let start = url
        .rfind(['\n', '\r', '\u{2028}', '\u{2029}'])
        .map_or(0, |i| i + url[i..].chars().next().map_or(1, char::len_utf8));
    match url[start..].find('#') {
        Some(i) => url[..start + i].to_string(),
        None => url.to_string(),
    }
}

/// `withoutUserInfo` (`url.ts:5`): drops `user:pass@` after the scheme.
pub fn without_user_info(url: &str) -> String {
    USER_INFO.replace(url, "$1").into_owned()
}

/// `samePage` (`url.ts:7`): the same URL apart from the fragment and one trailing `/`.
pub fn same_page(a: &str, b: &str) -> bool {
    let key = |u: &str| {
        let s = without_fragment(u);
        s.strip_suffix('/').map(str::to_string).unwrap_or(s)
    };
    key(a) == key(b)
}

/// `isWebPageUrl` (`url.ts:9`): `http:`, `https:` or `file:`, in any case.
pub fn is_web_page_url(url: &str) -> bool {
    ["http:", "https:", "file:"]
        .iter()
        .any(|p| starts_with_ci(url, p))
}

/// `displayHostOf` (`url.ts:11`).
pub fn display_host_of(url: &str) -> String {
    display_host(&host_of(url))
}

/// `pageHostLabel` (`url.ts:13`): `"File"` for `file:` URLs, else the breadcrumb host.
pub fn page_host_label(url: &str) -> Result<String, &'static str> {
    if url.starts_with("file:") {
        return Ok("File".to_string());
    }
    Ok(breadcrumb(url, None)?.host)
}

/// `urlTrail` (`url.ts:15`): the text after the host in the address bar.
pub fn url_trail(
    url: &str,
    title: &str,
    hovered: bool,
    show_full_url: bool,
) -> Result<String, &'static str> {
    let is_file = url.starts_with("file:");
    let host = page_host_label(url)?;
    let full = if is_file {
        let rest: Vec<u16> = url.encode_utf16().skip("file://".len()).collect();
        format!(" {}", decoded_or(&String::from_utf16_lossy(&rest)))
    } else {
        url_for_display(url)?
    };
    let path = match full.strip_prefix(host.as_str()) {
        Some(rest) if !is_file => rest.to_string(),
        _ => full,
    };
    if hovered {
        return Ok(path);
    }
    if show_full_url {
        return Ok(path.strip_suffix('/').map(str::to_string).unwrap_or(path));
    }
    let app = !starts_with_ci(url, "http:")
        && !starts_with_ci(url, "https:")
        && app_url_origin(url)?.is_some_and(|o| !o.is_empty());
    Ok(if app && !title.is_empty() {
        format!(" / {title}")
    } else {
        String::new()
    })
}

/// `decodedOr` (`url.ts:25`).
pub fn decoded_or(s: &str) -> String {
    decode_uri_component(s).unwrap_or_else(|| s.to_string())
}

/// `hostOrOrigin` (`url.ts:33`).
pub fn host_or_origin(origin: &str) -> String {
    let host = host_of(origin);
    if host.is_empty() {
        origin.to_string()
    } else {
        host
    }
}

/// `originForSecurityDisplay` (`url.ts:35`): the host alone for `https:`, else `scheme://host`.
pub fn origin_for_security_display(origin: &str) -> String {
    let Ok(url) = ::url::Url::parse(origin) else {
        return origin.to_string();
    };
    let host = match (url.host_str().unwrap_or_default(), url.port()) {
        ("", _) => return origin.to_string(),
        (host, Some(port)) => format!("{host}:{port}"),
        (host, None) => host.to_string(),
    };
    if url.scheme() == "https" {
        host
    } else {
        format!("{}://{host}", url.scheme())
    }
}

/// JavaScript `decodeURIComponent`: `None` where it throws `URIError`.
pub fn decode_uri_component(text: &str) -> Option<String> {
    decode(text, "")
}

/// JavaScript `decodeURI`: like `decodeURIComponent`, but escapes of `;/?:@&=+$,#` stay as they are.
pub fn decode_uri(text: &str) -> Option<String> {
    decode(text, ";/?:@&=+$,#")
}

/// Decodes `%XX` escapes as UTF-8. An escape that is not two hex digits, or bytes that are not
/// UTF-8, make `None`: a literal character never completes an escaped sequence, so a check of
/// the whole buffer finds the same errors as the escape-by-escape check of the specification.
fn decode(text: &str, reserved: &str) -> Option<String> {
    let b = text.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'%' {
            out.push(b[i]);
            i += 1;
            continue;
        }
        let hex = b
            .get(i + 1..i + 3)
            .filter(|h| h.iter().all(u8::is_ascii_hexdigit))?;
        let byte = u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?;
        if byte < 0x80 && reserved.as_bytes().contains(&byte) {
            out.extend_from_slice(&b[i..i + 3]);
        } else {
            out.push(byte);
        }
        i += 3;
    }
    String::from_utf8(out).ok()
}
