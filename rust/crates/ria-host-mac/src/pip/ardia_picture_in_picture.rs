// Ported from ardia packages/ardiacore/ios/ArdiaPictureInPicture.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.
// Companion header: packages/ardiacore/ios/ArdiaPictureInPicture.h (the entry points of namespace ardia_pip).

//! The AppKit side of Picture in Picture: the entry points that the page calls, the event monitors,
//! the activation of the app for the window, and the dev actions.

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::fmt::Write;
use std::ptr::NonNull;
use std::rc::{Rc, Weak};
use std::thread::LocalKey;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{MainThreadMarker, Message, msg_send, sel};
use objc2_app_kit::{
    NSApplicationDidBecomeActiveNotification, NSApplicationDidResignActiveNotification, NSEvent,
    NSEventMask, NSEventModifierFlags, NSEventType, NSFloatingWindowLevel, NSMenu, NSView,
    NSWindow, NSWindowDidBecomeKeyNotification, NSWindowStyleMask,
};
use objc2_foundation::{
    NSNotification, NSNotificationCenter, NSPoint, NSProcessInfo, NSRectFromString, NSString,
};
use serde_json::{Value, json};

use super::ardia_picture_in_picture_internal::{
    Edge, PipPage, after, background, begin_stamp, count_activation,
    is_chrome_document_picture_in_picture, is_chrome_floating_window,
    is_chrome_video_picture_in_picture, min_x, min_y, point_in_rect, pointer_of,
    prevent_activation, rect_string, resizable, resize_direction, screen_for, services,
    shared_application, size_string, stamp_onto, width,
};
use super::ardia_pip_controller::Controller;
use super::ardia_pip_controller_workspaces::{join_full_screen_spaces, joins_full_screen_spaces};
use super::ardia_pip_seek_math as seek;
use super::ardia_pip_zoom::Zoom;
use crate::windows::ardia_chrome_window_host::ChromeWindowHost;

type Monitor = RefCell<Option<Retained<AnyObject>>>;

/// The press that started a drag of the window background, as the `gGrab*` globals.
#[derive(Default)]
struct Grab {
    dragged: Weak<Controller>,
    local: NSPoint,
    mouse: NSPoint,
    travel: f64,
    skip: f64,
    clicks: isize,
    handle: bool,
}

thread_local! {
    static MENU_MONITOR: Monitor = const { RefCell::new(None) };
    static DRAG_MONITOR: Monitor = const { RefCell::new(None) };
    static ZOOM_MONITOR: Monitor = const { RefCell::new(None) };
    static GRAB: RefCell<Grab> = RefCell::new(Grab::default());
    static BACK_TO_TAB_ACTIVATION_UNTIL: Cell<f64> = const { Cell::new(0.0) };
    static ACTIVE_ONLY_FOR_PICTURE_IN_PICTURE: Cell<bool> = const { Cell::new(false) };
    static WATCHING_ACTIVE_ONLY: Cell<bool> = const { Cell::new(false) };
}

fn main_thread() -> MainThreadMarker {
    MainThreadMarker::new().expect("Picture in Picture runs on the main thread")
}

fn windows(mtm: MainThreadMarker) -> Vec<Retained<NSWindow>> {
    shared_application(mtm).windows().iter().collect()
}

fn note_pointer(event: &NSEvent, label: &str) {
    if let Some(services) = services() {
        (services.note_pointer)(event, label);
    }
}

fn log_activation(line: &str) {
    if let Some(services) = services() {
        (services.log_activation)(line);
    }
}

fn test_instance() -> bool {
    services().is_some_and(|services| services.launch.test_instance)
}

fn uptime() -> f64 {
    NSProcessInfo::processInfo().systemUptime()
}

/// `addLocalMonitorForEventsMatchingMask:handler:` once per slot; `swallow` returns true to drop the event.
fn install_monitor(
    slot: &'static LocalKey<Monitor>,
    mask: NSEventMask,
    swallow: impl Fn(&NSEvent) -> bool + 'static,
) {
    if slot.with(|monitor| monitor.borrow().is_some()) {
        return;
    }
    // SAFETY: AppKit gives the block a live event and takes back that event or null.
    let monitor = unsafe {
        NSEvent::addLocalMonitorForEventsMatchingMask_handler(
            mask,
            &RcBlock::new(move |event: NonNull<NSEvent>| {
                if swallow(event.as_ref()) {
                    std::ptr::null_mut()
                } else {
                    event.as_ptr()
                }
            }),
        )
    };
    slot.with(|slot| *slot.borrow_mut() = monitor);
}

