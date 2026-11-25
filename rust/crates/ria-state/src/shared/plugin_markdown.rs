// Ported from ardia apps/browser/src/lib/pluginMarkdown.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! A small Markdown reader for plugin text: headings, paragraphs, fenced code, lists, quotes and
//! rules, with bold, italic, code and http(s) links inline. The inline scanner runs in linear time.
//!
//! Changes from Ardia:
//! - Indexes are UTF-8 byte offsets where the TS uses UTF-16 code units. Every index the scanner
//!   keeps is at or next to an ASCII mark (`*`, `_`, `` ` ``, `[`, `]`, `\`), and a search from inside a
//!   multi-byte character finds the same ASCII mark, so the spans are the same.
//! - `RULE` uses a backreference (`\1`), which the `regex` crate lacks: it is three alternatives.
//!   JS `.` (no line terminators), `\s` (the JS set) and `\d` (ASCII) are written out.
//! - `SpanStyle` fields are `Option`s, as `undefined` is distinct from `false` for `??`.
//!   A `MarkdownSpan` keeps `bold`, `italic` and `code` as `bool`: the TS sets them only to `true`.
//! - The fence close pattern, built with `new RegExp` in the TS, is checked in code (`closes_fence`).

use regex::Regex;
use ria_core::identity::telemetry_wire::Json;
use std::collections::HashMap;
use std::sync::LazyLock;

use super::js::{is_js_space, js_number, js_trim};
use super::plugin_web_url::{PluginWebUrl, plugin_web_url};

/// The JS `\s` set, for character classes.
const S: &str =
    r"\t\n\x0B\f\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}";
/// JS `.` without the `s` flag.
const DOT: &str = r"[^\n\r\u{2028}\u{2029}]";

/// `MarkdownSpan` (`pluginMarkdown.ts:3`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MarkdownSpan {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub url: Option<String>,
}

/// `MarkdownListItem` (`pluginMarkdown.ts:5`).
#[derive(Debug, Clone, PartialEq)]
pub struct MarkdownListItem {
    pub marker: String,
    pub depth: usize,
    pub spans: Vec<MarkdownSpan>,
}

/// `MarkdownBlock` (`pluginMarkdown.ts:7`).
#[derive(Debug, Clone, PartialEq)]
pub enum MarkdownBlock {
    /// `level` is 1, 2 or 3.
    Heading {
        level: usize,
        spans: Vec<MarkdownSpan>,
    },
    Paragraph {
        spans: Vec<MarkdownSpan>,
    },
    Code {
        text: String,
        language: Option<String>,
    },
    List {
        ordered: bool,
        items: Vec<MarkdownListItem>,
    },
    Quote {
        blocks: Vec<MarkdownBlock>,
    },
    Rule,
}

/// `SpanStyle` (`pluginMarkdown.ts:15`): `MarkdownSpan` without its text.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpanStyle {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub code: Option<bool>,
    pub url: Option<String>,
}

const ESCAPABLE: &str = "\\`*_[]()#>!+.-";

fn re(source: &str) -> Regex {
    Regex::new(&source.replace("{S}", S).replace("{DOT}", DOT)).expect("static pattern")
}

