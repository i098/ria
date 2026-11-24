// Ported from ardia packages/ardiacore/ios/ArdiaPiPController+SelfTest.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The self-tests and dev mouse actions that the app ships for the Picture in Picture window.

use std::cell::RefCell;
use std::ffi::c_void;
use std::path::PathBuf;
use std::rc::Rc;

use objc2::rc::{Retained, Weak};
use objc2::runtime::AnyObject;
use objc2::{AnyThread, ClassType, msg_send};
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSControlStateValueOff, NSControlStateValueOn,
    NSDeviceRGBColorSpace, NSEvent, NSEventModifierFlags, NSEventType, NSFloatingWindowLevel,
    NSGraphicsContext, NSNormalWindowLevel, NSView, NSWindow,
};
use objc2_foundation::{NSArray, NSDictionary, NSPoint, NSProcessInfo, NSString, ns_string};
use serde_json::{Map, Value, json};

use super::ardia_picture_in_picture_internal::{
    BUTTON_MARGIN, BUTTON_SIZE, Edge, PEEK, RETURN_MARGIN, activations, after, height, max_x,
    max_y, mid_x, mid_y, min_x, min_y, point_string, prevents_activation, rect, rect_string,
    screen_for, screens, services, shared_application, width, window_root_view,
};
use super::ardia_pip_controller::Controller;

#[repr(C)]
struct CGContextRef {
    _private: [u8; 0],
}

// SAFETY: matches the runtime encoding `^{CGContext=}` of a CGContextRef.
unsafe impl objc2::Encode for CGContextRef {
    const ENCODING: objc2::Encoding = objc2::Encoding::Struct("CGContext", &[]);
}
unsafe impl objc2::RefEncode for CGContextRef {
    const ENCODING_REF: objc2::Encoding =
        objc2::Encoding::Pointer(&<Self as objc2::Encode>::ENCODING);
}

unsafe extern "C" {
    fn CGContextSetRGBFillColor(c: *mut c_void, r: f64, g: f64, b: f64, a: f64);
    fn CGContextFillRect(c: *mut c_void, rect: objc2_foundation::NSRect);
    fn CGContextScaleCTM(c: *mut c_void, sx: f64, sy: f64);
    fn CGContextTranslateCTM(c: *mut c_void, tx: f64, ty: f64);
}

/// `ArdiaLaunchConfig.shared.dataDirectory ?: @"/tmp"`.
fn data_dir() -> PathBuf {
    services().map_or_else(
        || PathBuf::from("/tmp"),
        |services| services.launch.data_directory().to_path_buf(),
    )
}

fn test_value(key: &str) -> Option<String> {
    services().and_then(|services| services.launch.test_value(key).map(str::to_owned))
}

fn write_json(path: PathBuf, value: &Value, pretty: bool) {
    let text = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    };
    if let Ok(text) = text {
        let _ = std::fs::write(path, text);
    }
}

fn layer_of(window: &NSWindow) -> Option<Retained<AnyObject>> {
    let content = window.contentView()?;
    // SAFETY: `layer` returns a CALayer or nil.
    unsafe { msg_send![&content, layer] }
}

fn corner_radius(layer: Option<&AnyObject>) -> f64 {
    // SAFETY: `cornerRadius` of a CALayer is a CGFloat.
    layer.map_or(0.0, |layer| unsafe { msg_send![layer, cornerRadius] })
}

fn masks_to_bounds(layer: Option<&AnyObject>) -> bool {
    // SAFETY: `masksToBounds` of a CALayer is a BOOL.
    layer.is_some_and(|layer| unsafe { msg_send![layer, masksToBounds] })
}

fn mouse_event(
    window: &NSWindow,
    kind: NSEventType,
    point: NSPoint,
    clicks: isize,
    pressure: f32,
) -> Option<Retained<NSEvent>> {
    NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
        kind,
        point,
        NSEventModifierFlags::empty(),
        NSProcessInfo::processInfo().systemUptime(),
        window.windowNumber(),
        None,
        0,
        clicks,
        pressure,
    )
}

