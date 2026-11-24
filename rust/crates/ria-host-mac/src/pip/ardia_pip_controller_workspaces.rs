// Ported from ardia packages/ardiacore/ios/ArdiaPiPController+Workspaces.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Lets Picture in Picture windows float over every Space, full-screen Spaces too.

use std::ptr::{self, NonNull};
use std::sync::Once;

use block2::RcBlock;
use objc2::runtime::{AnyClass, AnyObject, Bool, Imp, Sel};
use objc2::{ClassType, ffi, msg_send, sel};
use objc2_app_kit::{
    NSAccessibilitySystemFloatingWindowSubrole, NSFloatingWindowLevel, NSWindow,
    NSWindowCollectionBehavior, NSWindowLevel,
};
use objc2_foundation::NSNumber;

use super::ardia_picture_in_picture_internal::{
    is_chrome_floating_window, prevent_activation, replace_method, responds_to,
};
use super::ardia_pip_controller::Controller;

const RETAIN_NONATOMIC: usize = 1;
static FLOATS_OVER_WORKSPACES: u8 = 0;
static JOINS_FULL_SCREEN_SPACES: u8 = 0;

fn associated(window: &NSWindow, key: &'static u8) -> bool {
    // SAFETY: the key is a static address; the object is a live NSWindow.
    unsafe {
        !ffi::objc_getAssociatedObject(ptr::from_ref(window).cast(), ptr::from_ref(key).cast())
            .is_null()
    }
}

fn mark(window: &NSWindow, key: &'static u8) {
    let yes = NSNumber::numberWithBool(true);
    // SAFETY: the association retains the NSNumber; the key is a static address.
    unsafe {
        ffi::objc_setAssociatedObject(
            ptr::from_ref(window).cast_mut().cast(),
            ptr::from_ref(key).cast(),
            ptr::from_ref::<AnyObject>(&yes).cast_mut(),
            RETAIN_NONATOMIC,
        );
    }
}

fn floats_over_workspaces(window: &NSWindow) -> bool {
    if Controller::for_window(window).is_some() || associated(window, &FLOATS_OVER_WORKSPACES) {
        return true;
    }
    if !is_chrome_floating_window(window) {
        return false;
    }
    mark(window, &FLOATS_OVER_WORKSPACES);
    true
}

fn block_imp<A: block2::BlockFn + ?Sized>(block: &RcBlock<A>) -> Imp {
    // SAFETY: the runtime copies the block; its arguments are `self` and the method arguments.
    unsafe { ffi::imp_implementationWithBlock(RcBlock::as_ptr(block).cast()) }
}

fn answer_system_floating_subrole(window_class: &AnyClass) {
    let subrole = sel!(accessibilitySubrole);
    let Some(inherited) = window_class.instance_method(subrole) else {
        return;
    };
    // SAFETY: `accessibilitySubrole` takes no arguments and returns an NSString or nil.
    let chrome: unsafe extern "C-unwind" fn(NonNull<NSWindow>, Sel) -> *const AnyObject =
        unsafe { std::mem::transmute(inherited.implementation()) };
    let floating = RcBlock::new(move |window: NonNull<NSWindow>| -> *const AnyObject {
        // SAFETY: the runtime gives a live window as `self`.
        if floats_over_workspaces(unsafe { window.as_ref() }) {
            // SAFETY: an AppKit constant string.
            ptr::from_ref::<AnyObject>(unsafe { NSAccessibilitySystemFloatingWindowSubrole })
        } else {
            // SAFETY: the inherited implementation of the same selector.
            unsafe { chrome(window, subrole) }
        }
    });
    replace_method(window_class, subrole, block_imp(&floating));
}

fn join_before_first_shown(window: &NSWindow) {
    if window.isVisible()
        || window.level() < NSFloatingWindowLevel
        || !window
            .collectionBehavior()
            .contains(NSWindowCollectionBehavior::CanJoinAllSpaces)
    {
        return;
    }
    join_full_screen_spaces(window);
}

fn marked_to_join_full_screen_spaces(window: &NSWindow) -> bool {
    associated(window, &JOINS_FULL_SCREEN_SPACES)
}

