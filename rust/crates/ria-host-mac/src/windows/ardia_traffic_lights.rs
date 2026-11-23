// Ported from ardia packages/ardiacore/ios/ArdiaTrafficLights.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::ptr;
use std::rc::Rc;
use std::time::Duration;

use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyClass, AnyObject, Imp, NSObjectProtocol, Sel};
use objc2::{ffi, msg_send, sel};
use objc2_app_kit::{
    NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSColor, NSView, NSWindow,
    NSWindowButton, NSWindowDidExitFullScreenNotification, NSWindowDidResizeNotification,
    NSWindowStyleMask, NSWindowTabbingMode, NSWindowTitleVisibility,
};
use objc2_foundation::{
    NSArray, NSDictionary, NSNotification, NSNotificationCenter, NSNumber, NSPoint, NSRect, NSSize,
    NSString, ns_string,
};
use ria_engine::call::main_queue;

use super::ardia_interface_scale::scale;

const BUTTONS: [NSWindowButton; 3] = [
    NSWindowButton::CloseButton,
    NSWindowButton::MiniaturizeButton,
    NSWindowButton::ZoomButton,
];
const RETAIN_NONATOMIC: usize = 1;

thread_local! {
    static INSET_WINDOWS: RefCell<HashMap<*const NSWindow, std::rc::Weak<TrafficLights>>> =
        RefCell::default();
    static LAYOUT_HOOKS: RefCell<HashSet<usize>> = RefCell::default();
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Placement {
    pub center: Option<NSPoint>,
    pub trailing: bool,
}

impl Placement {
    fn center(self, window_width: f64, size: NSSize) -> NSPoint {
        let scale = scale();
        let at = self
            .center
            .unwrap_or_else(|| NSPoint::new(18. + size.width / 2., 20. + size.height / 2.));
        let x = at.x * scale;
        NSPoint::new(
            if self.trailing && self.center.is_some() {
                window_width - x
            } else {
                x
            },
            at.y * scale,
        )
    }
}

pub struct TrafficLights {
    window: Weak<NSWindow>,
    window_key: *const NSWindow,
    placement: Cell<Placement>,
    moved: Cell<Option<Placement>>,
    move_turn: Cell<u64>,
    observers: RefCell<Vec<Retained<AnyObject>>>,
}

impl TrafficLights {
    pub(super) fn configure(window: &NSWindow) -> Rc<Self> {
        let frame = window.frame();
        window.setStyleMask(window.styleMask() | NSWindowStyleMask::FullSizeContentView);
        if window.frame() != frame {
            window.setFrame_display(frame, false);
        }
        window.setTitlebarAppearsTransparent(true);
        window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
        window.setTitle(ns_string!("Ria"));
        window.setContentMinSize(NSSize::new(500., 460.));
        let provider = RcBlock::new(|appearance: std::ptr::NonNull<NSAppearance>| {
            // SAFETY: AppKit supplies the current appearance, and both names are AppKit constants.
            let dark = unsafe {
                let names = NSArray::from_slice(&[NSAppearanceNameDarkAqua, NSAppearanceNameAqua]);
                appearance
                    .as_ref()
                    .bestMatchFromAppearancesWithNames(&names)
                    .is_some_and(|name| name.isEqualToString(NSAppearanceNameDarkAqua))
            };
            let (r, g, b) = if dark {
                (0.17, 0.12, 0.14)
            } else {
                (0.93, 0.91, 0.90)
            };
            let color = Retained::autorelease_return(NSColor::colorWithSRGBRed_green_blue_alpha(
                r, g, b, 1.,
            ));
            std::ptr::NonNull::new(color).expect("AppKit color is not null")
        });
        // SAFETY: The callback returns an autoreleased NSColor, conforming to the provider contract.
        let background = unsafe { NSColor::colorWithName_dynamicProvider(None, &provider) };
        window.setBackgroundColor(Some(&background));
        // SAFETY: The engine and controller own the window, not the close operation.
        unsafe {
            window.setReleasedWhenClosed(false);
        }
        window.setTabbingMode(NSWindowTabbingMode::Disallowed);
        Self::inset(window)
    }

    fn inset(window: &NSWindow) -> Rc<Self> {
        let lights = Rc::new(Self {
            window: Weak::from(window),
            window_key: ptr::from_ref(window),
            placement: Cell::default(),
            moved: Cell::default(),
            move_turn: Cell::default(),
            observers: RefCell::default(),
        });
        INSET_WINDOWS.with(|windows| {
            windows
                .borrow_mut()
                .insert(lights.window_key, Rc::downgrade(&lights));
        });
        if let Some(class) = AnyClass::get(c"NSLocalWindowSharingWindow") {
            install_after_layout(class, sel!(_syncToWindowControls), update_sharing_window);
        }
        // SAFETY: These notifications carry the NSWindow supplied as the object filter.
        for name in unsafe {
            [
                NSWindowDidResizeNotification,
                NSWindowDidExitFullScreenNotification,
                ns_string!("ArdiaInterfaceScaleDidChange"),
            ]
        } {
            let weak = Rc::downgrade(&lights);
            let callback = RcBlock::new(move |_: std::ptr::NonNull<NSNotification>| {
                if let Some(lights) = weak.upgrade() {
                    if let Some(placement) = lights.moved.get() {
                        lights.set_placement(placement);
                    } else {
                        lights.retile();
                    }
                }
            });
            // SAFETY: The block receives an NSNotification and does not outlive its observer token.
            let observer = unsafe {
                NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                    Some(name),
                    (!name.isEqualToString(ns_string!("ArdiaInterfaceScaleDidChange")))
                        .then_some(window),
                    None,
                    &callback,
                )
            };
            lights.observers.borrow_mut().push(observer.into());
        }
        lights.retile();
        lights
    }

    pub fn set_placement(&self, placement: Placement) {
        let changed = self.placement.replace(placement) != placement;
        if self.moved.take().is_some() {
            if let Some(window) = self.window.load() {
                translate_buttons(&window, 0.);
            }
            self.retile();
        } else if changed {
            self.retile();
        }
    }

    pub(super) fn move_to(self: &Rc<Self>, placement: Placement) {
        let Some(window) = self.window.load() else {
            return;
        };
        let laid_out = self.placement.get();
        let Some(close) = window.standardWindowButton(NSWindowButton::CloseButton) else {
            return self.set_placement(placement);
        };
        let width = window.frame().size.width;
        let size = close.frame().size;
        let from = laid_out.center(width, size);
        let to = placement.center(width, size);
        if laid_out.trailing != placement.trailing
            || from.y != to.y
            || from.x == to.x
            || window.styleMask().contains(NSWindowStyleMask::FullScreen)
            || !translate_buttons(&window, to.x - from.x)
        {
            self.set_placement(placement);
            return;
        }
        self.moved.set(Some(placement));
        if let Some(frame) = frame_view(&window) {
            tuck_buttons(&window, &frame, to.x - from.x);
        }
        let turn = self.move_turn.get().wrapping_add(1);
        self.move_turn.set(turn);
        let weak = Rc::downgrade(self);
        main_queue::after(
            Duration::from_millis(50),
            Box::new(move || {
                if let Some(lights) = weak.upgrade()
                    && lights.move_turn.get() == turn
                    && let Some(placement) = lights.moved.get()
                {
                    lights.set_placement(placement);
                }
            }),
        );
    }

    fn retile(&self) {
        let Some(window) = self.window.load() else {
            return;
        };
        let Some(frame) = frame_view(&window) else {
            return;
        };
        install_close_origin(frame.class());
        install_after_layout(
            frame.class(),
            sel!(_positionSharingIndicator),
            update_indicator,
        );
        let placement = self.placement.get();
        if placement.center.is_some() {
            install_titlebar_extension(frame.class());
        }
        for name in [
            sel!(_updateTitlebarContainerViewFrameIfNecessary),
            sel!(_updateButtonPositions),
            sel!(_updateMouseTracking),
            sel!(_positionSharingIndicator),
        ] {
            send_no_args(&frame, name);
        }
        sync_sharing_windows(&window);
        tuck_buttons(&window, &frame, 0.);
    }
}

