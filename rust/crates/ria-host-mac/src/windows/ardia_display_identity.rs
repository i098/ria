// Ported from ardia packages/ardiacore/ios/ArdiaDisplayIdentity.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.
// Companion declaration: packages/ardiacore/ios/ArdiaDisplayIdentity.h.

use std::ffi::c_void;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::NSScreen;
use objc2_foundation::{NSNumber, NSPoint, NSRect, NSSize, NSString, ns_string};

use super::ardia_display_placement_math::on_screen;
use super::ardia_window_sight_math::Rect;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGDisplayCreateUUIDFromDisplayID(display: u32) -> *mut c_void;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFUUIDCreateString(allocator: *const c_void, uuid: *const c_void) -> *mut NSString;
    fn CFRelease(object: *const c_void);
}

pub fn identifier(screen: &NSScreen) -> Option<Retained<NSString>> {
    let number = screen
        .deviceDescription()
        .objectForKey(ns_string!("NSScreenNumber"))?
        .downcast::<NSNumber>()
        .ok()?;
    // SAFETY: The display ID comes from AppKit. The create rule gives us one reference to the UUID.
    let uuid = unsafe { CGDisplayCreateUUIDFromDisplayID(number.unsignedIntValue()) };
    if uuid.is_null() {
        return None;
    }
    // SAFETY: `uuid` is still owned here. The result is a +1 CFString, toll-free bridged to NSString.
    unsafe {
        let text = CFUUIDCreateString(std::ptr::null(), uuid);
        CFRelease(uuid);
        Retained::from_raw(text)
    }
}

pub fn screen_with_identifier(
    mtm: MainThreadMarker,
    wanted: &NSString,
) -> Option<Retained<NSScreen>> {
    if wanted.is_empty() {
        return None;
    }
    NSScreen::screens(mtm).iter().find(|screen| {
        identifier(screen).is_some_and(|value| {
            value.caseInsensitiveCompare(wanted) == objc2_foundation::NSComparisonResult::Same
        })
    })
}

pub fn frame_on_screen(mtm: MainThreadMarker, frame: NSRect, target: &NSScreen) -> NSRect {
    let screens = NSScreen::screens(mtm);
    let fitted = on_screen(
        rect(frame),
        screens.iter().map(|screen| rect(screen.frame())),
        screens.indexOfObject(target),
        rect(target.visibleFrame()),
    );
    NSRect::new(
        NSPoint::new(fitted.x, fitted.y),
        NSSize::new(fitted.width, fitted.height),
    )
}

fn rect(frame: NSRect) -> Rect {
    Rect::new(
        frame.origin.x,
        frame.origin.y,
        frame.size.width,
        frame.size.height,
    )
}
