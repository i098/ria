// Ported from ardia packages/ardiacore/ios/PageTabHost.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! `PageTabHost`: the view that holds one page view. The page joins the hierarchy and starts when the first props
//! have arrived, and it closes when the host goes away.
//!
//! Ardia's host is an Expo view that Expo sizes; here the caller that owns the layout calls `set_frame_size`.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::NSView;
use objc2_foundation::{NSRect, NSSize};
use serde_json::{Map, Value};

use super::page_events;
use super::page_tab::{EventSink, PageTab};

pub struct PageTabHost {
    view: Retained<NSView>,
    pub page: Box<dyn PageTab>,
    page_events: Rc<RefCell<HashSet<String>>>,
    dispatch: Rc<dyn Fn(Value)>,
    props_ready: Cell<bool>,
}

impl PageTabHost {
    /// `init`: `dispatch` is the `onPageEvent` event of the view.
    pub fn new(mtm: MainThreadMarker, page: Box<dyn PageTab>, dispatch: Rc<dyn Fn(Value)>) -> Self {
        let view = NSView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        view.setWantsLayer(true);
        Self {
            view,
            page,
            page_events: Rc::default(),
            dispatch,
            props_ready: Cell::new(false),
        }
    }

    pub fn view(&self) -> &NSView {
        &self.view
    }

    /// The `pageEvents` prop: the names of the events the app wants.
    pub fn set_page_events(&self, names: Option<Vec<String>>) {
        *self.page_events.borrow_mut() = names.unwrap_or_default().into_iter().collect();
    }

    /// `setFrameSize:`: the page follows the size of the host.
    pub fn set_frame_size(&self, size: NSSize) {
        self.view.setFrameSize(size);
        self.page.view().setFrame(self.view.bounds());
    }

    /// `propsDidUpdate`: the first call puts the page in the view and starts it; later calls do nothing.
    pub fn props_did_update(&self) {
        if self.props_ready.replace(true) {
            return;
        }
        let page = self.page.view();
        page.setFrame(self.view.bounds());
        self.view.addSubview(page);
        self.page.start_page();
    }

    /// `send`.
    pub fn send(&self, name: &str, payload: Map<String, Value>) {
        page_events::send(&*self.dispatch, &self.page_events.borrow(), name, payload);
    }

    /// The sink for the events of the page; it holds the host's event state, not the host.
    pub fn event_sink(&self) -> EventSink {
        let wanted = self.page_events.clone();
        let dispatch = self.dispatch.clone();
        Rc::new(move |name, payload| {
            page_events::send(&*dispatch, &wanted.borrow(), name, payload);
        })
    }
}

impl Drop for PageTabHost {
    fn drop(&mut self) {
        self.page.close_page();
    }
}
