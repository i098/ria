// Ported from ardia packages/ardiacore/ios/ArdiaPiPResizeCursor.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.
// Also ported here: packages/ardiacore/ios/ArdiaPiPController+ResizeCursor.mm.

//! Shows the resize cursor at the edges of a Picture in Picture window, also when the app is in the background.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::ffi::{CStr, c_void};
use std::ptr::{self, NonNull};
use std::rc::Rc;

use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyClass, AnyObject, Sel};
use objc2::{
    AnyThread, ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, ffi,
    msg_send, sel,
};
use objc2_app_kit::{
    NSApplication, NSCursor, NSCursorFrameResizeDirections, NSCursorFrameResizePosition, NSEvent,
    NSTrackingArea, NSTrackingAreaOptions, NSView, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{
    NSObject, NSOperatingSystemVersion, NSPoint, NSProcessInfo, NSRect, ns_string,
};
use serde_json::{Value, json};

use super::ardia_picture_in_picture_internal::{
    Edge, begin_stamp, height, point_in_rect, rect, replace_method, resizable, resize_direction,
    responds_to, size_string, stamp_onto, width,
};
use super::ardia_pip_controller::Controller;

/// The values of `_resizeDirectionForMouseLocation:`, in its order.
#[derive(Clone, Copy)]
enum ResizePosition {
    Right,
    TopRight,
    Top,
    TopLeft,
    Left,
    BottomLeft,
    Bottom,
    BottomRight,
}

const POSITIONS: [ResizePosition; 8] = [
    ResizePosition::Right,
    ResizePosition::TopRight,
    ResizePosition::Top,
    ResizePosition::TopLeft,
    ResizePosition::Left,
    ResizePosition::BottomLeft,
    ResizePosition::Bottom,
    ResizePosition::BottomRight,
];

fn resizes(window: &NSWindow) -> bool {
    Controller::for_window(window).is_some_and(|controller| controller.stashed_edge() == Edge::None)
        && window.isVisible()
        && resizable(window)
        && !window.styleMask().contains(NSWindowStyleMask::FullScreen)
        && responds_to(window, sel!(_resizeDirectionForMouseLocation:))
}

fn position_at(window: &NSWindow, point: NSPoint) -> Option<ResizePosition> {
    if !resizes(window) {
        return None;
    }
    let direction = resize_direction(window, point)?;
    usize::try_from(direction)
        .ok()
        .and_then(|index| POSITIONS.get(index).copied())
}

fn cursor_for(position: ResizePosition) -> Retained<NSCursor> {
    let frame = match position {
        ResizePosition::Top => NSCursorFrameResizePosition::Top,
        ResizePosition::Left => NSCursorFrameResizePosition::Left,
        ResizePosition::Bottom => NSCursorFrameResizePosition::Bottom,
        ResizePosition::Right => NSCursorFrameResizePosition::Right,
        ResizePosition::TopLeft => NSCursorFrameResizePosition::TopLeft,
        ResizePosition::TopRight => NSCursorFrameResizePosition::TopRight,
        ResizePosition::BottomLeft => NSCursorFrameResizePosition::BottomLeft,
        ResizePosition::BottomRight => NSCursorFrameResizePosition::BottomRight,
    };
    NSCursor::frameResizeCursorFromPosition_inDirections(frame, NSCursorFrameResizeDirections::All)
}

fn app_kit_cursor(name: &CStr, fallback: Retained<NSCursor>) -> Retained<NSCursor> {
    let selector = Sel::register(name);
    let class = NSCursor::class();
    // SAFETY: `respondsToSelector:` is sent to the NSCursor class.
    let responds: bool = unsafe { msg_send![class, respondsToSelector: selector] };
    if !responds {
        return fallback;
    }
    // SAFETY: the class answers the selector, which takes no arguments and returns an object.
    let cursor: Option<Retained<AnyObject>> =
        unsafe { msg_send![class, performSelector: selector] };
    cursor
        .and_then(|cursor| cursor.downcast::<NSCursor>().ok())
        .unwrap_or(fallback)
}

// The cursors before macOS 15, as in Ardia.
#[allow(deprecated)]
fn legacy_cursor_for(position: ResizePosition) -> Retained<NSCursor> {
    match position {
        ResizePosition::Top | ResizePosition::Bottom => NSCursor::resizeUpDownCursor(),
        ResizePosition::Left | ResizePosition::Right => NSCursor::resizeLeftRightCursor(),
        ResizePosition::TopLeft | ResizePosition::BottomRight => app_kit_cursor(
            c"_windowResizeNorthWestSouthEastCursor",
            NSCursor::crosshairCursor(),
        ),
        ResizePosition::TopRight | ResizePosition::BottomLeft => app_kit_cursor(
            c"_windowResizeNorthEastSouthWestCursor",
            NSCursor::crosshairCursor(),
        ),
    }
}

fn position_name(position: ResizePosition) -> &'static str {
    match position {
        ResizePosition::Top => "top",
        ResizePosition::Left => "left",
        ResizePosition::Bottom => "bottom",
        ResizePosition::Right => "right",
        ResizePosition::TopLeft => "topLeft",
        ResizePosition::TopRight => "topRight",
        ResizePosition::BottomLeft => "bottomLeft",
        ResizePosition::BottomRight => "bottomRight",
    }
}

