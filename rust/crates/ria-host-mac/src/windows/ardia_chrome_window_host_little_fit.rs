// Ported from ardia packages/ardiacore/ios/ArdiaChromeWindowHost+LittleFit.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use objc2::MainThreadOnly;
use objc2_app_kit::{NSScreen, NSWindow};
use objc2_foundation::{NSPoint, NSRect, NSSize};

use super::ardia_display_placement_math::Rect;
use super::ardia_little_fit_math::{
    Display, Placement, Size, fit, fit_landing, flip_from_top_left,
};
use super::ardia_window_full_screen::on_screen_window_bounds;

fn from_rect(rect: NSRect) -> Rect {
    Rect::new(
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
    )
}

fn to_rect(rect: Rect) -> NSRect {
    NSRect::new(NSPoint::new(rect.x, rect.y), NSSize::new(rect.w, rect.h))
}

pub fn little_frame_for_size(size: NSSize, minimum: NSSize, visible: NSRect) -> NSRect {
    to_rect(fit(
        from_rect(visible),
        Size::new(size.width, size.height),
        Size::new(minimum.width, minimum.height),
    ))
}

pub fn little_frame_fitting_landing(
    window: &NSWindow,
    minimum: NSSize,
    placement: Option<(NSSize, NSRect)>,
) -> Option<NSRect> {
    let screens = NSScreen::screens(window.mtm());
    let primary = screens.firstObject()?;
    let landed = flip_from_top_left(
        from_rect(on_screen_window_bounds(window)?),
        primary.frame().size.height,
    );
    let displays: Vec<_> = screens
        .iter()
        .map(|screen| Display {
            frame: from_rect(screen.frame()),
            visible: from_rect(screen.visibleFrame()),
        })
        .collect();
    let placement = placement.map(|(wanted, placed)| Placement {
        wanted: Size::new(wanted.width, wanted.height),
        placed: from_rect(placed),
    });
    fit_landing(
        landed,
        &displays,
        Size::new(minimum.width, minimum.height),
        placement,
    )
    .map(to_rect)
}
