// Ported from ardia apps/browser/src/lib/windowButtonLayout.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::window_button_layout`: layouts cross as
//! `{ leading, trailing }` arrays of button strings. The logic is in
//! `ria_state::shared::window_button_layout`.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, number, string_units};
use ria_state::shared::window_button_layout::{
    self as l, WindowButton, WindowButtonLayout, WindowButtonsAllowed,
};
use serde_json::{Value, json};

/// JS truthiness of a value from JSON.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        _ => as_number(v).map_or(!is_undefined(v), |n| n != 0.0 && !n.is_nan()),
    }
}

/// A button list of a JS layout; anything but an array throws at `.filter`, as in Ardia.
fn buttons_of(v: Option<&Value>, side: &str) -> Result<Vec<WindowButton>, String> {
    let Some(Value::Array(items)) = v else {
        return Err(format!("TypeError: {side}.filter is not a function"));
    };
    items
        .iter()
        .map(|item| {
            serde_json::from_value(item.clone())
                .map_err(|_| format!("TypeError: {item} is not a WindowButton"))
        })
        .collect()
}

fn layout_of(v: &Value) -> Result<WindowButtonLayout, String> {
    if v.is_null() || is_undefined(v) {
        return Err("TypeError: Cannot read properties of null (reading 'leading')".into());
    }
    Ok(WindowButtonLayout {
        leading: buttons_of(v.get("leading"), "leading")?,
        trailing: buttons_of(v.get("trailing"), "trailing")?,
    })
}

/// Port of `windowControlsWidth` (`apps/browser/src/lib/windowButtonLayout.ts:9`).
pub fn window_controls_width(count: Option<f64>) -> Result<Value, String> {
    Ok(number(l::window_controls_width(count)))
}

/// Port of `parseButtonLayout` (`apps/browser/src/lib/windowButtonLayout.ts:17`).
pub fn parse_button_layout(raw: Option<Value>) -> Result<Value, String> {
    let text = raw
        .as_ref()
        .and_then(string_units)
        .map(|units| String::from_utf16_lossy(&units));
    let layout = l::parse_button_layout(text.as_deref());
    Ok(json!({ "leading": layout.leading, "trailing": layout.trailing }))
}

/// Port of `windowButtonsFor` (`apps/browser/src/lib/windowButtonLayout.ts:37`).
pub fn window_buttons_for(buttons: Vec<String>, allowed: Value) -> Result<Vec<String>, String> {
    // Destructuring `null` or `undefined` throws; any other value reads its fields.
    if allowed.is_null() || is_undefined(&allowed) {
        return Err("TypeError: Cannot destructure the window buttons allowed".into());
    }
    let flag = |name: &str| allowed.get(name).filter(|v| !is_undefined(v)).map(truthy);
    let allowed = WindowButtonsAllowed {
        menu: flag("menu"),
        minimizable: flag("minimizable"),
        maximizable: flag("maximizable"),
    };
    Ok(l::window_buttons_for(&buttons, &allowed))
}

/// Port of `buttonLayoutSide` (`apps/browser/src/lib/windowButtonLayout.ts:42`).
pub fn button_layout_side(layout: Value) -> Result<Value, String> {
    Ok(json!(l::button_layout_side(&layout_of(&layout)?)))
}

/// Port of `buttonLayoutCount` (`apps/browser/src/lib/windowButtonLayout.ts:48`).
pub fn button_layout_count(layout: Value) -> Result<f64, String> {
    Ok(l::button_layout_count(&layout_of(&layout)?) as f64)
}
