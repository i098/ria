// Ported from ardia apps/browser/src/lib/emoji.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The emoji picker's sections and search: each term must start a word of the CLDR name or of its
//! keywords, without accents and case. Whole words of the name come first, then word starts of
//! the name, then keywords.
//!
//! Changes from Ardia:
//! - `EmojiEntry` borrows `emoji` and `name` from the static table; `emoji_sections` builds the
//!   sections once (`LazyLock`, as the `sections ??=` cache) and `search_emoji` gives references.
//! - `same_icon` takes `Option<&str>` for `string | null`.
//! - Text is UTF-8, not UTF-16; `includes` and `split` give the same results on whole strings.

use regex::Regex;
use std::sync::LazyLock;
use unicode_normalization::UnicodeNormalization;

use super::emoji_data::EMOJI_GROUPS;
use super::js::is_js_space;

/// `EmojiEntry` (`emoji.ts:3`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmojiEntry {
    pub emoji: &'static str,
    pub name: &'static str,
    pub name_words: String,
    pub words: String,
}

/// `EmojiSection` (`emoji.ts:4`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmojiSection {
    pub title: &'static str,
    pub emoji: Vec<EmojiEntry>,
}

static MARK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\p{M}").expect("static pattern"));

/// `fold` (`emoji.ts:8`): NFD, without marks, lower case.
fn fold(text: &str) -> String {
    MARK.replace_all(&text.nfd().collect::<String>(), "")
        .to_lowercase()
}

/// `wordsOf` (`emoji.ts:10`).
fn words_of(text: &str) -> String {
    format!(" {}", fold(text).replace(['.', ',', ':'], " "))
}

static SECTIONS: LazyLock<Vec<EmojiSection>> = LazyLock::new(|| {
    EMOJI_GROUPS
        .iter()
        .map(|group| EmojiSection {
            title: group.title,
            emoji: group
                .list
                .split('\n')
                .map(|line| {
                    let mut parts = line.split('\t');
                    let (emoji, name, words) = (
                        parts.next().unwrap_or(""),
                        parts.next().unwrap_or(""),
                        parts.next().unwrap_or(""),
                    );
                    EmojiEntry {
                        emoji,
                        name,
                        name_words: words_of(name),
                        words: format!(" {}", fold(words)),
                    }
                })
                .collect(),
        })
        .collect()
});

/// `emojiSections` (`emoji.ts:12`).
pub fn emoji_sections() -> &'static [EmojiSection] {
    &SECTIONS
}

/// `searchTerms` (`emoji.ts:23`): the folded words of `query`, split at JS `\s`, `.`, `,` and `:`.
pub fn search_terms(query: &str) -> Vec<String> {
    fold(query)
        .split(|c: char| is_js_space(c) || matches!(c, '.' | ',' | ':'))
        .filter(|term| !term.is_empty())
        .map(String::from)
        .collect()
}

/// `hasTerms` (`emoji.ts:29`).
fn has_terms(words: &str, terms: &[String]) -> bool {
    terms.iter().all(|term| words.contains(&format!(" {term}")))
}

/// `matchesTerms` (`emoji.ts:31`).
pub fn matches_terms(text: &str, terms: &[String]) -> bool {
    has_terms(&words_of(text), terms)
}

/// `searchEmoji` (`emoji.ts:33`).
pub fn search_emoji(terms: &[String]) -> Vec<&'static EmojiEntry> {
    let (mut by_word, mut by_name, mut by_keyword) = (Vec::new(), Vec::new(), Vec::new());
    for entry in emoji_sections().iter().flat_map(|section| &section.emoji) {
        let name_words = format!("{} ", entry.name_words);
        if terms
            .iter()
            .all(|term| name_words.contains(&format!(" {term} ")))
        {
            by_word.push(entry);
        } else if has_terms(&entry.name_words, terms) {
            by_name.push(entry);
        } else if has_terms(&(entry.name_words.clone() + &entry.words), terms) {
            by_keyword.push(entry);
        }
    }
    by_word.extend(by_name);
    by_word.extend(by_keyword);
    by_word
}

/// `bare` (`emoji.ts:46`): without U+FE0F, the emoji presentation selector.
fn bare(icon: &str) -> String {
    icon.replace('\u{FE0F}', "")
}

/// `sameIcon` (`emoji.ts:48`).
pub fn same_icon(a: Option<&str>, b: Option<&str>) -> bool {
    a == b
        || matches!((a, b), (Some(a), Some(b)) if !a.is_empty() && !b.is_empty() && bare(a) == bare(b))
}
