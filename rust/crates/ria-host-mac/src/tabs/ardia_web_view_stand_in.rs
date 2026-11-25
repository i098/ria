// Ported from ardia packages/ardiacore/ios/ArdiaWebView+StandIn.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::RefCell;
use std::ptr;
use std::rc::Rc;
use std::time::{Duration, Instant};

use objc2::rc::{Retained, Weak};
use objc2::{Message, class, msg_send};
use objc2_app_kit::{NSView, NSWindowOrderingMode};
use objc2_foundation::NSPoint;
use ria_engine::call::main_queue;

use crate::core::bindings::Tab;

use super::superview;

const TEARDOWN_DELAY: Duration = Duration::from_millis(100);
const TAKEOVER_LIFETIME: Duration = Duration::from_secs(1);

struct Loan {
    page: Retained<NSView>,
    tab: Retained<Tab>,
    lender: Weak<NSView>,
    close: bool,
    leaving_opacity: Option<f32>,
}

#[derive(Default)]
pub struct StandIn {
    generation: u64,
    loan: Option<Loan>,
}

pub struct Successor<'a> {
    pub host: &'a NSView,
    pub page: &'a NSView,
    pub visible_or_revealed: bool,
    pub awaiting_paint: bool,
    pub took_over_at: Instant,
}

fn mounted(page: &NSView, host: &NSView) -> bool {
    superview(page).is_some_and(|parent| ptr::eq(&*parent, host))
}

pub(super) fn without_actions(action: impl FnOnce()) {
    struct Transaction;
    impl Drop for Transaction {
        fn drop(&mut self) {
            // SAFETY: Paired with begin on the AppKit thread.
            unsafe {
                let _: () = msg_send![class!(CATransaction), commit];
            }
        }
    }
    // SAFETY: All callers mutate main-thread AppKit views.
    unsafe {
        let _: () = msg_send![class!(CATransaction), begin];
        let _: () = msg_send![class!(CATransaction), setDisableActions: true];
    }
    let _transaction = Transaction;
    action();
}

impl StandIn {
    pub fn lend(
        &mut self,
        lender: &NSView,
        tab: &Tab,
        standalone_or_devtools: bool,
        successor: Successor<'_>,
        now: Instant,
    ) -> Option<bool> {
        let page = tab.view()?;
        if self.loan.is_some()
            || standalone_or_devtools
            || !mounted(&page, lender)
            || page.isHidden()
            || lender.alphaValue() <= 0.0
            || lender.isHiddenOrHasHiddenAncestor()
            || !successor.visible_or_revealed
            || now.saturating_duration_since(successor.took_over_at) >= TAKEOVER_LIFETIME
            || !mounted(successor.page, successor.host)
        {
            return None;
        }
        let window = successor.host.window()?;
        if lender
            .window()
            .is_none_or(|source| !ptr::eq(&*source, &*window))
        {
            return None;
        }
        let had_focus = window
            .firstResponder()
            .and_then(|responder| responder.downcast::<NSView>().ok())
            .is_some_and(|view| view.isDescendantOf(&page));
        if had_focus {
            window.makeFirstResponder(None);
        }
        without_actions(|| {
            page.setFrame(successor.host.bounds());
            let (order, relative) = if successor.awaiting_paint {
                (NSWindowOrderingMode::Above, None)
            } else {
                (NSWindowOrderingMode::Below, Some(successor.page))
            };
            successor
                .host
                .addSubview_positioned_relativeTo(&page, order, relative);
            if successor.awaiting_paint {
                successor.host.setAlphaValue(1.0);
            }
        });
        self.loan = Some(Loan {
            page,
            tab: tab.retain(),
            lender: Weak::from(lender),
            close: false,
            leaving_opacity: None,
        });
        Some(had_focus)
    }

    pub fn drop_if_painted(&self, callback: impl FnOnce(u64) + 'static) {
        if self.loan.is_some() {
            let generation = self.generation;
            main_queue::after(TEARDOWN_DELAY, Box::new(move || callback(generation)));
        }
    }

    /// `_standIn.tab || _standIn.page`: a page is on loan to this view.
    pub fn lent(&self) -> bool {
        self.loan.is_some()
    }

    /// `_standIn.leaving`.
    pub fn leaving(&self) -> bool {
        self.loan
            .as_ref()
            .is_some_and(|loan| loan.leaving_opacity.is_some())
    }

    pub fn still_holds(&self, generation: u64) -> bool {
        self.generation == generation && self.loan.is_some()
    }

