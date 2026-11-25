// Ported from ardia apps/browser/src/lib/tabCorners.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::tab_corners`, through `tabCorners.harness.ts`: a look
//! holds a `Set` and a `Map` that the wire cannot carry, so `withTabCornersCase` takes a theme
//! document. The logic is in `ria_state::shared::tab_corners`.

#![forbid(unsafe_code)]

use std::borrow::Cow;

use crate::js::{as_number, is_undefined, number, string_units, undefined};
use ria_core::identity::telemetry_wire::{Json, js_order};
use ria_state::shared::tab_corners as t;
use ria_state::shared::theme_look::{
    DEFAULT_LOOK, Place, RADII, TYPE_SIZES, ThemeLook, parse_look_blocks, resolve_look,
};
use serde_json::{Value, json};

/// A JS value as `Json`, as `JSON.stringify` reads it: an `undefined` member is left out, an `undefined`
/// item is `null`.
fn json_of(v: &Value) -> Json {
    if let Some(n) = as_number(v) {
        return Json::Number(n);
    }
    if let Some(units) = string_units(v) {
        return Json::String(String::from_utf16_lossy(&units));
    }
    match v {
        Value::Bool(b) => Json::Bool(*b),
        Value::String(s) => Json::String(s.clone()),
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

/// `Json` as a JS value, object keys in JS order.
fn value_of(j: &Json) -> Value {
    match j {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => number(*n),
        Json::String(s) => Value::String(s.clone()),
        Json::Array(items) => Value::Array(items.iter().map(value_of).collect()),
        Json::Object(entries) => Value::Object(
            js_order(entries)
                .into_iter()
                .map(|(k, v)| (k.clone(), value_of(v)))
                .collect(),
        ),
    }
}

/// The harness's `lookOf`: the look of theme document `doc`; `null` or `undefined` is Ardia's own.
/// A document that is not an object has no blocks, as reading its keys gives `undefined`.
fn look_of(doc: Option<&Value>) -> ThemeLook {
    let Some(doc) = doc.filter(|d| !d.is_null() && !is_undefined(d)) else {
        return DEFAULT_LOOK.clone();
    };
    let fields = match json_of(doc) {
        Json::Object(fields) => fields,
        _ => Vec::new(),
    };
    resolve_look(Some(&parse_look_blocks(
        "themes/t.json",
        &fields,
        &mut Vec::new(),
    )))
}

/// The harness's `lookJson`: the look as a JS object, its `Set` and `Map` as arrays.
fn look_json(look: &ThemeLook) -> Value {
    let named = |names: &[&str], values: &[f64]| -> Value {
        Value::Object(
            names
                .iter()
                .zip(values)
                .map(|(k, v)| ((*k).to_owned(), number(*v)))
                .collect(),
        )
    };
    let sizes: Vec<&str> = TYPE_SIZES.iter().map(|(k, _)| *k).collect();
    let radii: Vec<&str> = RADII.iter().map(|(k, _)| *k).collect();
    json!({
        "family": look.family,
        "mono": look.mono,
        "sizes": named(&sizes, &look.sizes),
        "weights": value_of(&Json::Object(look.weights.clone())),
        "metrics": {
            "rowHeight": number(look.metrics.row_height),
            "rowGap": number(look.metrics.row_gap),
            "pinnedHeight": number(look.metrics.pinned_height),
        },
        "sidebarWidth": look.sidebar_width.map_or(Value::Null, number),
        "sidebarScale": number(look.sidebar_scale),
        "radii": named(&radii, &look.radii),
        "hidden": look.hidden.iter().map(value_of).collect::<Vec<_>>(),
        "addressField": look.address_field.name(),
        "newTab": look.new_tab.map(Place::name),
        "icons": look.icons.iter().map(|(k, v)| json!([k, value_of(v)])).collect::<Vec<_>>(),
    })
}

/// A `number | null` argument: `None` is `null`; any other value is not `null`.
fn number_or_null(v: &Value) -> Option<f64> {
    (!v.is_null()).then(|| as_number(v).unwrap_or(f64::NAN))
}

/// Port of `validTabCornersCase` (`tabCorners.harness.ts`): `validTabCorners` (`tabCorners.ts:5`).
pub fn valid_tab_corners_case(value: Value) -> Result<bool, String> {
    let value = (!is_undefined(&value)).then(|| json_of(&value));
    Ok(t::valid_tab_corners(value.as_ref()))
}

/// Port of `withTabCornersCase` (`tabCorners.harness.ts`): `withTabCorners` (`tabCorners.ts:8`);
/// `same` when it gives the look itself back. `undefined` corners, which the type excludes, take
/// the branch of a number no radius equals (`NaN`), and the radius they set is `undefined` again.
pub fn with_tab_corners_case(doc: Option<Value>, corners: Value) -> Result<Value, String> {
    let look = look_of(doc.as_ref());
    let corners_undefined = is_undefined(&corners);
    let corners = if corners_undefined {
        Some(f64::NAN)
    } else {
        number_or_null(&corners)
    };
    let out = t::with_tab_corners(&look, corners);
    let same = matches!(out, Cow::Borrowed(_));
    let mut look = look_json(&out);
    if corners_undefined {
        look["radii"]["chrome"] = undefined();
    }
    Ok(json!({ "look": look, "same": same }))
}

/// Port of `tabCornersControlCase` (`tabCorners.harness.ts`): `tabCornersControl`
/// (`tabCorners.ts:13`).
pub fn tab_corners_control_case(in_force: f64, chosen: Value) -> Result<Value, String> {
    let control = t::tab_corners_control(in_force, number_or_null(&chosen));
    Ok(json!({
        "position": number(control.position),
        "label": control.label,
        "followsTheme": control.follows_theme,
    }))
}
