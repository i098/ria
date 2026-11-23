// Ported from ardia packages/ardiacore/ios/ArdiaWindowFullScreen.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::RefCell;
use std::rc::Rc;

use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2::rc::{Retained, Weak};
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSApplication, NSApplicationDidBecomeActiveNotification,
    NSApplicationDidResignActiveNotification, NSScreen, NSWindow,
    NSWindowDidChangeOcclusionStateNotification, NSWindowDidEnterFullScreenNotification,
    NSWindowDidExitFullScreenNotification, NSWindowOcclusionState, NSWindowStyleMask,
    NSWindowWillCloseNotification, NSWindowWillEnterFullScreenNotification,
    NSWindowWillExitFullScreenNotification,
};
use objc2_foundation::{
    NSArray, NSDictionary, NSNotification, NSNotificationCenter, NSNumber, NSRect, NSString,
    ns_string,
};

use super::ardia_window_sight_math::{Rect, counts_as_cover, partly_visible};
use crate::core::bindings::Window;

type WindowInfo = NSDictionary<NSString, AnyObject>;

const CG_WINDOW_LIST_OPTION_ON_SCREEN_ABOVE_WINDOW: u32 = 1 << 1;
const CG_WINDOW_LIST_OPTION_INCLUDING_WINDOW: u32 = 1 << 3;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGWindowListCopyWindowInfo(options: u32, window: u32) -> *mut NSArray<WindowInfo>;
    fn CGRectMakeWithDictionaryRepresentation(dict: *const AnyObject, rect: *mut NSRect) -> bool;
    fn CGGetActiveDisplayList(max: u32, ids: *mut u32, count: *mut u32) -> i32;
    fn CGDisplayIsAsleep(id: u32) -> u32;
    fn CGDisplayBounds(id: u32) -> NSRect;
    fn CGSessionCopyCurrentDictionary() -> *mut WindowInfo;
}

fn number(info: &WindowInfo, key: &NSString) -> Option<Retained<NSNumber>> {
    info.objectForKey(key)?.downcast().ok()
}

fn sight_rect(rect: NSRect) -> Rect {
    Rect::new(
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
    )
}

fn window_server_shows(window: &NSWindow) -> bool {
    if !window.isVisible() || window.isMiniaturized() {
        return false;
    }
    let Ok(id) = u32::try_from(window.windowNumber()) else {
        return false;
    };
    // SAFETY: The flags select on-screen windows above this window, including this window.
    // The Copy result is +1 and the array and its dictionaries are toll-free bridged.
    let options =
        CG_WINDOW_LIST_OPTION_ON_SCREEN_ABOVE_WINDOW | CG_WINDOW_LIST_OPTION_INCLUDING_WINDOW;
    let Some(info) = (unsafe { Retained::from_raw(CGWindowListCopyWindowInfo(options, id)) }) else {
        return false;
    };
    let mut mine = None;
    let mut covers = Vec::new();
    for row in info.iter() {
        let Some(bounds) = row.objectForKey(ns_string!("kCGWindowBounds")) else {
            continue;
        };
        let mut rect = NSRect::ZERO;
        // SAFETY: kCGWindowBounds is a CFDictionary; `rect` is a writable CGRect.
        if !unsafe { CGRectMakeWithDictionaryRepresentation(&*bounds, &mut rect) } {
            continue;
        }
        if number(&row, ns_string!("kCGWindowNumber")).is_some_and(|n| n.unsignedIntValue() == id) {
            if !number(&row, ns_string!("kCGWindowIsOnscreen")).is_some_and(|n| n.boolValue()) {
                return false;
            }
            mine = Some(sight_rect(rect));
            continue;
        }
        let layer = number(&row, ns_string!("kCGWindowLayer")).map_or(0, |n| n.intValue());
        let alpha = number(&row, ns_string!("kCGWindowAlpha")).map_or(0., |n| n.doubleValue());
        if counts_as_cover(layer, alpha) {
            covers.push(sight_rect(rect));
        }
    }
    let Some(mine) = mine else {
        return false;
    };
    let mut ids = [0; 32];
    let mut count = 0;
    // SAFETY: The array has 32 writable slots and count is writable.
    if unsafe { CGGetActiveDisplayList(32, ids.as_mut_ptr(), &mut count) } != 0 {
        return false;
    }
    let mut displays = Vec::with_capacity(count as usize);
    for &id in &ids[..(count as usize).min(ids.len())] {
        // SAFETY: The successful display query populated this ID.
        if unsafe { CGDisplayIsAsleep(id) } == 0 {
            displays.push(sight_rect(unsafe { CGDisplayBounds(id) }));
        }
    }
    partly_visible(mine, &covers, &displays)
}