    pub fn closes_lent_tab(&mut self, host: &NSView, tab: &Tab) -> bool {
        let Some(loan) = self
            .loan
            .as_mut()
            .filter(|loan| ptr::eq(&*loan.tab, tab) && mounted(&loan.page, host))
        else {
            return false;
        };
        loan.close = true;
        true
    }

    fn clear(&mut self) -> Option<Loan> {
        let loan = self.loan.take()?;
        self.generation = self.generation.wrapping_add(1);
        Some(loan)
    }

    pub fn reclaim(&mut self, lender: &NSView, tab: &Tab) -> bool {
        let Some(page) = tab.view() else {
            return false;
        };
        let Some(window) = lender.window() else {
            return false;
        };
        let own_loan = self.loan.as_ref().is_some_and(|loan| {
            ptr::eq(&*loan.page, &*page)
                && loan
                    .lender
                    .load()
                    .is_some_and(|source| ptr::eq(&*source, lender))
        });
        if own_loan {
            if let Some(loan) = self.clear() {
                without_actions(|| restore_opacity(&loan));
            }
        } else if superview(&page).is_some() {
            return false;
        }
        if !mounted(&page, lender) {
            if page.window().is_none_or(|old| !ptr::eq(&*old, &*window)) {
                page.removeFromSuperview();
            }
            without_actions(|| lender.addSubview(&page));
        }
        true
    }

    pub fn drop_stand_in(
        state: &Rc<RefCell<Self>>,
        host: &NSView,
        current: impl FnOnce(&NSView) -> Option<Retained<NSView>> + 'static,
        restore: impl FnOnce(&NSView, &Tab) + 'static,
    ) {
        let generation = {
            let mut inner = state.borrow_mut();
            let Some(loan) = inner
                .loan
                .as_mut()
                .filter(|loan| loan.leaving_opacity.is_none())
            else {
                return;
            };
            if !mounted(&loan.page, host) {
                drop(inner);
                return finish(state, Some(host), current, restore);
            }
            without_actions(|| {
                loan.leaving_opacity = Some(loan.page.layer().map_or(0.0, |layer| {
                    let opacity = layer.opacity();
                    layer.setOpacity(0.0);
                    opacity
                }));
            });
            inner.generation = inner.generation.wrapping_add(1);
            inner.generation
        };
        let weak = Rc::downgrade(state);
        let host = Weak::from(host);
        main_queue::after(
            TEARDOWN_DELAY,
            Box::new(move || {
                if let Some(state) = weak
                    .upgrade()
                    .filter(|state| state.borrow().still_holds(generation))
                {
                    finish(&state, host.load().as_deref(), current, restore);
                }
            }),
        );
    }

    pub fn hit_test(
        &self,
        host: &NSView,
        point: NSPoint,
        hit: Option<Retained<NSView>>,
    ) -> Option<Retained<NSView>> {
        let Some(loan) = self.loan.as_ref().filter(|loan| {
            mounted(&loan.page, host)
                && hit
                    .as_ref()
                    .is_some_and(|hit| hit.isDescendantOf(&loan.page))
        }) else {
            return hit;
        };
        let local = host.convertPoint_fromView(point, superview(host).as_deref());
        let children = host.subviews();
        for index in (0..children.count()).rev() {
            let child = children.objectAtIndex(index);
            if !ptr::eq(&*child, &*loan.page)
                && !child.isHidden()
                && let Some(hit) = child.hitTest(local)
            {
                return Some(hit);
            }
        }
        Some(host.retain())
    }
}

fn restore_opacity(loan: &Loan) {
    if let Some(opacity) = loan.leaving_opacity
        && let Some(layer) = loan.page.layer()
        && layer.opacity() == 0.0
    {
        layer.setOpacity(opacity);
    }
}

fn finish(
    state: &RefCell<StandIn>,
    host: Option<&NSView>,
    current: impl FnOnce(&NSView) -> Option<Retained<NSView>>,
    restore: impl FnOnce(&NSView, &Tab),
) {
    let loan = state.borrow_mut().clear();
    let Some(loan) = loan else { return };
    without_actions(|| {
        if host.is_some_and(|host| mounted(&loan.page, host)) {
            loan.page.removeFromSuperview();
        }
        restore_opacity(&loan);
    });
    if loan.close && !loan.tab.closed() {
        loan.tab.close_now();
    } else if let Some(lender) = loan.lender.load().filter(|lender| {
        lender.window().is_some()
            && current(lender)
                .as_deref()
                .is_some_and(|shown| ptr::eq(shown, &*loan.page))
    }) && superview(&loan.page).is_none()
    {
        restore(&lender, &loan.tab);
    }
}
