// Ported from ardia apps/browser/src/lib/pluginNames.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_names`. `isWellFormed` takes a JSON value, so
//! a string with lone surrogates reaches it as UTF-16 code units. The logic is in `ria_state`.

#![forbid(unsafe_code)]

use crate::js::string_units;
use ria_state::shared::plugin_names as n;
use serde_json::{Value, json};

/// Port of `isWellFormed` (`apps/browser/src/lib/pluginNames.ts:19`).
pub fn is_well_formed(text: Value) -> Result<bool, String> {
    let units = string_units(&text).ok_or("TypeError: text is not a string")?;
    Ok(n::is_well_formed(&units))
}

/// Port of `isPagePath` (`apps/browser/src/lib/pluginNames.ts:31`).
pub fn is_page_path(path: String) -> Result<Value, String> {
    Ok(json!(n::is_page_path(&path)))
}
