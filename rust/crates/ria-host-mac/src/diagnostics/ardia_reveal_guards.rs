// Ported from ardia packages/ardiacore/ios/ArdiaRevealGuards.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Changes from Ardia: `install` takes the reveal-input answers as `RevealHooks`, where Ardia calls the
//! `ArdiaWebView` class methods. The walk from an accessibility element to its view, the first half of
//! `+[ArdiaWebView refusesInputAheadOfSwitch:]`, is here; the hooks answer for the view.
//! The native view does not supply `tabs::ardia_web_view_reveal_input::RevealInput` to the guards.
//! Connect it through `RevealHooks` and call `install` where Ardia's input monitors call `installRevealGuards`.

use std::cell::{Cell, RefCell};
use std::ffi::CStr;
use std::ptr::{self, NonNull};
use std::rc::Rc;
use std::sync::OnceLock;

use block2::{Block, RcBlock};
use objc2::ffi;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyClass, AnyObject, Bool, Imp, Method, Sel};
use objc2::{MainThreadMarker, Message, msg_send, sel};
use objc2_app_kit::{NSAccessibilityCustomAction, NSApplication, NSDragOperation, NSEvent, NSView};
use objc2_foundation::{NSArray, NSRange};

use crate::tabs::ardia_web_view_reveal_input::skips_event_monitors;

pub trait RevealHooks {
    /// `+[ArdiaWebView refusesInputAheadOfSwitch:]` for a view: it is in a tab page of a window with a reveal gap.
    fn refuses_view(&self, view: &NSView) -> bool;
    /// `+[ArdiaWebView anyRevealGap]`.
    fn any_gap(&self) -> bool;
    /// `+[ArdiaWebView swallowsAheadOfSwitch:]`.
    fn swallows_ahead(&self, event: &NSEvent) -> bool;
}

const ACCESSIBILITY_CLASSES: [&CStr; 2] = [c"BrowserAccessibilityCocoa", c"AXPlatformNodeCocoa"];

thread_local! {
    static HOOKS: RefCell<Option<Rc<dyn RevealHooks>>> = const { RefCell::new(None) };
    static GUARDED: Cell<usize> = const { Cell::new(0) };
}

static RESOLVE_UNGUARDED: OnceLock<Imp> = OnceLock::new();

/// `+[ArdiaWebView guardedAccessibilityCalls]`.
pub fn guarded_accessibility_calls() -> usize {
    GUARDED.get()
}

fn hooks() -> Option<Rc<dyn RevealHooks>> {
    HOOKS.with_borrow(Clone::clone)
}

fn holder_view(target: *mut AnyObject) -> Option<Retained<NSView>> {
    // SAFETY: Objective-C hands the target retained for the call.
    let mut holder = unsafe { Retained::retain(target) }?;
    for _ in 0..256 {
        let holder_ref: &AnyObject = &holder;
        if let Some(view) = holder_ref.downcast_ref::<NSView>() {
            return Some(view.retain());
        }
        // SAFETY: The selector is checked before the call, which returns an object or nil.
        let responds: bool =
            unsafe { msg_send![&*holder, respondsToSelector: sel!(accessibilityParent)] };
        if !responds {
            return None;
        }
        let parent: Option<Retained<AnyObject>> =
            unsafe { msg_send![&*holder, accessibilityParent] };
        holder = parent?;
    }
    None
}

fn refused(target: *mut AnyObject) -> bool {
    let Some(hooks) = hooks() else {
        return false;
    };
    holder_view(target).is_some_and(|view| hooks.refuses_view(&view))
}

fn object(object: NonNull<AnyObject>) -> *mut AnyObject {
    object.as_ptr()
}

fn changes_the_page(selector: Sel) -> bool {
    let name = selector.name().to_bytes();
    name.starts_with(b"accessibilityPerform")
        || name.starts_with(b"setAccessibility")
        || name == b"accessibilitySetValue:forAttribute:"
}

unsafe fn imp_of<F: ?Sized>(block: &Block<F>) -> Imp {
    // SAFETY: The runtime copies the block for the new method.
    unsafe { ffi::imp_implementationWithBlock(ptr::from_ref(block).cast_mut().cast()) }
}

/// `ardia_runtime::ReplaceMethod`: adds the method to `class` when it is inherited, else replaces it.
fn replace_method(class: Option<&AnyClass>, selector: Sel, make: impl FnOnce(Imp) -> Imp) {
    let Some(method) = class.and_then(|class| class.instance_method(selector)) else {
        return;
    };
    let class = class.expect("method came from the class");
    let ours = make(method.implementation());
    // SAFETY: The method and class are live runtime objects, and `ours` has the method's signature.
    unsafe {
        let types = ffi::method_getTypeEncoding(method);
        if !ffi::class_addMethod(ptr::from_ref(class).cast_mut(), selector, ours, types).as_bool() {
            ffi::method_setImplementation(method, ours);
        }
    }
}

