// Ported from ardia packages/core/src/omnibox.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Search-local copies of `hostOf` and `searchUrlFor`.
//! For the full port, see [`crate::omnibox::omnibox`].

use super::js_text::{encode_uri_component, is_line_terminator, js_trim};

/// What follows the host matches `(?::\d+)?[^?#]*(\?[^#]*)?(#.*)?$`: it fails only when the
/// fragment holds a line terminator, which `.` does not match.
fn tail_matches(rest: &str) -> bool {
    match rest.find('#') {
        Some(i) => !rest[i..].chars().any(is_line_terminator),
        None => true,
    }
}

/// The host group of `URL_PARTS` in `omnibox.ts:6`, lower-cased; `None` when the URL does not match.
fn parse_host(url: &str) -> Option<String> {
    let text = js_trim(url);
    let (scheme, after) = text.split_once("://")?;
    let mut chars = scheme.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic()
        || !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
    {
        return None;
    }
    // `(?:[^@/?#]*@)?` is optional, so the regex retries without it when the rest fails.
    let authority_end = after.find(['/', '?', '#']).unwrap_or(after.len());
    let userinfo_end = after[..authority_end].find('@').map(|i| i + 1);
    for start in userinfo_end.into_iter().chain(std::iter::once(0)) {
        let at = &after[start..];
        if at.starts_with('[')
            && let Some(close) = at.find(']')
            && tail_matches(&at[close + 1..])
        {
            return Some(at[..=close].to_lowercase());
        }
        let end = at.find([':', '/', '?', '#']).unwrap_or(at.len());
        if tail_matches(&at[end..]) {
            return Some(at[..end].to_lowercase());
        }
    }
    None
}

/// Port of `hostOf` (`omnibox.ts:16`).
pub fn host_of(url: &str) -> String {
    let host = parse_host(url).unwrap_or_default();
    host.strip_prefix("www.")
        .map(str::to_string)
        .unwrap_or(host)
}

/// Port of `searchUrlFor` (`omnibox.ts:161`). The query is trimmed and percent-encoded; a template
/// without `%s` gets the query appended.
pub fn search_url_for(template: &str, query: &str) -> String {
    let q = encode_uri_component(js_trim(query));
    if template.contains("%s") {
        template.replace("%s", &q)
    } else {
        format!("{template}{q}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_of_strips_www_and_userinfo() {
        assert_eq!(
            host_of("https://user@WWW.Example.com:8080/p?q#h"),
            "example.com"
        );
        assert_eq!(host_of("nope"), "");
        assert_eq!(host_of("http://a#\nb"), "");
    }

    #[test]
    fn search_url_appends_or_fills() {
        assert_eq!(
            search_url_for("https://x/?q=%s&r=%s", " a b "),
            "https://x/?q=a%20b&r=a%20b"
        );
        assert_eq!(search_url_for("https://x/", "é"), "https://x/%C3%A9");
    }
}
