// Ported from ardia packages/ardiacore/ios/ArdiaChromeWindowHost+Dev.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::OnceCell;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::Once;
use std::time::Duration;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Bool, Sel};
use objc2::{MainThreadOnly, Message, ffi, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSColor, NSEvent, NSEventMask, NSEventModifierFlags,
    NSEventType, NSResponder, NSUserInterfaceItemIdentification, NSView, NSWindow, NSWindowButton,
    NSWindowOrderingMode, NSWindowStyleMask, NSWorkspace,
};
use objc2_foundation::{
    NSArray, NSDate, NSDefaultRunLoopMode, NSInsetRect, NSMapTable, NSNotificationCenter,
    NSObjectProtocol, NSPoint, NSProcessInfo, NSRange, NSString,
};
use ria_engine::call::main_queue;
use serde_json::json;

use super::ardia_chrome_window_host::ChromeWindowHost;
use super::ardia_interface_scale::root_of_window;
use super::ardia_reserved_keys::reserved_menu_item;
use super::ardia_traffic_lights::frame_view;
use super::ardia_window_controller::Controllers;
use super::ardia_window_full_screen::{
    ACTED_OCCLUSION_GAP, act_occlusion, set_acted_transition, transition_duration,
};

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSetFlags(event: *const c_void, flags: u64);
    fn CGEventSetIntegerValueField(event: *const c_void, field: u32, value: i64);
}

#[derive(Default)]
pub(super) struct DevActions {
    covers: OnceCell<Retained<NSMapTable<NSWindow, NSWindow>>>,
}

fn flag(value: bool) -> String {
    u8::from(value).to_string()
}

fn describe_view(mut view: Option<Retained<NSView>>) -> String {
    let mut chain = String::new();
    for _ in 0..6 {
        let Some(current) = view else { break };
        if !chain.is_empty() {
            chain.push_str(" < ");
        }
        chain.push_str(&current.class().name().to_string_lossy());
        view = unsafe { current.superview() };
    }
    chain
}

fn describe_first_responder(responder: Option<Retained<NSResponder>>) -> String {
    let Some(responder) = responder else {
        return String::new();
    };
    match responder.downcast::<NSView>() {
        Ok(view) => describe_view(Some(view)),
        Err(responder) => responder.class().name().to_string_lossy().into_owned(),
    }
}

fn window_point(host: &ChromeWindowHost, window: &NSWindow, spec: &str) -> NSPoint {
    let mut fields = spec.split(',');
    match (fields.next(), fields.next()) {
        (Some(x), Some(y)) => {
            let x = NSString::from_str(x).doubleValue();
            let y = NSString::from_str(y).doubleValue();
            if let Some(root) = root_of_window(window).or_else(|| host.root_view(window)) {
                let bounds = root.bounds();
                let y = if root.isFlipped() {
                    y
                } else {
                    bounds.size.height - y
                };
                root.convertPoint_toView(
                    NSPoint::new(bounds.origin.x + x, bounds.origin.y + y),
                    None,
                )
            } else {
                NSPoint::new(
                    x,
                    frame_view(window).map_or(0., |frame| frame.bounds().size.height) - y,
                )
            }
        }
        _ => NSPoint::new(-1., -1.),
    }
}

fn hit(window: &NSWindow, point: NSPoint) -> Option<Retained<NSView>> {
    frame_view(window)?.hitTest(point)
}

fn mouse_event(
    window: &NSWindow,
    kind: NSEventType,
    point: NSPoint,
    count: isize,
    number: isize,
    pressure: f32,
) -> Option<Retained<NSEvent>> {
    NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
        kind, point, NSEventModifierFlags::empty(), NSProcessInfo::processInfo().systemUptime(),
        window.windowNumber(), None, number, count, pressure,
    )
}

