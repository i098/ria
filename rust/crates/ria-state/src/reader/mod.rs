// Ported from ardia apps/browser/src/state/reader/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `reader` (`apps/browser/src/state/reader`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.38 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `closeReader` (an exported function that writes a store).
    #[serde(rename = "closeReader")]
    CloseReader {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `runReaderCommand` (an exported function that writes a store).
    #[serde(rename = "runReaderCommand")]
    RunReaderCommand {
        command: String,
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `startReader` (an exported function that writes a store).
    #[serde(rename = "startReader")]
    StartReader,
    /// `stepReaderSize` (an exported function that writes a store).
    #[serde(rename = "stepReaderSize")]
    StepReaderSize { delta: Number },
    /// `toggleReader` (an exported function that writes a store).
    #[serde(rename = "toggleReader")]
    ToggleReader {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `toggleReaderFont` (an exported function that writes a store).
    #[serde(rename = "toggleReaderFont")]
    ToggleReaderFont,
}

/// Applies `action` to `app`. Slice 2.38 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.38")
}
