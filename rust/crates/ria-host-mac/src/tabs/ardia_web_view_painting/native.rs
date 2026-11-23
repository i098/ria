// Ported from ardia packages/ardiacore/ios/ArdiaWebView+Painting.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::{Cell, OnceCell};
use std::collections::VecDeque;

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Bool, NSObject, NSObjectProtocol};
use objc2::{MainThreadOnly, class, define_class, msg_send};
use objc2_app_kit::NSView;
use objc2_foundation::{NSArray, NSRect};

#[derive(Clone, Copy, Default)]
pub struct ContentDrift {
    pub layouts: u64,
    pub fits: u64,
    pub max_points: f64,
}

thread_local! {
    static DRIFT: Cell<ContentDrift> = Cell::new(ContentDrift::default());
    static FIT: OnceCell<Retained<WidgetLayerFit>> = const { OnceCell::new() };
}

pub fn take_content_drift() -> ContentDrift {
    DRIFT.with(Cell::take)
}

const FILLS_WIDGET: u32 = (1 << 1) | (1 << 4);

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "RiaWidgetLayerFit"]
    pub struct WidgetLayerFit;

    impl WidgetLayerFit {
        #[unsafe(method(layoutSublayersOfLayer:))]
        fn layout(&self, layer: &AnyObject) {
            let mut drift = DRIFT.with(Cell::get);
            drift.layouts += 1;
            // SAFETY: Core Animation passes a live CALayer. Its sublayers have the same selectors.
            unsafe {
                let bounds: NSRect = msg_send![layer, bounds];
                let sublayers: Option<Retained<NSArray<AnyObject>>> = msg_send![layer, sublayers];
                if let Some(sublayers) = sublayers {
                    for sublayer in &*sublayers {
                        let mask: u32 = msg_send![&*sublayer, autoresizingMask];
                        if mask != FILLS_WIDGET {
                            continue;
                        }
                        let frame: NSRect = msg_send![&*sublayer, frame];
                        if frame == bounds {
                            continue;
                        }
                        let points = frame_drift(frame, bounds);
                        if points >= 0.5 {
                            drift.fits += 1;
                            drift.max_points = drift.max_points.max(points);
                        }
                        let transaction = class!(CATransaction);
                        let (): () = msg_send![transaction, begin];
                        let (): () = msg_send![transaction, setDisableActions: Bool::YES];
                        let (): () = msg_send![&*sublayer, setFrame: bounds];
                        let (): () = msg_send![transaction, commit];
                    }
                }
            }
            DRIFT.with(|cell| cell.set(drift));
        }
    }
);

fn frame_drift(a: NSRect, b: NSRect) -> f64 {
    let dx = a.origin.x - b.origin.x;
    let dy = a.origin.y - b.origin.y;
    dx.abs()
        .max(dy.abs())
        .max((dx + a.size.width - b.size.width).abs())
        .max((dy + a.size.height - b.size.height).abs())
}

pub(super) fn fit_render_widgets(page: &NSView) {
    let Some(widget_class) = AnyClass::get(c"RenderWidgetHostViewCocoa") else {
        return;
    };
    let mtm = page.mtm();
    FIT.with(|cell| {
        let fit = cell.get_or_init(|| {
            // SAFETY: The class inherits NSObject's initializer and is confined to the main thread.
            unsafe { msg_send![WidgetLayerFit::alloc(mtm), init] }
        });
        let mut queue = VecDeque::from([objc2::Message::retain(page)]);
        while let Some(view) = queue.pop_front() {
            if !view.isKindOfClass(widget_class) {
                queue.extend(view.subviews().to_vec());
                continue;
            }
            if let Some(layer) = view.layer() {
                // SAFETY: The view vends a CALayer. The fit object implements the layout callback.
                unsafe {
                    let registered: *mut AnyObject = msg_send![&*layer, layoutManager];
                    if registered.cast_const() != Retained::as_ptr(fit).cast::<AnyObject>() {
                        let (): () = msg_send![&*layer, setLayoutManager: &**fit];
                    }
                    let (): () = msg_send![&*layer, setNeedsLayout];
                }
            }
        }
    });
}