fn tuck_buttons(window: &NSWindow, frame: &NSView, dx: f64) {
    let buttons = BUTTONS.map(|kind| window.standardWindowButton(kind));
    let Some(close) = buttons.iter().flatten().next() else {
        return;
    };
    let outside = if window.styleMask().contains(NSWindowStyleMask::FullScreen) {
        false
    } else {
        let left = close.convertRect_toView(close.bounds(), Some(frame));
        let right =
            buttons
                .iter()
                .flatten()
                .fold(left.origin.x + left.size.width, |right, button| {
                    let bounds = button.convertRect_toView(button.bounds(), Some(frame));
                    right.max(bounds.origin.x + bounds.size.width)
                });
        let bounds = frame.bounds();
        right + dx <= 0.
            || left.origin.x + dx >= bounds.size.width
            || if frame.isFlipped() {
                left.origin.y + left.size.height <= 0.
            } else {
                left.origin.y >= bounds.size.height
            }
    };
    for button in buttons.iter().flatten() {
        button.setHidden(outside);
    }
}

impl Drop for TrafficLights {
    fn drop(&mut self) {
        INSET_WINDOWS.with(|windows| windows.borrow_mut().remove(&self.window_key));
        let center = NSNotificationCenter::defaultCenter();
        for observer in self.observers.get_mut() {
            // SAFETY: These observer tokens came from this notification center.
            unsafe {
                center.removeObserver(observer);
            }
        }
    }
}

