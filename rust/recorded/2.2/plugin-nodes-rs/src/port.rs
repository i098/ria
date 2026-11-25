// Ported from ardia apps/browser/src/lib/pluginNodes.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_nodes`: the protocol structs become the JS
//! objects the TS builds, with each optional member left out when it is absent. The logic is in
//! `ria_state::shared::plugin_nodes`.

#![forbid(unsafe_code)]

use ria_core::identity::telemetry_wire::{Json, js_order};
use serde_json::Value;

/// A JS value as `Json`, object keys in JS order; `None` for `undefined`. An `undefined` inside
/// an array is `null`, inside an object it is left out; a string with lone surrogates loses them.
fn json_in(v: &Value) -> Option<Json> {
    if crate::js::is_undefined(v) {
        return None;
    }
    if let Some(n) = crate::js::as_number(v) {
        return Some(Json::Number(n));
    }
    if let Some(units) = crate::js::string_units(v) {
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

/// `Json` back as a JS value.
fn json_out(v: &Json) -> Value {
    match v {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => crate::js::number(*n),
        Json::String(s) => Value::String(s.clone()),
        Json::Array(items) => Value::Array(items.iter().map(json_out).collect()),
        Json::Object(entries) => Value::Object(
            js_order(entries)
                .into_iter()
                .map(|(k, x)| (k.clone(), json_out(x)))
                .collect(),
        ),
    }
}

use ria_state::shared::plugin_nodes as k;
use ria_state::shared::plugin_protocol::{PluginChild, PluginNode};
use serde_json::{Map, json};

/// Sets `key` when `value` is present.
fn put(out: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    if let Some(value) = value {
        out.insert(key.to_owned(), value);
    }
}

fn node_out(node: &PluginNode) -> Value {
    let children: Vec<Value> = node
        .children
        .iter()
        .map(|child| match child {
            PluginChild::Node(n) => node_out(n),
            PluginChild::Text(t) => Value::String(t.clone()),
        })
        .collect();
    let mut out = Map::new();
    out.insert("type".to_owned(), Value::String(node.r#type.clone()));
    out.insert(
        "props".to_owned(),
        json_out(&Json::Object(node.props.clone())),
    );
    out.insert("children".to_owned(), Value::Array(children));
    put(&mut out, "key", node.key.as_ref().map(json_out));
    Value::Object(out)
}

/// A raw node with each `undefined` in its props, at any depth, as `null`: `propValue` reads
/// `undefined` as `null` and keeps the key, where `json_in` leaves the member out.
fn node_in(v: &Value) -> Value {
    fn props(v: &Value) -> Value {
        match v {
            _ if crate::js::is_undefined(v) => Value::Null,
            Value::Array(items) => Value::Array(items.iter().map(props).collect()),
            Value::Object(o) => {
                Value::Object(o.iter().map(|(k, x)| (k.clone(), props(x))).collect())
            }
            _ => v.clone(),
        }
    }
    let Value::Object(o) = v else {
        return v.clone();
    };
    let node = o.iter().map(|(k, x)| {
        let x = match (k.as_str(), x) {
            ("props", Value::Object(_)) => props(x),
            ("children", Value::Array(items)) => Value::Array(items.iter().map(node_in).collect()),
            _ => x.clone(),
        };
        (k.clone(), x)
    });
    Value::Object(node.collect())
}

/// Port of `checkPluginNode` (`apps/browser/src/lib/pluginNodes.ts:69`).
pub fn check_plugin_node(raw: Value) -> Result<Value, String> {
    Ok(
        match k::check_plugin_node(json_in(&node_in(&raw)).as_ref()) {
            Ok(tree) => json!({ "ok": true, "tree": node_out(&tree) }),
            Err(problem) => json!({ "ok": false, "problem": problem }),
        },
    )
}

/// Port of `quickPickItemsOf` (`apps/browser/src/lib/pluginNodes.ts:82`).
pub fn quick_pick_items_of(raw: Value) -> Result<Option<Vec<Value>>, String> {
    let items = k::quick_pick_items_of(json_in(&raw).as_ref());
    Ok(items.map(|items| {
        items
            .into_iter()
            .map(|item| {
                let mut out = Map::new();
                out.insert("id".to_owned(), Value::String(item.id));
                out.insert("title".to_owned(), Value::String(item.title));
                put(&mut out, "subtitle", item.subtitle.map(Value::String));
                put(&mut out, "icon", item.icon.map(Value::String));
                put(&mut out, "badge", item.badge.map(Value::String));
                put(&mut out, "keywords", item.keywords.map(|k| json!(k)));
                put(&mut out, "picked", item.picked.map(Value::Bool));
                Value::Object(out)
            })
            .collect()
    }))
}

/// Port of `quickPickOptionsOf` (`apps/browser/src/lib/pluginNodes.ts:107`).
pub fn quick_pick_options_of(raw: Value) -> Result<Value, String> {
    let o = k::quick_pick_options_of(json_in(&raw).as_ref());
    let mut out = Map::new();
    put(&mut out, "title", o.title.map(Value::String));
    put(&mut out, "placeholder", o.placeholder.map(Value::String));
    put(&mut out, "canPickMany", o.can_pick_many.map(Value::Bool));
    Ok(Value::Object(out))
}

/// Port of `inputBoxOptionsOf` (`apps/browser/src/lib/pluginNodes.ts:118`).
pub fn input_box_options_of(raw: Value) -> Result<Option<Value>, String> {
    Ok(k::input_box_options_of(json_in(&raw).as_ref()).map(|o| {
        let mut out = Map::new();
        out.insert("title".to_owned(), Value::String(o.title));
        put(&mut out, "message", o.message.map(Value::String));
        put(&mut out, "placeholder", o.placeholder.map(Value::String));
        put(&mut out, "value", o.value.map(Value::String));
        put(&mut out, "password", o.password.map(Value::Bool));
        put(&mut out, "confirmTitle", o.confirm_title.map(Value::String));
        Value::Object(out)
    }))
}

/// Port of `toolbarUpdateOf` (`apps/browser/src/lib/pluginNodes.ts:135`).
pub fn toolbar_update_of(raw: Value) -> Result<Option<Value>, String> {
    let text = |t: Option<String>| t.map_or(Value::Null, Value::String);
    Ok(k::toolbar_update_of(json_in(&raw).as_ref()).map(|u| {
        let mut out = Map::new();
        put(&mut out, "icon", u.icon.map(Value::String));
        put(&mut out, "label", u.label.map(text));
        put(&mut out, "tooltip", u.tooltip.map(text));
        put(&mut out, "badge", u.badge.as_ref().map(json_out));
        put(&mut out, "tone", u.tone.map(Value::String));
        Value::Object(out)
    }))
}
