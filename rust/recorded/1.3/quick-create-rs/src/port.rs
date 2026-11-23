// Ported from ardia packages/core/src/quickCreate.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `omnibox::quick_create`: `limit` follows `Array.slice(0, limit)`.
//! The logic is in `ria_core::omnibox::quick_create`.

#![forbid(unsafe_code)]

use ria_core::omnibox::quick_create::{self, QuickCreate};
use serde_json::{Value, json};

/// `slice(0, end)` of a list of `len` items: NaN is 0, the rest truncates toward 0, negative counts from the end.
fn slice_end(len: usize, end: f64) -> usize {
    if end.is_nan() {
        return 0;
    }
    let end = end.trunc();
    if end < 0.0 {
        (len as f64 + end).max(0.0) as usize
    } else {
        end.min(len as f64) as usize
    }
}

fn row(c: &QuickCreate) -> Value {
    json!({ "id": c.id, "title": c.title, "url": c.url, "aliases": c.aliases, "site": c.site })
}

/// Port of `matchQuickCreate` (`packages/core/src/quickCreate.ts:29`).
pub fn match_quick_create(query: String, limit: Option<f64>) -> Result<Vec<Value>, String> {
    // The source slices the whole table by `limit`, so the rows are ranked first and cut after.
    let all = quick_create::match_quick_create(&query, usize::MAX);
    let end = slice_end(all.len(), limit.unwrap_or(3.0));
    Ok(all.into_iter().take(end).map(row).collect())
}
