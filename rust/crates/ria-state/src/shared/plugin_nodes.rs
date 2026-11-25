// Ported from ardia apps/browser/src/lib/pluginNodes.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Checks the UI trees, pickers, input boxes and toolbar updates a plugin sends, within limits on
//! size and depth, and strips control and direction characters from their text.
//!
//! Changes from Ardia:
//! - `NodeCheck` is `Result<PluginNode, String>`; a thrown `Error` inside is an `Err` message.
//! - `unknown` is `Option<&Json>`: `None` is `undefined`. `Json` has no `undefined` member, so a
//!   prop whose value is `undefined` is left out where the TS keeps it as `null`.
//! - `.slice(0, max)` counts UTF-16 units, as the TS does; a cut through a surrogate pair gives
//!   U+FFFD, where the TS keeps a lone surrogate that a Rust `String` cannot hold.

use super::decode::Fields;
use super::js::{js_max, js_min, js_round};
use super::plugin_protocol::{
    InputBoxOptions, PluginChild, PluginNode, QuickPickItem, QuickPickOptions, TONES, ToolbarUpdate,
};
use super::plugin_shortcuts::{plugin_shortcut_of, plugin_shortcut_text};
use regex::Regex;
use ria_core::identity::telemetry_wire::{Json, js_number_to_string, js_order};
use ria_core::search::js_text::js_trim;
use std::collections::HashSet;
use std::sync::LazyLock;

/// `NODE_LIMITS` (`pluginNodes.ts:6`).
pub struct NodeLimits {
    pub depth: usize,
    pub nodes: usize,
    pub text_bytes: usize,
    pub props: usize,
    pub prop_bytes: usize,
    pub items: usize,
}
/// `NODE_LIMITS` (`pluginNodes.ts:6`).
pub const NODE_LIMITS: NodeLimits = NodeLimits {
    depth: 24,
    nodes: 2000,
    text_bytes: 256 * 1024,
    props: 32,
    prop_bytes: 16 * 1024,
    items: 500,
};

static NODE_TYPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^[A-Z][A-Za-z0-9]{0,31}$").expect("NODE_TYPE"));
static PROP_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^[A-Za-z][A-Za-z0-9]{0,39}$").expect("PROP_NAME"));
static SYMBOL: LazyLock<Regex> = LazyLock::new(|| Regex::new("^[a-z0-9.]{1,64}$").expect("SYMBOL"));

/// `NodeCheck` (`pluginNodes.ts:13`).
pub type NodeCheck = Result<PluginNode, String>;

/// `Budget` (`pluginNodes.ts:15`).
struct Budget {
    nodes: usize,
    bytes: usize,
}

