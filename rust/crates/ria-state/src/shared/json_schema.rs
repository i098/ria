// Ported from ardia apps/browser/src/lib/jsonSchema.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JSON Schema documents and their file text.
//!
//! Changes from Ardia:
//! - `JsonSchema` (`jsonSchema.ts:3`) is a `Json` object: the TS type only limits the keys at compile time.
//! - `json_stringify` is `JSON.stringify(value, null, gap)`, which `schemaJson` and `saved_document` use.

use ria_core::identity::telemetry_wire::{Json, js_order};

/// `JsonType` (`jsonSchema.ts:1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonType {
    Object,
    Array,
    String,
    Number,
    Integer,
    Boolean,
    Null,
}

impl JsonType {
    /// The schema name of the type.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Object => "object",
            Self::Array => "array",
            Self::String => "string",
            Self::Number => "number",
            Self::Integer => "integer",
            Self::Boolean => "boolean",
            Self::Null => "null",
        }
    }
}

/// `JsonSchema` (`jsonSchema.ts:3`): an object with the keys `type`, `enum`, `const`, `default`,
/// `description`, `minimum`, `maximum`, `exclusiveMinimum`, `pattern`, `items`, `prefixItems`, `minItems`,
/// `maxItems`, `uniqueItems`, `properties`, `additionalProperties`, `propertyNames`, `required`, `anyOf`.
pub type JsonSchema = Json;

/// `JSON_SCHEMA_DIALECT` (`jsonSchema.ts:25`).
pub const JSON_SCHEMA_DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

/// `schemaJson` (`jsonSchema.ts:27`): the schema with 2-space indents and a final newline.
pub fn schema_json(schema: &Json) -> String {
    format!("{}\n", json_stringify(schema, Some("  ")))
}

/// `JSON.stringify(value, null, gap)`: `None` gives one line; `Some("")` gives several lines with no indent,
/// as V8 does for a `space` between 0 and 1.
pub fn json_stringify(value: &Json, gap: Option<&str>) -> String {
    let Some(gap) = gap else {
        return value.stringify();
    };
    let mut out = String::new();
    write_indented(&mut out, value, gap, "");
    out
}

fn write_indented(out: &mut String, value: &Json, gap: &str, indent: &str) {
    let inner = format!("{indent}{gap}");
    let (open, close, items): (char, char, Vec<(Option<&String>, &Json)>) = match value {
        Json::Array(items) if !items.is_empty() => {
            ('[', ']', items.iter().map(|v| (None, v)).collect())
        }
        Json::Object(entries) if !entries.is_empty() => (
            '{',
            '}',
            js_order(entries)
                .into_iter()
                .map(|(k, v)| (Some(k), v))
                .collect(),
        ),
        _ => return out.push_str(&value.stringify()),
    };
    out.push(open);
    for (i, (key, item)) in items.into_iter().enumerate() {
        out.push_str(if i == 0 { "\n" } else { ",\n" });
        out.push_str(&inner);
        if let Some(key) = key {
            out.push_str(&Json::String(key.clone()).stringify());
            out.push_str(": ");
        }
        write_indented(out, item, gap, &inner);
    }
    out.push('\n');
    out.push_str(indent);
    out.push(close);
}
