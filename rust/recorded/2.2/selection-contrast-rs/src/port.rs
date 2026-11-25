// Ported from ardia apps/browser/src/lib/selectionContrast.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::selection_contrast`. The logic is in
//! `ria_state::shared::selection_contrast`.
//! Arguments outside the TS types (for example a `color` that is not a colour name) are not covered:
//! the Rust side takes typed values, so the crate records with `--count 0` and only in-type inputs,
//! with the `null` and `undefined` cases where the TS really throws.

#![forbid(unsafe_code)]

use ria_state::shared::selection_contrast as s;

/// Port of `selectionLightnessDelta` (`apps/browser/src/lib/selectionContrast.ts:15`).
pub fn selection_lightness_delta(fill: String, backdrops: Vec<String>) -> Result<f64, String> {
    Ok(s::selection_lightness_delta(&fill, &backdrops))
}

/// Port of `visibleSelection` (`apps/browser/src/lib/selectionContrast.ts:34`).
pub fn visible_selection(
    fill: String,
    fallback: String,
    backdrops: Vec<String>,
) -> Result<String, String> {
    Ok(s::visible_selection(&fill, &fallback, &backdrops))
}

/// Port of `visibleLine` (`apps/browser/src/lib/selectionContrast.ts:36`).
pub fn visible_line(line: String, ink: String, backdrops: Vec<String>) -> Result<String, String> {
    Ok(s::visible_line(&line, &ink, &backdrops))
}