fn install_menu_monitor(mtm: MainThreadMarker) {
    install_monitor(&MENU_MONITOR, NSEventMask::RightMouseDown, move |event| {
        let Some(controller) = event
            .window(mtm)
            .and_then(|window| Controller::for_window(&window))
        else {
            return false;
        };
        NSMenu::popUpContextMenu_withEvent_forView(
            &controller.menu(),
            event,
            &controller.overlay(),
        );
        true
    });
}

fn click_skip_seconds(window: &NSWindow, local: NSPoint) -> f64 {
    match Controller::for_window(window) {
        Some(controller)
            if controller.view().is_some() && controller.stashed_edge() == Edge::None =>
        {
            seek::skip_seconds_at(local.x, width(window.frame()))
        }
        _ => 0.0,
    }
}

fn on_stash_handle(window: &NSWindow, event: &NSEvent) -> bool {
    let Some(controller) = Controller::for_window(window) else {
        return false;
    };
    let handle = controller.overlay().handle();
    if handle.isHidden() || controller.stashed_edge() == Edge::None {
        return false;
    }
    point_in_rect(
        handle.convertPoint_fromView(event.locationInWindow(), None),
        handle.bounds(),
    )
}

fn grabs_window_background(window: &NSWindow, event: &NSEvent) -> bool {
    if Controller::for_window(window).is_none()
        || event
            .modifierFlags()
            .contains(NSEventModifierFlags::Control)
    {
        return false;
    }
    if on_stash_handle(window, event) {
        return true;
    }
    let location = event.locationInWindow();
    if event.clickCount() > 1 && click_skip_seconds(window, location) == 0.0 {
        return false;
    }
    if resize_direction(window, location).is_some_and(|direction| direction != -1) {
        return false;
    }
    let Some(frame_view) = window.contentView().and_then(|content| superview(&content)) else {
        return false;
    };
    frame_view
        .hitTest(location)
        .is_some_and(|hit| std::ptr::eq(&*hit, &*frame_view))
}

fn note_grab_travel(mtm: MainThreadMarker, event: &NSEvent) {
    let mouse = pointer_of(mtm, event);
    GRAB.with(|grab| {
        let mut grab = grab.borrow_mut();
        grab.travel = grab
            .travel
            .max(seek::travel(mouse.x - grab.mouse.x, mouse.y - grab.mouse.y));
    });
}

fn install_drag_monitor(mtm: MainThreadMarker) {
    let mask =
        NSEventMask::LeftMouseDown | NSEventMask::LeftMouseDragged | NSEventMask::LeftMouseUp;
    install_monitor(&DRAG_MONITOR, mask, move |event| {
        if Zoom::continue_edge_resize(event) {
            return true;
        }
        if event.r#type() == NSEventType::LeftMouseDown {
            let window = event.window(mtm);
            if let Some(window) = &window
                && Zoom::begin_edge_resize(window, event)
            {
                if let Some(controller) = Controller::for_window(window) {
                    activate_with_only_picture_in_picture(&controller, false);
                }
                return true;
            }
            let grabbed = window.as_ref().and_then(|window| {
                grabs_window_background(window, event)
                    .then(|| Controller::for_window(window))
                    .flatten()
            });
            GRAB.with(|grab| {
                grab.borrow_mut().dragged = grabbed.as_ref().map_or_else(Weak::new, Rc::downgrade);
            });
            let (Some(grabbed), Some(window)) = (grabbed, window) else {
                return false;
            };
            activate_with_only_picture_in_picture(&grabbed, false);
            grabbed.drag_began(event);
            let mouse = pointer_of(mtm, event);
            let frame = window.frame();
            let skip = click_skip_seconds(&window, event.locationInWindow());
            let handle = on_stash_handle(&window, event);
            GRAB.with(|grab| {
                let mut grab = grab.borrow_mut();
                grab.mouse = mouse;
                grab.local = NSPoint::new(mouse.x - min_x(frame), mouse.y - min_y(frame));
                grab.travel = 0.0;
                grab.skip = skip;
                grab.clicks = event.clickCount();
                grab.handle = handle;
            });
            note_pointer(event, "pipDrag");
            return true;
        }
        let Some(dragged) = GRAB.with(|grab| grab.borrow().dragged.upgrade()) else {
            return false;
        };
        note_grab_travel(mtm, event);
        if event.r#type() == NSEventType::LeftMouseUp {
            GRAB.with(|grab| grab.borrow_mut().dragged = Weak::new());
            dragged.drag_ended(event);
            let (skip, travel, local, clicks, handle) = GRAB.with(|grab| {
                let grab = grab.borrow();
                (grab.skip, grab.travel, grab.local, grab.clicks, grab.handle)
            });
            if skip != 0.0 && seek::is_click(travel) {
                dragged.skip_dragged_by(skip, local, clicks as i64);
            }
            if handle && seek::is_click(travel) {
                dragged.unstash_from_handle();
            }
            note_pointer(event, "pipDrag");
        } else {
            dragged.drag_moved(event);
        }
        true
    });
}

