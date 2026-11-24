// Ported from ardia packages/ardiacore/ios/ArdiaActivation.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.
// The launch notifications also use ArdiaHost+Launch.mm.

use std::cell::RefCell;
use std::ptr::NonNull;
use std::rc::{Rc, Weak};

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{MainThreadMarker, msg_send};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDidBecomeActiveNotification,
    NSApplicationDidFinishLaunchingNotification, NSApplicationWillFinishLaunchingNotification,
};
use objc2_foundation::{
    NSAppleEventDescriptor, NSAppleEventManager, NSNotification, NSNotificationCenter,
    NSObjectProtocol, NSString,
};

use super::launch_sequence::{LaunchReason, LaunchSequence};
use super::open_urls;

thread_local! {
    static LAUNCH: RefCell<Weak<LaunchSequence>> = const { RefCell::new(Weak::new()) };
    static BACKGROUND: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(super) struct Observers([Retained<ProtocolObject<dyn NSObjectProtocol>>; 3]);

impl Observers {
    pub fn new(launch: &Rc<LaunchSequence>, background: bool) -> Self {
        LAUNCH.with(|slot| slot.replace(Rc::downgrade(launch)));
        BACKGROUND.set(background);
        let will = RcBlock::new(|_: NonNull<NSNotification>| {
            if let Some(mtm) = MainThreadMarker::new() {
                open_urls::install(mtm);
            }
            if BACKGROUND.get()
                && let Some(mtm) = MainThreadMarker::new()
            {
                NSApplication::sharedApplication(mtm)
                    .setActivationPolicy(NSApplicationActivationPolicy::Prohibited);
            }
        });
        let did = RcBlock::new(|_: NonNull<NSNotification>| {
            if let Some(mtm) = MainThreadMarker::new() {
                open_urls::install(mtm);
            }
            with_launch(|launch, mtm| {
                launch.app_kit_finished(launch_reason(mtm));
            })
        });
        let active = RcBlock::new(|_: NonNull<NSNotification>| {
            if BACKGROUND.get()
                && let Some(mtm) = MainThreadMarker::new()
            {
                NSApplication::sharedApplication(mtm).deactivate();
            }
        });
        let center = NSNotificationCenter::defaultCenter();
        // SAFETY: the blocks capture no state and access AppKit only on the main thread.
        unsafe {
            Self([
                center.addObserverForName_object_queue_usingBlock(
                    Some(NSApplicationWillFinishLaunchingNotification),
                    None,
                    None,
                    &will,
                ),
                center.addObserverForName_object_queue_usingBlock(
                    Some(NSApplicationDidFinishLaunchingNotification),
                    None,
                    None,
                    &did,
                ),
                center.addObserverForName_object_queue_usingBlock(
                    Some(NSApplicationDidBecomeActiveNotification),
                    None,
                    None,
                    &active,
                ),
            ])
        }
    }
}

impl Drop for Observers {
    fn drop(&mut self) {
        let center = NSNotificationCenter::defaultCenter();
        for observer in &self.0 {
            // SAFETY: each token came from this notification center.
            unsafe { center.removeObserver((**observer).as_ref()) };
        }
        LAUNCH.with(|slot| slot.replace(Weak::new()));
        BACKGROUND.set(false);
    }
}

fn with_launch(f: impl FnOnce(&LaunchSequence, MainThreadMarker)) {
    if let Some(mtm) = MainThreadMarker::new()
        && let Some(launch) = LAUNCH.with(|slot| slot.borrow().upgrade())
    {
        f(&launch, mtm);
    }
}

fn launch_reason(mtm: MainThreadMarker) -> LaunchReason {
    if NSApplication::sharedApplication(mtm).isHidden() {
        return LaunchReason::Hidden;
    }
    if let Some(event) = NSAppleEventManager::sharedAppleEventManager().currentAppleEvent() {
        // SAFETY: these are NSAppleEventDescriptor selectors with four-byte Apple Event codes.
        let class: u32 = unsafe { msg_send![&*event, eventClass] };
        let id: u32 = unsafe { msg_send![&*event, eventID] };
        if class != u32::from_be_bytes(*b"aevt") || id != u32::from_be_bytes(*b"oapp") {
            return LaunchReason::Documents;
        }
        let property: Option<Retained<NSAppleEventDescriptor>> =
            unsafe { msg_send![&*event, paramDescriptorForKeyword: u32::from_be_bytes(*b"prdt")] };
        if property.is_some_and(|property| {
            let code: u32 = unsafe { msg_send![&*property, enumCodeValue] };
            code == u32::from_be_bytes(*b"lgit")
        }) {
            return LaunchReason::Hidden;
        }
    }
    LaunchReason::Normal
}

pub(super) fn dark(mtm: MainThreadMarker) -> bool {
    use objc2_app_kit::{NSAppearanceNameAqua, NSAppearanceNameDarkAqua};
    use objc2_foundation::NSArray;
    // SAFETY: AppKit exports immutable appearance-name objects.
    let (dark, light) = unsafe { (NSAppearanceNameDarkAqua, NSAppearanceNameAqua) };
    let appearance = NSApplication::sharedApplication(mtm).effectiveAppearance();
    let names = NSArray::from_slice(&[dark, light]);
    appearance
        .bestMatchFromAppearancesWithNames(&names)
        .as_deref()
        == Some::<&NSString>(dark)
}
