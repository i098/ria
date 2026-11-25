// Ported from ardia apps/browser/src/lib/linkSource.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::link_source`. The logic is in `ria_state::shared::link_source`.

#![forbid(unsafe_code)]

use ria_state::shared::link_source::{LinkSource, source_matches as matches};
use serde_json::Value;

/// Port of `sourceMatches` (`apps/browser/src/lib/linkSource.ts:3`).
pub fn source_matches(names: Vec<String>, source: Option<Value>) -> Result<bool, String> {
    let field = |s: &Value, key: &str| s.get(key).and_then(Value::as_str).map(str::to_owned);
    let source = source.filter(|s| s.is_object()).map(|s| LinkSource {
        bundle_id: field(&s, "bundleId"),
        app_name: field(&s, "appName"),
    });
    Ok(matches(&names, source.as_ref()))
}
