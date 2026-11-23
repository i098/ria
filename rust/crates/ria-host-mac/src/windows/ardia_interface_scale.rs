// Ported from ardia packages/ardiacore/ios/ArdiaInterfaceScale.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::{Cell, RefCell};
use std::ptr::NonNull;
use std::time::Duration;

use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyClass, AnyObject};
use objc2_app_kit::{NSView, NSViewFrameDidChangeNotification, NSWindow};
use objc2_foundation::{
    NSNotification, NSNotificationCenter, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
    NSUserDefaults, ns_string,
};
use ria_engine::call::main_queue;

pub const MINIMUM: f64 = 0.8;
pub const MAXIMUM: f64 = 1.5;

thread_local! {
    static SCALE: Cell<Option<f64>> = const { Cell::new(None) };
    static CHANGE: Cell<u64> = const { Cell::new(0) };
    static SETTLED: Cell<u64> = const { Cell::new(0) };
    static ROOTS: RefCell<Vec<RootWatch>> = const { RefCell::new(Vec::new()) };
}

struct RootWatch {
    view: Weak<NSView>,
    observer: Retained<AnyObject>,
}

impl Drop for RootWatch {
    fn drop(&mut self) {
        // SAFETY: The token came from this notification center.
        unsafe { NSNotificationCenter::defaultCenter().removeObserver(&self.observer) };
    }
}

pub fn clamped(value: f64) -> f64 {
    if value.is_nan() || value <= 0.0 {
        return 1.0;
    }
    ((value * 100.0).round() / 100.0).clamp(MINIMUM, MAXIMUM)
}

pub fn scale() -> f64 {
    SCALE.with(|cached| {
        cached.get().unwrap_or_else(|| {
            let value = clamped(
                NSUserDefaults::standardUserDefaults().doubleForKey(ns_string!("interfaceScale")),
            );
            cached.set(Some(value));
            value
        })
    })
}

fn roots() -> Vec<Retained<NSView>> {
    ROOTS.with(|roots| {
        let mut roots = roots.borrow_mut();
        roots.retain(|root| root.view.load().is_some());
        roots.iter().filter_map(|root| root.view.load()).collect()
    })
}

fn notify(name: &NSString, view: Option<&NSView>) {
    // SAFETY: The notification has a local name and an optional live sender.
    unsafe {
        NSNotificationCenter::defaultCenter()
            .postNotificationName_object(name, view.map(|view| view as &AnyObject))
    };
}

fn fit_root(root: &NSView) {
    let frame = root.frame().size;
    let scale = scale();
    let size = NSSize::new(frame.width / scale, frame.height / scale);
    if root.bounds().size != size {
        root.setBoundsSize(size);
        root.setNeedsLayout(true);
    }
}

fn redraw(view: &NSView) {
    // SAFETY: The window views and their layers are accessed on the main thread.
    unsafe {
        if [c"ArdiaWebView", c"WKWebView", c"MTKView"]
            .into_iter()
            .any(|name| AnyClass::get(name).is_some_and(|class| view.isKindOfClass(class)))
        {
            return;
        }
        if let Some(layer) = view.layer() {
            if AnyClass::get(c"CAMetalLayer").is_some_and(|class| layer.isKindOfClass(class)) {
                return;
            }
            if layer.contents().is_some() && view.window().is_some() {
                let density = view.convertSizeToBacking(NSSize::new(1., 1.)).width;
                let previous: f64 = objc2::msg_send![&*layer, contentsScale];
                if density > 0. && (previous - density).abs() > density * 0.001 {
                    let _: () = objc2::msg_send![&*layer, setContentsScale: density];
                }
            }
        }
        view.setNeedsDisplay(true);
        for child in view.subviews().iter() {
            redraw(&child);
        }
    }
}

pub fn set_scale(asked: f64) {
    let value = clamped(asked);
    if value == scale() {
        return;
    }
    SCALE.set(Some(value));
    NSUserDefaults::standardUserDefaults().setDouble_forKey(value, ns_string!("interfaceScale"));
    for root in roots() {
        fit_root(&root);
    }
    notify(ns_string!("ArdiaInterfaceScaleDidChange"), None);
    for root in roots() {
        redraw(&root);
    }
    let change = CHANGE.get().wrapping_add(1);
    CHANGE.set(change);
    main_queue::after(
        Duration::from_millis(350),
        Box::new(move || {
            if SETTLED.get() != change && CHANGE.get() == change {
                layout_settled();
            }
        }),
    );
}

