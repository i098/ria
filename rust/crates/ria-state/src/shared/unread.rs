// Ported from ardia apps/browser/src/lib/unread.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Unread counts read from page titles ("(3) Inbox", "Inbox (3)", "4 unread", "• Discord") and
//! from badge-named favicons.
//!
//! Changes from Ardia:
//! - The patterns keep JavaScript regex rules by hand: `\s` is the JS set (with U+FEFF, without
//!   U+0085), `\d` is `[0-9]`, `\b` and the `i` flag are ASCII only, and `.` stops at the four JS line terminators.
//! - The `rust regex` crate has no lookahead: `LEADING_MARK` checks for the non-space after its match in code,
//!   and `BADGE_TOKEN` consumes the character its lookahead tests, which gives the same `test` result.
//! - `LABEL_COUNT` counts its label of 1 to 32 code units in code: JavaScript counts UTF-16 code units, the regex crate counts characters.
//! - `titleWithoutUnread` takes `Option<&Unread>`: an `unread` value of any other shape never matches, as in Ardia.
//! - `unreadCount` takes `more` as a `bool`; `toLowerCase` is `str::to_lowercase`.

use std::sync::LazyLock;

use regex::Regex;
use ria_core::identity::telemetry_wire::js_number_to_string;

use super::js::{is_js_space, js_number, js_trim};

/// `Unread` (`unread.ts:1`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unread {
    Count { label: String },
    Dot,
}

/// `UNREAD_DOT` (`unread.ts:3`).
pub const UNREAD_DOT: Unread = Unread::Dot;

/// The members of the JavaScript `\s` class, for use inside a character class.
const SPACE: &str = r"\t\n\x0B\x0C\r \x{A0}\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}\x{FEFF}";
/// `COUNT` (`unread.ts:5`).
const COUNT: &str = r"([0-9]{1,3}(?:,[0-9]{3})+|[0-9]{1,6})(\+?)";

fn pattern(text: &str) -> Regex {
    Regex::new(&text.replace("SP", SPACE).replace("COUNT", COUNT)).expect("static pattern")
}

/// `LEADING_COUNT` (`unread.ts:6`).
static LEADING_COUNT: LazyLock<Regex> = LazyLock::new(|| pattern(r"^[(\[]COUNT[)\]][SP]*[^SP]"));
/// `LABEL_COUNT` (`unread.ts:7`) without the 1 to 32 bound of the label, which `label_count` checks.
static LABEL_COUNT: LazyLock<Regex> = LazyLock::new(|| pattern(r"^([^0-9()\[\]]+)[SP]\(COUNT\)$"));
/// `PHRASE_COUNT` (`unread.ts:8`).
static PHRASE_COUNT: LazyLock<Regex> = LazyLock::new(|| {
    pattern(
        r"(?:^|[(\[SP])COUNT[SP]+(?i-u:unread|new (?:messages?|notifications?|e-?mails?|mails?|chats?))(?-u:\b)",
    )
});
/// `LEADING_BULLET` (`unread.ts:9`).
static LEADING_BULLET: LazyLock<Regex> = LazyLock::new(|| pattern(r"^•[SP]+[^SP•0-9]"));
/// `LEADING_STAR` (`unread.ts:10`).
static LEADING_STAR: LazyLock<Regex> = LazyLock::new(|| pattern(r"^\*[SP]+[^SP*0-9]"));
/// `SEGMENTS` (`unread.ts:11`).
static SEGMENTS: LazyLock<Regex> = LazyLock::new(|| pattern(r"[SP]+[-–—|·•:/][SP]+"));
/// `LEADING_MARK` (`unread.ts:49`) without its `(?=\S)` lookahead, which `title_without_unread` checks.
static LEADING_MARK: LazyLock<Regex> =
    LazyLock::new(|| pattern(r"^[SP]*(?:[(\[]COUNT[)\]][SP]*|•[SP]+)"));
