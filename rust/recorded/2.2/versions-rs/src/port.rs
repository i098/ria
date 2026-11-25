// Ported from ardia apps/browser/src/lib/versions.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::versions`. The logic is in `ria_state::shared::versions`.

#![forbid(unsafe_code)]

use ria_state::shared::versions as v;

/// Port of `compareVersions` (`apps/browser/src/lib/versions.ts:1`).
pub fn compare_versions(a: String, b: String) -> Result<f64, String> {
    Ok(v::compare_versions(&a, &b))
}
