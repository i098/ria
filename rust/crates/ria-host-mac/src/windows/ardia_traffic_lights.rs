// Ported from ardia packages/ardiacore/ios/ArdiaTrafficLights.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::{Cell, RefCell};
use std::ptr;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Duration;

use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyClass, AnyObject, NSObjectProtocol, Sel};
use objc2::{ffi, msg_send, sel};
use objc2_app_kit::{
    NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSColor, NSView, NSWindow,
    NSWindowButton, NSWindowDidExitFullScreenNotification, NSWindowDidResizeNotification,
    NSWindowStyleMask, NSWindowTabbingMode, NSWindowTitleVisibility,
};
use objc2_foundation::{
    NSArray, NSNotification, NSNotificationCenter, NSNumber, NSPoint, NSRect, NSSize, ns_string,
};
use ria_engine::call::main_queue;

const LIGHTS_SETTLE: Duration = Duration::from_millis(50);
const BUTTONS: [NSWindowButton; 3] = [
    NSWindowButton::CloseButton,
    NSWindowButton::MiniaturizeButton,
    NSWindowButton::ZoomButton,
];
static TITLEBAR_CENTER: u8 = 0;
const RETAIN_NONATOMIC: usize = 1;

type TitlebarFrame = unsafe extern "C-unwind" fn(&NSView, Sel) -> NSRect;
static ORIGINAL_TITLEBAR_FRAME: OnceLock<TitlebarFrame> = OnceLock::new();

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Placement {
    pub center: Option<NSPoint>,
    pub trailing: bool,
}

impl Placement {
    fn center(self, window_width: f64, size: NSSize) -> NSPoint {
        match self.center {
            Some(at) if self.trailing => NSPoint::new(window_width - at.x, at.y),
            Some(at) => at,
            None => NSPoint::new(18. + size.width / 2., 20. + size.height / 2.),
        }
    }
}

pub struct TrafficLights {
    window: Weak<NSWindow>,
    placement: Cell<Placement>,
    shown: Cell<Option<Placement>>,
    move_turn: Cell<u64>,
    observers: RefCell<Vec<Retained<AnyObject>>>,
}

impl TrafficLights {
    pub fn configure(window: &NSWindow) -> Rc<Self> {
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

    pub fn inset(window: &NSWindow) -> Rc<Self> {
        let lights = Rc::new(Self {
            window: Weak::from(window),
            placement: Cell::default(),
            shown: Cell::default(),
            move_turn: Cell::default(),
            observers: RefCell::default(),
        });
        // SAFETY: These notifications carry the NSWindow supplied as the object filter.
        for name in unsafe {
            [
                NSWindowDidResizeNotification,
                NSWindowDidExitFullScreenNotification,
            ]
        } {
            let weak = Rc::downgrade(&lights);
            let callback = RcBlock::new(move |_: std::ptr::NonNull<NSNotification>| {
                if let Some(lights) = weak.upgrade() {
                    lights.refit();
                }
            });
            // SAFETY: The block receives an NSNotification and does not outlive its observer token.
            let observer = unsafe {
                NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                    Some(name),
                    Some(window),
                    None,
                    &callback,
                )
            };
            lights.observers.borrow_mut().push(observer.into());
        }
        lights.retile(true);
        lights
    }

    pub fn placement(&self) -> Placement {
        self.placement.get()
    }

    pub fn set_placement(&self, placement: Placement) {
        let previous = self.placement.replace(placement);
        let changed = previous != placement;
        if previous.center.map(|at| at.y) != placement.center.map(|at| at.y)
            && let Some(window) = self.window.load()
        {
            set_titlebar_center(&window, placement);
        }
        if self.shown.take().is_some() || changed {
            self.retile(true);
        }
    }

    pub fn move_to(self: &Rc<Self>, placement: Placement) {
        let current = self.shown.get().unwrap_or_else(|| self.placement.get());
        if current == placement {
            return;
        }
        let Some(window) = self.window.load() else {
            return;
        };
        let Some(close) = window.standardWindowButton(NSWindowButton::CloseButton) else {
            self.set_placement(placement);
            return;
        };
        let size = close.frame().size;
        let width = window.frame().size.width;
        let base = self.placement.get();
        let laid_out = base.center(width, size);
        let target = placement.center(width, size);
        if placement.trailing != current.trailing
            || base.center.is_some() != placement.center.is_some()
            || laid_out.y != target.y
            || window.styleMask().contains(NSWindowStyleMask::FullScreen)
        {
            self.set_placement(placement);
            return;
        }
        self.shown
            .set((laid_out.x != target.x).then_some(placement));
        self.retile(false);
        let turn = self.move_turn.get().wrapping_add(1);
        self.move_turn.set(turn);
        if self.shown.get().is_none() {
            return;
        }
        let weak = Rc::downgrade(self);
        main_queue::after(
            LIGHTS_SETTLE,
            Box::new(move || {
                if let Some(lights) = weak.upgrade()
                    && lights.move_turn.get() == turn
                {
                    lights.refit();
                }
            }),
        );
    }

    fn refit(&self) {
        if let Some(shown) = self.shown.take() {
            self.set_placement(shown);
        } else {
            self.retile(true);
        }
    }

