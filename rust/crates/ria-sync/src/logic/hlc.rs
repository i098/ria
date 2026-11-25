// Ported from ardia packages/sync/src/hlc.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The hybrid logical clock: `wall.counter.device`, with wall and counter in base 36.
//!
//! Changes from Ardia: none.

use super::js::{js_min, now_ms, parse_int36, to_string36};
use std::rc::Rc;

/// A timestamp text.
pub type Hlc = String;

const WALL_DIGITS: usize = 9;
const COUNTER_DIGITS: usize = 4;
/// `36 ** COUNTER_DIGITS - 1`.
const COUNTER_MAX: f64 = 1_679_615.0;

fn pad_start(text: String, width: usize) -> String {
    let len = text.encode_utf16().count();
    if len >= width {
        text
    } else {
        format!("{}{text}", "0".repeat(width - len))
    }
}

fn format_hlc(wall: f64, counter: f64, device: &str) -> Hlc {
    let floor = wall.floor();
    let wall = if floor.is_nan() || floor > 0.0 {
        floor
    } else {
        0.0
    };
    format!(
        "{}.{}.{device}",
        pad_start(to_string36(wall), WALL_DIGITS),
        pad_start(to_string36(counter), COUNTER_DIGITS)
    )
}

/// `[wall, counter, device]` of a timestamp; a missing part reads as `"0"`.
fn parse_hlc(h: &str) -> (f64, f64) {
    let mut parts = h.split('.');
    let wall = parts.next().unwrap_or("0");
    let counter = parts.next().unwrap_or("0");
    (parse_int36(wall), parse_int36(counter))
}

/// Port of `hlcWall` (`packages/sync/src/hlc.ts:15`).
pub fn hlc_wall(h: &str) -> f64 {
    parse_int36(&h.chars().take(WALL_DIGITS).collect::<String>())
}

/// The clock of one device; `now` is `Date.now` unless a caller gives another.
pub struct Clock {
    wall: f64,
    counter: f64,
    pub device: String,
    now: Rc<dyn Fn() -> f64>,
}

impl Clock {
    /// Port of the constructor (`packages/sync/src/hlc.ts:23`).
    pub fn new(device: &str, last: Option<&str>, now: Option<Rc<dyn Fn() -> f64>>) -> Self {
        let mut clock = Clock {
            wall: 0.0,
            counter: 0.0,
            device: device.to_string(),
            now: now.unwrap_or_else(|| Rc::new(now_ms)),
        };
        if let Some(last) = last.filter(|l| !l.is_empty()) {
            clock.observe(last);
        }
        clock
    }

    /// Port of `tick` (`packages/sync/src/hlc.ts:29`).
    pub fn tick(&mut self) -> Hlc {
        let t = (self.now)();
        if t > self.wall {
            self.wall = t;
            self.counter = 0.0;
        } else if self.counter >= COUNTER_MAX {
            self.wall += 1.0;
            self.counter = 0.0;
        } else {
            self.counter += 1.0;
        }
        format_hlc(self.wall, self.counter, &self.device)
    }

    /// Port of `at` (`packages/sync/src/hlc.ts:43`).
    pub fn at(&self, time: f64) -> Hlc {
        format_hlc(js_min(time, (self.now)()), 0.0, &self.device)
    }

    /// Port of `observe` (`packages/sync/src/hlc.ts:47`).
    pub fn observe(&mut self, h: &str) {
        let (wall, counter) = parse_hlc(h);
        if wall > self.wall || (wall == self.wall && counter > self.counter) {
            self.wall = wall;
            self.counter = counter;
        }
    }

    /// Port of `last` (`packages/sync/src/hlc.ts:55`).
    pub fn last(&self) -> Hlc {
        format_hlc(self.wall, self.counter, &self.device)
    }
}