fn resize_cursor_at(window: &NSWindow, point: NSPoint) -> Option<Retained<NSCursor>> {
    let position = position_at(window, point)?;
    let macos_15 =
        NSProcessInfo::processInfo().isOperatingSystemAtLeastVersion(NSOperatingSystemVersion {
            majorVersion: 15,
            minorVersion: 0,
            patchVersion: 0,
        });
    Some(if macos_15 {
        cursor_for(position)
    } else {
        legacy_cursor_for(position)
    })
}

thread_local! {
    static BACKGROUND_CURSOR_RESULT: Cell<i32> = const { Cell::new(-100) };
    static CURSOR_UPDATE_CLASSES: RefCell<HashSet<usize>> = RefCell::default();
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFBooleanTrue: *const c_void;
    static kCFBooleanFalse: *const c_void;
}

fn set_cursor_in_background(allowed: bool) {
    type DefaultConnection = unsafe extern "C" fn() -> i32;
    type SetConnectionProperty =
        unsafe extern "C" fn(i32, i32, *const c_void, *const c_void) -> i32;
    // SAFETY: dlsym looks up the symbols by NUL-terminated name in the loaded images.
    let (connection, set) = unsafe {
        (
            libc::dlsym(libc::RTLD_DEFAULT, c"_CGSDefaultConnection".as_ptr()),
            libc::dlsym(libc::RTLD_DEFAULT, c"CGSSetConnectionProperty".as_ptr()),
        )
    };
    if connection.is_null() || set.is_null() {
        BACKGROUND_CURSOR_RESULT.with(|result| result.set(-99));
        return;
    }
    // SAFETY: the private CoreGraphics functions have these signatures, as in Ardia; NSString is a CFString.
    let result = unsafe {
        let connection: DefaultConnection = std::mem::transmute(connection);
        let set: SetConnectionProperty = std::mem::transmute(set);
        let cid = connection();
        let key = ptr::from_ref(ns_string!("SetsCursorInBackground")).cast::<c_void>();
        set(
            cid,
            cid,
            key,
            if allowed {
                kCFBooleanTrue
            } else {
                kCFBooleanFalse
            },
        )
    };
    BACKGROUND_CURSOR_RESULT.with(|cell| cell.set(result));
}

fn chrome_cursor(window: Option<&NSWindow>) -> Retained<NSCursor> {
    window
        .and_then(|window| {
            // SAFETY: `delegate` returns the window delegate or nil.
            let delegate: Option<Retained<AnyObject>> = unsafe { msg_send![window, delegate] };
            delegate
        })
        .filter(|delegate| {
            // SAFETY: `respondsToSelector:` is sent to a live object.
            unsafe { msg_send![&**delegate, respondsToSelector: sel!(cursor)] }
        })
        .and_then(|delegate| {
            // SAFETY: the delegate answers `cursor`, which takes no arguments and returns an object.
            let cursor: Option<Retained<AnyObject>> =
                unsafe { msg_send![&*delegate, performSelector: sel!(cursor)] };
            cursor
        })
        .and_then(|cursor| cursor.downcast::<NSCursor>().ok())
        .unwrap_or_else(NSCursor::arrowCursor)
}

