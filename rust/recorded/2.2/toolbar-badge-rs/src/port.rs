// Ported from ardia apps/browser/src/lib/toolbarBadge.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::toolbar_badge`. The logic is in `ria_state::shared::toolbar_badge`.

#![forbid(unsafe_code)]

use crate::js::{as_number, string_units};
use ria_state::shared::toolbar_badge::{ToolbarBadgeValue, toolbar_badge_text as text};
use serde_json::{Value, json};

/// Port of `toolbarBadgeText` (`apps/browser/src/lib/toolbarBadge.ts:5`).
pub fn toolbar_badge_text(value: Value) -> Result<Value, String> {
    if let Some(n) = as_number(&value) {
        return Ok(json!(text(ToolbarBadgeValue::Number(n))));
    }
    match value {
        Value::Null => Err("TypeError: Cannot read properties of null (reading 'length')".into()),
        // An array has a `length`: Ardia gives it back when short, else its first 4 items (its
        // `join(",")` text holds commas, so it is never a count).
        Value::Array(items) => Ok(Value::Array(items.into_iter().take(4).collect())),
        _ => {
            // A lone surrogate becomes U+FFFD: a Rust `String` cannot hold it (see `known-diffs.jsonl`).
            let units = string_units(&value).ok_or("TypeError: value.slice is not a function")?;
            Ok(json!(text(ToolbarBadgeValue::Text(
                &String::from_utf16_lossy(&units)
            ))))
        }
    }
}
