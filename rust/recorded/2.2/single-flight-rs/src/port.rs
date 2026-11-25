// Ported from ardia apps/browser/src/lib/singleFlight.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The replay of `singleFlight.harness.ts` on `ria_state::shared::single_flight::SingleFlight`.
//! A call during a pass is a reentrant `run` that joins it (`None`); in TypeScript it returns the
//! running promise, so `run() === current` is `true` there.

#![forbid(unsafe_code)]

use ria_state::shared::single_flight::SingleFlight;
use serde_json::{Value, json};
use std::cell::{Cell, RefCell};

/// Port of `replaySingleFlight` (`apps/browser/src/lib/singleFlight.harness.ts:7`).
pub async fn replay_single_flight(passes: Value, keep: Value) -> Result<Vec<Value>, String> {
    let list = passes.as_array().cloned().unwrap_or_default();
    let answers = keep.as_array().cloned().unwrap_or_default();
    let log = RefCell::new(Vec::new());
    let (next, asked) = (Cell::new(0), Cell::new(0));
    let flight = SingleFlight::new();
    loop {
        let result = flight.run(
            |flight: &SingleFlight| {
                let i = next.replace(next.get() + 1);
                let flag = |key: &str| list.get(i).is_some_and(|p| p[key] == Value::Bool(true));
                log.borrow_mut().push(json!(["pass", i, flight.again()]));
                if flag("join") {
                    let joined = flight.run(|_| Ok::<(), String>(()), || true).is_none();
                    log.borrow_mut().push(json!(["join", joined]));
                }
                if flag("repeat") {
                    flight.repeat();
                }
                log.borrow_mut().push(json!(["again", flight.again()]));
                if flag("fail") {
                    Err(format!("pass {i}"))
                } else {
                    Ok(())
                }
            },
            || {
                let v = answers.get(asked.replace(asked.get() + 1)) != Some(&Value::Bool(false));
                log.borrow_mut().push(json!(["keepGoing", v]));
                v
            },
        );
        log.borrow_mut().push(match result {
            Some(Err(message)) => json!(["failed", message]),
            _ => json!(["done"]),
        });
        if next.get() >= list.len() {
            break;
        }
    }
    Ok(log.into_inner())
}
