// Ported from ardia apps/browser/src/lib/legacy.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::legacy`, through `legacy.harness.ts`: `fromLegacyValue`
//! gives `[fromLegacy(value), result === value]`. The logic is in `ria_state::shared::legacy`.

#![forbid(unsafe_code)]

use ria_state::shared::legacy as l;
use serde_json::{Value, json};

/// Port of `fromLegacyUrl` (`apps/browser/src/lib/legacy.ts:16`).
pub fn from_legacy_url(url: String) -> Result<String, String> {
    Ok(l::from_legacy_url(&url))
}

/// Port of `fromLegacyValue` (`apps/browser/src/lib/legacy.harness.ts:8`), which calls `fromLegacy` (`legacy.ts:29`).
pub fn from_legacy_value(value: Value) -> Result<Value, String> {
    Ok(match l::from_legacy(&value) {
        Some(out) => json!([out, false]),
        // `NaN === NaN` is false.
        None => {
            let nan = value.get("$num").and_then(Value::as_str) == Some("NaN");
            json!([value, !nan])
        }
    })
}