fn screen_locked() -> bool {
    // SAFETY: The Copy rule returns a +1 dictionary, or null outside a console session.
    unsafe { Retained::from_raw(CGSessionCopyCurrentDictionary()) }.is_some_and(|info| {
        number(&info, ns_string!("CGSSessionScreenIsLocked")).is_some_and(|value| value.boolValue())
    })
}

#[derive(Default)]
struct Transition {
    transitioning: bool,
    settling: bool,
    acting: bool,
    restore_frame: Option<NSRect>,
    overruled: bool,
    seen: bool,
    closed: bool,
}

pub struct FullScreen {
    core: Weak<Window>,
    mtm: MainThreadMarker,
    background: bool,
    allow_occlusion: bool,
    state: RefCell<Transition>,
    observers: Vec<Retained<AnyObject>>,
}

impl FullScreen {
    pub fn new(
        core: &Window,
        mtm: MainThreadMarker,
        background: bool,
        allow_occlusion: bool,
    ) -> Rc<Self> {
        let window = core.window();
        // SAFETY: AppKit posts these notifications on the main thread and each callback holds a weak Rust reference.
        let names = unsafe {
            [
                (NSWindowWillEnterFullScreenNotification, true),
                (NSWindowWillExitFullScreenNotification, true),
                (NSWindowDidEnterFullScreenNotification, true),
                (NSWindowDidExitFullScreenNotification, true),
                (NSWindowDidChangeOcclusionStateNotification, true),
                (NSWindowWillCloseNotification, true),
                (NSApplicationDidResignActiveNotification, false),
                (NSApplicationDidBecomeActiveNotification, false),
            ]
        };
        Rc::new_cyclic(|weak: &std::rc::Weak<Self>| {
            let center = NSNotificationCenter::defaultCenter();
            let observers = names
                .into_iter()
                .map(|(name, local)| {
                    let weak = weak.clone();
                    let handler = RcBlock::new(move |_: std::ptr::NonNull<NSNotification>| {
                        if let Some(fullscreen) = weak.upgrade() {
                            fullscreen.notified(name);
                        }
                    });
                    // SAFETY: The object filter is a live NSWindow and the block has the notification ABI.
                    unsafe {
                        center.addObserverForName_object_queue_usingBlock(
                            Some(name),
                            if local { Some(&window) } else { None },
                            None,
                            &handler,
                        )
                    }
                })
                .map(Retained::into)
                .collect();
            Self {
                core: Weak::from(core),
                mtm,
                background,
                allow_occlusion,
                state: RefCell::new(Transition {
                    seen: true,
                    ..Transition::default()
                }),
                observers,
            }
        })
    }

    fn live_window(&self) -> Option<(Retained<Window>, Retained<NSWindow>)> {
        if self.state.borrow().closed {
            return None;
        }
        let core = self.core.load()?;
        let window = core.window();
        Some((core, window))
    }

    pub fn is_full_screen(&self) -> bool {
        self.live_window().is_some_and(|(core, window)| {
            core.acted_full_screen() || window.styleMask().contains(NSWindowStyleMask::FullScreen)
        })
    }

    pub fn seen(&self) -> bool {
        self.state.borrow().seen
    }
    pub fn seen_only_by_window_server(&self) -> bool {
        let state = self.state.borrow();
        state.seen && state.overruled
    }
    pub fn transitioning(&self) -> bool {
        self.state.borrow().transitioning
    }

    fn notified(&self, name: &NSString) {
        // SAFETY: These names are AppKit static notification constants.
        let (close, will, did, left_app) = unsafe {
            (
                name.isEqualToString(NSWindowWillCloseNotification),
                name.isEqualToString(NSWindowWillEnterFullScreenNotification)
                    || name.isEqualToString(NSWindowWillExitFullScreenNotification),
                name.isEqualToString(NSWindowDidEnterFullScreenNotification)
                    || name.isEqualToString(NSWindowDidExitFullScreenNotification),
                name.isEqualToString(NSApplicationDidResignActiveNotification),
            )
        };
        {
            let mut state = self.state.borrow_mut();
            if state.closed {
                return;
            }
            if close {
                state.closed = true;
                return;
            }
            if will {
                state.transitioning = true;
                state.settling = true;
            }
            if did {
                state.transitioning = false;
            }
        }
        self.update_seen(left_app);
    }