pub fn layout_settled() {
    SETTLED.set(CHANGE.get());
    for root in roots() {
        redraw(&root);
    }
    notify(ns_string!("ArdiaInterfaceScaleDidSettle"), None);
}

pub fn follow(root: &NSView) {
    if roots().iter().any(|view| std::ptr::eq(&**view, root)) {
        return;
    }
    root.setPostsFrameChangedNotifications(true);
    let weak = Weak::from(root);
    let observer = RcBlock::new(move |_: NonNull<NSNotification>| {
        if let Some(root) = weak.load() {
            fit_root(&root);
        }
    });
    // SAFETY: The sender and callback use AppKit's main thread.
    let observer = unsafe {
        NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
            Some(NSViewFrameDidChangeNotification),
            Some(root),
            None,
            &observer,
        )
    };
    ROOTS.with_borrow_mut(|roots| {
        roots.push(RootWatch {
            view: Weak::from(root),
            observer: observer.into(),
        })
    });
    fit_root(root);
}

pub fn root_attached(root: &NSView) {
    follow(root);
    fit_root(root);
    notify(ns_string!("ArdiaInterfaceScaleRootDidAttach"), Some(root));
}

pub fn fit_page_host(host: &NSView) {
    let size = window_size_from_root(host.frame().size);
    if host.bounds().size != size {
        host.setBoundsSize(size);
    }
}

pub fn root_of_window(window: &NSWindow) -> Option<Retained<NSView>> {
    roots().into_iter().find(|root| {
        root.window()
            .is_some_and(|owner| std::ptr::eq(&*owner, window))
    })
}

fn content_top(window: &NSWindow) -> f64 {
    window
        .contentView()
        .map_or(0., |view| view.frame().size.height)
}

pub fn window_point_from_root(point: NSPoint, window: &NSWindow) -> NSPoint {
    if let Some(root) = root_of_window(window) {
        return root.convertPoint_toView(point, None);
    }
    let scale = scale();
    NSPoint::new(point.x * scale, content_top(window) - point.y * scale)
}

pub fn window_rect_from_root(rect: NSRect, window: &NSWindow) -> NSRect {
    if let Some(root) = root_of_window(window) {
        return root.convertRect_toView(rect, None);
    }
    let size = window_size_from_root(rect.size);
    let origin = window_point_from_root(
        NSPoint::new(rect.origin.x, rect.origin.y + rect.size.height),
        window,
    );
    NSRect::new(origin, size)
}

pub fn root_point_from_window(point: NSPoint, window: &NSWindow) -> NSPoint {
    if let Some(root) = root_of_window(window) {
        return root.convertPoint_fromView(point, None);
    }
    let scale = scale();
    NSPoint::new(point.x / scale, (content_top(window) - point.y) / scale)
}

pub fn root_rect_from_window(rect: NSRect, window: &NSWindow) -> NSRect {
    if let Some(root) = root_of_window(window) {
        return root.convertRect_fromView(rect, None);
    }
    let size = root_size_from_window(rect.size);
    let origin = root_point_from_window(
        NSPoint::new(rect.origin.x, rect.origin.y + rect.size.height),
        window,
    );
    NSRect::new(origin, size)
}

pub fn screen_point_from_root(point: NSPoint, window: &NSWindow) -> NSPoint {
    window.convertPointToScreen(window_point_from_root(point, window))
}

pub fn root_point_from_screen(point: NSPoint, window: &NSWindow) -> NSPoint {
    root_point_from_window(window.convertPointFromScreen(point), window)
}

pub fn window_size_from_root(size: NSSize) -> NSSize {
    let scale = scale();
    NSSize::new(size.width * scale, size.height * scale)
}

pub fn root_size_from_window(size: NSSize) -> NSSize {
    let scale = scale();
    NSSize::new(size.width / scale, size.height / scale)
}

pub fn window_length_from_root(length: f64) -> f64 {
    length * scale()
}

pub fn root_length_from_window(length: f64) -> f64 {
    length / scale()
}
