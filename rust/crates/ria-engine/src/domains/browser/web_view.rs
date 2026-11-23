// Ported from ardia packages/ardiacore/ios/WebViewModule.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The `webView` domain. The host answers each call (`ArdiaWebView` settings and counters). The page view class
//! (`ChromiumTabView`) is the host's too.

use serde_json::{Value, json};

use super::Route;
use crate::call::CallError;

pub const HOST_CALLS: &[&str] = &[
    "setDisplayMediaPicker",
    "setPeekModifier",
    "setSplitLinkModifier",
    "setPeekOtherSiteTabs",
    "setSelectionPopover",
    "setBlockAutoplayEverywhere",
    "setCommandScrollZoom",
    "setLittleArdiaOpensNewTabLinks",
    "setPopupsOpenAsTabs",
    "setAutoPictureInPicture",
    "setVideoUpscaling",
    "setVideoUpscalingTestConditions",
    "videoUpscalingState",
    "takeContentDrift",
    "takeFirstPaintHolds",
];

pub fn route(call: &str) -> Option<Route> {
    HOST_CALLS.contains(&call).then_some(Route::Host)
}

/// The conditions of `setVideoUpscalingTestConditions`: each of `lowPower`, `thermalPressure` and `capturing` that is a
/// boolean in the arguments. Another type for one of them is an error.
pub fn upscaling_test_conditions(args: &Value) -> Result<Value, CallError> {
    let mut out = serde_json::Map::new();
    for key in ["lowPower", "thermalPressure", "capturing"] {
        match args.get(key) {
            None | Some(Value::Null) => {}
            Some(Value::Bool(on)) => _ = out.insert(key.into(), json!(on)),
            Some(_) => return Err(CallError::BadArguments),
        }
    }
    Ok(Value::Object(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_view_calls_go_to_the_host() {
        use crate::api::{Call, web_view::*};

        for name in [
            SetDisplayMediaPicker::NAME,
            SetPeekModifier::NAME,
            SetSplitLinkModifier::NAME,
            SetPeekOtherSiteTabs::NAME,
            SetSelectionPopover::NAME,
            SetBlockAutoplayEverywhere::NAME,
            SetCommandScrollZoom::NAME,
            SetLittleArdiaOpensNewTabLinks::NAME,
            SetPopupsOpenAsTabs::NAME,
            SetAutoPictureInPicture::NAME,
            SetVideoUpscaling::NAME,
            SetVideoUpscalingTestConditions::NAME,
            VideoUpscalingState::NAME,
            TakeContentDrift::NAME,
            TakeFirstPaintHolds::NAME,
        ] {
            assert!(
                matches!(super::super::route(name, false), Some(Route::Host)),
                "{name}"
            );
        }
    }

    #[test]
    fn upscaling_conditions_keep_only_booleans() {
        let args = json!({"lowPower": true, "capturing": null, "extra": 1});
        assert_eq!(
            upscaling_test_conditions(&args).unwrap(),
            json!({"lowPower": true})
        );
        assert_eq!(
            upscaling_test_conditions(&json!({"thermalPressure": "x"})),
            Err(CallError::BadArguments)
        );
    }
}