static LINK_TAIL: LazyLock<Regex> = LazyLock::new(|| {
    re(r#"^\([{S}]*<?((?:[^(){S}<>]|\([^(){S}]*\))+)>?(?:[{S}]+"[^"]*")?[{S}]*\)"#)
});
static FENCE: LazyLock<Regex> = LazyLock::new(|| re(r"^ {0,3}(`{3,}|~{3,})[{S}]*([^`{S}]*)"));
static HEADING: LazyLock<Regex> =
    LazyLock::new(|| re(r"^ {0,3}(#{1,6})(?:[{S}]+({DOT}*?))?[{S}]*$"));
static HEADING_TAIL: LazyLock<Regex> = LazyLock::new(|| re(r"[{S}]+#+$"));
static RULE: LazyLock<Regex> =
    LazyLock::new(|| re(r"^ {0,3}(?:-(?:[{S}]*-){2,}|\*(?:[{S}]*\*){2,}|_(?:[{S}]*_){2,})[{S}]*$"));
static QUOTE: LazyLock<Regex> = LazyLock::new(|| re(r"^ {0,3}>[{S}]?({DOT}*)$"));
static BULLET: LazyLock<Regex> = LazyLock::new(|| re(r"^([{S}]*)([-*+])[{S}]+({DOT}*)$"));
static ORDERED: LazyLock<Regex> = LazyLock::new(|| re(r"^([{S}]*)([0-9]{1,9})[.)][{S}]+({DOT}*)$"));
const MAX_DEPTH: usize = 4;
const MAX_INLINE_DEPTH: f64 = 16.0;

/// `markdownLink` (`pluginMarkdown.ts:31`): the checked web address, else `None`.
pub fn markdown_link(raw: &str) -> Option<String> {
    match plugin_web_url(&Json::String(raw.to_owned())) {
        PluginWebUrl::Url { url, .. } => Some(url),
        PluginWebUrl::Problem(_) => None,
    }
}

/// `InlineToken` (`pluginMarkdown.ts:36`).
struct InlineToken<'a> {
    end: usize,
    text: &'a str,
    extra: SpanStyle,
    nested: bool,
    href: Option<&'a str>,
}

/// `forwardSearch` (`pluginMarkdown.ts:38`): a search whose last answer serves later starts it still covers.
#[derive(Default)]
struct Forward {
    asked: Option<usize>,
    found: Option<usize>,
}

impl Forward {
    fn search(&mut self, from: usize, find: impl FnOnce(usize) -> Option<usize>) -> Option<usize> {
        if self.asked.is_some_and(|a| from >= a) && self.found.is_none_or(|f| f >= from) {
            return self.found;
        }
        self.asked = Some(from);
        self.found = find(from);
        self.found
    }
}

