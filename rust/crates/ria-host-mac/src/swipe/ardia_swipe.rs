// Ported from ardia packages/ardiacore/ios/ArdiaSwipe.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Trackpad swipes and wheel paging for the swipe areas of a window. Ardia's `ArdiaSwipeTarget` protocol has one
//! class, `SwipeArea`, so the targets here are swipe areas.
//!
//! Left for part 4 of slice 3.8: `tabs/` has no window-level `+[ArdiaChromeWindowHost rootViewOfWindow:]`, nor the
//! window-level neighbour-page wake of `ArdiaWebView+Painting.mm`. The host gives them through `set_hooks`.

use std::cell::{Cell, RefCell};
use std::ptr::NonNull;
use std::rc::Rc;

use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSEvent, NSEventMask, NSEventPhase, NSEventType, NSHapticFeedbackManager,
    NSHapticFeedbackPattern, NSHapticFeedbackPerformanceTime, NSHapticFeedbackPerformer, NSView,
    NSWindow,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use serde_json::{Value, json};

use super::ardia_swipe_hit_testing::{
    first_renderer, has_native_pager, hit_chain, hit_view, is_pager, native_content_scrolls,
    native_scroll_report, resolve_hit, target_at,
};
use super::ardia_swipe_internal::{
    GESTURE, Gesture, LEGACY_HIT_ROUTING, State, WRONG_RENDERER_HIT, diag, gesture, page_view_of,
    pointer, renderer_number, renderer_scroll_for, same, superview, tracing, window_number,
};
use super::swipe_area::SwipeArea;
use crate::tabs::ardia_renderer_scroll_observer::{RendererScroll, ScrollPhase};

mod simulation;
pub use simulation::simulate_in_window;

const START_DISTANCE: f64 = 4.0;
const START_DOMINANCE: f64 = 1.5;
const CANCEL_DOMINANCE: f64 = 2.0;
const MOMENTUM_START_WAIT: f64 = 0.1;

/// The window-level calls of other Ardia classes that `tabs/` and `windows/` do not have yet.
pub trait SwipeHooks {
    /// `+[ArdiaChromeWindowHost rootViewOfWindow:]`.
    fn root_view(&self, window: &NSWindow) -> Option<Retained<NSView>>;
    /// `+[ArdiaWebView wakeNeighbourPagesIn:]`.
    fn wake_neighbour_pages(&self, window: &NSWindow);
    /// `+[ArdiaWebView wakeNeighbourPagesIn:seconds:committed:]`.
    fn wake_neighbour_pages_for(&self, window: &NSWindow, seconds: f64, committed: bool);
    /// `+[ArdiaWebView releaseNeighbourPagesIn:]`.
    fn release_neighbour_pages(&self, window: &NSWindow);
}

#[derive(Default)]
struct OwnedMomentum {
    window: Option<Weak<NSWindow>>,
    ended_at: f64,
    armed: bool,
    flowing: bool,
}

thread_local! {
    static TARGETS: RefCell<Vec<Weak<SwipeArea>>> = RefCell::default();
    static HOOKS: RefCell<Option<Rc<dyn SwipeHooks>>> = RefCell::default();
    static GESTURE_SEQ: Cell<i32> = const { Cell::new(0) };
    static MONITOR: RefCell<Option<Retained<AnyObject>>> = RefCell::default();
    static PREPARED: RefCell<Option<Weak<SwipeArea>>> = RefCell::default();
    static MOMENTUM: RefCell<OwnedMomentum> = RefCell::default();
}

pub fn rect_value(r: NSRect) -> Value {
    json!([r.origin.x, r.origin.y, r.size.width, r.size.height])
}

pub fn targets() -> Vec<Retained<SwipeArea>> {
    TARGETS.with_borrow_mut(|targets| {
        targets.retain(|target| target.load().is_some());
        targets.iter().filter_map(Weak::load).collect()
    })
}

pub fn set_hooks(hooks: Rc<dyn SwipeHooks>) {
    HOOKS.set(Some(hooks));
}

pub fn hooks() -> Option<Rc<dyn SwipeHooks>> {
    HOOKS.with_borrow(Clone::clone)
}

