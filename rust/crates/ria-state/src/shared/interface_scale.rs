// Ported from ardia apps/browser/src/lib/interfaceScale.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The interface scale: 80 % to 150 % in steps of 5 %, and the lengths it scales.
//!
//! Changes from Ardia: `validInterfaceScale` takes an `f64`; the caller checks that an untyped value is a
//! number (`typeof value === "number"`, `interfaceScale.ts:12`).

use ria_core::identity::telemetry_wire::js_number_to_string;
use std::sync::LazyLock;

use super::js::js_round;

/// The bounds and step of the scale (`interfaceScale.ts:1`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InterfaceScale {
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub default: f64,
}

/// `INTERFACE_SCALE` (`interfaceScale.ts:1`).
pub const INTERFACE_SCALE: InterfaceScale = InterfaceScale {
    min: 0.8,
    max: 1.5,
    step: 0.05,
    default: 1.0,
};

// `Math.round` of the bounds and step of `INTERFACE_SCALE`, times 100 (`interfaceScale.ts:3-5`).
const PERCENT_STEP: f64 = 5.0;
const MIN_PERCENT: f64 = 80.0;
const MAX_PERCENT: f64 = 150.0;

/// `INTERFACE_SCALE_STEPS` (`interfaceScale.ts:7`): 0.8, 0.85, ... 1.5.
pub static INTERFACE_SCALE_STEPS: LazyLock<Vec<f64>> = LazyLock::new(|| {
    let count = ((MAX_PERCENT - MIN_PERCENT) / PERCENT_STEP) as usize + 1;
    (0..count)
        .map(|i| (MIN_PERCENT + i as f64 * PERCENT_STEP) / 100.0)
        .collect()
});

/// `interfaceScalePercent` (`interfaceScale.ts:9`).
pub fn interface_scale_percent(scale: f64) -> f64 {
    js_round(scale * 100.0)
}

/// `validInterfaceScale` (`interfaceScale.ts:11`): a whole step of the scale, within its bounds.
pub fn valid_interface_scale(value: f64) -> bool {
    if !value.is_finite() {
        return false;
    }
    let percent = interface_scale_percent(value);
    (value * 100.0 - percent).abs() < 1e-6
        && (MIN_PERCENT..=MAX_PERCENT).contains(&percent)
        && percent % PERCENT_STEP == 0.0
}

/// `steppedInterfaceScale` (`interfaceScale.ts:17`): the nearest step within the bounds.
pub fn stepped_interface_scale(value: f64) -> f64 {
    if !value.is_finite() {
        return INTERFACE_SCALE.default;
    }
    let percent =
        js_round(MAX_PERCENT.min(MIN_PERCENT.max(value * 100.0)) / PERCENT_STEP) * PERCENT_STEP;
    percent / 100.0
}

/// `sameInterfaceScale` (`interfaceScale.ts:23`).
pub fn same_interface_scale(a: f64, b: f64) -> bool {
    interface_scale_percent(a) == interface_scale_percent(b)
}

/// `interfaceScaleLabel` (`interfaceScale.ts:25`): `"110%"`.
pub fn interface_scale_label(scale: f64) -> String {
    format!("{}%", js_number_to_string(interface_scale_percent(scale)))
}

/// `rootLength` (`interfaceScale.ts:27`).
pub fn root_length(window_length: f64, scale: f64) -> f64 {
    window_length / scale
}

/// `windowLength` (`interfaceScale.ts:29`).
pub fn window_length(root_points: f64, scale: f64) -> f64 {
    root_points * scale
}

/// `snappedToGrid` (`interfaceScale.ts:31`): `value` on the nearest device pixel.
pub fn snapped_to_grid(value: f64, pixels_per_point: f64) -> f64 {
    js_round(value * pixels_per_point) / pixels_per_point
}