/// `source.indexOf(needle, from)` on bytes (`from` may be inside a character).
fn index_of(source: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from > source.len() {
        return None;
    }
    source[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

/// `inlineScanner` (`pluginMarkdown.ts:49`).
struct Scanner<'a> {
    source: &'a str,
    next_tick: Forward,
    next_bracket: Forward,
    next_star: Forward,
    strong_star: Forward,
    strong_underscore: Forward,
    next_line_end: Forward,
    tails: HashMap<usize, Option<(usize, &'a str)>>,
}

impl<'a> Scanner<'a> {
    fn new(source: &'a str) -> Self {
        Scanner {
            source,
            next_tick: Forward::default(),
            next_bracket: Forward::default(),
            next_star: Forward::default(),
            strong_star: Forward::default(),
            strong_underscore: Forward::default(),
            next_line_end: Forward::default(),
            tails: HashMap::new(),
        }
    }

    fn byte(&self, at: usize) -> Option<u8> {
        self.source.as_bytes().get(at).copied()
    }

    /// `isSpace(source[at])` for the character that starts at `at` (`None` past the end).
    fn space_at(&self, at: usize) -> bool {
        self.source
            .get(at..)
            .and_then(|s| s.chars().next())
            .is_none_or(is_js_space)
    }

    /// `isSpace(source[at - 1])` for the character that ends at `at`.
    fn space_before(&self, at: usize) -> bool {
        self.source[..at]
            .chars()
            .next_back()
            .is_none_or(is_js_space)
    }

    /// `[length of the match, href]` of `LINK_TAIL` at `at`.
    fn tail_at(&mut self, at: usize) -> Option<(usize, &'a str)> {
        let source = self.source;
        *self.tails.entry(at).or_insert_with(|| {
            let found = LINK_TAIL.captures(source.get(at..)?)?;
            Some((found[0].len(), found.get(1)?.as_str()))
        })
    }

    fn doubled(&mut self, at: usize, mark: u8) -> Option<InlineToken<'a>> {
        if self.byte(at + 1) != Some(mark) {
            return None;
        }
        let source = self.source;
        let bytes = source.as_bytes();
        let find = |from: usize| {
            let mut at = index_of(bytes, &[mark, mark], from);
            while let Some(a) = at {
                if bytes.get(a + 2) != Some(&mark) {
                    return Some(a);
                }
                at = index_of(bytes, &[mark, mark], a + 1);
            }
            None
        };
        let strong = if mark == b'*' {
            &mut self.strong_star
        } else {
            &mut self.strong_underscore
        };
        let close = strong.search(at + 3, find)?;
        let line_end = self.next_line_end.search(at + 2, |from| {
            let rest = bytes.get(from..)?;
            rest.iter()
                .enumerate()
                .position(|(i, b)| {
                    matches!(b, b'\n' | b'\r')
                        || (*b == 0xE2
                            && rest.get(i + 1) == Some(&0x80)
                            && matches!(rest.get(i + 2), Some(0xA8 | 0xA9)))
                })
                .map(|p| p + from)
        });
        if line_end.is_some_and(|e| e < close) {
            return None;
        }
        Some(InlineToken {
            end: close + 2,
            text: &source[at + 2..close],
            extra: SpanStyle {
                bold: Some(true),
                ..SpanStyle::default()
            },
            nested: true,
            href: None,
        })
    }

    fn token_at(&mut self, at: usize) -> Option<InlineToken<'a>> {
        let source = self.source;
        let bytes = source.as_bytes();
        match bytes[at] {
            b'\\' => {
                // ESCAPABLE is ASCII, so the escaped character ends at `at + 2`.
                self.byte(at + 1)
                    .filter(|b| ESCAPABLE.as_bytes().contains(b))?;
                Some(InlineToken {
                    end: at + 2,
                    text: &source[at + 1..at + 2],
                    extra: SpanStyle::default(),
                    nested: false,
                    href: None,
                })
            }
            b'`' => {
                let close = self
                    .next_tick
                    .search(at + 1, |from| index_of(bytes, b"`", from))?;
                (close > at + 1).then(|| InlineToken {
                    end: close + 1,
                    text: &source[at + 1..close],
                    extra: SpanStyle {
                        code: Some(true),
                        ..SpanStyle::default()
                    },
                    nested: false,
                    href: None,
                })
            }
            b'[' => {
                let close = self
                    .next_bracket
                    .search(at + 1, |from| index_of(bytes, b"]", from))?;
                if close <= at + 1 {
                    return None;
                }
                let (length, href) = self.tail_at(close + 1)?;
                Some(InlineToken {
                    end: close + 1 + length,
                    text: &source[at + 1..close],
                    extra: SpanStyle::default(),
                    nested: true,
                    href: Some(href),
                })
            }
            b'_' => self.doubled(at, b'_'),
            b'*' => {
                if let Some(strong) = self.doubled(at, b'*') {
                    return Some(strong);
                }
                if self.space_at(at + 1) || self.byte(at + 1) == Some(b'*') {
                    return None;
                }
                let close = self
                    .next_star
                    .search(at + 2, |from| index_of(bytes, b"*", from))?;
                if self.space_before(close) {
                    return None;
                }
                Some(InlineToken {
                    end: close + 1,
                    text: &source[at + 1..close],
                    extra: SpanStyle {
                        italic: Some(true),
                        ..SpanStyle::default()
                    },
                    nested: true,
                    href: None,
                })
            }
            _ => None,
        }
    }
}

