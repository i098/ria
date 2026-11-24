// Ported from ardia packages/ardiacore/ios/ArdiaPiPController+Swipe.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Two-finger swipes and mouse drags that move the Picture in Picture window, and their dev actions.

use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

use objc2::MainThreadMarker;
use objc2::rc::{Retained, Weak};
use objc2_app_kit::{
    NSApplication, NSCursor, NSEvent, NSEventModifierFlags, NSEventPhase, NSEventType, NSWindow,
    NSWorkspace,
};
use objc2_foundation::{NSPoint, NSProcessInfo, NSRect, NSString};
use serde_json::{Value, json};

use super::ardia_picture_in_picture::activate_with_only_picture_in_picture;
use super::ardia_picture_in_picture_internal::{self as internal, Edge, PEEK};
use super::ardia_pip_controller::{Controller, media_time};
use super::ardia_pip_seek_math as seek;
use super::ardia_pip_throw_math as throw;

thread_local! {
    static SWIPE_WINDOW: RefCell<Option<Weak<NSWindow>>> = const { RefCell::new(None) };
    static SWIPE_MOMENTUM: RefCell<Option<Weak<NSWindow>>> = const { RefCell::new(None) };
}

type Slot = std::thread::LocalKey<RefCell<Option<Weak<NSWindow>>>>;

fn load(slot: &'static Slot) -> Option<Retained<NSWindow>> {
    slot.with(|slot| slot.borrow().as_ref().and_then(Weak::load))
}

fn store(slot: &'static Slot, window: Option<&NSWindow>) {
    slot.with(|slot| *slot.borrow_mut() = window.map(Weak::new));
}

const SCROLL_EVENT_UNIT_PIXEL: u32 = 0;
const SCROLL_PHASE_BEGAN: i64 = 1;
const SCROLL_PHASE_CHANGED: i64 = 2;
const SCROLL_PHASE_ENDED: i64 = 4;
const MOMENTUM_PHASE_NONE: i64 = 0;
const MOMENTUM_PHASE_BEGIN: i64 = 1;
const MOMENTUM_PHASE_CONTINUE: i64 = 2;
const MOMENTUM_PHASE_END: i64 = 3;
const FIELD_DELTA_AXIS_1: u32 = 11;
const FIELD_DELTA_AXIS_2: u32 = 12;
const FIELD_IS_CONTINUOUS: u32 = 88;
const FIELD_FIXED_PT_DELTA_AXIS_1: u32 = 93;
const FIELD_FIXED_PT_DELTA_AXIS_2: u32 = 94;
const FIELD_POINT_DELTA_AXIS_1: u32 = 96;
const FIELD_POINT_DELTA_AXIS_2: u32 = 97;
const FIELD_SCROLL_PHASE: u32 = 99;
const FIELD_MOMENTUM_PHASE: u32 = 123;
const EVENT_FLAG_MASK_COMMAND: u64 = 0x0010_0000;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGWarpMouseCursorPosition(point: NSPoint) -> i32;
    fn CGEventCreateScrollWheelEvent2(
        source: *const c_void,
        units: u32,
        wheel_count: u32,
        wheel1: i32,
        wheel2: i32,
        wheel3: i32,
    ) -> *mut c_void;
    fn CGEventSetLocation(event: *mut c_void, location: NSPoint);
    fn CGEventSetIntegerValueField(event: *mut c_void, field: u32, value: i64);
    fn CGEventSetDoubleValueField(event: *mut c_void, field: u32, value: f64);
    fn CGEventSetFlags(event: *mut c_void, flags: u64);
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(object: *const c_void);
}

fn uptime() -> f64 {
    NSProcessInfo::processInfo().systemUptime()
}

fn reduce_motion() -> bool {
    NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
}

fn ns_point(p: throw::Point) -> NSPoint {
    NSPoint::new(p.x, p.y)
}

/// The top of the first screen: Core Graphics measures y down from it.
fn first_screen_top(mtm: MainThreadMarker) -> f64 {
    internal::screens(mtm)
        .first()
        .map_or(0.0, |screen| internal::max_y(screen.frame()))
}

