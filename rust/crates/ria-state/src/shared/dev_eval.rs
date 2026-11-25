// Ported from ardia apps/browser/src/lib/devEval.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The file names of the dev-build script channel.
//!
//! Not ported: `watchDevEval` and `scriptId` (`devEval.ts:4`, `devEval.ts:6`) poll a script file with
//! `setInterval` and give its JS source to the React Native dev harness to evaluate. Rust cannot evaluate
//! that JS, so only the file names, which the native side shares, are here.
//!
//! Changes from Ardia: only the constants are ported.

/// `DEV_EVAL_SCRIPT` (`devEval.ts:1`).
pub const DEV_EVAL_SCRIPT: &str = "dev-eval.js";
/// `DEV_EVAL_RESULT` (`devEval.ts:2`).
pub const DEV_EVAL_RESULT: &str = "dev-eval-result.json";