type Resolve = unsafe extern "C-unwind" fn(
    *mut AnyObject,
    Sel,
    Sel,
    *mut AnyObject,
    *mut AnyObject,
) -> *mut AnyObject;

fn guard_application(class: &AnyClass) {
    let resolve = sel!(targetForAction:to:from:);
    replace_method(Some(class), resolve, |original| {
        let _ = RESOLVE_UNGUARDED.set(original);
        // SAFETY: `original` is the implementation of `targetForAction:to:from:`.
        let original: Resolve = unsafe { std::mem::transmute(original) };
        let block = RcBlock::new(
            move |app: NonNull<AnyObject>,
                  action: Sel,
                  target: *mut AnyObject,
                  sender: *mut AnyObject| {
                let found = unsafe { original(object(app), resolve, action, target, sender) };
                if refused(found) {
                    ptr::null_mut()
                } else {
                    found
                }
            },
        );
        unsafe { imp_of(&block) }
    });
    let send = sel!(sendAction:to:from:);
    replace_method(Some(class), send, |original| {
        type Send = unsafe extern "C-unwind" fn(
            *mut AnyObject,
            Sel,
            Sel,
            *mut AnyObject,
            *mut AnyObject,
        ) -> Bool;
        // SAFETY: `original` is the implementation of `sendAction:to:from:`.
        let original: Send = unsafe { std::mem::transmute(original) };
        let block = RcBlock::new(
            move |app: NonNull<AnyObject>,
                  action: Sel,
                  target: *mut AnyObject,
                  sender: *mut AnyObject| {
                if hooks().is_some_and(|hooks| hooks.any_gap()) {
                    let found = if target.is_null() {
                        resolve_unguarded(app, action, sender)
                    } else {
                        target
                    };
                    if refused(found) {
                        return Bool::NO;
                    }
                }
                unsafe { original(object(app), send, action, target, sender) }
            },
        );
        unsafe { imp_of(&block) }
    });
    guard_send_event(class);
}

fn resolve_unguarded(
    app: NonNull<AnyObject>,
    action: Sel,
    sender: *mut AnyObject,
) -> *mut AnyObject {
    RESOLVE_UNGUARDED.get().map_or(ptr::null_mut(), |imp| {
        // SAFETY: The stored implementation is the original `targetForAction:to:from:`.
        let resolve: Resolve = unsafe { std::mem::transmute(*imp) };
        unsafe {
            resolve(
                object(app),
                sel!(targetForAction:to:from:),
                action,
                ptr::null_mut(),
                sender,
            )
        }
    })
}

fn guard_send_event(class: &AnyClass) {
    let dispatch = sel!(sendEvent:);
    replace_method(Some(class), dispatch, |original| {
        // SAFETY: `original` is the implementation of `sendEvent:`.
        let original: unsafe extern "C-unwind" fn(*mut AnyObject, Sel, NonNull<NSEvent>) =
            unsafe { std::mem::transmute(original) };
        let block = RcBlock::new(move |app: NonNull<AnyObject>, event: NonNull<NSEvent>| {
            // SAFETY: AppKit lends the event for the call.
            let shown = unsafe { event.as_ref() };
            if skips_event_monitors(shown)
                && hooks().is_some_and(|hooks| hooks.swallows_ahead(shown))
            {
                return;
            }
            unsafe { original(object(app), dispatch, event) }
        });
        unsafe { imp_of(&block) }
    });
}

