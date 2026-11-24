// Ported from ardia packages/ardiacore/ios/SwipeArea.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The view that takes swipes for the page layer. Ardia's Expo `EventDispatcher`s `onSwipe`, `onPagerPosition` and
//! `onPagerState` are one `events` sink here; the events reach it from the main queue, as Expo's reach JavaScript
//! later, so a handler may call back into the pager.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::NSView;
use objc2_foundation::{NSPoint, NSRect};
use ria_engine::call::main_queue;
use serde_json::Value;

use super::ardia_swipe::{add_target, remove_target};
use super::pager::Pager;
use super::pager_config::PagerConfig;

pub type Events = Rc<dyn Fn(&'static str, Value)>;

#[derive(Default)]
pub struct Ivars {
    pub can_swipe_back: Cell<bool>,
    pub can_swipe_forward: Cell<bool>,
    pub tracks_unavailable_directions: Cell<bool>,
    pub allows_vertical_motion: Cell<bool>,
    pub is_pager: Cell<bool>,
    pager: RefCell<Option<Rc<RefCell<Pager>>>>,
    pager_width: Cell<f64>,
    events: RefCell<Option<Events>>,
}

impl Drop for Ivars {
    fn drop(&mut self) {
        if let Some(pager) = self.pager.take() {
            main_queue::post(Box::new(move || pager.borrow_mut().prune()));
        }
    }
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "RiaSwipeArea"]
    #[ivars = Ivars]
    pub struct SwipeArea;

    impl SwipeArea {
        #[unsafe(method_id(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> Option<Retained<NSView>> { None }

        #[unsafe(method(viewDidMoveToWindow))]
        fn view_did_move_to_window(&self) {
            // SAFETY: NSView implements viewDidMoveToWindow with no arguments.
            let () = unsafe { msg_send![super(self), viewDidMoveToWindow] };
            if self.window().is_some() { add_target(self) } else { remove_target(self) }
        }
    }
);

impl SwipeArea {
    pub fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars::default());
        // SAFETY: The allocated class has NSView as its superclass.
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }

    pub fn set_events(&self, events: Option<Events>) {
        self.ivars().events.replace(events);
    }

    fn dispatch(&self, name: &'static str, event: Value) {
        if let Some(events) = self.ivars().events.borrow().clone() {
            main_queue::post(Box::new(move || events(name, event)));
        }
    }

    pub fn on_pager_position(&self, event: Value) {
        self.dispatch("onPagerPosition", event);
    }

    pub fn on_pager_state(&self, event: Value) {
        self.dispatch("onPagerState", event);
    }

    pub fn is_pager(&self) -> bool {
        self.ivars().is_pager.get()
    }

    pub fn pager(&self) -> Option<Rc<RefCell<Pager>>> {
        self.ivars().pager.borrow().clone()
    }

    pub fn has_native_pager(&self) -> bool {
        self.ivars().pager.borrow().is_some()
    }

    pub fn page_width(&self) -> f64 {
        let width = self.ivars().pager_width.get();
        if width > 0.0 {
            width
        } else {
            self.bounds().size.width
        }
    }

    pub fn swipe_event(&self, event: Value) {
        self.dispatch("onSwipe", event);
    }

    pub fn pager_input(&self, phase: &str, distance: f64, direction: i32, timestamp: f64) {
        if let Some(pager) = self.pager() {
            pager
                .borrow_mut()
                .input(self, phase, distance, direction, timestamp);
        }
    }

    pub fn configure_pager(&self, config: Option<&PagerConfig>) {
        let Some(config) = config else {
            let old = self.ivars().pager.take();
            if let Some(old) = old {
                old.borrow_mut().detach(self);
            }
            return;
        };
        if self
            .pager()
            .is_none_or(|pager| pager.borrow().key != config.key)
        {
            if let Some(old) = self.pager() {
                old.borrow_mut().detach(self);
            }
            let pager = Pager::named(&config.key);
            self.ivars().pager.replace(Some(pager.clone()));
            pager.borrow_mut().attach(self);
        }
        self.ivars().pager_width.set(config.width);
        if let Some(pager) = self.pager() {
            pager.borrow_mut().configure(
                config.selected,
                config.count,
                config.ack,
                config.order.clone(),
            );
        }
    }
}
