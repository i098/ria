// Ported from ardia apps/browser/src/lib/activeLook.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The look in force (Ardia's own until a theme sets one), and the icon swaps it makes in symbols
//! and menus.
//!
//! Changes from Ardia: the look in force is an `Arc<ThemeLook>` behind a lock; `active_look` gives
//! the `Arc`, and Ardia's `activeLook() === DEFAULT_LOOK` is a value comparison here. Menu items
//! (`SymbolItem`, generic `T` in Ardia) are `Json` objects; `with_icon_names` returns `Err` with the
//! TypeError where Ardia throws (an item that is not an object, `children` that is not an array),
//! and a borrowed `Cow` where Ardia returns the same array. `icon_name` returns `name` for an icon
//! that is not a string, which only unchecked look blocks hold (Ardia returns that value).

use std::borrow::Cow;
use std::sync::{Arc, LazyLock, RwLock};

use ria_core::identity::telemetry_wire::{Json, set_entry};

use super::theme_look::{DEFAULT_LOOK, ThemeLook};

/// `active` (`activeLook.ts:3`).
static ACTIVE: LazyLock<RwLock<Arc<ThemeLook>>> =
    LazyLock::new(|| RwLock::new(Arc::new(DEFAULT_LOOK.clone())));

/// `activeLook` (`activeLook.ts:5`).
pub fn active_look() -> Arc<ThemeLook> {
    ACTIVE
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

/// `setActiveLook` (`activeLook.ts:7`).
pub fn set_active_look(look: Arc<ThemeLook>) {
    *ACTIVE
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = look;
}

/// `active.icons.get(name) ?? name`: `None` when the look swaps nothing for `name`.
fn icon_of<'a>(look: &'a ThemeLook, name: &Json) -> Option<&'a Json> {
    let Json::String(name) = name else {
        return None;
    };
    look.icons
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, icon)| icon)
        .filter(|icon| !matches!(icon, Json::Null))
}

/// `iconName` (`activeLook.ts:11`).
pub fn icon_name(name: &str) -> String {
    match icon_of(&active_look(), &Json::String(name.into())) {
        Some(Json::String(icon)) => icon.clone(),
        _ => name.into(),
    }
}

/// JS truthiness.
fn truthy(value: &Json) -> bool {
    match value {
        Json::Null => false,
        Json::Bool(b) => *b,
        Json::Number(n) => *n != 0.0 && !n.is_nan(),
        Json::String(s) => !s.is_empty(),
        Json::Array(_) | Json::Object(_) => true,
    }
}

fn field<'a>(fields: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn with_icon_names_in<'i>(look: &ThemeLook, items: &'i [Json]) -> Result<Cow<'i, [Json]>, String> {
    if look.icons.is_empty() {
        return Ok(Cow::Borrowed(items));
    }
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let fields = match item {
            Json::Object(fields) => fields,
            // `"separator" in item` is false for an array, and `{ ...item }` keys its items by index.
            Json::Array(values) => {
                let spread = values
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (i.to_string(), v.clone()));
                out.push(Json::Object(spread.collect()));
                continue;
            }
            _ => {
                return Err("TypeError: Cannot use 'in' operator to search for 'separator'".into());
            }
        };
        if field(fields, "separator").is_some() {
            out.push(item.clone());
            continue;
        }
        let mut next = fields.clone();
        if let Some(symbol) = field(fields, "symbol") {
            let icon = icon_of(look, symbol).unwrap_or(symbol).clone();
            set_entry(&mut next, "symbol", icon);
        }
        if let Some(children) = field(fields, "children").filter(|c| truthy(c)) {
            let Json::Array(children) = children else {
                return Err("TypeError: items.map is not a function".into());
            };
            let children = with_icon_names_in(look, children)?.into_owned();
            set_entry(&mut next, "children", Json::Array(children));
        }
        out.push(Json::Object(next));
    }
    Ok(Cow::Owned(out))
}

/// `withIconNames` (`activeLook.ts:15`): the items with each `symbol` swapped for the look's icon,
/// submenus included.
pub fn with_icon_names(items: &[Json]) -> Result<Cow<'_, [Json]>, String> {
    with_icon_names_in(&active_look(), items)
}
