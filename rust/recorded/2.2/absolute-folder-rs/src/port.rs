// Ported from ardia apps/browser/src/lib/absoluteFolder.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::absolute_folder`. The logic is in `ria_state::shared::absolute_folder`.

#![forbid(unsafe_code)]

use ria_state::shared::absolute_folder::is_absolute_folder as is_absolute;
use serde_json::{Value, json};

/// Port of `isAbsoluteFolder` (`apps/browser/src/lib/absoluteFolder.ts:1`).
pub fn is_absolute_folder(path: String) -> Result<Value, String> {
    Ok(json!(is_absolute(&path)))
}
