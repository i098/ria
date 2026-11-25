// Ported from ardia apps/browser/src/lib/hoverMarks.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Not ported: `markHoverProps` and `isMarkedHover` (`hoverMarks.ts:3`, `hoverMarks.ts:8`) mark React
//! `onMouseEnter` handlers in a `WeakSet` by function identity, in `__DEV__` builds only, so that dev checks
//! can find hover props. They exist only for React components; Rust has no React props or function identity.
//!
//! Changes from Ardia: the whole file stays in TypeScript.
