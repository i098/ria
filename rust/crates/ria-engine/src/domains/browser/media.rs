// Ported from ardia packages/ardiacore/ios/MediaModule.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The `media` domain. Each call is answered by the host (`ArdiaHost` of Ardia): screen capture sources, device
//! chooser, cast dialog and capture control. None goes to an engine symbol.

use super::Route;

pub const HOST_CALLS: &[&str] = &[
    "displayMediaSources",
    "displayMediaThumbnails",
    "screenRecordingAllowed",
    "requestScreenRecording",
    "selectDevice",
    "cancelDeviceChooser",
    "refreshDeviceChooser",
    "openBluetoothSettings",
    "devShowBluetoothChooser",
    "devChooserEvents",
    "showCastDialog",
    "startCasting",
    "stopCasting",
    "closeCastDialog",
    "watchCastRoutes",
    "terminateCastRoute",
    "changeCaptureSource",
    "stopCapture",
];

pub fn route(call: &str) -> Option<Route> {
    HOST_CALLS.contains(&call).then_some(Route::Host)
}

/// The action that the device chooser calls of `MediaModule.swift` send to the host.
pub fn chooser_action(call: &str) -> Option<&'static str> {
    match call {
        "cancelDeviceChooser" => Some("cancel"),
        "refreshDeviceChooser" => Some("refresh"),
        "openBluetoothSettings" => Some("settings"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_call_of_the_api_file_has_a_host_route() {
        super::super::assert_routes_cover("media", include_str!("../../api/media.rs"));
        assert!(matches!(route("selectDevice"), Some(Route::Host)));
    }

    #[test]
    fn chooser_calls_map_to_their_actions() {
        assert_eq!(chooser_action("cancelDeviceChooser"), Some("cancel"));
        assert_eq!(chooser_action("refreshDeviceChooser"), Some("refresh"));
        assert_eq!(chooser_action("openBluetoothSettings"), Some("settings"));
        assert_eq!(chooser_action("selectDevice"), None);
    }
}
