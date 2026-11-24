// Ported from ardia packages/ardiacore/ios/ArdiaPictureInPictureInternal.h at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.
// Companion definitions: packages/ardiacore/ios/ArdiaPiPController.mm (helpers and the constants of the header).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;
use std::rc::Rc;

use objc2::MainThreadMarker;
use objc2::ffi;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Imp, Sel};
use objc2::{msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSEvent, NSScreen, NSView, NSWindow, NSWindowCollectionBehavior,
    NSWindowStyleMask,
};
use objc2_foundation::{NSObjectProtocol, NSPoint, NSRect, NSSize, NSUserDefaults};
use serde_json::Value;

use super::ardia_pip_throw_math as throw;
use crate::core::bindings::Window;
use crate::launch::launch_config::LaunchConfig;

pub const PEEK: f64 = 28.0;
pub const RETURN_MARGIN: f64 = 16.0;
pub const BUTTON_MARGIN: f64 = 12.0;
pub const BUTTON_SIZE: f64 = 28.0;
pub const CORNER_BEND_DURATION: f64 = 0.2;
pub const RIM_ALPHA: f64 = 0.17;

pub const ZOOM_DEFAULT: &str = "ACPictureInPictureZoom";
const THROW_DEFAULT: &str = "ACPictureInPictureThrow";

/// The side of the screen where the window is tucked. The numbers are the ones that Ardia reports.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Edge {
    #[default]
    None = 0,
    Left = -1,
    Right = 1,
}

/// What the Picture in Picture code needs from the page that owns the window (`ArdiaWebView`).
pub trait PipPage {
    fn browser_id(&self) -> i64;
    fn emit(&self, name: &str, payload: Value);
    fn exit_picture_in_picture(&self);
    fn seek_picture_in_picture_by(&self, seconds: f64);
    fn left_app_in_picture_in_picture(&self) -> bool;
    fn seen_by_user(&self) -> bool;
}

pub type StampOnto = dyn Fn(&Value, Value) -> Value;
pub type RootViewOf = dyn Fn(&NSWindow) -> Option<Retained<NSView>>;
pub type NotePointer = dyn Fn(&NSEvent, &str);

/// What the Picture in Picture code needs from the rest of the app.
pub struct Services {
    pub mtm: MainThreadMarker,
    pub launch: Rc<LaunchConfig>,
    /// `ArdiaConsentStamp.begin`.
    pub begin_stamp: Box<dyn Fn() -> Value>,
    /// `[stamp onto:payload]`.
    pub onto: Box<StampOnto>,
    /// `ArdiaWindowRootView`: the root view of the window, or its content view.
    pub root_view: Box<RootViewOf>,
    /// `[ArdiaWindowController reconsiderSightOf:]`.
    pub reconsider_sight: Box<dyn Fn(&NSWindow)>,
    /// `ardia_host::NotePointer`.
    pub note_pointer: Box<NotePointer>,
    /// `ardia_host::LogActivation`.
    pub log_activation: Box<dyn Fn(&str)>,
}

thread_local! {
    static SERVICES: RefCell<Option<Rc<Services>>> = const { RefCell::new(None) };
    static ACTIVATIONS: Cell<i64> = const { Cell::new(0) };
}

pub fn set_services(services: Services) {
    SERVICES.with(|slot| *slot.borrow_mut() = Some(Rc::new(services)));
}

pub fn services() -> Option<Rc<Services>> {
    SERVICES.with(|slot| slot.borrow().clone())
}

pub fn background() -> bool {
    services().is_some_and(|services| services.launch.background)
}

pub fn begin_stamp() -> Value {
    services().map_or(Value::Null, |services| (services.begin_stamp)())
}

pub fn stamp_onto(stamp: &Value, payload: Value) -> Value {
    match services() {
        Some(services) => (services.onto)(stamp, payload),
        None => payload,
    }
}

pub fn window_root_view(window: &NSWindow) -> Option<Retained<NSView>> {
    let services = services()?;
    (services.root_view)(window).or_else(|| window.contentView())
}

pub fn activations() -> i64 {
    ACTIVATIONS.with(Cell::get)
}

pub fn count_activation() {
    ACTIVATIONS.with(|count| count.set(count.get() + 1));
}

pub fn rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))
}

pub fn min_x(r: NSRect) -> f64 {
    r.origin.x
}
pub fn min_y(r: NSRect) -> f64 {
    r.origin.y
}
pub fn max_x(r: NSRect) -> f64 {
    r.origin.x + r.size.width
}
pub fn max_y(r: NSRect) -> f64 {
    r.origin.y + r.size.height
}
pub fn mid_x(r: NSRect) -> f64 {
    r.origin.x + r.size.width / 2.0
}
pub fn mid_y(r: NSRect) -> f64 {
    r.origin.y + r.size.height / 2.0
}
pub fn width(r: NSRect) -> f64 {
    r.size.width
}
pub fn height(r: NSRect) -> f64 {
    r.size.height
}

pub fn point_in_rect(p: NSPoint, r: NSRect) -> bool {
    p.x >= min_x(r) && p.x < max_x(r) && p.y >= min_y(r) && p.y < max_y(r)
}