fn direction_name(back: bool) -> &'static str {
    if back { "back" } else { "forward" }
}

fn available(target: &SwipeArea, back: bool) -> bool {
    if back {
        target.ivars().can_swipe_back.get()
    } else {
        target.ivars().can_swipe_forward.get()
    }
}

fn width(target: &SwipeArea) -> f64 {
    target.bounds().size.width
}

fn prepared() -> Option<Retained<SwipeArea>> {
    PREPARED.with_borrow(|prepared| prepared.as_ref()?.load())
}

fn release_prepared(timestamp: f64) {
    let Some(prepared) = PREPARED.take().and_then(|prepared| prepared.load()) else {
        return;
    };
    diag(|| json!({"diag": "pagerAbandon", "target": pointer(&*prepared)}));
    prepared.pager_input("abandon", 0.0, 0, timestamp);
}

fn target_points_per_window_point(target: &NSView) -> f64 {
    let unit = target.convertSize_fromView(NSSize::new(100.0, 100.0), None);
    if unit.width != 0.0 {
        unit.width.abs() / 100.0
    } else {
        1.0
    }
}

fn emit(phase: &str, velocity: f64) {
    let Some(target) = gesture(|g| g.target()) else {
        return;
    };
    let per_window_point = target_points_per_window_point(&target);
    let g = gesture(|g| {
        g.emitted += 1;
        g.clone()
    });
    let n = g.emitted;
    let distance = g.dx * f64::from(g.direction) * per_window_point;
    let dy = g.dy * per_window_point;
    let velocity = velocity * per_window_point;
    let direction = direction_name(g.direction > 0);
    diag(|| {
        json!({
            "diag": "emit", "seq": g.seq, "n": n, "phase": phase, "direction": direction,
            "distance": distance, "dx": g.dx, "dy": g.dy, "perWindowPoint": per_window_point,
            "velocity": velocity * f64::from(g.direction), "available": g.available,
            "width": width(&target), "target": pointer(&*target),
        })
    });
    if has_native_pager(&target) {
        target.pager_input(phase, distance, g.direction, g.time);
        return;
    }
    target.swipe_event(json!({
        "phase": phase,
        "direction": direction,
        "distance": distance,
        "dy": dy,
        "velocity": velocity * f64::from(g.direction),
        "available": g.available,
        "width": width(&target),
    }));
}

impl Gesture {
    pub fn sample(&mut self, timestamp: f64, delta_x: f64) {
        let i = self.samples % 6;
        self.samples += 1;
        self.times[i] = timestamp;
        self.deltas[i] = delta_x;
    }

    pub fn release_velocity(&self, now: f64) -> f64 {
        let mut sum = 0.0;
        let mut oldest = now;
        for k in 0..self.samples.min(6) {
            if now - self.times[k] > 0.1 {
                continue;
            }
            sum += self.deltas[k];
            oldest = oldest.min(self.times[k]);
        }
        if now - oldest > 0.004 {
            sum / (now - oldest)
        } else {
            0.0
        }
    }
}

fn handle_wheel(event: &NSEvent, window: Option<&NSWindow>, location: NSPoint) -> bool {
    let dx = event.scrollingDeltaX();
    if dx.abs() <= event.scrollingDeltaY().abs() {
        return false;
    }
    let (hit, target, _) = resolve_hit(window, location);
    let Some(target) = target.as_ref().filter(|target| is_pager(target)) else {
        let reject = if target.is_some() {
            "notPager"
        } else {
            "noTarget"
        };
        diag(|| json!({"diag": "wheel", "reject": reject, "hit": hit_chain(hit.as_deref())}));
        return false;
    };
    let back = dx > 0.0;
    let container = superview(target);
    if native_content_scrolls(
        hit.as_deref(),
        container.as_deref(),
        if back { 1 } else { -1 },
    ) {
        diag(|| {
            json!({
                "diag": "wheel",
                "reject": "nativeContentScrolls",
                "scrollers": native_scroll_report(hit.as_deref(), container.as_deref()),
            })
        });
        return false;
    }
    target.swipe_event(json!({
        "phase": "wheel",
        "direction": direction_name(back),
        "distance": dx.abs() * target_points_per_window_point(target),
        "dy": 0,
        "velocity": 0,
        "available": available(target, back),
        "width": width(target),
    }));
    true
}

