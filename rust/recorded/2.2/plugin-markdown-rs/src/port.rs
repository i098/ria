// Ported from ardia apps/browser/src/lib/pluginMarkdown.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_markdown`: spans and blocks as the TS objects.
//! The `style` of `parseInline` is read as JS reads any value.

#![forbid(unsafe_code)]

use ria_state::shared::plugin_markdown::{self as m, MarkdownBlock, MarkdownSpan, SpanStyle};
use serde_json::{Map, Value, json};

/// The JS truthiness of a style flag; `null` and `undefined` are absent, as for `??`.
fn flag(style: &Value, key: &str) -> Option<bool> {
    match style.get(key)? {
        Value::Null => None,
        v if crate::js::is_undefined(v) => None,
        Value::Bool(b) => Some(*b),
        Value::String(s) => Some(!s.is_empty()),
        v => Some(crate::js::as_number(v).is_none_or(|n| n != 0.0 && !n.is_nan())),
    }
}

/// The style as JS reads it: a string has `String.prototype.bold`, so its spans are bold.
fn style_in(style: &Value) -> SpanStyle {
    if style.is_string() {
        return SpanStyle {
            bold: Some(true),
            ..SpanStyle::default()
        };
    }
    SpanStyle {
        bold: flag(style, "bold"),
        italic: flag(style, "italic"),
        code: flag(style, "code"),
        url: style.get("url").and_then(Value::as_str).map(str::to_owned),
    }
}

fn span_out(s: &MarkdownSpan) -> Value {
    let mut o = Map::new();
    o.insert("text".into(), json!(s.text));
    for (key, on) in [("bold", s.bold), ("italic", s.italic), ("code", s.code)] {
        if on {
            o.insert(key.into(), json!(true));
        }
    }
    if let Some(url) = &s.url {
        o.insert("url".into(), json!(url));
    }
    Value::Object(o)
}

fn spans_out(spans: &[MarkdownSpan]) -> Vec<Value> {
    spans.iter().map(span_out).collect()
}

fn block_out(b: &MarkdownBlock) -> Value {
    match b {
        MarkdownBlock::Heading { level, spans } => {
            json!({ "kind": "heading", "level": level, "spans": spans_out(spans) })
        }
        MarkdownBlock::Paragraph { spans } => {
            json!({ "kind": "paragraph", "spans": spans_out(spans) })
        }
        MarkdownBlock::Code { text, language } => json!({
            "kind": "code",
            "text": text,
            "language": language.as_ref().map_or_else(crate::js::undefined, |l| json!(l)),
        }),
        MarkdownBlock::List { ordered, items } => json!({
            "kind": "list",
            "ordered": ordered,
            "items": items.iter().map(|i| json!({ "marker": i.marker, "depth": i.depth, "spans": spans_out(&i.spans) })).collect::<Vec<_>>(),
        }),
        MarkdownBlock::Quote { blocks } => {
            json!({ "kind": "quote", "blocks": blocks.iter().map(block_out).collect::<Vec<_>>() })
        }
        MarkdownBlock::Rule => json!({ "kind": "rule" }),
    }
}

/// Port of `markdownLink` (`apps/browser/src/lib/pluginMarkdown.ts:31`).
pub fn markdown_link(raw: String) -> Result<Option<String>, String> {
    Ok(m::markdown_link(&raw))
}

/// Port of `parseInline` (`apps/browser/src/lib/pluginMarkdown.ts:110`).
pub fn parse_inline(
    source: String,
    style: Value,
    depth: Option<f64>,
) -> Result<Vec<Value>, String> {
    let read = if crate::js::is_undefined(&style) {
        SpanStyle::default()
    } else {
        style_in(&style)
    };
    let spans = m::parse_inline(&source, &read, depth.unwrap_or(0.0));
    // shortcut: a `null` style throws at its first read; this names `bold`, the read of a text
    // span, also where a link is read first (`url`). Upgrade if a caller passes `null`.
    if style.is_null() && !spans.is_empty() {
        return Err("TypeError: Cannot read properties of null (reading 'bold')".into());
    }
    Ok(spans_out(&spans))
}

/// Port of `parseMarkdown` (`apps/browser/src/lib/pluginMarkdown.ts:157`).
pub fn parse_markdown(source: String, depth: Option<f64>) -> Result<Vec<Value>, String> {
    Ok(m::parse_markdown(&source, depth.unwrap_or(0.0))
        .iter()
        .map(block_out)
        .collect())
}
