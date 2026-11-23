// Ported from ardia packages/ardiacore/ios/ArdiaWebView+PageTab.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use serde_json::Value;

pub fn close_page(close_browser: impl FnOnce()) {
    close_browser();
}

pub fn download_favicon<C>(url: &str, fetch: impl FnOnce(&str, Option<&str>, C), completion: C)
where
    C: FnOnce(Option<Value>),
{
    fetch(url, None, completion);
}