fn prepare(window: Option<&NSWindow>, location: NSPoint, timestamp: f64) {
    release_prepared(timestamp);
    let (_, target, _) = resolve_hit(window, location);
    let Some(target) = target.filter(|target| has_native_pager(target)) else {
        return;
    };
    diag(|| json!({"diag": "pagerPrepare", "target": pointer(&*target)}));
    PREPARED.set(Some(Weak::new(&target)));
    target.pager_input("prepare", 0.0, 0, timestamp);
}

fn begin(
    window: Option<&NSWindow>,
    location: NSPoint,
    ignore_system_preference: bool,
    timestamp: f64,
) {
    let (state, old) = gesture(|g| (g.state, g.target()));
    if let Some(old) = old.filter(|old| state == State::Tracking && has_native_pager(old)) {
        old.pager_input("abandon", 0.0, 0, timestamp);
    }
    let seq = GESTURE_SEQ.get() + 1;
    GESTURE_SEQ.set(seq);
    GESTURE.set(Gesture {
        ignore_system_preference,
        seq,
        ..Gesture::default()
    });
    let (hit, target, fallback) = resolve_hit(window, location);
    if tracing() {
        trace_begin(
            window,
            location,
            hit.as_deref(),
            target.as_deref(),
            fallback,
            ignore_system_preference,
        );
    }
    let enabled = NSEvent::isSwipeTrackingFromScrollEventsEnabled() || ignore_system_preference;
    let target = match target {
        Some(target) if enabled || is_pager(&target) => target,
        target => {
            let reason = if target.is_some() {
                "systemPreferenceOff"
            } else if hit.is_some() {
                "noTarget"
            } else {
                "noHit"
            };
            diag(|| json!({"diag": "reject", "seq": seq, "reason": reason}));
            gesture(|g| g.state = State::Ignored);
            release_prepared(timestamp);
            return;
        }
    };
    if !same(prepared().as_deref(), Some(&*target)) {
        release_prepared(timestamp);
    }
    let renderer = hit.as_deref().and_then(renderer_scroll_for);
    if let Some(renderer) = &renderer {
        renderer.set(RendererScroll::default());
    }
    let renderer_hit = hit.as_deref().and_then(page_view_of).is_some();
    gesture(|g| {
        g.state = State::Pending;
        g.target = Some(Weak::new(&target));
        g.hit = hit.as_deref().map(Weak::new);
        g.renderer = renderer.as_ref().map(Rc::downgrade);
        g.renderer_hit = renderer_hit;
    });
}

fn trace_begin(
    window: Option<&NSWindow>,
    location: NSPoint,
    hit: Option<&NSView>,
    target: Option<&SwipeArea>,
    fallback: bool,
    ignore_system_preference: bool,
) {
    let seq = gesture(|g| g.seq);
    let mut why = Vec::new();
    let mut raw = window.and_then(|window| hit_view(window, location));
    if WRONG_RENDERER_HIT.get() {
        raw = window
            .and_then(NSWindow::contentView)
            .and_then(|content| first_renderer(&content))
            .or(raw);
    }
    if let Some(window) = window {
        target_at(window, location, raw.as_deref(), Some(&mut why));
    }
    diag(|| {
        json!({
            "diag": "hitRouting", "seq": seq, "raw": hit_chain(raw.as_deref()), "fallback": fallback,
            "wrongRendererHit": WRONG_RENDERER_HIT.get(), "legacyHitRouting": LEGACY_HIT_ROUTING.get(),
        })
    });
    let container = target.and_then(|target| superview(target));
    diag(|| {
        json!({
            "diag": "begin",
            "seq": seq,
            "window": window_number(window),
            "location": [location.x, location.y],
            "hit": hit_chain(hit),
            "targets": why,
            "target": target.map_or(String::new(), pointer),
            "canSwipeBack": target.is_some_and(|target| available(target, true)),
            "canSwipeForward": target.is_some_and(|target| available(target, false)),
            "scrollers": native_scroll_report(hit, container.as_deref()),
            "systemSwipe": NSEvent::isSwipeTrackingFromScrollEventsEnabled(),
            "ignorePreference": ignore_system_preference,
        })
    });
}

