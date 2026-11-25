// Ported from ardia apps/browser/src/lib/emojiData.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::emoji_data`. `emojiData.ts` exports only data, so
//! the harness `emojiData.harness.ts` gives `EMOJI_GROUPS` as the value of `emojiGroups()`.

#![forbid(unsafe_code)]

use ria_state::shared::emoji_data::EMOJI_GROUPS;
use serde_json::{Value, json};

/// Port of `emojiGroups` (`apps/browser/src/lib/emojiData.harness.ts:4`).
pub fn emoji_groups() -> Result<Vec<Value>, String> {
    Ok(EMOJI_GROUPS
        .iter()
        .map(|g| json!({ "title": g.title, "list": g.list }))
        .collect())
}