fn keep_resize_cursor_on_chrome_cursor_updates(window_class: &AnyClass) {
    if !CURSOR_UPDATE_CLASSES.with(|done| {
        done.borrow_mut()
            .insert(ptr::from_ref(window_class) as usize)
    }) {
        return;
    }
    let update = sel!(cursorUpdate:);
    let Some(method) = window_class.instance_method(update) else {
        return;
    };
    // SAFETY: `cursorUpdate:` takes one NSEvent and returns nothing.
    let original: unsafe extern "C-unwind" fn(&NSWindow, Sel, &NSEvent) =
        unsafe { std::mem::transmute(method.implementation()) };
    let block = RcBlock::new(move |window: NonNull<NSWindow>, event: NonNull<NSEvent>| {
        // SAFETY: AppKit sends `cursorUpdate:` to a live window with a live event.
        let (window, event) = unsafe { (window.as_ref(), event.as_ref()) };
        let resize = Controller::for_window(window)
            .and_then(|controller| controller.resize_cursor.borrow().clone());
        if resize.is_some_and(|resize| resize.follow(event.locationInWindow())) {
            return;
        }
        // SAFETY: `original` is the implementation that the window class had for this selector.
        unsafe { original(window, update, event) };
    });
    // SAFETY: the block has the signature of `cursorUpdate:` with the receiver first.
    let imp = unsafe { ffi::imp_implementationWithBlock(RcBlock::as_ptr(&block).cast()) };
    replace_method(window_class, update, imp);
}

struct TargetIvars(std::rc::Weak<ResizeCursor>);

define_class!(
    /// The owner of the tracking area: it gives the mouse events to the `ResizeCursor`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "RiaPiPResizeCursor"]
    #[ivars = TargetIvars]
    struct Target;

    impl Target {
        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, event: &NSEvent) {
            if let Some(cursor) = self.ivars().0.upgrade() {
                cursor.follow(event.locationInWindow());
            }
        }

        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) {
            if let Some(cursor) = self.ivars().0.upgrade() {
                cursor.follow(event.locationInWindow());
            }
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            if let Some(cursor) = self.ivars().0.upgrade() {
                cursor.stop_showing(Some(&NSCursor::arrowCursor()));
            }
        }
    }
);

pub struct ResizeCursor {
    mtm: MainThreadMarker,
    window: Weak<NSWindow>,
    view: Weak<NSView>,
    _target: Retained<Target>,
    tracking: Retained<NSTrackingArea>,
    showing: Cell<bool>,
    in_background: Cell<bool>,
    enabled: Cell<bool>,
    shown: Cell<i64>,
    resizes: Cell<i64>,
    resizes_without_cursor: Cell<i64>,
}

impl ResizeCursor {
    /// `initWithWindow:view:`.
    pub fn new(window: &NSWindow, view: &NSView) -> Rc<Self> {
        let mtm = window.mtm();
        let cursor = Rc::new_cyclic(|me| {
            let target = Target::alloc(mtm).set_ivars(TargetIvars(me.clone()));
            // SAFETY: The allocation has its Rust ivars before NSObject initialization.
            let target: Retained<Target> = unsafe { msg_send![super(target), init] };
            // SAFETY: The cursor keeps the owner alive and removes the area in `detach`.
            let tracking = unsafe {
                NSTrackingArea::initWithRect_options_owner_userInfo(
                    NSTrackingArea::alloc(),
                    NSRect::ZERO,
                    NSTrackingAreaOptions::MouseMoved
                        | NSTrackingAreaOptions::MouseEnteredAndExited
                        | NSTrackingAreaOptions::ActiveAlways
                        | NSTrackingAreaOptions::InVisibleRect,
                    Some(&target),
                    None,
                )
            };
            view.addTrackingArea(&tracking);
            Self {
                mtm,
                window: Weak::from(window),
                view: Weak::from(view),
                _target: target,
                tracking,
                showing: Cell::new(false),
                in_background: Cell::new(false),
                enabled: Cell::new(true),
                shown: Cell::new(0),
                resizes: Cell::new(0),
                resizes_without_cursor: Cell::new(0),
            }
        });
        keep_resize_cursor_on_chrome_cursor_updates(window.class());
        cursor
    }

    pub fn detach(&self) {
        if let Some(view) = self.view.load() {
            view.removeTrackingArea(&self.tracking);
        }
        self.stop_showing(None);
    }

    fn stop_showing(&self, cursor: Option<&NSCursor>) {
        if self.showing.get()
            && let Some(cursor) = cursor
        {
            cursor.set();
        }
        self.showing.set(false);
        if self.in_background.get() {
            set_cursor_in_background(false);
        }
        self.in_background.set(false);
    }

    pub fn showing(&self) -> bool {
        self.showing.get()
    }