    fn retile(&self, reset: bool) {
        let Some(window) = self.window.load() else {
            return;
        };
        let Some(frame) = frame_view(&window) else {
            return;
        };
        if reset {
            if self.placement.get().center.is_some() {
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
        }
        let buttons = BUTTONS.map(|kind| window.standardWindowButton(kind));
        let Some(close) = buttons.iter().flatten().next() else {
            return;
        };
        if window.styleMask().contains(NSWindowStyleMask::FullScreen) {
            for button in buttons.iter().flatten() {
                button.setHidden(false);
            }
            return;
        }
        let close_rect = close.convertRect_toView(close.bounds(), Some(&frame));
        let size = close_rect.size;
        let frame_size = frame.bounds().size;
        let placement = self.shown.get().unwrap_or_else(|| self.placement.get());
        let at = placement.center(window.frame().size.width, size);
        let target_x = at.x - size.width / 2.;
        let target_y = if frame.isFlipped() {
            at.y - size.height / 2.
        } else {
            frame_size.height - at.y - size.height / 2.
        };
        let dx = target_x - close_rect.origin.x;
        let dy = target_y - close_rect.origin.y;
        let mut right = close_rect.origin.x + size.width;
        for button in buttons.iter().flatten() {
            let bounds = button.convertRect_toView(button.bounds(), Some(&frame));
            right = f64::max(right, bounds.origin.x + bounds.size.width);
            move_view(button, &frame, dx, dy);
        }
        let outside =
            right + dx <= 0. || target_x >= frame_size.width || at.y + size.height / 2. <= 0.;
        for button in buttons.iter().flatten() {
            button.setHidden(outside);
        }
        adjust_sharing(&window, &frame, dx, dy);
    }
}

impl Drop for TrafficLights {
    fn drop(&mut self) {
        if self.placement.get().center.is_some()
            && let Some(window) = self.window.load()
        {
            set_titlebar_center(&window, Placement::default());
        }
        let center = NSNotificationCenter::defaultCenter();
        for observer in self.observers.get_mut() {
            // SAFETY: These observer tokens came from this notification center.
            unsafe {
                center.removeObserver(observer);
            }
        }
    }
}

fn set_titlebar_center(window: &NSWindow, placement: Placement) {
    let center = placement.center.map(|point| NSNumber::numberWithDouble(point.y));
    unsafe {
        ffi::objc_setAssociatedObject(
            ptr::from_ref::<AnyObject>(window).cast_mut(),
            ptr::from_ref(&TITLEBAR_CENTER).cast(),
            center.as_deref().map_or(ptr::null_mut(), |point| {
                ptr::from_ref::<AnyObject>(point).cast_mut()
            }),
            RETAIN_NONATOMIC,
        );
    }
}

fn install_titlebar_extension(class: &'static AnyClass) {
    if ORIGINAL_TITLEBAR_FRAME.get().is_some() {
        return;
    }
    let Some(method) = class.instance_method(sel!(_currentTitlebarContainerViewFrame)) else {
        return;
    };
    unsafe {
        let original = std::mem::transmute::<_, TitlebarFrame>(method.implementation());
        if ORIGINAL_TITLEBAR_FRAME.set(original).is_ok() {
            let replacement = std::mem::transmute::<TitlebarFrame, _>(extended_titlebar_frame);
            let _ = ffi::method_setImplementation(method, replacement);
        }
    }
}

unsafe extern "C-unwind" fn extended_titlebar_frame(frame: &NSView, selector: Sel) -> NSRect {
    let original = ORIGINAL_TITLEBAR_FRAME.get().expect("Titlebar implementation");
    let mut titlebar = unsafe { original(frame, selector) };
    let Some(window) = frame.window() else {
        return titlebar;
    };
    if window.styleMask().contains(NSWindowStyleMask::FullScreen) {
        return titlebar;
    }
    let center = unsafe {
        ffi::objc_getAssociatedObject(
            ptr::from_ref::<AnyObject>(&window),
            ptr::from_ref(&TITLEBAR_CENTER).cast(),
        )
        .cast::<NSNumber>()
        .as_ref()
    };
    let Some(center) = center else {
        return titlebar;
    };
    let height = window
        .standardWindowButton(NSWindowButton::CloseButton)
        .map_or(14., |button| button.frame().size.height);
    let reach = (center.doubleValue() + height / 2.).ceil();
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

fn move_view(view: &NSView, together: &NSView, dx: f64, dy: f64) {
    // SAFETY: The view hierarchy is accessible on the main thread.
    let Some(parent) = (unsafe { view.superview() }) else {
        return;
    };
    let before = view.frame().origin;
    let origin = together.convertPoint_toView(NSPoint::ZERO, Some(&parent));
    let shifted = together.convertPoint_toView(NSPoint::new(dx, dy), Some(&parent));
    view.setFrameOrigin(NSPoint::new(
        before.x + shifted.x - origin.x,
        before.y + shifted.y - origin.y,
    ));
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

fn adjust_sharing(window: &NSWindow, frame: &NSView, dx: f64, dy: f64) {
    if frame.respondsToSelector(sel!(sharingIndicator)) {
        // SAFETY: The private selector takes no arguments and returns a nullable object.
        let indicator: Option<Retained<AnyObject>> = unsafe { msg_send![frame, sharingIndicator] };
        if let Some(indicator) = indicator.and_then(|indicator| indicator.downcast::<NSView>().ok())
        {
            move_view(&indicator, frame, dx, dy);
            let shown = indicator.convertRect_toView(indicator.bounds(), Some(frame));
            indicator.setHidden(!objc2_foundation::NSContainsRect(frame.bounds(), shown));
        }
    }
    if let Some(children) = window.childWindows() {
        let sharing = objc2::runtime::AnyClass::get(c"NSLocalWindowSharingWindow");
        for child in children.iter() {
            if sharing.is_some_and(|class| child.isKindOfClass(class)) {
                send_no_args(&child, sel!(_syncToWindowControls));
            }
        }
    }
}

pub fn frame_view(window: &NSWindow) -> Option<Retained<NSView>> {
    // SAFETY: The view hierarchy is accessible on the main thread.
    unsafe { window.contentView()?.superview() }
}