/// `UNSAFE` (`pluginNodes.ts:11`).
fn is_unsafe(c: char) -> bool {
    matches!(c, '\u{0}'..='\u{8}' | '\u{b}' | '\u{c}' | '\u{e}'..='\u{1f}' | '\u{7f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// `clean` (`pluginNodes.ts:17`).
fn clean(text: &str) -> String {
    text.chars().filter(|c| !is_unsafe(*c)).collect()
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// `text.slice(0, max)` in UTF-16 units.
fn slice_units(text: &str, max: usize) -> String {
    if utf16_len(text) <= max {
        return text.to_owned();
    }
    String::from_utf16_lossy(&text.encode_utf16().take(max).collect::<Vec<_>>())
}

/// `JSON.stringify(text.slice(0, max))`: a lone surrogate from the cut is escaped, as V8 does.
fn quoted_slice(text: &str, max: usize) -> String {
    let units: Vec<u16> = text.encode_utf16().take(max).collect();
    let mut out = String::from("\"");
    for c in char::decode_utf16(units.iter().copied()) {
        match c {
            Ok('"') => out.push_str("\\\""),
            Ok('\\') => out.push_str("\\\\"),
            Ok('\u{8}') => out.push_str("\\b"),
            Ok('\u{c}') => out.push_str("\\f"),
            Ok('\n') => out.push_str("\\n"),
            Ok('\r') => out.push_str("\\r"),
            Ok('\t') => out.push_str("\\t"),
            Ok(c) if c < ' ' => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            Ok(c) => out.push(c),
            Err(e) => out.push_str(&format!("\\u{:04x}", e.unpaired_surrogate())),
        }
    }
    out.push('"');
    out
}

fn field<'a>(doc: &'a Fields, key: &str) -> Option<&'a Json> {
    doc.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// `propValue` (`pluginNodes.ts:19`).
fn prop_value(value: &Json, budget: &mut Budget, depth: usize) -> Result<Json, String> {
    match value {
        Json::Null | Json::Bool(_) => return Ok(value.clone()),
        Json::Number(n) => {
            return Ok(if n.is_finite() {
                value.clone()
            } else {
                Json::Null
            });
        }
        Json::String(s) => {
            budget.bytes += utf16_len(s);
            return Ok(Json::String(clean(s)));
        }
        _ => {}
    }
    if depth > 4 {
        return Err("has props nested too deep".to_owned());
    }
    match value {
        Json::Array(items) => {
            if items.len() > NODE_LIMITS.items {
                return Err(format!("has a prop list over {}", NODE_LIMITS.items));
            }
            Ok(Json::Array(
                items
                    .iter()
                    .map(|v| prop_value(v, budget, depth + 1))
                    .collect::<Result<_, _>>()?,
            ))
        }
        Json::Object(entries) => Ok(Json::Object(
            js_order(entries)
                .into_iter()
                .take(NODE_LIMITS.props)
                .map(|(k, v)| Ok((k.clone(), prop_value(v, budget, depth + 1)?)))
                .collect::<Result<_, String>>()?,
        )),
        _ => Ok(Json::Null),
    }
}

/// `nodeOf` (`pluginNodes.ts:34`).
fn node_of(raw: &Json, budget: &mut Budget, depth: usize) -> Result<PluginChild, String> {
    match raw {
        Json::String(s) => {
            budget.bytes += utf16_len(s);
            return Ok(PluginChild::Text(clean(s)));
        }
        Json::Number(n) => return Ok(PluginChild::Text(js_number_to_string(*n))),
        _ => {}
    }
    if depth > NODE_LIMITS.depth {
        return Err(format!("nests deeper than {}", NODE_LIMITS.depth));
    }
    budget.nodes += 1;
    if budget.nodes > NODE_LIMITS.nodes {
        return Err(format!("has more than {} nodes", NODE_LIMITS.nodes));
    }
    let Json::Object(raw) = raw else {
        return Err("has a node that isn't an object or text".to_owned());
    };
    let node_type = match field(raw, "type") {
        Some(Json::String(t)) if NODE_TYPE.is_match(t) => t.clone(),
        _ => return Err("has a node whose type isn't a component name".to_owned()),
    };
    let given = match field(raw, "props") {
        None => Vec::new(),
        Some(Json::Object(props)) => js_order(props),
        Some(_) => return Err("has props that aren't an object".to_owned()),
    };
    if given.len() > NODE_LIMITS.props {
        return Err(format!(
            "has a node with more than {} props",
            NODE_LIMITS.props
        ));
    }
    let mut props = Fields::new();
    for (name, value) in given {
        if !PROP_NAME.is_match(name) {
            return Err(format!("has a prop named {}", quoted_slice(name, 40)));
        }
        if name == "shortcut" {
            if let Some(shortcut) = plugin_shortcut_of(Some(value)) {
                props.push((name.clone(), Json::String(plugin_shortcut_text(&shortcut))));
            }
            continue;
        }
        let before = budget.bytes;
        props.push((name.clone(), prop_value(value, budget, 0)?));
        if budget.bytes - before > NODE_LIMITS.prop_bytes {
            return Err(format!(
                "has a prop over {} KB",
                NODE_LIMITS.prop_bytes / 1024
            ));
        }
    }
    let children = match field(raw, "children") {
        None => &[][..],
        Some(Json::Array(items)) => items,
        Some(_) => return Err("has children that aren't a list".to_owned()),
    };
    let key = match field(raw, "key") {
        Some(Json::String(k)) => Some(Json::String(slice_units(k, 200))),
        Some(&Json::Number(n)) if n.is_finite() => Some(Json::Number(n)),
        _ => None,
    };
    let children = children
        .iter()
        .filter(|child| !matches!(child, Json::Null | Json::Bool(_)))
        .map(|child| node_of(child, budget, depth + 1))
        .collect::<Result<_, _>>()?;
    if budget.bytes > NODE_LIMITS.text_bytes {
        return Err(format!(
            "has over {} KB of text",
            NODE_LIMITS.text_bytes / 1024
        ));
    }
    Ok(PluginChild::Node(PluginNode {
        r#type: node_type,
        key,
        props,
        children,
    }))
}

/// `checkPluginNode` (`pluginNodes.ts:67`).
pub fn check_plugin_node(raw: Option<&Json>) -> NodeCheck {
    let mut budget = Budget { nodes: 0, bytes: 0 };
    // `undefined` reads as a node that is not an object, as in the TS.
    match node_of(raw.unwrap_or(&Json::Null), &mut budget, 0)? {
        PluginChild::Text(_) => Err("must be a node, not text".to_owned()),
        PluginChild::Node(tree) => Ok(tree),
    }
}

/// `shortText` (`pluginNodes.ts:77`).
fn short_text(value: Option<&Json>, max: usize) -> Option<String> {
    match value {
        Some(Json::String(s)) if !js_trim(s).is_empty() => Some(slice_units(&clean(s), max)),
        _ => None,
    }
}

/// `symbolOrUndefined` (`pluginNodes.ts:78`).
fn symbol_or_undefined(value: Option<&Json>) -> Option<String> {
    match value {
        Some(Json::String(s)) if SYMBOL.is_match(s) => Some(s.clone()),
        _ => None,
    }
}

/// `shortText` that the TS reads by truthiness: an empty result is none.
fn short_some(value: Option<&Json>, max: usize) -> Option<String> {
    short_text(value, max).filter(|s| !s.is_empty())
}

/// `quickPickItemsOf` (`pluginNodes.ts:80`).
pub fn quick_pick_items_of(raw: Option<&Json>) -> Option<Vec<QuickPickItem>> {
    let Some(Json::Array(raw)) = raw else {
        return None;
    };
    if raw.len() > NODE_LIMITS.items {
        return None;
    }
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    for value in raw {
        let Json::Object(value) = value else {
            return None;
        };
        let id = short_some(field(value, "id"), 200);
        let title = short_some(field(value, "title"), 200);
        let (Some(id), Some(title)) = (id, title) else {
            return None;
        };
        if !seen.insert(id.clone()) {
            return None;
        }
        let keywords = match field(value, "keywords") {
            Some(Json::Array(k)) => Some(
                k.iter()
                    .filter_map(|k| short_text(Some(k), 40))
                    .take(8)
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        };
        items.push(QuickPickItem {
            id,
            title,
            subtitle: short_some(field(value, "subtitle"), 200),
            icon: symbol_or_undefined(field(value, "icon")),
            badge: short_some(field(value, "badge"), 24),
            keywords: keywords.filter(|k| !k.is_empty()),
            picked: (field(value, "picked") == Some(&Json::Bool(true))).then_some(true),
        });
    }
    Some(items)
}

/// `quickPickOptionsOf` (`pluginNodes.ts:106`).
pub fn quick_pick_options_of(raw: Option<&Json>) -> QuickPickOptions {
    let no_fields = Fields::new();
    let doc = match raw {
        Some(Json::Object(doc)) => doc,
        _ => &no_fields,
    };
    QuickPickOptions {
        title: short_some(field(doc, "title"), 80),
        placeholder: short_some(field(doc, "placeholder"), 120),
        can_pick_many: (field(doc, "canPickMany") == Some(&Json::Bool(true))).then_some(true),
    }
}

/// `inputBoxOptionsOf` (`pluginNodes.ts:117`).
pub fn input_box_options_of(raw: Option<&Json>) -> Option<InputBoxOptions> {
    let Some(Json::Object(raw)) = raw else {
        return None;
    };
    let title = short_some(field(raw, "title"), 80)?;
    Some(InputBoxOptions {
        title,
        message: short_some(field(raw, "message"), 400),
        placeholder: short_some(field(raw, "placeholder"), 120),
        value: match field(raw, "value") {
            Some(Json::String(v)) => Some(slice_units(&clean(v), 2000)),
            _ => None,
        },
        password: (field(raw, "password") == Some(&Json::Bool(true))).then_some(true),
        confirm_title: short_some(field(raw, "confirmTitle"), 30),
    })
}

/// `toolbarUpdateOf` (`pluginNodes.ts:135`).
pub fn toolbar_update_of(raw: Option<&Json>) -> Option<ToolbarUpdate> {
    let Some(Json::Object(raw)) = raw else {
        return None;
    };
    let mut out = ToolbarUpdate::default();
    if let Some(icon) = field(raw, "icon") {
        out.icon = Some(symbol_or_undefined(Some(icon))?);
    }
    if let Some(label) = field(raw, "label") {
        out.label = Some(if label == &Json::Null {
            None
        } else {
            short_text(Some(label), 24)
        });
    }
    if let Some(tooltip) = field(raw, "tooltip") {
        out.tooltip = Some(if tooltip == &Json::Null {
            None
        } else {
            short_text(Some(tooltip), 120)
        });
    }
    if let Some(badge) = field(raw, "badge") {
        out.badge = Some(match badge {
            Json::Null | Json::Bool(_) => badge.clone(),
            Json::Number(n) if n.is_finite() => {
                Json::Number(js_max(-9999.0, js_min(9999.0, js_round(*n))))
            }
            Json::String(s) => Json::String(slice_units(&clean(s), 6)),
            _ => return None,
        });
    }
    if let Some(tone) = field(raw, "tone") {
        match tone {
            Json::String(t) if TONES.contains(&t.as_str()) => out.tone = Some(t.clone()),
            _ => return None,
        }
    }
    Some(out)
}
