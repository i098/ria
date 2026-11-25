// Ported from ardia apps/browser/src/lib/fallback.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Fallbacks for failed engine calls, and the kind of a failed password save.
//!
//! Changes from Ardia:
//! - The thrown value is `errors::Thrown`.
//! - `orWarn` writes its warning with `eprintln!` instead of `console.warn`. `or_warn_text` gives that
//!   text (`fallback.ts:8`), so that it can be checked.

use super::errors::{Thrown, error_text};

/// `PasswordAction` (`fallback.ts:3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordAction {
    BrowserImport,
    CsvImport,
    SettingsEdit,
    SettingsDelete,
    SyncDelete,
}

impl PasswordAction {
    /// The TS string of the action.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BrowserImport => "browser import",
            Self::CsvImport => "csv import",
            Self::SettingsEdit => "settings edit",
            Self::SettingsDelete => "settings delete",
            Self::SyncDelete => "sync delete",
        }
    }
}

/// `orWarn` (`fallback.ts:5`): a handler that warns about the error and gives `fallback`.
pub fn or_warn<T: Clone>(fallback: T, action: Option<PasswordAction>) -> impl Fn(&Thrown) -> T {
    move |error| {
        eprintln!("{}", or_warn_text(action, error));
        fallback.clone()
    }
}

/// The warning of `orWarn` (`fallback.ts:8`).
pub fn or_warn_text(action: Option<PasswordAction>, error: &Thrown) -> String {
    let action = action
        .map(|a| format!("{}: ", a.as_str()))
        .unwrap_or_default();
    format!("[engine] {action}{}", error_text(error))
}

/// `PasswordFailure` (`fallback.ts:12`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordFailure {
    RefusedOrAlreadySaved,
    NotLandedInTime,
    Other,
}

impl PasswordFailure {
    /// The TS string of the failure.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RefusedOrAlreadySaved => "refused-or-already-saved",
            Self::NotLandedInTime => "not-landed-in-time",
            Self::Other => "other",
        }
    }
}

/// `passwordFailure` (`fallback.ts:14`).
pub fn password_failure(error: &Thrown) -> PasswordFailure {
    let text = error_text(error);
    if text.contains("Could not save the password") {
        PasswordFailure::RefusedOrAlreadySaved
    } else if text.contains("didn't take the change") {
        PasswordFailure::NotLandedInTime
    } else {
        PasswordFailure::Other
    }
}
