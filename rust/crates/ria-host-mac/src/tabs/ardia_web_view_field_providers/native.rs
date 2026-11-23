// Ported from ardia packages/ardiacore/ios/ArdiaWebView+FieldProviders.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyClass, AnyObject, AnyProtocol, Imp, NSObjectProtocol, Sel};
use objc2::{MainThreadMarker, class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSMenuItem, NSScreen, NSView, NSWorkspace, NSWorkspaceOpenConfiguration,
};
use objc2_foundation::{
    NSArray, NSFileManager, NSMethodSignature, NSRange, NSRect, NSString, NSURL,
};
use ria_engine::call::main_queue;
use serde_json::{Value, json};

use crate::core::convert::to_json;

pub fn verification_handler(url: &NSURL) -> Option<(Retained<NSURL>, String)> {
    let handler = NSWorkspace::sharedWorkspace().URLForApplicationToOpenURL(url)?;
    let path = handler.path()?;
    let app = NSFileManager::defaultManager().displayNameAtPath(&path);
    let name = app.to_string();
    let name = name.strip_suffix(".app").unwrap_or(&name).to_owned();
    (!name.is_empty()).then_some((handler, name))
}

pub fn open_verification_code(
    url: &NSURL,
    handler: &NSURL,
    background: bool,
    completion: impl FnOnce(Value) + 'static,
) {
    // ArdiaWebView+FieldProviders.mm:499-501.
    if background {
        completion(json!({"opened": true}));
        return;
    }
    let config = NSWorkspaceOpenConfiguration::configuration();
    config.setActivates(true);
    let completion = std::cell::Cell::new(Some(completion));
    let done = RcBlock::new(
        move |app: *mut objc2_app_kit::NSRunningApplication,
              error: *mut objc2_foundation::NSError| {
            if let Some(completion) = completion.take() {
                let opened = !app.is_null() && error.is_null();
                main_queue::post(Box::new(move || {
                    completion(if opened {
                        json!({"opened": true})
                    } else {
                        json!({"opened": false, "reason": "failed"})
                    });
                }));
            }
        },
    );
    NSWorkspace::sharedWorkspace().openURLs_withApplicationAtURL_configuration_completionHandler(
        &NSArray::from_slice(&[url]),
        handler,
        &config,
        Some(&done),
    );
}

pub fn system_panel_availability() -> Value {
    let heuristic = AnyClass::get(c"NSAutoFillHeuristicController");
    let action = heuristic.is_some() && action_available();
    let chromium = action && heuristic.is_some_and(|class| {
        // SAFETY: The class responds to standard NSObject introspection.
        unsafe { msg_send![class, respondsToSelector: sel!(_inputContextForChromiumSecureTextFieldFromResponder:)] }
    });
    json!({"chromium": chromium, "webkit": action})
}

fn action_available() -> bool {
    // SAFETY: NSObject implements instancesRespondToSelector: for Objective-C classes.
    unsafe {
        msg_send![class!(NSApplication), instancesRespondToSelector: sel!(_handleInsertFromPasswordsCommand:)]
    }
}

fn edits_web_text(view: &NSView) -> bool {
    let input_type = sel!(textInputType);
    if view.respondsToSelector(input_type) {
        // SAFETY: Check the Objective-C encoding before interpreting a private selector's result.
        unsafe {
            let signature: Option<Retained<NSMethodSignature>> =
                msg_send![view, methodSignatureForSelector: input_type];
            let Some(signature) = signature else {
                return false;
            };
            match *signature.methodReturnType().as_ptr().cast::<u8>() {
                b'i' | b'I' => {
                    let value: i32 = msg_send![view, textInputType];
                    value != 0
                }
                b'q' | b'Q' | b'l' | b'L' => {
                    let value: i64 = msg_send![view, textInputType];
                    value != 0
                }
                _ => false,
            }
        }
    } else {
        AnyProtocol::get(c"NSTextInputClient")
            .is_some_and(|protocol| view.conformsToProtocol(protocol))
            && view.inputContext().is_some()
    }
}

type AutoFillPass = unsafe extern "C-unwind" fn(*mut AnyObject, Sel);

static AUTO_FILL_PASS: std::sync::LazyLock<Option<AutoFillPass>> = std::sync::LazyLock::new(|| {
    let method =
        AnyClass::get(c"NSAutoFillHeuristicController")?.instance_method(auto_fill_selector())?;
    if method.arguments_count() != 2 || method.return_type().to_bytes() != b"v" {
        return None;
    }
    // SAFETY: The selector has no explicit arguments and returns void.
    unsafe {
        let replacement = std::mem::transmute::<AutoFillPass, Imp>(held_auto_fill_pass);
        Some(std::mem::transmute::<Imp, AutoFillPass>(
            method.set_implementation(replacement),
        ))
    }
});

