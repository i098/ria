// Ported from ardia packages/ardiacore/ios/ArdiaFocusState.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::ptr::NonNull;
use std::rc::Rc;

use block2::RcBlock;
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObjectProtocol};
use objc2_app_kit::{
    NSApplication, NSApplicationDidBecomeActiveNotification,
    NSApplicationDidResignActiveNotification, NSResponder, NSScreen, NSWindow,
    NSWindowDidBecomeKeyNotification, NSWindowDidBecomeMainNotification,
    NSWindowDidResignKeyNotification, NSWindowDidResignMainNotification,
};
use objc2_foundation::{
    NSDate, NSDateFormatter, NSNotification, NSNotificationCenter, NSNumber,
    NSObjectNSKeyValueCoding, ns_string,
};

use super::ardia_window_controller::Controllers;

const HISTORY_LENGTH: usize = 40;

type Observers = Vec<Retained<AnyObject>>;

pub struct FocusState {
    controllers: Rc<Controllers>,
    clock: Retained<NSDateFormatter>,
    history: RefCell<VecDeque<String>>,
    observers: Observers,
}

fn describe_window(window: &NSWindow, controllers: &Controllers) -> String {
    let mut flags = Vec::new();
    for (shown, flag) in [
        (window.isVisible(), "visible"),
        (window.alphaValue() < 0.01, "transparent"),
        (window.ignoresMouseEvents(), "ignoresMouse"),
        (window.isMiniaturized(), "minimized"),
        (window.isKeyWindow(), "key"),
        (window.isMainWindow(), "main"),
        (window.canBecomeKeyWindow(), "canKey"),
        (window.canBecomeMainWindow(), "canMain"),
    ] {
        if shown {
            flags.push(flag);
        }
    }
    let parent = window
        .parentWindow()
        .map(|window| format!("parent#{}", window.windowNumber()));
    if let Some(parent) = &parent {
        flags.push(parent);
    }
    if window.attachedSheet().is_some() {
        flags.push("sheet");
    }
    if window.respondsToSelector(sel!(preventKeyWindow))
        && window
            .valueForKey(ns_string!("preventKeyWindow"))
            .and_then(|value| value.downcast::<NSNumber>().ok())
            .is_some_and(|value| value.boolValue())
    {
        flags.push("preventKey");
    }
    if window.isVisible() && !window.isOnActiveSpace() {
        flags.push("otherSpace");
    }
    let screen = window.screen().map(|screen| {
        let index = NSScreen::screens(window.mtm()).indexOfObject(&screen);
        format!("screen{index}")
    });
    if let Some(screen) = &screen {
        flags.push(screen);
    }
    let role = controllers.for_window(window).map_or("", |controller| {
        if controller.utility() || controller.core.internal() {
            " internal"
        } else if controller.root.borrow().is_some() {
            " browser"
        } else {
            " ardia"
        }
    });
    format!(
        "{}#{}{role} level {} [{}]",
        window.class().name().to_string_lossy(),
        window.windowNumber(),
        window.level(),
        flags.join(" ")
    )
}

fn describe_responder(mut responder: Option<Retained<NSResponder>>) -> String {
    let mut chain = Vec::new();
    for _ in 0..4 {
        let Some(current) = responder else { break };
        chain.push(current.class().name().to_string_lossy());
        // SAFETY: The live window and its view tree own the responder chain on this thread.
        responder = unsafe { current.nextResponder() };
    }
    if chain.is_empty() {
        "none".into()
    } else {
        chain.join(" > ")
    }
}

