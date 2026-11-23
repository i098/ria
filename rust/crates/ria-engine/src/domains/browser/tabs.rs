// Ported from ardia packages/ardiacore/ios/TabsModule.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The `tabs` domain. `lastTabPolicy` goes to the engine; the host answers the other calls.
//! The test instance calls (`devPictureInPictureAction`, `devPagePhase`, `devClosePage`) are in the list too: the host
//! refuses them when the instance is not a test instance (`LaunchConfig.shared.testInstance`).

use super::{Route, passing};

/// Calls that the host answers.
pub const HOST_CALLS: &[&str] = &[
    "routeLinkTabs",
    "stripCommand",
    "strips",
    "reportCommits",
    "devBrowserId",
    "devEvents",
    "devWindowNumber",
    "devFocusPage",
    "devPictureInPicture",
    "devPostPeekMouseUp",
    "devScrollZoom",
];

/// Host calls that only a test instance answers.
pub const TEST_INSTANCE_CALLS: &[&str] =
    &["devPictureInPictureAction", "devPagePhase", "devClosePage"];

/// The functions that `TabsModule.swift` exports to the page layer by name, not through the call table.
/// They act on the page views of the host.
pub const HOST_FUNCTIONS: &[&str] = &[
    "prepareTransfer",
    "releaseTransfer",
    "claimLaunchTab",
    "revealTab",
    "dropTabReveals",
    "claimTabPress",
    "dropTabPress",
    "setTabsInSplits",
    "setNativeTabSwitch",
    "revealOffered",
    "discardOffered",
];

pub fn route(call: &str) -> Option<Route> {
    match call {
        "lastTabPolicy" => Some(passing("ac_tabs_last_tab_policy")),
        c if HOST_CALLS.contains(&c) || TEST_INSTANCE_CALLS.contains(&c) => Some(Route::Host),
        _ => None,
    }
}

/// Whether the call is answered only when the instance is a test instance.
pub fn is_test_instance_call(call: &str) -> bool {
    TEST_INSTANCE_CALLS.contains(&call)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_call_of_the_api_file_has_a_route() {
        super::super::assert_routes_cover("tabs", include_str!("../../api/tabs.rs"));
    }

    #[test]
    fn last_tab_policy_passes_to_its_symbol_and_the_strip_topic_keeps_both_names() {
        let Some(Route::Engine(route)) = route("lastTabPolicy") else {
            panic!("lastTabPolicy is an engine call");
        };
        assert_eq!(route.symbol, "ac_tabs_last_tab_policy");
        assert!(is_test_instance_call("devClosePage") && !is_test_instance_call("strips"));
        assert_eq!(super::super::event_name("tabs.strip"), Some("tabs.strips"));
        assert_eq!(super::super::event_name("tabs.strips"), None);
    }
}
