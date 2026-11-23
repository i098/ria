// Ported from ardia packages/core/src/textFragment.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Text fragment (`#:~:text=`) choice and URL building.

use crate::fuzzy::is_js_space;
use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextFragment {
    pub prefix: Option<String>,
    pub start: String,
    pub end: Option<String>,
    pub suffix: Option<String>,
}

pub struct SelectionContext<'a> {
    pub selected: &'a str,
    pub before: &'a str,
    pub after: &'a str,
    pub page_text: &'a str,
}

const MAX_EXACT_WORDS: usize = 10;
const RANGE_WORDS: usize = 3;
const MAX_CONTEXT_WORDS: usize = 5;

/// `s.replace(/\s+/g, " ").trim()`.
fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_space = false;
    for c in s.chars() {
        if is_js_space(c) {
            in_space = true;
        } else {
            if in_space && !out.is_empty() {
                out.push(' ');
            }
            in_space = false;
            out.push(c);
        }
    }
    out
}

fn words_of(s: &str) -> Vec<String> {
    normalize(s)
        .split(' ')
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// Overlapping occurrences of `needle` in `haystack`, as the `indexOf` loop counts them.
fn occurrences(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    let mut count = 0;
    let mut from = 0;
    while let Some(i) = haystack[from..].find(needle) {
        count += 1;
        let at = from + i;
        from = at + haystack[at..].chars().next().map_or(1, char::len_utf8);
    }
    count
}

static WORD_END: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\p{L}\p{N}_]+$").expect("static regex"));
static WORD_START: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\p{L}\p{N}_]+").expect("static regex"));

/// Moves the selection edges out to whole words; returns the selection, `before` and `after`.
fn to_word_boundaries<'a>(
    selected: &str,
    before: &'a str,
    after: &'a str,
) -> (String, &'a str, &'a str) {
    let head = if WORD_START.is_match(selected) {
        WORD_END.find(before).map_or("", |m| m.as_str())
    } else {
        ""
    };
    let tail = if WORD_END.is_match(selected) {
        WORD_START.find(after).map_or("", |m| m.as_str())
    } else {
        ""
    };
    (
        format!("{head}{selected}{tail}"),
        &before[..before.len() - head.len()],
        &after[tail.len()..],
    )
}

/// `selected.split(/\s*\n\s*/)`: a whitespace run that holds a line feed separates blocks.
fn split_blocks(s: &str) -> Vec<&str> {
    let mut blocks = Vec::new();
    let mut block_start = 0;
    let mut run: Option<(usize, bool)> = None; // start of the whitespace run, has a line feed
    for (i, c) in s.char_indices() {
        if is_js_space(c) {
            let (start, lf) = run.unwrap_or((i, false));
            run = Some((start, lf || c == '\n'));
        } else if let Some((start, true)) = run.take() {
            blocks.push(&s[block_start..start]);
            block_start = i;
        }
    }
    match run {
        Some((start, true)) => {
            blocks.push(&s[block_start..start]);
            blocks.push("");
        }
        _ => blocks.push(&s[block_start..]),
    }
    blocks
}

fn last_n(words: &[String], n: usize) -> String {
    words[words.len().saturating_sub(n)..].join(" ")
}

/// Port of `chooseTextFragment` (`textFragment.ts:39`).
pub fn choose_text_fragment(context: &SelectionContext) -> Option<TextFragment> {
    let (selected, before, after) =
        to_word_boundaries(context.selected, context.before, context.after);
    let blocks: Vec<String> = split_blocks(&selected)
        .into_iter()
        .map(normalize)
        .filter(|b| !b.is_empty())
        .collect();
    if blocks.is_empty() {
        return None;
    }
    let page = normalize(context.page_text).to_lowercase();
    let count = |text: &str| occurrences(&page, &text.to_lowercase());
    let all: Vec<String> = blocks.iter().flat_map(|b| words_of(b)).collect();
    let before_words = words_of(before);
    let after_words = words_of(after);

    if blocks.len() == 1 && all.len() <= MAX_EXACT_WORDS {
        let start = all.join(" ");
        if count(&start) <= 1 {
            return Some(TextFragment {
                start,
                ..Default::default()
            });
        }
        for n in 1.. {
            let prefix = last_n(&before_words, n);
            let suffix = after_words[..n.min(after_words.len())].join(" ");
            let exhausted =
                n >= MAX_CONTEXT_WORDS || n >= before_words.len().max(after_words.len());
            let joined = [prefix.as_str(), start.as_str(), suffix.as_str()]
                .iter()
                .filter(|s| !s.is_empty())
                .copied()
                .collect::<Vec<_>>()
                .join(" ");
            if exhausted || count(&joined) <= 1 {
                return Some(TextFragment {
                    prefix: (!prefix.is_empty()).then_some(prefix),
                    start,
                    end: None,
                    suffix: (!suffix.is_empty()).then_some(suffix),
                });
            }
        }
    }
    let first = words_of(&blocks[0]);
    let last = words_of(&blocks[blocks.len() - 1]);
    let most = if blocks.len() == 1 {
        all.len() / 2
    } else {
        first.len().min(last.len())
    };
    let mut n = RANGE_WORDS.min(most);
    while n < most && count(&first[..n].join(" ")) > 1 {
        n += 1;
    }
    let start = first[..n].join(" ");
    let end = last_n(&last, n);
    if count(&start) <= 1 || before_words.is_empty() {
        return Some(TextFragment {
            start,
            end: Some(end),
            ..Default::default()
        });
    }
    Some(TextFragment {
        prefix: Some(last_n(&before_words, RANGE_WORDS)),
        start,
        end: Some(end),
        suffix: None,
    })
}

/// `encodeURIComponent(s).replace(/-/g, "%2D")`.
fn encode_term(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'-' => out.push_str("%2D"),
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn text_directive(f: &TextFragment) -> String {
    let parts = [
        f.prefix.as_deref().map(|p| format!("{}-", encode_term(p))),
        Some(encode_term(&f.start)),
        f.end.as_deref().map(encode_term),
        f.suffix.as_deref().map(|s| format!("-{}", encode_term(s))),
    ];
    // An empty prefix or suffix is falsy in the original, so it adds no part.
    let parts: Vec<String> = parts
        .into_iter()
        .flatten()
        .filter(|p| !p.is_empty() && p != "-")
        .collect();
    format!("text={}", parts.join(","))
}

/// Port of `withTextFragment` (`textFragment.ts:78`).
pub fn with_text_fragment(url: &str, fragment: &TextFragment) -> String {
    let hash = url.find('#');
    let base = hash.map_or(url, |h| &url[..h]);
    let mut reference = hash.map_or("", |h| &url[h + 1..]);
    if let Some(directive) = reference.find(":~:") {
        reference = &reference[..directive];
    }
    format!("{base}#{reference}:~:{}", text_directive(fragment))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_blocks_on_whitespace_runs_with_a_line_feed() {
        assert_eq!(split_blocks("a b \n c\td\n"), vec!["a b", "c\td", ""]);
        assert_eq!(split_blocks("\n"), vec!["", ""]);
        assert_eq!(split_blocks("a  b"), vec!["a  b"]);
    }
}
