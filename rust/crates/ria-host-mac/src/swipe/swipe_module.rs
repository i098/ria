// Ported from ardia packages/ardiacore/ios/SwipeModule.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The functions of the `ArdiaSwipe` module, wired to `Pager` and the swipe functions of `ardia_swipe`.
//! `ria_engine::domains::browser::swipe` holds the names; this file runs them. The view part of the module
//! (`SwipeArea` props and `devLocate`) is `swipe_area.rs`.
//!
//! Changes from Ardia: the module is a function `call` that takes the arguments as a JSON array, where Ardia's Expo
//! module binds typed arguments. The caller must be on the main thread for the functions of
//! `MAIN_THREAD_FUNCTIONS`; `haptic` hops to the main queue itself, as `SwipeModule.swift:8` does.

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSApplication, NSWindow};
use objc2_foundation::NSPoint;
use ria_engine::call::main_queue;
use ria_engine::domains::browser::swipe::{
    DEBUG_FUNCTIONS, FUNCTIONS, MAIN_THREAD_FUNCTIONS, debug_reply,
};
use serde_json::{Value, json};

use super::ardia_swipe::{perform_haptic, simulate_in_window};
use super::pager::Pager;

fn text(args: &[Value], at: usize) -> Option<&str> {
    args.get(at)?.as_str()
}

fn number(args: &[Value], at: usize) -> Option<i64> {
    args.get(at)?.as_i64()
}

/// `order: [String]?`: absent or `null` is `None`; a list with a non-string is not an order.
fn order(args: &[Value], at: usize) -> Option<Option<Vec<String>>> {
    match args.get(at) {
        None | Some(Value::Null) => Some(None),
        Some(Value::Array(ids)) => ids
            .iter()
            .map(|id| id.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
            .map(Some),
        Some(_) => None,
    }
}

fn pager_call(name: &str, args: &[Value]) -> Option<bool> {
    let pager = Pager::existing(text(args, 0)?);
    Some(match name {
        "switchPager" => {
            let (index, order) = (number(args, 1)?, order(args, 2)?);
            pager.is_some_and(|pager| pager.borrow_mut().switch_to(index, order))
        }
        "selectPager" => {
            let (id, ids) = (text(args, 1)?, order(args, 2)??);
            pager.is_some_and(|pager| pager.borrow_mut().select(id, ids))
        }
        _ => {
            let (delta, wrap, order) = (number(args, 1)?, args.get(2)?.as_bool()?, order(args, 3)?);
            pager.is_some_and(|pager| pager.borrow_mut().step(delta, wrap, order))
        }
    })
}

#[cfg(debug_assertions)]
fn pager_debug(key: &str) -> Value {
    Pager::existing(key).map_or(Value::Null, |pager| pager.borrow().debug())
}

#[cfg(not(debug_assertions))]
fn pager_debug(_key: &str) -> Value {
    Value::Null
}

/// Runs one function of the module that answers at once. `None`: the name is not one of those functions
/// (`devSimulate` has `dev_simulate`), or the arguments do not fit it.
pub fn call(name: &str, args: &[Value]) -> Option<Value> {
    if DEBUG_FUNCTIONS.contains(&name) && name != "devSimulate" {
        return debug_reply(name, || pager_debug(text(args, 0).unwrap_or_default()));
    }
    if !FUNCTIONS.contains(&name) && !MAIN_THREAD_FUNCTIONS.contains(&name) {
        return None;
    }
    match name {
        "haptic" => {
            let pattern = text(args, 0)?.to_owned();
            main_queue::post(Box::new(move || perform_haptic(&pattern)));
            Some(Value::Null)
        }
        "pagerState" => Some(Pager::snapshot(text(args, 0)?).unwrap_or(Value::Null)),
        _ => pager_call(name, args).map(Value::Bool),
    }
}

fn window_numbered(mtm: MainThreadMarker, number: i64) -> Option<Retained<NSWindow>> {
    NSApplication::sharedApplication(mtm).windowWithWindowNumber(number as isize)
}

/// `devSimulate(x, y, windowNumber, steps, ignorePreference)`. The reply comes through `completion`; a release
/// build answers `{"error": "DEV builds only"}` without a window lookup.
pub fn dev_simulate(
    args: &[Value],
    mtm: MainThreadMarker,
    completion: impl FnOnce(Value) + 'static,
) {
    if let Some(reply) = debug_reply("devSimulate", || Value::Null).filter(|reply| !reply.is_null())
    {
        return completion(reply);
    }
    let parsed = (|| {
        let point = NSPoint::new(args.first()?.as_f64()?, args.get(1)?.as_f64()?);
        let steps = args.get(3)?.as_array()?.clone();
        Some((point, number(args, 2)?, steps, args.get(4)?.as_bool()?))
    })();
    let Some((point, number, steps, ignore)) = parsed else {
        return completion(json!({"error": "bad arguments"}));
    };
    match window_numbered(mtm, number) {
        Some(window) => simulate_in_window(&window, point, steps, ignore, completion),
        None => completion(json!({"error": format!("no window {number}")})),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn pager_calls_without_a_pager_answer_false_and_null() {
        for name in ["switchPager", "selectPager", "stepPager"] {
            let args = match name {
                "switchPager" => vec![json!("none"), json!(1), Value::Null],
                "selectPager" => vec![json!("none"), json!("p"), json!(["p"])],
                _ => vec![json!("none"), json!(1), json!(true), Value::Null],
            };
            assert_eq!(call(name, &args), Some(json!(false)), "{name}");
        }
        assert_eq!(call("pagerState", &[json!("none")]), Some(Value::Null));
    }

    #[test]
    fn unknown_names_and_bad_arguments_are_refused() {
        assert_eq!(call("nope", &[]), None);
        assert_eq!(call("switchPager", &[json!("k"), json!("x")]), None);
        assert_eq!(
            call("selectPager", &[json!("k"), json!("p"), json!([1])]),
            None
        );
    }
}
