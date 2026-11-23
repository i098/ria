// Ported from ardia packages/core/src/appUrls.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `url::app_urls`: it maps the JSON values of the recording
//! to typed values. The logic is in `ria_core::url::app_urls`.

#![forbid(unsafe_code)]

use ria_core::url::app_urls::{self, AppUrlRoute};
use serde_json::{Value, json};

/// Port of `toAppUrl` (`packages/core/src/appUrls.ts:19`).
pub fn to_app_url(url: String) -> Result<String, String> {
    Ok(app_urls::to_app_url(&url))
}

/// Port of `toEngineUrl` (`packages/core/src/appUrls.ts:27`).
pub fn to_engine_url(url: String) -> Result<String, String> {
    Ok(app_urls::to_engine_url(&url))
}

/// Port of `appUrlOrigin` (`packages/core/src/appUrls.ts:41`).
pub fn app_url_origin(url: String) -> Result<Option<String>, String> {
    app_urls::app_url_origin(&url).map_err(|e| format!("TypeError: {e}"))
}

/// Port of `appUrlRoute` (`packages/core/src/appUrls.ts:52`).
pub fn app_url_route(url: String) -> Result<Option<Value>, String> {
    Ok(app_urls::app_url_route(&url)
        .map_err(|e| format!("TypeError: {e}"))?
        .map(|route| match route {
            AppUrlRoute::Page(page) => json!({"kind": "page", "page": page.as_str()}),
            AppUrlRoute::Settings { section } => json!({"kind": "settings", "section": section}),
            AppUrlRoute::NewTab => json!({"kind": "newTab"}),
            AppUrlRoute::Engine { url } => json!({"kind": "engine", "url": url}),
        }))
}
