// Ported from ardia packages/ardiacore/ios/Pager.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The native pager of the profile strip: drag tracking with rubber banding, then a critically damped spring to the
//! selected page, driven by the screen's display link with a 60 Hz timer as backstop.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::ptr::NonNull;
use std::rc::{self, Rc};
use std::sync::Mutex;

use block2::RcBlock;
use objc2::encode::{Encode, Encoding};
use objc2::rc::{Retained, Weak};
use objc2::runtime::NSObject;
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSScreen, NSWindow};
use objc2_foundation::{NSRunLoop, NSRunLoopCommonModes, NSTimer};
use serde_json::{Value, json};

use super::ardia_swipe::{SwipeHooks, hooks, perform_haptic};
use super::swipe_area::SwipeArea;

const TRACKING_SCALE: f64 = 0.5;
const RUBBER_DIMENSION: f64 = 255.0;
const RUBBER_COEFFICIENT: f64 = 0.15;
const FLICK_VELOCITY: f64 = 5.0;
const OMEGA: f64 = 2.0 * std::f64::consts::PI / 0.25;
const REST_DISPLACEMENT: f64 = 0.003;
const REST_SPEED: f64 = 0.1;
const HISTORY_WINDOW: f64 = 0.1;
const LINK_STALL: f64 = 0.05;

#[link(name = "QuartzCore", kind = "framework")]
unsafe extern "C" {
    fn CACurrentMediaTime() -> f64;
}

fn now() -> f64 {
    // SAFETY: CACurrentMediaTime has no preconditions.
    unsafe { CACurrentMediaTime() }
}

fn rubber_band(value: f64, dimension: f64) -> f64 {
    (1.0 - 1.0 / (value * RUBBER_COEFFICIENT / dimension + 1.0)) * dimension
}

/// `CAFrameRateRange`.
#[repr(C)]
struct FrameRateRange {
    minimum: f32,
    maximum: f32,
    preferred: f32,
}

// SAFETY: The layout matches `CAFrameRateRange`, three floats.
unsafe impl Encode for FrameRateRange {
    const ENCODING: Encoding = Encoding::Struct(
        "CAFrameRateRange",
        &[Encoding::Float, Encoding::Float, Encoding::Float],
    );
}

type Shared = Rc<RefCell<Pager>>;

pub struct FramesIvars {
    pager: rc::Weak<RefCell<Pager>>,
    generation: u64,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "RiaPagerFrames"]
    #[ivars = FramesIvars]
    struct PagerFrames;

    impl PagerFrames {
        #[unsafe(method(frame:))]
        fn frame(&self, link: &NSObject) {
            let ivars = self.ivars();
            let alive = ivars.pager.upgrade().is_some_and(|pager| pager.borrow_mut().frame(ivars.generation, true));
            if !alive {
                // SAFETY: `link` is the CADisplayLink that calls this target.
                let () = unsafe { msg_send![link, invalidate] };
            }
        }
    }
);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Idle,
    Tracking,
    Holding,
    Settling,
}

impl Phase {
    fn name(self) -> &'static str {
        match self {
            Phase::Idle => "idle",
            Phase::Tracking => "tracking",
            Phase::Holding => "holding",
            Phase::Settling => "settling",
        }
    }
}

#[derive(Default)]
struct Drag {
    start: f64,
    lo: i64,
    hi: i64,
    home: i64,
    detent: i64,
    area: Option<Weak<SwipeArea>>,
}

impl Drag {
    fn area(&self) -> Option<Retained<SwipeArea>> {
        self.area.as_ref()?.load()
    }

    fn is(&self, area: &SwipeArea) -> bool {
        self.area().is_some_and(|own| std::ptr::eq(&*own, area))
    }
}

#[derive(Clone, Copy)]
struct Spring {
    from: f64,
    target: i64,
    velocity: f64,
    start: f64,
}

thread_local! {
    static PAGERS: RefCell<HashMap<String, Shared>> = RefCell::default();
    static COUNTER: Cell<i64> = const { Cell::new(0) };
}

/// The pager states by key; `pagerState` reads them from any thread.
static SNAPSHOTS: Mutex<BTreeMap<String, Value>> = Mutex::new(BTreeMap::new());

