// Ported from ardia apps/browser/src/lib/webloc.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::webloc`, through `webloc.harness.ts`.
//! The logic is in `ria_state::shared::webloc`.

#![forbid(unsafe_code)]

use ria_state::shared::webloc as w;
use serde_json::{Value, json};

/// Port of `weblocText` (`apps/browser/src/lib/webloc.ts:14`).
pub fn webloc_text(url: String, id: Option<String>) -> Result<Value, String> {
    Ok(json!(w::webloc_text(&url, id.as_deref())))
}

/// Port of `parseWebloc` (`apps/browser/src/lib/webloc.ts:35`).
pub fn parse_webloc(text: String) -> Result<Value, String> {
    Ok(json!(w::parse_webloc(&text)))
}

/// Port of `weblocLinkId` (`apps/browser/src/lib/webloc.ts:37`).
pub fn webloc_link_id(text: String) -> Result<Value, String> {
    Ok(json!(w::webloc_link_id(&text)))
}

/// Shape of `newLinkId` (`apps/browser/src/lib/webloc.ts:39`): 9 or more base-36 digits, new each call.
pub fn new_link_id_shape() -> Result<bool, String> {
    let id = w::new_link_id();
    Ok(id.len() >= 9
        && id
            .bytes()
            .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase())
        && id != w::new_link_id())
}