/// The `^[a-z][a-z0-9+.-]*:\/\/` scheme of `hostOfOrigin` and `fileNameOf`, and the `originOf` pattern.
static SCHEME: LazyLock<Regex> = LazyLock::new(|| pattern(r"^[a-zA-Z][a-zA-Z0-9+.-]*://"));
static PORT: LazyLock<Regex> = LazyLock::new(|| pattern(r":[0-9]+$"));
static ORIGIN: LazyLock<Regex> = LazyLock::new(|| pattern(r"^[a-zA-Z][a-zA-Z0-9+.-]*://[^/?#]*"));
static QUERY: LazyLock<Regex> = LazyLock::new(|| pattern(r"[?#][^\n\r\x{2028}\x{2029}]*$"));
static SCHEME_HOST: LazyLock<Regex> =
    LazyLock::new(|| pattern(r"^[a-zA-Z][a-zA-Z0-9+.-]*://[^/]*"));
/// `BADGE_TOKEN` (`unread.ts:96`), with its lookahead consumed.
static BADGE_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    pattern(
        r"(?:^|[-_.])(?i-u:unread|badge|badged|notif|notification|notifications|alert|dot|count|mention|mentions)(?:[-_.0-9]|$)",
    )
});

/// `MAX_LABEL_WORDS` (`unread.ts:12`).
const MAX_LABEL_WORDS: usize = 3;
/// `UNREAD_LABEL_WORDS` (`unread.ts:13`).
const UNREAD_LABEL_WORDS: [&str; 16] = [
    "inbox",
    "mail",
    "email",
    "messages",
    "message",
    "chat",
    "chats",
    "notifications",
    "notification",
    "mentions",
    "activity",
    "conversations",
    "dms",
    "unread",
    "updates",
    "alerts",
];
/// `STAR_MEANS_UNREAD` (`unread.ts:31`).
const STAR_MEANS_UNREAD: [&str; 1] = ["slack.com"];

/// `looksLikeYear` (`unread.ts:33`).
fn looks_like_year(digits: &str) -> bool {
    digits.len() == 4 && (1900.0..=2100.0).contains(&js_number(digits))
}

/// `countOf` (`unread.ts:35`): `raw` digits (with thousands commas) and a `+` as a count.
fn count_of(raw: &str, plus: &str) -> Option<Unread> {
    let digits = raw.replace(',', "");
    if looks_like_year(&digits) {
        return None;
    }
    let n = js_number(&digits);
    if !n.is_finite() || n <= 0.0 {
        return None;
    }
    unread_count(n, !plus.is_empty())
}

/// `unreadCount` (`unread.ts:43`): a count badge, `99+` past 99, `None` below 1.
pub fn unread_count(n: f64, more: bool) -> Option<Unread> {
    if !n.is_finite() || n < 1.0 {
        return None;
    }
    let whole = n.floor();
    let label = if whole > 99.0 {
        "99+".to_string()
    } else if more {
        format!("{}+", js_number_to_string(whole))
    } else {
        js_number_to_string(whole)
    };
    Some(Unread::Count { label })
}

/// `titleWithoutUnread` (`unread.ts:51`): the title without its leading count or bullet when the badge shows the same.
pub fn title_without_unread(title: &str, unread: Option<&Unread>) -> String {
    let Some(unread) = unread else {
        return title.to_string();
    };
    let Some(mark) = LEADING_MARK.captures(title) else {
        return title.to_string();
    };
    let end = mark.get(0).map_or(0, |m| m.end());
    // `(?=\S)`: the mark must be followed by a character that is not a space.
    if title[end..].chars().next().is_none_or(is_js_space) {
        return title.to_string();
    }
    let said = match (mark.get(1), mark.get(2)) {
        (Some(count), Some(plus)) => count_of(count.as_str(), plus.as_str()),
        _ => Some(UNREAD_DOT),
    };
    if said.as_ref() == Some(unread) {
        title[end..].to_string()
    } else {
        title.to_string()
    }
}

