// Ported from ardia apps/browser/src/lib/pressMotion.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::press_motion`. The harness (`pressMotion.harness.ts`) replays steps; listeners log their index. The logic is in `ria_state::shared::press_motion`.

#![forbid(unsafe_code)]

use ria_state::shared::js::js_number;
use ria_state::shared::press_motion::{
    Listener, PRESS_MOTIONS, PressMotion, press_motion_now, set_press_motion,
    subscribe_press_motion,
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

/// `Number(step.slice(prefix.length))` as an index below `len`.
fn index(step: &str, prefix: &str, len: usize) -> Option<usize> {
    let n = js_number(step.strip_prefix(prefix)?);
    (n >= 0.0 && n.fract() == 0.0 && n < len as f64).then_some(n as usize)
}

/// Port of `replayPressMotion` (`apps/browser/src/lib/pressMotion.harness.ts:6`).
pub fn replay_press_motion(steps: Vec<String>) -> Result<Vec<Value>, String> {
    let called = Arc::new(Mutex::new(Vec::<usize>::new()));
    let mut listeners: Vec<Listener> = Vec::new();
    let mut unsubscribes: Vec<Box<dyn Fn() -> bool>> = Vec::new();
    let subscribe = |listener: Listener, unsubscribes: &mut Vec<Box<dyn Fn() -> bool>>| {
        unsubscribes.push(Box::new(subscribe_press_motion(listener)));
        json!(unsubscribes.len() - 1)
    };
    let out = steps
        .iter()
        .map(|step| match step.as_str() {
            "still" | "bounce" => {
                called.lock().unwrap().clear();
                set_press_motion(if step == "still" {
                    PressMotion::Still
                } else {
                    PressMotion::Bounce
                });
                json!(*called.lock().unwrap())
            }
            "now" => json!(press_motion_now().as_str()),
            "motions" => json!(PRESS_MOTIONS.map(PressMotion::as_str)),
            "subscribe" => {
                let id = listeners.len();
                let log = called.clone();
                listeners.push(Arc::new(move || log.lock().unwrap().push(id)));
                subscribe(listeners[id].clone(), &mut unsubscribes)
            }
            _ => {
                if let Some(i) = index(step, "subscribe:", listeners.len()) {
                    subscribe(listeners[i].clone(), &mut unsubscribes)
                } else if let Some(i) = index(step, "unsubscribe:", unsubscribes.len()) {
                    json!(unsubscribes[i]())
                } else {
                    Value::Null
                }
            }
        })
        .collect();
    for unsubscribe in &unsubscribes {
        unsubscribe();
    }
    set_press_motion(PressMotion::Still);
    Ok(out)
}