struct AutoFillHold {
    page: Weak<NSView>,
    until: Instant,
    skipped: bool,
}

thread_local! {
    static HOLD: RefCell<Option<AutoFillHold>> = const { RefCell::new(None) };
    static PANEL_WATCH: Cell<u64> = const { Cell::new(0) };
}

fn auto_fill_selector() -> Sel {
    sel!(_showOrHideAutoFillForCurrentTextInputContextIfAppropriate)
}

fn page_has_responder(page: &NSView) -> bool {
    page.window()
        .and_then(|window| window.firstResponder())
        .is_some_and(|responder| {
            let object: &AnyObject = &responder;
            object
                .downcast_ref::<NSView>()
                .is_some_and(|view| view.isDescendantOf(page))
        })
}

unsafe extern "C-unwind" fn held_auto_fill_pass(receiver: *mut AnyObject, selector: Sel) {
    if MainThreadMarker::new().is_some()
        && HOLD.with(|hold| {
            let mut slot = hold.borrow_mut();
            if let Some(hold) = slot.as_mut()
                && hold.until > Instant::now()
                && hold
                    .page
                    .load()
                    .is_some_and(|page| page_has_responder(&page))
            {
                hold.skipped = true;
                return true;
            }
            *slot = None;
            false
        })
    {
        return;
    }
    if let Some(original) = *AUTO_FILL_PASS {
        // SAFETY: Forward the original receiver and selector with the verified signature.
        unsafe { original(receiver, selector) };
    }
}

fn start_auto_fill_hold(page: &NSView) -> Option<Instant> {
    (*AUTO_FILL_PASS)?;
    let wait = Duration::from_millis(1500);
    let until = Instant::now() + wait;
    HOLD.with(|hold| {
        *hold.borrow_mut() = Some(AutoFillHold {
            page: Weak::new(page),
            until,
            skipped: false,
        })
    });
    main_queue::after(wait, Box::new(move || end_auto_fill_hold(until)));
    Some(until)
}

fn end_auto_fill_hold(until: Instant) {
    let skipped = HOLD.with(|hold| {
        let mut slot = hold.borrow_mut();
        if slot.as_ref().is_none_or(|hold| hold.until != until) {
            return false;
        }
        slot.take().is_some_and(|hold| hold.skipped)
    });
    if !skipped {
        return;
    }
    let Some(class) = AnyClass::get(c"NSAutoFillHeuristicController") else {
        return;
    };
    let Some(original) = *AUTO_FILL_PASS else {
        return;
    };
    // SAFETY: Check that the controller class has its singleton accessor before calling it.
    unsafe {
        let shared = sel!(sharedInstance);
        let responds: bool = msg_send![class, respondsToSelector: shared];
        if responds {
            let controller: *mut AnyObject = msg_send![class, sharedInstance];
            if !controller.is_null() {
                original(controller, auto_fill_selector());
            }
        }
    }
}

pub fn when_system_panel_closes(closed: impl FnOnce() + 'static) {
    let watch = PANEL_WATCH.with(|watch| {
        let next = watch.get().wrapping_add(1);
        watch.set(next);
        next
    });
    watch_system_panel(watch, Instant::now() + Duration::from_secs(120), closed);
}

fn watch_system_panel(watch: u64, deadline: Instant, closed: impl FnOnce() + 'static) {
    main_queue::after(
        Duration::from_millis(250),
        Box::new(move || {
            if !PANEL_WATCH.with(|current| current.get() == watch) {
                return;
            }
            if Instant::now() < deadline {
                let (frames, hidden) = panel_frames();
                if hidden || !frames.is_empty() {
                    watch_system_panel(watch, deadline, closed);
                    return;
                }
            }
            closed();
        }),
    );
}

#[derive(Default)]
pub struct SystemPasswordPanel {
    last: Option<Instant>,
}