fn install_zoom_monitor() {
    install_monitor(
        &ZOOM_MONITOR,
        NSEventMask::ScrollWheel | NSEventMask::Magnify,
        |event| Controller::handle_swipe(event) || Zoom::handle(event),
    );
}

fn attach(mtm: MainThreadMarker, view: &Rc<dyn PipPage>, host: &str, stamp: &Value) -> bool {
    for window in windows(mtm) {
        let controller = Controller::for_window(&window);
        if !is_chrome_video_picture_in_picture(&window)
            && !(controller.is_some() && window.isVisible())
        {
            continue;
        }
        if !window.hasShadow() {
            continue;
        }
        if controller
            .as_ref()
            .and_then(|controller| controller.view())
            .is_some_and(|shown| shown.browser_id() != view.browser_id())
        {
            continue;
        }
        let fresh = controller.is_none();
        let controller = controller.unwrap_or_else(|| Controller::new(&window));
        controller.set_view(Some(view));
        controller.set_host(host);
        controller.round_corners();
        join_full_screen_spaces(&window);
        prevent_activation(&window);
        if fresh {
            view.emit(
                "pictureInPicture",
                stamp_onto(
                    stamp,
                    json!({"kind": "video", "active": true, "fullScreenSpaces": joins_full_screen_spaces(&window), "leftApp": view.left_app_in_picture_in_picture(), "leftTab": !view.seen_by_user()}),
                ),
            );
        }
        if background() {
            window.setAlphaValue(0.0);
            window.setIgnoresMouseEvents(true);
        }
        install_menu_monitor(mtm);
        install_drag_monitor(mtm);
        install_zoom_monitor();
        let self_test = if background() {
            services()
                .and_then(|services| services.launch.test_value("pipSelfTest").map(str::to_owned))
        } else {
            None
        };
        match self_test.as_deref() {
            Some(test @ ("close" | "backToTab")) if fresh => controller.run_button_self_test(test),
            Some("hover") if fresh => controller.run_hover_self_test(),
            Some(_) if fresh => controller.run_self_test(),
            _ => {}
        }
        return true;
    }
    false
}

fn dev_show_alpha(window: &NSWindow, alpha: f32) -> String {
    type MainConnection = unsafe extern "C" fn() -> i32;
    type SetAlpha = unsafe extern "C" fn(i32, i32, f32) -> i32;
    // SAFETY: `dlsym` looks up the names in the loaded images.
    let (connection, set) = unsafe {
        (
            libc::dlsym(libc::RTLD_DEFAULT, c"CGSMainConnectionID".as_ptr()),
            libc::dlsym(libc::RTLD_DEFAULT, c"CGSSetWindowAlpha".as_ptr()),
        )
    };
    if !test_instance() {
        return "test instances only".into();
    }
    if connection.is_null() || set.is_null() {
        return "no CGSSetWindowAlpha".into();
    }
    // SAFETY: the private SkyLight functions have these signatures, as in Ardia.
    let refused = unsafe {
        let connection: MainConnection = std::mem::transmute(connection);
        let set: SetAlpha = std::mem::transmute(set);
        set(connection(), window.windowNumber() as i32, alpha)
    };
    if refused == 0 { "started" } else { "refused" }.into()
}

fn superview(view: &NSView) -> Option<Retained<NSView>> {
    // SAFETY: the view is live; `superview` returns its parent or nil.
    unsafe { view.superview() }
}

fn layer_of(view: &NSView) -> Option<Retained<AnyObject>> {
    // SAFETY: `layer` returns a CALayer or nil.
    unsafe { msg_send![view, layer] }
}

fn corner_radius_of(layer: Option<&AnyObject>) -> f64 {
    // SAFETY: `cornerRadius` is a CGFloat property of CALayer.
    layer.map_or(0.0, |layer| unsafe { msg_send![layer, cornerRadius] })
}