fn answer_nonactivating_panel_for_marked_windows() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let selector = sel!(_isNonactivatingPanel);
        let class = NSWindow::class();
        let Some(method) = class.instance_method(selector) else {
            return;
        };
        // SAFETY: `_isNonactivatingPanel` takes no arguments and returns a BOOL.
        let original: unsafe extern "C-unwind" fn(NonNull<NSWindow>, Sel) -> Bool =
            unsafe { std::mem::transmute(method.implementation()) };
        let block = RcBlock::new(move |window: NonNull<NSWindow>| -> Bool {
            // SAFETY: the runtime gives a live window as `self`; `original` has this signature.
            Bool::new(
                marked_to_join_full_screen_spaces(unsafe { window.as_ref() })
                    || unsafe { original(window, selector) }.as_bool(),
            )
        });
        replace_method(class, selector, block_imp(&block));
    });
}

fn join_when_chrome_makes_it_float(window_class: &AnyClass) {
    let behavior_selector = sel!(setCollectionBehavior:);
    if let Some(method) = window_class.instance_method(behavior_selector) {
        // SAFETY: `setCollectionBehavior:` takes one NSWindowCollectionBehavior.
        let original: unsafe extern "C-unwind" fn(
            NonNull<NSWindow>,
            Sel,
            NSWindowCollectionBehavior,
        ) = unsafe { std::mem::transmute(method.implementation()) };
        let block = RcBlock::new(
            move |window: NonNull<NSWindow>, behavior: NSWindowCollectionBehavior| {
                // SAFETY: the runtime gives a live window as `self`; `original` has this signature.
                unsafe { original(window, behavior_selector, behavior) };
                join_before_first_shown(unsafe { window.as_ref() });
            },
        );
        replace_method(window_class, behavior_selector, block_imp(&block));
    }
    let level_selector = sel!(setLevel:);
    if let Some(method) = window_class.instance_method(level_selector) {
        // SAFETY: `setLevel:` takes one NSWindowLevel.
        let original: unsafe extern "C-unwind" fn(NonNull<NSWindow>, Sel, NSWindowLevel) =
            unsafe { std::mem::transmute(method.implementation()) };
        let block = RcBlock::new(move |window: NonNull<NSWindow>, level: NSWindowLevel| {
            // SAFETY: the runtime gives a live window as `self`; `original` has this signature.
            unsafe { original(window, level_selector, level) };
            join_before_first_shown(unsafe { window.as_ref() });
        });
        replace_method(window_class, level_selector, block_imp(&block));
    }
}

pub fn joins_full_screen_spaces(window: &NSWindow) -> bool {
    let needed = NSWindowCollectionBehavior::CanJoinAllSpaces
        | NSWindowCollectionBehavior::FullScreenAuxiliary;
    let nonactivating = responds_to(window, sel!(_isNonactivatingPanel)) && {
        // SAFETY: the private getter returns a BOOL.
        unsafe { msg_send![window, _isNonactivatingPanel] }
    };
    nonactivating && window.collectionBehavior() & needed == needed
}

pub fn join_full_screen_spaces(window: &NSWindow) {
    let exclusive = NSWindowCollectionBehavior::Primary
        | NSWindowCollectionBehavior::Auxiliary
        | NSWindowCollectionBehavior::FullScreenPrimary
        | NSWindowCollectionBehavior::FullScreenNone;
    let joined = (window.collectionBehavior() & !exclusive)
        | NSWindowCollectionBehavior::CanJoinAllSpaces
        | NSWindowCollectionBehavior::FullScreenAuxiliary;
    if joined != window.collectionBehavior() {
        window.setCollectionBehavior(joined);
    }
    mark(window, &JOINS_FULL_SCREEN_SPACES);
}

pub fn float_over_active_space(window: &NSWindow) {
    answer_nonactivating_panel_for_marked_windows();
    window.setCollectionBehavior(
        window.collectionBehavior() & !NSWindowCollectionBehavior::MoveToActiveSpace,
    );
    join_full_screen_spaces(window);
    prevent_activation(window);
}

/// `+[ArdiaPiPController(Workspaces) load]`.
pub fn install() {
    static ONCE: Once = Once::new();
    ONCE.call_once(install_once);
}

fn install_once() {
    let Some(frameless) = AnyClass::get(c"NativeWidgetMacFramelessNSWindow") else {
        return;
    };
    answer_system_floating_subrole(frameless);
    answer_nonactivating_panel_for_marked_windows();
    join_when_chrome_makes_it_float(frameless);
}
