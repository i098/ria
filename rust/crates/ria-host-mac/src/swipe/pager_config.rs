// Ported from ardia packages/ardiacore/ios/PagerConfig.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

/// The `nativePager` prop of a swipe area.
#[derive(Clone, Debug, PartialEq)]
pub struct PagerConfig {
    pub key: String,
    pub selected: i64,
    pub count: i64,
    pub width: f64,
    pub ack: i64,
    pub order: Option<Vec<String>>,
}

impl Default for PagerConfig {
    fn default() -> Self {
        Self {
            key: String::new(),
            selected: 0,
            count: 1,
            width: 0.0,
            ack: 0,
            order: None,
        }
    }
}
