// Ported from ardia apps/browser/src/lib/errors.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The text of a thrown value.
//!
//! Changes from Ardia: a thrown value is a `Thrown`, because Rust has no `instanceof Error`: an `Error`
//! carries its `message`, any other value is `Json` or `undefined`. A JS `String` can hold a lone
//! surrogate; a Rust `String` cannot.

use ria_core::identity::telemetry_wire::{Json, js_number_to_string};

/// A value that a JS `throw` or a rejected promise carries.
#[derive(Debug, Clone, PartialEq)]
pub enum Thrown {
    /// An `Error` (or subclass) with its `message`.
    Error(String),
    /// Any other value.
    Value(Json),
    /// `undefined`.
    Undefined,
}

/// `errorText` (`errors.ts:1`): the message of an `Error`, else `String(error)`.
pub fn error_text(error: &Thrown) -> String {
    match error {
        Thrown::Error(message) => message.clone(),
        Thrown::Value(value) => js_string(value),
        Thrown::Undefined => "undefined".into(),
    }
}

/// `String(value)` for a JSON value: arrays join their items with `,` (`null` items are empty),
/// objects are `[object Object]`.
pub fn js_string(value: &Json) -> String {
    match value {
        Json::Null => "null".into(),
        Json::Bool(b) => b.to_string(),
        Json::Number(n) => js_number_to_string(*n),
        Json::String(s) => s.clone(),
        Json::Array(items) => items
            .iter()
            .map(|item| {
                if *item == Json::Null {
                    String::new()
                } else {
                    js_string(item)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Json::Object(_) => "[object Object]".into(),
    }
}
