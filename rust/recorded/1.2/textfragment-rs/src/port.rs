// Ported from ardia packages/core/src/textFragment.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `url::text_fragment`: it maps the JSON objects of the
//! recording to typed values. The logic is in `ria_core::url::text_fragment`.

#![forbid(unsafe_code)]

use ria_core::url::text_fragment::{self, SelectionContext, TextFragment};
use serde_json::{Map, Value};

// A number is accepted as its JS text: the original coerces it the same way.
fn field(obj: &Map<String, Value>, key: &str) -> Result<String, String> {
    match obj.get(key) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(Value::Number(n)) => Ok(n.to_string()),
        _ => Err(format!("TypeError: {key} is not a string")),
    }
}

/// The JS string of a truthy field value, `None` for a falsy one (`prefix && ...` skips it).
fn truthy_string(obj: Option<&Value>) -> Option<String> {
    match obj {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::Number(n)) if n.as_f64() != Some(0.0) => Some(n.to_string()),
        Some(Value::Bool(true)) => Some("true".into()),
        Some(Value::Array(_)) => Some(String::new()),
        Some(Value::Object(_)) => Some("[object Object]".into()),
        _ => None,
    }
}

/// `encodeURIComponent(start)` takes any value; a missing one is the text `undefined`.
fn start_string(value: Option<&Value>) -> String {
    match value {
        None => "undefined".into(),
        Some(Value::Null) => "null".into(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Array(_)) => String::new(),
        Some(Value::Object(_)) => "[object Object]".into(),
    }
}

/// Port of `chooseTextFragment` (`packages/core/src/textFragment.ts:39`).
pub fn choose_text_fragment(context: Value) -> Result<Option<Value>, String> {
    let Value::Object(obj) = &context else {
        return Err("TypeError: Cannot destructure property 'selected' of context".into());
    };
    let (selected, before, after, page_text) = (
        field(obj, "selected")?,
        field(obj, "before")?,
        field(obj, "after")?,
        field(obj, "pageText")?,
    );
    let context = SelectionContext {
        selected: &selected,
        before: &before,
        after: &after,
        page_text: &page_text,
    };
    Ok(text_fragment::choose_text_fragment(&context).map(|f| {
        let mut o = Map::new();
        for (key, value) in [
            ("prefix", f.prefix),
            ("start", Some(f.start)),
            ("end", f.end),
            ("suffix", f.suffix),
        ] {
            if let Some(v) = value {
                o.insert(key.into(), v.into());
            }
        }
        Value::Object(o)
    }))
}

/// Port of `withTextFragment` (`packages/core/src/textFragment.ts:78`).
pub fn with_text_fragment(url: String, fragment: Value) -> Result<String, String> {
    if fragment.is_null() {
        return Err(
            "TypeError: Cannot destructure property 'prefix' of 'fragment' as it is null.".into(),
        );
    }
    // A string, number or array has none of the fields: they read as `undefined`.
    let get = |key: &str| fragment.as_object().and_then(|o| o.get(key));
    let fragment = TextFragment {
        prefix: truthy_string(get("prefix")),
        start: start_string(get("start")),
        end: truthy_string(get("end")),
        suffix: truthy_string(get("suffix")),
    };
    Ok(text_fragment::with_text_fragment(&url, &fragment))
}
