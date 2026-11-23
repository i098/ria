// Ported from ardia packages/core/src/ingestKeys.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `identity::ingest_keys`. The logic is in `ria_core`.

#![forbid(unsafe_code)]

use ria_core::identity::ingest_keys;

/// Port of `ingestSourceOf` (`packages/core/src/ingestKeys.ts:15`).
pub fn ingest_source_of(key: Option<String>) -> Result<Option<String>, String> {
    Ok(ingest_keys::ingest_source_of(key.as_deref()).map(|s| s.as_str().to_owned()))
}
