// Ported from ardia apps/browser/src/lib/time.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Relative times ("3 hours ago") and durations in minutes, hours or days.
//!
//! Changes from Ardia:
//! - `ago` takes `now` as a required argument; Ardia defaults it to `Date.now()`. Callers pass the clock.
//! - `durationMinutes` takes a [`DurationUnit`]; Ardia's object lookup gives `NaN`, and so `null`, for any other unit text.
//! - Numbers print with JavaScript `String(n)` rules (`NaN years ago`, `1e+21 days`).

use ria_core::identity::telemetry_wire::js_number_to_string;

use super::js::{js_max, js_number, js_round, js_trim};

/// `DurationUnit` (`time.ts:18`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurationUnit {
    Minutes,
    Hours,
    Days,
}

impl DurationUnit {
    /// The unit's name as Ardia writes it.
    pub fn name(self) -> &'static str {
        match self {
            DurationUnit::Minutes => "minutes",
            DurationUnit::Hours => "hours",
            DurationUnit::Days => "days",
        }
    }

    /// `MINUTES_PER[unit]` (`time.ts:20`).
    fn minutes(self) -> f64 {
        match self {
            DurationUnit::Minutes => 1.0,
            DurationUnit::Hours => 60.0,
            DurationUnit::Days => 1440.0,
        }
    }
}

/// `MAX_DURATION_MINUTES` (`time.ts:22`).
pub const MAX_DURATION_MINUTES: f64 = 365.0 * 1440.0;

/// `units` (`time.ts:1`).
fn units(n: f64, unit: &str) -> String {
    if n == 1.0 {
        format!("1 {unit} ago")
    } else {
        format!("{} {unit}s ago", js_number_to_string(n))
    }
}

/// `ago` (`time.ts:3`): how long before `now` the time `at` was, both in milliseconds.
pub fn ago(at: f64, now: f64) -> String {
    let s = js_max(0.0, (now - at) / 1000.0);
    if s < 60.0 {
        return "just now".into();
    }
    let m = (s / 60.0).floor();
    if m < 60.0 {
        return units(m, "minute");
    }
    let h = (m / 60.0).floor();
    if h < 24.0 {
        return units(h, "hour");
    }
    let d = (h / 24.0).floor();
    if d == 1.0 {
        return "yesterday".into();
    }
    if d < 7.0 {
        return format!("{} days ago", js_number_to_string(d));
    }
    if d < 30.0 {
        return units((d / 7.0).floor(), "week");
    }
    if d < 365.0 {
        return units((d / 30.0).floor(), "month");
    }
    units((d / 365.0).floor(), "year")
}

/// `durationParts` (`time.ts:24`): `(amount, unit)` in the largest unit that holds `minutes` exactly.
pub fn duration_parts(minutes: f64) -> (f64, DurationUnit) {
    // Rust `%` on `f64` keeps the sign of the dividend, as JavaScript `%` does.
    for unit in [DurationUnit::Days, DurationUnit::Hours] {
        if minutes % unit.minutes() == 0.0 {
            return (minutes / unit.minutes(), unit);
        }
    }
    (minutes, DurationUnit::Minutes)
}

/// `durationTitle` (`time.ts:30`): "1 hour", "45 minutes".
pub fn duration_title(minutes: f64) -> String {
    let (amount, unit) = duration_parts(minutes);
    let name = unit.name();
    format!(
        "{} {}",
        js_number_to_string(amount),
        if amount == 1.0 {
            &name[..name.len() - 1]
        } else {
            name
        }
    )
}

/// `durationMinutes` (`time.ts:35`): typed text as whole minutes, `None` outside 1 minute to one year.
pub fn duration_minutes(amount: &str, unit: DurationUnit) -> Option<f64> {
    let text = js_trim(amount);
    // `/^\d+(\.\d+)?$/`: `\d` is ASCII in JavaScript.
    let (int, frac) = text.split_once('.').unwrap_or((text, "0"));
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(int) || !digits(frac) {
        return None;
    }
    let minutes = js_round(js_number(text) * unit.minutes());
    (1.0..=MAX_DURATION_MINUTES)
        .contains(&minutes)
        .then_some(minutes)
}
