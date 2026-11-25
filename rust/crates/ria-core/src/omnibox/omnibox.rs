// Ported from ardia packages/core/src/omnibox.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Omnibox input: URL fixup, spaced addresses, search URLs, paste handling, breadcrumbs and
//! display URLs. The regexes of the original are spelled out as string scans: without the `u`
//! flag their classes and case folding are ASCII, and `\s` is the JS whitespace set.
//!
//! Changes from Ardia: `fixup` and `spacedAddress` are split into helpers for the complexity
//! gate with unchanged behavior.

use std::collections::HashSet;
use std::sync::LazyLock;

use unicode_normalization::UnicodeNormalization;

use crate::fuzzy::is_js_space;
use crate::identity::telemetry_wire::js_number_to_string;
use crate::search::js_text::{encode_uri_component, js_trim};
use crate::url::app_urls::{app_url_origin, to_app_url};
use crate::url::clean_url::clean_url;
use crate::url::idn::{display_host, punycode_encode};
use crate::url::tlds::PSL_TLDS;
use crate::url::{is_line_terminator, strip_prefix_ci};

/// The groups of `URL_PARTS` (`omnibox.ts:8`); scheme and host are lower-cased.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUrl {
    pub scheme: String,
    pub host: String,
    pub port: String,
    pub path: String,
    pub query: String,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasteAction {
    Go { url: String },
    Search { query: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Breadcrumb {
    pub host: String,
    pub trail: Vec<String>,
}

const KNOWN_SCHEMES: [&str; 28] = [
    "http",
    "https",
    "file",
    "about",
    "data",
    "blob",
    "mailto",
    "tel",
    "sms",
    "facetime",
    "ftp",
    "view-source",
    "ardia",
    "chrome",
    "chrome-extension",
    "devtools",
    "x-apple.systempreferences",
    "itms-apps",
    "slack",
    "zoommtg",
    "vscode",
    "cursor",
    "notion",
    "figma",
    "spotify",
    "linear",
    "obsidian",
    "raycast",
];

const LOCAL_TLDS: [&str; 6] = ["meta", "local", "localhost", "test", "internal", "lan"];

static REGISTRIES: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| PSL_TLDS.split(is_js_space).collect());

const WIKI_SITES: [&str; 11] = [
    "wikipedia",
    "wiktionary",
    "wikimedia",
    "wikiquote",
    "wikibooks",
    "wikisource",
    "wikinews",
    "wikiversity",
    "wikivoyage",
    "wikidata",
    "mediawiki",
];

fn is_known_scheme(scheme: &str) -> bool {
    KNOWN_SCHEMES.contains(&scheme)
}

fn known_tld(tld: &str) -> bool {
    LOCAL_TLDS.contains(&tld) || REGISTRIES.contains(tld)
}

/// `/^([a-z][a-z0-9+.-]*):/i`: the scheme and the text after its colon.
pub(crate) fn split_scheme(text: &str) -> Option<(&str, &str)> {
    let colon = text.find(':')?;
    let scheme = &text[..colon];
    let mut bytes = scheme.bytes();
    let valid = bytes.next().is_some_and(|b| b.is_ascii_alphabetic())
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'.' | b'-'));
    valid.then(|| (scheme, &text[colon + 1..]))
}

/// What follows the host in `URL_PARTS`: `(?::(\d+))?([^?#]*)(\?[^#]*)?(#.*)?$`.
fn parse_tail(rest: &str) -> Option<(&str, &str, &str, &str)> {
    let digits = rest
        .strip_prefix(':')
        .map_or(0, |r| r.bytes().take_while(u8::is_ascii_digit).count());
    let (port, rest) = if digits > 0 {
        (&rest[1..=digits], &rest[digits + 1..])
    } else {
        ("", rest)
    };
    let query_at = rest.find(['?', '#']).unwrap_or(rest.len());
    let (path, rest) = rest.split_at(query_at);
    let hash_at = rest.find('#').unwrap_or(rest.len());
    let (query, hash) = rest.split_at(hash_at);
    (!hash.contains(is_line_terminator)).then_some((port, path, query, hash))
}

