// Ported from ardia packages/ardiacore/ios/ArdiaPiPZoom.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Zoom of a Picture in Picture window: Command-scroll and pinch, and the edge resize that a stand-in panel shows.

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::ptr;

use objc2::encode::{Encode, Encoding, RefEncode};
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyClass, AnyObject, NSObject};
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, Message, class, define_class, ffi, msg_send,
    sel,
};
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSEvent, NSEventModifierFlags, NSEventPhase, NSEventType, NSPanel,
    NSWindow, NSWindowAnimationBehavior, NSWindowOrderingMode, NSWindowStyleMask,
};
use objc2_foundation::{
    NSArray, NSPoint, NSRect, NSRunLoop, NSRunLoopCommonModes, NSSize, NSString, ns_string,
};
use serde_json::{Value, json};

use super::ardia_picture_in_picture_internal::{
    Edge, RIM_ALPHA, after, begin_stamp, equal_rects, height,
    is_chrome_document_picture_in_picture, is_chrome_floating_window, max_x, max_y, mid_x, mid_y,
    min_x, min_y, point_in_rect, pointer_of, rect, rect_string, resizable, resize_direction,
    screen_for, services, stamp_onto, throw_rect, width, zoom_enabled,
};
use super::ardia_pip_controller::Controller;
use super::ardia_pip_throw_math::{Point, std_clamp};
use super::ardia_pip_zoom_math as zoom_math;

const RETAIN_NONATOMIC: usize = 1;
/// `kZoomKey`: the key of the associated object that holds the zoom of a window.
static ZOOM_KEY: u8 = 0;
const SIDES: [[i32; 2]; 8] = [
    [1, 0],
    [1, 1],
    [0, 1],
    [-1, 1],
    [-1, 0],
    [-1, -1],
    [0, -1],
    [1, -1],
];

/// `CATransform3D`.
#[repr(C)]
#[derive(Clone, Copy)]
struct Transform3D([f64; 16]);

const DOUBLE: Encoding = Encoding::Double;
// SAFETY: CATransform3D is 16 CGFloats, in this order.
unsafe impl Encode for Transform3D {
    const ENCODING: Encoding = Encoding::Struct("CATransform3D", &[DOUBLE; 16]);
}

impl Transform3D {
    fn scale(sx: f64, sy: f64, sz: f64) -> Self {
        let mut m = [0.0; 16];
        (m[0], m[5], m[10], m[15]) = (sx, sy, sz, 1.0);
        Self(m)
    }
}

#[repr(C)]
struct CGPath {
    _private: [u8; 0],
}

// SAFETY: CGPathRef is a pointer to the opaque `struct CGPath`.
unsafe impl RefEncode for CGPath {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Encoding::Struct("CGPath", &[]));
}

#[link(name = "QuartzCore", kind = "framework")]
unsafe extern "C" {
    static kCACornerCurveCircular: &'static NSString;
    fn CACurrentMediaTime() -> f64;
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGPathCreateWithRoundedRect(
        rect: NSRect,
        corner_width: f64,
        corner_height: f64,
        transform: *const c_void,
    ) -> *mut CGPath;
    fn CGPathRelease(path: *mut CGPath);
}

fn media_time() -> f64 {
    // SAFETY: CACurrentMediaTime takes no arguments and returns the host time.
    unsafe { CACurrentMediaTime() }
}

/// `[CATransaction begin]` and `setDisableActions:YES` around `work`.
fn without_actions(work: impl FnOnce()) {
    // SAFETY: CATransaction class methods, called on the main thread and paired.
    unsafe {
        let _: () = msg_send![class!(CATransaction), begin];
        let _: () = msg_send![class!(CATransaction), setDisableActions: true];
    }
    work();
    // SAFETY: as above.
    unsafe {
        let _: () = msg_send![class!(CATransaction), commit];
    }
}

fn new_layer() -> Retained<AnyObject> {
    // SAFETY: `+[CALayer layer]` returns an autoreleased CALayer.
    unsafe { msg_send![class!(CALayer), layer] }
}

fn superlayer(layer: Option<&AnyObject>) -> Option<Retained<AnyObject>> {
    // SAFETY: `superlayer` is a CALayer getter that returns a CALayer or nil.
    layer.and_then(|layer| unsafe { msg_send![layer, superlayer] })
}

fn bounds(layer: Option<&AnyObject>) -> NSRect {
    // SAFETY: `bounds` is a CALayer getter that returns a CGRect.
    layer.map_or(NSRect::ZERO, |layer| unsafe { msg_send![layer, bounds] })
}

fn add_sublayer(layer: &AnyObject, sublayer: &AnyObject) {
    // SAFETY: both are CALayers.
    unsafe { msg_send![layer, addSublayer: sublayer] }
}

/// `LayerHostIn`: the first `CALayerHost` in the tree of `layer`.
fn layer_host_in(layer: &AnyObject) -> Option<Retained<AnyObject>> {
    let host = AnyClass::get(c"CALayerHost")?;
    // SAFETY: `isKindOfClass:` is sent to an NSObject; `sublayers` is a CALayer getter of an array of CALayers.
    unsafe {
        if msg_send![layer, isKindOfClass: host] {
            return Some(layer.retain());
        }
        let sublayers: Option<Retained<NSArray<AnyObject>>> = msg_send![layer, sublayers];
        sublayers?
            .iter()
            .find_map(|sublayer| layer_host_in(&sublayer))
    }
}

