// Ported from ardia apps/browser/src/lib/platformChrome.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::platform_chrome` (the Mac build). The logic is in
//! `ria_state::shared::platform_chrome`.

#![forbid(unsafe_code)]

use ria_state::shared::platform_chrome::{self as p, WindowButtonStyle};
use serde_json::Value;

/// Port of `applyWindowButtonStyle` (`apps/browser/src/lib/platformChrome.ts:22`). The Mac build
/// reads neither argument, as Ardia's does not: a style outside the type and any layout give
/// `false` there, so they cross as `Native` and no layout.
pub fn apply_window_button_style(style: String, layout: Option<Value>) -> Result<bool, String> {
    let _ = layout;
    let style = serde_json::from_value(Value::String(style)).unwrap_or(WindowButtonStyle::Native);
    Ok(p::apply_window_button_style(style, None))
}
