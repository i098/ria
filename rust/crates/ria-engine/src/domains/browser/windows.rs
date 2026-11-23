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
    fn each_call_of_the_api_file_has_a_host_route() {
        super::super::assert_routes_cover("windows", include_str!("../../api/windows.rs"));
    }
}
