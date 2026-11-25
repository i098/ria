// Ported from ardia apps/browser/src/lib/keys.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The modifier keys of a click.
//!
//! Not ported: `modifiersOf` (`keys.ts:4`) reads a React Native `GestureResponderEvent` (`keys.ts:1` imports
//! `react-native`). Only the `ClickModifiers` type is here.
//!
//! Changes from Ardia: only the type is ported.

/// `ClickModifiers` (`keys.ts:3`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClickModifiers {
    pub meta_key: bool,
    pub alt_key: bool,
    pub shift_key: bool,
}
