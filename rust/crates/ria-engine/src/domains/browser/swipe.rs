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

pub fn debug_reply(call: &str, run: impl FnOnce() -> serde_json::Value) -> Option<serde_json::Value> {
    if !DEBUG_FUNCTIONS.contains(&call) {
        return None;
    }
    Some(if cfg!(debug_assertions) {
        run()
    } else if call == "devSimulate" {
        serde_json::json!({"error": "DEV builds only"})
    } else {
        serde_json::Value::Null
    })
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    #[test]
    fn development_calls_follow_the_build_mode() {
        for (name, release) in [
            ("devPagerState", Value::Null),
            ("devSimulate", json!({"error": "DEV builds only"})),
        ] {
            let ran = std::cell::Cell::new(false);
            let result = super::debug_reply(name, || {
                ran.set(true);
                json!({"ran": true})
            });
            if cfg!(debug_assertions) {
                assert_eq!(result, Some(json!({"ran": true})));
                assert!(ran.get());
            } else {
                assert_eq!(result, Some(release));
                assert!(!ran.get());
            }
        }
    }
}