    pub fn enabled(&self) -> bool {
        self.enabled.get()
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.set(enabled);
        if !enabled {
            self.stop_showing(Some(&NSCursor::arrowCursor()));
        }
    }

    pub fn follow(&self, point_in_window: NSPoint) -> bool {
        let window = self.window.load();
        if !self.enabled.get() {
            return false;
        }
        if NSEvent::pressedMouseButtons() != 0 {
            return self.showing.get();
        }
        if let Some(resize) = window
            .as_deref()
            .and_then(|window| resize_cursor_at(window, point_in_window))
        {
            if !self.in_background.get() {
                set_cursor_in_background(true);
                self.in_background.set(true);
            }
            resize.set();
            if !self.showing.get() {
                self.shown.set(self.shown.get() + 1);
            }
            self.showing.set(true);
            return true;
        }
        self.stop_showing(Some(&chrome_cursor(window.as_deref())));
        false
    }

    pub fn resize_started(&self) -> Value {
        let window = self.window.load();
        let pointer = window.as_deref().map_or(NSPoint::ZERO, |window| {
            window.convertPointFromScreen(NSEvent::mouseLocation())
        });
        let inside = window.as_deref().is_some_and(|window| {
            let frame = window.frame();
            point_in_rect(pointer, rect(0.0, 0.0, width(frame), height(frame)))
        });
        let cursor_shown = !inside || self.showing.get();
        self.resizes.set(self.resizes.get() + 1);
        if !cursor_shown {
            self.resizes_without_cursor
                .set(self.resizes_without_cursor.get() + 1);
        }
        json!({"inside": inside, "cursorShown": cursor_shown})
    }

    pub fn probe(&self) -> Value {
        let Some(window) = self.window.load() else {
            return json!({});
        };
        let size = window.frame().size;
        const OFFSETS: [i64; 13] = [-6, -3, -1, 0, 1, 2, 3, 4, 5, 6, 8, 10, 14];
        // shortcut: Ardia walks an NSDictionary, whose order is unspecified; this keeps the literal order.
        let anchors = [
            ("left", NSPoint::new(0.0, size.height / 2.0)),
            ("right", NSPoint::new(size.width, size.height / 2.0)),
            ("top", NSPoint::new(size.width / 2.0, size.height)),
            ("bottom", NSPoint::new(size.width / 2.0, 0.0)),
            ("topLeft", NSPoint::new(0.0, size.height)),
            ("bottomRight", NSPoint::new(size.width, 0.0)),
        ];
        let edges: Vec<Value> = anchors
            .iter()
            .map(|&(name, anchor)| {
                let in_x = if anchor.x == 0.0 {
                    1.0
                } else if anchor.x == size.width {
                    -1.0
                } else {
                    0.0
                };
                let in_y = if anchor.y == 0.0 {
                    1.0
                } else if anchor.y == size.height {
                    -1.0
                } else {
                    0.0
                };
                let row: Vec<Value> = OFFSETS
                    .iter()
                    .map(|&offset| {
                        let point = NSPoint::new(
                            anchor.x + in_x * offset as f64,
                            anchor.y + in_y * offset as f64,
                        );
                        let position = position_at(&window, point);
                        json!({
                            "in": offset,
                            "direction": resize_direction(&window, point).unwrap_or(-1),
                            "cursor": position.map_or("", position_name),
                        })
                    })
                    .collect();
                json!({"edge": name, "points": row})
            })
            .collect();
        json!({
            "size": size_string(size),
            "resizes": resizes(&window),
            "enabled": self.enabled.get(),
            "backgroundCursorResult": BACKGROUND_CURSOR_RESULT.with(Cell::get),
            "appActive": NSApplication::sharedApplication(self.mtm).isActive(),
            "showing": self.showing.get(),
            "shown": self.shown.get(),
            "resizeStarts": self.resizes.get(),
            "resizeStartsWithoutCursor": self.resizes_without_cursor.get(),
            "edges": edges,
        })
    }
}

impl Controller {
    /// `ArdiaPiPController (ResizeCursor) -resizeStarted`.
    pub fn resize_started(&self) {
        let stamp = begin_stamp();
        let started = self
            .resize_cursor
            .borrow()
            .as_ref()
            .map_or(Value::Null, |cursor| cursor.resize_started());
        if let Some(view) = self.view() {
            view.emit("pictureInPictureResized", stamp_onto(&stamp, started));
        }
    }
}