fn with_mouse_fields(
    event: Retained<NSEvent>,
    flags: Option<u64>,
    other: bool,
) -> Option<Retained<NSEvent>> {
    if flags.is_none() && !other {
        return Some(event);
    }
    let cg = event.CGEvent()?;
    let pointer = std::ptr::from_ref(&*cg).cast();
    // SAFETY: `cg` holds the event alive while Core Graphics updates its fields.
    unsafe {
        if let Some(flags) = flags.filter(|flags| *flags != 0) {
            CGEventSetFlags(pointer, flags);
        }
        if other {
            const MOUSE_EVENT_BUTTON_NUMBER: u32 = 3;
            const CENTER_BUTTON: i64 = 2;
            CGEventSetIntegerValueField(pointer, MOUSE_EVENT_BUTTON_NUMBER, CENTER_BUTTON);
        }
    }
    NSEvent::eventWithCGEvent(&cg)
}

fn click(host: &ChromeWindowHost, window: &NSWindow, spec: &str) -> String {
    let point = window_point(host, window, spec);
    let target = hit(window, point);
    let reply = describe_view(target.clone());
    let page = target.as_ref().is_some_and(|target| {
        AnyClass::get(c"RenderWidgetHostViewCocoa").is_some_and(|class| target.isKindOfClass(class))
    });
    let right = spec.ends_with(",right");
    let (down, up) = if right {
        (NSEventType::RightMouseDown, NSEventType::RightMouseUp)
    } else {
        (NSEventType::LeftMouseDown, NSEventType::LeftMouseUp)
    };
    let window = window.retain();
    main_queue::post(Box::new(move || {
        let Some(down) = mouse_event(&window, down, point, 1, 0, 1.) else {
            return;
        };
        let Some(up) = mouse_event(&window, up, point, 1, 0, 1.) else {
            return;
        };
        if let Some(target) = target.filter(|_| page) {
            if right {
                target.rightMouseDown(&down);
                target.rightMouseUp(&up);
            } else {
                target.mouseDown(&down);
                target.mouseUp(&up);
            }
        } else {
            window.sendEvent(&down);
            window.sendEvent(&up);
        }
    }));
    reply
}

fn mouse(host: &ChromeWindowHost, window: &NSWindow, spec: &str) -> String {
    use NSEventType as Event;
    let mut fields = spec.split(':');
    let (Some(kind), Some(at)) = (fields.next(), fields.next()) else {
        return "mouse:<down|up|dragged|moved|rightdown|rightup|otherdown|otherup>:<x>,<y>[,<clickCount>[,<flags>[,<eventNumber>]]]".into();
    };
    let kind = match kind {
        "down" => Event::LeftMouseDown,
        "up" => Event::LeftMouseUp,
        "dragged" => Event::LeftMouseDragged,
        "moved" => Event::MouseMoved,
        "rightdown" => Event::RightMouseDown,
        "rightup" => Event::RightMouseUp,
        "otherdown" => Event::OtherMouseDown,
        "otherup" => Event::OtherMouseUp,
        _ => return "unknown mouse event".into(),
    };
    let mut fields = at.split(',').skip(2);
    let count = fields
        .next()
        .map_or((kind != Event::MouseMoved) as isize, |n| {
            NSString::from_str(n).integerValue()
        });
    let flags = fields
        .next()
        .map(|n| NSString::from_str(n).longLongValue() as u64);
    let number = fields
        .next()
        .map_or(0, |n| NSString::from_str(n).integerValue());
    let pressure = if matches!(
        kind,
        Event::LeftMouseUp | Event::RightMouseUp | Event::MouseMoved
    ) {
        0.
    } else {
        1.
    };
    let point = window_point(host, window, at);
    let other = matches!(kind, Event::OtherMouseDown | Event::OtherMouseUp);
    if let Some(event) = mouse_event(window, kind, point, count, number, pressure)
        .and_then(|event| with_mouse_fields(event, flags, other))
    {
        NSApplication::sharedApplication(window.mtm()).postEvent_atStart(&event, false);
    }
    describe_view(hit(window, point))
}