/// `parseUrl` (`omnibox.ts:10`).
pub fn parse_url(url: &str) -> Option<ParsedUrl> {
    let (scheme, after) = split_scheme(js_trim(url))?;
    let after = after.strip_prefix("//")?;
    let authority_end = after.find(['/', '?', '#']).unwrap_or(after.len());
    let userinfo_end = after[..authority_end].find('@').map(|i| i + 1);
    // The optional userinfo and the host alternatives backtrack in this order.
    for start in userinfo_end.into_iter().chain([0]) {
        let at = &after[start..];
        let bracket = at
            .strip_prefix('[')
            .and_then(|r| r.find(']'))
            .map(|close| close + 2);
        let plain = at.find([':', '/', '?', '#']).unwrap_or(at.len());
        for host_end in bracket.into_iter().chain([plain]) {
            if let Some((port, path, query, hash)) = parse_tail(&at[host_end..]) {
                return Some(ParsedUrl {
                    scheme: scheme.to_ascii_lowercase(),
                    host: at[..host_end].to_lowercase(),
                    port: port.to_string(),
                    path: path.to_string(),
                    query: query.to_string(),
                    hash: hash.to_string(),
                });
            }
        }
    }
    None
}

/// Port of `hostOf` (`omnibox.ts:16`).
pub fn host_of(url: &str) -> String {
    let host = parse_url(url).map(|p| p.host).unwrap_or_default();
    match host.strip_prefix("www.") {
        Some(rest) => rest.to_string(),
        None => host,
    }
}

/// `/^(\d{1,3}\.){3}\d{1,3}$/`.
fn is_ipv4(host: &str) -> bool {
    let parts: Vec<&str> = host.split('.').collect();
    parts.len() == 4
        && parts
            .iter()
            .all(|p| (1..=3).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_digit()))
}

/// `/^[\d.]+$/`.
fn is_numeric(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|b| b.is_ascii_digit() || b == b'.')
}

/// `isLocalHost` (`omnibox.ts:30`).
fn is_local_host(host: &str) -> bool {
    host == "localhost"
        || !host.contains('.')
        || host.starts_with('[')
        || is_ipv4(host)
        || [
            ".local",
            ".localhost",
            ".test",
            ".internal",
            ".lan",
            ".home.arpa",
        ]
        .iter()
        .any(|suffix| host.ends_with(suffix))
}

/// `isLabel` (`omnibox.ts:41`): `[a-z0-9\u00a1-\uffff-]+` with the `i` flag, and no dash at
/// either end. Astral characters are two code units of that range.
fn is_label(part: &str) -> bool {
    !part.is_empty()
        && part
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c >= '\u{a1}')
        && !part.starts_with('-')
        && !part.ends_with('-')
}

/// `isTld` (`omnibox.ts:42`).
fn is_tld(part: &str) -> bool {
    (is_label(part) && !part.starts_with(|c: char| c.is_ascii_digit()))
        || strip_prefix_ci(part, "xn--").is_some_and(|r| {
            !r.is_empty() && r.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

/// `isHostName` (`omnibox.ts:44`).
fn is_host_name(name: &str) -> bool {
    let ipv6 = name.len() > 2
        && name.starts_with('[')
        && name.ends_with(']')
        && name[1..name.len() - 1]
            .bytes()
            .all(|b| b.is_ascii_hexdigit() || b == b':' || b == b'.');
    if name.to_lowercase() == "localhost" || ipv6 || is_ipv4(name) {
        return true;
    }
    match name.rsplit_once('.') {
        None => is_label(name),
        Some((labels, tld)) => labels.split('.').all(is_label) && is_tld(tld),
    }
}

struct HostMatch<'a> {
    authority: &'a str,
    name: &'a str,
    port: &'a str,
}

/// `matchHost` (`omnibox.ts:52`).
fn match_host(input: &str) -> Option<HostMatch<'_>> {
    let authority = &input[..input.find(['/', '?', '#']).unwrap_or(input.len())];
    let colon = authority.rfind(':').filter(|&c| {
        let digits = &authority[c + 1..];
        (1..=5).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit())
    });
    let (bare, port) = match colon {
        Some(c) => authority.split_at(c),
        None => (authority, ""),
    };
    let name = bare.strip_suffix('.').unwrap_or(bare);
    is_host_name(name).then_some(HostMatch {
        authority,
        name,
        port,
    })
}

