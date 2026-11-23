// Ported from ardia packages/ardiacore/ios/ArdiaWebView+Recording.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::ptr;
use std::rc::Rc;

use block2::DynBlock;
use objc2::msg_send;
use objc2::rc::Retained;
use objc2_app_kit::{NSView, NSWindow};

use crate::core::bindings::{CGImageRef, Tab, Window};
use crate::windows::ardia_window_controller::Controllers;

use super::superview;

pub fn recording_shows_page(host: &NSView, tab: Option<&Tab>) -> bool {
    tab.is_some_and(|tab| {
        let page = tab.view();
        superview(&page).is_some_and(|parent| ptr::eq(&*parent, host))
            && !page.isHidden()
            && page.alphaValue() > 0.01
    })
}

pub fn copy_recording_picture(
    host: &NSView,
    tab: Option<&Tab>,
    completion: &DynBlock<dyn Fn(CGImageRef)>,
) {
    if let Some(tab) = tab.filter(|tab| {
        recording_shows_page(host, Some(tab))
            && host.window().is_some()
            && !host.isHiddenOrHasHiddenAncestor()
    }) {
        tab.copy_picture(0.5, completion);
    } else {
        completion.call((CGImageRef(ptr::null_mut()),));
    }
}

pub fn owned_picture_in_picture_windows(
    controllers: &Controllers,
    window: &NSWindow,
    candidates: impl IntoIterator<Item = (Retained<NSWindow>, Retained<Tab>)>,
) -> Vec<Retained<NSWindow>> {
    let Some(owner) = controllers
        .for_window(window)
        .filter(|owner| ptr::eq(&*owner.core.window(), window))
    else {
        return Vec::new();
    };
    candidates
        .into_iter()
        .filter_map(|(candidate, tab)| {
            (!tab.closed()
                && controllers
                    .holding(&tab)
                    .is_some_and(|holder| Rc::ptr_eq(&holder, &owner)))
            .then_some(candidate)
        })
        .collect()
}

pub fn recording_window_is_private(controllers: &Controllers, window: &NSWindow) -> bool {
    controllers
        .for_window(window)
        .and_then(|owner| owner.core.active_profile())
        .is_none_or(|profile| profile.off_the_record())
}

pub fn recording_corner_radius(window: &NSWindow, picture_in_picture_radius: Option<f64>) -> f64 {
    picture_in_picture_radius.unwrap_or_else(|| {
        window
            .contentView()
            .and_then(|view| view.layer())
            // SAFETY: CALayer's cornerRadius returns a CGFloat.
            .map_or(0.0, |layer| unsafe { msg_send![&*layer, cornerRadius] })
    })
}

pub fn copy_recording_picture_of_window(
    window: &NSWindow,
    completion: &DynBlock<dyn Fn(CGImageRef)>,
) {
    Window::copy_picture_of_chrome_window(window, completion);
}