fn drag(host: &ChromeWindowHost, window: &NSWindow, spec: &str) -> String {
    const USAGE: &str = "drag:[flags=<modifier flags>;]<x>,<y>;<x>,<y>[;…]";
    let (flags, points) = if let Some(prefix) = spec.strip_prefix("flags=") {
        let Some((value, points)) = prefix.split_once(';') else {
            return USAGE.into();
        };
        (NSString::from_str(value).longLongValue() as u64, points)
    } else {
        (0, spec)
    };
    let mut path = Vec::new();
    for point in points.split(';') {
        if point.starts_with("flags=") {
            return USAGE.into();
        }
        path.push(window_point(host, window, point));
    }
    let [start, .., end] = path.as_slice() else {
        return USAGE.into();
    };
    let app = NSApplication::sharedApplication(window.mtm());
    let mut index = 0;
    let mut schedule = |kind, point| {
        let delay = if index == 0 { 0 } else { 150 + index * 16 };
        index += 1;
        if let Some(event) = mouse_event(window, kind, point, 1, 0, 1.)
            .and_then(|event| with_mouse_fields(event, (flags != 0).then_some(flags), false))
        {
            let app = app.clone();
            main_queue::after(
                Duration::from_millis(delay),
                Box::new(move || app.sendEvent(&event)),
            );
        }
    };
    schedule(NSEventType::LeftMouseDown, *start);
    for pair in path.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        for step in 1..=12 {
            let t = f64::from(step) / 12.;
            schedule(
                NSEventType::LeftMouseDragged,
                NSPoint::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t),
            );
        }
    }
    schedule(NSEventType::LeftMouseUp, *end);
    format!("{} ({index} events)", describe_view(hit(window, *start)))
}

fn fake_active_clicks() -> String {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        let selector = sel!(acceptsFirstMouse:);
        for class in [c"NSTextView", c"RenderWidgetHostViewCocoa", c"WKWebView"]
            .into_iter()
            .filter_map(AnyClass::get)
        {
            let Some(method) = class.instance_method(selector) else {
                continue;
            };
            // SAFETY: The selector takes an NSEvent and returns BOOL on NSView subclasses.
            let original: unsafe extern "C-unwind" fn(&NSView, Sel, Option<&NSEvent>) -> Bool =
                unsafe { std::mem::transmute(method.implementation()) };
            let callback = RcBlock::new(move |view: NonNull<NSView>, event: *const NSEvent| {
                // SAFETY: AppKit passes live arguments to this method on the main thread.
                let view = unsafe { view.as_ref() };
                if view.window().is_some_and(|window| window.isKeyWindow()) {
                    Bool::YES
                } else {
                    unsafe { original(view, selector, event.as_ref()) }
                }
            });
            // SAFETY: The runtime copies the block. Replace on this class, not its superclass.
            unsafe {
                let imp = ffi::imp_implementationWithBlock(RcBlock::as_ptr(&callback).cast());
                ffi::class_replaceMethod(
                    std::ptr::from_ref(class).cast_mut(),
                    selector,
                    imp,
                    ffi::method_getTypeEncoding(method),
                );
            }
        }
    });
    "1".into()
}

fn post_notification(window: &NSWindow, value: &str) -> String {
    let Some((scope, name)) = value
        .split_once(':')
        .filter(|(_, name)| !name.contains(':'))
    else {
        return "postNotification:<workspace|app|window|windows>:<name>".into();
    };
    let app = NSApplication::sharedApplication(window.mtm());
    let name = NSString::from_str(name);
    let center = NSNotificationCenter::defaultCenter();
    // SAFETY: All notification senders are live AppKit objects on the main thread.
    unsafe {
        match scope {
            "workspace" => {
                let workspace = NSWorkspace::sharedWorkspace();
                workspace
                    .notificationCenter()
                    .postNotificationName_object(&name, Some(&workspace));
            }
            "app" => center.postNotificationName_object(&name, Some(&app)),
            "window" => center.postNotificationName_object(&name, Some(window)),
            "windows" => {
                for each in app.windows().iter() {
                    center.postNotificationName_object(&name, Some(&each));
                }
            }
            _ => return "unknown center".into(),
        }
    }
    "posted".into()
}