fn border_width_of(layer: Option<&AnyObject>) -> f64 {
    // SAFETY: `borderWidth` is a CGFloat property of CALayer.
    layer.map_or(0.0, |layer| unsafe { msg_send![layer, borderWidth] })
}

fn masks_to_bounds(layer: Option<&AnyObject>) -> bool {
    // SAFETY: `masksToBounds` is a BOOL property of CALayer.
    layer.is_some_and(|layer| unsafe { msg_send![layer, masksToBounds] })
}

fn class_name(object: &AnyObject) -> String {
    object.class().name().to_string_lossy().into_owned()
}

fn ns_string_or_empty(string: Option<Retained<NSString>>) -> String {
    string.map_or_else(String::new, |string| string.to_string())
}

fn dev_describe_layer(layer: &AnyObject, depth: usize, out: &mut String) {
    // SAFETY: every getter below is a property of CALayer with the type it is read as.
    let (name, frame, bounds, mask_to_bounds, curve, opacity, scale, hidden, mask, sublayers) = unsafe {
        let name: Option<Retained<NSString>> = msg_send![layer, name];
        let frame: objc2_foundation::NSRect = msg_send![layer, frame];
        let bounds: objc2_foundation::NSRect = msg_send![layer, bounds];
        let curve: Retained<NSString> = msg_send![layer, cornerCurve];
        let opacity: f32 = msg_send![layer, opacity];
        let scale: f64 = msg_send![layer, contentsScale];
        let hidden: bool = msg_send![layer, isHidden];
        let mask: Option<Retained<AnyObject>> = msg_send![layer, mask];
        let sublayers: Option<Retained<objc2_foundation::NSArray<AnyObject>>> =
            msg_send![layer, sublayers];
        (
            name,
            frame,
            bounds,
            masks_to_bounds(Some(layer)),
            curve,
            opacity,
            scale,
            hidden,
            mask,
            sublayers,
        )
    };
    let mask = mask.map_or_else(String::new, |mask| {
        // SAFETY: `frame` is a CGRect property of CALayer.
        let frame: objc2_foundation::NSRect = unsafe { msg_send![&mask, frame] };
        format!(" layerMask {}", rect_string(frame))
    });
    let _ = writeln!(
        out,
        "{}L {} {} frame {} bounds {} radius {:.2} mask {} curve {} border {:.1} opacity {:.2} scale {:.1} hidden {}{}",
        "  ".repeat(depth),
        class_name(layer),
        ns_string_or_empty(name),
        rect_string(frame),
        rect_string(bounds),
        corner_radius_of(Some(layer)),
        u8::from(mask_to_bounds),
        curve,
        border_width_of(Some(layer)),
        opacity,
        scale,
        u8::from(hidden),
        mask,
    );
    for sublayer in sublayers.iter().flat_map(|sublayers| sublayers.iter()) {
        dev_describe_layer(&sublayer, depth + 1, out);
    }
}

fn dev_describe_view(view: &NSView, depth: usize, out: &mut String) {
    let _ = writeln!(
        out,
        "{}V {} frame {} bounds {} layer {} flipped {}",
        "  ".repeat(depth),
        class_name(view),
        rect_string(view.frame()),
        rect_string(view.bounds()),
        u8::from(layer_of(view).is_some()),
        u8::from(view.isFlipped()),
    );
    for subview in view.subviews() {
        dev_describe_view(&subview, depth + 1, out);
    }
}

fn dev_find_view(view: &NSView, class: &str) -> Option<Retained<NSView>> {
    if class_name(view) == class {
        return Some(view.retain());
    }
    view.subviews()
        .iter()
        .find_map(|subview| dev_find_view(&subview, class))
}

fn dev_json(object: &Value) -> String {
    serde_json::to_string(object).unwrap_or_default()
}

