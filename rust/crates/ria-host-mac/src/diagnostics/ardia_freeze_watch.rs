// Ported from ardia packages/ardiacore/ios/ArdiaFreezeWatch.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Also ports ArdiaFreezeWatch.h. Changes from Ardia: Ria has no React Native JavaScript thread and no
//! Hermes, so only the main run loop is watched. The `ACFreezeWatch` module methods are plain functions,
//! the `js` tally stays empty, and no stall has a GC reading (-1). Without a JavaScript thread, the input ring
//! that only the JavaScript stall used is not kept.

use std::cell::Cell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use objc2::MainThreadMarker;
use serde_json::{Value, json};

use super::ardia_freeze_capture::{self as capture, Ended, Stretch, now_ticks, ticks_to_ms};

const STALL_MS: f64 = 250.0;
const BUCKET_COUNT: usize = 6;
const BUCKET_FLOORS_MS: [f64; BUCKET_COUNT] = [250.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0];
const INPUT_WAIT_FLOOR_MS: f64 = 50.0;
const MAIN: usize = 0;

const ENTRY: usize = 1 << 0;
const BEFORE_WAITING: usize = 1 << 5;
const AFTER_WAITING: usize = 1 << 6;
const EXIT: usize = 1 << 7;

#[repr(C)]
struct ObserverContext {
    version: isize,
    info: *mut c_void,
    retain: *const c_void,
    release: *const c_void,
    copy_description: *const c_void,
}

type Callout = extern "C" fn(observer: *mut c_void, activity: usize, info: *mut c_void);

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFRunLoopCommonModes: *const c_void;
    fn CFRunLoopGetMain() -> *mut c_void;
    fn CFRunLoopObserverCreate(
        allocator: *const c_void,
        activities: usize,
        repeats: u8,
        order: isize,
        callout: Callout,
        context: *mut ObserverContext,
    ) -> *mut c_void;
    fn CFRunLoopAddObserver(rl: *mut c_void, observer: *mut c_void, mode: *const c_void);
}

#[derive(Clone, Copy, Default)]
struct StallTally {
    buckets: [u32; BUCKET_COUNT],
    longest_ms: f64,
    input_waits: u32,
    input_wait_max_ms: f64,
    gc_measured: u32,
    gc_ms: f64,
    gc_longest_ms: f64,
    gc_bound: u32,
}

/// The main-thread half of Ardia's `LoopWatch`; the stretch is shared with the capture.
#[derive(Default)]
struct LoopWatch {
    epoch_at_start: Cell<u64>,
    input_wait_ms: Cell<f64>,
    depth: Cell<i32>,
    observed: Cell<bool>,
}

thread_local! {
    static MAIN_WATCH: LoopWatch = LoopWatch::default();
}

static MAIN_STRETCH: LazyLock<Arc<Stretch>> = LazyLock::new(Arc::default);
static EPOCH: AtomicU64 = AtomicU64::new(0);
static TALLY: Mutex<[StallTally; 2]> = Mutex::new(
    [StallTally {
        buckets: [0; BUCKET_COUNT],
        longest_ms: 0.0,
        input_waits: 0,
        input_wait_max_ms: 0.0,
        gc_measured: 0,
        gc_ms: 0.0,
        gc_longest_ms: 0.0,
        gc_bound: 0,
    }; 2],
);

fn tally() -> std::sync::MutexGuard<'static, [StallTally; 2]> {
    TALLY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn bucket_of(ms: f64) -> usize {
    let mut bucket = 0;
    while bucket + 1 < BUCKET_COUNT && ms >= BUCKET_FLOORS_MS[bucket + 1] {
        bucket += 1;
    }
    bucket
}

fn record_stall(thread: usize, ms: f64, epoch: u64, input_wait_ms: f64, gc_ms: f64) {
    let mut tallies = tally();
    if epoch == 0 || epoch != EPOCH.load(Ordering::SeqCst) {
        return;
    }
    let tally = &mut tallies[thread];
    tally.buckets[bucket_of(ms)] += 1;
    tally.longest_ms = tally.longest_ms.max(ms);
    if input_wait_ms >= INPUT_WAIT_FLOOR_MS {
        tally.input_waits += 1;
        tally.input_wait_max_ms = tally.input_wait_max_ms.max(input_wait_ms);
    }
    if gc_ms >= 0.0 {
        tally.gc_measured += 1;
        tally.gc_ms += gc_ms;
        tally.gc_longest_ms = tally.gc_longest_ms.max(gc_ms);
        if gc_ms * 2.0 >= ms {
            tally.gc_bound += 1;
        }
    }
}

extern "C" fn loop_activity(_observer: *mut c_void, activity: usize, _info: *mut c_void) {
    MAIN_WATCH.with(|watch| match activity {
        ENTRY => watch.depth.set(watch.depth.get() + 1),
        EXIT => watch.depth.set(watch.depth.get() - 1),
        AFTER_WAITING => woke(watch),
        _ => slept(watch),
    });
}

fn woke(watch: &LoopWatch) {
    let now = now_ticks();
    let stretch = &**MAIN_STRETCH;
    stretch.id.fetch_add(1, Ordering::Release);
    stretch.start.store(now, Ordering::Release);
    watch.epoch_at_start.set(EPOCH.load(Ordering::Relaxed));
    watch.input_wait_ms.set(0.0);
    capture::arm();
}

