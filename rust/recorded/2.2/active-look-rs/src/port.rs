// Ported from ardia apps/browser/src/lib/activeLook.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::active_look`, through `activeLook.harness.ts`: a look
//! holds a `Set` and a `Map` that the wire cannot carry, so each case takes a theme document and
//! sets its look, makes the call, and sets Ardia's own look back. The logic is in
//! `ria_state::shared::active_look`.

#![forbid(unsafe_code)]

use std::borrow::Cow;
use std::sync::Arc;

use crate::js::{as_number, is_undefined, number, string_units};
use ria_core::identity::telemetry_wire::{Json, js_order};
use ria_state::shared::active_look as a;
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

/// The harness's `withLook`: runs `call` with the look of `doc` in force, then Ardia's own again.
fn with_look<T>(doc: Option<&Value>, call: impl FnOnce() -> T) -> T {
    a::set_active_look(Arc::new(look_of(doc)));
    let out = call();
    a::set_active_look(Arc::new(DEFAULT_LOOK.clone()));
    out
}

/// Port of `activeLookCase` (`activeLook.harness.ts`): `activeLook` (`activeLook.ts:5`) after
/// `setActiveLook` (`activeLook.ts:7`).
pub fn active_look_case(doc: Option<Value>) -> Result<Value, String> {
    Ok(with_look(doc.as_ref(), || look_json(&a::active_look())))
}

/// Port of `iconNameCase` (`activeLook.harness.ts`): `iconName` (`activeLook.ts:11`).
pub fn icon_name_case(doc: Option<Value>, name: String) -> Result<String, String> {
    Ok(with_look(doc.as_ref(), || a::icon_name(&name)))
}

/// Port of `withIconNamesCase` (`activeLook.harness.ts`): `withIconNames` (`activeLook.ts:15`);
/// `same` when it gives the items themselves back. Items that are not an array come back as they
/// are when the look swaps no icon, and throw at `items.map` otherwise, as in Ardia.
pub fn with_icon_names_case(doc: Option<Value>, items: Value) -> Result<Value, String> {
    with_look(doc.as_ref(), || {
        let Value::Array(list) = &items else {
            if a::active_look().icons.is_empty() {
                return Ok(json!({ "items": items, "same": true }));
            }
            return Err("TypeError: items.map is not a function".into());
        };
        let list: Vec<Json> = list.iter().map(json_of).collect();
        let out = a::with_icon_names(&list)?;
        let same = matches!(out, Cow::Borrowed(_));
        Ok(json!({ "items": out.iter().map(value_of).collect::<Vec<_>>(), "same": same }))
    })
}