fn snapshots() -> std::sync::MutexGuard<'static, BTreeMap<String, Value>> {
    SNAPSHOTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn next() -> i64 {
    COUNTER.set(COUNTER.get() + 1);
    COUNTER.get()
}

pub struct Pager {
    me: rc::Weak<RefCell<Pager>>,
    pub key: String,
    areas: Vec<Weak<SwipeArea>>,
    configured: bool,
    count: i64,
    order: Option<Vec<String>>,
    selected: i64,
    position: f64,
    velocity: f64,
    phase: Phase,
    sequence: i64,
    selection_sequence: i64,
    prop_selected: i64,
    prop_ack: i64,
    drag: Drag,
    history: Vec<(f64, f64)>,
    spring: Option<Spring>,
    link: Option<Retained<NSObject>>,
    last_link_frame: Rc<Cell<f64>>,
    timer: Option<Retained<NSTimer>>,
    generation: u64,
    #[cfg(debug_assertions)]
    last_input: Value,
}

impl Pager {
    pub fn named(key: &str) -> Shared {
        if let Some(pager) = Self::existing(key) {
            return pager;
        }
        let pager = Rc::new_cyclic(|me| RefCell::new(Self::new(me.clone(), key)));
        PAGERS.with_borrow_mut(|pagers| pagers.insert(key.to_owned(), pager.clone()));
        pager
    }

    pub fn existing(key: &str) -> Option<Shared> {
        PAGERS.with_borrow(|pagers| pagers.get(key).cloned())
    }

    pub fn snapshot(key: &str) -> Option<Value> {
        snapshots().get(key).cloned()
    }

    fn new(me: rc::Weak<RefCell<Pager>>, key: &str) -> Self {
        Self {
            me,
            key: key.to_owned(),
            areas: Vec::new(),
            configured: false,
            count: 1,
            order: None,
            selected: 0,
            position: 0.0,
            velocity: 0.0,
            phase: Phase::Idle,
            sequence: 0,
            selection_sequence: 0,
            prop_selected: 0,
            prop_ack: 0,
            drag: Drag::default(),
            history: Vec::new(),
            spring: None,
            link: None,
            last_link_frame: Rc::default(),
            timer: None,
            generation: 0,
            #[cfg(debug_assertions)]
            last_input: json!({}),
        }
    }

    fn areas(&self) -> Vec<Retained<SwipeArea>> {
        self.areas.iter().filter_map(Weak::load).collect()
    }

    pub fn attach(&mut self, area: &SwipeArea) {
        self.areas.retain(|known| {
            known
                .load()
                .is_some_and(|known| !std::ptr::eq(&*known, area))
        });
        self.areas.push(Weak::new(area));
        if self.configured {
            area.on_pager_position(json!({"position": self.position}));
        }
    }

    pub fn detach(&mut self, area: &SwipeArea) {
        self.areas.retain(|known| {
            known
                .load()
                .is_some_and(|known| !std::ptr::eq(&*known, area))
        });
        self.prune();
    }

    pub fn prune(&mut self) {
        let registered = Self::existing(&self.key)
            .is_some_and(|pager| std::ptr::eq(Rc::as_ptr(&pager), self.me.as_ptr()));
        if !self.areas().is_empty() || !registered {
            return;
        }
        self.stop_spring();
        PAGERS.with_borrow_mut(|pagers| pagers.remove(&self.key));
        snapshots().remove(&self.key);
    }

    pub fn configure(&mut self, selected: i64, count: i64, ack: i64, order: Option<Vec<String>>) {
        let count = count.max(1);
        self.prop_selected = selected.max(0).min(count - 1);
        self.prop_ack = self.prop_ack.max(ack);
        if !self.configured {
            self.configured = true;
            self.count = count;
            self.order = order;
            self.selected = self.prop_selected;
            self.position = self.selected as f64;
            self.sequence = next();
            self.publish();
            self.emit_position();
            return;
        }
        if count != self.count || (order.is_some() && order != self.order) {
            let old = std::mem::replace(&mut self.order, order);
            self.count = count;
            let moved = old
                .filter(|_| self.prop_ack < self.selection_sequence)
                .and_then(|old| old.get(self.selected as usize).cloned())
                .and_then(|id| self.order.as_ref()?.iter().position(|known| *known == id));
            self.cancel_motion();
            self.jump(
                moved.map_or(self.prop_selected, |index| index as i64),
                false,
            );
            return;
        }
        self.reconcile();
    }

