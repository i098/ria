// Ported from ardia apps/browser/src/state/find/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `find` (`apps/browser/src/state/find`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.42 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OpenFindOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replace: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `closeFind` (an exported function that writes a store).
    #[serde(rename = "closeFind")]
    CloseFind {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `findStep` (an exported function that writes a store).
    #[serde(rename = "findStep")]
    FindStep {
        #[serde(rename = "tabId")]
        tab_id: String,
        backwards: bool,
    },
    /// `openFind` (an exported function that writes a store).
    #[serde(rename = "openFind")]
    OpenFind {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(default)]
        options: Option<OpenFindOptions>,
    },
    /// `takeFindFocusRequest` (an exported function that writes a store).
    #[serde(rename = "takeFindFocusRequest")]
    TakeFindFocusRequest {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
}

/// Applies `action` to `app`. Slice 2.42 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.42")
}