    fn update_seen(&self, left_app: bool) {
        let Some((core, window)) = self.live_window() else {
            return;
        };
        let app = NSApplication::sharedApplication(self.mtm);
        let active = app.isActive();
        let (visible, overruled) = if self.background && !self.allow_occlusion {
            (true, false)
        } else {
            if screen_locked() {
                return;
            }
            let app_kit = window
                .occlusionState()
                .contains(NSWindowOcclusionState::Visible);
            let server = (!active || !app_kit) && window_server_shows(&window);
            (
                if active { app_kit || server } else { server },
                !app_kit && server,
            )
        };
        let active_fullscreen = active
            && (core.acted_full_screen()
                || window.styleMask().contains(NSWindowStyleMask::FullScreen));
        let changed = {
            let mut state = self.state.borrow_mut();
            if state.settling && !state.transitioning && visible {
                state.settling = false;
            }
            let previous = (state.seen, state.overruled);
            state.seen = state.settling || visible || active_fullscreen;
            state.overruled = overruled;
            previous != (state.seen, state.overruled)
        };
        if changed || left_app {
            let info = left_app.then(|| {
                let info = NSDictionary::<NSString, AnyObject>::from_slices(
                    &[ns_string!("leftApp")],
                    &[&*NSNumber::numberWithBool(true)],
                );
                // SAFETY: Erase the dictionary's key type for AppKit's userInfo argument.
                unsafe { Retained::cast_unchecked::<NSDictionary>(info) }
            });
            // SAFETY: This notification passes only the live window and an immutable bool dictionary.
            unsafe {
                NSNotificationCenter::defaultCenter().postNotificationName_object_userInfo(
                    ns_string!("ArdiaWindowSeenDidChange"),
                    Some(&window),
                    info.as_deref(),
                )
            };
        }
    }

    pub fn reconsider_sight(&self) {
        self.update_seen(false);
    }

    pub fn toggle(&self) {
        if !self.background {
            if let Some((_, window)) = self.live_window() {
                window.toggleFullScreen(None);
            }
            return;
        }
        self.act_window_full_screen(!self.is_full_screen());
    }

    pub fn act_window_full_screen(&self, entering: bool) {
        let Some((core, window)) = self.live_window() else {
            return;
        };
        {
            let mut state = self.state.borrow_mut();
            if state.acting || core.acted_full_screen() == entering {
                return;
            }
            state.acting = true;
        }
        let center = NSNotificationCenter::defaultCenter();
        // SAFETY: These AppKit notification names and the window are valid for this call.
        unsafe {
            center.postNotificationName_object(
                if entering {
                    NSWindowWillEnterFullScreenNotification
                } else {
                    NSWindowWillExitFullScreenNotification
                },
                Some(&window),
            );
        }
        if self.state.borrow().closed {
            return;
        }
        if entering {
            self.state.borrow_mut().restore_frame = Some(window.frame());
            core.set_acted_full_screen(true);
            if let Some(screen) = window.screen().or_else(|| NSScreen::mainScreen(self.mtm)) {
                window.setFrame_display(screen.frame(), true);
            }
        } else {
            let restore = self.state.borrow_mut().restore_frame.take();
            if let Some(frame) = restore {
                window.setFrame_display(frame, true);
            }
        }
        {
            let mut state = self.state.borrow_mut();
            if state.closed {
                return;
            }
            state.acting = false;
        }
        if !entering {
            core.set_acted_full_screen(false);
        }
        unsafe {
            center.postNotificationName_object(
                if entering {
                    NSWindowDidEnterFullScreenNotification
                } else {
                    NSWindowDidExitFullScreenNotification
                },
                Some(&window),
            );
        }
    }
}

impl Drop for FullScreen {
    fn drop(&mut self) {
        let center = NSNotificationCenter::defaultCenter();
        for observer in &self.observers {
            // SAFETY: Each object is the token returned by the same notification center.
            unsafe {
                center.removeObserver(observer);
            }
        }
    }
}