    fn reconcile(&mut self) {
        if self.phase == Phase::Idle
            && self.prop_ack >= self.selection_sequence
            && self.prop_selected != self.selected
        {
            self.jump(self.prop_selected, false);
        }
    }

    fn jump(&mut self, index: i64, selection: bool) {
        self.announce_activity(|hooks, window| hooks.wake_neighbour_pages(window));
        self.selected = index;
        self.position = index as f64;
        self.velocity = 0.0;
        self.sequence = next();
        if selection {
            self.selection_sequence = self.sequence;
        }
        self.publish();
        self.emit_position();
    }

    pub fn input(
        &mut self,
        area: &SwipeArea,
        input: &str,
        distance: f64,
        direction: i32,
        timestamp: f64,
    ) {
        #[cfg(debug_assertions)]
        {
            self.last_input = json!({"phase": input, "distance": distance, "direction": direction, "timestamp": timestamp});
        }
        match input {
            "prepare" => self.prepare(),
            "abandon" => self.abandon(area),
            "began" => {
                self.begin(area);
                self.track(distance, direction, timestamp, true);
            }
            "changed" if self.phase == Phase::Tracking && self.drag.is(area) => {
                self.track(distance, direction, timestamp, true);
            }
            "ended" | "cancelled" if self.phase == Phase::Tracking && self.drag.is(area) => {
                self.track(distance, direction, timestamp, false);
                self.release(self.release_velocity(timestamp), input == "cancelled");
            }
            _ => {}
        }
    }

    fn prepare(&mut self) {
        self.announce_activity(|hooks, window| hooks.wake_neighbour_pages_for(window, 1.0, false));
        if self.phase == Phase::Settling {
            self.stop_spring();
            self.phase = Phase::Holding;
            self.publish();
        }
    }

    fn abandon(&mut self, area: &SwipeArea) {
        if self.phase == Phase::Holding {
            self.settle(self.selected, 0.0, None);
        } else if self.phase == Phase::Tracking && self.drag.is(area) {
            self.release(0.0, false);
        } else {
            self.announce_activity(|hooks, window| hooks.release_neighbour_pages(window));
        }
    }

    fn begin(&mut self, area: &SwipeArea) {
        self.announce_activity(|hooks, window| hooks.wake_neighbour_pages(window));
        self.stop_spring();
        let home = self.selected;
        let x = self.position;
        let lo = 0.max((home - 1).min(x.floor() as i64));
        let hi = (self.count - 1).min((home + 1).max(x.ceil() as i64));
        let detent = (x.round() as i64).max(lo).min(hi);
        self.drag = Drag {
            start: x,
            lo,
            hi,
            home,
            detent,
            area: Some(Weak::new(area)),
        };
        self.history.clear();
        self.velocity = 0.0;
        self.phase = Phase::Tracking;
        self.sequence = next();
        self.publish();
        self.emit_state("began");
    }

    fn track(&mut self, distance: f64, direction: i32, timestamp: f64, sample: bool) {
        let Some(area) = self.drag.area() else {
            return;
        };
        let width = area.page_width();
        if width <= 0.0 {
            return;
        }
        let sign = if direction > 0 { -1.0 } else { 1.0 };
        let raw = self.drag.start + sign * distance * TRACKING_SCALE / width;
        let (lo, hi) = (self.drag.lo as f64, self.drag.hi as f64);
        let x = if raw > hi {
            hi + rubber_band((raw - hi) * width, RUBBER_DIMENSION) / width
        } else if raw < lo {
            lo - rubber_band((lo - raw) * width, RUBBER_DIMENSION) / width
        } else {
            raw
        };
        self.position = x;
        if sample {
            let time = timestamp.max(self.history.last().map_or(timestamp, |last| last.0));
            self.history.push((time, x));
            self.history
                .retain(|entry| time - entry.0 <= HISTORY_WINDOW);
        }
        let nearest = (x.round() as i64).max(self.drag.lo).min(self.drag.hi);
        if nearest != self.drag.detent {
            self.drag.detent = nearest;
            perform_haptic("alignment");
        }
        self.publish();
        self.emit_position();
    }

