// Ported from ardia apps/browser/src/lib/pluginShortcuts.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria_state::shared::plugin_shortcuts` for `pluginShortcuts.harness.ts`:
//! it reads shortcuts, events and bindings from JSON and writes the results back as JS objects.
//! Actions of the generic functions are JSON values that carry `props`.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, string_units};
use ria_core::identity::telemetry_wire::{Json, js_order};
use ria_state::shared::decode::Fields;
use ria_state::shared::plugin_shortcuts as s;
use ria_state::shared::plugin_shortcuts::{
    CommandFlag, PluginShortcut, ShortcutBinding, ShortcutKeyEvent, ShortcutRegistration,
};
use serde_json::{Value, json};

/// A JS value as `Json`, object keys in JS order; `None` for `undefined`. An `undefined` member is
/// left out, an `undefined` item is `null`.
fn json_in(v: &Value) -> Option<Json> {
    if is_undefined(v) {
        return None;
    }
    if let Some(n) = as_number(v) {
        return Some(Json::Number(n));
    }
    if let Some(units) = string_units(v) {
        return Some(Json::String(String::from_utf16_lossy(&units)));
    }
    Some(match v {
        Value::Bool(b) => Json::Bool(*b),
        Value::Array(items) => Json::Array(
            items
                .iter()
                .map(|x| json_in(x).unwrap_or(Json::Null))
                .collect(),
        ),
        Value::Object(o) => {
            let entries: Vec<(String, Json)> = o
                .iter()
                .filter_map(|(k, x)| Some((k.clone(), json_in(x)?)))
                .collect();
            Json::Object(js_order(&entries).into_iter().cloned().collect())
        }
        _ => Json::Null,
    })
}

/// `!!value`.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(t) => !t.is_empty(),
        _ => match as_number(v) {
            Some(n) => n != 0.0 && !n.is_nan(),
            None => !is_undefined(v),
        },
    }
}

fn flag(v: &Value, key: &str) -> bool {
    v.get(key).is_some_and(truthy)
}

/// A plain object: not `null`, a list, or a marker for `undefined` or a number.
fn is_object(v: &Value) -> bool {
    v.is_object() && !is_undefined(v) && as_number(v).is_none()
}

/// The list items of `v`, else the harness error for `what`.
fn list<'a>(v: &'a Value, what: &str) -> Result<&'a [Value], String> {
    v.as_array()
        .map(Vec::as_slice)
        .ok_or(format!("TypeError: {what} must be a list"))
}

fn shortcut_in(v: &Value) -> Result<PluginShortcut, String> {
    let key = v
        .get("key")
        .filter(|_| is_object(v))
        .and_then(Value::as_str)
        .ok_or("TypeError: shortcut.key must be text")?;
    Ok(PluginShortcut {
        key: key.to_owned(),
        control: flag(v, "control"),
        option: flag(v, "option"),
        shift: flag(v, "shift"),
    })
}

fn shortcut_out(x: &PluginShortcut) -> Value {
    json!({ "key": x.key, "control": x.control, "option": x.option, "shift": x.shift })
}

fn event_in(v: &Value) -> Result<ShortcutKeyEvent, String> {
    if !is_object(v) {
        return Err("TypeError: e must be an object".into());
    }
    Ok(ShortcutKeyEvent {
        key: v.get("key").and_then(Value::as_str).map(str::to_owned),
        meta_key: flag(v, "metaKey"),
        ctrl_key: flag(v, "ctrlKey"),
        alt_key: flag(v, "altKey"),
        shift_key: flag(v, "shiftKey"),
    })
}

fn command_flag_in(flag: &Value) -> Result<CommandFlag, String> {
    match flag.as_str() {
        Some("metaKey") => Ok(CommandFlag::MetaKey),
        Some("ctrlKey") => Ok(CommandFlag::CtrlKey),
        _ => Err("TypeError: commandFlag must be metaKey or ctrlKey".into()),
    }
}

fn handled_out(keys: Vec<s::ShortcutHandledKey>) -> Value {
    keys.into_iter()
        .map(|k| json!({ "key": k.key, "metaKey": k.meta_key, "ctrlKey": k.ctrl_key, "altKey": k.alt_key, "shiftKey": k.shift_key }))
        .collect()
}

/// A JSON action and its `props`.
struct Action<'a> {
    value: &'a Value,
    props: Fields,
}

impl s::Props for Action<'_> {
    fn props(&self) -> &Fields {
        &self.props
    }
}

fn binding_in(v: &Value) -> Result<ShortcutBinding<Value>, String> {
    let shortcut = shortcut_in(v.get("shortcut").unwrap_or(&Value::Null))?;
    Ok(ShortcutBinding {
        shortcut,
        action: v.get("action").cloned().unwrap_or(Value::Null),
    })
}

fn binding_out(b: &ShortcutBinding<Value>) -> Value {
    json!({ "shortcut": shortcut_out(&b.shortcut), "action": b.action })
}

fn bindings_in(values: &Value) -> Result<Vec<ShortcutBinding<Value>>, String> {
    list(values, "bindings")?.iter().map(binding_in).collect()
}

/// Port of `parsePluginShortcut` (`apps/browser/src/lib/pluginShortcuts.ts:68`).
pub fn parse_plugin_shortcut_case(raw: Value) -> Result<Value, String> {
    Ok(match s::parse_plugin_shortcut(json_in(&raw).as_ref()) {
        Some(x) => {
            let p = &x.shortcut;
            json!({ "key": p.key, "command": x.command, "control": p.control, "option": p.option, "shift": p.shift })
        }
        None => Value::Null,
    })
}