impl FocusState {
    pub fn watch(_mtm: MainThreadMarker, controllers: Rc<Controllers>) -> Rc<Self> {
        let clock = NSDateFormatter::new();
        clock.setDateFormat(Some(ns_string!("HH:mm:ss.SSS")));
        Rc::new_cyclic(|weak: &std::rc::Weak<Self>| {
            let center = NSNotificationCenter::defaultCenter();
            // SAFETY: AppKit publishes these notifications on the main thread.
            let names = unsafe {
                [
                    (NSWindowDidBecomeKeyNotification, "became key", true),
                    (NSWindowDidResignKeyNotification, "resigned key", true),
                    (NSWindowDidBecomeMainNotification, "became main", true),
                    (NSWindowDidResignMainNotification, "resigned main", true),
                    (
                        NSApplicationDidBecomeActiveNotification,
                        "app active",
                        false,
                    ),
                    (
                        NSApplicationDidResignActiveNotification,
                        "app inactive",
                        false,
                    ),
                ]
            };
            let observers = names
                .into_iter()
                .map(|(name, what, from_window)| {
                    let weak = weak.clone();
                    let callback = RcBlock::new(move |note: NonNull<NSNotification>| {
                        let Some(focus) = weak.upgrade() else { return };
                        // SAFETY: The notification lives for the duration of this callback.
                        let window = unsafe { note.as_ref() }
                            .object()
                            .filter(|_| from_window)
                            .and_then(|object| object.downcast::<NSWindow>().ok());
                        focus.record(what, window.as_deref());
                    });
                    // SAFETY: The callback handles local AppKit notifications and checks the sender type.
                    unsafe {
                        center.addObserverForName_object_queue_usingBlock(
                            Some(name),
                            None,
                            None,
                            &callback,
                        )
                    }
                    .into()
                })
                .collect();
            Self {
                controllers,
                clock,
                history: RefCell::new(VecDeque::with_capacity(HISTORY_LENGTH)),
                observers,
            }
        })
    }

    pub fn record(&self, change: &str, window: Option<&NSWindow>) {
        let time = self.clock.stringFromDate(&NSDate::date());
        let entry = match window {
            Some(window) => format!(
                "{time} {change} {}",
                describe_window(window, &self.controllers)
            ),
            None => format!("{time} {change}"),
        };
        let mut history = self.history.borrow_mut();
        if history.len() == HISTORY_LENGTH {
            history.pop_front();
        }
        history.push_back(entry);
    }

    pub fn summary(&self, app: &NSApplication) -> String {
        let describe = |window: Option<&NSWindow>| {
            window.map_or_else(
                || "none".into(),
                |window| describe_window(window, &self.controllers),
            )
        };
        let key = app.keyWindow();
        let main = app.mainWindow();
        let active = if app.isActive() { "active" } else { "inactive" };
        let mut main_line = format!("main {}", describe(main.as_deref()));
        if let Some(main) = main.as_deref()
            && !key.as_deref().is_some_and(|key| std::ptr::eq(key, main))
        {
            main_line.push_str(", responder ");
            main_line.push_str(&describe_responder(main.firstResponder()));
        }
        let mut lines = vec![
            format!(
                "app {active}, modal {}",
                describe(app.modalWindow().as_deref())
            ),
            format!(
                "key {}, responder {}",
                describe(key.as_deref()),
                describe_responder(key.as_ref().and_then(|window| window.firstResponder()))
            ),
            main_line,
        ];
        for window in app.windows().iter() {
            let same =
                |other: Option<&NSWindow>| other.is_some_and(|other| std::ptr::eq(other, &*window));
            if same(key.as_deref())
                || same(main.as_deref())
                || !(window.isVisible() || window.canBecomeKeyWindow())
            {
                continue;
            }
            let mut line = format!("window {}", describe(Some(&window)));
            if self
                .controllers
                .for_window(&window)
                .is_some_and(|controller| controller.root.borrow().is_some())
            {
                line.push_str(", responder ");
                line.push_str(&describe_responder(window.firstResponder()));
            }
            lines.push(line);
        }
        lines.extend(
            self.history
                .borrow()
                .iter()
                .map(|change| format!("was {change}")),
        );
        lines.join("\n")
    }
}

impl Drop for FocusState {
    fn drop(&mut self) {
        let center = NSNotificationCenter::defaultCenter();
        for observer in &self.observers {
            // SAFETY: The token came from this notification center.
            unsafe { center.removeObserver(observer) };
        }
    }
}