    fn release_velocity(&self, timestamp: f64) -> f64 {
        let recent: Vec<_> = self
            .history
            .iter()
            .filter(|entry| timestamp - entry.0 <= HISTORY_WINDOW)
            .collect();
        match (recent.first(), recent.last()) {
            (Some(first), Some(last)) if recent.len() >= 2 && last.0 > first.0 => {
                (last.1 - first.1) / (last.0 - first.0)
            }
            _ => 0.0,
        }
    }

    fn release(&mut self, v: f64, cancelled: bool) {
        let home = self.drag.home;
        let area = self.drag.area();
        let width = area.as_ref().map_or(0.0, |area| area.page_width());
        self.drag.area = None;
        let target = if cancelled {
            home
        } else {
            release_target(self.position, v, width, home, self.count)
        };
        self.settle(target, self.towards(target, v), area.as_deref());
    }

    fn towards(&self, target: i64, v: f64) -> f64 {
        let d = target as f64 - self.position;
        if d != 0.0 && v != 0.0 && (d > 0.0) == (v > 0.0) {
            v
        } else {
            0.0
        }
    }

    pub fn switch_to(&mut self, index: i64, order: Option<Vec<String>>) -> bool {
        if !self.configured {
            return false;
        }
        self.sync(order);
        let target = index.max(0).min(self.count - 1);
        self.drag.area = None;
        let v = if self.phase == Phase::Settling {
            self.velocity
        } else {
            0.0
        };
        self.settle(target, self.towards(target, v), None);
        true
    }

    fn sync(&mut self, order: Option<Vec<String>>) {
        let Some(ids) = order.filter(|ids| Some(ids) != self.order.as_ref() && !ids.is_empty())
        else {
            return;
        };
        let current = self
            .order
            .as_ref()
            .and_then(|order| order.get(self.selected as usize))
            .cloned();
        let id = current
            .filter(|current| ids.contains(current))
            .unwrap_or_else(|| {
                ids[self.prop_selected.max(0).min(ids.len() as i64 - 1) as usize].clone()
            });
        self.select(&id, ids);
    }

    pub fn select(&mut self, profile_id: &str, ids: Vec<String>) -> bool {
        if !self.configured {
            return false;
        }
        let Some(index) = ids.iter().position(|id| id == profile_id) else {
            return false;
        };
        self.cancel_motion();
        self.count = ids.len() as i64;
        self.order = Some(ids);
        self.jump(index as i64, true);
        self.emit_state("selected");
        true
    }

    pub fn step(&mut self, delta: i64, wrap: bool, order: Option<Vec<String>>) -> bool {
        if !self.configured {
            return false;
        }
        self.sync(order);
        let target = if wrap {
            (self.selected + delta).rem_euclid(self.count)
        } else {
            (self.selected + delta).max(0).min(self.count - 1)
        };
        target == self.selected || self.switch_to(target, None)
    }

    fn settle(&mut self, target: i64, v: f64, origin: Option<&SwipeArea>) {
        self.announce_activity(|hooks, window| hooks.wake_neighbour_pages(window));
        if target != self.selected {
            self.selected = target;
            self.sequence = next();
            self.selection_sequence = self.sequence;
            self.publish();
            self.emit_state("selected");
        }
        self.stop_spring();
        self.phase = Phase::Settling;
        self.velocity = v;
        self.spring = Some(Spring {
            from: self.position,
            target,
            velocity: v,
            start: now(),
        });
        let screen = origin
            .and_then(|origin| origin.window())
            .and_then(|window| window.screen())
            .or_else(|| self.areas().iter().find_map(|area| area.window()?.screen()));
        if let Some(screen) = screen {
            self.start_link(&screen);
        }
        self.start_timer();
        self.publish();
    }