fn slept(watch: &LoopWatch) {
    let now = now_ticks();
    let stretch = &*MAIN_STRETCH;
    let start = stretch.start.swap(0, Ordering::AcqRel);
    if start == 0 {
        return;
    }
    capture::idle();
    let ms = ticks_to_ms(now - start);
    if ms < STALL_MS {
        return;
    }
    let input_wait_ms = watch.input_wait_ms.get();
    record_stall(MAIN, ms, watch.epoch_at_start.get(), input_wait_ms, -1.0);
    capture::stretch_ended(Ended {
        thread: MAIN,
        source: Arc::clone(stretch),
        id: stretch.id.load(Ordering::Acquire),
        ms,
        input_wait_ms,
        gc_ms: -1.0,
        gc_count: -1.0,
        epoch: watch.epoch_at_start.get(),
    });
}

fn observe_main(watch: &LoopWatch) {
    if watch.observed.replace(true) {
        return;
    }
    let mut context = ObserverContext {
        version: 0,
        info: std::ptr::null_mut(),
        retain: std::ptr::null(),
        release: std::ptr::null(),
        copy_description: std::ptr::null(),
    };
    // SAFETY: The observers live for the process on the main run loop, which also lives for the process.
    unsafe {
        let wake = CFRunLoopObserverCreate(
            std::ptr::null(),
            ENTRY | AFTER_WAITING,
            1,
            isize::MIN,
            loop_activity,
            &mut context,
        );
        let sleep = CFRunLoopObserverCreate(
            std::ptr::null(),
            BEFORE_WAITING | EXIT,
            1,
            isize::MAX,
            loop_activity,
            &mut context,
        );
        let main = CFRunLoopGetMain();
        watch.depth.set(1);
        CFRunLoopAddObserver(main, wake, kCFRunLoopCommonModes);
        CFRunLoopAddObserver(main, sleep, kCFRunLoopCommonModes);
        capture::watch(MAIN, libc::pthread_self(), Some(Arc::clone(&MAIN_STRETCH)));
    }
}

fn describe(tally: &StallTally) -> Value {
    json!({
        "buckets": tally.buckets,
        "longestMs": tally.longest_ms.round(),
        "inputWaits": tally.input_waits,
        "inputWaitMaxMs": tally.input_wait_max_ms.round(),
        "gcMeasured": tally.gc_measured,
        "gcMs": tally.gc_ms.round(),
        "gcLongestMs": tally.gc_longest_ms.round(),
        "gcBound": tally.gc_bound,
    })
}

/// `ArdiaFreezeNoteInput`: the uptime of an input event, in seconds.
pub fn note_input(event_uptime: f64) {
    let event_ms = event_uptime * 1000.0;
    if MainThreadMarker::new().is_none() || MAIN_STRETCH.start.load(Ordering::Relaxed) == 0 {
        return;
    }
    MAIN_WATCH.with(|watch| {
        let wait = ticks_to_ms(now_ticks()) - event_ms;
        watch.input_wait_ms.set(watch.input_wait_ms.get().max(wait));
    });
}

/// `start`: watches the main run loop, once, and sends each freeze record to `emit`.
pub fn start(_mtm: MainThreadMarker, emit: impl Fn(Value) + Send + Sync + 'static) -> bool {
    if MAIN_WATCH.with(|watch| watch.observed.get()) {
        return false;
    }
    capture::set_emitter(emit);
    MAIN_WATCH.with(observe_main);
    true
}

/// `setConsentEpoch:`.
pub fn set_consent_epoch(epoch: f64) {
    let mut tallies = tally();
    EPOCH.store(if epoch > 0.0 { epoch as u64 } else { 0 }, Ordering::SeqCst);
    *tallies = Default::default();
}

/// `take:`: the stall tallies since the last take, when `epoch` is the current consent epoch.
pub fn take(epoch: f64) -> Option<Value> {
    let mut tallies = tally();
    if epoch <= 0.0 || epoch as u64 != EPOCH.load(Ordering::SeqCst) {
        return None;
    }
    let [main, js] = std::mem::take(&mut *tallies);
    Some(json!({ "main": describe(&main), "js": describe(&js) }))
}

/// `annotate:activity:`.
pub fn annotate(freeze_id: f64, activity: &str) {
    if freeze_id > 0.0 {
        capture::annotate(freeze_id as u64, activity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stalls_fall_in_the_bucket_of_their_floor() {
        assert_eq!(bucket_of(250.0), 0);
        assert_eq!(bucket_of(499.9), 0);
        assert_eq!(bucket_of(500.0), 1);
        assert_eq!(bucket_of(60_000.0), 5);
    }

    #[test]
    fn stalls_count_only_in_the_current_epoch() {
        set_consent_epoch(7.0);
        record_stall(MAIN, 600.0, 7, 80.0, -1.0);
        record_stall(MAIN, 600.0, 6, 0.0, -1.0);
        let taken = take(7.0).expect("the current epoch takes");
        assert_eq!(taken["main"]["buckets"], json!([0, 1, 0, 0, 0, 0]));
        assert_eq!(taken["main"]["inputWaits"], 1);
        assert_eq!(taken["js"]["longestMs"], 0.0);
        assert!(take(6.0).is_none());
    }
}
