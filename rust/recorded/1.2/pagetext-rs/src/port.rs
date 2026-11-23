// Ported from ardia packages/core/src/pageText.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of the pure string functions of `ria-core` `url::page_text`. The rest of
//! `pageText.ts` (`displayUrl`, which needs `urlForDisplay` of slice 1.3, and the cached
//! `PageText` accessors) stays TypeScript until the omnibox moves.

#![forbid(unsafe_code)]

use ria_core::url::page_text;

/// Port of `pageKey` (`packages/core/src/pageText.ts:8`).
pub fn page_key(url: String) -> Result<String, String> {
    Ok(page_text::page_key(&url))
}

/// Port of `withRootSlash` (`packages/core/src/pageText.ts:10`).
pub fn with_root_slash(url: String) -> Result<String, String> {
    Ok(page_text::with_root_slash(&url))
}

/// Port of `logVisits` (`packages/core/src/pageText.ts:29`).
pub fn log_visits(visits: f64) -> Result<f64, String> {
    Ok(page_text::log_visits(visits))
}

/// Port of `hostPart` (`packages/core/src/pageText.ts:64`).
pub fn host_part(shown: String) -> Result<String, String> {
    Ok(page_text::host_part(&shown).to_string())
}
