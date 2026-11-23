// Ported from ardia packages/core/src/index.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Ria core logic. One `pub mod` line for each module of `packages/core/src/index.ts`;
//! a later slice adds the line of its own module.

pub mod errors;
pub mod fuzzy;

#[cfg(test)]
mod fuzzy_reference;