fn subroles(window: &NSWindow) -> String {
    let app = NSApplication::sharedApplication(window.mtm());
    let windows: Vec<_> = app
        .orderedWindows()
        .iter()
        .filter(|window| window.isVisible() && window.parentWindow().is_none())
        .map(|window| {
            // SAFETY: The window implements AppKit's accessibility subrole selector.
            let subrole: Option<Retained<NSString>> =
                unsafe { msg_send![&*window, accessibilitySubrole] };
            json!({
                "number": window.windowNumber(),
                "title": window.title().to_string(),
                "identifier": window.identifier().map_or_else(String::new, |id| id.to_string()),
                "subrole": subrole.map_or_else(String::new, |value| value.to_string()),
            })
        })
        .collect();
    json!(windows).to_string()
}

fn accessibility_tree(window: &NSWindow) -> String {
    fn walk(element: &AnyObject, depth: usize, lines: &mut Vec<String>) {
        if lines.len() > 400 || depth > 40 {
            return;
        }
        // SAFETY: The elements conform to the Objective-C object protocol.
        let responds = |selector| unsafe {
            let supports: Bool = msg_send![element, respondsToSelector: selector];
            supports.as_bool()
        };
        let mut line = " ".repeat(depth);
        let role = sel!(accessibilityRole);
        if responds(role) {
            // SAFETY: AppKit declares this selector as returning an NSString.
            let role: Option<Retained<NSString>> = unsafe { msg_send![element, accessibilityRole] };
            if let Some(role) = role {
                line.push_str(&role.to_string());
            }
        } else {
            line.push('?');
        }
        let mut separator = ": ";
        for selector in [
            sel!(accessibilityTitle),
            sel!(accessibilityLabel),
            sel!(accessibilityValue),
        ] {
            if !responds(selector) {
                continue;
            }
            // SAFETY: These AppKit accessibility getters return objects.
            let value: Option<Retained<AnyObject>> =
                unsafe { msg_send![element, performSelector: selector] };
            if let Some(text) = value
                .and_then(|value| value.downcast::<NSString>().ok())
                .filter(|text| !text.is_empty())
            {
                line.push_str(separator);
                line.push_str(&text.to_string());
                separator = " | ";
            }
        }
        lines.push(line);
        if responds(sel!(accessibilityChildren)) {
            // SAFETY: AppKit declares this selector as returning an array of accessibility objects.
            let children: Option<Retained<NSArray<AnyObject>>> =
                unsafe { msg_send![element, accessibilityChildren] };
            if let Some(children) = children {
                for child in children.iter() {
                    walk(&child, depth + 1, lines);
                    if lines.len() > 400 {
                        break;
                    }
                }
            }
        }
    }
    let mut lines = Vec::new();
    walk(window, 0, &mut lines);
    lines.join("\n")
}

fn type_text(window: &NSWindow, text: &str) -> String {
    let text = NSString::from_str(text);
    for index in 0..text.length() {
        let unit = text.substringWithRange(NSRange::new(index, 1));
        for kind in [NSEventType::KeyDown, NSEventType::KeyUp] {
            if let (Some(responder), Some(event)) = (
                window.firstResponder(),
                key_event(window, kind, NSEventModifierFlags::empty(), &unit, 0),
            ) {
                if kind == NSEventType::KeyDown {
                    responder.keyDown(&event);
                } else {
                    responder.keyUp(&event);
                }
            }
        }
    }
    describe_first_responder(window.firstResponder())
}