/// `parseInline` (`pluginMarkdown.ts:110`): the styled spans of one line of text. `depth` is the
/// TS number, any `f64`.
pub fn parse_inline(source: &str, style: &SpanStyle, depth: f64) -> Vec<MarkdownSpan> {
    let mut spans: Vec<MarkdownSpan> = Vec::new();
    let mut push = |text: &str, extra: &SpanStyle| {
        if text.is_empty() {
            return;
        }
        let next = MarkdownSpan {
            text: text.to_owned(),
            bold: extra.bold.or(style.bold).unwrap_or(false),
            italic: extra.italic.or(style.italic).unwrap_or(false),
            code: extra.code.or(style.code).unwrap_or(false),
            url: extra
                .url
                .as_ref()
                .or(style.url.as_ref())
                .filter(|u| !u.is_empty())
                .cloned(),
        };
        match spans.last_mut() {
            Some(last)
                if last.bold == next.bold
                    && last.italic == next.italic
                    && last.code == next.code
                    && last.url == next.url =>
            {
                last.text.push_str(text);
            }
            _ => spans.push(next),
        }
    };
    let link_style = |href: &str| SpanStyle {
        url: style.url.clone().or_else(|| markdown_link(href)),
        ..SpanStyle::default()
    };
    let mut scanner = Scanner::new(source);
    let bytes = source.as_bytes();
    let mut text_from = 0;
    let mut last_index = 0;
    while let Some(found) = bytes
        .get(last_index..)
        .and_then(|rest| {
            rest.iter()
                .position(|b| matches!(b, b'\\' | b'`' | b'[' | b'*' | b'_'))
        })
        .map(|p| p + last_index)
    {
        last_index = found + 1;
        let Some(token) = scanner.token_at(found) else {
            continue;
        };
        push(&source[text_from..found], &SpanStyle::default());
        if !token.nested {
            push(token.text, &token.extra);
        } else if depth >= MAX_INLINE_DEPTH {
            match token.href {
                None => push(token.text, &token.extra),
                Some(href) => push(token.text, &link_style(href)),
            }
        } else {
            let inner = match token.href {
                None => SpanStyle {
                    bold: token.extra.bold.or(style.bold),
                    italic: token.extra.italic.or(style.italic),
                    code: token.extra.code.or(style.code),
                    url: style.url.clone(),
                },
                Some(href) => SpanStyle {
                    url: link_style(href).url,
                    ..style.clone()
                },
            };
            for span in parse_inline(token.text, &inner, depth + 1.0) {
                let extra = SpanStyle {
                    bold: span.bold.then_some(true),
                    italic: span.italic.then_some(true),
                    code: span.code.then_some(true),
                    url: span.url,
                };
                push(&span.text, &extra);
            }
        }
        text_from = token.end;
        last_index = token.end;
    }
    push(&source[text_from..], &SpanStyle::default());
    spans
}

/// `indentOf` (`pluginMarkdown.ts:143`): the list depth of the leading space, tabs as 4.
fn indent_of(lead: &str) -> usize {
    let width: usize = lead
        .chars()
        .map(|c| if c == '\t' { 4 } else { c.len_utf16() })
        .sum();
    MAX_DEPTH.min(width / 2)
}

/// The result of `listMatch`.
struct ListLine<'a> {
    ordered: bool,
    depth: usize,
    marker: String,
    text: &'a str,
}

/// `listMatch` (`pluginMarkdown.ts:145`).
fn list_match(line: &str) -> Option<ListLine<'_>> {
    if let Some(bullet) = BULLET.captures(line) {
        return Some(ListLine {
            ordered: false,
            depth: indent_of(&bullet[1]),
            marker: "•".to_owned(),
            text: bullet.get(3)?.as_str(),
        });
    }
    let ordered = ORDERED.captures(line)?;
    Some(ListLine {
        ordered: true,
        depth: indent_of(&ordered[1]),
        marker: format!("{}.", js_number(&ordered[2])),
        text: ordered.get(3)?.as_str(),
    })
}

/// `startsBlock` (`pluginMarkdown.ts:153`).
fn starts_block(line: &str, quotes: bool) -> bool {
    FENCE.is_match(line)
        || HEADING.is_match(line)
        || RULE.is_match(line)
        || (quotes && QUOTE.is_match(line))
        || list_match(line).is_some()
}

