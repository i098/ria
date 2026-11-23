// Ported from ardia packages/ardiacore/ios/ArdiaWebView+InputMonitors.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
mod native;
#[cfg(target_os = "macos")]
pub use native::{
    CommandScrollMonitor, EventMonitor, PrewarmHooks, WheelTarget, ZoomHooks, install_prewarm,
};

const CREDIT_AGE: Duration = Duration::from_millis(500);
const REARM: Duration = Duration::from_secs(1);
const CREDIT_LIMIT: usize = 64;
const ZOOM_STEP: f64 = 30.0;

#[derive(Default)]
pub struct CommandWheel {
    armed: Option<Instant>,
    credits: VecDeque<(Instant, f64)>,
}

impl CommandWheel {
    pub fn arm(&mut self, now: Instant) -> bool {
        if self
            .armed
            .is_some_and(|armed| now.saturating_duration_since(armed) <= REARM)
        {
            return false;
        }
        self.armed = Some(now);
        true
    }

    pub fn disarm(&mut self) -> bool {
        self.armed.take().is_some()
    }

    pub fn offer(&mut self, now: Instant, delta: f64) -> bool {
        self.drop_stale(now);
        if self.credits.len() == CREDIT_LIMIT {
            self.credits.pop_front();
        }
        self.credits.push_back((now, delta));
        self.arm(now)
    }

    pub fn reply(&mut self, now: Instant, page_delta: f64, handled: bool) -> Option<f64> {
        if !page_delta.is_finite() {
            return None;
        }
        self.drop_stale(now);
        while let Some((_, credit)) = self.credits.pop_front() {
            if direction(credit) == direction(-page_delta) {
                return (!handled).then_some(credit);
            }
        }
        None
    }

    fn drop_stale(&mut self, now: Instant) {
        while self
            .credits
            .front()
            .is_some_and(|(at, _)| now.saturating_duration_since(*at) > CREDIT_AGE)
        {
            self.credits.pop_front();
        }
    }
}

fn direction(delta: f64) -> i32 {
    i32::from(delta > 0.0) - i32::from(delta < 0.0)
}

#[derive(Default)]
pub struct ScrollZoom {
    trackpad: bool,
    accumulated: f64,
}

impl ScrollZoom {
    pub fn candidate(
        &mut self,
        command: bool,
        begins: bool,
        no_phase_or_momentum: bool,
        trackpad: bool,
    ) -> bool {
        self.trackpad = if begins {
            trackpad
        } else {
            !no_phase_or_momentum && (self.trackpad || trackpad)
        };
        command && !self.trackpad
    }

    pub fn begin(&mut self) {
        self.accumulated = 0.0;
    }

    pub fn step(&mut self, delta: f64) -> Option<i32> {
        self.accumulated += delta;
        if self.accumulated.abs() < ZOOM_STEP {
            return None;
        }
        let step = direction(self.accumulated);
        self.accumulated = 0.0;
        Some(step)
    }
}

pub fn runs_page_script(url: &str) -> bool {
    ["https:", "http:", "file:"]
        .into_iter()
        .any(|scheme| url.starts_with(scheme))
}

#[cfg(target_os = "macos")]
pub fn can_wake_frames(
    tab: &crate::core::bindings::Tab,
    host: &objc2_app_kit::NSView,
    visible: bool,
    frozen: bool,
) -> bool {
    visible
        && !frozen
        && !tab.closed()
        && !tab.discarded()
        && !host.isHiddenOrHasHiddenAncestor()
        && runs_page_script(&tab.url().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_native_credit_is_consumed_once() {
        let now = Instant::now();
        let mut wheel = CommandWheel::default();
        assert!(wheel.offer(now, -30.0));
        assert!(!wheel.offer(now, 60.0));
        assert_eq!(wheel.reply(now, -13.0, false), Some(60.0));
        assert_eq!(wheel.reply(now, -13.0, false), None);
        assert_eq!(wheel.reply(now, 13.0, false), None);
    }

    #[test]
    fn invalid_or_stale_page_reply_cannot_zoom() {
        let now = Instant::now();
        let mut wheel = CommandWheel::default();
        wheel.offer(now, 30.0);
        assert_eq!(wheel.reply(now, f64::NAN, false), None);
        assert_eq!(wheel.reply(now + CREDIT_AGE, -30.0, true), None);
        wheel.offer(now, -30.0);
        assert_eq!(
            wheel.reply(now + CREDIT_AGE + Duration::from_nanos(1), 30.0, false),
            None
        );
    }

    #[test]
    fn a_trackpad_gesture_cannot_turn_into_a_wheel() {
        let mut zoom = ScrollZoom::default();
        assert!(!zoom.candidate(true, true, false, true));
        assert!(!zoom.candidate(true, false, false, false));
        assert!(zoom.candidate(true, false, true, false));
        assert_eq!(zoom.step(12.0), None);
        assert_eq!(zoom.step(18.0), Some(1));
        assert_eq!(zoom.step(18.0), None);
    }
}