fn guard_page(widget: Option<&AnyClass>) {
    let requestor = sel!(validRequestorForSendType:returnType:);
    replace_method(widget, requestor, |original| {
        type Requestor = unsafe extern "C-unwind" fn(
            *mut AnyObject,
            Sel,
            *mut AnyObject,
            *mut AnyObject,
        ) -> *mut AnyObject;
        // SAFETY: `original` is the implementation of `validRequestorForSendType:returnType:`.
        let original: Requestor = unsafe { std::mem::transmute(original) };
        let block = RcBlock::new(
            move |page: NonNull<AnyObject>, send: *mut AnyObject, give: *mut AnyObject| {
                if refused(object(page)) {
                    ptr::null_mut()
                } else {
                    unsafe { original(object(page), requestor, send, give) }
                }
            },
        );
        unsafe { imp_of(&block) }
    });
    let touch_bar = sel!(makeTouchBar);
    replace_method(widget, touch_bar, |original| {
        // SAFETY: `original` is the implementation of `makeTouchBar`.
        let original: unsafe extern "C-unwind" fn(*mut AnyObject, Sel) -> *mut AnyObject =
            unsafe { std::mem::transmute(original) };
        let block = RcBlock::new(move |page: NonNull<AnyObject>| {
            if refused(object(page)) {
                ptr::null_mut()
            } else {
                unsafe { original(object(page), touch_bar) }
            }
        });
        unsafe { imp_of(&block) }
    });
    let focused = sel!(setAccessibilityFocused:);
    replace_method(widget, focused, |original| {
        // SAFETY: `original` is the implementation of `setAccessibilityFocused:`.
        let original: unsafe extern "C-unwind" fn(*mut AnyObject, Sel, Bool) =
            unsafe { std::mem::transmute(original) };
        let block = RcBlock::new(move |page: NonNull<AnyObject>, is_focused: Bool| {
            if !refused(object(page)) {
                unsafe { original(object(page), focused, is_focused) }
            }
        });
        unsafe { imp_of(&block) }
    });
}

fn guard_drops(contents: Option<&AnyClass>) {
    for drag in [sel!(draggingEntered:), sel!(draggingUpdated:)] {
        replace_method(contents, drag, |original| {
            // SAFETY: `original` is the implementation of a dragging destination call.
            let original: unsafe extern "C-unwind" fn(
                *mut AnyObject,
                Sel,
                *mut AnyObject,
            ) -> NSDragOperation = unsafe { std::mem::transmute(original) };
            let block = RcBlock::new(move |page: NonNull<AnyObject>, info: *mut AnyObject| {
                if refused(object(page)) {
                    NSDragOperation::None
                } else {
                    unsafe { original(object(page), drag, info) }
                }
            });
            unsafe { imp_of(&block) }
        });
    }
    let drop = sel!(performDragOperation:);
    replace_method(contents, drop, |original| {
        // SAFETY: `original` is the implementation of `performDragOperation:`.
        let original: unsafe extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject) -> Bool =
            unsafe { std::mem::transmute(original) };
        let block = RcBlock::new(move |page: NonNull<AnyObject>, info: *mut AnyObject| {
            Bool::new(
                !refused(object(page)) && unsafe { original(object(page), drop, info) }.as_bool(),
            )
        });
        unsafe { imp_of(&block) }
    });
}

/// `GuardedCall`: a guarded implementation for the method's signature, or `None` when no shape fits.
fn guarded_call(selector: Sel, original: Imp, method: &Method) -> Option<Imp> {
    let result = method.return_type().to_bytes().first().copied();
    let answers = matches!(result, Some(b'B' | b'c'));
    if !answers && result != Some(b'v') {
        return None;
    }
    let count = method.arguments_count().saturating_sub(2);
    let argument = |index| {
        method
            .argument_type(index)
            .map(|kind| kind.to_bytes().to_vec())
            .unwrap_or_default()
    };
    let (first, second) = (argument(2), argument(3));
    match (count, answers) {
        (0, true) => Some(answer_guard(selector, original)),
        (0, false) => Some(void_guard(selector, original)),
        (_, true) => None,
        (1, false) => one_argument_guard(selector, original, &first),
        (2, false) if first == b"@" && second == b"@" => Some(two_object_guard(selector, original)),
        _ => None,
    }
}

fn answer_guard(selector: Sel, original: Imp) -> Imp {
    // SAFETY: `guarded_call` checked the method returns a BOOL and takes no arguments.
    let original: unsafe extern "C-unwind" fn(*mut AnyObject, Sel) -> Bool =
        unsafe { std::mem::transmute(original) };
    let block = RcBlock::new(move |element: NonNull<AnyObject>| {
        Bool::new(
            !refused(object(element)) && unsafe { original(object(element), selector) }.as_bool(),
        )
    });
    unsafe { imp_of(&block) }
}

fn void_guard(selector: Sel, original: Imp) -> Imp {
    // SAFETY: `guarded_call` checked the method returns void and takes no arguments.
    let original: unsafe extern "C-unwind" fn(*mut AnyObject, Sel) =
        unsafe { std::mem::transmute(original) };
    let block = RcBlock::new(move |element: NonNull<AnyObject>| {
        if !refused(object(element)) {
            unsafe { original(object(element), selector) }
        }
    });
    unsafe { imp_of(&block) }
}

fn one_argument_guard(selector: Sel, original: Imp, first: &[u8]) -> Option<Imp> {
    if first == b"@" {
        return Some(guard_with::<*mut AnyObject>(selector, original));
    }
    if first == b"B" || first == b"c" {
        return Some(guard_with::<Bool>(selector, original));
    }
    first
        .starts_with(b"{_NSRange=")
        .then(|| guard_with::<NSRange>(selector, original))
}

