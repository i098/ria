// Ported from ardia apps/browser/src/lib/toolbarBadge.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Toolbar badge text: at most 4 characters, with long counts rounded to a unit (`12k`) instead of cut.
//!
//! Changes from Ardia:
//! - `toolbarBadgeText` takes a [`ToolbarBadgeValue`] for the TypeScript `string | number`.
//! - Lengths and the 4-character cut count UTF-16 code units, as JavaScript does. A cut through a
//!   surrogate pair gives U+FFFD where JavaScript keeps a lone surrogate, which a Rust `String` cannot hold.

use std::sync::LazyLock;

use regex::Regex;
use ria_core::identity::telemetry_wire::js_number_to_string;

use super::js::{js_number, js_round};

/// `BADGE_CHARS` (`toolbarBadge.ts:1`).
const BADGE_CHARS: usize = 4;

/// `COUNT` (`toolbarBadge.ts:2`). `\d` and the `i` flag are ASCII only in JavaScript without the `u` flag.
static COUNT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(-?)([0-9]+(?:\.[0-9]+)?)([kmbKMB]?)$").expect("static pattern")
});

/// `UNITS` (`toolbarBadge.ts:3`).
const UNITS: [&str; 4] = ["", "k", "m", "b"];

/// The `string | number` argument of `toolbarBadgeText`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ToolbarBadgeValue<'a> {
    Text(&'a str),
    Number(f64),
}

/// `value.slice(0, BADGE_CHARS)` on UTF-16 code units.
fn badge_slice(text: &str) -> String {
    String::from_utf16_lossy(&text.encode_utf16().take(BADGE_CHARS).collect::<Vec<_>>())
}

/// `toolbarBadgeText` (`toolbarBadge.ts:5`).
pub fn toolbar_badge_text(value: ToolbarBadgeValue) -> String {
    match value {
        ToolbarBadgeValue::Number(n) => {
            let text = if n.abs() > 999.0 {
                format!("{}999+", if n < 0.0 { "-" } else { "" })
            } else {
                js_number_to_string(n)
            };
            badge_slice(&text)
        }
        ToolbarBadgeValue::Text(text) if text.encode_utf16().count() <= BADGE_CHARS => {
            text.to_string()
        }
        ToolbarBadgeValue::Text(text) => rounded_count(text).unwrap_or_else(|| badge_slice(text)),
    }
}

/// `roundedCount` (`toolbarBadge.ts:11`): the count rounded to 1 or 0 decimals in this or a larger unit.
fn rounded_count(text: &str) -> Option<String> {
    let m = COUNT.captures(text)?;
    let (sign, suffix) = (&m[1], &m[3]);
    let upper = suffix.bytes().any(|b| b.is_ascii_uppercase());
    let start = UNITS
        .iter()
        .position(|u| u.eq_ignore_ascii_case(suffix))
        .unwrap_or(0);
    let mut amount = js_number(&m[2]);
    for unit in &UNITS[start..] {
        let name = if upper {
            unit.to_ascii_uppercase()
        } else {
            unit.to_string()
        };
        for decimals in [1.0, 0.0] {
            let shown = format!(
                "{sign}{}{name}",
                js_number_to_string(rounded_to(amount, decimals))
            );
            if shown.len() <= BADGE_CHARS {
                return Some(shown);
            }
        }
        amount /= 1000.0;
    }
    None
}

/// `shifted` (`toolbarBadge.ts:28`): `value` times `10^places`, done on the decimal text so that halves stay exact.
fn shifted(value: f64, places: f64) -> f64 {
    let text = js_number_to_string(value);
    let (mantissa, exponent) = text.split_once('e').unwrap_or((&text, "0"));
    // `Infinity` and `NaN` give `NaN` here, as in JavaScript.
    js_number(&format!(
        "{mantissa}e{}",
        js_number_to_string(js_number(exponent) + places)
    ))
}

/// `roundedTo` (`toolbarBadge.ts:33`).
fn rounded_to(value: f64, decimals: f64) -> f64 {
    shifted(js_round(shifted(value, decimals)), -decimals)
}
