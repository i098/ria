// Ported from ardia packages/core/src/anonymousIdentity.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `identity::anonymous_identity`. The logic is in `ria_core`.

#![forbid(unsafe_code)]

use ria_core::identity::anonymous_identity as identity;
use serde_json::json;

/// Port of `hslHex` (`packages/core/src/anonymousIdentity.ts:64`).
pub fn hsl_hex(hue: f64, saturation: f64, lightness: f64) -> Result<String, String> {
    Ok(identity::hsl_hex(hue, saturation, lightness))
}

/// Port of `anonymousIdentity` (`packages/core/src/anonymousIdentity.ts:80`).
pub fn anonymous_identity(id: String) -> Result<serde_json::Value, String> {
    let found = identity::anonymous_identity(&id);
    Ok(json!({
        "name": found.name,
        "avatar": { "colors": found.avatar.colors, "angle": found.avatar.angle },
    }))
}
