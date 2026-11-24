// Ported from ardia packages/ardiacore/ios/ArdiaSwipe+Simulation.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! `devSimulate`: synthetic scroll events through the swipe code, with the trace of each step.

use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;
use std::time::Duration;

use objc2::encode::{Encode, Encoding};
use objc2::rc::Retained;
use objc2::{ClassType, MainThreadMarker, MainThreadOnly, Message, msg_send};
use objc2_app_kit::{NSApplication, NSEvent, NSMenuItem, NSScreen, NSView, NSWindow};
use objc2_foundation::{NSPoint, NSSelectorFromString, NSString};
use ria_engine::call::main_queue;
use serde_json::{Map, Value, json};

use super::super::ardia_swipe_hit_testing::{hit_view, target_at};
use super::super::ardia_swipe_internal::{
    LEGACY_HIT_ROUTING, State, TRACE, WRONG_RENDERER_HIT, gesture, page_view_of, renderer_number,
    window_number,
};
use super::{handle_discrete_swipe, handle_scroll, hooks};
use crate::core::bindings::PageView;
use crate::windows::ardia_interface_scale::root_of_window;

/// `CGEventRef`.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct CGEventRef(*mut c_void);

// SAFETY: `CGEventRef` is a pointer to the opaque struct `__CGEvent`.
unsafe impl Encode for CGEventRef {
    const ENCODING: Encoding = Encoding::Pointer(&Encoding::Struct("__CGEvent", &[]));
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreateScrollWheelEvent2(
        source: *const c_void,
        units: u32,
        wheel_count: u32,
        wheel1: i32,
        wheel2: i32,
        wheel3: i32,
    ) -> CGEventRef;
    fn CGEventSetIntegerValueField(event: CGEventRef, field: u32, value: i64);
    fn CGEventSetDoubleValueField(event: CGEventRef, field: u32, value: f64);
    fn CGEventSetFlags(event: CGEventRef, flags: u64);
    fn CGEventSetTimestamp(event: CGEventRef, timestamp: u64);
    fn CGEventSetLocation(event: CGEventRef, location: NSPoint);
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(object: *const c_void);
}

unsafe extern "C" {
    fn clock_gettime_nsec_np(clock: libc::clockid_t) -> u64;
}

const UNIT_PIXEL: u32 = 0;
const UNIT_LINE: u32 = 1;
const DELTA_AXIS_1: u32 = 11;
const DELTA_AXIS_2: u32 = 12;
const IS_CONTINUOUS: u32 = 88;
const FIXED_PT_DELTA_AXIS_1: u32 = 93;
const FIXED_PT_DELTA_AXIS_2: u32 = 94;
const POINT_DELTA_AXIS_1: u32 = 96;
const POINT_DELTA_AXIS_2: u32 = 97;
const SCROLL_PHASE: u32 = 99;
const MOMENTUM_PHASE: u32 = 123;
const WINDOW_NUMBER: u32 = 51;
const FLAG_MASK_SHIFT: u64 = 0x2_0000;

fn uptime_ns() -> u64 {
    // SAFETY: CLOCK_UPTIME_RAW is a valid clock.
    unsafe { clock_gettime_nsec_np(libc::CLOCK_UPTIME_RAW) }
}

fn number(step: &Value, key: &str) -> Option<f64> {
    step.get(key).and_then(Value::as_f64)
}

fn flag(step: &Value, key: &str) -> bool {
    step.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn responder_report(hit: Option<&NSView>) -> Value {
    let Some(page) = hit.and_then(page_view_of) else {
        return json!({});
    };
    let mut valid = Map::new();
    for action in [
        "checkSpelling:",
        "showGuessPanel:",
        "toggleContinuousSpellChecking:",
        "toggleGrammarChecking:",
        "startSpeaking:",
        "stopSpeaking:",
    ] {
        let title = NSString::from_str(action);
        let selector = NSSelectorFromString(&title);
        // SAFETY: The menu item only carries the selector; nothing sends it.
        let item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(page.mtm()),
                &title,
                Some(selector),
                &NSString::new(),
            )
        };
        // SAFETY: The page view of the engine implements NSUserInterfaceValidations.
        let ok: bool = unsafe { msg_send![&page, validateUserInterfaceItem: &*item] };
        valid.insert(action.into(), ok.into());
    }
    json!({"takesSwipes": PageView::takes_swipes(&page), "valid": valid})
}

