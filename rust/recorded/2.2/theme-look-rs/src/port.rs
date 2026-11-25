// Ported from ardia apps/browser/src/lib/themeLook.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::theme_look`: blocks cross as JSON, a `ThemeLook` as
//! the object TS builds (`hidden` a `Set`, `icons` a `Map`). `parseLookBlocks` gives its blocks; the
//! problems it pushes stay in Rust. The logic is in `ria_state::shared::theme_look`.
//! Arguments outside the TS types (for example a `color` that is not a colour name) are not covered:
//! the Rust side takes typed values, so the crate records with `--count 0` and only in-type inputs,
//! with the `null` and `undefined` cases where the TS really throws.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, number, string_units};
use ria_core::identity::telemetry_wire::Json;
use ria_state::shared::theme_look::{
    self as t, LOOK_KEYS, LookBlocks, Place, RADII, RowMetrics, SidebarTextSize, TYPE_SIZES,
    ThemeLook,
};
use serde_json::{Value, json};

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

/// `Json` as a JS value.
fn value_of(j: &Json) -> Value {
    match j {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => number(*n),
        Json::String(s) => Value::String(s.clone()),
        Json::Array(items) => Value::Array(items.iter().map(value_of).collect()),
        Json::Object(o) => Value::Object(o.iter().map(|(k, v)| (k.clone(), value_of(v))).collect()),
    }
}

/// A `LookBlocks` object from JS: a block that is `undefined` is left out.
fn blocks_of(v: &Value) -> LookBlocks {
    let block = |key| v.get(key).filter(|b| !is_undefined(b)).map(json_of);
    LookBlocks {
        type_: block("type"),
        metrics: block("metrics"),
        layout: block("layout"),
        icons: block("icons"),
    }
}

fn blocks_value(b: &LookBlocks) -> Value {
    Value::Object(
        LOOK_KEYS
            .iter()
            .filter_map(|key| Some(((*key).to_owned(), value_of(b.get(key)?))))
            .collect(),
    )
}

fn place_of(v: Option<&Value>) -> Option<Place> {
    match v?.as_str()? {
        "top" => Some(Place::Top),
        "bottom" => Some(Place::Bottom),
        _ => None,
    }
}

fn named(names: impl Iterator<Item = &'static str>, values: &[f64]) -> Value {
    Value::Object(
        names
            .zip(values)
            .map(|(k, v)| (k.into(), number(*v)))
            .collect(),
    )
}

fn entries_value(entries: &[(String, Json)]) -> Value {
    Value::Object(
        entries
            .iter()
            .map(|(k, v)| (k.clone(), value_of(v)))
            .collect(),
    )
}

fn look_value(l: &ThemeLook) -> Value {
    let icons = l
        .icons
        .iter()
        .map(|(k, v)| json!([k, value_of(v)]))
        .collect::<Vec<_>>();
    json!({
        "family": l.family,
        "mono": l.mono,
        "sizes": named(TYPE_SIZES.iter().map(|(k, _)| *k), &l.sizes),
        "weights": entries_value(&l.weights),
        "metrics": {
            "rowHeight": number(l.metrics.row_height),
            "rowGap": number(l.metrics.row_gap),
            "pinnedHeight": number(l.metrics.pinned_height),
        },
        "sidebarWidth": l.sidebar_width.map_or(Value::Null, number),
        "sidebarScale": number(l.sidebar_scale),
        "radii": named(RADII.iter().map(|(k, _)| *k), &l.radii),
        "hidden": {"$set": l.hidden.iter().map(value_of).collect::<Vec<_>>()},
        "addressField": l.address_field.name(),
        "newTab": l.new_tab.map(Place::name),
        "icons": {"$map": icons},
    })
}

