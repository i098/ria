// Ported from ardia apps/browser/src/lib/themeGesture.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The rig of `themeGesture.harness.ts` around `ria_state::shared::theme_gesture`: each dep call
//! is logged in order, a look stands for its `intensity`, and `current()` is a counter that the
//! rig bumps where the TS rig makes a new object.
//! Arguments outside the TS types (for example a `color` that is not a colour name) are not covered:
//! the Rust side takes typed values, so the crate records with `--count 0` and only in-type inputs,
//! with the `null` and `undefined` cases where the TS really throws.

#![forbid(unsafe_code)]

use crate::js::{as_number, number};
use ria_state::shared::profile_look::{
    ProfileTheme, ThemeColor, ThemeLook, ThemeMode, ThemeStop, ThemeTexture,
};
use ria_state::shared::theme_gesture::{ThemeEdit, ThemeGestureDeps, theme_gesture};
use serde_json::{Value, json};
use std::collections::BTreeMap;

struct Rig {
    log: Vec<Value>,
    frames: BTreeMap<u64, ()>,
    next: u64,
    showing: String,
    current: u64,
}

fn intensity(theme: Option<&ProfileTheme>) -> Value {
    theme.map_or(Value::Null, |t| number(t.look.intensity))
}

impl ThemeGestureDeps for Rig {
    type Current = u64;
    fn apply(&mut self, theme: Option<&ProfileTheme>) {
        self.log
            .push(json!(["apply", self.showing, intensity(theme)]));
        self.current += 1;
    }
    fn showing(&self) -> String {
        self.showing.clone()
    }
    fn current(&self) -> u64 {
        self.current
    }
    fn draft(&mut self, edit: Option<&ThemeEdit>) {
        let edit = edit.map_or(Value::Null, |e| {
            json!([intensity(e.theme.as_ref()), e.target])
        });
        self.log.push(json!(["draft", edit]));
    }
    fn request_frame(&mut self) -> u64 {
        let id = self.next;
        self.log.push(json!(["requestFrame", id]));
        self.frames.insert(id, ());
        self.next += 1;
        id
    }
    fn cancel_frame(&mut self, handle: u64) {
        self.log.push(json!(["cancelFrame", handle]));
        self.frames.remove(&handle);
    }
}

/// The `look(intensity)` of the TS rig.
fn look(intensity: f64) -> ProfileTheme {
    ProfileTheme {
        look: ThemeLook {
            stops: vec![ThemeStop {
                color: ThemeColor {
                    r: 0.5,
                    g: 0.4,
                    b: 0.6,
                    a: 1.0,
                },
                x: 0.5,
                y: 0.5,
            }],
            intensity,
            grain: 0.0,
            texture: ThemeTexture::Grain,
            translucency: 1.0,
            angle: None,
        },
        mode: ThemeMode::Auto,
        dark: None,
    }
}

/// Port of `replayThemeGesture` (`apps/browser/src/lib/themeGesture.harness.ts:11`).
pub fn replay_theme_gesture(
    showing: String,
    steps: Vec<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, String> {
    let mut g = theme_gesture(Rig {
        log: Vec::new(),
        frames: BTreeMap::new(),
        next: 1,
        showing,
        current: 0,
    });
    for step in &steps {
        let target = step
            .get("target")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        match step.get("op").and_then(Value::as_str) {
            Some("begin") => g.begin(&target),
            Some("change") => {
                let theme = step.get("intensity").and_then(as_number).map(look);
                g.change(
                    theme,
                    &target,
                    step.get("settle") == Some(&Value::Bool(true)),
                );
            }
            Some("selectionChanged") => g.selection_changed(),
            Some("cancel") => g.cancel(),
            Some("dispose") => g.dispose(),
            Some("show") => g.deps.showing = target,
            Some("outside") => g.deps.current += 1,
            Some("tick") => {
                let runs = std::mem::take(&mut g.deps.frames);
                runs.keys().for_each(|_| g.run_frame());
            }
            _ => g.deps.log.push(json!(["skip"])),
        }
        let waiting = g.deps.frames.len();
        g.deps.log.push(json!(["frames", waiting]));
    }
    Ok(g.deps.log)
}