/// Port of `isJavaScriptUrl` (`omnibox.ts:65`).
pub fn is_javascript_url(url: &str) -> bool {
    let text: String = url
        .trim_start_matches(|c| c <= ' ')
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let text = strip_prefix_ci(&text, "view-source:").unwrap_or(&text);
    strip_prefix_ci(text, "javascript:").is_some()
}

/// Port of `withoutJavaScriptScheme` (`omnibox.ts:67`).
pub fn without_javascript_scheme(text: &str) -> &str {
    let mut out = text;
    while let Some(rest) = strip_prefix_ci(
        out.trim_start_matches(|c| c <= ' ' || c == '\u{7f}'),
        "javascript:",
    ) {
        out = rest;
    }
    out
}

/// Port of `fixupUrl` (`omnibox.ts:73`).
pub fn fixup_url(raw: &str) -> Option<String> {
    fixup(raw)
        .filter(|url| !url.is_empty() && !is_javascript_url(url))
        .map(|url| to_app_url(&url))
}

struct SpacedAddress {
    escaped: String,
    certain: bool,
}

/// `WIKI_HOST` (`omnibox.ts:80`).
fn is_wiki_host(name: &str) -> bool {
    name.strip_suffix(".org").is_some_and(|site| {
        WIKI_SITES.iter().any(|wiki| {
            site.strip_suffix(wiki)
                .is_some_and(|before| before.is_empty() || before.ends_with('.'))
        })
    })
}

/// `/^[^\s?#]*\?(?:[^\s#]*&)?[^\s#&=]+=/`: the query holds a `name=` pair.
fn has_query_param(rest: &str) -> bool {
    let Some(at) = rest.find(|c| is_js_space(c) || c == '?' || c == '#') else {
        return false;
    };
    let Some(query) = rest[at..].strip_prefix('?') else {
        return false;
    };
    let end = query
        .find(|c| is_js_space(c) || c == '#')
        .unwrap_or(query.len());
    query[..end]
        .split('&')
        .any(|pair| pair.find('=').is_some_and(|eq| eq > 0))
}

/// `spacedAddress` (`omnibox.ts:82`).
fn spaced_address(input: &str) -> Option<SpacedAddress> {
    if !input.contains(' ') || input.chars().any(|c| c != ' ' && is_js_space(c)) {
        return None;
    }
    let end = input.find(['/', '?', '#'])?;
    if end == 0 || input[..end].contains(' ') || input[end..].starts_with("/ ") {
        return None;
    }
    let host = match_host(input)?;
    let name = host.name.to_lowercase();
    if is_known_scheme(&name) || name == "javascript" {
        return None;
    }
    let local = spaced_local_host(&name, host.port)?;
    let rest = &input[host.authority.len()..];
    let wiki = is_wiki_host(&name)
        && rest
            .strip_prefix("/wiki/")
            .and_then(|r| r.chars().next())
            .is_some_and(|c| !is_js_space(c) && !matches!(c, '/' | '?' | '#'));
    Some(SpacedAddress {
        escaped: input.replace(' ', "%20"),
        certain: local || wiki || has_query_param(rest),
    })
}

/// The port and host checks of `spacedAddress`: `None` rejects the address, else whether the
/// host is local.
fn spaced_local_host(name: &str, port_text: &str) -> Option<bool> {
    let has_port = !port_text.is_empty();
    let port: u32 = port_text.get(1..).and_then(|d| d.parse().ok()).unwrap_or(0);
    if has_port && !(1..=65535).contains(&port) {
        return None;
    }
    let dotted = name.contains('.') && !is_numeric(name);
    let ip = name.starts_with('[') || is_ipv4(name);
    let local = name == "localhost"
        || ip
        || (has_port && !(dotted && !is_local_host(name) && (port == 80 || port == 443)));
    if !dotted && !local {
        return None;
    }
    Some(local)
}

