// Ported from ardia packages/ardiacore/ios/ArdiaHoverProbe+Dev.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Changes from Ardia: the "rn" cover is a `RiaDevCover` view of the app's own image in place of an
//! `RCTView`, so the probe still sees it as the app's cover.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ptr::{self, NonNull};

use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyClass, Bool, Imp, Sel};
use objc2::{MainThreadOnly, define_class, ffi, msg_send, sel};
use objc2_app_kit::{
    NSBackingStoreType, NSEvent, NSEventModifierFlags, NSEventType, NSPanel, NSView, NSWindow,
    NSWindowOrderingMode, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use serde_json::json;

use super::*;
use crate::windows::ardia_interface_scale::window_point_from_root;

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "RiaDevCover"]
    struct DevCoverView;
);

thread_local! {
    static DEV_COVERS: RefCell<Vec<Retained<NSView>>> = const { RefCell::new(Vec::new()) };
    static MADE: RefCell<HashMap<String, Retained<NSWindow>>> = RefCell::default();
    static NOT_MAIN_ORIGINAL: Cell<Option<Imp>> = const { Cell::new(None) };
    static NOT_MAIN_WINDOW: RefCell<Option<Weak<NSWindow>>> = const { RefCell::new(None) };
    static INVISIBLE: RefCell<Option<Retained<NSWindow>>> = const { RefCell::new(None) };
}

fn top_left_point(window: &NSWindow, spec: &str) -> NSPoint {
    let n: Vec<&str> = spec.split(',').collect();
    if n.len() < 2 {
        return NSPoint::new(-1.0, -1.0);
    }
    let number = |text: &str| text.trim().parse().unwrap_or(0.0);
    window_point_from_root(NSPoint::new(number(n[0]), number(n[1])), window)
}

fn class_name(view: &NSView) -> String {
    view.class().name().to_string_lossy().into_owned()
}

fn dev_cover(window: &NSWindow, how: &str, at: &str) -> String {
    for cover in DEV_COVERS.take() {
        cover.removeFromSuperview();
    }
    if how == "off" {
        return "uncovered".into();
    }
    let Some(page) = page_at(window, top_left_point(window, at)) else {
        return "no page there".into();
    };
    let rn = how == "rn";
    let cover_host = if rn {
        host().root(window)
    } else {
        superview(&page)
    };
    let Some(cover_host) = cover_host else {
        return "no host".into();
    };
    let frame = cover_host.convertRect_fromView(page.convertRect_toView(page.bounds(), None), None);
    let cover: Retained<NSView> = if rn {
        // SAFETY: `RiaDevCover` is an `NSView` and inherits `initWithFrame:`.
        let cover: Retained<DevCoverView> =
            unsafe { msg_send![DevCoverView::alloc(mtm()), initWithFrame: frame] };
        cover.setAlphaValue(0.0);
        cover.into_super()
    } else {
        NSView::initWithFrame(NSView::alloc(mtm()), frame)
    };
    cover_host.addSubview(&cover);
    DEV_COVERS.with_borrow_mut(|covers| covers.push(cover));
    hit_at(window, top_left_point(window, at))
        .map_or_else(|| "(null)".into(), |hit| class_name(&hit))
}

fn dev_hover_state() -> String {
    let (open, reason, healed, heal_worked) = EPISODE.with_borrow(|episode| {
        let reason = if episode.open {
            episode.first.reason.name()
        } else {
            ""
        };
        (episode.open, reason, episode.healed, episode.heal_worked)
    });
    json!({
        "verdict": LAST_VERDICT.with_borrow(|verdict| verdict.clone()).unwrap_or_default(),
        "open": open,
        "episodeReason": reason,
        "healed": healed,
        "healWorked": heal_worked,
        "pending": PENDING_REASON.get().name(),
        "watching": MONITOR.with_borrow(Option::is_some),
        "emitted": DEV_EVENTS.with_borrow(|events| events.clone()),
    })
    .to_string()
}

fn made_window(command: &str, panel: bool) -> Retained<NSWindow> {
    MADE.with_borrow_mut(|made| {
        made.entry(command.to_owned())
            .or_insert_with(|| {
                let frame = NSRect::new(NSPoint::ZERO, NSSize::new(40.0, 40.0));
                let style = NSWindowStyleMask::Titled;
                let backing = NSBackingStoreType::Buffered;
                if panel {
                    let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
                        NSPanel::alloc(mtm()),
                        frame,
                        style,
                        backing,
                        true,
                    );
                    return panel.into_super();
                }
                // SAFETY: A plain titled window with a buffered backing store.
                unsafe {
                    NSWindow::initWithContentRect_styleMask_backing_defer(
                        NSWindow::alloc(mtm()),
                        frame,
                        style,
                        backing,
                        true,
                    )
                }
            })
            .clone()
    })
}

