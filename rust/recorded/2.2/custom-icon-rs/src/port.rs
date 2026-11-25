// Ported from ardia apps/browser/src/lib/customIcon.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::custom_icon`. The logic is in `ria_state::shared::custom_icon`.

#![forbid(unsafe_code)]

use ria_state::shared::custom_icon as c;
use serde_json::{Value, json};

/// Port of `symbolIcon` (`apps/browser/src/lib/customIcon.ts:3`).
pub fn symbol_icon(name: String) -> Result<Value, String> {
    Ok(json!(c::symbol_icon(&name)))
}

/// Port of `iconSymbol` (`apps/browser/src/lib/customIcon.ts:5`).
pub fn icon_symbol(icon: Option<String>) -> Result<Option<String>, String> {
    Ok(c::icon_symbol(icon.as_deref()).map(str::to_owned))
}