fn dev_framing(controller: &Controller) -> String {
    let window = &controller.window;
    let content = window.contentView();
    let content_layer = content.as_deref().and_then(layer_of);
    let compositor = content
        .as_deref()
        .and_then(|content| dev_find_view(content, "ViewsCompositorSuperview"));
    let overlay = controller.overlay();
    let rim = layer_of(&overlay);
    let style = window.styleMask();
    let framing = json!({
        "styleMask": style.0,
        "fullSizeContentView": style.contains(NSWindowStyleMask::FullSizeContentView),
        "titled": style.contains(NSWindowStyleMask::Titled),
        "window": size_string(window.frame().size),
        "contentRect": rect_string(window.contentRectForFrameRect(window.frame())),
        "content": rect_string(content.as_ref().map_or(objc2_foundation::NSRect::ZERO, |content| content.frame())),
        "compositor": compositor.map_or_else(String::new, |compositor| rect_string(compositor.convertRect_toView(compositor.bounds(), None))),
        "overlay": rect_string(overlay.convertRect_toView(overlay.bounds(), None)),
        "contentRadius": corner_radius_of(content_layer.as_deref()),
        "contentMasks": masks_to_bounds(content_layer.as_deref()),
        "rimRadius": corner_radius_of(rim.as_deref()),
        "rimWidth": border_width_of(rim.as_deref()),
        "radius": controller.corner_radius(),
        "scale": window.backingScaleFactor(),
    });
    dev_json(&framing)
}

fn dev_layer_tree(window: &NSWindow) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "window {} frame {} content {} style {} scale {:.1} opaque {} shadow {}",
        class_name(window),
        rect_string(window.frame()),
        rect_string(window.contentRectForFrameRect(window.frame())),
        window.styleMask().0,
        window.backingScaleFactor(),
        u8::from(window.isOpaque()),
        u8::from(window.hasShadow()),
    );
    if let Some(frame_view) = window
        .contentView()
        .and_then(|content| superview(&content).or(Some(content)))
    {
        dev_describe_view(&frame_view, 0, &mut out);
        if let Some(layer) = layer_of(&frame_view) {
            dev_describe_layer(&layer, 0, &mut out);
        }
    }
    out
}

pub fn mark_back_to_tab_activation() {
    if !background() && !shared_application(main_thread()).isActive() {
        BACK_TO_TAB_ACTIVATION_UNTIL.with(|until| until.set(uptime() + 2.0));
    }
}

pub fn consume_back_to_tab_activation() -> bool {
    let pending = uptime() <= BACK_TO_TAB_ACTIVATION_UNTIL.with(Cell::get);
    BACK_TO_TAB_ACTIVATION_UNTIL.with(|until| until.set(0.0));
    pending
}

fn end_active_only_for_picture_in_picture() {
    if !ACTIVE_ONLY_FOR_PICTURE_IN_PICTURE.with(Cell::get) {
        return;
    }
    ACTIVE_ONLY_FOR_PICTURE_IN_PICTURE.with(|active| active.set(false));
    if let Some(services) = services() {
        for window in windows(services.mtm) {
            (services.reconsider_sight)(&window);
        }
    }
}

fn watch_active_only_for_picture_in_picture() {
    if WATCHING_ACTIVE_ONLY.with(Cell::get) {
        return;
    }
    WATCHING_ACTIVE_ONLY.with(|watching| watching.set(true));
    let center = NSNotificationCenter::defaultCenter();
    let resigned =
        RcBlock::new(|_: NonNull<NSNotification>| end_active_only_for_picture_in_picture());
    let became_key = RcBlock::new(|note: NonNull<NSNotification>| {
        // SAFETY: the notification stays live during this callback.
        let window = unsafe { note.as_ref() }
            .object()
            .and_then(|object| object.downcast::<NSWindow>().ok());
        if window
            .and_then(|window| Controller::for_window(&window))
            .is_none()
        {
            end_active_only_for_picture_in_picture();
        }
    });
    // SAFETY: both blocks take AppKit's notifications on the main thread; the center keeps the observers.
    unsafe {
        center.addObserverForName_object_queue_usingBlock(
            Some(NSApplicationDidResignActiveNotification),
            None,
            None,
            &resigned,
        );
        center.addObserverForName_object_queue_usingBlock(
            Some(NSWindowDidBecomeKeyNotification),
            None,
            None,
            &became_key,
        );
    }
}

pub fn active_only_for_picture_in_picture() -> bool {
    ACTIVE_ONLY_FOR_PICTURE_IN_PICTURE.with(Cell::get)
}

#[repr(C)]
struct ProcessSerialNumber {
    high: u32,
    low: u32,
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn GetCurrentProcess(process: *mut ProcessSerialNumber) -> i16;
}