fn install_titlebar_extension(class: &'static AnyClass) {
    let selector = sel!(_currentTitlebarContainerViewFrame);
    install_layout_hook(class, selector, |original| {
        let original: unsafe extern "C-unwind" fn(&NSView, Sel) -> NSRect =
            unsafe { std::mem::transmute(original) };
        let layout = RcBlock::new(move |frame: ptr::NonNull<NSView>| {
            let frame = unsafe { frame.as_ref() };
            let titlebar = unsafe { original(frame, selector) };
            extended_titlebar_frame(frame, titlebar)
        });
        unsafe { ffi::imp_implementationWithBlock(RcBlock::as_ptr(&layout).cast()) }
    });
}

fn extended_titlebar_frame(frame: &NSView, mut titlebar: NSRect) -> NSRect {
    let Some(window) = frame.window() else {
        return titlebar;
    };
    let Some(center) = inset_placement(&window).and_then(|placement| placement.center) else {
        return titlebar;
    };
    let height = window
        .standardWindowButton(NSWindowButton::CloseButton)
        .map_or(14., |button| button.frame().size.height);
    let reach = (center.y * scale() + height / 2.).ceil();
    let flipped = frame.isFlipped();
    let bottom = if flipped {
        titlebar.origin.y + titlebar.size.height
    } else {
        frame.bounds().size.height - titlebar.origin.y
    };
    let grow = reach - bottom;
    if grow > 0. {
        titlebar.size.height += grow;
        if !flipped {
            titlebar.origin.y -= grow;
        }
    }
    titlebar
}

fn send_no_args(object: &AnyObject, selector: Sel) {
    if let Some(method) = object.class().instance_method(selector) {
        // SAFETY: All selectors passed here are AppKit methods with no arguments and no return value.
        unsafe {
            let call: unsafe extern "C-unwind" fn(&AnyObject, Sel) =
                std::mem::transmute(method.implementation());
            call(object, selector);
        }
    }
}

fn inset_placement(window: &NSWindow) -> Option<Placement> {
    if window.styleMask().contains(NSWindowStyleMask::FullScreen) {
        return None;
    }
    INSET_WINDOWS.with(|windows| {
        let windows = windows.borrow();
        let lights = windows.get(&ptr::from_ref(window))?.upgrade()?;
        Some(lights.placement.get())
    })
}

