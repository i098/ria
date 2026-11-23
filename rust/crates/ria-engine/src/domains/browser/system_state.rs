// Ported from ardia packages/ardiacore/ios/SystemState.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The system state that the engine reports as the `engine.systemState` event: power and memory.
//! The dispatch source, the IOKit run loop source and the power state observer are AppKit code of `ria-host-mac`;
//! this file holds the values and the rules.

use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MemoryPressure {
    #[default]
    Normal,
    Warning,
    Critical,
}

impl MemoryPressure {
    /// The level of a memory pressure event: critical wins over warning.
    pub fn from_flags(critical: bool, warning: bool) -> Self {
        if critical {
            Self::Critical
        } else if warning {
            Self::Warning
        } else {
            Self::Normal
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }
}

/// One power source of IOKit: the capacity of an internal battery.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerSource {
    pub internal_battery: bool,
    pub current: f64,
    pub max: f64,
}

/// The charge of the internal battery from 0 to 1. The last internal battery in the list wins.
pub fn battery_level(sources: &[PowerSource]) -> Option<f64> {
    sources
        .iter()
        .rfind(|s| s.internal_battery && s.max > 0.0)
        .map(|s| s.current / s.max)
}

#[derive(Debug, Default)]
pub struct SystemState {
    pub memory_pressure: MemoryPressure,
}

impl SystemState {
    /// The `engine.systemState` payload. `on_battery` is true when the providing power source is the battery.
    pub fn snapshot(
        &self,
        on_battery: bool,
        battery_level: Option<f64>,
        low_power_mode: bool,
        physical_memory: u64,
    ) -> Value {
        json!({
            "onBattery": on_battery,
            "batteryLevel": battery_level,
            "lowPowerMode": low_power_mode,
            "memoryPressure": self.memory_pressure.as_str(),
            "physicalMemory": physical_memory as f64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn critical_wins_over_warning() {
        assert_eq!(
            MemoryPressure::from_flags(true, true),
            MemoryPressure::Critical
        );
        assert_eq!(
            MemoryPressure::from_flags(false, true),
            MemoryPressure::Warning
        );
        assert_eq!(
            MemoryPressure::from_flags(false, false),
            MemoryPressure::Normal
        );
    }

    #[test]
    fn battery_level_skips_empty_and_external_sources() {
        let source = |internal_battery, current, max| PowerSource {
            internal_battery,
            current,
            max,
        };
        assert_eq!(battery_level(&[]), None);
        assert_eq!(
            battery_level(&[source(false, 1.0, 2.0), source(true, 1.0, 0.0)]),
            None
        );
        assert_eq!(battery_level(&[source(true, 1.0, 4.0)]), Some(0.25));
    }

    #[test]
    fn snapshot_has_the_wire_keys() {
        let state = SystemState {
            memory_pressure: MemoryPressure::Warning,
        };
        let v = state.snapshot(true, None, false, 8 << 30);
        assert_eq!(v["memoryPressure"], "warning");
        assert_eq!(v["batteryLevel"], Value::Null);
        assert_eq!(v["physicalMemory"], 8589934592.0);
    }
}
