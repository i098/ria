// Ported from ardia apps/browser/src/lib/pluginNames.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Names a plugin page cannot take, and the check of a plugin page path.
//!
//! Changes from Ardia:
//! - `isWellFormed` reads UTF-16 code units (`&[u16]`), as the TS string is. A Rust `&str` cannot
//!   hold a lone surrogate, so `isPagePath` skips that check: it is always true for a `&str`.
//! - `path.length` counts UTF-16 code units, as in JS.
//! - `\s` in `PAGE_PATH` is the JS whitespace set, written out (Rust's `\s` lacks U+FEFF and has U+0085).

use regex::Regex;
use std::sync::LazyLock;

/// `RESERVED_PAGE_NAMES` (`pluginNames.ts:1`).
pub const RESERVED_PAGE_NAMES: &[&str] = &[
    "history",
    "bookmarks",
    "downloads",
    "newtab",
    "settings",
    "about",
    "accessibility",
    "app-service-internals",
    "apps",
    "attribution-internals",
    "autofill-internals",
    "blob-internals",
    "bluetooth-internals",
    "chrome",
    "chrome-urls",
    "components",
    "connectors-internals",
    "crashes",
    "credits",
    "device-log",
    "dino",
    "discards",
    "download-internals",
    "extensions",
    "extensions-internals",
    "flags",
    "gcm-internals",
    "gpu",
    "help",
    "histograms",
    "history-clusters-internals",
    "indexeddb-internals",
    "inspect",
    "interstitials",
    "invalidations",
    "local-state",
    "management",
    "media-engagement",
    "media-internals",
    "memory-internals",
    "metrics-internals",
    "net-export",
    "net-internals",
    "network-errors",
    "new-tab-page",
    "ntp-tiles-internals",
    "omnibox",
    "optimization-guide-internals",
    "password-manager",
    "password-manager-internals",
    "policy",
    "predictors",
    "prefs-internals",
    "print",
    "process-internals",
    "quota-internals",
    "reset-password",
    "safe-browsing",
    "serviceworker-internals",
    "signin-internals",
    "site-engagement",
    "sync-internals",
    "system",
    "terms",
    "topics-internals",
    "tracing",
    "translate-internals",
    "ukm",
    "usb-internals",
    "user-actions",
    "version",
    "web-app-internals",
    "webrtc-internals",
    "webrtc-logs",
    "webui-gallery",
    "whats-new",
];

static PAGE_PATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(/[A-Za-z0-9._~!$&'()*+,;=:@%-]*)*(\?[^#\t\n\x0B\f\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]*)?$",
    )
    .expect("static pattern")
});

/// `MAX_PAGE_PATH` (`pluginNames.ts:17`).
pub const MAX_PAGE_PATH: usize = 512;

/// `isWellFormed` (`pluginNames.ts:19`): no lone surrogate in the UTF-16 code units.
pub fn is_well_formed(text: &[u16]) -> bool {
    let mut i = 0;
    while i < text.len() {
        let unit = text[i];
        if (0xd800..=0xdbff).contains(&unit) {
            if !text
                .get(i + 1)
                .is_some_and(|n| (0xdc00..=0xdfff).contains(n))
            {
                return false;
            }
            i += 1;
        } else if (0xdc00..=0xdfff).contains(&unit) {
            return false;
        }
        i += 1;
    }
    true
}

/// `isPagePath` (`pluginNames.ts:31`).
pub fn is_page_path(path: &str) -> bool {
    path.encode_utf16().count() <= MAX_PAGE_PATH
        && (path.is_empty() || PAGE_PATH.is_match(path))
        && !path.split(['/', '?']).any(|part| part == "..")
}
