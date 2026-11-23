// Ported from ardia packages/ardiacore/ios/WindowsModule.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The `windows` domain. The host answers both calls (`ArdiaHost.chromeWindows`, `ArdiaChromeWindowHost.devAction`).

use super::Route;

pub const HOST_CALLS: &[&str] = &["list", "dev"];

pub fn route(call: &str) -> Option<Route> {
    HOST_CALLS.contains(&call).then_some(Route::Host)
}

#[cfg(test)]
mod tests {
    #[test]
    fn windows_calls_go_to_the_host() {
        use crate::api::{Call, windows::*};

        for name in [List::NAME, Dev::NAME] {
            assert!(
                matches!(super::super::route(name, false), Some(super::Route::Host)),
                "{name}"
            );
        }
    }
}
