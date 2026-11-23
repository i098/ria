// Ported from ardia packages/ardiacore/ios/ArdiaRendererScrollObserver.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use objc2::rc::{Retained, Weak};
use objc2::runtime::NSObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send};
use objc2_app_kit::NSView;
use ria_engine::call::main_queue;

use crate::core::bindings::{PageScrollAck, PageView};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollPhase {
    #[default]
    AwaitingBegin,
    AwaitingFirstUpdate,
    FirstUpdateConsumed,
    FirstUpdateUnconsumed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RendererScroll {
    pub phase: ScrollPhase,
    pub overscroll: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollEvent {
    Ack(PageScrollAck, bool),
    Overscroll(bool),
}

impl RendererScroll {
    pub fn apply(&mut self, event: ScrollEvent) {
        self.phase = match (self.phase, event) {
            (ScrollPhase::AwaitingBegin, ScrollEvent::Ack(PageScrollAck::Begin, _)) => {
                ScrollPhase::AwaitingFirstUpdate
            }
            (ScrollPhase::AwaitingFirstUpdate, ScrollEvent::Ack(PageScrollAck::Update, true)) => {
                ScrollPhase::FirstUpdateConsumed
            }
            (ScrollPhase::AwaitingFirstUpdate, ScrollEvent::Ack(PageScrollAck::Update, false)) => {
                ScrollPhase::FirstUpdateUnconsumed
            }
            (phase, _) => phase,
        };
        if let ScrollEvent::Overscroll(allowed) = event {
            self.overscroll = Some(allowed);
        }
    }
}

type Scrolls = HashMap<*const NSView, (Weak<NSView>, Rc<Cell<RendererScroll>>)>;

pub struct Ivars {
    scrolls: RefCell<Scrolls>,
    trace: Rc<dyn Fn(ScrollEvent)>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "RiaRendererScrollObserver"]
    #[ivars = Ivars]
    pub struct RendererScrollObserver;

    impl RendererScrollObserver {
        #[unsafe(method(pageView:ackedScroll:consumed:))]
        fn acked(&self, view: &NSView, ack: isize, consumed: bool) {
            let ack = if ack == PageScrollAck::Begin as isize { PageScrollAck::Begin } else { PageScrollAck::Update };
            self.enqueue(view, ScrollEvent::Ack(ack, consumed));
        }

        #[unsafe(method(pageView:overscrolledAllowingHorizontal:))]
        fn overscrolled(&self, view: &NSView, allowed: bool) {
            self.enqueue(view, ScrollEvent::Overscroll(allowed));
        }
    }
);

impl RendererScrollObserver {
    pub fn install(mtm: MainThreadMarker, trace: impl Fn(ScrollEvent) + 'static) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars {
            scrolls: RefCell::default(),
            trace: Rc::new(trace),
        });
        // SAFETY: The allocated class has NSObject as its superclass.
        let observer: Retained<Self> = unsafe { msg_send![super(this), init] };
        // SAFETY: Both selectors of ArdiaCorePageScrollObserver are registered above.
        unsafe { PageView::set_scroll_observer(Some(&observer)) };
        observer
    }

    pub fn renderer_scroll_for(&self, hit: &NSView) -> Option<Rc<Cell<RendererScroll>>> {
        let page = page_view_of(hit)?;
        if !PageView::take_swipes(&page) {
            return None;
        }
        let mut scrolls = self.ivars().scrolls.borrow_mut();
        scrolls.retain(|_, (page, _)| page.load().is_some());
        Some(
            scrolls
                .entry(std::ptr::from_ref(&*page))
                .or_insert_with(|| (Weak::new(&*page), Rc::default()))
                .1
                .clone(),
        )
    }

    fn enqueue(&self, view: &NSView, event: ScrollEvent) {
        let observer = Weak::new(self);
        let view = Weak::new(view);
        main_queue::post(Box::new(move || {
            let Some(observer) = observer.load() else {
                return;
            };
            (observer.ivars().trace)(event);
            let Some(view) = view.load() else { return };
            if let Some((_, scroll)) = observer
                .ivars()
                .scrolls
                .borrow()
                .get(&std::ptr::from_ref(&*view))
            {
                let mut next = scroll.get();
                next.apply(event);
                scroll.set(next);
            }
        }));
    }
}

pub fn page_view_of(hit: &NSView) -> Option<Retained<NSView>> {
    let mut view = hit.retain();
    loop {
        if PageView::is_page_view(&view) {
            return Some(view);
        }
        view = super::superview(&view)?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_first_update_changes_renderer_claim() {
        let mut scroll = RendererScroll::default();
        scroll.apply(ScrollEvent::Ack(PageScrollAck::Update, true));
        assert_eq!(scroll.phase, ScrollPhase::AwaitingBegin);
        scroll.apply(ScrollEvent::Ack(PageScrollAck::Begin, false));
        scroll.apply(ScrollEvent::Ack(PageScrollAck::Update, false));
        scroll.apply(ScrollEvent::Ack(PageScrollAck::Update, true));
        scroll.apply(ScrollEvent::Overscroll(true));
        assert_eq!(scroll.phase, ScrollPhase::FirstUpdateUnconsumed);
        assert_eq!(scroll.overscroll, Some(true));
    }
}
