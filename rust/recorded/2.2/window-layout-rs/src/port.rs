// Ported from ardia apps/browser/src/lib/windowLayout.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::window_layout`: object fields are read as JS truth
//! values and numbers cross as JS numbers. The logic is in `ria_state::shared::window_layout`.
//! Arguments outside the TS types (for example a `color` that is not a colour name) are not covered:
//! the Rust side takes typed values, so the crate records with `--count 0` and only in-type inputs,
//! with the `null` and `undefined` cases where the TS really throws.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, number, string_units};
use ria_core::identity::telemetry_wire::Json;
use ria_state::shared::window_layout as w;
use serde_json::{Value, json};

/// The `TypeError` of `v[key]` when `v` is `null` or `undefined`.
fn readable(v: &Value, key: &str) -> Result<(), String> {
    let what = match v {
        Value::Null => "null",
        _ if is_undefined(v) => "undefined",
        _ => return Ok(()),
    };
    Err(format!(
        "TypeError: Cannot read properties of {what} (reading '{key}')"
    ))
}

/// A JS value as `Json`: an `undefined` member is left out, an `undefined` item is `null`.
fn json_of(v: &Value) -> Json {
    if let Some(n) = as_number(v) {
        return Json::Number(n);
    }
    if let Some(units) = string_units(v) {
        return Json::String(String::from_utf16_lossy(&units));
    }
    match v {
        Value::Bool(b) => Json::Bool(*b),
        Value::Array(items) => Json::Array(items.iter().map(json_of).collect()),
        Value::Object(o) if !is_undefined(v) => Json::Object(
            o.iter()
                .filter(|(_, v)| !is_undefined(v))
                .map(|(k, v)| (k.clone(), json_of(v)))
                .collect(),
        ),
        _ => Json::Null,
    }
}

/// `!!v[key]`.
fn truthy(v: &Value, key: &str) -> bool {
    match v.get(key) {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => !s.is_empty(),
        Some(x) => as_number(x).map_or(!is_undefined(x), |n| n != 0.0 && !n.is_nan()),
    }
}

/// Port of `sidebarWidthLimit` (`apps/browser/src/lib/windowLayout.ts:10`).
pub fn sidebar_width_limit(window_width: Option<f64>) -> Result<f64, String> {
    Ok(w::sidebar_width_limit(window_width))
}

/// Port of `shownSidebarWidth` (`apps/browser/src/lib/windowLayout.ts:13`).
pub fn shown_sidebar_width(saved: f64, window_width: Option<f64>) -> Result<f64, String> {
    Ok(w::shown_sidebar_width(saved, window_width))
}

/// Port of `validPageFrame` (`apps/browser/src/lib/windowLayout.ts:22`).
pub fn valid_page_frame(v: serde_json::Value) -> Result<bool, String> {
    Ok(w::valid_page_frame(&json_of(&v)))
}

/// Port of `pageFrameInsetOf` (`apps/browser/src/lib/windowLayout.ts:24`).
pub fn page_frame_inset_of(inset: f64, opts: serde_json::Value) -> Result<f64, String> {
    readable(&opts, "borderlessMode")?;
    let opts = w::PageFrameOpts {
        borderless_mode: truthy(&opts, "borderlessMode"),
        zero_in_full_screen: truthy(&opts, "zeroInFullScreen"),
        full_screen: truthy(&opts, "fullScreen"),
        zero_when_tabs_hidden: truthy(&opts, "zeroWhenTabsHidden"),
        tabs_hidden: truthy(&opts, "tabsHidden"),
    };
    Ok(w::page_frame_inset_of(inset, &opts))
}

/// Port of `pageFrameInset` (`apps/browser/src/lib/windowLayout.ts:40`).
pub fn page_frame_inset(settings: serde_json::Value) -> Result<serde_json::Value, String> {
    readable(&settings, "pageFrame")?;
    let page_frame = settings
        .get("pageFrame")
        .and_then(as_number)
        .unwrap_or(f64::NAN);
    let settings = w::PageFrameSettings {
        page_frame,
        borderless_mode: truthy(&settings, "borderlessMode"),
    };
    Ok(number(w::page_frame_inset(&settings)))
}

/// Port of `pageFrameOf` (`apps/browser/src/lib/windowLayout.ts:56`).
pub fn page_frame_of(
    inset: f64,
    window_radius: f64,
    drawn_window_radius: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // A default parameter: `undefined` (the bridge sends it as a JSON value) takes `windowRadius`.
    let f = w::page_frame_of(inset, window_radius, as_number(&drawn_window_radius));
    Ok(json!({
        "inset": number(f.inset),
        "radius": number(f.radius),
        "splitGap": number(f.split_gap),
        "peek": {
            "left": number(f.peek.left),
            "top": number(f.peek.top),
            "bottom": number(f.peek.bottom),
            "radius": number(f.peek.radius),
        },
    }))
}