/// `asciiLabel` (`omnibox.ts:102`).
fn ascii_label(label: &str) -> String {
    if label.is_ascii() {
        label.to_ascii_lowercase()
    } else {
        let folded: String = label.nfkc().flat_map(char::to_lowercase).collect();
        format!("xn--{}", punycode_encode(&folded))
    }
}

/// `addressKey` (`omnibox.ts:105`).
fn address_key(url: &str) -> String {
    let text = js_trim(url);
    let Some((scheme, after)) = split_scheme(text) else {
        return text.to_string();
    };
    let Some(after) = after.strip_prefix("//") else {
        return text.to_string();
    };
    let (authority, rest) = after.split_at(after.find(['/', '?', '#']).unwrap_or(after.len()));
    if rest.contains(is_line_terminator) {
        return text.to_string();
    }
    let scheme = scheme.to_ascii_lowercase();
    let web = scheme == "http" || scheme == "https";
    let default_port = match scheme.as_str() {
        "https" => 443.0,
        "http" => 80.0,
        _ => -1.0,
    };
    let host_end = authority
        .find([':', '@', '[', ']'])
        .unwrap_or(authority.len());
    let mut key: String = authority[..host_end]
        .split('.')
        .map(ascii_label)
        .collect::<Vec<_>>()
        .join(".");
    key.push_str(&authority[host_end..]);
    key.make_ascii_lowercase();
    if web
        && let Some(colon) = key.rfind(':')
        && colon + 1 < key.len()
        && key[colon + 1..].bytes().all(|b| b.is_ascii_digit())
    {
        let port: f64 = key[colon + 1..].parse().unwrap_or(f64::INFINITY);
        key.truncate(colon);
        if port != default_port {
            key.push(':');
            key.push_str(&js_number_to_string(port));
        }
    }
    let slash = if web && !rest.starts_with('/') {
        "/"
    } else {
        ""
    };
    format!("{scheme}://{key}{slash}{rest}")
}

/// Port of `sameAddress` (`omnibox.ts:116`).
pub fn same_address(a: &str, b: &str) -> bool {
    address_key(a) == address_key(b)
}

/// Port of `spacedAddressGuess` (`omnibox.ts:118`).
pub fn spaced_address_guess(raw: &str) -> Option<String> {
    let spaced = spaced_address(js_trim(raw))?;
    if spaced.certain {
        None
    } else {
        fixup_url(&spaced.escaped)
    }
}

/// `text.replace(/\s/g, "%20")`.
fn escape_spaces(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if is_js_space(c) {
            out.push_str("%20");
        } else {
            out.push(c);
        }
    }
    out
}

/// `fixup` (`omnibox.ts:123`).
fn fixup(raw: &str) -> Option<String> {
    let trimmed = js_trim(raw);
    let spaced = spaced_address(trimmed);
    let input = match &spaced {
        Some(s) if s.certain => s.escaped.as_str(),
        _ => trimmed,
    };
    let scheme = split_scheme(input);
    let address_start = scheme.is_some_and(|(_, after)| after.starts_with("//"))
        || input
            .strip_prefix('/')
            .and_then(|r| r.chars().next())
            .is_some_and(|c| c != '/' && !is_js_space(c));
    if input.is_empty() || (input.contains(is_js_space) && !address_start) {
        return None;
    }
    if let Some(url) = scheme_fixup(input, scheme) {
        return Some(url);
    }
    host_fixup(input)
}

