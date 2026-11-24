// Ported from ardia packages/ardiacore/ios/ArdiaPiPOverlay.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The view above the page of a Picture in Picture window that holds the handle.

use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSAutoresizingMaskOptions, NSView};
use objc2_foundation::{NSPoint, NSRect};

use super::ardia_picture_in_picture_internal::{Edge, PEEK, point_in_rect, rect};
use super::ardia_pip_handle::Handle;

pub struct OverlayIvars {
    handle: Retained<Handle>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "RiaPiPOverlay"]
    #[ivars = OverlayIvars]
    pub struct Overlay;

    impl Overlay {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        #[unsafe(method_id(hitTest:))]
        fn hit_test(&self, point: NSPoint) -> Option<Retained<NSView>> {
            // SAFETY: `superview` returns the live parent view or nil.
            let superview = unsafe { self.superview() };
            let local = self.convertPoint_fromView(point, superview.as_deref());
            let handle = &self.ivars().handle;
            (!handle.isHidden() && point_in_rect(local, handle.frame()))
                .then(|| Retained::into_super(handle.clone()))
        }

        #[unsafe(method(layout))]
        fn layout(&self) {
            // SAFETY: This calls NSView's implementation, which takes no arguments.
            unsafe { msg_send![super(self), layout] }
            let size = self.bounds().size;
            let handle = &self.ivars().handle;
            handle.setFrame(if handle.edge() == Edge::Left {
                rect(size.width - PEEK, 0.0, PEEK, size.height)
            } else {
                rect(0.0, 0.0, PEEK, size.height)
            });
        }
    }
);

impl Overlay {
    /// `initWithFrame:`.
    pub fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(OverlayIvars {
            handle: Handle::new(mtm, NSRect::ZERO),
        });
        // SAFETY: The allocation has its Rust ivars before NSView initialization.
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };
        this.setWantsLayer(true);
        if let Some(layer) = this.layer() {
            // SAFETY: CALayer takes a CGFloat z position.
            let _: () = unsafe { msg_send![&*layer, setZPosition: 100.0f64] };
        }
        this.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        let handle = &this.ivars().handle;
        handle.setHidden(true);
        handle.setAlphaValue(0.0);
        this.addSubview(handle);
        this
    }

    pub fn handle(&self) -> Retained<Handle> {
        self.ivars().handle.clone()
    }
}
