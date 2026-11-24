// Ported from ardia packages/ardiacore/ios/ArdiaSwipeInternal.h at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The state that `ardia_swipe`, its simulation and its hit testing share. Ardia's `ArdiaRendererScroll` is
//! `RendererScroll` of `tabs::ardia_renderer_scroll_observer`; `renderer_scroll_for` installs that observer once.

use std::cell::{Cell, OnceCell, RefCell};
use std::rc::{self, Rc};

use objc2::MainThreadOnly;
use objc2::rc::{Retained, Weak};
use objc2_app_kit::{NSView, NSWindow};
use serde_json::{Value, json};

use super::swipe_area::SwipeArea;
use crate::core::bindings::PageScrollAck;
pub use crate::tabs::ardia_renderer_scroll_observer::page_view_of;
use crate::tabs::ardia_renderer_scroll_observer::{
    RendererScroll, RendererScrollObserver, ScrollEvent, ScrollPhase,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Idle,
    Pending,
    Tracking,
    Ignored,
}

#[derive(Clone, Default)]
pub struct Gesture {
    pub state: State,
    pub target: Option<Weak<SwipeArea>>,
    pub renderer: Option<rc::Weak<Cell<RendererScroll>>>,
    pub hit: Option<Weak<NSView>>,
    pub renderer_hit: bool,
    pub dx: f64,
    pub dy: f64,
    pub direction: i32,
    pub available: bool,
    pub times: [f64; 6],
    pub deltas: [f64; 6],
    pub samples: usize,
    pub ignore_system_preference: bool,
    pub seq: i32,
    pub emitted: i32,
    pub time: f64,
}

impl Gesture {
    pub fn target(&self) -> Option<Retained<SwipeArea>> {
        self.target.as_ref()?.load()
    }

    pub fn hit(&self) -> Option<Retained<NSView>> {
        self.hit.as_ref()?.load()
    }

    pub fn renderer(&self) -> Option<Rc<Cell<RendererScroll>>> {
        self.renderer.as_ref()?.upgrade()
    }
}

thread_local! {
    pub static TRACE: RefCell<Option<Vec<Value>>> = const { RefCell::new(None) };
    pub static WRONG_RENDERER_HIT: Cell<bool> = const { Cell::new(false) };
    pub static LEGACY_HIT_ROUTING: Cell<bool> = const { Cell::new(false) };
    pub static GESTURE: RefCell<Gesture> = RefCell::default();
    static OBSERVER: OnceCell<Retained<RendererScrollObserver>> = const { OnceCell::new() };
}

pub fn tracing() -> bool {
    TRACE.with_borrow(Option::is_some)
}

/// Ardia's `Diag`: the entry is built only while a trace runs.
pub fn diag(entry: impl FnOnce() -> Value) {
    if tracing() {
        let entry = entry();
        TRACE.with_borrow_mut(|trace| trace.as_mut().map(|trace| trace.push(entry)));
    }
}

pub fn gesture<R>(f: impl FnOnce(&mut Gesture) -> R) -> R {
    GESTURE.with_borrow_mut(f)
}

pub fn superview(view: &NSView) -> Option<Retained<NSView>> {
    // SAFETY: The live AppKit hierarchy is confined to the main thread.
    unsafe { view.superview() }
}

pub fn same<T>(a: Option<&T>, b: Option<&T>) -> bool {
    a.map(std::ptr::from_ref) == b.map(std::ptr::from_ref)
}

pub fn window_number(window: Option<&NSWindow>) -> isize {
    window.map_or(0, NSWindow::windowNumber)
}

pub fn pointer<T>(object: &T) -> String {
    format!("{:p}", std::ptr::from_ref(object))
}

/// Ardia's `RendererScrollFor`: the scroll state of the page that holds `hit`, when that page takes swipes.
pub fn renderer_scroll_for(hit: &NSView) -> Option<Rc<Cell<RendererScroll>>> {
    OBSERVER.with(|observer| {
        observer
            .get_or_init(|| RendererScrollObserver::install(hit.mtm(), trace_ack))
            .renderer_scroll_for(hit)
    })
}

fn trace_ack(event: ScrollEvent) {
    diag(|| match event {
        ScrollEvent::Ack(ack, consumed) => json!({
            "ack": "gesture",
            "kind": if matches!(ack, PageScrollAck::Begin) { "begin" } else { "update" },
            "consumed": consumed,
        }),
        ScrollEvent::Overscroll(allowed) => json!({"ack": "overscroll", "allowsX": allowed}),
    });
}

/// The number of Ardia's `RendererScroll` value, for the trace.
pub fn renderer_number(scroll: RendererScroll) -> i32 {
    match scroll.phase {
        ScrollPhase::AwaitingBegin => 1,
        ScrollPhase::AwaitingFirstUpdate => 2,
        ScrollPhase::FirstUpdateUnconsumed => 3,
        ScrollPhase::FirstUpdateConsumed => 4,
    }
}