/// The `fixup` steps for a scheme or a file path; `None` when none of them applies.
fn scheme_fixup(input: &str, scheme: Option<(&str, &str)>) -> Option<String> {
    if [
        "about",
        "data",
        "mailto",
        "tel",
        "sms",
        "facetime",
        "view-source",
    ]
    .iter()
    .any(|s| strip_prefix_ci(input, s).is_some_and(|r| r.starts_with(':')))
    {
        return Some(input.to_string());
    }
    if let Some((scheme, after)) = scheme {
        let scheme = scheme.to_ascii_lowercase();
        let known = is_known_scheme(&scheme);
        if known && (after.starts_with("//") || !after.starts_with(|c: char| c.is_ascii_digit())) {
            return Some(format!("{scheme}{}", escape_spaces(&input[scheme.len()..])));
        }
        if !known && after.starts_with("//") {
            return Some(input.to_string());
        }
    }
    if input.starts_with('/') && !input.starts_with("//") {
        return Some(format!("file://{}", escape_spaces(input)));
    }
    None
}

/// The `fixup` steps that read the host of an input without a scheme.
fn host_fixup(input: &str) -> Option<String> {
    if !input.contains(['.', ':', '/', '?', '#', '[']) {
        return (input.to_lowercase() == "localhost").then(|| format!("http://{input}"));
    }
    let host = match_host(input)?;
    let name = host.name.to_lowercase();
    let numeric = is_numeric(&name);
    if numeric && !name.contains('.') {
        return None;
    }
    let rest = &input[host.authority.len()..];
    let proto = if is_local_host(&name) {
        "http"
    } else {
        "https"
    };
    if is_bare_host(&name, numeric) {
        return Some(format!("{proto}://{input}"));
    }
    port_or_path_fixup(input, rest, !host.port.is_empty(), proto)
}

/// Whether `fixup` takes the host `name` alone as an address: localhost, an IP, or a name with
/// a known TLD.
fn is_bare_host(name: &str, numeric: bool) -> bool {
    let dotted = name.contains('.') && !numeric && !name.starts_with('[');
    let tld = match name.rfind('.') {
        Some(dot) if dotted => &name[dot + 1..],
        _ => "",
    };
    let special = name == "localhost" || name.starts_with('[') || numeric;
    special || (dotted && (known_tld(tld) || tld.starts_with("xn--")))
}

/// The last `fixup` step: a host with an unknown TLD is an address only with a port, a path
/// ending in `/`, or two of port, path, query and fragment.
fn port_or_path_fixup(input: &str, rest: &str, has_port: bool, proto: &str) -> Option<String> {
    if !has_port && !rest.contains(['/', '?', '#']) {
        return None;
    }
    let path = &rest[..rest.find(['?', '#']).unwrap_or(rest.len())];
    // `query.length > 1` and `fragment.length > 1`: a character follows the `?` or the `#`.
    let query = rest
        .find('?')
        .is_some_and(|q| rest[q + 1..].starts_with(|c| c != '#'));
    let fragment = rest
        .find('#')
        .is_some_and(|h| rest[h + 1..].starts_with(|c| !is_line_terminator(c)));
    let parts = [has_port, !path.is_empty(), query, fragment]
        .iter()
        .filter(|&&p| p)
        .count();
    (has_port || path.ends_with('/') || parts > 1).then(|| format!("{proto}://{input}"))
}

/// Port of `searchUrlFor` (`omnibox.ts:161`). Rust strings hold no lone surrogates, so the
/// `wellFormed` step of the original has nothing to replace.
pub fn search_url_for(template: &str, query: &str) -> String {
    let q = encode_uri_component(js_trim(query));
    if template.contains("%s") {
        template.replace("%s", &q)
    } else {
        format!("{template}{q}")
    }
}

/// Port of `resolveInput` (`omnibox.ts:166`).
pub fn resolve_input(raw: &str, search: &str) -> String {
    let input = js_trim(raw);
    if input.is_empty() {
        return String::new();
    }
    if let Some(query) = input.strip_prefix('?') {
        return if query.is_empty() {
            String::new()
        } else {
            search_url_for(search, query)
        };
    }
    fixup_url(input).unwrap_or_else(|| search_url_for(search, input))
}

