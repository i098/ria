// Ported from ardia apps/browser/src/lib/absoluteFolder.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Absolute folder paths.
//!
//! Changes from Ardia: none.

/// `isAbsoluteFolder` (`absoluteFolder.ts:1`).
pub fn is_absolute_folder(path: &str) -> bool {
    path.starts_with('/')
}
