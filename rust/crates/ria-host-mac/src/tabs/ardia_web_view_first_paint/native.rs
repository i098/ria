// Ported from ardia packages/ardiacore/ios/ArdiaWebView+FirstPaint.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::RefCell;
use std::rc::Rc;

use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyObject, Bool};
use objc2::{class, msg_send};
use objc2_app_kit::NSView;
use objc2_foundation::{NSNumber, NSString};
use ria_engine::call::main_queue;

use super::*;
use crate::core::bindings::Tab;
use crate::tabs::ardia_web_view_painting::{FrameMark, frame_mark, render_widget_in};

thread_local! {
    static HOLDS: RefCell<FirstPaintHolds> = RefCell::default();
}

pub struct PagePaintHold {
    state: RefCell<FirstPaint>,
    page: RefCell<Option<Weak<NSView>>>,
    consent_epoch: Box<dyn Fn() -> String>,
}

impl PagePaintHold {
    pub fn new(consent_epoch: impl Fn() -> String + 'static) -> Rc<Self> {
        Rc::new(Self {
            state: RefCell::default(),
            page: RefCell::default(),
            consent_epoch: Box::new(consent_epoch),
        })
    }

    pub fn reset(&self, created: bool) {
        self.show(PaintEnd::Detached);
        self.state.borrow_mut().reset(created);
    }

    pub fn note_scripted(&self, scripted: bool) {
        self.state.borrow_mut().scripted = scripted;
    }

    pub fn hold_created(self: &Rc<Self>, tab: &Tab, host: &NSView, surface: PaintSurface) {
        if self
            .state
            .borrow_mut()
            .take_created_while_loading(tab.loading())
        {
            self.hold(tab, host, true, surface);
        }
    }

    pub fn hold(
        self: &Rc<Self>,
        tab: &Tab,
        host: &NSView,
        created: bool,
        mut surface: PaintSurface,
    ) {
        let page = tab.view();
        if !crate::tabs::superview(&page).is_some_and(|parent| std::ptr::eq(&*parent, host)) {
            return;
        }
        surface.attached = true;
        if !self.state.borrow().can_hold(created, &surface) {
            return;
        }
        page.setWantsLayer(true);
        let Some(layer) = page.layer() else { return };
        // SAFETY: NSView.layer is a CALayer, whose opacity is a float.
        surface.opacity = unsafe { msg_send![&*layer, opacity] };
        let Some(generation) =
            self.state
                .borrow_mut()
                .hold(created, surface, &(self.consent_epoch)(), Instant::now())
        else {
            return;
        };
        *self.page.borrow_mut() = Some(Weak::new(&*page));
        set_opacity(&layer, 0.0, false);
        tab.call_page(&NSString::from_str("firstPaint"), &NSString::from_str("{}"));
        self.end_after(HOLD_TIMEOUT, generation, PaintEnd::TimedOut);
    }

    pub fn show(&self, end: PaintEnd) {
        let generation = self.state.borrow().generation;
        self.show_generation(generation, end);
    }

    pub fn show_after_load(self: &Rc<Self>) {
        let Some((generation, scripted)) = self.state.borrow().load_wait() else {
            return;
        };
        if scripted {
            self.end_after(SCRIPTED_LOAD_TIMEOUT, generation, PaintEnd::Loaded);
        } else if let Some(widget) = self.page().as_deref().and_then(render_widget_in) {
            self.wait_for_frame(
                generation,
                frame_mark(&widget),
                Instant::now() + LOADED_FRAME_TIMEOUT,
            );
        } else {
            self.show_generation(generation, PaintEnd::Loaded);
        }
    }

    pub fn take_reports() -> FirstPaintHolds {
        HOLDS.with(|holds| std::mem::take(&mut *holds.borrow_mut()))
    }

    fn page(&self) -> Option<Retained<NSView>> {
        self.page.borrow().as_ref().and_then(Weak::load)
    }

    fn show_generation(&self, generation: u64, end: PaintEnd) {
        let epoch = (self.consent_epoch)();
        let opacity = HOLDS.with(|holds| {
            self.state.borrow_mut().settle(
                generation,
                end,
                &epoch,
                Instant::now(),
                &mut holds.borrow_mut(),
            )
        });
        let Some(opacity) = opacity else { return };
        let page = self.page.borrow_mut().take().and_then(|page| page.load());
        if let Some(layer) = page.and_then(|page| page.layer()) {
            set_opacity(&layer, opacity, end != PaintEnd::Detached);
        }
    }

    fn end_after(self: &Rc<Self>, wait: Duration, generation: u64, end: PaintEnd) {
        let hold = Rc::downgrade(self);
        main_queue::after(
            wait,
            Box::new(move || {
                if let Some(hold) = hold.upgrade() {
                    hold.show_generation(generation, end);
                }
            }),
        );
    }

    fn wait_for_frame(self: &Rc<Self>, generation: u64, mark: FrameMark, deadline: Instant) {
        let hold = Rc::downgrade(self);
        main_queue::after(
            FRAME_INTERVAL,
            Box::new(move || {
                let Some(hold) = hold
                    .upgrade()
                    .filter(|hold| hold.state.borrow().is_holding(generation))
                else {
                    return;
                };
                let widget = hold.page().as_deref().and_then(render_widget_in);
                if Instant::now() >= deadline
                    || widget.is_none_or(|widget| frame_mark(&widget) != mark)
                {
                    hold.show_generation(generation, PaintEnd::Loaded);
                } else {
                    hold.wait_for_frame(generation, mark, deadline);
                }
            }),
        );
    }
}

fn set_opacity(layer: &AnyObject, opacity: f32, fade: bool) {
    // SAFETY: The layer is a CALayer. These selectors use the documented QuartzCore argument types.
    unsafe {
        let transaction = class!(CATransaction);
        let (): () = msg_send![transaction, begin];
        let (): () = msg_send![transaction, setDisableActions: Bool::YES];
        let (): () = msg_send![layer, setOpacity: opacity];
        if fade {
            let key_path = NSString::from_str("opacity");
            let animation: Retained<AnyObject> =
                msg_send![class!(CABasicAnimation), animationWithKeyPath: &*key_path];
            let timing: *mut AnyObject = objc2::runtime::MessageReceiver::send_message(
                class!(CAMediaTimingFunction),
                objc2::sel!(functionWithControlPoints::::),
                (0.23f32, 1.0f32, 0.32f32, 1.0f32),
            );
            let from = NSNumber::new_f32(0.0);
            let to = NSNumber::new_f32(opacity);
            let (): () = msg_send![&*animation, setFromValue: &*from];
            let (): () = msg_send![&*animation, setToValue: &*to];
            let (): () = msg_send![&*animation, setDuration: 0.16f64];
            let (): () = msg_send![&*animation, setTimingFunction: timing];
            let key = NSString::from_str("ardia.held.page");
            let (): () = msg_send![layer, addAnimation: &*animation, forKey: &*key];
        }
        let (): () = msg_send![transaction, commit];
    }
}
