// Ported from ardia apps/browser/src/lib/sha256.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::sha256`, through the harness `sha256.harness.ts`:
//! a `Uint8Array` cannot cross rust-anything's binary wire, so the bytes cross as an array of
//! numbers both ways. The logic is in `ria_state::shared::sha256`.

#![forbid(unsafe_code)]

use ria_state::shared::sha256 as s;
use serde_json::Value;

/// Port of `sha256` (`apps/browser/src/lib/sha256.harness.ts:4`).
pub fn sha256(text: String) -> Result<Vec<f64>, String> {
    Ok(s::sha256(&text).map(f64::from).to_vec())
}

/// Port of `base64Url` (`apps/browser/src/lib/sha256.harness.ts:9`): the numbers of `bytes`,
/// each kept modulo 256 as `Uint8Array.from` does (`NaN` and infinities are `0`).
pub fn base64_url(bytes: Value) -> Result<String, String> {
    let bytes: Vec<u8> = bytes
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(crate::js::as_number)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        .into_iter()
        .map(|x| {
            if x.is_finite() {
                x.trunc().rem_euclid(256.0) as u8
            } else {
                0
            }
        })
        .collect();
    Ok(s::base64_url(&bytes))
}
