// Ported from ardia packages/core/src/cleanUrl.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `url::clean_url`. The logic is in `ria_core::url::clean_url`.

#![forbid(unsafe_code)]

use ria_core::url::clean_url as core;
use serde_json::{Value, json};

/// Port of `cleanUrlReport` (`packages/core/src/cleanUrl.ts:52`).
pub fn clean_url_report(url: String) -> Result<Value, String> {
    let report = core::clean_url_report(&url);
    Ok(json!({"url": report.url, "removedTracker": report.removed_tracker}))
}

/// Port of `cleanUrl` (`packages/core/src/cleanUrl.ts:72`).
pub fn clean_url(url: String) -> Result<String, String> {
    Ok(core::clean_url(&url))
}

/// Port of `markdownLink` (`packages/core/src/cleanUrl.ts:76`).
pub fn markdown_link(title: String, url: String) -> Result<String, String> {
    Ok(core::markdown_link(&title, &url))
}
