// Ported from ardia packages/core/src/omnibox.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `omnibox::omnibox`: it maps the JSON values of the recording
//! to typed values. The logic is in `ria_core::omnibox::omnibox`.

#![forbid(unsafe_code)]

use ria_core::omnibox::omnibox::{self, PasteAction};
use serde_json::{Value, json};

/// Port of `hostOf` (`packages/core/src/omnibox.ts:16`).
pub fn host_of(url: String) -> Result<String, String> {
    Ok(omnibox::host_of(&url))
}

/// Port of `isJavaScriptUrl` (`packages/core/src/omnibox.ts:65`).
pub fn is_java_script_url(url: String) -> Result<Value, String> {
    Ok(Value::Bool(omnibox::is_javascript_url(&url)))
}

/// Port of `withoutJavaScriptScheme` (`packages/core/src/omnibox.ts:67`).
pub fn without_java_script_scheme(text: String) -> Result<String, String> {
    Ok(omnibox::without_javascript_scheme(&text).to_string())
}

/// Port of `fixupUrl` (`packages/core/src/omnibox.ts:73`).
pub fn fixup_url(raw: String) -> Result<Option<String>, String> {
    Ok(omnibox::fixup_url(&raw))
}

/// Port of `sameAddress` (`packages/core/src/omnibox.ts:116`).
pub fn same_address(a: String, b: String) -> Result<Value, String> {
    Ok(Value::Bool(omnibox::same_address(&a, &b)))
}

/// Port of `spacedAddressGuess` (`packages/core/src/omnibox.ts:118`).
pub fn spaced_address_guess(raw: String) -> Result<Option<String>, String> {
    Ok(omnibox::spaced_address_guess(&raw))
}

/// Port of `searchUrlFor` (`packages/core/src/omnibox.ts:161`).
pub fn search_url_for(template: String, query: String) -> Result<String, String> {
    Ok(omnibox::search_url_for(&template, &query))
}

/// Port of `resolveInput` (`packages/core/src/omnibox.ts:166`).
pub fn resolve_input(raw: String, search: String) -> Result<String, String> {
    Ok(omnibox::resolve_input(&raw, &search))
}

/// Port of `classifyPaste` (`packages/core/src/omnibox.ts:175`).
pub fn classify_paste(text: String) -> Result<Option<Value>, String> {
    Ok(omnibox::classify_paste(&text).map(|action| match action {
        PasteAction::Go { url } => json!({"kind": "go", "url": url}),
        PasteAction::Search { query } => json!({"kind": "search", "query": query}),
    }))
}

/// Port of `sanitizePaste` (`packages/core/src/omnibox.ts:183`).
pub fn sanitize_paste(text: String) -> Result<String, String> {
    Ok(omnibox::sanitize_paste(&text))
}

/// Port of `breadcrumb` (`packages/core/src/omnibox.ts:193`).
pub fn breadcrumb(url: String, title: Option<String>) -> Result<Value, String> {
    let crumb =
        omnibox::breadcrumb(&url, title.as_deref()).map_err(|e| format!("TypeError: {e}"))?;
    Ok(json!({"host": crumb.host, "trail": crumb.trail}))
}

/// Port of `urlForDisplay` (`packages/core/src/omnibox.ts:211`).
pub fn url_for_display(url: String) -> Result<String, String> {
    omnibox::url_for_display(&url).map_err(|e| format!("TypeError: {e}"))
}