fn press(kind: NSEventType) -> f32 {
    if kind == NSEventType::LeftMouseDown {
        1.0
    } else {
        0.0
    }
}

/// The leading number of the text, as `-[NSString doubleValue]` reads it (0 when none).
fn leading_number(text: &str) -> f64 {
    let text = text.trim_start();
    let end = text
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_digit() || c == '.' || (i == 0 && (c == '-' || c == '+'))))
        .map_or(text.len(), |(i, _)| i);
    (0..=end)
        .rev()
        .find_map(|n| text[..n].parse::<f64>().ok())
        .unwrap_or(0.0)
}

impl Controller {
    pub fn snapshot(&self, name: &str) -> String {
        let window = &self.window;
        let dir = data_dir();
        let path = dir.join(format!("pip-{name}.png"));
        let Some(content) = window_root_view(window) else {
            return path.to_string_lossy().into_owned();
        };
        let size = content.bounds().size;
        // SAFETY: a null plane list makes the rep allocate its own pixels.
        let rep = unsafe {
            NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
                NSBitmapImageRep::alloc(),
                std::ptr::null_mut(),
                (size.width * 2.0) as isize,
                (size.height * 2.0) as isize,
                8,
                4,
                true,
                false,
                NSDeviceRGBColorSpace,
                0,
                0,
            )
        };
        if let Some(rep) = rep {
            if let Some(context) = NSGraphicsContext::graphicsContextWithBitmapImageRep(&rep) {
                // SAFETY: `CGContext` returns the live CGContextRef of the graphics context.
                let cg: *mut CGContextRef = unsafe { msg_send![&context, CGContext] };
                let cg_raw = cg.cast::<c_void>();
                // SAFETY: `cg` stays alive while `context` does; `renderInContext:` takes a CGContextRef.
                unsafe {
                    CGContextSetRGBFillColor(cg_raw, 0.36, 0.42, 0.5, 1.0);
                    CGContextFillRect(cg_raw, rect(0.0, 0.0, size.width * 2.0, size.height * 2.0));
                    CGContextScaleCTM(cg_raw, 2.0, 2.0);
                    if content.isFlipped() {
                        CGContextTranslateCTM(cg_raw, 0.0, size.height);
                        CGContextScaleCTM(cg_raw, 1.0, -1.0);
                    }
                    let layer: Option<Retained<AnyObject>> = msg_send![&content, layer];
                    if let Some(layer) = layer {
                        let _: () = msg_send![&layer, renderInContext: cg];
                    }
                }
            }
            // SAFETY: an empty property dictionary is valid for PNG.
            let data = unsafe {
                rep.representationUsingType_properties(
                    NSBitmapImageFileType::PNG,
                    &NSDictionary::new(),
                )
            };
            if let Some(data) = data {
                data.writeToFile_atomically(&NSString::from_str(&path.to_string_lossy()), true);
            }
        }
        path.to_string_lossy().into_owned()
    }

    pub fn run_self_test(&self) {
        let Some(me) = self.me.upgrade() else {
            return;
        };
        let mtm = self.mtm;
        let steps: Rc<RefCell<Vec<Value>>> = Rc::default();
        let window = self.window.clone();
        let start = window.frame();
        let outermost = |right: bool| {
            let mut pick = screen_for(mtm, start);
            for screen in screens(mtm) {
                let f = screen.frame();
                if mid_y(start) < min_y(f) || mid_y(start) >= max_y(f) {
                    continue;
                }
                let pf = pick
                    .as_ref()
                    .map_or(rect(0.0, 0.0, 0.0, 0.0), |p| p.frame());
                if if right {
                    max_x(f) > max_x(pf)
                } else {
                    min_x(f) < min_x(pf)
                } {
                    pick = Some(screen);
                }
            }
            pick.map_or(rect(0.0, 0.0, 0.0, 0.0), |p| p.visibleFrame())
        };
        let (visible_right, visible_left) = (outermost(true), outermost(false));
        let record = {
            let (steps, window) = (Rc::clone(&steps), window.clone());
            Rc::new(move |name: &str, pass: bool, extra: Value| {
                let mut step = Map::new();
                step.insert("step".into(), json!(name));
                step.insert("pass".into(), json!(pass));
                step.insert("frame".into(), json!(rect_string(window.frame())));
                if let Value::Object(extra) = extra {
                    step.extend(extra);
                }
                let step = Value::Object(step);
                eprintln!(
                    "[pip-selftest] {name} {} {step}",
                    if pass { "PASS" } else { "FAIL" }
                );
                steps.borrow_mut().push(step);
            })
        };
        let dir = data_dir();
        let write = {
            let (me, steps, window, dir) = (
                Rc::clone(&me),
                Rc::clone(&steps),
                window.clone(),
                dir.clone(),
            );
            move || {
                let result = json!({
                    "windowNumber": window.windowNumber(),
                    "host": me.host(),
                    "steps": *steps.borrow(),
                });
                write_json(dir.join("pip-selftest.json"), &result, true);
            }
        };
        let rounded = {
            let (me, window) = (Rc::clone(&me), window.clone());
            Rc::new(move || {
                let layer = layer_of(&window);
                corner_radius(layer.as_deref()) == me.corner_radius()
                    && masks_to_bounds(layer.as_deref())
                    && !window.isOpaque()
                    && window.hasShadow()
                    && overlay_border(&me) == 1.0
            })
        };
        let click_handle = {
            let (me, window) = (Rc::clone(&me), window.clone());
            Rc::new(move || {
                let handle = me.overlay().handle();
                let bounds = handle.bounds();
                let point =
                    handle.convertPoint_toView(NSPoint::new(mid_x(bounds), mid_y(bounds)), None);
                let app = shared_application(mtm);
                for kind in [NSEventType::LeftMouseDown, NSEventType::LeftMouseUp] {
                    if let Some(event) = mouse_event(&window, kind, point, 1, press(kind)) {
                        app.postEvent_atStart(&event, false);
                    }
                }
                point_string(point)
            })
        };
        let right_click_at = Rc::new(RefCell::new(String::new()));
        let left_click_at = Rc::new(RefCell::new(String::new()));
        let drag_to = {
            let (me, window) = (Rc::clone(&me), window.clone());
            Rc::new(move |x: f64| {
                me.stop_motion();
                me.state.saw_drag.set(true);
                window.setFrameOrigin(NSPoint::new(x, min_y(window.frame())));
                me.cancel_settle();
                me.settle();
            })
        };
        let snapshot = {
            let me = Rc::clone(&me);
            Rc::new(move |name: &str| {
                me.snapshot(name);
            })
        };

        let root = window_root_view(&window);
        let overlay = me.overlay();
        let overlay_view: &NSView = &overlay;
        // SAFETY: `superview` is read on the main thread while the view is alive.
        let parent = unsafe { overlay_view.superview() };
        let attached = root.as_ref().is_some_and(|root| {
            parent
                .as_ref()
                .is_some_and(|parent| std::ptr::eq::<NSView>(&**parent, &**root))
                && root
                    .subviews()
                    .lastObject()
                    .is_some_and(|last| std::ptr::eq::<NSView>(&*last, overlay_view))
        });
        record(
            "attached",
            attached,
            json!({"level": window.level(), "keepOnTop": me.keep_on_top(), "shadow": window.hasShadow()}),
        );

        {
            let (me, window, record, rounded, snapshot, drag_to) = (
                Rc::clone(&me),
                window.clone(),
                Rc::clone(&record),
                Rc::clone(&rounded),
                Rc::clone(&snapshot),
                Rc::clone(&drag_to),
            );
            after(0.6, move || {
                let overlay = me.overlay();
                record(
                    "nothing of ours at rest",
                    overlay.handle().isHidden()
                        && overlay.hitTest(NSPoint::new(20.0, 20.0)).is_none(),
                    Value::Null,
                );
                let layer = layer_of(&window);
                record(
                    "rounded corners",
                    rounded(),
                    json!({
                        "cornerRadius": corner_radius(layer.as_deref()),
                        "masksToBounds": masks_to_bounds(layer.as_deref()),
                        "opaque": window.isOpaque(),
                        "shadow": window.hasShadow(),
                        "rim": overlay_border(&me),
                    }),
                );
                snapshot("rest");
                drag_to(max_x(visible_right) - width(window.frame()) * 0.3);
            });
        }
        {
            let (me, window, record, snapshot) = (
                Rc::clone(&me),
                window.clone(),
                Rc::clone(&record),
                Rc::clone(&snapshot),
            );
            after(2.2, move || {
                let handle = me.overlay().handle();
                record(
                    "drag past right edge stashes",
                    me.stashed_edge() == Edge::Right
                        && (min_x(window.frame()) - (max_x(visible_right) - PEEK)).abs() < 0.5
                        && !handle.isHidden()
                        && handle.alphaValue() > 0.99,
                    json!({"handle": rect_string(handle.frame())}),
                );
                snapshot("stashed-right");
                window.setFrameOrigin(NSPoint::new(
                    max_x(visible_right) - width(window.frame()) - 40.0,
                    min_y(window.frame()),
                ));
            });
        }
        {
            let (me, window, record, click_handle, right_click_at) = (
                Rc::clone(&me),
                window.clone(),
                Rc::clone(&record),
                Rc::clone(&click_handle),
                Rc::clone(&right_click_at),
            );
            after(3.4, move || {
                record(
                    "Chrome's move re-stashes",
                    me.stashed_edge() == Edge::Right
                        && (min_x(window.frame()) - (max_x(visible_right) - PEEK)).abs() < 0.5,
                    Value::Null,
                );
                *right_click_at.borrow_mut() = click_handle();
            });
        }
        {
            let (me, window, record, drag_to, right_click_at) = (
                Rc::clone(&me),
                window.clone(),
                Rc::clone(&record),
                Rc::clone(&drag_to),
                Rc::clone(&right_click_at),
            );
            after(4.6, move || {
                record(
                    "handle click brings it back",
                    me.stashed_edge() == Edge::None
                        && me.overlay().handle().isHidden()
                        && (max_x(window.frame()) - (max_x(visible_right) - RETURN_MARGIN)).abs()
                            < 0.5,
                    json!({"clickedAt": *right_click_at.borrow()}),
                );
                drag_to(min_x(visible_left) - width(window.frame()) * 0.7);
            });
        }
        {
            let (me, window, record, snapshot, click_handle, left_click_at) = (
                Rc::clone(&me),
                window.clone(),
                Rc::clone(&record),
                Rc::clone(&snapshot),
                Rc::clone(&click_handle),
                Rc::clone(&left_click_at),
            );
            after(5.8, move || {
                let overlay = me.overlay();
                let handle = overlay.handle();
                record(
                    "drag past left edge stashes",
                    me.stashed_edge() == Edge::Left
                        && (max_x(window.frame()) - (min_x(visible_left) + PEEK)).abs() < 0.5
                        && min_x(handle.frame()) == width(overlay.bounds()) - PEEK,
                    json!({"handle": rect_string(handle.frame())}),
                );
                snapshot("stashed-left");
                *left_click_at.borrow_mut() = click_handle();
            });
        }
        {
            let (me, window, record, drag_to, left_click_at) = (
                Rc::clone(&me),
                window.clone(),
                Rc::clone(&record),
                Rc::clone(&drag_to),
                Rc::clone(&left_click_at),
            );
            after(7.0, move || {
                record(
                    "left handle click brings it back",
                    me.stashed_edge() == Edge::None
                        && me.overlay().handle().isHidden()
                        && (min_x(window.frame()) - (min_x(visible_left) + RETURN_MARGIN)).abs()
                            < 0.5,
                    json!({"clickedAt": *left_click_at.borrow()}),
                );
                drag_to(min_x(visible_left) - width(window.frame()) * 0.7);
            });
        }
        {
            let (me, record, drag_to) = (Rc::clone(&me), Rc::clone(&record), Rc::clone(&drag_to));
            after(8.2, move || {
                record(
                    "drag past left edge stashes again",
                    me.stashed_edge() == Edge::Left,
                    Value::Null,
                );
                drag_to(min_x(visible_left) + 100.0);
            });
        }
        {
            let (me, window, record, dir) = (
                Rc::clone(&me),
                window.clone(),
                Rc::clone(&record),
                dir.clone(),
            );
            after(9.4, move || {
                record(
                    "drag out of the stash",
                    me.stashed_edge() == Edge::None
                        && (min_x(window.frame()) - (min_x(visible_left) + 100.0)).abs() < 0.5,
                    Value::Null,
                );
                let plist = NSString::from_str(
                    &dir.join("Chromium/ArdiaPictureInPicture.plist")
                        .to_string_lossy(),
                );
                // SAFETY: `arrayWithContentsOfFile:` returns an array or nil.
                let saved: Option<Retained<NSArray<AnyObject>>> = unsafe {
                    msg_send![NSArray::<AnyObject>::class(), arrayWithContentsOfFile: &*plist]
                };
                // SAFETY: the saved frame is a list of NSNumbers.
                let saved: Vec<i32> = saved.map_or_else(Vec::new, |saved| {
                    saved
                        .iter()
                        .map(|value| unsafe { msg_send![&value, intValue] })
                        .collect()
                });
                let frame = window.frame();
                let top = screens(me.mtm).first().map_or(0.0, |s| max_y(s.frame())) - max_y(frame);
                record(
                    "Chrome remembers where it was left",
                    saved.len() == 4
                        && saved[0] == min_x(frame) as i32
                        && saved[1] == top as i32
                        && saved[2] == width(frame) as i32
                        && saved[3] == height(frame) as i32,
                    json!({"saved": saved}),
                );
                let title = ns_string!("Keep Window on Top");
                let on_before = me
                    .menu()
                    .itemWithTitle(title)
                    .is_some_and(|item| item.state() == NSControlStateValueOn);
                me.toggle_keep_on_top();
                let off = window.level() == NSNormalWindowLevel
                    && me
                        .menu()
                        .itemWithTitle(title)
                        .is_some_and(|item| item.state() == NSControlStateValueOff);
                me.toggle_keep_on_top();
                let titles: Vec<String> = me
                    .menu()
                    .itemArray()
                    .iter()
                    .map(|item| item.title().to_string())
                    .collect();
                record(
                    "Keep Window on Top toggles",
                    on_before && off && window.level() == NSFloatingWindowLevel,
                    json!({"menu": titles}),
                );
                window.setFrame_display(objc2_foundation::NSInsetRect(start, 40.0, 22.5), true);
            });
        }
        {
            let (window, record, rounded) =
                (window.clone(), Rc::clone(&record), Rc::clone(&rounded));
            after(10.0, move || {
                record("still rounded after a resize", rounded(), Value::Null);
                window.setFrame_display(start, true);
            });
        }
        {
            let (me, record) = (Rc::clone(&me), Rc::clone(&record));
            after(10.6, move || {
                record("menu: Back to Tab", me.view().is_some(), Value::Null);
                me.back_to_tab();
            });
        }
        after(12.1, move || {
            record(
                "PiP closed after Back to Tab",
                !window.isVisible(),
                Value::Null,
            );
            write();
        });
    }

    pub fn run_button_self_test(&self, button: &str) {
        let window = self.window.clone();
        let dir = data_dir();
        let center = BUTTON_MARGIN + BUTTON_SIZE / 2.0;
        let point = NSPoint::new(
            if button == "close" {
                width(window.frame()) - center
            } else {
                center
            },
            height(window.frame()) - center,
        );
        let mtm = self.mtm;
        let button = button.to_owned();
        {
            let window = window.clone();
            after(1.5, move || {
                if let (Some(content), Some(event)) = (
                    window.contentView(),
                    mouse_event(&window, NSEventType::MouseMoved, point, 1, 0.0),
                ) {
                    content.mouseMoved(&event);
                }
            });
        }
        after(2.1, move || {
            let prevents = prevents_activation(&window);
            let before = activations();
            let app = shared_application(mtm);
            for kind in [NSEventType::LeftMouseDown, NSEventType::LeftMouseUp] {
                if let Some(event) = mouse_event(&window, kind, point, 1, press(kind)) {
                    app.postEvent_atStart(&event, false);
                }
            }
            after(1.0, move || {
                let result = json!({
                    "button": button,
                    "closed": !window.isVisible(),
                    "point": point_string(point),
                    "preventsActivation": prevents,
                    "activations": activations() - before,
                    "appActive": shared_application(mtm).isActive(),
                });
                eprintln!("[pip-selftest] {result}");
                write_json(dir.join("pip-button-selftest.json"), &result, false);
            });
        });
    }

    pub fn run_hover_self_test(&self) {
        let window = &self.window;
        let dir = data_dir();
        let place = test_value("pipHover");
        let center = BUTTON_MARGIN + BUTTON_SIZE / 2.0;
        let size = window.frame().size;
        let point = match place.as_deref() {
            Some("close") => NSPoint::new(size.width - center, size.height - center),
            Some("back") => NSPoint::new(center, size.height - center),
            _ => NSPoint::new(size.width / 2.0, size.height * 0.7),
        };
        hover_every_second(Weak::from_retained(window), point);
        let layer = layer_of(window);
        let result = json!({
            "windowNumber": window.windowNumber(),
            "frame": rect_string(window.frame()),
            "hover": place.as_deref().unwrap_or("video"),
            "cornerRadius": corner_radius(layer.as_deref()),
            "masksToBounds": masks_to_bounds(layer.as_deref()),
            "opaque": window.isOpaque(),
        });
        write_json(dir.join("pip-hover.json"), &result, false);
    }

    pub fn dev_mouse(&self, args: &[String]) {
        if args.len() != 2 && args.len() != 3 {
            return;
        }
        let clicks = if args.len() == 3 {
            (leading_number(&args[2]) as isize).max(1)
        } else {
            1
        };
        let xy: Vec<&str> = args[1].split(',').collect();
        if xy.len() != 2 {
            return;
        }
        let window = &self.window;
        let point = NSPoint::new(
            leading_number(xy[0]),
            height(window.frame()) - leading_number(xy[1]),
        );
        let kind = match args[0].as_str() {
            "moved" => NSEventType::MouseMoved,
            "down" => NSEventType::LeftMouseDown,
            "up" => NSEventType::LeftMouseUp,
            "dragged" => NSEventType::LeftMouseDragged,
            _ => return,
        };
        let pressure = f32::from(u8::from(
            kind == NSEventType::LeftMouseDown || kind == NSEventType::LeftMouseDragged,
        ));
        let Some(event) = mouse_event(window, kind, point, clicks, pressure) else {
            return;
        };
        if kind == NSEventType::MouseMoved {
            if let Some(content) = window.contentView() {
                content.mouseMoved(&event);
            }
        } else {
            shared_application(self.mtm).postEvent_atStart(&event, false);
        }
    }
}

fn overlay_border(me: &Controller) -> f64 {
    let overlay = me.overlay();
    // SAFETY: `layer` returns a CALayer or nil; `borderWidth` is a CGFloat.
    unsafe {
        let layer: Option<Retained<AnyObject>> = msg_send![&overlay, layer];
        layer.map_or(0.0, |layer| msg_send![&layer, borderWidth])
    }
}

/// The repeating one-second NSTimer of `runHoverSelfTest`: it stops once the window is gone or hidden.
fn hover_every_second(window: Weak<NSWindow>, point: NSPoint) {
    after(1.0, move || {
        let Some(strong) = window.load().filter(|window| window.isVisible()) else {
            return;
        };
        if let (Some(content), Some(event)) = (
            strong.contentView(),
            mouse_event(&strong, NSEventType::MouseMoved, point, 0, 0.0),
        ) {
            content.mouseMoved(&event);
        }
        hover_every_second(window, point);
    });
}