fn window_point_from_root(window: &NSWindow, point: NSPoint) -> NSPoint {
    let root = root_of_window(window).or_else(|| hooks().and_then(|hooks| hooks.root_view(window)));
    let Some(root) = root else {
        let Some(content) = window.contentView() else {
            return point;
        };
        let local = if content.isFlipped() {
            point
        } else {
            NSPoint::new(point.x, content.bounds().size.height - point.y)
        };
        return content.convertPoint_toView(local, None);
    };
    let bounds = root.bounds();
    let y = if root.isFlipped() {
        bounds.min().y + point.y
    } else {
        bounds.max().y - point.y
    };
    root.convertPoint_toView(NSPoint::new(bounds.min().x + point.x, y), None)
}

fn scroll_phase_of(phase: &str) -> i64 {
    match phase {
        "began" => 1,
        "changed" => 2,
        "ended" => 4,
        "cancelled" => 8,
        "mayBegin" => 128,
        _ => 0,
    }
}

fn momentum_phase_of(phase: &str) -> i64 {
    match phase {
        "momentumBegan" => 1,
        "momentumEnded" => 3,
        _ => 2,
    }
}

/// Ardia's `SyntheticScroll`: a scroll event in `window` at `in_window`, as the trackpad or a wheel sends it.
fn synthetic_scroll(
    window: &NSWindow,
    in_window: NSPoint,
    step: &Value,
    start_ns: u64,
) -> Option<Retained<NSEvent>> {
    let phase = step["phase"].as_str().unwrap_or_default();
    let (dx, dy) = (
        number(step, "dx").unwrap_or(0.0),
        number(step, "dy").unwrap_or(0.0),
    );
    let wheel = phase == "wheel";
    let unit = if wheel { UNIT_LINE } else { UNIT_PIXEL };
    // SAFETY: A null source is allowed; the event has 2 axes.
    let cg = unsafe {
        CGEventCreateScrollWheelEvent2(
            std::ptr::null(),
            unit,
            2,
            dy.round() as i32,
            dx.round() as i32,
            0,
        )
    };
    if cg.0.is_null() {
        return None;
    }
    let int = |field, value| unsafe { CGEventSetIntegerValueField(cg, field, value) };
    let double = |field, value| unsafe { CGEventSetDoubleValueField(cg, field, value) };
    let int_of = |field, key| {
        if let Some(value) = number(step, key) {
            int(field, value as i64);
        }
    };
    int(IS_CONTINUOUS, i64::from(!wheel));
    let timestamp = number(step, "timestampMs")
        .map_or_else(uptime_ns, |ms| start_ns + (ms * 1e6).round() as u64);
    // SAFETY: `cg` is the live event made above.
    unsafe {
        if flag(step, "shift") {
            CGEventSetFlags(cg, FLAG_MASK_SHIFT);
        }
        CGEventSetTimestamp(cg, timestamp);
    }
    int_of(DELTA_AXIS_1, "dyLine");
    int_of(DELTA_AXIS_2, "dxLine");
    if !wheel {
        double(FIXED_PT_DELTA_AXIS_1, number(step, "dyFixed").unwrap_or(dy));
        double(FIXED_PT_DELTA_AXIS_2, number(step, "dxFixed").unwrap_or(dx));
    }
    int_of(POINT_DELTA_AXIS_1, "dyPoint");
    int_of(POINT_DELTA_AXIS_2, "dxPoint");
    if phase.starts_with("momentum") {
        int(MOMENTUM_PHASE, momentum_phase_of(phase));
    } else if !wheel {
        int(SCROLL_PHASE, scroll_phase_of(phase));
    }
    int_of(SCROLL_PHASE, "rawPhase");
    int_of(MOMENTUM_PHASE, "rawMomentumPhase");
    place(cg, window, in_window);
    // SAFETY: `cg` is a live CGEvent; NSEvent copies what it needs.
    let event: Option<Retained<NSEvent>> =
        unsafe { msg_send![NSEvent::class(), eventWithCGEvent: cg] };
    // SAFETY: `cg` came from a Create function and has one owner.
    unsafe { CFRelease(cg.0) };
    event
}

