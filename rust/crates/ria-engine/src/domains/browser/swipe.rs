// Ported from ardia packages/ardiacore/ios/SwipeModule.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The functions of the `ArdiaSwipe` module. They are not in `api.ts`: the page layer calls them by name, and
//! the pager and the swipe area are AppKit code of `ria-host-mac` (slice 3.11). This file holds the names and the
//! rule that the `dev*` functions answer only in a debug build.

/// Functions that act at once.
pub const FUNCTIONS: &[&str] = &["haptic", "pagerState"];

/// Functions that run on the main thread and return a flag.
pub const MAIN_THREAD_FUNCTIONS: &[&str] = &["switchPager", "selectPager", "stepPager"];

/// Functions that answer only in a debug build; a release build answers `null`, or `{"error": "DEV builds only"}`
/// for `devSimulate`.
pub const DEBUG_FUNCTIONS: &[&str] = &["devPagerState", "devSimulate"];

/// The answer of `devSimulate` in a release build.
pub fn release_simulate_answer() -> serde_json::Value {
    serde_json::json!({"error": "DEV builds only"})
}

#[cfg(test)]
mod tests {
    #[test]
    fn release_builds_refuse_simulation() {
        assert_eq!(super::release_simulate_answer()["error"], "DEV builds only");
    }
}