pub fn equal_rects(a: NSRect, b: NSRect) -> bool {
    a.origin.x == b.origin.x
        && a.origin.y == b.origin.y
        && a.size.width == b.size.width
        && a.size.height == b.size.height
}

pub fn equal_points(a: NSPoint, b: NSPoint) -> bool {
    a.x == b.x && a.y == b.y
}

pub fn is_empty_rect(r: NSRect) -> bool {
    !(r.size.width > 0.0 && r.size.height > 0.0)
}

fn intersection_area(a: NSRect, b: NSRect) -> f64 {
    let wide = max_x(a).min(max_x(b)) - min_x(a).max(min_x(b));
    let tall = max_y(a).min(max_y(b)) - min_y(a).max(min_y(b));
    if wide > 0.0 && tall > 0.0 {
        wide * tall
    } else {
        0.0
    }
}

pub fn rect_string(r: NSRect) -> String {
    objc2_foundation::NSString::from_rect(r).to_string()
}

pub fn point_string(p: NSPoint) -> String {
    objc2_foundation::NSString::from_point(p).to_string()
}

pub fn size_string(s: NSSize) -> String {
    objc2_foundation::NSString::from_size(s).to_string()
}

pub fn user_default_bool(key: &str, otherwise: bool) -> bool {
    let defaults = NSUserDefaults::standardUserDefaults();
    // SAFETY: `objectForKey:` returns an object or nil; `boolValue` is sent only to an NSNumber or NSString.
    unsafe {
        let value: Option<Retained<AnyObject>> =
            msg_send![&defaults, objectForKey: &*objc2_foundation::NSString::from_str(key)];
        value.map_or(otherwise, |value| msg_send![&value, boolValue])
    }
}

pub fn throw_enabled() -> bool {
    user_default_bool(THROW_DEFAULT, true)
}

pub fn set_user_default_bool(key: &str, value: bool) {
    NSUserDefaults::standardUserDefaults()
        .setBool_forKey(value, &objc2_foundation::NSString::from_str(key));
}

pub fn zoom_enabled() -> bool {
    user_default_bool(ZOOM_DEFAULT, true)
}

pub fn screens(mtm: MainThreadMarker) -> Vec<Retained<NSScreen>> {
    NSScreen::screens(mtm).iter().collect()
}

pub fn overlapping_screen(mtm: MainThreadMarker, frame: NSRect) -> Option<Retained<NSScreen>> {
    let mut best = None;
    let mut best_area = 0.0;
    for screen in screens(mtm) {
        let area = intersection_area(frame, screen.frame());
        if area > best_area {
            best_area = area;
            best = Some(screen);
        }
    }
    best
}

pub fn screen_for(mtm: MainThreadMarker, frame: NSRect) -> Option<Retained<NSScreen>> {
    overlapping_screen(mtm, frame).or_else(|| NSScreen::mainScreen(mtm))
}

pub fn screen_at(mtm: MainThreadMarker, x: f64, y: f64) -> bool {
    screens(mtm)
        .iter()
        .any(|screen| point_in_rect(NSPoint::new(x, y), screen.frame()))
}

/// The pointer in global bottom-left coordinates, from the Core Graphics location of the event.
pub fn pointer_of(mtm: MainThreadMarker, event: &NSEvent) -> NSPoint {
    let top = screens(mtm)
        .first()
        .map_or(0.0, |screen| max_y(screen.frame()));
    let point = event.CGEvent().map_or(NSPoint::ZERO, |cg| {
        // SAFETY: `CGEvent` returns a live CGEventRef owned by the NSEvent.
        unsafe { cg_event_location(Retained::as_ptr(&cg).cast_mut().cast()) }
    });
    NSPoint::new(point.x, top - point.y)
}

unsafe extern "C" {
    #[link_name = "CGEventGetLocation"]
    fn cg_event_location(event: *mut c_void) -> NSPoint;
}

pub fn throw_rect(r: NSRect) -> throw::Rect {
    throw::Rect {
        x: min_x(r),
        y: min_y(r),
        w: width(r),
        h: height(r),
    }
}

pub fn from_throw_rect(r: throw::Rect) -> NSRect {
    rect(r.x, r.y, r.w, r.h)
}

pub fn screen_index(mtm: MainThreadMarker, screen: Option<&NSScreen>) -> Option<usize> {
    let screen = screen?;
    screens(mtm)
        .iter()
        .position(|candidate| std::ptr::eq(&**candidate, screen))
}

pub fn throw_screens(mtm: MainThreadMarker) -> Vec<throw::Screen> {
    screens(mtm)
        .iter()
        .map(|screen| throw::Screen {
            frame: throw_rect(screen.frame()),
            visible: throw_rect(screen.visibleFrame()),
        })
        .collect()
}

fn shows_page(view: &NSView) -> bool {
    if let Some(page) = AnyClass::get(c"RenderWidgetHostViewCocoa")
        && view.isKindOfClass(page)
    {
        return true;
    }
    view.subviews().iter().any(|child| shows_page(&child))
}

