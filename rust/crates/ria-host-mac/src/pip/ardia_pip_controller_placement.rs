// Ported from ardia packages/ardiacore/ios/ArdiaPiPController+Placement.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Keeps AppKit from pulling a Picture in Picture window back where the throw math put it.

use std::ptr::NonNull;

use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2::runtime::{AnyClass, Sel};
use objc2::{ffi, sel};
use objc2_app_kit::{NSScreen, NSWindow};
use objc2_foundation::NSRect;

use super::ardia_picture_in_picture_internal::{replace_method, throw_rect, throw_screens};
use super::ardia_pip_controller::Controller;
use super::ardia_pip_throw_math as throw;

fn where_the_throw_maths_may_put_it(mtm: MainThreadMarker, rect: NSRect) -> bool {
    let screens = throw_screens(mtm);
    !screens.is_empty() && throw::shortfall(throw_rect(rect), &screens) <= 1e-9
}

/// `+[ArdiaPiPController(Placement) load]`.
pub fn install() {
    let constrain = sel!(constrainFrameRect:toScreen:);
    let Some(class) = AnyClass::get(c"NativeWidgetMacFramelessNSWindow") else {
        return;
    };
    let Some(method) = class.instance_method(constrain) else {
        return;
    };
    // SAFETY: `constrainFrameRect:toScreen:` takes a rect and a screen or nil, and returns a rect.
    let original: unsafe extern "C-unwind" fn(
        NonNull<NSWindow>,
        Sel,
        NSRect,
        *const NSScreen,
    ) -> NSRect = unsafe { std::mem::transmute(method.implementation()) };
    let block = RcBlock::new(
        move |window: NonNull<NSWindow>, rect: NSRect, screen: *const NSScreen| {
            // SAFETY: the runtime gives a live window as `self`.
            let live = unsafe { window.as_ref() };
            if Controller::for_window(live).is_some()
                && where_the_throw_maths_may_put_it(MainThreadMarker::from(live), rect)
            {
                return rect;
            }
            // SAFETY: the replaced implementation of the same selector.
            unsafe { original(window, constrain, rect, screen) }
        },
    );
    // SAFETY: the runtime copies the block; its arguments are `self` and the method arguments.
    let imp = unsafe { ffi::imp_implementationWithBlock(RcBlock::as_ptr(&block).cast()) };
    replace_method(class, constrain, imp);
}
