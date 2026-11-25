// Ported from ardia apps/browser/src/lib/customIcon.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Custom icons that name an SF Symbol: `symbol:<name>`.
//!
//! Changes from Ardia: none.

const SYMBOL_PREFIX: &str = "symbol:";

/// `symbolIcon` (`customIcon.ts:3`).
pub fn symbol_icon(name: &str) -> String {
    format!("{SYMBOL_PREFIX}{name}")
}

/// `iconSymbol` (`customIcon.ts:5`): the symbol name of a symbol icon, else `None`.
pub fn icon_symbol(icon: Option<&str>) -> Option<&str> {
    icon?.strip_prefix(SYMBOL_PREFIX)
}
