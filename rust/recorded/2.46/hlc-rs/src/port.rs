// Ria original, no Ardia source.

//! Replays the sessions of `hlc.harness.ts` on `ria_sync::logic::hlc`.

#![forbid(unsafe_code)]

use ria_sync::logic::hlc::{Clock, hlc_wall};
use serde_json::{Value, json};
use std::cell::Cell;
use std::rc::Rc;

fn number(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

/// Port of `replayClock` (`hlc.harness.ts:8`).
pub fn replay_clock(input: String) -> Result<Vec<Value>, String> {
    let sessions: Vec<Value> = serde_json::from_str(&input).map_err(|e| e.to_string())?;
    Ok(sessions
        .iter()
        .map(|s| {
            let time = Rc::new(Cell::new(number(&s["now"])));
            let now = Rc::clone(&time);
            let mut clock = Clock::new(
                s["device"].as_str().unwrap_or_default(),
                s["last"].as_str(),
                Some(Rc::new(move || now.get())),
            );
            let mut out = vec![json!(clock.device), json!(clock.last())];
            for step in s["steps"].as_array().into_iter().flatten() {
                match step[0].as_str() {
                    Some("now") => time.set(number(&step[1])),
                    Some("tick") => out.push(json!(clock.tick())),
                    Some("at") => out.push(json!(clock.at(number(&step[1])))),
                    Some("observe") => clock.observe(step[1].as_str().unwrap_or_default()),
                    Some("last") => out.push(json!(clock.last())),
                    _ => out.push(crate::js::number(hlc_wall(
                        step[1].as_str().unwrap_or_default(),
                    ))),
                }
            }
            Value::Array(out)
        })
        .collect())
}