fn reject(reason: &str, extra: Option<Value>) -> bool {
    let (seq, dx, dy, time) = gesture(|g| (g.seq, g.dx, g.dy, g.time));
    diag(|| {
        let mut entry = json!({"diag": "reject", "seq": seq, "reason": reason, "dx": dx, "dy": dy});
        if let (Some(entry), Some(Value::Object(extra))) = (entry.as_object_mut(), extra) {
            entry.extend(extra);
        }
        entry
    });
    gesture(|g| g.state = State::Ignored);
    release_prepared(time);
    false
}

fn wait(reason: &str) -> bool {
    let (seq, dx, dy) = gesture(|g| (g.seq, g.dx, g.dy));
    diag(|| json!({"diag": "wait", "seq": seq, "reason": reason, "dx": dx, "dy": dy}));
    false
}

fn should_track(target: &SwipeArea) -> bool {
    let (dx, dy) = gesture(|g| (g.dx, g.dy));
    let vertical_intent = if is_pager(target) {
        3.0 * START_DISTANCE
    } else {
        START_DISTANCE
    };
    if dy.abs() >= vertical_intent && CANCEL_DOMINANCE * dx.abs() < dy.abs() {
        return reject("verticalIntent", None);
    }
    let below = if has_native_pager(target) {
        dx == 0.0
    } else {
        dx.abs() < START_DISTANCE
    };
    if below || START_DOMINANCE * dx.abs() < dy.abs() {
        return wait("belowStart");
    }
    let direction = if dx > 0.0 { 1 } else { -1 };
    let (renderer, hit) = gesture(|g| (g.renderer(), g.hit()));
    let container = superview(target);
    if let Some(renderer) = renderer {
        if let Some(refused) = renderer_refuses(renderer.get()) {
            return refused;
        }
    } else if native_content_scrolls(hit.as_deref(), container.as_deref(), direction) {
        let extra = tracing().then(|| {
            json!({"direction": direction, "scrollers": native_scroll_report(hit.as_deref(), container.as_deref())})
        });
        return reject("nativeContentScrolls", extra);
    }
    let available = available(target, direction > 0);
    if !available && !target.ivars().tracks_unavailable_directions.get() {
        return reject(
            "unavailable",
            tracing().then(|| json!({"direction": direction})),
        );
    }
    let seq = gesture(|g| {
        g.direction = direction;
        g.available = available;
        g.seq
    });
    diag(|| {
        json!({
            "diag": "track", "seq": seq, "direction": direction, "available": available, "dx": dx, "dy": dy,
            "scrollers": native_scroll_report(hit.as_deref(), container.as_deref()),
        })
    });
    true
}

/// `Some(false)` when the renderer state stops the swipe now; `None` when the swipe may track.
fn renderer_refuses(scroll: RendererScroll) -> Option<bool> {
    match scroll.phase {
        ScrollPhase::FirstUpdateConsumed => return Some(reject("rendererConsumed", None)),
        ScrollPhase::FirstUpdateUnconsumed => {}
        _ => {
            let reason = if tracing() {
                format!("renderer{}", renderer_number(scroll))
            } else {
                "rendererAck".into()
            };
            return Some(wait(&reason));
        }
    }
    match scroll.overscroll {
        None => Some(wait("rendererNoOverscroll")),
        Some(false) => Some(reject("overscrollBehavior", None)),
        Some(true) => None,
    }
}

