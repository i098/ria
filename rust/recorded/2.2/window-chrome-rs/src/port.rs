// Ported from ardia apps/browser/src/lib/windowChrome.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::window_chrome`: a chrome crosses as a JS object with
//! camel-case keys; an omitted one is `WINDOW_CHROME`. The logic is in
//! `ria_state::shared::window_chrome`. `onWindowChromeChange` takes a function, which the bridge
//! cannot carry; the unit test in `window_chrome.rs` checks it.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, number_to_string, string_units, undefined};
use ria_state::shared::js::js_number;
use ria_state::shared::window_chrome::{self as c, WindowButton, WindowChrome};
use serde_json::Value;

/// JS `String(v)` of a value from JSON, for `Number([x])`.
fn js_string(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(items) => items.iter().map(js_string).collect::<Vec<_>>().join(","),
        _ if is_undefined(v) => String::new(),
        _ => as_number(v).map_or_else(|| "[object Object]".into(), number_to_string),
    }
}

/// JS `Number(v)`, which `>=`, `-` and `Math.max` apply to a field; a missing field is `NaN`.
fn to_number(v: Option<&Value>) -> f64 {
    match v {
        None => f64::NAN,
        Some(Value::Null) => 0.0,
        Some(Value::Bool(b)) => f64::from(u8::from(*b)),
        Some(Value::String(s)) => js_number(s),
        Some(v @ Value::Array(_)) => js_number(&js_string(v)),
        Some(v) => as_number(v).unwrap_or(f64::NAN),
    }
}

/// `buttons.length`: `Err` when `buttons` is missing or `null` (reading `.length` throws), `NaN`
/// when it has no `length` (a number, a boolean, an object).
fn length_of(v: Option<&Value>) -> Result<f64, String> {
    match v {
        None | Some(Value::Null) => Err(UNDEFINED_LENGTH.into()),
        Some(v) if is_undefined(v) => Err(UNDEFINED_LENGTH.into()),
        Some(Value::Array(items)) => Ok(items.len() as f64),
        Some(v) => Ok(string_units(v).map_or(f64::NAN, |units| units.len() as f64)),
    }
}

/// A button list as long as `length`: the functions read only the length of a list, and Ardia
/// does not check its items.
fn buttons_of(length: &Result<f64, String>) -> Vec<WindowButton> {
    let n = length
        .as_ref()
        .ok()
        .filter(|n| n.is_finite())
        .map_or(0, |n| *n as usize);
    vec![WindowButton::Close; n]
}

/// A chrome argument: `None` is `undefined` (the default `WINDOW_CHROME`). `null` throws at the
/// first read, as in Ardia; each list keeps its `length` for the reads that need it.
struct Chrome {
    chrome: WindowChrome,
    leading: Result<f64, String>,
    trailing: Result<f64, String>,
}

const NULL_READ: &str = "TypeError: Cannot read properties of null";
const UNDEFINED_LENGTH: &str = "TypeError: Cannot read properties of undefined (reading 'length')";

fn chrome_of(v: &Value) -> Result<Option<Chrome>, String> {
    if is_undefined(v) {
        return Ok(None);
    }
    if v.is_null() {
        return Err(NULL_READ.into());
    }
    let leading = length_of(v.get("leadingButtons"));
    let trailing = length_of(v.get("trailingButtons"));
    Ok(Some(Chrome {
        chrome: WindowChrome {
            leading_width: to_number(v.get("leadingWidth")),
            trailing_width: to_number(v.get("trailingWidth")),
            title_bar_height: to_number(v.get("titleBarHeight")),
            leading_buttons: buttons_of(&leading),
            trailing_buttons: buttons_of(&trailing),
        },
        leading,
        trailing,
    }))
}

/// Port of `captionRoom` (`apps/browser/src/lib/windowChrome.ts:6`).
pub fn caption_room(from_right: f64, from_top: Option<f64>, chrome: Value) -> Result<f64, String> {
    let chrome = chrome_of(&chrome)?;
    Ok(c::caption_room(
        from_right,
        from_top,
        chrome.as_ref().map(|c| &c.chrome),
    ))
}

/// Port of `captionClearTop` (`apps/browser/src/lib/windowChrome.ts:11`).
pub fn caption_clear_top(
    top: f64,
    row_height: f64,
    from_right: f64,
    chrome: Value,
) -> Result<f64, String> {
    let chrome = chrome_of(&chrome)?;
    Ok(c::caption_clear_top(
        top,
        row_height,
        from_right,
        chrome.as_ref().map(|c| &c.chrome),
    ))
}

/// Port of `trailingClearance` (`apps/browser/src/lib/windowChrome.ts:16`). Ardia reads the chrome
/// only when `top < controlsBottom`, so a `null` chrome throws only then.
pub fn trailing_clearance(
    top: f64,
    margin: f64,
    controls_bottom: f64,
    chrome: Value,
) -> Result<f64, String> {
    let chrome = match chrome_of(&chrome) {
        Err(_) if top >= controls_bottom || top.is_nan() || controls_bottom.is_nan() => None,
        chrome => chrome?,
    };
    Ok(c::trailing_clearance(
        top,
        margin,
        controls_bottom,
        chrome.as_ref().map(|c| &c.chrome),
    ))
}

/// Port of `trafficLightsDrawn` (`apps/browser/src/lib/windowChrome.ts:22`). Ardia reads
/// `leadingButtons.length` only when `leadingWidth > 0`.
pub fn traffic_lights_drawn(chrome: Value) -> Result<bool, String> {
    let chrome = chrome_of(&chrome)?;
    if let Some(c) = &chrome
        && c.chrome.leading_width > 0.0
    {
        c.leading.clone()?;
    }
    Ok(c::traffic_lights_drawn(chrome.as_ref().map(|c| &c.chrome)))
}

/// Port of `windowControlsDrawn` (`apps/browser/src/lib/windowChrome.ts:24`).
pub fn window_controls_drawn(chrome: Value) -> Result<bool, String> {
    let chrome = chrome_of(&chrome)?;
    if let Some(c) = &chrome {
        // `NaN + n > 0` is false; the lists stand in only for whole lengths.
        if (c.leading.clone()? + c.trailing.clone()?).is_nan() {
            return Ok(false);
        }
    }
    Ok(c::window_controls_drawn(chrome.as_ref().map(|c| &c.chrome)))
}

/// Port of `windowChromeVersion` (`apps/browser/src/lib/windowChrome.ts:29`).
pub fn window_chrome_version() -> Result<f64, String> {
    Ok(c::window_chrome_version())
}

/// Port of `noteWindowChromeChanged` (`apps/browser/src/lib/windowChrome.ts:36`).
pub fn note_window_chrome_changed() -> Result<Value, String> {
    c::note_window_chrome_changed();
    Ok(undefined())
}
