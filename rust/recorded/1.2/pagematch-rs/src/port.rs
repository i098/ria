// Ported from ardia packages/core/src/pageMatch.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `parseQuery` of `ria-core` `url::page_match`. The scoring functions
//! take cached `PageText` objects and the narrowing chain is module state; both stay
//! TypeScript until the omnibox moves (slice 1.3).

#![forbid(unsafe_code)]

use ria_core::url::page_match;
use serde_json::{Value, json};

/// Port of `parseQuery` (`packages/core/src/pageMatch.ts:15`).
pub fn parse_query(q: String, tokens: Vec<String>) -> Result<Value, String> {
    let refs: Vec<&str> = tokens.iter().map(String::as_str).collect();
    let query = page_match::parse_query(&q, &refs);
    Ok(json!({
        "q": query.q,
        "segmentStarts": query.segment_starts.map(|(a, b)| json!([a, b])),
        "word": query.word,
        "spaced": query.spaced,
        "tokens": query.tokens.iter().map(|t| json!({"text": t.text, "word": t.word, "spaced": t.spaced})).collect::<Vec<_>>(),
        "needle": query.needle,
    }))
}