/// `-[NSString doubleValue]` of the argument at `i`, or `fallback` when there is none.
fn number(args: &[String], i: usize, fallback: f64) -> f64 {
    args.get(i)
        .map_or(fallback, |arg| NSString::from_str(arg).doubleValue())
}

fn swipe_target_for(mtm: MainThreadMarker, event: &NSEvent) -> Option<Rc<Controller>> {
    let window = event.window(mtm).or_else(|| {
        let point = internal::pointer_of(mtm, event);
        NSApplication::sharedApplication(mtm)
            .orderedWindows()
            .iter()
            .find(|candidate| {
                candidate.isVisible() && internal::point_in_rect(point, candidate.frame())
            })
    })?;
    Controller::for_window(&window).filter(|_| window.isVisible())
}

impl Controller {
    pub fn abandon_swipe_on(window: &NSWindow) {
        if load(&SWIPE_WINDOW).is_some_and(|swiped| std::ptr::eq(&*swiped, window)) {
            store(&SWIPE_WINDOW, None);
        }
        if let Some(controller) = Controller::for_window(window) {
            controller.abandon_swipe();
        }
    }

    pub fn handle_swipe(event: &NSEvent) -> bool {
        let Some(mtm) = MainThreadMarker::new() else {
            return false;
        };
        if event.r#type() != NSEventType::ScrollWheel {
            return false;
        }
        let phase = event.phase();
        let momentum = event.momentumPhase();
        if momentum != NSEventPhase::None {
            let coasting = load(&SWIPE_MOMENTUM);
            if momentum.intersects(NSEventPhase::Ended | NSEventPhase::Cancelled) {
                store(&SWIPE_MOMENTUM, None);
            }
            if momentum.contains(NSEventPhase::Began)
                && let Some(controller) = coasting.as_deref().and_then(Controller::for_window)
            {
                controller.swipe_coasted(event);
            }
            return coasting.is_some();
        }
        if phase == NSEventPhase::None {
            return false;
        }
        if phase.intersects(NSEventPhase::Began | NSEventPhase::MayBegin) {
            if let Some(controller) = load(&SWIPE_MOMENTUM)
                .as_deref()
                .and_then(Controller::for_window)
            {
                controller.swipe_interrupted();
            }
            store(&SWIPE_MOMENTUM, None);
            if let Some(window) = load(&SWIPE_WINDOW) {
                Controller::abandon_swipe_on(&window);
            }
            if phase != NSEventPhase::Began
                || !internal::throw_enabled()
                || !event.hasPreciseScrollingDeltas()
                || event
                    .modifierFlags()
                    .contains(NSEventModifierFlags::Command)
            {
                return false;
            }
            let Some(controller) = swipe_target_for(mtm, event) else {
                return false;
            };
            store(&SWIPE_WINDOW, Some(&controller.window));
            activate_with_only_picture_in_picture(&controller, true);
            controller.swipe_began(event);
            return true;
        }
        let Some(window) = load(&SWIPE_WINDOW) else {
            return false;
        };
        let Some(controller) = Controller::for_window(&window) else {
            return false;
        };
        if phase.intersects(NSEventPhase::Ended | NSEventPhase::Cancelled) {
            store(&SWIPE_WINDOW, None);
            store(&SWIPE_MOMENTUM, Some(&window));
            controller.swipe_ended(event, phase.contains(NSEventPhase::Cancelled));
        } else {
            controller.swipe_moved(event);
        }
        true
    }

    pub fn swipe_ended(&self, event: &NSEvent, cancelled: bool) {
        let state = &self.state;
        self.move_by(throw::finger_delta(
            event.scrollingDeltaX(),
            event.scrollingDeltaY(),
            event.isDirectionInvertedFromDevice(),
        ));
        state.swiping.set(false);
        let delta = state.swipe_last_delta.get();
        let last = throw::Point {
            x: delta.x,
            y: delta.y,
        };
        let fresh = uptime() - state.swipe_last_at.get() <= throw::VELOCITY_WINDOW;
        let last_velocity = throw::swipe_velocity(last);
        if cancelled {
            if state.stashed_edge.get() != Edge::None && self.edge_beyond() == Edge::None {
                self.set_stashed_edge(Edge::None);
            }
            return self.swipe_settled_at(
                self.window.frame(),
                false,
                "cancelled",
                Edge::None,
                NSPoint::ZERO,
            );
        }
        let out = self.leave_tuck_if_pulled_out(if fresh {
            ns_point(last_velocity)
        } else {
            NSPoint::ZERO
        });
        if !internal::is_empty_rect(out) {
            return self.swipe_settled_at(out, true, "cameOut", Edge::None, NSPoint::ZERO);
        }
        let edge = self.edge_beyond();
        if edge != Edge::None {
            self.stash_at(edge);
            return self.swipe_settled_at(
                self.frame_stashed_at(edge),
                false,
                "tucked",
                edge,
                NSPoint::ZERO,
            );
        }
        if state.stashed_edge.get() != Edge::None {
            self.set_stashed_edge(Edge::None);
        }
        if reduce_motion() {
            return self.swipe_released(NSPoint::ZERO, "reduceMotion");
        }
        if fresh && throw::swipe_surely_flings(last) {
            return self.swipe_released(ns_point(last_velocity), "lastEvent");
        }
        state.awaiting_momentum.set(true);
        let me = self.me.clone();
        state.momentum_wait.schedule(throw::MOMENTUM_WAIT, move || {
            if let Some(me) = me.upgrade() {
                me.swipe_did_not_coast();
            }
        });
    }

    fn forget_momentum(&self) {
        if !self.state.awaiting_momentum.replace(false) {
            return;
        }
        self.state.momentum_wait.cancel();
    }

    fn swipe_did_not_coast(&self) {
        if !self.state.awaiting_momentum.replace(false) {
            return;
        }
        self.state.swipe_placed.set(self.window.frame());
        self.swipe_released(NSPoint::ZERO, "placed");
    }

    fn report_placement(&self) {
        let state = &self.state;
        if state.placement_stamp.borrow().is_null() {
            return;
        }
        state.placement_report.cancel();
        let stamp = state.placement_stamp.borrow().clone();
        self.report_release("placed", &stamp);
        *state.placement_stamp.borrow_mut() = Value::Null;
    }

    pub fn end_momentum_wait(&self) {
        self.report_placement();
        if !self.state.awaiting_momentum.get() {
            return;
        }
        self.forget_momentum();
        if !internal::background() {
            NSCursor::setHiddenUntilMouseMoves(false);
        }
        let stamp = self.state.swipe_stamp.borrow().clone();
        self.report_release("placed", &stamp);
    }

    fn swipe_interrupted(&self) {
        self.report_placement();
        if !self.state.awaiting_momentum.get() {
            return;
        }
        self.forget_momentum();
        self.swipe_settled_at(
            self.window.frame(),
            false,
            "interrupted",
            Edge::None,
            NSPoint::ZERO,
        );
    }

    pub fn swipe_coasted(&self, event: &NSEvent) {
        let state = &self.state;
        let awaiting = state.awaiting_momentum.get();
        let late = !awaiting
            && !state.placement_stamp.borrow().is_null()
            && !state.swiping.get()
            && !state.dragging.get()
            && !state.animating.get()
            && internal::equal_rects(self.window.frame(), state.swipe_placed.get());
        if !awaiting && !late {
            return;
        }
        state.placement_report.cancel();
        *state.placement_stamp.borrow_mut() = Value::Null;
        self.forget_momentum();
        let delta = throw::finger_delta(
            event.scrollingDeltaX(),
            event.scrollingDeltaY(),
            event.isDirectionInvertedFromDevice(),
        );
        let velocity = throw::swipe_velocity(delta);
        let flings = throw::drag_flings(velocity);
        self.swipe_released(
            if flings {
                ns_point(velocity)
            } else {
                NSPoint::ZERO
            },
            if flings { "flicked" } else { "coastedSlowly" },
        );
    }

    pub fn edge_flicked_past(&self, velocity: NSPoint) -> Edge {
        let frame = self.window.frame();
        let landing = throw::flick_landing(
            internal::throw_rect(frame),
            throw::Point {
                x: velocity.x,
                y: velocity.y,
            },
        );
        let visible = self
            .home_screen()
            .map_or(NSRect::ZERO, |screen| screen.visibleFrame());
        let aimed_at = self.edge_beyond_for(internal::from_throw_rect(landing));
        let already_past = if aimed_at == Edge::Right {
            internal::max_x(frame) > internal::max_x(visible)
        } else {
            internal::min_x(frame) < internal::min_x(visible)
        };
        if already_past { aimed_at } else { Edge::None }
    }

    pub fn report_release(&self, outcome: &str, stamp: &Value) {
        if stamp.is_null() {
            return;
        }
        let payload = internal::stamp_onto(stamp, json!({ "outcome": outcome }));
        if let Some(view) = self.view() {
            view.emit("pictureInPictureReleased", payload);
        }
    }

    fn swipe_released(&self, velocity: NSPoint, how: &str) {
        let frame = self.window.frame();
        let swipe_grab = self.state.swipe_grab.get();
        let grab = NSPoint::new(
            internal::min_x(frame) + swipe_grab.x,
            internal::min_y(frame) + swipe_grab.y,
        );
        let mut how = how;
        let mut edge = Edge::None;
        let mut destination = frame;
        let mut thrown = false;
        if velocity.x != 0.0 || velocity.y != 0.0 {
            edge = self.edge_flicked_past(velocity);
            if edge != Edge::None {
                self.stash_at(edge);
                destination = self.frame_stashed_at(edge);
                how = "flickedAway";
            } else if self.throw_from(grab, velocity) {
                thrown = true;
            }
        } else if self.settle_into_corner_pulled_into(NSPoint::ZERO) {
            thrown = true;
            how = "settledInCorner";
        }
        if thrown {
            let target = self.state.last_throw.borrow().get("target").cloned();
            destination = target
                .as_ref()
                .and_then(Value::as_str)
                .map_or(NSRect::ZERO, |target| {
                    objc2_foundation::NSRectFromString(&NSString::from_str(target))
                });
        }
        self.swipe_settled_at(destination, thrown, how, edge, velocity);
    }

    fn swipe_settled_at(
        &self,
        destination: NSRect,
        thrown: bool,
        how: &str,
        edge: Edge,
        velocity: NSPoint,
    ) {
        let state = &self.state;
        let swipe_grab = state.swipe_grab.get();
        let landing = NSPoint::new(
            internal::min_x(destination) + swipe_grab.x,
            internal::min_y(destination) + swipe_grab.y,
        );
        let unmoved = internal::equal_points(NSEvent::mouseLocation(), state.swipe_pointer.get());
        let warp = edge == Edge::None
            && unmoved
            && (!thrown || media_time() - state.swipe_began_at.get() >= 0.15);
        if !internal::background() {
            if warp {
                let at = NSPoint::new(landing.x, first_screen_top(self.mtm) - landing.y);
                // SAFETY: the function takes a point in global display coordinates.
                unsafe { CGWarpMouseCursorPosition(at) };
                NSCursor::setHiddenUntilMouseMoves(true);
            } else {
                NSCursor::setHiddenUntilMouseMoves(false);
            }
        }
        let outcome = match how {
            "cameOut" => "pullOut",
            "settledInCorner" => "cornerPull",
            "flickedAway" => "flickTuck",
            _ if edge != Edge::None => "tuck",
            _ if thrown => "corner",
            _ => "placed",
        };
        if how == "placed" {
            let stamp = state.swipe_stamp.borrow().clone();
            *state.placement_stamp.borrow_mut() = stamp;
            let me = self.me.clone();
            state
                .placement_report
                .schedule(throw::MOMENTUM_LATE, move || {
                    if let Some(me) = me.upgrade() {
                        me.report_placement();
                    }
                });
        } else {
            let stamp = state.swipe_stamp.borrow().clone();
            self.report_release(outcome, &stamp);
        }
        *state.last_swipe.borrow_mut() = json!({
            "outcome": outcome,
            "start": internal::rect_string(state.swipe_start.get()),
            "released": internal::rect_string(self.window.frame()),
            "lastDelta": internal::point_string(state.swipe_last_delta.get()),
            "velocity": internal::point_string(velocity),
            "how": how,
            "flings": velocity.x != 0.0 || velocity.y != 0.0,
            "thrown": thrown,
            "stashed": edge as i64,
            "warp": if warp { internal::point_string(landing) } else { String::new() },
        });
        self.rescue_if_lost();
    }

    pub fn settle_into_corner_pulled_into(&self, velocity: NSPoint) -> bool {
        let frame = self.window.frame();
        let visible = internal::throw_rect(
            self.home_screen()
                .map_or(NSRect::ZERO, |screen| screen.visibleFrame()),
        );
        let corner = throw::corner_pulled_into(
            internal::throw_rect(self.state.pull_start.get()),
            internal::throw_rect(frame),
            visible,
        );
        let Some(corner) = corner else {
            return false;
        };
        if !internal::throw_enabled() || reduce_motion() {
            return false;
        }
        let target = throw::corner_frame(
            corner,
            internal::width(frame),
            internal::height(frame),
            visible,
        );
        self.throw_to(internal::from_throw_rect(target), velocity, Some(corner));
        true
    }

    pub fn leave_tuck_if_pulled_out(&self, velocity: NSPoint) -> NSRect {
        let from = self.state.pulled_from.replace(Edge::None);
        if from == Edge::None {
            return NSRect::ZERO;
        }
        let frame = self.window.frame();
        let tucked_on = self.state.pulled_from_screen.borrow().clone();
        let visible = tucked_on
            .or_else(|| self.home_screen())
            .map_or(NSRect::ZERO, |screen| screen.visibleFrame());
        let from_tucked = if from == Edge::Right {
            internal::max_x(visible) - PEEK - internal::min_x(frame)
        } else {
            internal::max_x(frame) - (internal::min_x(visible) + PEEK)
        };
        if self.edge_beyond() != Edge::None && !throw::pulled_out_of_tuck(from_tucked) {
            return NSRect::ZERO;
        }
        if throw::pulled_past_half(
            internal::throw_rect(frame),
            internal::throw_rect(visible),
            from == Edge::Right,
        ) {
            return NSRect::ZERO;
        }
        self.leave_tuck_at(from, visible, velocity)
    }

    pub fn drag_began(&self, event: &NSEvent) {
        let state = &self.state;
        Controller::abandon_swipe_on(&self.window);
        self.stop_motion();
        state.pulled_from.set(state.stashed_edge.get());
        *state.pulled_from_screen.borrow_mut() = self.gesture_home();
        state.pull_start.set(self.window.frame());
        let pointer = internal::pointer_of(self.mtm, event);
        let frame = self.window.frame();
        state.dragging.set(true);
        state.drag_moving.set(false);
        state.drag_from.set(pointer);
        state.drag_grab.set(NSPoint::new(
            pointer.x - internal::min_x(frame),
            pointer.y - internal::min_y(frame),
        ));
        *state.drag_home.borrow_mut() = self.gesture_home();
        state.drag_began_at.set(event.timestamp());
        *state.drag_stamp.borrow_mut() = internal::begin_stamp();
        state.drag_off_cursor.set(0);
        state.drag_max_off.set(0.0);
        state.drag_slowest_move.set(0.0);
        state.drag_trace.borrow_mut().clear();
        let mut samples = state.drag_samples.borrow_mut();
        samples.clear();
        samples.push(throw::Sample {
            t: event.timestamp(),
            x: pointer.x,
            y: pointer.y,
        });
    }

    pub fn follow_pointer(&self, event: &NSEvent) {
        let state = &self.state;
        let pointer = internal::pointer_of(self.mtm, event);
        let from = state.drag_from.get();
        state.drag_moving.set(
            state.drag_moving.get()
                || !seek::is_click(seek::travel(pointer.x - from.x, pointer.y - from.y)),
        );
        if !state.drag_moving.get() {
            return;
        }
        {
            let mut samples = state.drag_samples.borrow_mut();
            if samples.len() >= 64 {
                samples.drain(..32);
            }
            samples.push(throw::Sample {
                t: event.timestamp(),
                x: pointer.x,
                y: pointer.y,
            });
        }
        let frame = self.window.frame();
        let screens = internal::throw_screens(self.mtm);
        let drag_grab = state.drag_grab.get();
        let at = throw::Point {
            x: pointer.x,
            y: pointer.y,
        };
        let grab = throw::Point {
            x: drag_grab.x,
            y: drag_grab.y,
        };
        let home = internal::screen_index(self.mtm, state.drag_home.borrow().as_deref());
        let next = throw::drag_to(internal::throw_rect(frame), at, grab, &screens, home);
        let origin = NSPoint::new(next.x, next.y);
        let before = media_time();
        if !internal::equal_points(origin, frame.origin) {
            self.window.setFrameOrigin(origin);
        }
        let move_ms = (media_time() - before) * 1000.0;
        let placed = self.window.frame();
        let held = throw::held_back(next, at, grab);
        let off = if held {
            0.0
        } else {
            (internal::min_x(placed) + drag_grab.x - pointer.x)
                .abs()
                .max((internal::min_y(placed) + drag_grab.y - pointer.y).abs())
        };
        if off >= 1.0 {
            state.drag_off_cursor.set(state.drag_off_cursor.get() + 1);
        }
        state.drag_max_off.set(state.drag_max_off.get().max(off));
        state
            .drag_slowest_move
            .set(state.drag_slowest_move.get().max(move_ms));
        let mut trace = state.drag_trace.borrow_mut();
        if trace.len() < 2400 {
            trace.push(json!([
                ((event.timestamp() - state.drag_began_at.get()) * 10000.0).round() / 10000.0,
                pointer.x,
                pointer.y,
                internal::min_x(placed),
                internal::min_y(placed),
                held,
                (move_ms * 1000.0).round() / 1000.0,
                ((uptime() - event.timestamp()) * 1e6).round() / 1000.0,
            ]));
        }
    }

    pub fn drag_moved(&self, event: &NSEvent) {
        if self.state.dragging.get() {
            self.follow_pointer(event);
        }
    }

    pub fn unstash_from_handle(&self) {
        let state = &self.state;
        if state.stashed_edge.get() == Edge::None {
            return;
        }
        self.unstash();
        let drag_stamp = state.drag_stamp.borrow().clone();
        let stamp = if drag_stamp.is_null() {
            internal::begin_stamp()
        } else {
            drag_stamp
        };
        let payload = internal::stamp_onto(&stamp, json!({}));
        if let Some(view) = self.view() {
            view.emit("pictureInPictureUnstashed", payload);
        }
    }

    pub fn drag_ended(&self, event: &NSEvent) {
        let state = &self.state;
        if !state.dragging.get() {
            return;
        }
        self.follow_pointer(event);
        state.dragging.set(false);
        if !state.drag_moving.get() {
            state.drag_samples.borrow_mut().clear();
            state.drag_trace.borrow_mut().clear();
            return;
        }
        let frame = self.window.frame();
        let drag_grab = state.drag_grab.get();
        let grabbed = NSPoint::new(
            internal::min_x(frame) + drag_grab.x,
            internal::min_y(frame) + drag_grab.y,
        );
        let velocity = throw::release_velocity(&state.drag_samples.borrow(), event.timestamp());
        state.drag_samples.borrow_mut().clear();
        let trace = state.drag_trace.take();
        let off_cursor = state.drag_off_cursor.get();
        let slowest = state.drag_slowest_move.get();
        *state.last_drag.borrow_mut() = json!({
            "grab": internal::point_string(drag_grab),
            "moves": trace.len(),
            "offCursorMoves": off_cursor,
            "maxOffCursor": (state.drag_max_off.get() * 100.0).round() / 100.0,
            "slowestMoveMs": (slowest * 1000.0).round() / 1000.0,
            "trace": trace,
        });
        let stamp = state.drag_stamp.borrow().clone();
        let payload = internal::stamp_onto(
            &stamp,
            json!({ "offCursor": off_cursor > 0, "slowMove": slowest > 8.0 }),
        );
        if let Some(view) = self.view() {
            view.emit("pictureInPictureDragged", payload);
        }
        self.drag_released_at(grabbed, ns_point(velocity));
    }

    pub fn dev_swipe(&self, args: &[String]) {
        let fx = number(args, 0, 0.0);
        let fy = number(args, 1, 0.0);
        let steps = args
            .get(2)
            .map_or(10, |arg| NSString::from_str(arg).intValue())
            .max(1);
        let mode = args.get(3).map_or("trackpad", String::as_str);
        let wheel = mode == "wheel";
        let command = mode == "command";
        let pause = if mode == "pause" { 0.3 } else { 0.0 };
        let window = self.window.clone();
        let mtm = self.mtm;
        let make = Rc::new(
            move |phase: i64, momentum: i64, fdx: f64, fdy: f64| -> Option<Retained<NSEvent>> {
                // SAFETY: a null source is allowed; the new event is released below.
                let cg = unsafe {
                    CGEventCreateScrollWheelEvent2(
                        std::ptr::null(),
                        SCROLL_EVENT_UNIT_PIXEL,
                        2,
                        0,
                        0,
                        0,
                    )
                };
                if cg.is_null() {
                    return None;
                }
                let f = window.frame();
                // SAFETY: `cg` is a live scroll wheel event and the fields are scroll wheel fields.
                unsafe {
                    CGEventSetLocation(
                        cg,
                        NSPoint::new(
                            internal::mid_x(f),
                            first_screen_top(mtm) - internal::mid_y(f),
                        ),
                    );
                    CGEventSetIntegerValueField(cg, FIELD_IS_CONTINUOUS, if wheel { 0 } else { 1 });
                    CGEventSetIntegerValueField(
                        cg,
                        FIELD_SCROLL_PHASE,
                        if wheel { 0 } else { phase },
                    );
                    CGEventSetIntegerValueField(cg, FIELD_MOMENTUM_PHASE, momentum);
                    CGEventSetFlags(cg, if command { EVENT_FLAG_MASK_COMMAND } else { 0 });
                }
                // SAFETY: `cg` points to a live CGEvent.
                let inverted = NSEvent::eventWithCGEvent(unsafe { &*cg.cast() })
                    .is_some_and(|event| event.isDirectionInvertedFromDevice());
                let sdx = if inverted { fdx } else { -fdx };
                let sdy = if inverted { -fdy } else { fdy };
                // SAFETY: as above.
                unsafe {
                    if wheel {
                        CGEventSetIntegerValueField(
                            cg,
                            FIELD_DELTA_AXIS_1,
                            (sdy / 10.0).round() as i64,
                        );
                        CGEventSetIntegerValueField(
                            cg,
                            FIELD_DELTA_AXIS_2,
                            (sdx / 10.0).round() as i64,
                        );
                    } else {
                        CGEventSetIntegerValueField(
                            cg,
                            FIELD_POINT_DELTA_AXIS_1,
                            sdy.round() as i64,
                        );
                        CGEventSetIntegerValueField(
                            cg,
                            FIELD_POINT_DELTA_AXIS_2,
                            sdx.round() as i64,
                        );
                        CGEventSetDoubleValueField(cg, FIELD_FIXED_PT_DELTA_AXIS_1, sdy);
                        CGEventSetDoubleValueField(cg, FIELD_FIXED_PT_DELTA_AXIS_2, sdx);
                    }
                }
                // SAFETY: `cg` points to a live CGEvent; the NSEvent keeps its own reference.
                let event = NSEvent::eventWithCGEvent(unsafe { &*cg.cast() });
                // SAFETY: `cg` came from a Create function, so this code owns one reference.
                unsafe { CFRelease(cg) };
                event
            },
        );
        let post = |seconds: f64, phase: i64, momentum: i64, fdx: f64, fdy: f64| {
            let make = Rc::clone(&make);
            internal::after(seconds, move || {
                if let Some(event) = make(phase, momentum, fdx, fdy) {
                    NSApplication::sharedApplication(mtm).postEvent_atStart(&event, false);
                }
            });
        };
        let interval = 1.0 / 120.0;
        let per_step_x = fx / f64::from(steps);
        let per_step_y = fy / f64::from(steps);
        post(0.0, SCROLL_PHASE_BEGAN, MOMENTUM_PHASE_NONE, 0.0, 0.0);
        for i in 1..=steps {
            post(
                f64::from(i) * interval,
                SCROLL_PHASE_CHANGED,
                MOMENTUM_PHASE_NONE,
                per_step_x,
                per_step_y,
            );
        }
        let end = f64::from(steps + 1) * interval + pause;
        post(end, SCROLL_PHASE_ENDED, MOMENTUM_PHASE_NONE, 0.0, 0.0);
        if mode == "momentum" {
            post(
                end + interval,
                0,
                MOMENTUM_PHASE_BEGIN,
                per_step_x,
                per_step_y,
            );
            for i in 2..=6 {
                let i = f64::from(i);
                post(
                    end + i * interval,
                    0,
                    MOMENTUM_PHASE_CONTINUE,
                    per_step_x / i,
                    per_step_y / i,
                );
            }
            post(end + 7.0 * interval, 0, MOMENTUM_PHASE_END, 0.0, 0.0);
        }
    }

    pub fn dev_drag(&self, args: &[String]) {
        let interval = number(args, 3, 8.0) / 1000.0;
        let pause = number(args, 4, 0.0) / 1000.0;
        let window = self.window.clone();
        let mtm = self.mtm;
        let frame = window.frame();
        let began = uptime();
        let grab = NSPoint::new(internal::width(frame) * 0.5, internal::height(frame) * 0.3);
        let start = NSPoint::new(
            internal::min_x(frame) + grab.x,
            internal::min_y(frame) + grab.y,
        );
        let post = |seconds: f64, kind: NSEventType, screen: NSPoint| {
            let window = window.clone();
            internal::after(seconds, move || {
                let f = window.frame();
                let local =
                    NSPoint::new(screen.x - internal::min_x(f), screen.y - internal::min_y(f));
                if let Some(event) = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
                    kind,
                    local,
                    NSEventModifierFlags::empty(),
                    began + seconds,
                    window.windowNumber(),
                    None,
                    0,
                    1,
                    if kind == NSEventType::LeftMouseUp { 0.0 } else { 1.0 },
                ) {
                    NSApplication::sharedApplication(mtm).postEvent_atStart(&event, false);
                }
            });
        };
        post(0.0, NSEventType::LeftMouseDown, start);
        let mut legs = vec![0];
        let mut i = 5;
        while i + 2 < args.len() {
            legs.push(i);
            i += 3;
        }
        let mut at = start;
        let mut t = 0.0;
        for i in legs {
            let fx = number(args, i, 0.0);
            let fy = number(args, i + 1, 0.0);
            let steps = (number(args, i + 2, 10.0) as i32).max(1);
            let from = at;
            for step in 1..=steps {
                let (step, count) = (f64::from(step), f64::from(steps));
                post(
                    t + step * interval,
                    NSEventType::LeftMouseDragged,
                    NSPoint::new(from.x + fx * step / count, from.y + fy * step / count),
                );
            }
            t += f64::from(steps) * interval;
            at = NSPoint::new(from.x + fx, from.y + fy);
        }
        post(t + pause + 0.004, NSEventType::LeftMouseUp, at);
    }
}