/// `CopyGeometry`: a new layer with the geometry of `source`.
fn copy_geometry(source: &AnyObject) -> Retained<AnyObject> {
    let layer = new_layer();
    // SAFETY: CALayer geometry getters and setters, with their CoreGraphics and CATransform3D types.
    unsafe {
        let anchor: NSPoint = msg_send![source, anchorPoint];
        let _: () = msg_send![&layer, setAnchorPoint: anchor];
        let _: () = msg_send![&layer, setBounds: bounds(Some(source))];
        let position: NSPoint = msg_send![source, position];
        let _: () = msg_send![&layer, setPosition: position];
        let transform: Transform3D = msg_send![source, transform];
        let _: () = msg_send![&layer, setTransform: transform];
        let sublayer_transform: Transform3D = msg_send![source, sublayerTransform];
        let _: () = msg_send![&layer, setSublayerTransform: sublayer_transform];
        let flipped: bool = msg_send![source, isGeometryFlipped];
        let _: () = msg_send![&layer, setGeometryFlipped: flipped];
    }
    layer
}

fn number_arg(arg: &str) -> Retained<NSString> {
    NSString::from_str(arg)
}

/// Repeats `tick` every `interval` seconds while it returns true, as a repeating NSTimer does.
fn repeat(interval: f64, mut tick: impl FnMut() -> bool + 'static) {
    after(interval, move || {
        if tick() {
            repeat(interval, tick);
        }
    });
}

/// The instance variables of `ArdiaPiPZoom`.
#[derive(Default)]
pub struct ZoomState {
    window: Weak<NSWindow>,
    last: Cell<NSRect>,
    width: Cell<f64>,
    aspect: Cell<f64>,
    target: Cell<f64>,
    pointer: Cell<NSPoint>,
    /// `_ease`: true while the ease timer runs; `ease_generation` stops a running timer.
    ease: Cell<bool>,
    ease_generation: Cell<u64>,
    stand_in: RefCell<Option<Retained<NSPanel>>>,
    card: RefCell<Option<Retained<AnyObject>>>,
    clip: RefCell<Option<Retained<AnyObject>>>,
    mirror: RefCell<Option<Retained<AnyObject>>>,
    outer: RefCell<Option<Retained<AnyObject>>>,
    inner: RefCell<Option<Retained<AnyObject>>>,
    host: RefCell<Option<Retained<AnyObject>>>,
    source: RefCell<Weak<AnyObject>>,
    handing_off: Cell<bool>,
    /// `_standInStamp`: `Value::Null` when nil.
    stand_in_stamp: RefCell<Value>,
    stand_in_pinch: Cell<bool>,
    follow: Cell<f64>,
    resize_start: Cell<NSRect>,
    resize_from: Cell<NSPoint>,
    resize_direction: Cell<isize>,
    released_at: Cell<f64>,
    link: RefCell<Option<Retained<AnyObject>>>,
    stage: Cell<NSRect>,
    pinch_target: Cell<NSRect>,
    shown: Cell<NSRect>,
    ticked_at: Cell<f64>,
    stand_in_width: Cell<f64>,
    hidden_alpha: Cell<f64>,
    applied_at: RefCell<Vec<f64>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "RiaPiPZoom"]
    #[ivars = ZoomState]
    pub struct Zoom;

    impl Zoom {
        #[unsafe(method(tick:))]
        fn tick(&self, link: &AnyObject) {
            self.tick_with(link);
        }
    }
);

thread_local! {
    /// `gZoomGesture`.
    static ZOOM_GESTURE: RefCell<Weak<NSWindow>> = RefCell::default();
    /// `gEdgeResizing`.
    static EDGE_RESIZING: RefCell<Weak<Zoom>> = RefCell::default();
}

fn zoom_gesture() -> Option<Retained<NSWindow>> {
    ZOOM_GESTURE.with(|gesture| gesture.borrow().load())
}

fn set_zoom_gesture(window: Option<&NSWindow>) {
    ZOOM_GESTURE.with(|gesture| *gesture.borrow_mut() = window.map(Weak::new).unwrap_or_default());
}

fn edge_resizing() -> Option<Retained<Zoom>> {
    EDGE_RESIZING.with(|zoom| zoom.borrow().load())
}

fn set_edge_resizing(zoom: Option<&Zoom>) {
    EDGE_RESIZING.with(|slot| *slot.borrow_mut() = zoom.map(Weak::new).unwrap_or_default());
}

/// AppKit sends the events that this file handles on the main thread.
fn main_thread() -> MainThreadMarker {
    MainThreadMarker::new().expect("Picture in Picture zoom runs on the main thread")
}

