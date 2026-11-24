// Ported from ardia packages/ardiacore/ios/ChromiumTabView.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! `ChromiumTabView`: the page host of the Chromium page view. Events of the page go to the `onPageEvent` channel.

use std::ops::Deref;
use std::rc::Rc;

use objc2::MainThreadMarker;
use serde_json::Value;

use super::page_tab::PageTab;
use super::page_tab_host::PageTabHost;

pub struct ChromiumTabView(PageTabHost);

impl ChromiumTabView {
    /// `init`: `page` is the `ArdiaWebView` of `makePage`; its events go to `send` (`webView:event:payload:`).
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
