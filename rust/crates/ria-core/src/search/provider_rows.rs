// Ported from ardia packages/core/src/providerRows.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Omnibox provider rows: the checks that keep untrusted provider output safe to show, the
//! OpenSearch reply parser, static row matching and the global provider section.

use std::collections::HashSet;

use super::js_text::{encode_uri_component, is_js_space, is_line_terminator, js_trim};

/// A JS value of an input that the TS code types as `unknown`: what JSON carries, plus
/// `undefined`. Object keys keep their JS order. Rust strings hold no lone surrogates.
#[derive(Clone, Debug, PartialEq)]
pub enum JsValue {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsValue>),
    Object(Vec<(String, JsValue)>),
}

const UNDEFINED: JsValue = JsValue::Undefined;

impl JsValue {
    /// `value[key]` for a JSON-shaped value: `undefined` when the key is missing or the value is
    /// not an object.
    pub fn get(&self, key: &str) -> &JsValue {
        match self {
            JsValue::Object(fields) => fields
                .iter()
                .find(|(k, _)| k == key)
                .map_or(&UNDEFINED, |(_, v)| v),
            _ => &UNDEFINED,
        }
    }

    /// The string, when the value is one (`typeof value === "string"`).
    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// JS truthiness (`!!value`).
    pub fn is_truthy(&self) -> bool {
        match self {
            JsValue::Undefined | JsValue::Null => false,
            JsValue::Bool(b) => *b,
            JsValue::Number(n) => *n != 0.0 && !n.is_nan(),
            JsValue::String(s) => !s.is_empty(),
            JsValue::Array(_) | JsValue::Object(_) => true,
        }
    }

    /// `!!value && typeof value === "object" && !Array.isArray(value)`.
    pub fn is_plain_object(&self) -> bool {
        matches!(self, JsValue::Object(_))
    }
}

/// `PROVIDER_LIMITS` (`providerRows.ts:10`). Lengths count UTF-16 units, as JS `.length` does.
pub struct ProviderLimits {
    pub title: usize,
    pub subtitle: usize,
    pub url: usize,
    pub icon_bytes: usize,
    pub rows_per_provider: usize,
    pub rows_per_global_section: usize,
    pub global_rows: usize,
    pub keyword: usize,
    pub provider_title: usize,
}

/// `PROVIDER_LIMITS` (`providerRows.ts:10`).
pub const PROVIDER_LIMITS: ProviderLimits = ProviderLimits {
    title: 120,
    subtitle: 160,
    url: 4096,
    icon_bytes: 16_384,
    rows_per_provider: 20,
    rows_per_global_section: 3,
    global_rows: 6,
    keyword: 32,
    provider_title: 60,
};

/// `ProviderRowInput` (`providerRows.ts:4`): a row as a provider sends it; `badge` is never read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProviderRowInput {
    pub title: String,
    pub url: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
}

/// `ProviderSection` (`providerRows.ts:6`). `rows` stays a [`JsValue`]: the code checks each row
/// again, so the type does not promise their shape.
#[derive(Clone, Debug, PartialEq)]
pub struct ProviderSection {
    pub id: String,
    pub title: String,
    pub rows: JsValue,
}

/// The `kind: "provider"` variant of `Suggestion` (`suggestion.ts`), defined here until the
/// slice that ports `suggestion.ts` lands. `kind` is implied.
#[derive(Clone, Debug, PartialEq)]
pub struct ProviderSuggestion {
    pub provider_id: String,
    pub provider: String,
    pub title: String,
    pub url: String,
    pub icon: Option<String>,
    pub subtitle: Option<String>,
}

/// What `globalProviderRows` reads of an already shown `Suggestion`: its `kind` and, when it has
/// one, its `url`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShownSuggestion<'a> {
    pub kind: &'a str,
    pub url: Option<&'a str>,
}

/// The JS `.length` of `text`.
pub(crate) fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// `text.slice(0, max)` in UTF-16 units; a surrogate pair cut in half is dropped, since a Rust
/// string cannot hold its first half.
fn utf16_prefix(text: &str, max: usize) -> &str {
    let mut units = 0;
    for (at, c) in text.char_indices() {
        units += c.len_utf16();
        if units > max {
            return &text[..at];
        }
    }
    text
}

/// `text` without the ASCII-case-insensitive `prefix` (a regex `^prefix` with the `i` flag).
pub(crate) fn strip_prefix_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &text[prefix.len()..])
}

