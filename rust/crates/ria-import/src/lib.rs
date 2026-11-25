// Ported from ardia packages/import/src/index.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Ria import. `logic` is the safe-Rust port of `packages/import/src`; the Mac readers of
//! the browsers come later as a `native` module that implements `logic::ImportNative`.

pub mod logic;