    fn start_link(&mut self, screen: &NSScreen) {
        let frames = PagerFrames::alloc(screen.mtm()).set_ivars(FramesIvars {
            pager: self.me.clone(),
            generation: self.generation,
        });
        // SAFETY: The allocated class has NSObject as its superclass.
        let frames: Retained<PagerFrames> = unsafe { msg_send![super(frames), init] };
        // SAFETY: `frame:` is the one-argument method of PagerFrames; the display link retains its target.
        let link: Retained<NSObject> =
            unsafe { msg_send![screen, displayLinkWithTarget: &*frames, selector: sel!(frame:)] };
        let fps = 60.max(screen.maximumFramesPerSecond()) as f32;
        let range = FrameRateRange {
            minimum: 60.0,
            maximum: fps,
            preferred: fps,
        };
        // SAFETY: CADisplayLink has these methods on macOS 14 and later.
        unsafe {
            let () = msg_send![&link, setPreferredFrameRateRange: range];
            let () = msg_send![&link, addToRunLoop: &*NSRunLoop::mainRunLoop(), forMode: NSRunLoopCommonModes];
        }
        self.link = Some(link);
        self.last_link_frame.set(now());
    }

    fn start_timer(&mut self) {
        let pager = self.me.clone();
        let generation = self.generation;
        let linked = self.link.is_some();
        let last_link_frame = self.last_link_frame.clone();
        let block = RcBlock::new(move |timer: NonNull<NSTimer>| {
            // SAFETY: NSTimer passes itself, alive, to its block.
            let timer = unsafe { timer.as_ref() };
            let Some(pager) = pager.upgrade() else {
                return timer.invalidate();
            };
            if linked && now() - last_link_frame.get() < LINK_STALL {
                return;
            }
            if !pager.borrow_mut().frame(generation, false) {
                timer.invalidate();
            }
        });
        // SAFETY: The block takes the timer, as NSTimer requires.
        let timer =
            unsafe { NSTimer::timerWithTimeInterval_repeats_block(1.0 / 60.0, true, &block) };
        // SAFETY: The main run loop is valid on the main thread.
        unsafe { NSRunLoop::mainRunLoop().addTimer_forMode(&timer, NSRunLoopCommonModes) };
        self.timer = Some(timer);
    }

    fn frame(&mut self, generation: u64, from_link: bool) -> bool {
        if generation != self.generation || self.spring.is_none() {
            return false;
        }
        if from_link {
            self.last_link_frame.set(now());
        }
        self.spring_step();
        generation == self.generation
    }

    fn spring_step(&mut self) {
        let Some(s) = self.spring else {
            return;
        };
        let (x, v) = spring_at(s, now() - s.start);
        if x.abs() < REST_DISPLACEMENT && v.abs() < REST_SPEED {
            self.stop_spring();
            self.position = s.target as f64;
            self.velocity = 0.0;
            self.phase = Phase::Idle;
            self.sequence = next();
            self.publish();
            self.emit_position();
            self.emit_state("settled");
            self.reconcile();
            return;
        }
        self.position = s.target as f64 + x;
        self.velocity = v;
        self.publish();
        self.emit_position();
    }

    fn stop_spring(&mut self) {
        self.generation += 1;
        if let Some(link) = self.link.take() {
            // SAFETY: `link` is a CADisplayLink.
            let () = unsafe { msg_send![&link, invalidate] };
        }
        if let Some(timer) = self.timer.take() {
            timer.invalidate();
        }
        self.spring = None;
    }

    fn cancel_motion(&mut self) {
        self.stop_spring();
        self.drag.area = None;
        self.phase = Phase::Idle;
        self.velocity = 0.0;
    }

    fn emit_position(&self) {
        let area = if self.phase == Phase::Tracking {
            self.drag.area()
        } else {
            None
        };
        let all = self.areas();
        let area = area
            .or_else(|| all.iter().find(|area| area.window().is_some()).cloned())
            .or_else(|| all.first().cloned());
        if let Some(area) = area {
            area.on_pager_position(json!({"position": self.position}));
        }
    }

    fn emit_state(&self, name: &str) {
        let mut event = json!({
            "phase": name, "sequence": self.sequence, "position": self.position, "selected": self.selected,
            "velocity": self.velocity,
        });
        if self.order.is_some() {
            event["profileId"] = self.selected_id();
        }
        for area in self.areas() {
            area.on_pager_state(event.clone());
        }
    }

