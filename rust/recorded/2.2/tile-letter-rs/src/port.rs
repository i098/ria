// Ported from ardia apps/browser/src/lib/tileLetter.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::tile_letter`. The logic is in `ria_state::shared::tile_letter`.

#![forbid(unsafe_code)]

use ria_state::shared::tile_letter as t;

/// Port of `tileLetterOf` (`apps/browser/src/lib/tileLetter.ts:19`).
pub fn tile_letter_of(url: String, title: String) -> Result<String, String> {
    Ok(t::tile_letter_of(&url, &title))
}