fn dev_make_key(browser: &NSWindow, command: &str, mode: &str) -> String {
    let panel = command == "panelKey";
    let other = made_window(command, panel);
    // SAFETY: `MADE` keeps the window, so AppKit must not release it on close.
    unsafe { other.setReleasedWhenClosed(false) };
    if panel {
        other.orderFront(None);
    }
    if mode == "main" {
        other.setAlphaValue(0.0);
        other.setIgnoresMouseEvents(true);
        other.orderBack(None);
        other.makeMainWindow();
    }
    other.makeKeyWindow();
    let host = host();
    let app = app();
    format!(
        "key {}, main {}, browser {}",
        host.describe_focus_window(app.keyWindow().as_deref()),
        host.describe_focus_window(app.mainWindow().as_deref()),
        host.describe_focus_window(Some(browser)),
    )
}

fn dev_not_main(window: &NSWindow, on: bool) -> String {
    if NOT_MAIN_ORIGINAL.get().is_none() {
        fake_is_main_window(window.class());
    }
    NOT_MAIN_WINDOW.set(on.then(|| Weak::new(window)));
    host().describe_focus_window(Some(window))
}

/// Replaces `isMainWindow` of `class` with one that answers NO for the `notMain` window.
fn fake_is_main_window(class: &AnyClass) {
    let selector = sel!(isMainWindow);
    let Some(method) = class.instance_method(selector) else {
        return;
    };
    let original = method.implementation();
    NOT_MAIN_ORIGINAL.set(Some(original));
    // SAFETY: `isMainWindow` takes no argument and returns `BOOL`.
    let original: unsafe extern "C-unwind" fn(NonNull<NSWindow>, Sel) -> Bool =
        unsafe { std::mem::transmute(original) };
    let fake = RcBlock::new(move |w: NonNull<NSWindow>| -> Bool {
        // SAFETY: The runtime passes the receiving window.
        let window = unsafe { w.as_ref() };
        let faked = NOT_MAIN_WINDOW.with_borrow(|faked| same(window, faked.as_ref()));
        // SAFETY: `original` is the class's own `isMainWindow`.
        Bool::new(!faked && unsafe { original(w, selector) }.as_bool())
    });
    // SAFETY: The runtime copies the block, and the fake has the method's type encoding.
    unsafe {
        let imp = ffi::imp_implementationWithBlock(ptr::from_ref(&*fake).cast_mut().cast());
        ffi::class_replaceMethod(
            ptr::from_ref(class).cast_mut(),
            selector,
            imp,
            ffi::method_getTypeEncoding(method),
        );
    }
}

fn invisible_window(browser: &NSWindow) -> Retained<NSWindow> {
    INVISIBLE.with_borrow_mut(|cover| {
        cover
            .get_or_insert_with(|| {
                // SAFETY: A plain borderless window with a buffered backing store.
                let window = unsafe {
                    NSWindow::initWithContentRect_styleMask_backing_defer(
                        NSWindow::alloc(mtm()),
                        browser.frame(),
                        NSWindowStyleMask::Borderless,
                        NSBackingStoreType::Buffered,
                        false,
                    )
                };
                // SAFETY: `INVISIBLE` keeps the window.
                unsafe { window.setReleasedWhenClosed(false) };
                window.setAlphaValue(0.0);
                window
            })
            .clone()
    })
}

fn dev_invisible_cover(browser: &NSWindow, how: &str) -> String {
    let cover = invisible_window(browser);
    match how {
        "off" => {
            cover.orderOut(None);
            "removed".into()
        }
        "click" => {
            click_middle(&cover);
            dev_hover_state()
        }
        _ => {
            cover.setIgnoresMouseEvents(false);
            cover.setFrame_display(browser.frame(), false);
            cover.orderWindow_relativeTo(NSWindowOrderingMode::Above, browser.windowNumber());
            format!(
                "cover #{} visible {}",
                cover.windowNumber(),
                u8::from(cover.isVisible())
            )
        }
    }
}

fn click_middle(cover: &NSWindow) {
    let frame = cover.frame();
    let inside = NSPoint::new(frame.size.width / 2.0, frame.size.height / 2.0);
    let app = app();
    for kind in [NSEventType::LeftMouseDown, NSEventType::LeftMouseUp] {
        let event = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
            kind,
            inside,
            NSEventModifierFlags::empty(),
            uptime(),
            cover.windowNumber(),
            None,
            0,
            1,
            1.0,
        );
        if let Some(event) = event {
            app.sendEvent(&event);
        }
    }
}

