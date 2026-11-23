// Ported from ardia packages/ardiacore/ios/ArdiaWebView+InputMonitors.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::{CStr, c_char, c_void};
use std::ptr::NonNull;
use std::rc::Rc;

use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2::rc::{Retained, Weak};
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSEvent, NSEventMask, NSEventModifierFlags, NSEventPhase, NSEventType, NSScreen, NSTouchPhase,
};

use super::ScrollZoom;

pub struct EventMonitor(Retained<AnyObject>);

impl EventMonitor {
    fn install(
        _mtm: MainThreadMarker,
        mask: NSEventMask,
        swallow: impl Fn(&NSEvent) -> bool + 'static,
    ) -> Option<Self> {
        // SAFETY: AppKit supplies a live event. Return that same event or null.
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
        }?;
        Some(Self(monitor))
    }
}

impl Drop for EventMonitor {
    fn drop(&mut self) {
        // SAFETY: AppKit issued the monitor token to `install`.
        unsafe { NSEvent::removeMonitor(&self.0) };
    }
}

pub enum WheelTarget {
    None,
    Native,
    PageScript,
}

pub trait ZoomHooks {
    fn disarm(&self);
    fn arm_at_pointer(&self);
    fn target(&self, event: &NSEvent) -> WheelTarget;
    fn credit_page(&self, event: &NSEvent, delta: f64);
    fn zoom(&self, event: &NSEvent, direction: i32);
}

pub struct CommandScrollMonitor {
    _events: [EventMonitor; 3],
    enabled: Rc<Cell<bool>>,
    hooks: Rc<dyn ZoomHooks>,
}

impl CommandScrollMonitor {
    pub fn install(mtm: MainThreadMarker, hooks: impl ZoomHooks + 'static) -> Option<Self> {
        let hooks: Rc<dyn ZoomHooks> = Rc::new(hooks);
        let enabled = Rc::new(Cell::new(true));
        let touches = Rc::new(Cell::new((0, 0.0)));
        let seen = touches.clone();
        let touch = EventMonitor::install(mtm, NSEventMask::Gesture, move |event| {
            seen.set((
                event
                    .touchesMatchingPhase_inView(NSTouchPhase::Touching, None)
                    .count(),
                event.timestamp(),
            ));
            false
        })?;
        let flags_hooks = hooks.clone();
        let flags_enabled = enabled.clone();
        let flags = EventMonitor::install(mtm, NSEventMask::FlagsChanged, move |event| {
            if !event
                .modifierFlags()
                .contains(NSEventModifierFlags::Command)
            {
                flags_hooks.disarm();
            } else if flags_enabled.get() {
                flags_hooks.arm_at_pointer();
            }
            false
        })?;
        let scroll_hooks = hooks.clone();
        let scroll_enabled = enabled.clone();
        let zoom = RefCell::new(ScrollZoom::default());
        let trackpads = RefCell::new(HashMap::new());
        let scroll = EventMonitor::install(mtm, NSEventMask::ScrollWheel, move |event| {
            let phase = event.phase();
            let begins = phase.intersects(NSEventPhase::Began | NSEventPhase::MayBegin);
            let (fingers, touched) = touches.get();
            let gesture = !phase.is_empty()
                && ((begins && fingers >= 2 && event.timestamp() - touched < 0.5)
                    || from_trackpad(event, &trackpads));
            let command = event
                .modifierFlags()
                .contains(NSEventModifierFlags::Command);
            let no_phase = phase.is_empty() && event.momentumPhase().is_empty();
            if !zoom
                .borrow_mut()
                .candidate(command, begins, no_phase, gesture)
            {
                scroll_hooks.disarm();
                return false;
            }
            if !scroll_enabled.get() {
                return false;
            }
            let delta = event.scrollingDeltaY()
                * if event.hasPreciseScrollingDeltas() {
                    1.0
                } else {
                    30.0
                };
            let target = scroll_hooks.target(event);
            if phase == NSEventPhase::Began && !matches!(target, WheelTarget::None) {
                zoom.borrow_mut().begin();
            }
            match target {
                WheelTarget::None => false,
                WheelTarget::PageScript => {
                    scroll_hooks.credit_page(event, delta);
                    false
                }
                WheelTarget::Native => {
                    let step = zoom.borrow_mut().step(delta);
                    if let Some(step) = step {
                        scroll_hooks.zoom(event, step);
                    }
                    true
                }
            }
        })?;
        Some(Self {
            _events: [touch, flags, scroll],
            enabled,
            hooks,
        })
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.set(enabled);
        if !enabled {
            self.hooks.disarm();
        }
    }
}

pub trait PrewarmHooks {
    fn skip(&self, event: &NSEvent) -> bool;
    fn swallow_ahead(&self, event: &NSEvent) -> bool;
    fn reveal_for_press(&self, event: &NSEvent);
    fn wake_frames_if_outside_pages(&self, event: &NSEvent) -> bool;
}