fn install_layout_hook(class: &AnyClass, selector: Sel, replacement: impl FnOnce(Imp) -> Imp) {
    let Some(method) = class.instance_method(selector) else {
        return;
    };
    if LAYOUT_HOOKS.with(|hooks| hooks.borrow_mut().insert(ptr::from_ref(method) as usize)) {
        unsafe { ffi::method_setImplementation(method, replacement(method.implementation())) };
    }
}

fn install_close_origin(class: &AnyClass) {
    let selector = sel!(_closeButtonOrigin);
    install_layout_hook(class, selector, |original| {
        let original: unsafe extern "C-unwind" fn(&NSView, Sel) -> NSPoint =
            unsafe { std::mem::transmute(original) };
        let layout = RcBlock::new(move |frame: ptr::NonNull<NSView>| {
            let frame = unsafe { frame.as_ref() };
            if let Some(window) = frame.window()
                && let Some(placement) = inset_placement(&window)
            {
                let size = window
                    .standardWindowButton(NSWindowButton::CloseButton)
                    .map_or(NSSize::new(14., 14.), |close| close.frame().size);
                let center = placement.center(window.frame().size.width, size);
                return NSPoint::new(
                    center.x - size.width / 2.,
                    frame.bounds().size.height - center.y - size.height / 2.,
                );
            }
            unsafe { original(frame, selector) }
        });
        unsafe { ffi::imp_implementationWithBlock(RcBlock::as_ptr(&layout).cast()) }
    });
}

fn install_after_layout<T: objc2::Message>(class: &AnyClass, selector: Sel, after: fn(&T)) {
    install_layout_hook(class, selector, |original| {
        let original: unsafe extern "C-unwind" fn(&T, Sel) =
            unsafe { std::mem::transmute(original) };
        let layout = RcBlock::new(move |object: ptr::NonNull<T>| {
            let object = unsafe { object.as_ref() };
            unsafe { original(object, selector) };
            after(object);
        });
        unsafe { ffi::imp_implementationWithBlock(RcBlock::as_ptr(&layout).cast()) }
    });
}

fn update_indicator(frame: &NSView) {
    static TUCKED: u8 = 0;
    if frame.respondsToSelector(sel!(sharingIndicator)) {
        let indicator: Option<Retained<AnyObject>> = unsafe { msg_send![frame, sharingIndicator] };
        if let Some(indicator) = indicator.and_then(|indicator| indicator.downcast::<NSView>().ok())
        {
            let follows = frame
                .window()
                .is_some_and(|window| inset_placement(&window).is_some());
            let shown = indicator.convertRect_toView(indicator.bounds(), Some(frame));
            let outside = follows && !objc2_foundation::NSContainsRect(frame.bounds(), shown);
            let key = ptr::from_ref(&TUCKED).cast();
            let object = ptr::from_ref::<AnyObject>(&indicator);
            unsafe {
                let was_tucked = !ffi::objc_getAssociatedObject(object, key).is_null();
                if outside != was_tucked {
                    let tucked = outside.then(|| NSNumber::numberWithBool(true));
                    ffi::objc_setAssociatedObject(
                        object.cast_mut(),
                        key,
                        tucked.as_deref().map_or(ptr::null_mut(), |value| {
                            ptr::from_ref::<AnyObject>(value).cast_mut()
                        }),
                        RETAIN_NONATOMIC,
                    );
                    indicator.setHidden(outside);
                }
            }
        }
    }
}

