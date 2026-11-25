// Ported from ardia apps/browser/src/lib/interfaceScale.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::interface_scale`. The logic is in `ria_state::shared::interface_scale`.

#![forbid(unsafe_code)]

use crate::js::as_number;
use ria_state::shared::interface_scale as i;
use serde_json::Value;

/// Port of `interfaceScalePercent` (`apps/browser/src/lib/interfaceScale.ts:9`).
pub fn interface_scale_percent(scale: f64) -> Result<f64, String> {
    Ok(i::interface_scale_percent(scale))
}

/// Port of `validInterfaceScale` (`apps/browser/src/lib/interfaceScale.ts:11`): only a number can be valid.
pub fn valid_interface_scale(value: Value) -> Result<bool, String> {
    Ok(as_number(&value).is_some_and(i::valid_interface_scale))
}

/// Port of `steppedInterfaceScale` (`apps/browser/src/lib/interfaceScale.ts:17`).
pub fn stepped_interface_scale(value: f64) -> Result<f64, String> {
    Ok(i::stepped_interface_scale(value))
}

/// Port of `sameInterfaceScale` (`apps/browser/src/lib/interfaceScale.ts:23`).
pub fn same_interface_scale(a: f64, b: f64) -> Result<bool, String> {
    Ok(i::same_interface_scale(a, b))
}

/// Port of `interfaceScaleLabel` (`apps/browser/src/lib/interfaceScale.ts:25`).
pub fn interface_scale_label(scale: f64) -> Result<String, String> {
    Ok(i::interface_scale_label(scale))
}

/// Port of `rootLength` (`apps/browser/src/lib/interfaceScale.ts:27`).
pub fn root_length(window_length: f64, scale: f64) -> Result<f64, String> {
    Ok(i::root_length(window_length, scale))
}

/// Port of `windowLength` (`apps/browser/src/lib/interfaceScale.ts:29`).
pub fn window_length(root_points: f64, scale: f64) -> Result<f64, String> {
    Ok(i::window_length(root_points, scale))
}

/// Port of `snappedToGrid` (`apps/browser/src/lib/interfaceScale.ts:31`).
pub fn snapped_to_grid(value: f64, pixels_per_point: f64) -> Result<f64, String> {
    Ok(i::snapped_to_grid(value, pixels_per_point))
}