/// The `HIDDEN` class (`providerRows.ts:22`): controls, soft hyphen, bidi and zero-width marks.
fn is_hidden(c: char) -> bool {
    matches!(c,
        '\u{0}'..='\u{1f}' | '\u{7f}'..='\u{9f}' | '\u{ad}' | '\u{61c}' | '\u{200b}'..='\u{200f}'
        | '\u{2028}'..='\u{202e}' | '\u{2060}'..='\u{206f}' | '\u{feff}' | '\u{fff9}'..='\u{fffb}')
}

/// `pageKey` (`pageText.ts:8`): `url.replace(/#.*$/, "").replace(/\/$/, "")`. `.` stops at line
/// terminators, so the fragment cut starts at the first `#` with none after it.
pub(crate) fn page_key(url: &str) -> &str {
    let mut cut = url.len();
    for (at, _) in url.match_indices('#') {
        if !url[at..].chars().any(is_line_terminator) {
            cut = at;
            break;
        }
    }
    let url = &url[..cut];
    url.strip_suffix('/').unwrap_or(url)
}

/// `Array.prototype.slice` end index for `end` over `len` items.
fn slice_end(end: f64, len: usize) -> usize {
    let len_f = len as f64;
    let end = if end.is_nan() { 0.0 } else { end.trunc() };
    if end < 0.0 {
        (len_f + end).max(0.0) as usize
    } else {
        end.min(len_f) as usize
    }
}

/// Port of `providerText` (`providerRows.ts:29`): hidden characters and runs of white space
/// become one space; text over `max` UTF-16 units keeps `max - 1` code points and an ellipsis.
/// `None` is a non-string value.
pub fn provider_text(value: Option<&str>, max: f64) -> String {
    let Some(value) = value else {
        return String::new();
    };
    let mut collapsed = String::with_capacity(value.len());
    let mut in_space = false;
    for c in value.chars() {
        if is_hidden(c) || is_js_space(c) {
            if !in_space {
                collapsed.push(' ');
            }
            in_space = true;
        } else {
            collapsed.push(c);
            in_space = false;
        }
    }
    let text = js_trim(&collapsed);
    if (utf16_len(text) as f64) <= max {
        return text.to_string();
    }
    let keep = slice_end(max - 1.0, text.chars().count());
    let cut = text
        .char_indices()
        .nth(keep)
        .map_or(text.len(), |(at, _)| at);
    format!("{}…", text[..cut].trim_end_matches(is_js_space))
}

/// `AUTHORITY` (`providerRows.ts:25`): `^(?:\[[0-9a-f:.]+\]|[^:@\[\]]+)(?::\d{1,5})?$/i`.
fn is_web_authority(authority: &str) -> bool {
    let port = if let Some(inner) = authority.strip_prefix('[') {
        let Some(close) = inner.find(']') else {
            return false;
        };
        let host = &inner[..close];
        if host.is_empty()
            || !host
                .bytes()
                .all(|b| b.is_ascii_hexdigit() || b == b':' || b == b'.')
        {
            return false;
        }
        &inner[close + 1..]
    } else {
        let end = authority.find(':').unwrap_or(authority.len());
        let host = &authority[..end];
        if host.is_empty() || host.contains(['@', '[', ']']) {
            return false;
        }
        &authority[end..]
    };
    port.is_empty()
        || port
            .strip_prefix(':')
            .is_some_and(|d| (1..=5).contains(&d.len()) && d.bytes().all(|b| b.is_ascii_digit()))
}

/// Port of `providerUrl` (`providerRows.ts:36`): the trimmed URL when it is http(s) with a plain
/// authority and holds no white space or hidden character. `None` in is a non-string value.
pub fn provider_url(value: Option<&str>) -> Option<&str> {
    let url = js_trim(value?);
    if url.is_empty()
        || utf16_len(url) > PROVIDER_LIMITS.url
        || url.chars().any(|c| is_js_space(c) || is_hidden(c))
    {
        return None;
    }
    let rest = strip_prefix_ci(url, "https://").or_else(|| strip_prefix_ci(url, "http://"))?;
    let authority = &rest[..rest.find(['/', '?', '#']).unwrap_or(rest.len())];
    (!authority.is_empty() && is_web_authority(authority)).then_some(url)
}

/// `IMAGE_DATA_URL` (`providerRows.ts:26`).
fn is_image_data_url(value: &str) -> bool {
    const TYPES: [&str; 6] = ["png", "jpeg", "gif", "webp", "x-icon", "vnd.microsoft.icon"];
    let Some(rest) = strip_prefix_ci(value, "data:image/") else {
        return false;
    };
    TYPES.iter().any(|t| {
        strip_prefix_ci(rest, t)
            .and_then(|r| strip_prefix_ci(r, ";base64,"))
            .is_some_and(|body| {
                let data = body.trim_end_matches('=');
                body.len() - data.len() <= 2
                    && !data.is_empty()
                    && data
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/')
            })
    })
}

