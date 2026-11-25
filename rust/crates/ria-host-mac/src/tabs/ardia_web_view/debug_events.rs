// Ported from ardia packages/ardiacore/ios/ArdiaWebView.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Changes from Ardia: Use Rust's debug assertion flag and the host uptime clock.
//! Truncate at a Unicode scalar boundary to keep Rust strings valid.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

use objc2::MainThreadMarker;
use objc2_foundation::NSProcessInfo;
use serde_json::{Map, Value, json};

const LIMIT: usize = 80;

thread_local! {
    static EVENTS: RefCell<HashMap<i32, VecDeque<Value>>> = RefCell::default();
}

/// `NoteEvent`, `ArdiaWebView.mm:96–110`.
pub fn note_event(_: MainThreadMarker, browser: i32, name: &str, mut payload: Map<String, Value>) {
    for value in payload.values_mut() {
        if let Value::String(text) = value {
            let scheme = text.split_once(':').map(|(scheme, _)| scheme);
            if let Some(scheme) = scheme.filter(|scheme| {
                scheme.eq_ignore_ascii_case("otpauth")
                    || scheme.eq_ignore_ascii_case("otpauth-migration")
            }) {
                text.truncate(scheme.len() + 1);
            }
            let mut units = 0;
            if let Some((end, _)) = text.char_indices().find(|(_, c)| {
                units += c.len_utf16();
                units > 200
            }) {
                text.truncate(end);
            }
        }
    }
    let t = (NSProcessInfo::processInfo().systemUptime() * 1000.0).round();
    EVENTS.with_borrow_mut(|events| {
        let ring = events
            .entry(browser)
            .or_insert_with(|| VecDeque::with_capacity(LIMIT));
        if ring.len() == LIMIT {
            ring.pop_front();
        }
        ring.push_back(json!({"name": name, "payload": payload, "t": t}));
    });
}

pub fn recent_events(_: MainThreadMarker, browser: i32) -> Vec<Value> {
    EVENTS.with_borrow(|events| {
        events
            .get(&browser)
            .map_or_else(Vec::new, |ring| ring.iter().cloned().collect())
    })
}