/// `text.replace(/\s+/g, " ").slice(0, max)`, with `max` in UTF-16 units. A surrogate pair that
/// the slice cuts becomes U+FFFD: the original keeps a lone high surrogate.
fn collapse_spaces(text: &str, max: usize) -> String {
    let mut out = String::new();
    let mut units = 0;
    let mut in_space = false;
    for c in text.chars() {
        let space = is_js_space(c);
        if space && in_space {
            continue;
        }
        in_space = space;
        let c = if space { ' ' } else { c };
        units += c.len_utf16();
        if units > max {
            if units == max + 1 && c.len_utf16() == 2 {
                out.push('\u{fffd}');
            }
            break;
        }
        out.push(c);
    }
    out
}

/// Port of `classifyPaste` (`omnibox.ts:175`).
pub fn classify_paste(text: &str) -> Option<PasteAction> {
    let trimmed = js_trim(without_javascript_scheme(js_trim(text)));
    if trimmed.is_empty() {
        return None;
    }
    let url =
        if trimmed.contains(is_js_space) && !spaced_address(trimmed).is_some_and(|s| s.certain) {
            None
        } else {
            fixup_url(trimmed)
        };
    if let Some(url) = url
        && strip_prefix_ci(&url, "about:").is_none()
        && strip_prefix_ci(&url, "data:").is_none()
    {
        return Some(PasteAction::Go {
            url: clean_url(&url),
        });
    }
    Some(PasteAction::Search {
        query: collapse_spaces(trimmed, 2000),
    })
}

/// Port of `sanitizePaste` (`omnibox.ts:183`).
pub fn sanitize_paste(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let trimmed = js_trim(text);
    if trimmed.is_empty() {
        return " ".to_string();
    }
    // `split(/(\s+)/)`: the words and the whitespace runs between them.
    let mut runs: Vec<(&str, bool)> = Vec::new();
    let (mut start, mut space) = (0, false);
    for (i, c) in trimmed.char_indices() {
        if is_js_space(c) != space {
            runs.push((&trimmed[start..i], space));
            (start, space) = (i, !space);
        }
    }
    runs.push((&trimmed[start..], space));
    let breaks = |run: &str| run.contains(['\r', '\n']);
    let spaced = runs.iter().any(|&(run, space)| space && !breaks(run));
    let joined: String = runs
        .iter()
        .map(|&(run, space)| match space && breaks(run) {
            false => run,
            true if spaced => " ",
            true => "",
        })
        .collect();
    without_javascript_scheme(&joined)
        .trim_start_matches(is_js_space)
        .to_string()
}

/// The default port of `DEFAULT_PORTS` (`omnibox.ts:191`).
fn default_port(scheme: &str) -> Option<&'static str> {
    match scheme {
        "http" | "ws" => Some("80"),
        "https" | "wss" => Some("443"),
        "ftp" => Some("21"),
        _ => None,
    }
}

/// `title.split(/\s[|/·–—-]\s/)`.
fn split_title(title: &str) -> Vec<&str> {
    let chars: Vec<(usize, char)> = title.char_indices().collect();
    let mut parts = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i + 2 < chars.len() {
        if is_js_space(chars[i].1)
            && matches!(chars[i + 1].1, '|' | '/' | '·' | '–' | '—' | '-')
            && is_js_space(chars[i + 2].1)
        {
            parts.push(&title[start..chars[i].0]);
            start = chars.get(i + 3).map_or(title.len(), |&(at, _)| at);
            i += 3;
        } else {
            i += 1;
        }
    }
    parts.push(&title[start..]);
    parts
}

