// Ported from ardia apps/browser/src/lib/linkWrappers.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::link_wrappers`. The logic is in `ria_state::shared::link_wrappers`.

#![forbid(unsafe_code)]

use ria_state::shared::link_wrappers as l;

/// Port of `linkDestination` (`apps/browser/src/lib/linkWrappers.ts:47`).
pub fn link_destination(url: String) -> Result<String, String> {
    Ok(l::link_destination(&url))
}