fn swallow_owned_momentum(event: &NSEvent, window: Option<&NSWindow>) -> bool {
    let momentum = event.momentumPhase();
    let swallowed = MOMENTUM.with_borrow_mut(|owned| {
        if !owned.armed {
            return false;
        }
        let same_window = same(
            owned.window.as_ref().and_then(Weak::load).as_deref(),
            window,
        );
        if !owned.flowing {
            owned.flowing = same_window
                && momentum.contains(NSEventPhase::Began)
                && event.timestamp() - owned.ended_at <= MOMENTUM_START_WAIT;
            if !owned.flowing {
                *owned = OwnedMomentum::default();
                return false;
            }
        } else if !same_window {
            return false;
        }
        if momentum.intersects(NSEventPhase::Ended | NSEventPhase::Cancelled) {
            *owned = OwnedMomentum::default();
        }
        true
    });
    if swallowed {
        let seq = gesture(|g| g.seq);
        diag(|| json!({"diag": "ownMomentum", "seq": seq, "momentum": momentum.0}));
    }
    swallowed
}

/// Ardia's `HandleScroll`; `true` means that the swipe swallowed the event.
pub fn handle_scroll(
    event: &NSEvent,
    window: Option<&NSWindow>,
    location: NSPoint,
    ignore_system_preference: bool,
) -> bool {
    let phase = event.phase();
    if phase.is_empty() && event.momentumPhase().is_empty() {
        return handle_wheel(event, window, location);
    }
    if !event.hasPreciseScrollingDeltas() {
        diag(|| json!({"diag": "pass", "reason": "notPrecise"}));
        return false;
    }
    if phase.is_empty() {
        return swallow_owned_momentum(event, window);
    }
    MOMENTUM.take();
    let timestamp = event.timestamp();
    if phase.contains(NSEventPhase::MayBegin) {
        prepare(window, location, timestamp);
        return false;
    }
    if phase.contains(NSEventPhase::Began) {
        begin(window, location, ignore_system_preference, timestamp);
    }
    let ends = phase.intersects(NSEventPhase::Ended | NSEventPhase::Cancelled);
    if matches!(gesture(|g| g.state), State::Idle | State::Ignored) {
        if ends {
            release_prepared(timestamp);
        }
        return false;
    }
    let Some(target) = gesture(|g| g.target()) else {
        reject("targetGone", None);
        return false;
    };
    gesture(|g| {
        g.dx += event.scrollingDeltaX();
        g.dy += event.scrollingDeltaY();
        g.time = timestamp;
        g.sample(timestamp, event.scrollingDeltaX());
    });
    if ends {
        end(phase.contains(NSEventPhase::Ended), window, timestamp);
        return false;
    }
    track(&target, timestamp)
}

fn end(ended: bool, window: Option<&NSWindow>, timestamp: f64) {
    if gesture(|g| g.state) != State::Tracking {
        let (seq, dx, dy) = gesture(|g| (g.seq, g.dx, g.dy));
        diag(|| json!({"diag": "endUntracked", "seq": seq, "dx": dx, "dy": dy}));
        gesture(|g| g.state = State::Idle);
        release_prepared(timestamp);
        return;
    }
    emit(
        if ended { "ended" } else { "cancelled" },
        gesture(|g| g.release_velocity(timestamp)),
    );
    let renderer_hit = gesture(|g| {
        g.state = State::Idle;
        g.renderer_hit
    });
    if !renderer_hit {
        MOMENTUM.set(OwnedMomentum {
            window: window.map(Weak::new),
            ended_at: timestamp,
            armed: true,
            flowing: false,
        });
    }
}

fn track(target: &SwipeArea, timestamp: f64) -> bool {
    if gesture(|g| g.state) == State::Pending {
        if !should_track(target) {
            return false;
        }
        gesture(|g| g.state = State::Tracking);
        if same(prepared().as_deref(), Some(target)) {
            PREPARED.take();
        }
        emit("began", 0.0);
        return true;
    }
    let (dx, dy) = gesture(|g| (g.dx, g.dy));
    if !is_pager(target)
        && !target.ivars().allows_vertical_motion.get()
        && CANCEL_DOMINANCE * dx.abs() < dy.abs()
    {
        emit("cancelled", 0.0);
        gesture(|g| g.state = State::Ignored);
        return false;
    }
    emit("changed", gesture(|g| g.release_velocity(timestamp)));
    true
}