/// Port of `providerIcon` (`providerRows.ts:44`): a small base64 `data:image` or an http(s) URL.
pub fn provider_icon(value: Option<&str>) -> Option<&str> {
    let value = value.filter(|v| !v.is_empty())?;
    if strip_prefix_ci(value, "data:").is_some() {
        return (utf16_len(value) <= PROVIDER_LIMITS.icon_bytes && is_image_data_url(value))
            .then_some(value);
    }
    if strip_prefix_ci(value, "http://").is_some() || strip_prefix_ci(value, "https://").is_some() {
        provider_url(Some(value))
    } else {
        None
    }
}

/// Port of `safeProviderRows` (`providerRows.ts:50`): at most `limit` rows with a safe URL, one
/// per page, with capped text. `rows` that is not an array gives no rows.
pub fn safe_provider_rows(
    id: &str,
    title: &str,
    rows: &JsValue,
    limit: f64,
) -> Vec<ProviderSuggestion> {
    let JsValue::Array(rows) = rows else {
        return vec![];
    };
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let provider = provider_text(Some(title), PROVIDER_LIMITS.provider_title as f64);
    for row in rows {
        if out.len() as f64 >= limit {
            break;
        }
        if !row.is_truthy() || !matches!(row, JsValue::Array(_) | JsValue::Object(_)) {
            continue;
        }
        let Some(url) = provider_url(row.get("url").as_str()) else {
            continue;
        };
        if !seen.insert(page_key(url)) {
            continue;
        }
        let mut title = provider_text(row.get("title").as_str(), PROVIDER_LIMITS.title as f64);
        if title.is_empty() {
            title = utf16_prefix(url, PROVIDER_LIMITS.title).to_string();
        }
        let subtitle = provider_text(
            row.get("subtitle").as_str(),
            PROVIDER_LIMITS.subtitle as f64,
        );
        out.push(ProviderSuggestion {
            provider_id: id.to_string(),
            provider: provider.clone(),
            title,
            url: url.to_string(),
            icon: provider_icon(row.get("icon").as_str()).map(str::to_string),
            subtitle: (!subtitle.is_empty()).then_some(subtitle),
        });
    }
    out
}

/// Port of `fillProviderTemplate` (`providerRows.ts:79`): every `{query}` becomes the encoded,
/// trimmed query; `None` for an empty query or an unsafe result.
pub fn fill_provider_template(template: &str, query: &str) -> Option<String> {
    let q = js_trim(query);
    if q.is_empty() {
        return None;
    }
    let url = template.replace("{query}", &encode_uri_component(q));
    provider_url(Some(&url)).map(str::to_string)
}

/// Port of `parseProviderReply` (`providerRows.ts:85`). An OpenSearch reply
/// (`[query, texts, details?, urls?]`) gives `{ title, url, subtitle? }` objects; any other list
/// gives its plain objects as they are, unchecked as in TS, so the result stays [`JsValue`].
pub fn parse_provider_reply(body: &JsValue, template: Option<&str>) -> Vec<JsValue> {
    let JsValue::Array(body) = body else {
        return vec![];
    };
    let list = |i: usize| match body.get(i) {
        Some(JsValue::Array(items)) => Some(items.as_slice()),
        _ => None,
    };
    if let (Some(JsValue::String(_)), Some(texts)) = (body.first(), list(1)) {
        let details = list(2).unwrap_or_default();
        let urls = list(3).unwrap_or_default();
        let template = template.filter(|t| !t.is_empty());
        let mut rows = Vec::new();
        for (i, text) in texts.iter().enumerate() {
            let Some(text) = text.as_str().filter(|t| !js_trim(t).is_empty()) else {
                continue;
            };
            let url = match urls.get(i).and_then(JsValue::as_str) {
                Some(url) if !url.is_empty() => Some(url.to_string()),
                _ => template.and_then(|t| fill_provider_template(t, text)),
            };
            let Some(url) = url else {
                continue;
            };
            let mut row = vec![
                ("title".to_string(), JsValue::String(text.to_string())),
                ("url".to_string(), JsValue::String(url)),
            ];
            if let Some(subtitle) = details
                .get(i)
                .and_then(JsValue::as_str)
                .filter(|s| !s.is_empty())
            {
                row.push((
                    "subtitle".to_string(),
                    JsValue::String(subtitle.to_string()),
                ));
            }
            rows.push(JsValue::Object(row));
        }
        return rows;
    }
    body.iter()
        .filter(|row| row.is_plain_object())
        .cloned()
        .collect()
}

