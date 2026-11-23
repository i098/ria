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
    fn media_calls_go_to_the_host() {
        use crate::api::{Call, media::*};

        for name in [
            DisplayMediaSources::NAME,
            DisplayMediaThumbnails::NAME,
            ScreenRecordingAllowed::NAME,
            RequestScreenRecording::NAME,
            SelectDevice::NAME,
            CancelDeviceChooser::NAME,
            RefreshDeviceChooser::NAME,
            OpenBluetoothSettings::NAME,
            DevShowBluetoothChooser::NAME,
            DevChooserEvents::NAME,
            ShowCastDialog::NAME,
            StartCasting::NAME,
            StopCasting::NAME,
            CloseCastDialog::NAME,
            WatchCastRoutes::NAME,
            TerminateCastRoute::NAME,
            ChangeCaptureSource::NAME,
            StopCapture::NAME,
        ] {
            assert!(
                matches!(super::super::route(name, false), Some(Route::Host)),
                "{name}"
            );
        }
    }

    #[test]
    fn chooser_calls_map_to_their_actions() {
        assert_eq!(chooser_action("cancelDeviceChooser"), Some("cancel"));
        assert_eq!(chooser_action("refreshDeviceChooser"), Some("refresh"));
        assert_eq!(chooser_action("openBluetoothSettings"), Some("settings"));
        assert_eq!(chooser_action("selectDevice"), None);
    }
}