pub fn handle_discrete_swipe(window: Option<&NSWindow>, location: NSPoint, delta_x: f64) -> bool {
    if delta_x.abs() < 0.5 {
        return false;
    }
    let Some(window) = window else {
        return false;
    };
    let hit = hit_view(window, location);
    let Some(target) = target_at(window, location, hit.as_deref(), None) else {
        return false;
    };
    let back = delta_x > 0.0;
    if !available(&target, back) {
        return false;
    }
    target.swipe_event(json!({
        "phase": "swipe",
        "direction": direction_name(back),
        "distance": 0,
        "dy": 0,
        "velocity": 0,
        "available": true,
        "width": width(&target),
    }));
    true
}

fn monitor_event(mtm: MainThreadMarker, event: &NSEvent) -> bool {
    let window = event.window(mtm);
    let kind = event.r#type();
    diag(|| {
        json!({
            "diag": "monitorEntry",
            "type": kind.0,
            "window": window_number(window.as_deref()),
            "phase": event.phase().0,
            "momentum": if kind == NSEventType::ScrollWheel { event.momentumPhase().0 as i64 } else { -1 },
            "timestamp": event.timestamp(),
        })
    });
    let swallowed = if kind == NSEventType::Swipe {
        handle_discrete_swipe(window.as_deref(), event.locationInWindow(), event.deltaX())
    } else {
        handle_scroll(event, window.as_deref(), event.locationInWindow(), false)
    };
    let result_window = if swallowed {
        -1
    } else {
        window_number(window.as_deref())
    };
    diag(|| json!({"diag": "monitorExit", "resultWindow": result_window, "swallowed": swallowed}));
    swallowed
}

fn install_monitor(mtm: MainThreadMarker) {
    if MONITOR.with_borrow(Option::is_some) {
        return;
    }
    let handler = RcBlock::new(move |event: NonNull<NSEvent>| {
        // SAFETY: AppKit gives the monitor a live event on the main thread.
        if monitor_event(mtm, unsafe { event.as_ref() }) {
            std::ptr::null_mut()
        } else {
            event.as_ptr()
        }
    });
    // SAFETY: The handler returns the same event or null, as AppKit requires.
    let monitor = unsafe {
        NSEvent::addLocalMonitorForEventsMatchingMask_handler(
            NSEventMask::ScrollWheel | NSEventMask::Swipe,
            &handler,
        )
    };
    MONITOR.set(monitor);
}

pub fn perform_haptic(pattern: &str) {
    let pattern = match pattern {
        "levelChange" => NSHapticFeedbackPattern::LevelChange,
        "alignment" => NSHapticFeedbackPattern::Alignment,
        _ => NSHapticFeedbackPattern::Generic,
    };
    let time = if pattern == NSHapticFeedbackPattern::LevelChange {
        NSHapticFeedbackPerformanceTime::Default
    } else {
        NSHapticFeedbackPerformanceTime::Now
    };
    NSHapticFeedbackManager::defaultPerformer()
        .performFeedbackPattern_performanceTime(pattern, time);
}

pub fn add_target(target: &SwipeArea) {
    TARGETS.with_borrow_mut(|targets| {
        if !targets
            .iter()
            .any(|known| same(known.load().as_deref(), Some(target)))
        {
            targets.push(Weak::new(target));
        }
    });
    install_monitor(target.mtm());
}

pub fn remove_target(target: &SwipeArea) {
    TARGETS.with_borrow_mut(|targets| {
        targets.retain(|known| {
            known
                .load()
                .is_some_and(|known| !std::ptr::eq(&*known, target))
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_velocity_uses_only_the_last_tenth_of_a_second() {
        let mut g = Gesture::default();
        for (time, delta) in [(0.80, 50.0), (0.95, 10.0), (0.98, 10.0), (1.0, 10.0)] {
            g.sample(time, delta);
        }
        assert!((g.release_velocity(1.0) - 30.0 / 0.05).abs() < 1e-9);
        assert_eq!(g.release_velocity(0.952), 0.0);
    }

    #[test]
    fn samples_keep_the_last_six_deltas() {
        let mut g = Gesture::default();
        for i in 0..8 {
            g.sample(f64::from(i), f64::from(i));
        }
        assert_eq!(g.deltas, [6.0, 7.0, 2.0, 3.0, 4.0, 5.0]);
    }
}
