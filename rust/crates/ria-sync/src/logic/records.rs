// Ported from ardia packages/sync/src/records.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The value shapes that the sync scopes store.

use serde::{Deserialize, Serialize};

/// The open tabs of one device.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DeviceTabs {
    pub n: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub k: Option<String>,
    pub tabs: Vec<TabRef>,
}

/// One tab of `DeviceTabs`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TabRef {
    pub u: String,
    pub t: String,
}

/// A tab sent to another device.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SendRecord {
    pub u: String,
    pub t: String,
    pub to: String,
    pub from: String,
    pub at: f64,
}

/// A bookmark or a bookmark folder (`k` is `"u"` or `"f"`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BookmarkValue {
    pub k: String,
    pub p: String,
    pub t: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub u: Option<String>,
    pub a: f64,
    pub pos: String,
}

/// The visits of one address.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HistoryValue {
    pub t: String,
    pub n: f64,
    pub vt: Vec<f64>,
}