/// Activates the app with only the Picture in Picture window in front: the other windows of the app stay behind.
pub fn activate_with_only_picture_in_picture(controller: &Controller, move_window: bool) {
    let window = &controller.window;
    let mtm = controller.mtm;
    if shared_application(mtm).isActive() {
        return;
    }
    count_activation();
    let activation_stamp = begin_stamp();
    let source = controller.view.borrow().clone();
    let became_active = Rc::new(Cell::new(false));
    let seen = Rc::clone(&became_active);
    // SAFETY: the block takes AppKit's notification on the main thread.
    let watch = unsafe {
        NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
            Some(NSApplicationDidBecomeActiveNotification),
            None,
            None,
            &RcBlock::new(move |_: NonNull<NSNotification>| seen.set(true)),
        )
    };
    let report_after = if background() { 1.5 } else { 0.5 };
    after(report_after, move || {
        // SAFETY: `watch` is the observer token that the center returned.
        unsafe { NSNotificationCenter::defaultCenter().removeObserver(watch.as_ref()) };
        drop(watch);
        if !shared_application(mtm).isActive() {
            end_active_only_for_picture_in_picture();
        }
        if let Some(source) = source.and_then(|source| source.upgrade()) {
            let activated = became_active.get() || shared_application(mtm).isActive();
            source.emit(
                "pictureInPictureActivated",
                stamp_onto(
                    &activation_stamp,
                    json!({"move": move_window, "activated": activated}),
                ),
            );
        }
    });
    if background() {
        return log_activation(
            "activate with only Picture in Picture in front (skipped in a test instance)",
        );
    }
    watch_active_only_for_picture_in_picture();
    ACTIVE_ONLY_FOR_PICTURE_IN_PICTURE.with(|active| active.set(true));
    mark_back_to_tab_activation();
    type BringsOneWindowForward = unsafe extern "C" fn(*mut ProcessSerialNumber, u32, u32) -> i32;
    const USER_GENERATED: u32 = 0x200;
    // SAFETY: `dlsym` looks up the name in the loaded images.
    let bring_forward: *mut c_void = unsafe {
        libc::dlsym(
            libc::RTLD_DEFAULT,
            c"_SLPSSetFrontProcessWithOptions".as_ptr(),
        )
    };
    let mut process = ProcessSerialNumber { high: 0, low: 0 };
    // SAFETY: `GetCurrentProcess` writes the serial number of this process.
    let known = unsafe { GetCurrentProcess(&mut process) } == 0;
    let brought = known && !bring_forward.is_null() && {
        // SAFETY: the private SkyLight function has this signature, as in Ardia.
        let bring_forward: BringsOneWindowForward = unsafe { std::mem::transmute(bring_forward) };
        // SAFETY: `process` is a valid serial number.
        unsafe { bring_forward(&mut process, window.windowNumber() as u32, USER_GENERATED) == 0 }
    };
    if !brought {
        consume_back_to_tab_activation();
        return end_active_only_for_picture_in_picture();
    }
    window.makeKeyWindow();
}

/// `ActivateApp`: activates the app and returns whether it came from another app.
pub fn activate_app(why: &str) -> bool {
    count_activation();
    let app = shared_application(main_thread());
    let from_other_app = !app.isActive();
    if background() {
        log_activation(&format!(
            "activateIgnoringOtherApps (skipped in a test instance): {why}"
        ));
        return from_other_app;
    }
    mark_back_to_tab_activation();
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);
    from_other_app
}

/// `ardia_pip::WatchZoom`.
pub fn watch_zoom() {
    install_zoom_monitor();
}

/// `ardia_pip::GrabbedWindow`: the window whose background the user drags now.
pub fn grabbed_window() -> Option<Retained<NSWindow>> {
    GRAB.with(|grab| grab.borrow().dragged.upgrade())
        .map(|controller| controller.window.clone())
}

/// `ardia_pip::TabRequestedActivation`: true when the page is in a visible Picture in Picture window.
pub fn tab_requested_activation(view: &Rc<dyn PipPage>, from_other_app: &mut bool) -> bool {
    *from_other_app = false;
    for window in windows(main_thread()) {
        let shows = Controller::for_window(&window)
            .and_then(|controller| controller.view())
            .is_some_and(|shown| std::ptr::addr_eq(Rc::as_ptr(&shown), Rc::as_ptr(view)));
        if shows && window.isVisible() {
            mark_back_to_tab_activation();
            if window.hasShadow() {
                continue;
            }
            *from_other_app = activate_app("Back to Tab (Chrome's button)");
            return true;
        }
    }
    false
}

/// `ardia_pip::VideoEntered`: looks for Chrome's video window every 50 ms, 20 times at most.
pub fn video_entered(view: &Rc<dyn PipPage>, host: &str, stamp: &Value) {
    look_for_video(Rc::downgrade(view), host.to_owned(), stamp.clone(), 0);
}