/// Port of `pluginShortcutText` (`apps/browser/src/lib/pluginShortcuts.ts:81`).
pub fn plugin_shortcut_text_case(shortcut: Value) -> Result<String, String> {
    Ok(s::plugin_shortcut_text(&shortcut_in(&shortcut)?))
}

/// Port of `isRefusedShortcut` (`apps/browser/src/lib/pluginShortcuts.ts:85`).
pub fn is_refused_shortcut_case(shortcut: Value) -> Result<bool, String> {
    let x = s::CommandShortcut {
        shortcut: shortcut_in(&shortcut)?,
        command: flag(&shortcut, "command"),
    };
    Ok(s::is_refused_shortcut(&x))
}

/// Port of `pluginShortcutOf` (`apps/browser/src/lib/pluginShortcuts.ts:91`).
pub fn plugin_shortcut_of_case(raw: Value) -> Result<Value, String> {
    Ok(s::plugin_shortcut_of(json_in(&raw).as_ref()).map_or(Value::Null, |x| shortcut_out(&x)))
}

/// Port of `pluginShortcutModifiers` (`apps/browser/src/lib/pluginShortcuts.ts:97`).
pub fn plugin_shortcut_modifiers_case(shortcut: Value) -> Result<Vec<String>, String> {
    Ok(s::plugin_shortcut_modifiers(&shortcut_in(&shortcut)?)
        .into_iter()
        .map(|m| m.as_str().to_owned())
        .collect())
}

/// Port of `pluginShortcutKeyChar` (`apps/browser/src/lib/pluginShortcuts.ts:103`).
pub fn plugin_shortcut_key_char_case(shortcut: Value) -> Result<String, String> {
    Ok(s::plugin_shortcut_key_char(&shortcut_in(&shortcut)?))
}

/// Port of `pluginShortcutHandledKeys` (`apps/browser/src/lib/pluginShortcuts.ts:120`).
pub fn plugin_shortcut_handled_keys_case(
    shortcut: Value,
    command_flag: Value,
) -> Result<Value, String> {
    Ok(handled_out(s::plugin_shortcut_handled_keys(
        &shortcut_in(&shortcut)?,
        command_flag_in(&command_flag)?,
    )))
}

/// Port of `pluginShortcutMatches` (`apps/browser/src/lib/pluginShortcuts.ts:125`).
pub fn plugin_shortcut_matches_case(
    shortcut: Value,
    e: Value,
    command_flag: Value,
) -> Result<bool, String> {
    Ok(s::plugin_shortcut_matches(
        &shortcut_in(&shortcut)?,
        &event_in(&e)?,
        command_flag_in(&command_flag)?,
    ))
}

/// Port of `bindingsOf` (`apps/browser/src/lib/pluginShortcuts.ts:137`).
pub fn bindings_of_case(actions: Value) -> Result<Value, String> {
    let actions: Vec<Action> = list(&actions, "actions")?
        .iter()
        .map(|value| {
            match value
                .get("props")
                .filter(|p| is_object(value) && is_object(p))
                .and_then(json_in)
            {
                Some(Json::Object(props)) => Ok(Action { value, props }),
                _ => Err("TypeError: action.props must be an object".to_owned()),
            }
        })
        .collect::<Result<_, _>>()?;
    Ok(s::bindings_of(&actions)
        .into_iter()
        .map(|b| json!({ "shortcut": shortcut_out(&b.shortcut), "action": b.action.value }))
        .collect())
}

/// Port of `mergeShortcutBindings` (`apps/browser/src/lib/pluginShortcuts.ts:144`).
pub fn merge_shortcut_bindings_case(registrations: Value, held: Value) -> Result<Value, String> {
    let registrations = list(&registrations, "registrations")?
        .iter()
        .map(|v| {
            if !is_object(v) {
                return Err("TypeError: registration must be an object".to_owned());
            }
            let whole = |key: &str| {
                v.get(key)
                    .and_then(Value::as_i64)
                    .ok_or(format!("TypeError: registration.{key}"))
            };
            Ok(ShortcutRegistration {
                depth: whole("depth")?,
                seq: whole("seq")?,
                bindings: bindings_in(v.get("bindings").unwrap_or(&Value::Null))?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let held: Vec<String> = if is_undefined(&held) {
        Vec::new()
    } else {
        list(&held, "held")?
            .iter()
            .map(|h| {
                h.as_str()
                    .map(str::to_owned)
                    .ok_or("TypeError: held must be a list of text")
            })
            .collect::<Result<_, _>>()?
    };
    let merged = s::merge_shortcut_bindings(registrations, |x| {
        held.contains(&s::plugin_shortcut_text(x))
    });
    Ok(merged.iter().map(binding_out).collect())
}

/// Port of `pressedBinding` (`apps/browser/src/lib/pluginShortcuts.ts:160`).
pub fn pressed_binding_case(
    bindings: Value,
    e: Value,
    command_flag: Value,
) -> Result<Value, String> {
    let bindings = bindings_in(&bindings)?;
    let e = event_in(&e)?;
    Ok(
        s::pressed_binding(&bindings, &e, command_flag_in(&command_flag)?)
            .map_or(Value::Null, binding_out),
    )
}

/// Port of `handledKeysOf` (`apps/browser/src/lib/pluginShortcuts.ts:164`).
pub fn handled_keys_of_case(bindings: Value, command_flag: Value) -> Result<Value, String> {
    Ok(handled_out(s::handled_keys_of(
        &bindings_in(&bindings)?,
        command_flag_in(&command_flag)?,
    )))
}
