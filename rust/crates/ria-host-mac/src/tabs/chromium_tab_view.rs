// Ported from ardia packages/ardiacore/ios/ChromiumTabView.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! `ChromiumTabView`: the page host of the Chromium page view. Events of the page go to the `onPageEvent` channel.
//!
//! Changes from Ardia: The factory returns a native view. The host takes a separate `PageTab` until the port implements that trait.

use std::ops::Deref;
use std::rc::Rc;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_foundation::NSRect;
use serde_json::Value;

use super::ardia_web_view::{ArdiaWebView, LiveViews};
use super::page_tab::PageTab;
use super::page_tab_host::PageTabHost;

pub struct ChromiumTabView(PageTabHost);

impl ChromiumTabView {
    /// `makePage`: Create a zero-frame Chromium view and register it.
    pub fn make_page(mtm: MainThreadMarker, live: &LiveViews) -> Retained<ArdiaWebView> {
        ArdiaWebView::new(mtm, NSRect::ZERO, live)
    }

    /// `init`: Send the supplied page's events through the host.
    pub fn new(
        mtm: MainThreadMarker,
        page: Box<dyn PageTab>,
        on_page_event: Rc<dyn Fn(Value)>,
    ) -> Self {
        let host = PageTabHost::new(mtm, page, on_page_event);
        host.page.set_event_sink(host.event_sink());
        Self(host)
    }
}

impl Deref for ChromiumTabView {
    type Target = PageTabHost;

    fn deref(&self) -> &PageTabHost {
        &self.0
    }
}
