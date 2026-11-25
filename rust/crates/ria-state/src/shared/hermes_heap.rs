// Ported from ardia apps/browser/src/lib/hermesHeap.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Not ported: `hermesHeapBytes`, `hermesHeapMegabytes` and `hermesCollections` (`hermesHeap.ts:7`,
//! `hermesHeap.ts:16`, `hermesHeap.ts:21`) read `HermesInternal.getInstrumentedStats()`, the heap statistics
//! of the Hermes JS engine of React Native. Rust code has no Hermes heap to measure.
//!
//! Changes from Ardia: the whole file stays in TypeScript.
