// Ported from ardia apps/browser/src/lib/singleLine.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Text on one line: line breaks and the space around them become one space.
//!
//! Changes from Ardia: none. The patterns spell out the JS `\s` set; U+0085 is a line break here but not
//! JS space.

use regex::Regex;
use std::sync::LazyLock;

use super::js::js_trim;

/// The characters of `[\r\n\u2028\u2029\u0085\v\f]` (`singleLine.ts:1`).
const BREAK: &str = r"\r\n\u{2028}\u{2029}\u{85}\x0B\x0C";
/// The JS `\s` set (`ria_core::search::js_text::is_js_space`).
const SPACE: &str = r"\t\n\x0B\x0C\r \u{A0}\u{1680}\u{2000}-\u{200A}\u{2028}\u{2029}\u{202F}\u{205F}\u{3000}\u{FEFF}";

static LINE_BREAKS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!("[{SPACE}]*[{BREAK}]+[{SPACE}]*")).expect("static pattern")
});

/// `singleLine` (`singleLine.ts:4`): text without line breaks comes back as it is.
pub fn single_line(text: &str) -> String {
    if !text.contains([
        '\r', '\n', '\u{2028}', '\u{2029}', '\u{85}', '\u{b}', '\u{c}',
    ]) {
        return text.to_owned();
    }
    js_trim(&LINE_BREAKS.replace_all(text, " ")).to_owned()
}