fn describe_rect(rect: NSRect) -> String {
    format!(
        "{{{{{}, {}}}, {{{}, {}}}}}",
        rect.origin.x, rect.origin.y, rect.size.width, rect.size.height
    )
}

fn cover_chain(hit: &NSView) -> String {
    let mut chain = Vec::new();
    let mut view = Some(hit.retain());
    while let Some(v) = view
        && chain.len() < 6
    {
        chain.push(format!(
            "{}({:.2}){}",
            class_name(&v),
            v.alphaValue(),
            describe_rect(v.convertRect_toView(v.bounds(), None))
        ));
        view = superview(&v);
    }
    chain.join(" < ")
}

fn dev_see_through_covers(window: &NSWindow) -> String {
    let mut found: Vec<String> = Vec::new();
    let Some(frame) = window.contentView().as_deref().and_then(superview) else {
        return String::new();
    };
    let bounds = frame.bounds();
    let mut y = 8.0;
    while y < bounds.size.height {
        let mut x = 8.0;
        while x < bounds.size.width {
            if let Some(line) = see_through_at(window, NSPoint::new(x, y))
                && !found.contains(&line)
            {
                found.push(line);
            }
            x += 16.0;
        }
        y += 16.0;
    }
    found.join("\n")
}

fn see_through_at(window: &NSWindow, point: NSPoint) -> Option<String> {
    page_at(window, point)?;
    let hit = hit_at(window, point)?;
    if enclosing_page(Some(&hit)).is_some() || !see_through(Some(&hit)) {
        return None;
    }
    Some(cover_chain(&hit))
}

fn dev_blocker_key() -> String {
    let host = host();
    let app = app();
    for window in app.windows().iter() {
        if host.is_hidden_page_window(&window) {
            window.makeKeyWindow();
            return format!(
                "blocker {}, key {}",
                host.describe_focus_window(Some(&window)),
                host.describe_focus_window(app.keyWindow().as_deref())
            );
        }
    }
    "no blocker-page window".into()
}

/// `HoverProbeDev`: the dev commands of the hover probe, as `command:rest[:more]`.
pub fn hover_probe_dev(window: &NSWindow, argument: Option<&str>) -> String {
    let parts: Vec<&str> = argument.unwrap_or_default().split(':').collect();
    let (command, rest) = (parts[0], parts.get(1).copied().unwrap_or_default());
    let on = rest == "1";
    match command {
        "watch" => dev_watch(on),
        "check" => {
            let trigger = if rest.ends_with(",click") {
                HoverTrigger::Click
            } else {
                HoverTrigger::Move
            };
            let point = top_left_point(window, rest);
            handle_hover_check(Some(window), point, trigger, host().consent_epoch());
        }
        "preventKey" => {
            // SAFETY: `preventKeyWindow` is a `BOOL` property of Chromium's windows.
            unsafe {
                window.setValue_forKey(
                    Some(&NSNumber::new_bool(on)),
                    ns_string!("preventKeyWindow"),
                )
            };
            return host().describe_focus_window(Some(window));
        }
        "halfActive" => {
            DEV_HALF_ACTIVE.set(on);
            return hover_probe_line();
        }
        "menuActive" => {
            DEV_MENU_ACTIVE.set(on);
            return format!(
                "menuActive {}, real {}, menu shown {}",
                u8::from(menu_active(window)),
                u8::from(window_flag(window, c"hasViewsMenuActive")),
                u8::from(menu_shown(window))
            );
        }
        "hiddenKey" | "panelKey" => return dev_make_key(window, command, rest),
        "notMain" => return dev_not_main(window, on),
        "invisibleCover" => return dev_invisible_cover(window, rest),
        "responderNil" => return dev_responder_nil(window, on),
        "seeThrough" => return dev_see_through_covers(window),
        "blockerKey" => return dev_blocker_key(),
        "cover" => return dev_cover(window, rest, parts.get(2).copied().unwrap_or_default()),
        _ => {}
    }
    dev_hover_state()
}

fn dev_watch(on: bool) {
    WATCH_IN_BACKGROUND.set(on);
    if on {
        start_hover_watch();
    } else if host().background() {
        stop_hover_watch();
    }
}

fn dev_responder_nil(window: &NSWindow, on: bool) -> String {
    if on {
        window.makeFirstResponder(None);
    }
    let holder = holder_of(window);
    format!(
        "responder {}, lost {}",
        holder.name(),
        u8::from(responder_lost(holder, true))
    )
}
