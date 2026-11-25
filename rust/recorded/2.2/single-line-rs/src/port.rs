// Ported from ardia apps/browser/src/lib/singleLine.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::single_line`. The logic is in `ria_state::shared::single_line`.

#![forbid(unsafe_code)]

use ria_state::shared::single_line as s;
use serde_json::{Value, json};

/// Port of `singleLine` (`apps/browser/src/lib/singleLine.ts:4`).
pub fn single_line(text: String) -> Result<Value, String> {
    Ok(json!(s::single_line(&text)))
}