fn key_event(
    window: &NSWindow,
    kind: NSEventType,
    flags: NSEventModifierFlags,
    chars: &NSString,
    code: u16,
) -> Option<Retained<NSEvent>> {
    NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
        kind, NSPoint::ZERO, flags, NSProcessInfo::processInfo().systemUptime(),
        window.windowNumber(), None, chars, chars, false, code,
    )
}

fn key_action(controllers: &Controllers, window: &NSWindow, name: &str, value: &str) -> String {
    let mut fields = value.split(':');
    let (Some(flags), Some(chars)) = (fields.next(), fields.next()) else {
        return format!("{name}:<flags>:<characters>[:<keyCode>]");
    };
    let flags =
        NSEventModifierFlags::from_bits_retain(NSString::from_str(flags).longLongValue() as usize);
    let chars = NSString::from_str(chars);
    let code = fields
        .next()
        .map_or(0, |code| NSString::from_str(code).intValue() as u16);
    let event = key_event(window, NSEventType::KeyDown, flags, &chars, code);
    let app = NSApplication::sharedApplication(window.mtm());
    if name == "reserved" {
        return event
            .and_then(|event| {
                let context = event
                    .window(window.mtm())
                    .and_then(|window| controllers.for_window(&window))
                    .map_or_else(Default::default, |controller| controller.key_context());
                reserved_menu_item(&app, &event, context)
            })
            .and_then(|item| item.identifier())
            .map_or_else(String::new, |id| id.to_string());
    }
    let down = event.and_then(|event| {
        app.postEvent_atStart(&event, true);
        app.nextEventMatchingMask_untilDate_inMode_dequeue(
            NSEventMask::KeyDown,
            Some(&NSDate::distantPast()),
            unsafe { NSDefaultRunLoopMode },
            true,
        )
    });
    let Some(down) = down else {
        return "keys: the key event didn't come back".into();
    };
    let handler = if window.performKeyEquivalent(&down) {
        "window"
    } else if app
        .mainMenu()
        .is_some_and(|menu| menu.performKeyEquivalent(&down))
    {
        "mainMenu"
    } else {
        if let Some(responder) = window.firstResponder() {
            responder.keyDown(&down);
        }
        "keyDown"
    };
    format!(
        "{handler} (first responder {})",
        describe_first_responder(window.firstResponder())
    )
}