fn same_window(a: Option<&NSWindow>, b: Option<&NSWindow>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => ptr::eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

impl Zoom {
    fn zooms(window: &NSWindow) -> bool {
        if !window.isVisible() || !window.styleMask().contains(NSWindowStyleMask::Resizable) {
            return false;
        }
        if let Some(controller) = Controller::for_window(window) {
            return controller.stashed_edge() == Edge::None;
        }
        is_chrome_document_picture_in_picture(window) && is_chrome_floating_window(window)
    }

    fn for_window(window: &NSWindow) -> Retained<Zoom> {
        let key = ptr::from_ref(&ZOOM_KEY).cast();
        let object = ptr::from_ref::<NSWindow>(window).cast::<AnyObject>();
        // SAFETY: the associated object under ZOOM_KEY is only ever a Zoom.
        let found = unsafe {
            Retained::retain(
                ffi::objc_getAssociatedObject(object, key)
                    .cast_mut()
                    .cast::<Zoom>(),
            )
        };
        found.unwrap_or_else(|| {
            let state = ZoomState {
                window: Weak::new(window),
                ..ZoomState::default()
            };
            let this = Self::alloc(window.mtm()).set_ivars(state);
            // SAFETY: the allocation has its Rust ivars before NSObject initialization.
            let zoom: Retained<Zoom> = unsafe { msg_send![super(this), init] };
            // SAFETY: the window retains the zoom, as `OBJC_ASSOCIATION_RETAIN_NONATOMIC` does in Ardia.
            unsafe {
                ffi::objc_setAssociatedObject(
                    object.cast_mut(),
                    key,
                    ptr::from_ref::<Zoom>(&zoom).cast_mut().cast(),
                    RETAIN_NONATOMIC,
                );
            }
            zoom
        })
    }

    /// `+handle:`: zooms the window under a Command-scroll or a pinch; true when the event is used.
    pub fn handle(event: &NSEvent) -> bool {
        let handled = Self::route(event);
        let trace = services().is_some_and(|services| services.launch.traced("pipZoom"));
        if trace {
            let mtm = main_thread();
            let window = event.window(mtm);
            let scroll = event.r#type() == NSEventType::ScrollWheel;
            eprintln!(
                "[pip-zoom] type {} window {} ({}) cmd {} phase {}/{} dy {:.2} precise {} mag {:.3} -> {} {}",
                event.r#type().0,
                event.windowNumber(),
                window.as_ref().map_or("(null)".into(), |window| window
                    .class()
                    .name()
                    .to_string_lossy()),
                i32::from(
                    event
                        .modifierFlags()
                        .contains(NSEventModifierFlags::Command)
                ),
                if scroll { event.phase().bits() } else { 0 },
                if scroll {
                    event.momentumPhase().bits()
                } else {
                    0
                },
                if scroll { event.scrollingDeltaY() } else { 0.0 },
                i32::from(scroll && event.hasPreciseScrollingDeltas()),
                if event.r#type() == NSEventType::Magnify {
                    event.magnification()
                } else {
                    0.0
                },
                if handled { "zoomed" } else { "passed" },
                rect_string(window.map_or(NSRect::ZERO, |window| window.frame())),
            );
        }
        handled
    }

    fn window_for(event: &NSEvent) -> Option<Retained<NSWindow>> {
        let mtm = main_thread();
        if let Some(window) = event.window(mtm) {
            return Some(window);
        }
        let point = event.locationInWindow();
        super::ardia_picture_in_picture_internal::shared_application(mtm)
            .orderedWindows()
            .iter()
            .find(|window| point_in_rect(point, window.frame()) && Self::zooms(window))
    }

    fn route(event: &NSEvent) -> bool {
        let mut window = Self::window_for(event);
        let command = event
            .modifierFlags()
            .contains(NSEventModifierFlags::Command);
        if event.r#type() == NSEventType::ScrollWheel {
            let (phase, momentum) = (event.phase(), event.momentumPhase());
            let wheel = phase.is_empty() && momentum.is_empty();
            if wheel || phase.intersects(NSEventPhase::Began | NSEventPhase::MayBegin) {
                set_zoom_gesture(None);
            }
            let gesture = zoom_gesture();
            if gesture.is_some() {
                window.clone_from(&gesture);
            }
            let Some(window) = window else { return false };
            if !command && !same_window(gesture.as_deref(), Some(&window)) {
                return false;
            }
            if !Self::zooms(&window) || !zoom_enabled() {
                return false;
            }
            if !wheel {
                set_zoom_gesture(Some(&window));
            }
            if momentum.intersects(NSEventPhase::Ended | NSEventPhase::Cancelled) {
                set_zoom_gesture(None);
            }
            let dy = event.scrollingDeltaY();
            if dy == 0.0 {
                return true;
            }
            let precise = event.hasPreciseScrollingDeltas();
            Self::for_window(&window).zoom_by(
                zoom_math::scroll_factor(dy, precise),
                Self::screen_point(event),
                !precise,
            );
            return true;
        }
        let Some(window) = window else { return false };
        if !Self::zooms(&window) || !zoom_enabled() {
            return false;
        }
        let zoom = Self::for_window(&window);
        if event.phase().contains(NSEventPhase::Began) {
            zoom.pinch_began();
        }
        zoom.pinch_by(
            zoom_math::magnify_factor(event.magnification()),
            Self::screen_point(event),
        );
        if event
            .phase()
            .intersects(NSEventPhase::Ended | NSEventPhase::Cancelled)
        {
            zoom.pinch_ended();
        }
        true
    }

    /// `+devPinch:args:`: a pinch by `total` in `steps` steps at `hz`, from the centre of the window.
    pub fn dev_pinch(window: &NSWindow, args: &[String]) {
        if args.len() != 3 || !Self::zooms(window) {
            return;
        }
        let total = number_arg(&args[0]).doubleValue();
        let steps = number_arg(&args[1]).intValue().max(1);
        let hz = number_arg(&args[2]).doubleValue().max(1.0);
        let zoom = Self::for_window(window);
        zoom.ivars().applied_at.borrow_mut().clear();
        let frame = window.frame();
        let centre = NSPoint::new(mid_x(frame), mid_y(frame));
        let factor = total.powf(1.0 / f64::from(steps));
        let mut left = steps;
        zoom.pinch_began();
        let window = window.retain();
        repeat(1.0 / hz, move || {
            if !window.isVisible() {
                return false;
            }
            let more = left > 0;
            left -= 1;
            if !more {
                zoom.pinch_ended();
                return false;
            }
            zoom.pinch_by(factor, centre);
            true
        });
    }

    /// `+devPinchStats:`.
    pub fn dev_pinch_stats(window: &NSWindow) -> Value {
        let zoom = Self::for_window(window);
        let state = zoom.ivars();
        let applied = state.applied_at.borrow();
        let mut gaps: Vec<f64> = applied
            .windows(2)
            .map(|pair| (pair[1] - pair[0]) * 1000.0)
            .collect();
        gaps.sort_by(f64::total_cmp);
        let at = |q: f64| {
            if gaps.is_empty() {
                return 0.0;
            }
            let index = (gaps.len() - 1).min((q * (gaps.len() - 1) as f64) as usize);
            (gaps[index] * 100.0).round() / 100.0
        };
        let stand_in = state.stand_in.borrow().is_some();
        json!({
            "fps": window.screen().map_or(0, |screen| screen.maximumFramesPerSecond()),
            "applied": applied.len(),
            "gapMsMedian": at(0.5),
            "gapMsP90": at(0.9),
            "gapMsMax": at(1.0),
            "standIn": stand_in,
            "alpha": window.alphaValue(),
            "restingAlpha": if stand_in { state.hidden_alpha.get() } else { window.alphaValue() },
            "frame": rect_string(window.frame()),
            "shown": rect_string(state.shown.get()),
        })
    }

    /// `+devStandInWindow:`: the window number of the stand-in, or "" when none shows.
    pub fn dev_stand_in_window(window: &NSWindow) -> String {
        let zoom = Self::for_window(window);
        let stand_in = zoom.ivars().stand_in.borrow();
        stand_in
            .as_ref()
            .map_or_else(String::new, |panel| panel.windowNumber().to_string())
    }

    /// `+beginEdgeResize:event:`: starts an edge resize that the stand-in shows; true when it started.
    pub fn begin_edge_resize(window: &NSWindow, event: &NSEvent) -> bool {
        let Some(controller) = Controller::for_window(window) else {
            return false;
        };
        if controller.stashed_edge() != Edge::None
            || event
                .modifierFlags()
                .contains(NSEventModifierFlags::Control)
            || !window.isVisible()
            || !resizable(window)
        {
            return false;
        }
        let Some(direction) = resize_direction(window, event.locationInWindow()) else {
            return false;
        };
        if !(0..=7).contains(&direction) {
            return false;
        }
        let zoom = Self::for_window(window);
        zoom.pinch_began();
        let state = zoom.ivars();
        if state.stand_in.borrow().is_none() {
            return false;
        }
        state.follow.set(0.0);
        state.stand_in_pinch.set(false);
        state.resize_start.set(state.pinch_target.get());
        state.resize_from.set(pointer_of(window.mtm(), event));
        state.resize_direction.set(direction);
        set_edge_resizing(Some(&zoom));
        controller.resize_started();
        true
    }

    /// `+continueEdgeResize:`: follows the pointer during an edge resize; true when the event is used.
    pub fn continue_edge_resize(event: &NSEvent) -> bool {
        let Some(zoom) = edge_resizing() else {
            return false;
        };
        if event.r#type() == NSEventType::LeftMouseDown {
            set_edge_resizing(None);
            zoom.pinch_ended();
            return false;
        }
        zoom.edge_resize_to(pointer_of(zoom.mtm(), event));
        if event.r#type() == NSEventType::LeftMouseUp {
            set_edge_resizing(None);
            zoom.pinch_ended();
        }
        true
    }

    fn window(&self) -> Option<Retained<NSWindow>> {
        self.ivars().window.load()
    }

    fn edge_resize_to(&self, pointer: NSPoint) {
        let state = self.ivars();
        let start = state.resize_start.get();
        let [sx, sy] = SIDES[state.resize_direction.get() as usize].map(f64::from);
        let aspect = width(start) / height(start).max(1.0);
        let from = state.resize_from.get();
        let by_x = sx * (pointer.x - from.x);
        let by_y = sy * (pointer.y - from.y) * aspect;
        let (low, high) =
            zoom_math::width_limits(aspect, self.min_size(), self.max_size(), self.visible());
        let growth = if sx != 0.0 && sy != 0.0 {
            zoom_math::corner_growth(by_x, by_y, aspect)
        } else if sx != 0.0 {
            by_x
        } else {
            by_y
        };
        let next_width = std_clamp(width(start) + growth, low, high).round();
        let next_height = (next_width / aspect).round();
        let x = if sx > 0.0 {
            min_x(start)
        } else if sx < 0.0 {
            max_x(start) - next_width
        } else {
            mid_x(start) - next_width / 2.0
        };
        let y = if sy > 0.0 {
            min_y(start)
        } else if sy < 0.0 {
            max_y(start) - next_height
        } else {
            mid_y(start) - next_height / 2.0
        };
        let visible = self.visible();
        let x = zoom_math::kept_within(
            x,
            next_width,
            min_x(start),
            width(start),
            visible.x,
            visible.max_x(),
        );
        let y = zoom_math::kept_within(
            y,
            next_height,
            min_y(start),
            height(start),
            visible.y,
            visible.max_y(),
        );
        let next = rect(x.round(), y.round(), next_width, next_height);
        if !zoom_math::finite(throw_rect(next)) {
            return;
        }
        state.width.set(next_width);
        state.aspect.set(aspect);
        state.pinch_target.set(next);
    }

    fn pinch_began(&self) {
        let state = self.ivars();
        state.follow.set(zoom_math::PINCH_FOLLOW);
        self.stop_ease();
        state.target.set(0.0);
        if state.stand_in.borrow().is_some() && state.handing_off.get() {
            self.gesture_ended(false);
            state.handing_off.set(false);
            *state.stand_in_stamp.borrow_mut() = begin_stamp();
            state.stand_in_pinch.set(true);
            return;
        }
        self.finish_hand_off(false);
        let Some(window) = self.window() else { return };
        let controller = Controller::for_window(&window);
        let content_layer: Option<Retained<AnyObject>> = window
            .contentView()
            // SAFETY: `layer` is an NSView getter of a CALayer or nil.
            .and_then(|content| unsafe { msg_send![&content, layer] });
        let source = content_layer.as_deref().and_then(layer_host_in);
        let host_class = AnyClass::get(c"CALayerHost");
        // SAFETY: `valueForKey:` of the private `contextId` property of CALayerHost, an NSNumber.
        let context: Option<Retained<AnyObject>> = source
            .as_deref()
            .and_then(|source| unsafe { msg_send![source, valueForKey: ns_string!("contextId")] });
        // SAFETY: `unsignedIntValue` is sent to an NSNumber.
        let context_id: u32 = context
            .as_deref()
            .map_or(0, |context| unsafe { msg_send![context, unsignedIntValue] });
        let parent = superlayer(source.as_deref());
        let grandparent = superlayer(parent.as_deref());
        let (Some(controller), Some(source), Some(host_class), Some(parent), Some(grandparent)) =
            (controller, source, host_class, parent, grandparent)
        else {
            return;
        };
        if context_id == 0 {
            return;
        }
        controller.stop_motion();
        let mtm = self.mtm();
        let frame = window.frame();
        let visible = screen_for(mtm, frame).map_or(NSRect::ZERO, |screen| screen.visibleFrame());
        let stage = objc2_foundation::NSUnionRect(visible, frame);
        let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
            NSPanel::alloc(mtm),
            stage,
            NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
            NSBackingStoreType::Buffered,
            false,
        );
        // SAFETY: the panel is kept in `stand_in` until it is ordered out.
        unsafe { panel.setReleasedWhenClosed(false) };
        panel.setOpaque(false);
        panel.setBackgroundColor(Some(&NSColor::clearColor()));
        panel.setHasShadow(false);
        panel.setIgnoresMouseEvents(true);
        panel.setLevel(window.level());
        panel.setCollectionBehavior(window.collectionBehavior());
        panel.setAnimationBehavior(NSWindowAnimationBehavior::None);
        let Some(content) = panel.contentView() else {
            return;
        };
        content.setWantsLayer(true);
        let radius = controller.corner_radius();
        let card = new_layer();
        let clip = new_layer();
        let shadow = NSColor::blackColor().CGColor();
        let rim = NSColor::colorWithWhite_alpha(1.0, RIM_ALPHA).CGColor();
        // SAFETY: CALayer setters with their CoreGraphics types; shadowOpacity is a float.
        unsafe {
            let _: () = msg_send![&card, setAnchorPoint: NSPoint::ZERO];
            let _: () = msg_send![&card, setShadowColor: &*shadow];
            let _: () = msg_send![&card, setShadowOpacity: 0.32f32];
            let _: () = msg_send![&card, setShadowRadius: 14.0f64];
            let _: () = msg_send![&card, setShadowOffset: NSSize::new(0.0, -6.0)];
            let _: () = msg_send![&clip, setAnchorPoint: NSPoint::ZERO];
            let _: () = msg_send![&clip, setCornerRadius: radius];
            let _: () = msg_send![&clip, setCornerCurve: kCACornerCurveCircular];
            let _: () = msg_send![&clip, setMasksToBounds: true];
            let _: () = msg_send![&clip, setBorderWidth: 1.0f64];
            let _: () = msg_send![&clip, setBorderColor: &*rim];
        }
        let outer = copy_geometry(&grandparent);
        let inner = copy_geometry(&parent);
        // SAFETY: CALayerHost is a CALayer subclass; `contextId` is its private property; geometry as in copy_geometry.
        let host: Retained<AnyObject> = unsafe {
            let host: Retained<AnyObject> = msg_send![host_class, new];
            let _: () =
                msg_send![&host, setValue: context.as_deref(), forKey: ns_string!("contextId")];
            let anchor: NSPoint = msg_send![&source, anchorPoint];
            let _: () = msg_send![&host, setAnchorPoint: anchor];
            let _: () = msg_send![&host, setBounds: bounds(Some(&source))];
            let position: NSPoint = msg_send![&source, position];
            let _: () = msg_send![&host, setPosition: position];
            host
        };
        add_sublayer(&inner, &host);
        add_sublayer(&outer, &inner);
        let mirror = new_layer();
        // SAFETY: CALayer setters with their CoreGraphics types.
        unsafe {
            let _: () = msg_send![&mirror, setAnchorPoint: NSPoint::ZERO];
            let _: () = msg_send![&mirror, setFrame: rect(0.0, 0.0, width(frame), height(frame))];
            let _: () = msg_send![&mirror, setZPosition: -1.0f64];
        }
        add_sublayer(&mirror, &outer);
        add_sublayer(&clip, &mirror);
        add_sublayer(&card, &clip);
        // SAFETY: `layer` is an NSView getter; the content view wants a layer, so it has one.
        let content_layer: Option<Retained<AnyObject>> = unsafe { msg_send![&content, layer] };
        if let Some(content_layer) = content_layer {
            add_sublayer(&content_layer, &card);
        }
        *state.stand_in.borrow_mut() = Some(panel.clone());
        *state.stand_in_stamp.borrow_mut() = begin_stamp();
        state.stand_in_pinch.set(true);
        *state.outer.borrow_mut() = Some(outer);
        *state.inner.borrow_mut() = Some(inner);
        *state.host.borrow_mut() = Some(host);
        *state.source.borrow_mut() = Weak::new(&source);
        state.handing_off.set(false);
        *state.card.borrow_mut() = Some(card);
        *state.clip.borrow_mut() = Some(clip);
        *state.mirror.borrow_mut() = Some(mirror);
        state.stage.set(stage);
        state.stand_in_width.set(width(frame));
        state.pinch_target.set(frame);
        state.shown.set(frame);
        self.place(frame);
        panel.orderWindow_relativeTo(NSWindowOrderingMode::Above, window.windowNumber());
        state.hidden_alpha.set(window.alphaValue());
        window.setAlphaValue(
            state
                .hidden_alpha
                .get()
                .min(zoom_math::HITTABLE_HIDDEN_ALPHA),
        );
        state.ticked_at.set(0.0);
        // SAFETY: `displayLinkWithTarget:selector:` returns a CADisplayLink that sends `tick:` to this zoom;
        // `addToRunLoop:forMode:` schedules it on the main run loop.
        let link: Retained<AnyObject> = unsafe {
            let link: Retained<AnyObject> =
                msg_send![&content, displayLinkWithTarget: self, selector: sel!(tick:)];
            let _: () = msg_send![&link, addToRunLoop: &*NSRunLoop::mainRunLoop(), forMode: NSRunLoopCommonModes];
            link
        };
        *state.link.borrow_mut() = Some(link);
    }

    fn place(&self, frame: NSRect) {
        if !zoom_math::finite(throw_rect(frame)) || width(frame) < 1.0 {
            return;
        }
        let state = self.ivars();
        let stage = state.stage.get();
        let local = rect(
            min_x(frame) - min_x(stage),
            min_y(frame) - min_y(stage),
            width(frame),
            height(frame),
        );
        let card = state.card.borrow().clone();
        let clip = state.clip.borrow().clone();
        let mirror = state.mirror.borrow().clone();
        let copies = [
            state.outer.borrow().clone(),
            state.inner.borrow().clone(),
            state.host.borrow().clone(),
        ];
        let source = state.source.borrow().load();
        without_actions(|| {
            // SAFETY: CALayer setters and getters with their CoreGraphics and CATransform3D types; messages to nil
            // layers are skipped, as ObjC skips them.
            unsafe {
                if let Some(card) = &card {
                    let _: () = msg_send![card, setFrame: local];
                }
                if let Some(clip) = &clip {
                    let _: () =
                        msg_send![clip, setFrame: rect(0.0, 0.0, width(local), height(local))];
                }
                let clip_bounds = bounds(clip.as_deref());
                let corner: f64 = clip
                    .as_deref()
                    .map_or(0.0, |clip| msg_send![clip, cornerRadius]);
                let outline = CGPathCreateWithRoundedRect(clip_bounds, corner, corner, ptr::null());
                if let Some(card) = &card {
                    let _: () = msg_send![card, setShadowPath: outline];
                }
                CGPathRelease(outline);
                let parent = superlayer(source.as_deref());
                let grandparent = superlayer(parent.as_deref());
                if let (Some(source), Some(parent), Some(grandparent)) =
                    (&source, &parent, &grandparent)
                {
                    for (copy, from) in copies.iter().zip([grandparent, parent, source]) {
                        let Some(copy) = copy else { continue };
                        let _: () = msg_send![copy, setBounds: bounds(Some(from))];
                        let position: NSPoint = msg_send![from, position];
                        let _: () = msg_send![copy, setPosition: position];
                        let transform: Transform3D = msg_send![from, transform];
                        let _: () = msg_send![copy, setTransform: transform];
                    }
                }
                let drawn_width = width(bounds(grandparent.as_deref()));
                let drawn = if drawn_width > 1.0 {
                    drawn_width
                } else {
                    state.stand_in_width.get()
                };
                let scale = width(local) / drawn;
                if let Some(mirror) = &mirror {
                    let _: () =
                        msg_send![mirror, setTransform: Transform3D::scale(scale, scale, 1.0)];
                }
            }
        });
    }

    fn tick_with(&self, link: &AnyObject) {
        let state = self.ivars();
        if !self.window().is_some_and(|window| window.isVisible()) {
            return self.cancel_stand_in();
        }
        // SAFETY: `timestamp` is a CADisplayLink getter of a CFTimeInterval.
        let now: f64 = unsafe { msg_send![link, timestamp] };
        let ticked_at = state.ticked_at.get();
        let dt = if ticked_at > 0.0 {
            (now - ticked_at).min(0.05)
        } else {
            1.0 / 120.0
        };
        state.ticked_at.set(now);
        let shown = state.shown.get();
        let target = state.pinch_target.get();
        if state.handing_off.get() {
            self.place(shown);
            let source = state.source.borrow().load();
            let drawn = width(bounds(
                superlayer(superlayer(source.as_deref()).as_deref()).as_deref(),
            ));
            let since = now - state.released_at.get();
            let caught_up = (drawn - width(target)).abs() < 1.0;
            if (caught_up && since > zoom_math::HAND_OFF_LINGER)
                || since > zoom_math::HAND_OFF_TIMEOUT
            {
                self.finish_hand_off(!caught_up);
            }
            return;
        }
        if equal_rects(shown, target) {
            return;
        }
        let follow = state.follow.get();
        let follow = if follow > 0.0 {
            1.0 - (-dt / follow).exp()
        } else {
            1.0
        };
        let mut next = rect(
            min_x(shown) + (min_x(target) - min_x(shown)) * follow,
            min_y(shown) + (min_y(target) - min_y(shown)) * follow,
            width(shown) + (width(target) - width(shown)) * follow,
            height(shown) + (height(target) - height(shown)) * follow,
        );
        if (width(next) - width(target)).abs() < 0.5
            && (min_x(next) - min_x(target)).abs() < 0.5
            && (min_y(next) - min_y(target)).abs() < 0.5
        {
            next = target;
        }
        state.shown.set(next);
        self.place(next);
        let mut applied = state.applied_at.borrow_mut();
        if applied.len() < 4096 {
            applied.push(media_time());
        }
    }

    fn pinch_by(&self, factor: f64, pointer: NSPoint) {
        let state = self.ivars();
        if state.stand_in.borrow().is_none() {
            return self.zoom_by(factor, pointer, false);
        }
        let frame = state.pinch_target.get();
        let live = self.window().map_or(NSRect::ZERO, |window| window.frame());
        let aspect = state.aspect.get();
        if state.width.get() <= 0.0
            || (state.width.get() - width(frame)).abs() > 1.0
            || aspect <= 0.0
            || !aspect.is_finite()
            || (width(live) / aspect - height(live)).abs() > 1.0
        {
            state.aspect.set(width(live) / height(live).max(1.0));
            state.width.set(width(frame));
        }
        state.pointer.set(pointer);
        let result = zoom_math::zoom(
            throw_rect(frame),
            state.width.get(),
            state.aspect.get(),
            factor,
            Point {
                x: pointer.x,
                y: pointer.y,
            },
            self.visible(),
            self.min_size(),
            self.max_size(),
        );
        if !zoom_math::finite(result.frame) {
            return;
        }
        state.width.set(result.width);
        state.pinch_target.set(rect(
            result.frame.x,
            result.frame.y,
            result.frame.w,
            result.frame.h,
        ));
    }

    fn pinch_ended(&self) {
        let state = self.ivars();
        if state.stand_in.borrow().is_none() || state.handing_off.get() {
            return;
        }
        let target = state.pinch_target.get();
        state.shown.set(target);
        self.place(target);
        let window = self.window();
        if let Some(window) = &window
            && !equal_rects(window.frame(), target)
        {
            window.setFrame_display(target, true);
        }
        state
            .last
            .set(window.map_or(NSRect::ZERO, |window| window.frame()));
        state.width.set(width(target));
        state.handing_off.set(true);
        let ticked_at = state.ticked_at.get();
        state.released_at.set(if ticked_at > 0.0 {
            ticked_at
        } else {
            media_time()
        });
    }

    fn gesture_ended(&self, timed_out: bool) {
        let state = self.ivars();
        let stamp = state.stand_in_stamp.replace(Value::Null);
        if stamp.is_null() {
            return;
        }
        let view = self
            .window()
            .and_then(|window| Controller::for_window(&window))
            .and_then(|controller| controller.view());
        if let Some(view) = view {
            view.emit(
                "pictureInPictureStandIn",
                stamp_onto(
                    &stamp,
                    json!({"pinch": state.stand_in_pinch.get(), "timedOut": timed_out}),
                ),
            );
        }
    }

    fn cancel_stand_in(&self) {
        let state = self.ivars();
        *state.stand_in_stamp.borrow_mut() = Value::Null;
        if edge_resizing().is_some_and(|zoom| ptr::eq(&*zoom, self)) {
            set_edge_resizing(None);
        }
        state.handing_off.set(false);
        self.finish_hand_off(false);
    }

    fn finish_hand_off(&self, timed_out: bool) {
        let state = self.ivars();
        let Some(panel) = state.stand_in.borrow().clone() else {
            return;
        };
        if state.handing_off.get() {
            self.gesture_ended(timed_out);
        }
        *state.stand_in_stamp.borrow_mut() = Value::Null;
        *state.stand_in.borrow_mut() = None;
        state.handing_off.set(false);
        if let Some(link) = state.link.take() {
            // SAFETY: `invalidate` removes the CADisplayLink from its run loops.
            let _: () = unsafe { msg_send![&link, invalidate] };
        }
        without_actions(|| {
            if let Some(window) = self.window() {
                window.setAlphaValue(state.hidden_alpha.get());
            }
            panel.orderOut(None);
        });
        *state.card.borrow_mut() = None;
        *state.clip.borrow_mut() = None;
        *state.mirror.borrow_mut() = None;
        *state.outer.borrow_mut() = None;
        *state.inner.borrow_mut() = None;
        *state.host.borrow_mut() = None;
    }

    fn screen_point(event: &NSEvent) -> NSPoint {
        match event.window(main_thread()) {
            Some(window) => window.convertPointToScreen(event.locationInWindow()),
            None => event.locationInWindow(),
        }
    }

    /// `[_ease invalidate], _ease = nil`.
    fn stop_ease(&self) {
        let state = self.ivars();
        state.ease.set(false);
        state
            .ease_generation
            .set(state.ease_generation.get().wrapping_add(1));
    }

    fn zoom_by(&self, factor: f64, pointer: NSPoint, ease: bool) {
        let Some(window) = self.window() else { return };
        let state = self.ivars();
        if let Some(controller) = Controller::for_window(&window) {
            controller.stop_motion();
        }
        let frame = window.frame();
        if !equal_rects(frame, state.last.get()) || state.width.get() <= 0.0 {
            let aspect = width(frame) / height(frame).max(1.0);
            let known = state.aspect.get();
            if !(known > 0.0 && (width(frame) / known - height(frame)).abs() <= 1.0) {
                state.aspect.set(aspect);
            }
            state.width.set(width(frame));
            state.target.set(0.0);
            self.stop_ease();
        }
        state.pointer.set(pointer);
        if !ease {
            self.stop_ease();
            state.target.set(0.0);
            return self.step(factor);
        }
        let (low, high) = zoom_math::width_limits(
            state.aspect.get(),
            self.min_size(),
            self.max_size(),
            self.visible(),
        );
        let from = if state.target.get() > 0.0 {
            state.target.get()
        } else {
            state.width.get()
        };
        state.target.set((from * factor).max(low).min(high));
        if state.ease.get() {
            return;
        }
        state.ease.set(true);
        let generation = state.ease_generation.get();
        let me = Weak::new(self);
        repeat(1.0 / 120.0, move || {
            let Some(zoom) = me.load() else { return false };
            let state = zoom.ivars();
            if state.ease_generation.get() != generation {
                return false;
            }
            let window = zoom.window();
            if !window.as_ref().is_some_and(|window| {
                window.isVisible() && equal_rects(window.frame(), state.last.get())
            }) {
                state.ease.set(false);
                state.target.set(0.0);
                return false;
            }
            let (width, target) = (state.width.get(), state.target.get());
            let done = (target - width).abs() < 0.25;
            zoom.step(
                if done {
                    target
                } else {
                    width + (target - width) * 0.25
                } / width,
            );
            if !done {
                return true;
            }
            state.ease.set(false);
            state.target.set(0.0);
            false
        });
        state.last.set(frame);
    }

    fn min_size(&self) -> zoom_math::Size {
        let size = self
            .window()
            .map_or(NSSize::ZERO, |window| window.contentMinSize());
        zoom_math::Size {
            w: size.width,
            h: size.height,
        }
    }

    fn max_size(&self) -> zoom_math::Size {
        let size = self
            .window()
            .map_or(NSSize::ZERO, |window| window.contentMaxSize());
        zoom_math::Size {
            w: if size.width > 0.0 {
                size.width
            } else {
                f64::MAX
            },
            h: if size.height > 0.0 {
                size.height
            } else {
                f64::MAX
            },
        }
    }

    fn visible(&self) -> super::ardia_pip_throw_math::Rect {
        let frame = self.window().map_or(NSRect::ZERO, |window| window.frame());
        throw_rect(
            screen_for(self.mtm(), frame).map_or(NSRect::ZERO, |screen| screen.visibleFrame()),
        )
    }

    fn step(&self, factor: f64) {
        let Some(window) = self.window() else { return };
        let state = self.ivars();
        let frame = window.frame();
        let pointer = state.pointer.get();
        let result = zoom_math::zoom(
            throw_rect(frame),
            state.width.get(),
            state.aspect.get(),
            factor,
            Point {
                x: pointer.x,
                y: pointer.y,
            },
            self.visible(),
            self.min_size(),
            self.max_size(),
        );
        state.width.set(result.width);
        let next = rect(
            result.frame.x,
            result.frame.y,
            result.frame.w,
            result.frame.h,
        );
        if !equal_rects(next, frame) {
            window.setFrame_display(next, true);
        }
        state.last.set(window.frame());
    }
}