/// `hostOfOrigin` (`unread.ts:60`): the lower-case host of an origin, without scheme or port.
pub fn host_of_origin(origin: &str) -> String {
    let rest = SCHEME.replace(origin, "");
    PORT.replace(&rest, "").to_lowercase()
}

/// `onSite` (`unread.ts:64`).
fn on_site(host: &str, sites: &[&str]) -> bool {
    sites
        .iter()
        .any(|site| host == *site || host.strip_suffix(site).is_some_and(|h| h.ends_with('.')))
}

/// `brandOf` (`unread.ts:66`): the label before the top-level domain.
fn brand_of(host: &str) -> &str {
    let labels: Vec<&str> = host.split('.').filter(|l| !l.is_empty()).collect();
    if labels.len() >= 2 {
        labels[labels.len() - 2]
    } else {
        labels.first().copied().unwrap_or("")
    }
}

/// `labelMeansUnread` (`unread.ts:71`).
fn label_means_unread(label: &str, host: &str) -> bool {
    let lower = js_trim(label).to_lowercase();
    let words: Vec<&str> = lower.split(is_js_space).filter(|w| !w.is_empty()).collect();
    if words.len() > MAX_LABEL_WORDS {
        return false;
    }
    let brand = brand_of(host);
    words.iter().any(|w| {
        UNREAD_LABEL_WORDS.contains(w) || (brand.encode_utf16().count() > 2 && *w == brand)
    })
}

/// `LABEL_COUNT.exec(text)` (`unread.ts:7`): label, digits and plus.
fn label_count(text: &str) -> Option<(&str, &str, &str)> {
    let m = LABEL_COUNT.captures(text)?;
    let (label, count, plus) = (m.get(1)?.as_str(), m.get(2)?.as_str(), m.get(3)?.as_str());
    (1..=32)
        .contains(&label.encode_utf16().count())
        .then_some((label, count, plus))
}

/// `unreadInTitle` (`unread.ts:78`): the unread count or dot that a page title shows, if any.
pub fn unread_in_title(raw: &str, host: &str) -> Option<Unread> {
    let title = js_trim(raw);
    if title.is_empty() {
        return None;
    }
    for re in [&*LEADING_COUNT, &*PHRASE_COUNT] {
        if let Some(m) = re.captures(title) {
            return count_of(&m[1], &m[2]);
        }
    }
    let first = SEGMENTS.find(title).map_or(title, |m| &title[..m.start()]);
    if let Some((label, count, plus)) = label_count(first)
        && label_means_unread(label, host)
    {
        return count_of(count, plus);
    }
    if LEADING_BULLET.is_match(title) {
        return Some(UNREAD_DOT);
    }
    (LEADING_STAR.is_match(title) && on_site(host, &STAR_MEANS_UNREAD)).then_some(UNREAD_DOT)
}

/// `originOf` (`unread.ts:91`): the lower-case `scheme://host:port` of a URL, or `""`.
pub fn origin_of(url: &str) -> String {
    ORIGIN
        .find(url)
        .map_or_else(String::new, |m| m.as_str().to_lowercase())
}

/// `fileNameOf` (`unread.ts:98`): the last path segment, without query or fragment.
fn file_name_of(url: &str) -> String {
    let no_query = QUERY.replace(url, "");
    let path = SCHEME_HOST.replace(&no_query, "");
    path[path.rfind('/').map_or(0, |i| i + 1)..].to_string()
}

/// `faviconIsBadgeNamed` (`unread.ts:103`).
pub fn favicon_is_badge_named(url: &str) -> bool {
    !url.starts_with("data:") && BADGE_TOKEN.is_match(&file_name_of(url))
}

/// `faviconIsOrdinary` (`unread.ts:105`).
pub fn favicon_is_ordinary(url: &str) -> bool {
    !url.is_empty() && !url.starts_with("data:") && !favicon_is_badge_named(url)
}

/// `faviconLooksBadged` (`unread.ts:107`).
pub fn favicon_looks_badged(home: &str, now: &str) -> bool {
    !home.is_empty() && now != home && favicon_is_badge_named(now)
}
