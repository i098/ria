// Ported from ardia packages/ardiacore/ios/PageEvents.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! `PageEvents`: the one event channel of a page view. A page event reaches the app only when the app asked for it
//! by name in the `pageEvents` prop.

use std::collections::HashSet;

use serde_json::{Map, Value, json};

/// The name of the event of the view (`PageEvents.channel`).
pub const CHANNEL: &str = "onPageEvent";

/// `PageEvents.send`: `dispatch` gets `{name, data}` when `wanted` holds `name`, and nothing otherwise.
pub fn send(
    dispatch: &dyn Fn(Value),
    wanted: &HashSet<String>,
    name: &str,
    payload: Map<String, Value>,
) {
    if wanted.contains(name) {
        dispatch(json!({ "name": name, "data": payload }));
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    #[test]
    fn only_wanted_names_reach_the_app() {
        let seen = RefCell::new(Vec::new());
        let dispatch = |value| seen.borrow_mut().push(value);
        let wanted = HashSet::from(["title".to_owned()]);
        let mut payload = Map::new();
        payload.insert("title".into(), json!("a"));
        send(&dispatch, &wanted, "title", payload.clone());
        send(&dispatch, &wanted, "url", payload);
        assert_eq!(
            *seen.borrow(),
            [json!({"name": "title", "data": {"title": "a"}})]
        );
    }
}
