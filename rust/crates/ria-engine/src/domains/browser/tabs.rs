// Ported from ardia packages/ardiacore/ios/TabsModule.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The `tabs` domain. `lastTabPolicy` goes to the engine; the host answers the other calls.
//! The host answers the three test instance calls only when they are registered for a test instance.

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

pub fn route(call: &str, test_instance: bool) -> Option<Route> {
    match call {
        "lastTabPolicy" => Some(passing("ac_tabs_last_tab_policy")),
        c if HOST_CALLS.contains(&c) || (test_instance && TEST_INSTANCE_CALLS.contains(&c)) => {
            Some(Route::Host)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_calls_go_to_the_host_except_for_last_tab_policy() {
        use super::super::route;
        use crate::api::{Call, tabs::*};

        for name in [
            RouteLinkTabs::NAME,
            StripCommand::NAME,
            Strips::NAME,
            ReportCommits::NAME,
            DevBrowserId::NAME,
            DevEvents::NAME,
            DevWindowNumber::NAME,
            DevFocusPage::NAME,
            DevPictureInPicture::NAME,
            DevPostPeekMouseUp::NAME,
            DevScrollZoom::NAME,
        ] {
            assert!(matches!(route(name, false), Some(Route::Host)), "{name}");
        }
        let Some(Route::Engine(policy)) = route(LastTabPolicy::NAME, false) else {
            panic!("lastTabPolicy is an engine call");
        };
        assert_eq!(policy.symbol, "ac_tabs_last_tab_policy");
    }

    #[test]
    fn test_instance_calls_are_not_registered_in_normal_instances() {
        use super::super::route;
        use crate::api::{Call, tabs::*};

        for name in [
            DevPictureInPictureAction::NAME,
            DevPagePhase::NAME,
            DevClosePage::NAME,
        ] {
            assert!(route(name, false).is_none(), "{name}");
            assert!(matches!(route(name, true), Some(Route::Host)), "{name}");
        }
    }
}