fn look_for_video(view: Weak<dyn PipPage>, host: String, stamp: Value, tries: u32) {
    let Some(strong) = view.upgrade() else {
        return;
    };
    if attach(main_thread(), &strong, &host, &stamp) || tries + 1 >= 20 {
        return;
    }
    drop(strong);
    after(0.05, move || look_for_video(view, host, stamp, tries + 1));
}

/// `ardia_pip::DocumentEntered`: looks for Chrome's document window every 50 ms, 20 times at most.
pub fn document_entered(view: &Rc<dyn PipPage>, stamp: &Value) {
    look_for_document(Rc::downgrade(view), stamp.clone(), 0);
}

fn look_for_document(view: Weak<dyn PipPage>, stamp: Value, tries: u32) {
    let strong = view.upgrade();
    let found = windows(main_thread()).into_iter().rfind(|window| {
        window.isVisible()
            && is_chrome_document_picture_in_picture(window)
            && is_chrome_floating_window(window)
    });
    if let (Some(strong), Some(found)) = (&strong, &found) {
        join_full_screen_spaces(found);
        strong.emit(
            "pictureInPicture",
            stamp_onto(
                &stamp,
                json!({"kind": "document", "active": true, "fullScreenSpaces": joins_full_screen_spaces(found), "leftApp": strong.left_app_in_picture_in_picture(), "leftTab": !strong.seen_by_user()}),
            ),
        );
    }
    if strong.is_none() || found.is_some() || tries + 1 >= 20 {
        return;
    }
    drop(strong);
    after(0.05, move || look_for_document(view, stamp, tries + 1));
}

/// `+[ArdiaWebView devPictureInPicture]`: the state of every window that is or may be Picture in Picture.
pub fn dev_picture_in_picture() -> Vec<Value> {
    let mtm = main_thread();
    let mut list = Vec::new();
    for window in windows(mtm) {
        let controller = Controller::for_window(&window);
        if controller.is_none()
            && !(window.isVisible() && window.level() >= NSFloatingWindowLevel)
            && !is_chrome_document_picture_in_picture(&window)
        {
            continue;
        }
        let content = window.contentView().as_deref().and_then(layer_of);
        let content = content.as_deref();
        let controller = controller.as_deref();
        // SAFETY: `accessibilitySubrole` returns an NSString or nil.
        let subrole: Option<Retained<NSString>> =
            unsafe { msg_send![&window, accessibilitySubrole] };
        let rim = controller.and_then(|controller| layer_of(&controller.overlay()));
        let visible_frame = screen_for(mtm, window.frame())
            .map_or(objc2_foundation::NSRect::ZERO, |screen| {
                screen.visibleFrame()
            });
        let mut entry = json!({
            "window": window.windowNumber(),
            "class": class_name(&window),
            "title": window.title().to_string(),
            "video": is_chrome_video_picture_in_picture(&window),
            "document": is_chrome_document_picture_in_picture(&window),
            "frame": rect_string(window.frame()),
            "visible": window.isVisible(),
            "alpha": window.alphaValue(),
            "ignoresMouseEvents": window.ignoresMouseEvents(),
            "level": window.level(),
            "subrole": ns_string_or_empty(subrole),
            "canHide": window.canHide(),
            "collectionBehavior": window.collectionBehavior().0,
            "joinsFullScreenSpaces": joins_full_screen_spaces(&window),
            "styled": controller.is_some(),
        });
        let more = json!({
            "rounded": controller.is_some_and(|controller| corner_radius_of(content) == controller.corner_radius() && masks_to_bounds(content) && !window.isOpaque()),
            "cornerRadius": corner_radius_of(content),
            "rim": border_width_of(rim.as_deref()),
            "keepOnTop": controller.is_some_and(Controller::keep_on_top),
            "stashed": controller.map_or(0, |controller| controller.stashed_edge() as i64),
            "key": window.isKeyWindow(),
            "host": controller.map_or_else(String::new, Controller::host),
            "backToTab": controller.and_then(Controller::view).is_some(),
            "minSize": size_string(window.contentMinSize()),
            "maxSize": size_string(window.contentMaxSize()),
            "resizable": resizable(&window),
            "visibleFrame": rect_string(visible_frame),
        });
        if let (Some(entry), Value::Object(more)) = (entry.as_object_mut(), more) {
            entry.extend(more);
        }
        list.push(entry);
    }
    list
}