pub fn is_chrome_floating_window(window: &NSWindow) -> bool {
    Window::is_chrome_frameless_window(window)
        && window.isVisible()
        && window.level() >= objc2_app_kit::NSFloatingWindowLevel
        && window
            .collectionBehavior()
            .contains(NSWindowCollectionBehavior::CanJoinAllSpaces)
}

pub fn is_chrome_video_picture_in_picture(window: &NSWindow) -> bool {
    is_chrome_floating_window(window)
        && !window
            .contentView()
            .is_some_and(|content| shows_page(&content))
}

pub fn is_chrome_document_picture_in_picture(window: &NSWindow) -> bool {
    Window::is_chrome_frameless_window(window)
        && window
            .contentView()
            .is_some_and(|content| shows_page(&content))
}

pub fn responds_to(window: &NSWindow, selector: Sel) -> bool {
    // SAFETY: `respondsToSelector:` is sent to a live NSObject.
    unsafe { msg_send![window, respondsToSelector: selector] }
}

pub fn prevent_activation(window: &NSWindow) {
    if responds_to(window, sel!(_setPreventsActivation:)) {
        // SAFETY: the private setter takes one BOOL, as in Ardia's category.
        unsafe { msg_send![window, _setPreventsActivation: true] }
    }
}

pub fn prevents_activation(window: &NSWindow) -> bool {
    responds_to(window, sel!(_preventsActivation)) && {
        // SAFETY: the private getter returns a BOOL.
        unsafe { msg_send![window, _preventsActivation] }
    }
}

/// The edge-or-corner under the point, or -1 when the point is inside the window.
pub fn resize_direction(window: &NSWindow, point: NSPoint) -> Option<isize> {
    responds_to(window, sel!(_resizeDirectionForMouseLocation:)).then(|| {
        // SAFETY: the private method takes a point and returns an NSInteger.
        unsafe { msg_send![window, _resizeDirectionForMouseLocation: point] }
    })
}

pub fn resizable(window: &NSWindow) -> bool {
    window.styleMask().contains(NSWindowStyleMask::Resizable)
}

/// Replaces `selector` of `class` and keeps the implementation that it had, as `ardia_runtime::ReplaceMethod` does.
/// If `class` only inherits the method, the replacement is added to `class` and its superclass stays as it was.
pub fn replace_method(class: &AnyClass, selector: Sel, replacement: Imp) -> bool {
    let Some(method) = class.instance_method(selector) else {
        return false;
    };
    let original = method.implementation();
    // SAFETY: the method is live; the encoding string lives as long as the method.
    let encoding = unsafe { ffi::method_getTypeEncoding(std::ptr::from_ref(method)) };
    // SAFETY: `replacement` has the signature of `selector`; `class_addMethod` copies the encoding.
    let added = unsafe {
        ffi::class_addMethod(
            std::ptr::from_ref(class).cast_mut().cast(),
            selector,
            replacement,
            encoding,
        )
    };
    if !added.as_bool() {
        // SAFETY: the method belongs to `class` and `replacement` has its signature.
        unsafe { method.set_implementation(replacement) };
    }
    ORIGINALS.with(|map| {
        map.borrow_mut()
            .insert((std::ptr::from_ref(class), selector), original)
    });
    true
}

thread_local! {
    static ORIGINALS: RefCell<HashMap<(*const AnyClass, Sel), Imp>> = RefCell::new(HashMap::new());
}

/// The implementation that `replace_method` replaced, found from the class of `object` or its superclasses.
pub fn original_of(object: &AnyObject, selector: Sel) -> Option<Imp> {
    let mut class = Some(object.class());
    ORIGINALS.with(|map| {
        let map = map.borrow();
        while let Some(current) = class {
            if let Some(original) = map.get(&(std::ptr::from_ref(current), selector)) {
                return Some(*original);
            }
            class = current.superclass();
        }
        None
    })
}

/// A delay that a newer request or `cancel` replaces: `performSelector:afterDelay:` with `cancelPreviousPerformRequests`.
#[derive(Default)]
pub struct Delay(Rc<Cell<u64>>);

impl Delay {
    pub fn schedule(&self, seconds: f64, work: impl FnOnce() + 'static) {
        let generation = self.0.get().wrapping_add(1);
        self.0.set(generation);
        let current = Rc::clone(&self.0);
        ria_engine::call::main_queue::after(
            std::time::Duration::from_secs_f64(seconds),
            Box::new(move || {
                if current.get() == generation {
                    current.set(generation.wrapping_add(1));
                    work();
                }
            }),
        );
    }

    pub fn cancel(&self) {
        self.0.set(self.0.get().wrapping_add(1));
    }
}

pub fn after(seconds: f64, work: impl FnOnce() + 'static) {
    ria_engine::call::main_queue::after(
        std::time::Duration::from_secs_f64(seconds),
        Box::new(work),
    );
}

pub fn shared_application(mtm: MainThreadMarker) -> Retained<NSApplication> {
    NSApplication::sharedApplication(mtm)
}
