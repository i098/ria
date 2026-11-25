// Ported from ardia apps/browser/src/lib/afterFirstWindow.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Not ported: `afterFirstWindow` and `firstWindowCommitted` (`afterFirstWindow.ts:16`,
//! `afterFirstWindow.ts:25`) hold launch work until React Native commits the first window, then run it as
//! `setTimeout` tasks of the JS event loop (with a 5 s fallback timer). `ria-state` has no event loop or
//! window commit; Rust callers schedule deferred launch work with the runtime that owns their window.
//!
//! Changes from Ardia: the whole file stays in TypeScript.