/// `new RegExp("(^|[^a-z0-9])" + escaped(first)).test(title)`: `first` occurs at the start of
/// `title` or right after a character outside `[a-z0-9]`.
fn starts_a_word(title: &str, first: &str) -> bool {
    let mut from = 0;
    while let Some(found) = title[from..].find(first) {
        let at = from + found;
        if !title[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        {
            return true;
        }
        from = at + title[at..].chars().next().map_or(1, char::len_utf8);
    }
    false
}

/// Port of `matchProviderRows` (`providerRows.ts:104`): the rows holding every typed word,
/// title-start matches first, then word-start matches, in input order within a score.
pub fn match_provider_rows<'a>(
    rows: &'a [ProviderRowInput],
    text: &str,
) -> Vec<&'a ProviderRowInput> {
    let lower = js_trim(text).to_lowercase();
    let tokens: Vec<&str> = lower.split(is_js_space).filter(|t| !t.is_empty()).collect();
    let Some(first) = tokens.first() else {
        return rows.iter().collect();
    };
    let mut scored: Vec<(&ProviderRowInput, u8)> = Vec::new();
    for row in rows {
        let title = row.title.to_lowercase();
        let haystack = format!(
            "{title} {} {}",
            row.subtitle.as_deref().unwrap_or("").to_lowercase(),
            row.url.to_lowercase()
        );
        if !tokens.iter().all(|t| haystack.contains(t)) {
            continue;
        }
        let score = if title.starts_with(first) {
            3
        } else if starts_a_word(&title, first) {
            2
        } else {
            1
        };
        scored.push((row, score));
    }
    scored.sort_by_key(|&(_, score)| std::cmp::Reverse(score));
    scored.into_iter().map(|(row, _)| row).collect()
}

/// Port of `globalProviderRows` (`providerRows.ts:119`): a few rows from each global section,
/// skipping pages already shown.
pub fn global_provider_rows(
    sections: &[ProviderSection],
    shown: &[ShownSuggestion],
) -> Vec<ProviderSuggestion> {
    let mut taken: HashSet<String> = shown
        .iter()
        .filter(|s| s.kind != "search")
        .filter_map(|s| s.url.map(|url| page_key(url).to_string()))
        .collect();
    let mut out = Vec::new();
    for section in sections {
        let mut in_section = 0;
        let rows = safe_provider_rows(
            &section.id,
            &section.title,
            &section.rows,
            PROVIDER_LIMITS.rows_per_provider as f64,
        );
        for row in rows {
            if out.len() >= PROVIDER_LIMITS.global_rows
                || in_section >= PROVIDER_LIMITS.rows_per_global_section
            {
                break;
            }
            if !taken.insert(page_key(&row.url).to_string()) {
                continue;
            }
            out.push(row);
            in_section += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_key_cuts_fragment_and_one_slash() {
        assert_eq!(page_key("https://a.com/x/#h"), "https://a.com/x");
        assert_eq!(page_key("https://a.com//"), "https://a.com/");
        assert_eq!(page_key("https://a.com/#a\n#b"), "https://a.com/#a\n");
    }

    #[test]
    fn urls_need_a_plain_authority() {
        assert_eq!(
            provider_url(Some(" https://[2001:db8::1]:80/ ")),
            Some("https://[2001:db8::1]:80/")
        );
        assert_eq!(provider_url(Some("https://good.com@evil.com/")), None);
        assert_eq!(provider_url(Some("https://a.com:123456/")), None);
        assert_eq!(
            provider_icon(Some("data:image/png;base64,AA==")),
            Some("data:image/png;base64,AA==")
        );
        assert_eq!(provider_icon(Some("data:image/png;base64,AA===")), None);
    }

    #[test]
    fn text_is_cleaned_and_capped() {
        assert_eq!(provider_text(Some("a\u{202e}b\0c\n x"), 60.0), "a b c x");
        assert_eq!(provider_text(Some("ab cd"), 4.0), "ab…");
        assert_eq!(provider_text(Some("😀😀😀"), 2.0), "😀…");
    }

    #[test]
    fn word_start_is_found_past_a_mid_word_hit() {
        assert!(starts_a_word("xc++ c++", "c++"));
        assert!(!starts_a_word("abc", "bc"));
    }
}
