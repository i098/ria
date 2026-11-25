// Ported from ardia apps/browser/src/lib/emoji.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::emoji`: entries cross as `{ emoji, name, nameWords,
//! words }`. The logic is in `ria_state::shared::emoji`.

#![forbid(unsafe_code)]

use ria_state::shared::emoji as e;
use serde_json::{Value, json};

fn entry(x: &e::EmojiEntry) -> Value {
    json!({ "emoji": x.emoji, "name": x.name, "nameWords": x.name_words, "words": x.words })
}

/// Port of `emojiSections` (`apps/browser/src/lib/emoji.ts:12`).
pub fn emoji_sections() -> Result<Vec<Value>, String> {
    Ok(e::emoji_sections()
        .iter()
        .map(
            |s| json!({ "title": s.title, "emoji": s.emoji.iter().map(entry).collect::<Vec<_>>() }),
        )
        .collect())
}

/// Port of `searchTerms` (`apps/browser/src/lib/emoji.ts:23`).
pub fn search_terms(query: String) -> Result<Vec<String>, String> {
    Ok(e::search_terms(&query))
}

/// Port of `matchesTerms` (`apps/browser/src/lib/emoji.ts:31`).
pub fn matches_terms(text: String, terms: Vec<String>) -> Result<Value, String> {
    Ok(json!(e::matches_terms(&text, &terms)))
}

/// Port of `searchEmoji` (`apps/browser/src/lib/emoji.ts:33`).
pub fn search_emoji(terms: Vec<String>) -> Result<Vec<Value>, String> {
    Ok(e::search_emoji(&terms).into_iter().map(entry).collect())
}

/// Port of `sameIcon` (`apps/browser/src/lib/emoji.ts:48`). `null` and `undefined` differ in JS:
/// `===` compares the raw values, and only two strings go to `ria_state`.
pub fn same_icon(a: Value, b: Value) -> Result<Value, String> {
    Ok(json!(
        a == b
            || matches!((a.as_str(), b.as_str()), (Some(a), Some(b)) if e::same_icon(Some(a), Some(b)))
    ))
}