pub fn install_prewarm(
    mtm: MainThreadMarker,
    mask: NSEventMask,
    hooks: impl PrewarmHooks + 'static,
) -> Option<EventMonitor> {
    let displays: RefCell<Vec<(Option<Weak<NSScreen>>, f64)>> = RefCell::default();
    EventMonitor::install(mtm, mask, move |event| {
        if hooks.skip(event) {
            return false;
        }
        if hooks.swallow_ahead(event) {
            return true;
        }
        if matches!(
            event.r#type(),
            NSEventType::LeftMouseDown | NSEventType::LeftMouseUp
        ) {
            hooks.reveal_for_press(event);
            let Some(window) = event.window(mtm) else {
                return false;
            };
            let screen = window.screen();
            let timestamp = event.timestamp();
            let mut displays = displays.borrow_mut();
            displays.retain(|(known, _)| known.as_ref().is_none_or(|known| known.load().is_some()));
            let index = displays.iter().position(|(known, _)| {
                known
                    .as_ref()
                    .and_then(Weak::load)
                    .as_deref()
                    .map(std::ptr::from_ref)
                    == screen.as_deref().map(std::ptr::from_ref)
            });
            let last = index.map_or(0.0, |index| displays[index].1);
            if timestamp - last >= 0.1 && hooks.wake_frames_if_outside_pages(event) {
                if let Some(index) = index {
                    displays[index].1 = timestamp;
                } else {
                    displays.push((screen.as_deref().map(Weak::new), timestamp));
                }
            }
        }
        false
    })
}

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IORegistryEntryIDMatching(id: u64) -> *mut c_void;
    fn IOServiceGetMatchingService(port: u32, matching: *mut c_void) -> u32;
    fn IOObjectGetClass(object: u32, name: *mut c_char) -> i32;
    fn IORegistryEntryGetParentEntry(entry: u32, plane: *const c_char, parent: *mut u32) -> i32;
    fn IOObjectRelease(object: u32) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(value: *const c_void);
}

fn from_trackpad(event: &NSEvent, cache: &RefCell<HashMap<u64, bool>>) -> bool {
    let Some(event) = event.CGEvent() else {
        return false;
    };
    type CopyEvent = unsafe extern "C" fn(*const c_void) -> *mut c_void;
    type SenderOf = unsafe extern "C" fn(*const c_void) -> u64;
    static SYMBOLS: std::sync::LazyLock<Option<(CopyEvent, SenderOf)>> =
        std::sync::LazyLock::new(|| {
            // SAFETY: These are the optional HID symbols used by the native monitor.
            unsafe {
                let copy = libc::dlsym(libc::RTLD_DEFAULT, c"CGEventCopyIOHIDEvent".as_ptr());
                let sender = libc::dlsym(libc::RTLD_DEFAULT, c"IOHIDEventGetSenderID".as_ptr());
                if copy.is_null() || sender.is_null() {
                    return None;
                }
                Some((
                    std::mem::transmute::<*mut c_void, CopyEvent>(copy),
                    std::mem::transmute::<*mut c_void, SenderOf>(sender),
                ))
            }
        });
    let Some((copy, sender)) = *SYMBOLS else {
        return false;
    };
    // SAFETY: AppKit supplies the CGEvent. Release the owned HID event after reading the sender.
    let sender = unsafe {
        let hid = copy(std::ptr::from_ref(&*event).cast());
        if hid.is_null() {
            return false;
        }
        let id = sender(hid);
        CFRelease(hid);
        id
    };
    sender != 0
        && *cache
            .borrow_mut()
            .entry(sender)
            .or_insert_with(|| trackpad_sender(sender))
}

fn trackpad_sender(sender: u64) -> bool {
    // SAFETY: Service and parent calls return owned registry entry handles. Release every handle once.
    unsafe {
        let matching = IORegistryEntryIDMatching(sender);
        if matching.is_null() {
            return false;
        }
        let mut entry = IOServiceGetMatchingService(0, matching);
        let mut trackpad = false;
        for _ in 0..4 {
            if entry == 0 {
                break;
            }
            let mut name = [0; 128];
            if IOObjectGetClass(entry, name.as_mut_ptr()) == 0 {
                let name = CStr::from_ptr(name.as_ptr()).to_bytes();
                trackpad = name.windows(8).any(|name| name == b"Trackpad");
                if trackpad || name.windows(11).any(|name| name == b"EventDriver") {
                    break;
                }
            }
            let mut parent = 0;
            if IORegistryEntryGetParentEntry(entry, c"IOService".as_ptr(), &mut parent) != 0 {
                parent = 0;
            }
            IOObjectRelease(entry);
            entry = parent;
        }
        if entry != 0 {
            IOObjectRelease(entry);
        }
        trackpad
    }
}
