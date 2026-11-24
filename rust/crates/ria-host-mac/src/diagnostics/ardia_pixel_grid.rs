// Ported from ardia packages/ardiacore/ios/ArdiaPixelGridModule.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Changes from Ardia: the `ACPixelGrid` React Native module is a `PixelGrid` value. Ardia sets the Yoga point
//! scale factor; here `apply` gets the scale and answers whether the interface must lay out again, and the
//! interface layer calls `will_lay_out` and `did_mount` where Ardia observes the React Native UI manager.

use std::cell::{Cell, RefCell};
use std::ptr::NonNull;
use std::rc::{Rc, Weak};

use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSApplication, NSApplicationDidChangeScreenParametersNotification,
    NSWindowDidChangeBackingPropertiesNotification, NSWindowDidChangeOcclusionStateNotification,
    NSWindowDidChangeScreenNotification, NSWindowWillCloseNotification,
};
use objc2_foundation::{
    NSNotification, NSNotificationCenter, NSNotificationName, NSOperationQueue, ns_string,
};
use ria_engine::call::main_queue;

use crate::windows::ardia_interface_scale;

#[derive(Clone, Copy, PartialEq)]
enum Relayout {
    None,
    Asked,
    Running,
}

fn coarsest_root_window_scale(mtm: MainThreadMarker, visible_only: bool) -> f64 {
    let mut coarsest: f64 = 0.;
    for window in NSApplication::sharedApplication(mtm).windows().iter() {
        let scale = window.backingScaleFactor();
        if (visible_only && !window.isVisible())
            || scale <= 0.
            || (coarsest > 0. && scale >= coarsest)
            || ardia_interface_scale::root_of_window(&window).is_none()
        {
            continue;
        }
        coarsest = scale;
    }
    coarsest
}

fn coarsest_interface_window_scale(mtm: MainThreadMarker) -> f64 {
    let shown = coarsest_root_window_scale(mtm, true);
    if shown > 0. {
        shown
    } else {
        coarsest_root_window_scale(mtm, false)
    }
}

fn grid_changing_notifications() -> [&'static NSNotificationName; 7] {
    // SAFETY: AppKit's notification names are immutable static strings.
    unsafe {
        [
            ns_string!("ArdiaInterfaceScaleDidChange"),
            ns_string!("ArdiaInterfaceScaleRootDidAttach"),
            NSWindowDidChangeBackingPropertiesNotification,
            NSWindowDidChangeScreenNotification,
            NSWindowDidChangeOcclusionStateNotification,
            NSWindowWillCloseNotification,
            NSApplicationDidChangeScreenParametersNotification,
        ]
    }
}

pub struct PixelGrid {
    mtm: MainThreadMarker,
    apply: Box<dyn Fn(f64) -> bool>,
    observers: RefCell<Vec<Retained<AnyObject>>>,
    applied: Cell<f64>,
    pending: Cell<bool>,
    relayout: Cell<Relayout>,
}

impl PixelGrid {
    /// `apply` sets the interface's point scale factor and answers whether it changed.
    pub fn new(mtm: MainThreadMarker, apply: impl Fn(f64) -> bool + 'static) -> Rc<Self> {
        let grid = Rc::new(Self {
            mtm,
            apply: Box::new(apply),
            observers: RefCell::default(),
            applied: Cell::new(0.),
            pending: Cell::new(false),
            relayout: Cell::new(Relayout::None),
        });
        let observers =
            grid_changing_notifications().map(|name| observe(name, Rc::downgrade(&grid)));
        grid.observers.borrow_mut().extend(observers);
        grid.follow_windows_soon();
        grid
    }

    fn follow_windows_soon(self: &Rc<Self>) {
        if self.pending.get() {
            return;
        }
        self.pending.set(true);
        let weak = Rc::downgrade(self);
        main_queue::post(Box::new(move || {
            if let Some(grid) = weak.upgrade() {
                grid.pending.set(false);
                grid.follow_windows();
            }
        }));
    }

    fn follow_windows(&self) {
        let scale = coarsest_interface_window_scale(self.mtm) * ardia_interface_scale::scale();
        if scale <= 0. || scale == self.applied.get() {
            return;
        }
        self.applied.set(scale);
        if (self.apply)(scale) {
            self.relayout.set(Relayout::Asked);
        }
    }

    /// `uiManagerWillPerformLayout:`.
    pub fn will_lay_out(&self) {
        if self.relayout.get() == Relayout::Asked {
            self.relayout.set(Relayout::Running);
        }
    }

    /// `uiManagerDidPerformMounting:`.
    pub fn did_mount(&self) {
        if self.relayout.get() != Relayout::Running {
            return;
        }
        self.relayout.set(Relayout::None);
        main_queue::post(Box::new(ardia_interface_scale::layout_settled));
    }
}

impl Drop for PixelGrid {
    fn drop(&mut self) {
        let center = NSNotificationCenter::defaultCenter();
        for observer in self.observers.get_mut() {
            // SAFETY: These observer tokens came from this notification center.
            unsafe { center.removeObserver(observer) };
        }
    }
}

fn observe(name: &NSNotificationName, grid: Weak<PixelGrid>) -> Retained<AnyObject> {
    let block = RcBlock::new(move |_: NonNull<NSNotification>| {
        if let Some(grid) = grid.upgrade() {
            grid.follow_windows_soon();
        }
    });
    // SAFETY: The block runs on the main operation queue.
    let token = unsafe {
        NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
            Some(name),
            None,
            Some(&NSOperationQueue::mainQueue()),
            &block,
        )
    };
    token.into()
}