/// A `ThemeLook` from JS; a field that is missing or of the wrong type is `DEFAULT_LOOK`'s.
fn look_of(v: &Value) -> ThemeLook {
    let d = &*t::DEFAULT_LOOK;
    let num = |o: Option<&Value>, key: &str, fallback: f64| {
        o.and_then(|o| o.get(key))
            .and_then(as_number)
            .unwrap_or(fallback)
    };
    let sizes = v.get("sizes");
    let radii = v.get("radii");
    let metrics = v.get("metrics");
    let entries = |key| match v.get(key).map(json_of) {
        Some(Json::Object(o)) => o,
        _ => Vec::new(),
    };
    let wrapped = |key, tag| {
        v.get(key)
            .and_then(|w| w.get(tag))
            .and_then(Value::as_array)
    };
    ThemeLook {
        family: v.get("family").and_then(Value::as_str).map(Into::into),
        mono: v
            .get("mono")
            .and_then(Value::as_str)
            .map_or(d.mono.clone(), Into::into),
        sizes: std::array::from_fn(|i| num(sizes, TYPE_SIZES[i].0, d.sizes[i])),
        weights: entries("weights"),
        metrics: RowMetrics {
            row_height: num(metrics, "rowHeight", d.metrics.row_height),
            row_gap: num(metrics, "rowGap", d.metrics.row_gap),
            pinned_height: num(metrics, "pinnedHeight", d.metrics.pinned_height),
        },
        sidebar_width: v.get("sidebarWidth").and_then(as_number),
        sidebar_scale: num(Some(v), "sidebarScale", d.sidebar_scale),
        radii: std::array::from_fn(|i| num(radii, RADII[i].0, d.radii[i])),
        hidden: wrapped("hidden", "$set")
            .map_or_else(Vec::new, |items| items.iter().map(json_of).collect()),
        address_field: place_of(v.get("addressField")).unwrap_or(d.address_field),
        new_tab: place_of(v.get("newTab")),
        icons: wrapped("icons", "$map").map_or_else(Vec::new, |items| {
            items
                .iter()
                .filter_map(|e| Some((e.get(0)?.as_str()?.to_owned(), json_of(e.get(1)?))))
                .collect()
        }),
    }
}

/// `blocks ?? null` as blocks.
fn maybe_blocks(v: Option<&Value>) -> Option<LookBlocks> {
    v.filter(|b| !b.is_null() && !is_undefined(b))
        .map(blocks_of)
}

/// Port of `parseLookBlocks` (`apps/browser/src/lib/themeLook.ts:221`).
pub fn parse_look_blocks(
    file: String,
    doc: serde_json::Value,
    problems: Vec<String>,
) -> Result<serde_json::Value, String> {
    if doc.is_null() || is_undefined(&doc) {
        let what = if doc.is_null() { "null" } else { "undefined" };
        return Err(format!(
            "TypeError: Cannot read properties of {what} (reading 'type')"
        ));
    }
    let fields = match json_of(&doc) {
        Json::Object(o) => o,
        _ => Vec::new(),
    };
    let mut problems = problems;
    Ok(blocks_value(&t::parse_look_blocks(
        &file,
        &fields,
        &mut problems,
    )))
}

/// Port of `resolveLook` (`apps/browser/src/lib/themeLook.ts:254`).
pub fn resolve_look(blocks: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
    Ok(look_value(&t::resolve_look(
        maybe_blocks(blocks.as_ref()).as_ref(),
    )))
}

/// Port of `withSidebarText` (`apps/browser/src/lib/themeLook.ts:287`): a size with no scale gives
/// `look` back as it came.
pub fn with_sidebar_text(
    look: serde_json::Value,
    size: serde_json::Value,
) -> Result<serde_json::Value, String> {
    match size.as_str().and_then(SidebarTextSize::from_name) {
        Some(size) if size != SidebarTextSize::Default => {
            Ok(look_value(&t::with_sidebar_text(&look_of(&look), size)))
        }
        _ => Ok(look),
    }
}

/// Port of `lookUse` (`apps/browser/src/lib/themeLook.ts:303`).
pub fn look_use(blocks: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
    let u = t::look_use(maybe_blocks(blocks.as_ref()).as_ref());
    Ok(json!({
        "type": u.type_, "metrics": u.metrics, "layout": u.layout, "icons": u.icons,
        "hidden": u.hidden, "moved": u.moved, "iconOverrides": u.icon_overrides,
    }))
}

/// Port of `lookSchemas` (`apps/browser/src/lib/themeLook.ts:337`).
pub fn look_schemas() -> Result<serde_json::Value, String> {
    Ok(entries_value(&t::look_schemas()))
}

/// Port of `lookDefaults` (`apps/browser/src/lib/themeLook.ts:341`).
pub fn look_defaults() -> Result<serde_json::Value, String> {
    Ok(blocks_value(&t::look_defaults()))
}
