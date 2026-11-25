// Ported from ardia packages/sync/src/index.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Ria sync logic: the pure part of `packages/sync/src`. The `index.ts` re-exports map to the
//! `pub mod` lines of `logic`; the native bridges of `native.ts` and `sharedFolders.ts` are
//! types and traits here, and slice 5.13 implements them.
#![allow(clippy::type_complexity)] // shortcut: callback fields stay inline, as in the TypeScript types.


pub mod logic;
