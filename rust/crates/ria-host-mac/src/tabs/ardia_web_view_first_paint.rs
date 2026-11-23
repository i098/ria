// Ported from ardia packages/ardiacore/ios/ArdiaWebView+FirstPaint.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
mod native;
#[cfg(target_os = "macos")]
pub use native::PagePaintHold;

pub const HOLD_TIMEOUT: Duration = Duration::from_secs(4);
pub const LOADED_FRAME_TIMEOUT: Duration = Duration::from_millis(500);
pub const SCRIPTED_LOAD_TIMEOUT: Duration = Duration::from_secs(1);
pub const FRAME_INTERVAL: Duration = Duration::from_nanos(8_333_333);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintEnd {
    Painted,
    Loaded,
    Pressed,
    TimedOut,
    Detached,
}

#[derive(Default)]
pub struct FirstPaintHolds {
    pub held: u64,
    pub painted: u64,
    pub loaded: u64,
    pub pressed: u64,
    pub timed_out: u64,
    elapsed: Duration,
}

impl FirstPaintHolds {
    pub fn held_ms(&self) -> u64 {
        let rounded = (self.elapsed.as_nanos() + 500_000) / 1_000_000;
        u64::try_from(rounded).unwrap_or(u64::MAX)
    }

    fn note(&mut self, end: PaintEnd, elapsed: Duration) {
        match end {
            PaintEnd::Painted => self.painted += 1,
            PaintEnd::Loaded => self.loaded += 1,
            PaintEnd::Pressed => self.pressed += 1,
            PaintEnd::TimedOut => self.timed_out += 1,
            PaintEnd::Detached => return,
        }
        self.held += 1;
        self.elapsed += elapsed;
    }
}

#[derive(Default)]
pub struct FirstPaint {
    generation: u64,
    created: bool,
    scripted: bool,
    phase: Phase,
}

#[derive(Default)]
enum Phase {
    #[default]
    Ready,
    Held {
        since: Instant,
        cold_epoch: Option<String>,
        opacity: f32,
    },
    Settled,
}

pub struct PaintSurface {
    pub attached: bool,
    pub small_window: bool,
    pub temporary: bool,
    pub standalone: bool,
    pub extension_host: bool,
    pub visible_or_revealing: bool,
    pub opacity: f32,
}

impl PaintSurface {
    fn cold(&self, created: bool) -> bool {
        created
            && !self.small_window
            && !self.temporary
            && !self.standalone
            && !self.extension_host
            && self.visible_or_revealing
    }
}

impl FirstPaint {
    pub fn reset(&mut self, created: bool) {
        self.phase = Phase::Ready;
        self.created = created;
        self.scripted = false;
        self.generation = self.generation.wrapping_add(1);
    }

    pub fn take_created_while_loading(&mut self, loading: bool) -> bool {
        std::mem::take(&mut self.created) && loading
    }

    pub fn load_wait(&self) -> Option<(u64, bool)> {
        self.is_holding(self.generation)
            .then_some((self.generation, self.scripted))
    }

    fn can_hold(&self, created: bool, surface: &PaintSurface) -> bool {
        matches!(self.phase, Phase::Ready)
            && surface.attached
            && (surface.small_window || surface.cold(created))
    }

    pub fn hold(
        &mut self,
        created: bool,
        surface: PaintSurface,
        epoch: &str,
        now: Instant,
    ) -> Option<u64> {
        if !self.can_hold(created, &surface) {
            return None;
        }
        self.generation = self.generation.wrapping_add(1);
        self.phase = Phase::Held {
            since: now,
            cold_epoch: (surface.cold(created) && !epoch.is_empty()).then(|| epoch.to_owned()),
            opacity: surface.opacity,
        };
        Some(self.generation)
    }

    pub fn is_holding(&self, generation: u64) -> bool {
        self.generation == generation && matches!(self.phase, Phase::Held { .. })
    }

    pub fn settle(
        &mut self,
        generation: u64,
        end: PaintEnd,
        epoch: &str,
        now: Instant,
        counters: &mut FirstPaintHolds,
    ) -> Option<f32> {
        if !self.is_holding(generation) {
            return None;
        }
        let Phase::Held {
            since,
            cold_epoch,
            opacity,
        } = std::mem::replace(&mut self.phase, Phase::Settled)
        else {
            unreachable!();
        };
        self.generation = self.generation.wrapping_add(1);
        if cold_epoch.as_deref() == Some(epoch) {
            counters.note(end, now.saturating_duration_since(since));
        }
        Some(opacity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cold_surface() -> PaintSurface {
        PaintSurface {
            attached: true,
            small_window: false,
            temporary: false,
            standalone: false,
            extension_host: false,
            visible_or_revealing: true,
            opacity: 0.8,
        }
    }

    #[test]
    fn stale_callbacks_do_not_release_a_new_page() {
        let now = Instant::now();
        let mut state = FirstPaint::default();
        let mut counters = FirstPaintHolds::default();
        let old = state.hold(true, cold_surface(), "one", now).unwrap();
        state.reset(true);
        let current = state.hold(true, cold_surface(), "two", now).unwrap();
        assert_eq!(
            state.settle(old, PaintEnd::Painted, "two", now, &mut counters),
            None
        );
        assert!(state.is_holding(current));
    }

    #[test]
    fn only_a_live_consent_epoch_records_a_cold_paint() {
        let now = Instant::now();
        let mut state = FirstPaint::default();
        let mut counters = FirstPaintHolds::default();
        let stale = state.hold(true, cold_surface(), "old", now).unwrap();
        assert_eq!(
            state.settle(stale, PaintEnd::Painted, "new", now, &mut counters),
            Some(0.8)
        );
        assert_eq!(counters.held, 0);
        state.reset(true);
        let live = state.hold(true, cold_surface(), "new", now).unwrap();
        let end = now + Duration::from_micros(7_600);
        assert_eq!(
            state.settle(live, PaintEnd::Loaded, "new", end, &mut counters),
            Some(0.8)
        );
        assert_eq!(
            state.settle(live, PaintEnd::TimedOut, "new", end, &mut counters),
            None
        );
        assert_eq!(
            (counters.held, counters.loaded, counters.held_ms()),
            (1, 1, 8)
        );
    }
}
