// Ported from ardia packages/ardiacore/ios/ArdiaPiPHandle.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The tab at the screen edge that shows a tucked Picture in Picture window again.

use std::cell::{Cell, RefCell};

use objc2::rc::Retained;
use objc2::{AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSColor, NSCursor, NSEvent, NSFontWeightSemibold, NSImage, NSImageSymbolConfiguration,
    NSImageView, NSTrackingArea, NSTrackingAreaOptions, NSView,
};
use objc2_foundation::{NSRect, ns_string};

use super::ardia_picture_in_picture_internal::{Edge, height, rect, width};

pub struct HandleIvars {
    chevron: Retained<NSImageView>,
    tracking: RefCell<Option<Retained<NSTrackingArea>>>,
    hovered: Cell<bool>,
    edge: Cell<Edge>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "RiaPiPHandle"]
    #[ivars = HandleIvars]
    pub struct Handle;

    impl Handle {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        #[unsafe(method(updateTrackingAreas))]
        fn update_tracking_areas(&self) {
            // SAFETY: This calls NSView's implementation, which takes no arguments.
            unsafe { msg_send![super(self), updateTrackingAreas] }
            if let Some(tracking) = self.ivars().tracking.borrow_mut().take() {
                self.removeTrackingArea(&tracking);
            }
            // SAFETY: The handle owns the area and removes it before it adds a new one.
            let tracking = unsafe {
                NSTrackingArea::initWithRect_options_owner_userInfo(
                    NSTrackingArea::alloc(),
                    NSRect::ZERO,
                    NSTrackingAreaOptions::MouseEnteredAndExited
                        | NSTrackingAreaOptions::ActiveAlways
                        | NSTrackingAreaOptions::InVisibleRect
                        | NSTrackingAreaOptions::CursorUpdate,
                    Some(self),
                    None,
                )
            };
            self.addTrackingArea(&tracking);
            *self.ivars().tracking.borrow_mut() = Some(tracking);
        }

        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
            self.set_hovered(true);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            self.set_hovered(false);
        }

        #[unsafe(method(cursorUpdate:))]
        fn cursor_update(&self, _event: &NSEvent) {
            NSCursor::pointingHandCursor().set();
        }

        #[unsafe(method(layout))]
        fn layout(&self) {
            // SAFETY: This calls NSView's implementation, which takes no arguments.
            unsafe { msg_send![super(self), layout] }
            let (w, h) = (16.0, 28.0);
            let bounds = self.bounds();
            self.ivars().chevron.setFrame(rect(
                ((width(bounds) - w) / 2.0).floor(),
                ((height(bounds) - h) / 2.0).floor(),
                w,
                h,
            ));
        }
    }
);

impl Handle {
    /// `initWithFrame:`.
    pub fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(HandleIvars {
            chevron: NSImageView::new(mtm),
            tracking: RefCell::default(),
            hovered: Cell::default(),
            edge: Cell::default(),
        });
        // SAFETY: The allocation has its Rust ivars before NSView initialization.
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };
        this.setWantsLayer(true);
        let chevron = &this.ivars().chevron;
        chevron.setContentTintColor(Some(&NSColor::colorWithWhite_alpha(1.0, 0.95)));
        // SAFETY: `NSFontWeightSemibold` is an AppKit constant.
        let weight = unsafe { NSFontWeightSemibold };
        chevron.setSymbolConfiguration(Some(
            &NSImageSymbolConfiguration::configurationWithPointSize_weight(15.0, weight),
        ));
        this.addSubview(chevron);
        this.setToolTip(Some(ns_string!("Show Picture in Picture")));
        this.set_hovered(false);
        this
    }

    pub fn hovered(&self) -> bool {
        self.ivars().hovered.get()
    }

    pub fn set_hovered(&self, hovered: bool) {
        self.ivars().hovered.set(hovered);
        if let Some(layer) = self.layer() {
            let color =
                NSColor::colorWithWhite_alpha(0.0, if hovered { 0.7 } else { 0.5 }).CGColor();
            // SAFETY: CALayer takes a borrowed CGColor and retains it as needed.
            let _: () = unsafe { msg_send![&*layer, setBackgroundColor: &*color] };
        }
    }

    pub fn edge(&self) -> Edge {
        self.ivars().edge.get()
    }

    pub fn set_edge(&self, edge: Edge) {
        self.ivars().edge.set(edge);
        let name = if edge == Edge::Left {
            ns_string!("chevron.compact.right")
        } else {
            ns_string!("chevron.compact.left")
        };
        self.ivars().chevron.setImage(
            NSImage::imageWithSystemSymbolName_accessibilityDescription(
                name,
                Some(ns_string!("Show Picture in Picture")),
            )
            .as_deref(),
        );
        self.setNeedsLayout(true);
    }
}