fn update_sharing_window(pill: &NSWindow) {
    static TUCKED: u8 = 0;
    let key = ptr::from_ref(&TUCKED).cast();
    let object = ptr::from_ref::<AnyObject>(pill);
    let tucked = unsafe {
        Retained::retain(
            ffi::objc_getAssociatedObject(object, key)
                .cast_mut()
                .cast::<NSDictionary<NSString, NSNumber>>(),
        )
    };
    let host: Option<Retained<AnyObject>> = if pill.respondsToSelector(sel!(hostWindow)) {
        unsafe { msg_send![pill, hostWindow] }
    } else {
        None
    };
    let outside = host
        .and_then(|host| host.downcast::<NSWindow>().ok())
        .is_some_and(|host| {
            inset_placement(&host).is_some()
                && !objc2_foundation::NSContainsRect(host.frame(), pill.frame())
        });
    let alpha = ns_string!("alpha");
    let ignores = ns_string!("ignores");
    if outside {
        if tucked.is_none() {
            let saved = NSDictionary::from_slices(
                &[alpha, ignores],
                &[
                    &*NSNumber::numberWithDouble(pill.alphaValue()),
                    &*NSNumber::numberWithBool(pill.ignoresMouseEvents()),
                ],
            );
            unsafe {
                ffi::objc_setAssociatedObject(
                    object.cast_mut(),
                    key,
                    ptr::from_ref::<AnyObject>(&*saved).cast_mut(),
                    RETAIN_NONATOMIC,
                );
            }
        }
        pill.setAlphaValue(0.);
        pill.setIgnoresMouseEvents(true);
    } else if let Some(saved) = tucked {
        unsafe {
            ffi::objc_setAssociatedObject(
                object.cast_mut(),
                key,
                ptr::null_mut(),
                RETAIN_NONATOMIC,
            );
        }
        pill.setAlphaValue(saved.objectForKey(alpha).unwrap().doubleValue());
        pill.setIgnoresMouseEvents(saved.objectForKey(ignores).unwrap().boolValue());
    }
}

fn sync_sharing_windows(window: &NSWindow) {
    if let Some(children) = window.childWindows() {
        let sharing = AnyClass::get(c"NSLocalWindowSharingWindow");
        for child in children.iter() {
            if sharing.is_some_and(|class| child.isKindOfClass(class)) {
                send_no_args(&child, sel!(_syncToWindowControls));
            }
        }
    }
}

fn frame_view(window: &NSWindow) -> Option<Retained<NSView>> {
    // SAFETY: The view hierarchy is accessible on the main thread.
    unsafe { window.contentView()?.superview() }
}

fn translate_buttons(window: &NSWindow, dx: f64) -> bool {
    let Some(transaction) = AnyClass::get(c"CATransaction") else {
        return false;
    };
    let buttons = BUTTONS.map(|kind| window.standardWindowButton(kind));
    let layers =
        buttons.map(|button| button.and_then(|button| unsafe { msg_send![&*button, layer] }));
    let [Some(a), Some(b), Some(c)]: [Option<Retained<AnyObject>>; 3] = layers else {
        return false;
    };
    let offset = NSNumber::numberWithDouble(dx);
    // SAFETY: CATransaction and CALayer support these selectors and the translation key path.
    unsafe {
        let _: () = msg_send![transaction, begin];
        let _: () = msg_send![transaction, setDisableActions: objc2::runtime::Bool::YES];
        for layer in [a, b, c] {
            let _: () = msg_send![&*layer, setValue: &*offset, forKeyPath: ns_string!("transform.translation.x")];
        }
        let _: () = msg_send![transaction, commit];
    }
    true
}

pub(super) fn frame_corner_radius(window: &NSWindow) -> Option<f64> {
    let frame = frame_view(window)?;
    if frame.respondsToSelector(sel!(roundedCornerRadius)) {
        // SAFETY: NSThemeFrame's selector takes no arguments and returns a float.
        let radius: f32 = unsafe { msg_send![&*frame, roundedCornerRadius] };
        if radius.is_finite() && radius > 0. {
            return Some(radius.into());
        }
    }
    // SAFETY: AppKit returns the view's CALayer, whose cornerRadius is a CGFloat.
    let layer: Option<Retained<AnyObject>> = unsafe { msg_send![&*frame, layer] };
    let radius: f64 = unsafe { msg_send![&*layer?, cornerRadius] };
    (radius.is_finite() && radius > 0.).then_some(radius)
}