/// The fence close pattern of `parseMarkdown` (`pluginMarkdown.ts:170`): up to 3 spaces, at least
/// `length` of `mark`, then only spaces.
fn closes_fence(line: &str, mark: char, length: usize) -> bool {
    let lead = line.len() - line.trim_start_matches(' ').len();
    if lead > 3 {
        return false;
    }
    let rest = &line[lead..];
    let run = rest.len() - rest.trim_start_matches(mark).len();
    run >= length && rest[run..].chars().all(is_js_space)
}

/// `parseMarkdown` (`pluginMarkdown.ts:157`): the blocks of `source`; quotes nest up to 4 deep.
pub fn parse_markdown(source: &str, depth: f64) -> Vec<MarkdownBlock> {
    let quotes = depth < MAX_DEPTH as f64;
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.split('\n').collect();
    let mut blocks = Vec::new();
    let mut at = 0;
    let plain = SpanStyle::default();
    while at < lines.len() {
        let line = lines[at];
        if js_trim(line).is_empty() {
            at += 1;
            continue;
        }
        if let Some(fence) = FENCE.captures(line) {
            let mark = if fence[1].starts_with('`') { '`' } else { '~' };
            let length = fence[1].len();
            let mut body = Vec::new();
            at += 1;
            while at < lines.len() && !closes_fence(lines[at], mark, length) {
                body.push(lines[at]);
                at += 1;
            }
            at += 1;
            let language = Some(&fence[2]).filter(|l| !l.is_empty()).map(str::to_owned);
            blocks.push(MarkdownBlock::Code {
                text: body.join("\n"),
                language,
            });
            continue;
        }
        if let Some(heading) = HEADING.captures(line) {
            let level = heading[1].len().min(3);
            let text = heading.get(2).map_or("", |m| m.as_str());
            let text = HEADING_TAIL.replace(text, "");
            blocks.push(MarkdownBlock::Heading {
                level,
                spans: parse_inline(&text, &plain, 0.0),
            });
            at += 1;
            continue;
        }
        if RULE.is_match(line) {
            blocks.push(MarkdownBlock::Rule);
            at += 1;
            continue;
        }
        if quotes && QUOTE.is_match(line) {
            let mut inner = Vec::new();
            while let Some(q) = lines.get(at).and_then(|l| QUOTE.captures(l)) {
                inner.push(q.get(1).map_or("", |m| m.as_str()));
                at += 1;
            }
            blocks.push(MarkdownBlock::Quote {
                blocks: parse_markdown(&inner.join("\n"), depth + 1.0),
            });
            continue;
        }
        if let Some(first) = list_match(line) {
            let mut items = Vec::new();
            let mut texts: Vec<String> = Vec::new();
            while at < lines.len() {
                let current = lines[at];
                let item = list_match(current);
                match item {
                    Some(item) if item.depth > 0 || item.ordered == first.ordered => {
                        items.push(MarkdownListItem {
                            marker: item.marker,
                            depth: item.depth,
                            spans: Vec::new(),
                        });
                        texts.push(item.text.to_owned());
                    }
                    None if !js_trim(current).is_empty()
                        && current.chars().next().is_some_and(is_js_space)
                        && !texts.is_empty() =>
                    {
                        let last = texts.last_mut().expect("texts is not empty");
                        last.push(' ');
                        last.push_str(js_trim(current));
                    }
                    _ => break,
                }
                at += 1;
            }
            for (item, text) in items.iter_mut().zip(&texts) {
                item.spans = parse_inline(text, &plain, 0.0);
            }
            blocks.push(MarkdownBlock::List {
                ordered: first.ordered,
                items,
            });
            continue;
        }
        let mut paragraph = vec![js_trim(line)];
        at += 1;
        while at < lines.len() && !js_trim(lines[at]).is_empty() && !starts_block(lines[at], quotes)
        {
            paragraph.push(js_trim(lines[at]));
            at += 1;
        }
        blocks.push(MarkdownBlock::Paragraph {
            spans: parse_inline(&paragraph.join(" "), &plain, 0.0),
        });
    }
    blocks
}