/// Puts the event at `in_window` on the screen and in `window`.
fn place(cg: CGEventRef, window: &NSWindow, in_window: NSPoint) {
    let screen = window.convertPointToScreen(in_window);
    let main_height = NSScreen::screens(window.mtm())
        .firstObject()
        .map_or(0.0, |main| main.frame().size.height);
    // SAFETY: `RTLD_DEFAULT` searches every loaded image; the private symbol may be missing.
    let set_window_location =
        unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"CGEventSetWindowLocation".as_ptr()) };
    // SAFETY: `cg` is a live event; the window location function, when present, takes the event and a point.
    unsafe {
        CGEventSetLocation(cg, NSPoint::new(screen.x, main_height - screen.y));
        CGEventSetIntegerValueField(cg, WINDOW_NUMBER, window.windowNumber() as i64);
        if !set_window_location.is_null() {
            let set: unsafe extern "C" fn(CGEventRef, NSPoint) =
                std::mem::transmute(set_window_location);
            set(
                cg,
                NSPoint::new(in_window.x, window.frame().size.height - in_window.y),
            );
        }
    }
}

struct Run {
    window: Retained<NSWindow>,
    in_window: NSPoint,
    steps: Vec<Value>,
    ignore_system_preference: bool,
    start_ns: u64,
    hit: Option<Retained<NSView>>,
    place: Value,
    index: usize,
    log: Vec<Value>,
    completion: Option<Box<dyn FnOnce(Value)>>,
}

/// `+[ArdiaSwipe simulateInWindow:point:steps:ignoreSystemPreference:completion:]`. `point` is in the root view,
/// from its top left.
pub fn simulate_in_window(
    window: &NSWindow,
    point: NSPoint,
    steps: Vec<Value>,
    ignore_system_preference: bool,
    completion: impl FnOnce(Value) + 'static,
) {
    TRACE.set(Some(Vec::new()));
    let start_ns = uptime_ns();
    let in_window = window_point_from_root(window, point);
    let hit = hit_view(window, in_window);
    let place = json!({
        "hit": hit.as_deref().map_or(String::new(), |hit| hit.class().name().to_string_lossy().into_owned()),
        "target": target_at(window, in_window, hit.as_deref(), None).is_some(),
        "window": window.windowNumber(),
        "systemSwipeEnabled": NSEvent::isSwipeTrackingFromScrollEventsEnabled(),
    });
    let run = Rc::new(RefCell::new(Run {
        window: window.retain(),
        in_window,
        steps,
        ignore_system_preference,
        start_ns,
        hit,
        place,
        index: 0,
        log: Vec::new(),
        completion: Some(Box::new(completion)),
    }));
    step(run);
}

fn step(run: Rc<RefCell<Run>>) {
    let (s, index, count) = {
        let r = run.borrow();
        (r.steps.get(r.index).cloned(), r.index, r.steps.len())
    };
    let Some(s) = s else {
        finish(&run);
        return;
    };
    run.borrow_mut().index = index + 1;
    let start_ns = run.borrow().start_ns;
    let at_ms = (uptime_ns() - start_ns) as f64 / 1e6;
    let entry = run_step(&run.borrow(), &s, at_ms);
    run.borrow_mut().log.extend(entry);
    let following = (index + 1 < count).then(|| run.borrow().steps[index + 1].clone());
    let delay_ns = match following
        .as_ref()
        .and_then(|following| number(following, "atMs"))
    {
        Some(at) => (start_ns as i64 + (at * 1e6).round() as i64) - uptime_ns() as i64,
        None => (number(&s, "delayMs").unwrap_or(16.0) * 1e6).round() as i64,
    };
    let delay = Duration::from_nanos(delay_ns.max(0) as u64);
    main_queue::after(delay, Box::new(move || step(run)));
}

fn finish(run: &Rc<RefCell<Run>>) {
    let mut r = run.borrow_mut();
    let result = json!({
        "where": r.place,
        "startNs": r.start_ns,
        "events": std::mem::take(&mut r.log),
        "acks": TRACE.take().unwrap_or_default(),
        "responder": responder_report(r.hit.as_deref()),
    });
    if let Some(completion) = r.completion.take() {
        drop(r);
        completion(result);
    }
}

