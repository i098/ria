// Ported from ardia packages/ardiacore/ios/ArdiaFieldTiming.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Also ports ArdiaFieldTiming.h. Changes from Ardia: the `ACFieldTiming` React Native module is plain
//! functions. `set_enabled` takes the web-view switch that Ardia calls as `ArdiaWebViewsSetFieldTiming`, and
//! `mark_frame` marks at the next Core Animation commit where Ardia waits for the React Native UI block. No
//! module owner is kept, because no bridge invalidates it.
// Part 4 of slice 3.8 is not merged: `ArdiaWebView` has no Ria type, so no caller of the marks or of the
// `set_enabled` switch exists yet.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Bool};
use objc2::{msg_send, sel};
use objc2_app_kit::{NSEvent, NSEventMask, NSEventType};
use objc2_foundation::{NSDate, NSProcessInfo};
use ria_engine::call::main_queue;
use serde_json::{Value, json};

use super::ardia_freeze_watch;

const MAX_MARKS: usize = 512;
const POST_COMMIT: u32 = 2;

static ENABLED: AtomicBool = AtomicBool::new(false);
static LAST_INPUT_AT: AtomicU64 = AtomicU64::new(0);
static LAST_INPUT_KIND: AtomicI32 = AtomicI32::new(0);
static MARKS: Mutex<VecDeque<(String, String, f64)>> = Mutex::new(VecDeque::new());

thread_local! {
    static INPUT_MONITOR: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
}

fn marks() -> MutexGuard<'static, VecDeque<(String, String, f64)>> {
    MARKS.lock().unwrap_or_else(PoisonError::into_inner)
}

fn input_monitor() -> Option<Retained<AnyObject>> {
    let mask = NSEventMask::KeyDown | NSEventMask::LeftMouseDown | NSEventMask::LeftMouseUp;
    let block = RcBlock::new(|event: NonNull<NSEvent>| {
        // SAFETY: AppKit lends a live event for the call.
        let shown = unsafe { event.as_ref() };
        LAST_INPUT_AT.store(event_time(shown).to_bits(), Ordering::SeqCst);
        let kind = if shown.r#type() == NSEventType::KeyDown {
            1
        } else {
            2
        };
        LAST_INPUT_KIND.store(kind, Ordering::SeqCst);
        ardia_freeze_watch::note_input(shown.timestamp());
        event.as_ptr()
    });
    // SAFETY: The handler returns the event AppKit gave it.
    unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &block) }
}

/// `SetEnabled`. `web_views` is `ArdiaWebViewsSetFieldTiming`; it runs on the main queue.
pub fn set_enabled(enabled: bool, web_views: impl FnOnce(bool) + Send + 'static) {
    ENABLED.store(enabled, Ordering::SeqCst);
    main_queue::post(Box::new(move || {
        INPUT_MONITOR.with_borrow_mut(|monitor| {
            if enabled && monitor.is_none() {
                *monitor = input_monitor();
            } else if !enabled && let Some(token) = monitor.take() {
                // SAFETY: AppKit issued this monitor token.
                unsafe { NSEvent::removeMonitor(&token) };
            }
        });
        web_views(ENABLED.load(Ordering::SeqCst));
    }));
    if !enabled {
        marks().clear();
    }
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::SeqCst)
}

pub fn now() -> f64 {
    NSDate::date().timeIntervalSince1970() * 1000.0
}

pub fn event_time(event: &NSEvent) -> f64 {
    now() - (NSProcessInfo::processInfo().systemUptime() - event.timestamp()) * 1000.0
}

pub fn mark(key: &str, kind: &str, at: f64) {
    if !enabled() || key.is_empty() || !at.is_finite() {
        return;
    }
    let mut marks = marks();
    if marks.len() >= MAX_MARKS {
        marks.pop_front();
    }
    marks.push_back((key.to_owned(), kind.to_owned(), (at * 10.0).round() / 10.0));
}

pub fn mark_now(key: &str, kind: &str) {
    if enabled() {
        mark(key, kind, now());
    }
}

pub fn mark_at_commit(key: &str, kind: &str) {
    if !enabled() || key.is_empty() {
        return;
    }
    let (key, kind) = (key.to_owned(), kind.to_owned());
    let add = sel!(addCommitHandler:forPhase:);
    let transaction = AnyClass::get(c"CATransaction");
    // SAFETY: The selector check comes first; a class answers `respondsToSelector:`.
    let answers = transaction.is_some_and(|class| unsafe {
        let responds: Bool = msg_send![class, respondsToSelector: add];
        responds.as_bool()
    });
    let Some(transaction) = transaction.filter(|_| answers) else {
        mark(&key, &kind, now());
        return;
    };
    let handler = RcBlock::new(move || mark(&key, &kind, now()));
    // SAFETY: The private method takes a block, which it copies, and a phase number.
    let () = unsafe { msg_send![transaction, addCommitHandler: &*handler, forPhase: POST_COMMIT] };
}

/// `markFrame:kind:`.
pub fn mark_frame(key: &str, kind: &str) {
    if enabled() {
        mark_at_commit(key, kind);
    }
}

/// `lastInput`: the time and kind (1 key, 2 click) of the last input.
pub fn last_input() -> Value {
    let at = f64::from_bits(LAST_INPUT_AT.load(Ordering::SeqCst));
    json!([
        (at * 10.0).round() / 10.0,
        LAST_INPUT_KIND.load(Ordering::SeqCst)
    ])
}

/// `drain`: the marks since the last drain, each `[key, kind, at]`.
pub fn drain() -> Value {
    let drained: Vec<Value> = marks()
        .drain(..)
        .map(|(key, kind, at)| json!([key, kind, at]))
        .collect();
    Value::Array(drained)
}
