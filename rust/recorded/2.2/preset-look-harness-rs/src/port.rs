// Ported from ardia apps/browser/src/lib/presetLook.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Replays `trackPress` of `ria_state::shared::preset_look` as `presetLook.harness.ts` does.
//! Arguments outside the TS types (for example a `color` that is not a colour name) are not covered:
//! the Rust side takes typed values, so the crate records with `--count 0` and only in-type inputs,
//! with the `null` and `undefined` cases where the TS really throws.

#![forbid(unsafe_code)]

use crate::js::{as_number, number};
use ria_state::shared::preset_look::track_press;
use serde_json::Value;

/// Port of `replayTrackPress` (`apps/browser/src/lib/presetLook.harness.ts:7`).
pub fn replay_track_press(steps: Vec<serde_json::Value>) -> Result<Vec<serde_json::Value>, String> {
    let mut press = track_press();
    let n = |step: &Value, key| step.get(key).and_then(as_number).unwrap_or(f64::NAN);
    Ok(steps
        .iter()
        .map(|step| match step.get("op").and_then(Value::as_str) {
            Some("armed") => Value::Bool(press.armed()),
            Some("grant") => {
                press.grant(n(step, "trackPixels"));
                Value::Null
            }
            Some("at") => {
                let snap_to = step.get("snapTo").and_then(as_number);
                press
                    .at(
                        n(step, "pageX"),
                        n(step, "trackX"),
                        n(step, "trackPixels"),
                        snap_to,
                    )
                    .map_or(Value::Null, number)
            }
            _ => Value::String("skip".into()),
        })
        .collect())
}
