// Ported from ardia apps/browser/src/state/session/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `session` (`apps/browser/src/state/session`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.33 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LaunchOverrides {
    #[serde(
        rename = "restoreSession",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub restore_session: Option<bool>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `loadSession` (an exported function that writes a store).
    #[serde(rename = "loadSession")]
    LoadSession {
        #[serde(default)]
        __0: Option<LaunchOverrides>,
    },
    /// `startPersistence` (an exported function that writes a store).
    #[serde(rename = "startPersistence")]
    StartPersistence {
        #[serde(default)]
        overrides: Option<LaunchOverrides>,
    },
}

/// Applies `action` to `app`. Slice 2.33 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.33")
}
