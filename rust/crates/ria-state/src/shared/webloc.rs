// Ported from ardia apps/browser/src/lib/webloc.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! macOS `.webloc` link files: write the plist, read the URL and the Ardia link id back.
//!
//! Changes from Ardia:
//! - The patterns spell out the JavaScript `\s` set and ASCII-only case folding: Rust `(?i)` would
//!   also match `ſ` for `s` and the Kelvin sign for `k`.
//! - `unescapeXml` gives U+FFFD for `&#xD800;` to `&#xDFFF;`: a Rust `String` cannot hold a lone surrogate.
//! - `newLinkId` takes its random part from the std hasher seed in place of `Math.random`.

use regex::{Captures, Regex};
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

use super::js::js_trim;

/// JavaScript `\s`.
const JS_SPACE: &str = r"[\t\n\x0B\x0C\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]";

static ENTITY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"&(#[xX][0-9a-fA-F]+|#[0-9]+|[a-zA-Z]+);").expect("static pattern")
});
static COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<!--.*?-->").expect("static pattern"));

/// `<key>\s*KEY\s*</key>\s*<string>([\s\S]*?)</string>` with the `i` flag and no `u` flag.
fn key_pattern(key: &str) -> Regex {
    let ci = |text: &str| -> String {
        text.chars()
            .map(|c| {
                if c.is_ascii_alphabetic() {
                    format!("[{}{}]", c.to_ascii_lowercase(), c.to_ascii_uppercase())
                } else {
                    c.to_string()
                }
            })
            .collect()
    };
    let s = JS_SPACE;
    Regex::new(&format!(
        r"<{k}>{s}*{key}{s}*</{k}>{s}*<{st}>((?s:.)*?)</{st}>",
        k = ci("key"),
        key = ci(key),
        st = ci("string")
    ))
    .expect("static pattern")
}

/// `escapeXml` (`webloc.ts:4`).
pub fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// `unescapeXml` (`webloc.ts:6`): named and numeric character references.
pub fn unescape_xml(text: &str) -> String {
    ENTITY
        .replace_all(text, |c: &Captures| {
            let (whole, name) = (&c[0], &c[1]);
            let Some(digits) = name.strip_prefix('#') else {
                return match name.to_ascii_lowercase().as_str() {
                    "amp" => "&",
                    "lt" => "<",
                    "gt" => ">",
                    "quot" => "\"",
                    "apos" => "'",
                    // `NAMED` is an object literal: `constructor` is `Object.prototype.constructor`.
                    "constructor" => "function Object() { [native code] }",
                    _ => whole,
                }
                .to_string();
            };
            let (digits, radix) = match digits.strip_prefix(['x', 'X']) {
                Some(hex) => (hex, 16),
                None => (digits, 10),
            };
            let code = digits.chars().try_fold(0u32, |acc, d| {
                acc.checked_mul(radix)?.checked_add(d.to_digit(radix)?)
            });
            match code.filter(|&c| c <= 0x10ffff) {
                Some(code) => char::from_u32(code).unwrap_or('\u{fffd}').to_string(),
                None => whole.to_string(),
            }
        })
        .into_owned()
}

/// `weblocText` (`webloc.ts:14`): the plist text of a `.webloc` file.
pub fn webloc_text(url: &str, id: Option<&str>) -> String {
    let mut lines = vec![
        r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_string(),
        r#"<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">"#.to_string(),
        r#"<plist version="1.0">"#.to_string(),
        "<dict>".to_string(),
        "\t<key>URL</key>".to_string(),
        format!("\t<string>{}</string>", escape_xml(url)),
    ];
    if let Some(id) = id.filter(|id| !id.is_empty()) {
        lines.push("\t<key>ArdiaLink</key>".to_string());
        lines.push(format!("\t<string>{}</string>", escape_xml(id)));
    }
    lines.extend(["</dict>", "</plist>", ""].map(str::to_string));
    lines.join("\n")
}

/// `stringFor` (`webloc.ts:29`): the trimmed `<string>` after `<key>KEY</key>`, outside comments.
pub fn string_for(text: &str, key: &str) -> Option<String> {
    let plain = COMMENT.replace_all(text, "");
    let found = key_pattern(key).captures(&plain)?;
    let value = unescape_xml(&found[1]);
    let value = js_trim(&value);
    (!value.is_empty()).then(|| value.to_string())
}

/// `parseWebloc` (`webloc.ts:35`).
pub fn parse_webloc(text: &str) -> Option<String> {
    string_for(text, "URL")
}

/// `weblocLinkId` (`webloc.ts:37`).
pub fn webloc_link_id(text: &str) -> Option<String> {
    string_for(text, "ArdiaLink")
}

/// Base 36 digits of `n`.
fn base36(mut n: u64) -> String {
    let mut digits = Vec::new();
    loop {
        digits.push(char::from_digit((n % 36) as u32, 36).expect("digit below 36"));
        n /= 36;
        if n == 0 {
            break;
        }
    }
    digits.iter().rev().collect()
}

/// `newLinkId` (`webloc.ts:39`): the time in base 36, then 8 random base-36 digits.
pub fn new_link_id() -> String {
    use std::hash::{BuildHasher, RandomState};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    let random = base36(RandomState::new().hash_one(now));
    format!(
        "{}{}",
        base36(now),
        &random[random.len().saturating_sub(8)..]
    )
}
