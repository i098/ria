// Ported from ardia apps/browser/src/lib/toolbarRow.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::toolbar_row`: rows cross as JS objects with
//! camel-case keys. The logic is in `ria_state::shared::toolbar_row`.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, number, undefined};
use ria_state::shared::toolbar_row::{self as r, ToolbarRow, ToolbarTrailingOptions};
use serde_json::{Value, json};

/// A number field of a JS object; a missing one is `undefined`, which arithmetic turns into `NaN`.
fn field(o: &Value, name: &str) -> f64 {
    o.get(name).and_then(as_number).unwrap_or(f64::NAN)
}

/// The row of a JS call; `null` throws as Ardia's first property read does.
fn row_from(v: &Value) -> Result<ToolbarRow, String> {
    if v.is_null() || is_undefined(v) {
        return Err("TypeError: Cannot read properties of null".into());
    }
    Ok(ToolbarRow {
        height: field(v, "height"),
        center: field(v, "center"),
        button: field(v, "button"),
        button_radius: field(v, "buttonRadius"),
        field: field(v, "field"),
        field_radius: field(v, "fieldRadius"),
        field_button: field(v, "fieldButton"),
        field_button_radius: field(v, "fieldButtonRadius"),
        extension_button: field(v, "extensionButton"),
    })
}

fn row_to(row: &ToolbarRow) -> Value {
    json!({
        "height": number(row.height),
        "center": number(row.center),
        "button": number(row.button),
        "buttonRadius": number(row.button_radius),
        "field": number(row.field),
        "fieldRadius": number(row.field_radius),
        "fieldButton": number(row.field_button),
        "fieldButtonRadius": number(row.field_button_radius),
        "extensionButton": number(row.extension_button),
    })
}

/// Port of `toolbarRowOf` (`apps/browser/src/lib/toolbarRow.ts:18`).
pub fn toolbar_row_of(compact: bool) -> Result<Value, String> {
    Ok(row_to(&r::toolbar_row_of(compact)))
}

/// Port of `urlFieldOf` (`apps/browser/src/lib/toolbarRow.ts:20`).
pub fn url_field_of(row: Value) -> Result<Value, String> {
    let f = r::url_field_of(&row_from(&row)?);
    // `height` is `row.field` as it is: `undefined` when the row has none.
    let height = if row.get("field").is_none() {
        undefined()
    } else {
        number(f.height)
    };
    Ok(json!({ "top": number(f.top), "height": height }))
}

/// Port of `compactLift` (`apps/browser/src/lib/toolbarRow.ts:22`).
pub fn compact_lift(row: Value) -> Result<Value, String> {
    Ok(number(r::compact_lift(&row_from(&row)?)))
}

/// Port of `fieldInsetOf` (`apps/browser/src/lib/toolbarRow.ts:26`).
pub fn field_inset_of(row: Value) -> Result<Value, String> {
    Ok(number(r::field_inset_of(&row_from(&row)?)))
}

/// Port of `fieldAloneInsetOf` (`apps/browser/src/lib/toolbarRow.ts:28`).
pub fn field_alone_inset_of(row: Value, card_radius: f64) -> Result<Value, String> {
    Ok(number(r::field_alone_inset_of(
        &row_from(&row)?,
        card_radius,
    )))
}

/// Port of `toolbarTrailing` (`apps/browser/src/lib/toolbarRow.ts:33`).
pub fn toolbar_trailing(row: Value, o: Value) -> Result<Value, String> {
    let row = row_from(&row)?;
    if o.is_null() || is_undefined(&o) {
        return Err("TypeError: Cannot read properties of null (reading 'split')".into());
    }
    let options = ToolbarTrailingOptions {
        pinned_width: field(&o, "pinnedWidth"),
        // A string's `split` is `String.prototype.split`, a function, so it is truthy.
        split: o.is_string() || o.get("split").is_some_and(truthy),
        card_radius: o.get("cardRadius").and_then(as_number),
    };
    let t = r::toolbar_trailing(&row, &options);
    Ok(json!({
        "fieldRight": number(t.field_right),
        "extensionsRight": number(t.extensions_right),
        "splitRight": number(t.split_right),
        "inkGap": number(t.ink_gap),
    }))
}

/// JS truthiness of a value from JSON.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        _ => as_number(v).map_or(!is_undefined(v), |n| n != 0.0 && !n.is_nan()),
    }
}