fn guard_with<T: objc2::encode::EncodeArgument + 'static>(selector: Sel, original: Imp) -> Imp {
    // SAFETY: `one_argument_guard` checked the method returns void and takes one `T`.
    let original: unsafe extern "C-unwind" fn(*mut AnyObject, Sel, T) =
        unsafe { std::mem::transmute(original) };
    let block = RcBlock::new(move |element: NonNull<AnyObject>, value: T| {
        if !refused(object(element)) {
            unsafe { original(object(element), selector, value) }
        }
    });
    unsafe { imp_of(&block) }
}

fn two_object_guard(selector: Sel, original: Imp) -> Imp {
    // SAFETY: `guarded_call` checked the method returns void and takes two objects.
    let original: unsafe extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject, *mut AnyObject) =
        unsafe { std::mem::transmute(original) };
    let block = RcBlock::new(
        move |element: NonNull<AnyObject>, value: *mut AnyObject, name: *mut AnyObject| {
            if !refused(object(element)) {
                unsafe { original(object(element), selector, value, name) }
            }
        },
    );
    unsafe { imp_of(&block) }
}

fn guard_accessibility_of(class: &AnyClass) -> usize {
    let mut guarded = 0;
    for method in class.instance_methods().iter() {
        let selector = method.name();
        if !changes_the_page(selector) {
            continue;
        }
        if let Some(guard) = guarded_call(selector, method.implementation(), method) {
            // SAFETY: `guard` has the method's signature.
            unsafe { ffi::method_setImplementation(&**method, guard) };
            guarded += 1;
        }
    }
    guarded
}

fn guarded_custom_actions(
    element: NonNull<AnyObject>,
    actions: *mut NSArray<NSAccessibilityCustomAction>,
) -> *mut NSArray<NSAccessibilityCustomAction> {
    // SAFETY: The original returns the array autoreleased or nil.
    let Some(list) = (unsafe { actions.as_ref() }) else {
        return actions;
    };
    let vendor = Weak::from(unsafe { element.as_ref() });
    for action in list.iter() {
        // SAFETY: A non-null handler is a live block, which `copy` retains.
        let Some(perform) = (unsafe { RcBlock::copy(action.handler()) }) else {
            continue;
        };
        let vendor = vendor.clone();
        let handler = RcBlock::new(move || {
            let holder = vendor.load();
            Bool::new(
                holder.is_some_and(|holder| !refused(Retained::as_ptr(&holder).cast_mut()))
                    && perform.call(()).as_bool(),
            )
        });
        action.setHandler(Some(&handler));
    }
    actions
}

fn guard_custom_actions(class: &AnyClass) -> usize {
    let custom_actions = sel!(accessibilityCustomActions);
    let mut guarded = 0;
    for method in class
        .instance_methods()
        .iter()
        .filter(|method| method.name() == custom_actions)
    {
        type Actions = unsafe extern "C-unwind" fn(
            *mut AnyObject,
            Sel,
        )
            -> *mut NSArray<NSAccessibilityCustomAction>;
        // SAFETY: The method is `accessibilityCustomActions`, which returns an array.
        let original: Actions = unsafe { std::mem::transmute(method.implementation()) };
        let block = RcBlock::new(move |element: NonNull<AnyObject>| {
            guarded_custom_actions(element, unsafe {
                original(object(element), custom_actions)
            })
        });
        // SAFETY: The block has the method's signature.
        unsafe { ffi::method_setImplementation(&**method, imp_of(&block)) };
        guarded += 1;
    }
    guarded
}

/// `+[ArdiaWebView installRevealGuards]`. The hooks of the first call stay.
pub fn install(mtm: MainThreadMarker, hooks: Rc<dyn RevealHooks>) {
    if HOOKS.with_borrow(Option::is_some) {
        return;
    }
    HOOKS.set(Some(hooks));
    guard_application(NSApplication::sharedApplication(mtm).class());
    guard_page(AnyClass::get(c"RenderWidgetHostViewCocoa"));
    guard_drops(AnyClass::get(c"WebContentsViewCocoa"));
    let classes = ACCESSIBILITY_CLASSES.map(AnyClass::get);
    let mut guarded: usize = classes
        .iter()
        .flatten()
        .map(|class| guard_accessibility_of(class))
        .sum();
    guarded += classes
        .iter()
        .flatten()
        .map(|class| guard_custom_actions(class))
        .sum::<usize>();
    GUARDED.set(GUARDED.get() + guarded);
}