impl DevActions {
    pub(super) fn run(
        &self,
        host: &ChromeWindowHost,
        controllers: &Controllers,
        window: &NSWindow,
        action: &str,
    ) -> Option<String> {
        let (name, argument) = action
            .split_once(':')
            .map_or((action, None), |(name, argument)| (name, Some(argument)));
        let value = argument.unwrap_or_default();
        match name {
            "fakeFullScreenMs" => {
                let transition = set_acted_transition(value);
                Some(format!("{:.0}", transition * 1000.))
            }
            "fakeFullScreenOcclusionMs" => {
                let gap = NSString::from_str(value).doubleValue();
                ACTED_OCCLUSION_GAP.set((gap.is_finite() && gap != -1.).then_some(gap / 1000.));
                Some(format!("{gap:.0}"))
            }
            "fullScreen" | "fakeFullScreen" => {
                let Some(controller) = controllers.for_window(window) else {
                    return Some("not an ArdiaCore window".into());
                };
                let fullscreen = &controller.fullscreen;
                if name == "fakeFullScreen"
                    && let Some(argument) = argument
                {
                    let mut fields = argument.split(':');
                    let entering = fields.next() == Some("1");
                    let transition = fields.next().map_or(0., transition_duration);
                    fullscreen.act_window_full_screen_with_timing(entering, transition);
                }
                let full_screen = fullscreen.is_full_screen();
                let mut reply = json!({
                    "fullScreen": full_screen,
                    "styleMask": window.styleMask().contains(NSWindowStyleMask::FullScreen),
                    "acted": controller.core.acted_full_screen(),
                    "chromeCounts": controller.core.acted_full_screen(),
                });
                if let Some(page_state) = fullscreen.page_state(full_screen) {
                    reply["pageFullScreen"] = page_state;
                }
                Some(reply.to_string())
            }
            "fakeOcclusion" => {
                let controller = controllers.for_window(window);
                Some(flag(act_occlusion(
                    window,
                    controller.as_deref().map(|controller| &*controller.fullscreen),
                    (value != "off").then_some(value == "visible"),
                )))
            }
            "fakeWindowServer" => {
                controllers
                    .for_window(window)?
                    .fullscreen
                    .act_window_server(value == "shows");
                Some("ok".into())
            }
            "focusState" => Some(host.focus_state(window.mtm())),
            "hit" => Some(describe_view(hit(
                window,
                window_point(host, window, value),
            ))),
            "click" => Some(click(host, window, value)),
            "mouse" => Some(mouse(host, window, value)),
            "drag" => Some(drag(host, window, value)),
            "fakeActiveClicks" => Some(fake_active_clicks()),
            "postNotification" => Some(post_notification(window, value)),
            "subroles" => Some(subroles(window)),
            "ax" => Some(accessibility_tree(window)),
            "type" => Some(type_text(window, value)),
            "reserved" | "keys" => Some(key_action(controllers, window, name, value)),
            "performClose" => {
                let window = window.retain();
                main_queue::post(Box::new(move || window.performClose(None)));
                Some("ok".into())
            }
            "cover" | "uncover" => Some(self.cover(window, name == "cover")),
            "shadow" => {
                window.setHasShadow(value == "1");
                Some("ok".into())
            }
            "ns" => {
                if value == "out" {
                    window.orderOut(None)
                } else {
                    window.orderFront(None)
                };
                Some(format!("visible={}", flag(window.isVisible())))
            }
            "root" => {
                host.root_view(window)?.setHidden(value == "hide");
                Some("ok".into())
            }
            "opaque" => {
                window.setOpaque(value == "1");
                if !window.isOpaque() {
                    window.setBackgroundColor(Some(&NSColor::clearColor()));
                }
                Some(format!("opaque={}", flag(window.isOpaque())))
            }
            "lights" => {
                let button = window.standardWindowButton(NSWindowButton::CloseButton)?;
                let frame = button.convertRect_toView(button.bounds(), None);
                Some(format!(
                    "{:.1},{:.1},{:.1},{:.1}",
                    frame.origin.x, frame.origin.y, frame.size.width, frame.size.height
                ))
            }
            _ => None,
        }
    }

    fn cover(&self, window: &NSWindow, covered: bool) -> String {
        if !covered {
            if let Some(covers) = self.covers.get()
                && let Some(cover) = covers.objectForKey(Some(window))
            {
                cover.orderOut(None);
                covers.removeObjectForKey(Some(window));
            }
            return "uncovered".into();
        }
        let covers = self
            .covers
            .get_or_init(NSMapTable::weakToStrongObjectsMapTable);
        let cover = covers.objectForKey(Some(window)).unwrap_or_else(|| {
            let cover = unsafe {
                let cover = NSWindow::initWithContentRect_styleMask_backing_defer(
                    NSWindow::alloc(window.mtm()),
                    NSInsetRect(window.frame(), -4., -4.),
                    NSWindowStyleMask::Borderless,
                    NSBackingStoreType::Buffered,
                    false,
                );
                cover.setReleasedWhenClosed(false);
                cover
            };
            cover.setOpaque(true);
            cover.setBackgroundColor(Some(&NSColor::windowBackgroundColor()));
            cover.setIgnoresMouseEvents(true);
            covers.setObject_forKey(Some(&cover), Some(window));
            cover
        });
        cover.orderWindow_relativeTo(NSWindowOrderingMode::Above, window.windowNumber());
        NSString::from_rect(cover.frame()).to_string()
    }
}