/// `+[ArdiaWebView devPictureInPictureAction:]`: runs a dev action on the last visible Picture in Picture window.
pub fn dev_picture_in_picture_action(action: &str) -> String {
    let mut controller = None;
    for window in windows(main_thread()) {
        if let Some(found) = Controller::for_window(&window)
            && window.isVisible()
        {
            controller = Some(found);
        }
    }
    let Some(controller) = controller else {
        return String::new();
    };
    let list =
        |rest: &str, separator: char| rest.split(separator).map(str::to_owned).collect::<Vec<_>>();
    if action == "selftest" {
        controller.run_self_test();
    } else if matches!(action, "close" | "backToTab") {
        controller.run_button_self_test(action);
    } else if action == "menu" {
        return controller
            .menu()
            .itemArray()
            .iter()
            .map(|item| item.title().to_string())
            .collect::<Vec<_>>()
            .join("|");
    } else if let Some(name) = action.strip_prefix("snapshot:") {
        return controller.snapshot(name);
    } else if let Some(frame) = action.strip_prefix("frame:") {
        controller
            .window
            .setFrame_display(NSRectFromString(&NSString::from_str(frame)), true);
    } else if let Some(alpha) = action.strip_prefix("alpha:") {
        return dev_show_alpha(&controller.window, NSString::from_str(alpha).floatValue());
    } else if let Some(rest) = action.strip_prefix("mouse:") {
        controller.dev_mouse(&list(rest, ':'));
    } else if action == "stash" {
        controller.stash_at(Edge::Right);
    } else if action == "unstash" {
        controller.unstash();
    } else if action == "keepOnTop" {
        controller.set_keep_on_top(!controller.keep_on_top());
    } else if action == "handle" {
        let handle = controller.overlay().handle();
        handle.set_hovered(!handle.hovered());
    } else if let Some(rest) = action.strip_prefix("swipe:") {
        controller.dev_swipe(&list(rest, ','));
    } else if let Some(rest) = action.strip_prefix("drag:") {
        controller.dev_drag(&list(rest, ','));
    } else if action == "layers" {
        return dev_layer_tree(&controller.window);
    } else if action == "framing" {
        return dev_framing(&controller);
    } else if action == "throw" {
        return dev_json(&controller.throw_info());
    } else if action == "skip" {
        return dev_json(&controller.skip_info());
    } else if action == "resizeCursor" {
        return dev_json(&controller.resize_cursor().probe());
    } else if let Some(rest) = action.strip_prefix("pinch:") {
        Zoom::dev_pinch(&controller.window, &list(rest, ','));
    } else if action == "pinchStats" {
        return dev_json(&Zoom::dev_pinch_stats(&controller.window));
    } else if action == "standIn" {
        return Zoom::dev_stand_in_window(&controller.window);
    } else if let Some(state) = action.strip_prefix("resizeCursor:") {
        controller.resize_cursor().set_enabled(state == "on");
    } else if action == "interactive" {
        if !test_instance() {
            return "test instances only".into();
        }
        let window = &controller.window;
        // The class's own setter, as `class_getMethodImplementation` finds it.
        if let Some(method) = window.class().instance_method(sel!(setAlphaValue:)) {
            // SAFETY: `setAlphaValue:` takes one CGFloat and returns nothing.
            unsafe {
                let set_alpha: unsafe extern "C" fn(&NSWindow, Sel, f64) =
                    std::mem::transmute(method.implementation());
                set_alpha(window, sel!(setAlphaValue:), 1.0);
            }
        }
        window.setIgnoresMouseEvents(false);
        return dev_show_alpha(window, 1.0);
    } else if let Some(seconds) = action.strip_prefix("skip:") {
        let frame = controller.window.frame();
        let centre = NSPoint::new(width(frame) / 2.0, frame.size.height / 2.0);
        return if controller.skip_by(NSString::from_str(seconds).doubleValue(), centre, 0) {
            "started"
        } else {
            "no page"
        }
        .into();
    } else {
        return String::new();
    }
    "started".into()
}

/// The category `ArdiaChromeWindowHost (PictureInPicture)`: class methods, so associated functions here.
impl ChromeWindowHost {
    pub fn is_video_picture_in_picture_window(window: &NSWindow) -> bool {
        Controller::for_window(window).is_some()
    }

    pub fn consume_picture_in_picture_activation() -> bool {
        consume_back_to_tab_activation()
    }

    pub fn active_only_for_picture_in_picture() -> bool {
        active_only_for_picture_in_picture()
    }

    pub fn activate_for_picture_in_picture_return() -> bool {
        activate_app("Back to Tab (WebKit Picture in Picture)")
    }
}
