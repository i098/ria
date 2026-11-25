// Ported from ardia apps/browser/src/lib/decode.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Checks and fallbacks that read untyped data (`unknown` in TS) as typed values.
//!
//! Changes from Ardia:
//! - `unknown` is `ria_core::identity::telemetry_wire::Json`: it has no `undefined`, functions or class
//!   instances. Pass `Json::Null` for `undefined`; every check here treats both the same.
//! - `stringOr` and `numberOr` return `T`, built with `T::from` from the string or number, so a fallback of
//!   `null` is `None` (`T = Option<_>`).

use ria_core::identity::telemetry_wire::Json;

/// `Fields` (`decode.ts:1`): the entries of a plain object.
pub type Fields = Vec<(String, Json)>;

/// `isFields` (`decode.ts:3`): a plain object, not `null` or an array.
pub fn is_fields(v: &Json) -> bool {
    matches!(v, Json::Object(_))
}

/// `isString` (`decode.ts:4`).
pub fn is_string(v: &Json) -> bool {
    matches!(v, Json::String(_))
}

/// `isNumber` (`decode.ts:5`): a finite number.
pub fn is_number(v: &Json) -> bool {
    matches!(v, Json::Number(n) if n.is_finite())
}

/// `isStringList` (`decode.ts:6`).
pub fn is_string_list(v: &Json) -> bool {
    matches!(v, Json::Array(items) if items.iter().all(is_string))
}

/// `stringOr` (`decode.ts:8`).
pub fn string_or<'a, T: From<&'a str>>(v: &'a Json, fallback: T) -> T {
    match v {
        Json::String(s) => T::from(s),
        _ => fallback,
    }
}

/// `numberOr` (`decode.ts:9`).
pub fn number_or<T: From<f64>>(v: &Json, fallback: T) -> T {
    match v {
        Json::Number(n) if n.is_finite() => T::from(*n),
        _ => fallback,
    }
}

/// `listOf` (`decode.ts:10`): the items of an array, else none.
pub fn list_of(v: &Json) -> &[Json] {
    match v {
        Json::Array(items) => items,
        _ => &[],
    }
}