/// One step; the log entries it adds.
fn run_step(run: &Run, s: &Value, at_ms: f64) -> Vec<Value> {
    let phase = s["phase"].as_str().unwrap_or_default();
    if phase == "swipe" {
        let handled = handle_discrete_swipe(
            Some(&run.window),
            run.in_window,
            number(s, "dx").unwrap_or(0.0),
        );
        return vec![json!({"phase": phase, "swallowed": handled})];
    }
    let Some(event) = synthetic_scroll(&run.window, run.in_window, s, run.start_ns) else {
        return vec![json!({"phase": phase, "error": "no event"})];
    };
    let mtm = run.window.mtm();
    let mut log = Vec::new();
    let location = event.locationInWindow();
    let event_window = event.window(mtm);
    if !same_window(event_window.as_deref(), &run.window)
        || (location.x - run.in_window.x).abs() > 1.0
        || (location.y - run.in_window.y).abs() > 1.0
    {
        let shown = event_window.map_or("(null)".to_owned(), |window| format!("{window:?}"));
        log.push(json!({
            "phase": phase,
            "error": format!(
                "event window {shown} at {{{}, {}}}, expected {{{}, {}}}",
                location.x, location.y, run.in_window.x, run.in_window.y
            ),
        }));
    }
    WRONG_RENDERER_HIT.set(flag(s, "wrongRendererHit"));
    LEGACY_HIT_ROUTING.set(flag(s, "legacyHitRouting"));
    let late_ms = number(s, "atMs").map_or(0.0, |at| at_ms - at);
    let mut entry = json!({
        "phase": phase,
        "time": event.timestamp(),
        "atMs": at_ms,
        "lateMs": late_ms,
        "nsPhase": event.phase().0,
        "nsMomentumPhase": event.momentumPhase().0,
        "scrollingDeltaX": event.scrollingDeltaX(),
        "scrollingDeltaY": event.scrollingDeltaY(),
    });
    if s["dispatch"].as_str() == Some("app") {
        dispatch_to_app(mtm, &event, &mut entry);
    } else {
        let swallowed = handle_scroll(
            &event,
            event.window(mtm).as_deref(),
            location,
            run.ignore_system_preference,
        );
        let renderer =
            gesture(|g| g.renderer()).map_or(-1, |renderer| renderer_number(renderer.get()));
        entry["swallowed"] = swallowed.into();
        entry["renderer"] = renderer.into();
        if !swallowed {
            run.window.sendEvent(&event);
        }
    }
    entry["seq"] = gesture(|g| g.seq).into();
    entry["state"] = state_number(gesture(|g| g.state)).into();
    entry["targetWindow"] = window_number(
        gesture(|g| g.target())
            .and_then(|target| target.window())
            .as_deref(),
    )
    .into();
    WRONG_RENDERER_HIT.set(false);
    LEGACY_HIT_ROUTING.set(false);
    log.push(entry);
    log
}

/// Sends the event through `NSApp`, and reads from the trace whether the monitor saw and swallowed it.
fn dispatch_to_app(mtm: MainThreadMarker, event: &NSEvent, entry: &mut Value) {
    let mark = TRACE.with_borrow(|trace| trace.as_ref().map_or(0, Vec::len));
    NSApplication::sharedApplication(mtm).sendEvent(event);
    let (entered, swallowed) = TRACE.with_borrow(|trace| {
        let during = trace
            .as_deref()
            .map_or(&[][..], |trace| &trace[mark.min(trace.len())..]);
        let entered = during.iter().any(|entry| entry["diag"] == "monitorEntry");
        let exit = during
            .iter()
            .rev()
            .find(|entry| entry["diag"] == "monitorExit");
        (
            entered,
            exit.map_or(Value::Null, |exit| exit["swallowed"].clone()),
        )
    });
    entry["dispatch"] = "app".into();
    entry["monitorEntered"] = entered.into();
    entry["monitorSwallowed"] = swallowed;
}

fn same_window(a: Option<&NSWindow>, b: &NSWindow) -> bool {
    a.is_some_and(|a| std::ptr::eq(a, b))
}

fn state_number(state: State) -> i32 {
    match state {
        State::Idle => 0,
        State::Pending => 1,
        State::Tracking => 2,
        State::Ignored => 3,
    }
}