impl SystemPasswordPanel {
    pub fn open(
        &mut self,
        mtm: MainThreadMarker,
        page: &NSView,
        background: bool,
        allow_background_panel: bool,
        completion: impl FnOnce(Value) + 'static,
    ) {
        let not_shown = |reason: &str| json!({"shown": false, "reason": reason});
        if AnyClass::get(c"NSAutoFillHeuristicController").is_none() || !action_available() {
            completion(not_shown("unavailable"));
            return;
        }
        let Some(window) = page.window().filter(|window| window.isKeyWindow()) else {
            completion(not_shown("notKey"));
            return;
        };
        let editing = window.firstResponder().is_some_and(|responder| {
            let view: &AnyObject = &responder;
            view.downcast_ref::<NSView>()
                .is_some_and(|view| view.isDescendantOf(page) && edits_web_text(view))
        });
        if !editing {
            completion(not_shown("notEditing"));
            return;
        }
        let now = Instant::now();
        if self
            .last
            .is_some_and(|last| now.duration_since(last) < Duration::from_millis(600))
        {
            completion(not_shown("throttled"));
            return;
        }
        // ArdiaWebView+FieldProviders.mm:273-279.
        self.last = Some(now);
        if background && !allow_background_panel {
            completion(json!({"shown": true}));
            return;
        }
        let before = panel_frames().0;
        let hold = start_auto_fill_hold(page);
        let app = NSApplication::sharedApplication(mtm);
        let action = sel!(_handleInsertFromPasswordsCommand:);
        // SAFETY: The action is available and the window's responder was verified as a descendant of the page.
        let sent = unsafe {
            let item = NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc(),
                &NSString::from_str("AutoFill"),
                Some(action),
                &NSString::from_str(""),
            );
            app.targetForAction_to_from(action, None, Some(&item))
                .is_some_and(|target| app.sendAction_to_from(action, Some(&target), Some(&item)))
        };
        if !sent {
            if let Some(until) = hold {
                end_auto_fill_hold(until);
            }
            completion(not_shown("refused"));
            return;
        }
        let page = Weak::new(page);
        main_queue::after(
            Duration::from_millis(600),
            Box::new(move || {
                if let Some(report) = panel_report(&before, &page) {
                    completion(report);
                } else {
                    main_queue::after(
                        Duration::from_millis(900),
                        Box::new(move || {
                            completion(
                                panel_report(&before, &page)
                                    .unwrap_or_else(|| not_shown("noPanel")),
                            );
                        }),
                    );
                }
            }),
        );
    }
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGWindowListCopyWindowInfo(options: u32, relative_window: u32) -> *mut NSArray<AnyObject>;
}

// ArdiaWebView+FieldProviders.mm:176-205, 224-236, 247-253.
fn panel_frames() -> (HashMap<u64, f64>, bool) {
    // SAFETY: The copy returns an owned CFArray, which is toll-free bridged to NSArray.
    let windows = unsafe { Retained::from_raw(CGWindowListCopyWindowInfo(1 | 16, 0)) };
    let mut frames = HashMap::new();
    let mut hidden = false;
    if let Some(windows) = windows {
        for window in windows.iter() {
            let window = to_json(&window);
            let owner = window["kCGWindowOwnerName"].as_str().unwrap_or_default();
            if owner.is_empty()
                && window["kCGWindowOwnerPID"].as_u64() != Some(u64::from(std::process::id()))
            {
                hidden = true;
            }
            let bounds = &window["kCGWindowBounds"];
            if owner == "AutoFillPanelService"
                && bounds["Width"].as_f64().is_some_and(|width| width >= 100.0)
                && let (Some(height), Some(y), Some(id)) = (
                    bounds["Height"].as_f64(),
                    bounds["Y"].as_f64(),
                    window["kCGWindowNumber"].as_u64(),
                )
                && height >= 100.0
            {
                frames.insert(id, y + height / 2.0);
            }
        }
    }
    (frames, hidden)
}

fn panel_report(before: &HashMap<u64, f64>, page: &Weak<NSView>) -> Option<Value> {
    let (frames, hidden) = panel_frames();
    if let Some((_, panel_mid_y)) = frames.iter().find(|(id, _)| !before.contains_key(*id)) {
        let side = page.load().and_then(|page| panel_side(&page, *panel_mid_y));
        return Some(side.map_or_else(
            || json!({"shown": true}),
            |side| json!({"shown": true, "panel": side}),
        ));
    }
    hidden.then(|| json!({"shown": true}))
}

fn panel_side(page: &NSView, panel_mid_y: f64) -> Option<&'static str> {
    let responder = page.window()?.firstResponder()?;
    if !responder.conformsToProtocol(AnyProtocol::get(c"NSTextInputClient")?) {
        return None;
    }
    // SAFETY: NSTextInputClient defines these selectors and their range and rect types.
    let caret: NSRect = unsafe {
        let range: NSRange = msg_send![&*responder, selectedRange];
        msg_send![&*responder, firstRectForCharacterRange: range, actualRange: std::ptr::null_mut::<NSRange>()]
    };
    if caret == NSRect::ZERO {
        return None;
    }
    let screen = NSScreen::screens(MainThreadMarker::new()?).firstObject()?;
    let caret_mid_y = screen.frame().size.height - (caret.origin.y + caret.size.height / 2.0);
    Some(if panel_mid_y >= caret_mid_y {
        "below"
    } else {
        "above"
    })
}
