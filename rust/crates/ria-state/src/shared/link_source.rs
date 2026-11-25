// Ported from ardia apps/browser/src/lib/linkSource.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Which app opened a link, and whether a rule names it.
//!
//! Changes from Ardia: none. Case folds with Rust's `to_lowercase`, the full Unicode mapping that JS
//! `toLowerCase` uses; the two can differ where their Unicode versions differ.

/// `LinkSource` (`linkSource.ts:1`); `null` is `None` at the call.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LinkSource {
    pub bundle_id: Option<String>,
    pub app_name: Option<String>,
}

/// `sourceMatches` (`linkSource.ts:3`): no names match any source; else a name equals the bundle id or app
/// name, ignoring case.
pub fn source_matches(names: &[impl AsRef<str>], source: Option<&LinkSource>) -> bool {
    if names.is_empty() {
        return true;
    }
    let given: Vec<String> = source
        .into_iter()
        .flat_map(|s| [&s.bundle_id, &s.app_name])
        .filter_map(|n| n.as_deref().filter(|n| !n.is_empty()))
        .map(str::to_lowercase)
        .collect();
    names
        .iter()
        .any(|n| given.contains(&n.as_ref().to_lowercase()))
}