    fn announce_activity(&self, tell: impl Fn(&dyn SwipeHooks, &NSWindow)) {
        let Some(hooks) = hooks() else {
            return;
        };
        let mut windows: Vec<Retained<NSWindow>> = Vec::new();
        for window in self.areas().iter().filter_map(|area| area.window()) {
            if !windows.iter().any(|known| std::ptr::eq(&**known, &*window)) {
                windows.push(window);
            }
        }
        for window in &windows {
            tell(&*hooks, window);
        }
    }

    fn publish(&self) {
        let mut state = json!({
            "sequence": self.sequence, "selectionSequence": self.selection_sequence, "position": self.position,
            "selected": self.selected, "phase": self.phase.name(), "velocity": self.velocity, "count": self.count,
        });
        if self.order.is_some() {
            state["profileId"] = self.selected_id();
        }
        snapshots().insert(self.key.clone(), state);
    }

    fn selected_id(&self) -> Value {
        self.order
            .as_ref()
            .and_then(|ids| ids.get(usize::try_from(self.selected).ok()?))
            .map_or(Value::Null, |id| Value::String(id.clone()))
    }

    #[cfg(debug_assertions)]
    pub fn debug(&self) -> Value {
        let mut out = Self::snapshot(&self.key).unwrap_or_else(|| json!({}));
        out["areas"] = json!(self.areas().len());
        out["propSelected"] = json!(self.prop_selected);
        out["propAck"] = json!(self.prop_ack);
        out["history"] = json!(self.history.len());
        out["lastInput"] = self.last_input.clone();
        if self.phase == Phase::Tracking {
            let d = &self.drag;
            out["drag"] = json!({"start": d.start, "lo": d.lo, "hi": d.hi, "home": d.home, "detent": d.detent});
        }
        out
    }
}

/// The page a release settles on: the nearest page, or the next page in the flick's direction, at most one page
/// from `home`.
fn release_target(position: f64, v: f64, width: f64, home: i64, count: i64) -> i64 {
    let flick = (v * width).abs() >= FLICK_VELOCITY;
    let target = if !flick {
        position.round() as i64
    } else if v > 0.0 {
        position.floor() as i64 + 1
    } else {
        position.ceil() as i64 - 1
    };
    0.max(home - 1).max((count - 1).min(home + 1).min(target))
}

/// The displacement from the target and the velocity of a critically damped spring at `t`.
fn spring_at(s: Spring, t: f64) -> (f64, f64) {
    let x0 = s.from - s.target as f64;
    let b = s.velocity + OMEGA * x0;
    let decay = (-OMEGA * t).exp();
    ((x0 + b * t) * decay, (s.velocity - OMEGA * b * t) * decay)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rubber_band_slows_and_caps_overdrag() {
        assert_eq!(rubber_band(0.0, RUBBER_DIMENSION), 0.0);
        assert!(rubber_band(100.0, RUBBER_DIMENSION) < 100.0);
        assert!(rubber_band(1e9, RUBBER_DIMENSION) < RUBBER_DIMENSION);
    }

    #[test]
    fn release_stays_within_one_page_of_home() {
        assert_eq!(release_target(1.4, 0.0, 300.0, 1, 5), 1);
        assert_eq!(release_target(1.2, 0.1, 300.0, 1, 5), 2);
        assert_eq!(release_target(1.2, -0.1, 300.0, 1, 5), 1);
        assert_eq!(release_target(3.9, 0.0, 300.0, 1, 5), 2);
        assert_eq!(release_target(-0.4, -1.0, 300.0, 0, 5), 0);
    }

    #[test]
    fn spring_starts_at_its_origin_and_comes_to_rest() {
        let s = Spring {
            from: 0.0,
            target: 1,
            velocity: 0.0,
            start: 0.0,
        };
        assert_eq!(spring_at(s, 0.0), (-1.0, 0.0));
        let (x, v) = spring_at(s, 2.0);
        assert!(x.abs() < REST_DISPLACEMENT && v.abs() < REST_SPEED);
    }
}
