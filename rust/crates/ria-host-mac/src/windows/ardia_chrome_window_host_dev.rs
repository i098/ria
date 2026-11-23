// Ported from ardia packages/ardiacore/ios/ArdiaChromeWindowHost+Dev.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::{Cell, OnceCell};
use std::time::Duration;

use objc2::rc::Retained;
use objc2::{MainThreadOnly, Message};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSColor, NSEvent, NSEventMask, NSEventModifierFlags,
    NSEventType, NSResponder, NSUserInterfaceItemIdentification, NSView, NSWindow, NSWindowButton,
    NSWindowOcclusionState, NSWindowOrderingMode, NSWindowStyleMask,
};
use objc2_foundation::{
    NSDate, NSDefaultRunLoopMode, NSInsetRect, NSMapTable, NSPoint, NSProcessInfo, NSString,
};
use ria_engine::call::main_queue;
use serde_json::json;

use super::ardia_chrome_window_host::ChromeWindowHost;
use super::ardia_interface_scale::root_of_window;
use super::ardia_reserved_keys::reserved_menu_item;
use super::ardia_traffic_lights::frame_view;
use super::ardia_window_controller::Controllers;

#[derive(Default)]
pub(super) struct DevActions {
    transition: Cell<Duration>,
    gap: Cell<Option<f64>>,
    covers: OnceCell<Retained<NSMapTable<NSWindow, NSWindow>>>,
}

fn milliseconds(text: &str) -> f64 {
    text.parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .unwrap_or(0.)
}

fn duration(ms: f64) -> Duration {
    Duration::try_from_secs_f64(ms.max(0.) / 1000.).unwrap_or_default()
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

fn hit(host: &ChromeWindowHost, window: &NSWindow, spec: &str) -> String {
    let Some(frame) = frame_view(window) else {
        return String::new();
    };
    let mut fields = spec.split(',');
    let point = match (fields.next(), fields.next()) {
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
                NSPoint::new(x, frame.bounds().size.height - y)
            }
        }
        _ => NSPoint::new(-1., -1.),
    };
    describe_view(frame.hitTest(point))
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
    let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
        NSEventType::KeyDown, NSPoint::ZERO, flags, NSProcessInfo::processInfo().systemUptime(),
        window.windowNumber(), None, &chars, &chars, false, code,
    );
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
                let transition = duration(milliseconds(value));
                self.transition.set(transition);
                Some(format!("{:.0}", transition.as_secs_f64() * 1000.))
            }
            "fakeFullScreenOcclusionMs" => {
                let gap = milliseconds(value);
                self.gap.set((gap != -1.).then_some(gap / 1000.));
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
                    let transition = fields
                        .next()
                        .map_or(self.transition.get(), |ms| duration(milliseconds(ms)));
                    if entering != fullscreen.is_full_screen() {
                        fullscreen.act_window_full_screen_with_timing(
                            entering,
                            transition,
                            self.gap.get(),
                        );
                    }
                }
                Some(
                    json!({
                        "fullScreen": fullscreen.is_full_screen(),
                        "styleMask": window.styleMask().contains(NSWindowStyleMask::FullScreen),
                        "acted": controller.core.acted_full_screen(),
                        "chromeCounts": controller.core.acted_full_screen(),
                        "transitioning": fullscreen.transitioning(),
                    })
                    .to_string(),
                )
            }
            "fakeOcclusion" => {
                let controller = controllers.for_window(window)?;
                controller
                    .fullscreen
                    .act_occlusion((value != "off").then_some(value == "visible"));
                Some(flag(
                    window
                        .occlusionState()
                        .contains(NSWindowOcclusionState::Visible),
                ))
            }
            "fakeWindowServer" => {
                controllers
                    .for_window(window)?
                    .fullscreen
                    .act_window_server(value == "shows");
                Some("ok".into())
            }
            "focusState" => Some(host.focus_state(window.mtm())),
            "hit" => Some(hit(host, window, value)),
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
        let covers = self.covers.get_or_init(NSMapTable::weakToStrongObjectsMapTable);
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