/// Port of `breadcrumb` (`omnibox.ts:193`). `Err` is the `TypeError` of `appUrlOrigin`.
pub fn breadcrumb(url: &str, title: Option<&str>) -> Result<Breadcrumb, &'static str> {
    let parsed = parse_url(url);
    let app = app_url_origin(url)?;
    let host = match (&app, &parsed) {
        (Some(app), _) => app.clone(),
        (None, Some(p)) => {
            let shown = display_host(&p.host);
            let port = if !p.port.is_empty() && default_port(&p.scheme) != Some(p.port.as_str()) {
                format!(":{}", p.port)
            } else {
                String::new()
            };
            format!("{}{port}", shown.strip_prefix("www.").unwrap_or(&shown))
        }
        (None, None) => url.to_string(),
    };
    let mut seen = HashSet::from([host.to_lowercase()]);
    if app.is_some()
        && let Some(p) = &parsed
    {
        seen.insert(p.host.clone());
    }
    let trail = split_title(title.unwrap_or(""))
        .into_iter()
        .map(js_trim)
        .filter(|s| !s.is_empty() && seen.insert(s.to_lowercase()))
        .map(str::to_string)
        .collect();
    Ok(Breadcrumb { host, trail })
}

/// `replace(/^([^/]*\/\/[^/?#]*)\/(?=[?#]|$)/, "$1")`: drop the root slash after the authority.
fn without_root_slash(url: String) -> String {
    let Some(first) = url.find('/') else {
        return url;
    };
    if !url[first..].starts_with("//") {
        return url;
    }
    let start = first + 2;
    let end = start
        + url[start..]
            .find(['/', '?', '#'])
            .unwrap_or(url.len() - start);
    let after = &url[end..];
    match after.strip_prefix('/') {
        Some(tail) if tail.is_empty() || tail.starts_with(['?', '#']) => {
            format!("{}{tail}", &url[..end])
        }
        _ => url,
    }
}

/// Port of `urlForDisplay` (`omnibox.ts:211`). `Err` is the `TypeError` of `appUrlOrigin`.
pub fn url_for_display(url: &str) -> Result<String, &'static str> {
    if app_url_origin(url)?.is_some() {
        return Ok(without_root_slash(to_app_url(url)));
    }
    let rest = match split_scheme(url).and_then(|(_, after)| after.strip_prefix("//")) {
        Some(after) => {
            let end = after.find(['/', '?', '#']).unwrap_or(after.len());
            after[..end].rfind('@').map_or(after, |at| &after[at + 1..])
        }
        None => url,
    };
    let host_len = rest.find(['/', '?', '#', ':']).unwrap_or(rest.len());
    let shown = format!("{}{}", display_host(&rest[..host_len]), &rest[host_len..]);
    Ok(strip_prefix_ci(&shown, "www.")
        .unwrap_or(&shown)
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_url_backtracks_past_userinfo() {
        assert_eq!(
            host_of("https://user@WWW.Example.com:8080/p?q#h"),
            "example.com"
        );
        assert_eq!(host_of("http://a#\nb"), "");
        let p = parse_url("http://[::1]:80/x?y#z").unwrap_or_else(|| panic!("no match"));
        assert_eq!((p.host.as_str(), p.port.as_str()), ("[::1]", "80"));
    }

    #[test]
    fn query_param_needs_a_name() {
        assert!(has_query_param("/a?x=1"));
        assert!(has_query_param("?=&b=2"));
        assert!(!has_query_param("/a?=1"));
        assert!(!has_query_param("/a b?x=1"));
    }

    #[test]
    fn same_address_normalizes_ports() {
        assert!(same_address(
            "HTTPS://Example.com:0443",
            "https://example.com/"
        ));
        assert!(!same_address("ftp://a:21", "ftp://a"));
    }

    #[test]
    fn paste_slice_counts_utf16_units() {
        let text = format!("{}😀", "a".repeat(1999));
        assert_eq!(
            collapse_spaces(&text, 2000).chars().last(),
            Some('\u{fffd}')
        );
        assert_eq!(sanitize_paste("a\r\nb"), "ab");
        assert_eq!(sanitize_paste("a b\nc"), "a b c");
        assert_eq!(sanitize_paste(" \n"), " ");
    }

    #[test]
    fn title_splits_on_spaced_separators() {
        assert_eq!(split_title("a - - b"), vec!["a", "- b"]);
    }
}
