// Ported from ardia packages/ardiacore/ios/ArdiaPiPController.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The controller of one Picture in Picture window. Other files of this module add `impl Controller` blocks.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, NSObject};
use objc2::{
    AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send, sel,
};
use objc2_app_kit::{
    NSAnimationContext, NSApplication, NSApplicationDidChangeScreenParametersNotification, NSColor,
    NSControlStateValueOff, NSControlStateValueOn, NSCursor, NSEvent, NSFloatingWindowLevel,
    NSMenu, NSMenuItem, NSNormalWindowLevel, NSScreen, NSView, NSWindow,
    NSWindowDidChangeScreenNotification, NSWindowDidMoveNotification,
    NSWindowDidResizeNotification, NSWindowWillCloseNotification,
    NSWindowWillStartLiveResizeNotification, NSWorkspace,
};
use objc2_foundation::{
    NSNotification, NSNotificationCenter, NSPoint, NSProcessInfo, NSRect, NSString, ns_string,
};
use serde_json::{Value, json};

use super::ardia_picture_in_picture::activate_app;
use super::ardia_picture_in_picture_internal::{
    self as internal, CORNER_BEND_DURATION, Delay, Edge, PEEK, PipPage, RETURN_MARGIN, RIM_ALPHA,
};
use super::ardia_pip_overlay::Overlay;
use super::ardia_pip_resize_cursor::ResizeCursor;
use super::ardia_pip_throw_math::{self as throw, Corner};

/// Everything that `ArdiaPiPController` keeps in instance variables.
/// The ObjC names are in the comments; field names are the snake case of them.
#[derive(Default)]
pub struct State {
    pub host: RefCell<String>,
    pub stashed_edge: Cell<Edge>,
    pub pulled_from: Cell<Edge>,
    pub pulled_from_screen: RefCell<Option<Retained<NSScreen>>>,
    pub pull_start: Cell<NSRect>,
    pub keep_on_top: Cell<bool>,
    /// `_slide` and the NSTimer: the generation of the running slide, 0 when none runs.
    pub slide: Delay,
    pub animating: Cell<bool>,
    pub saw_drag: Cell<bool>,
    pub swiping: Cell<bool>,
    pub swipe_grab: Cell<NSPoint>,
    pub swipe_last_delta: Cell<NSPoint>,
    pub swipe_pointer: Cell<NSPoint>,
    pub swipe_began_at: Cell<f64>,
    pub swipe_last_at: Cell<f64>,
    pub awaiting_momentum: Cell<bool>,
    pub swipe_placed: Cell<NSRect>,
    pub swipe_start: Cell<NSRect>,
    pub dragging: Cell<bool>,
    pub drag_moving: Cell<bool>,
    pub drag_grab: Cell<NSPoint>,
    pub drag_from: Cell<NSPoint>,
    pub drag_began_at: Cell<f64>,
    pub drag_stamp: RefCell<Value>,
    pub swipe_stamp: RefCell<Value>,
    pub placement_stamp: RefCell<Value>,
    pub drag_off_cursor: Cell<i64>,
    pub drag_max_off: Cell<f64>,
    pub drag_slowest_move: Cell<f64>,
    pub drag_trace: RefCell<Vec<Value>>,
    pub drag_samples: RefCell<Vec<throw::Sample>>,
    pub drag_home: RefCell<Option<Retained<NSScreen>>>,
    pub last_drag: RefCell<Value>,
    pub last_release: RefCell<Value>,
    pub last_skip: RefCell<Value>,
    pub skips: Cell<i64>,
    pub last_swipe: RefCell<Value>,
    pub last_throw: RefCell<serde_json::Map<String, Value>>,
    pub last_screen_fit: RefCell<Value>,
    pub last_screen: RefCell<Option<Retained<NSScreen>>>,
    pub swipe_home: RefCell<Option<Retained<NSScreen>>>,
    pub stash_screen: RefCell<Option<Retained<NSScreen>>>,
    /// Timers of `performSelector:afterDelay:` that the file of the owner cancels with `Delay::cancel`.
    pub momentum_wait: Delay,
    pub settle_delay: Delay,
    pub placement_report: Delay,
    /// Tokens of the notification observers.
    pub observers: RefCell<Vec<Retained<AnyObject>>>,
}

pub struct Controller {
    pub mtm: MainThreadMarker,
    pub window: Retained<NSWindow>,
    pub view: RefCell<Option<Weak<dyn PipPage>>>,
    pub state: State,
    pub me: Weak<Controller>,
    /// Set by `Controller::new` from the files of the overlay, the resize cursor.
    pub overlay: RefCell<Option<Retained<Overlay>>>,
    pub resize_cursor: RefCell<Option<Rc<ResizeCursor>>>,
    /// The target of the menu items. NSMenuItem holds its target weakly, so the controller owns it.
    menu_target: RefCell<Option<Retained<MenuTarget>>>,
}

thread_local! {
    pub static CONTROLLERS: RefCell<HashMap<*const NSWindow, Rc<Controller>>> = RefCell::new(HashMap::new());
}

const KEEP_ON_TOP_DEFAULT: &str = "ACPictureInPictureKeepOnTop";
const SLIDE_INTERVAL: f64 = 1.0 / 120.0;

unsafe extern "C" {
    fn CACurrentMediaTime() -> f64;
}

/// `CACurrentMediaTime`.
pub fn media_time() -> f64 {
    // SAFETY: the function takes no argument and has no precondition.
    unsafe { CACurrentMediaTime() }
}

fn corner_name(corner: Option<Corner>) -> &'static str {
    match corner {
        Some(Corner::TopLeft) => "topLeft",
        Some(Corner::TopRight) => "topRight",
        Some(Corner::BottomLeft) => "bottomLeft",
        Some(Corner::BottomRight) => "bottomRight",
        None => "none",
    }
}

fn layer_radius(layer: &AnyObject) -> f64 {
    // SAFETY: both messages are CALayer properties; `presentationLayer` may be nil.
    unsafe {
        let shown: Option<Retained<AnyObject>> = msg_send![layer, presentationLayer];
        match shown {
            Some(shown) => msg_send![&*shown, cornerRadius],
            None => msg_send![layer, cornerRadius],
        }
    }
}

fn pressed_left() -> bool {
    NSEvent::pressedMouseButtons() & 1 != 0
}

struct MenuTargetState(Weak<Controller>);

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "RiaPiPMenuTarget"]
    #[ivars = MenuTargetState]
    struct MenuTarget;

    impl MenuTarget {
        #[unsafe(method(backToTabFromMenu:))]
        fn back_to_tab_from_menu(&self, _sender: Option<&AnyObject>) {
            if let Some(controller) = self.ivars().0.upgrade() {
                controller.back_to_tab();
            }
        }

        #[unsafe(method(toggleKeepOnTop:))]
        fn toggle_keep_on_top(&self, _sender: Option<&AnyObject>) {
            if let Some(controller) = self.ivars().0.upgrade() {
                controller.toggle_keep_on_top();
            }
        }
    }
);

impl MenuTarget {
    fn new(controller: Weak<Controller>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(MenuTargetState(controller));
        // SAFETY: The allocation has its Rust ivars before NSObject initialization.
        unsafe { msg_send![super(this), init] }
    }
}

/// Runs `step` every 1/120 s until it returns false or the controller is gone: the `_slide` NSTimer.
fn run_slide(me: Weak<Controller>, step: Rc<dyn Fn(&Controller) -> bool>) {
    let Some(controller) = me.upgrade() else {
        return;
    };
    controller.state.slide.schedule(SLIDE_INTERVAL, move || {
        let Some(controller) = me.upgrade() else {
            return;
        };
        if step(&controller) {
            run_slide(me, step);
        }
    });
}

impl Controller {
    pub fn new(window: &NSWindow) -> Rc<Controller> {
        let mtm = MainThreadMarker::new().expect("PiP controllers live on the main thread");
        let controller = Rc::new_cyclic(|me| Controller {
            mtm,
            window: window.retain(),
            view: RefCell::new(None),
            state: State::default(),
            me: me.clone(),
            overlay: RefCell::new(None),
            resize_cursor: RefCell::new(None),
            menu_target: RefCell::new(None),
        });
        let root = internal::window_root_view(window);
        let bounds = root.as_ref().map_or(NSRect::ZERO, |root| root.bounds());
        let overlay = Overlay::new(mtm, bounds);
        if let Some(root) = &root {
            root.addSubview(&overlay);
        }
        *controller.resize_cursor.borrow_mut() = Some(ResizeCursor::new(window, &overlay));
        *controller.overlay.borrow_mut() = Some(overlay);
        CONTROLLERS.with(|map| {
            map.borrow_mut()
                .insert(std::ptr::from_ref(window), Rc::clone(&controller))
        });

        controller.observe(unsafe { NSWindowDidMoveNotification }, Some(window), |c| {
            c.window_moved()
        });
        controller.observe(
            unsafe { NSWindowDidResizeNotification },
            Some(window),
            |c| c.window_moved(),
        );
        controller.observe(
            unsafe { NSWindowDidResizeNotification },
            Some(window),
            |c| c.round_corners(),
        );
        controller.observe(
            unsafe { NSWindowDidChangeScreenNotification },
            Some(window),
            |c| c.screen_changed(),
        );
        let app = NSApplication::sharedApplication(mtm);
        controller.observe(
            unsafe { NSApplicationDidChangeScreenParametersNotification },
            Some(&app),
            |c| c.screen_changed(),
        );
        controller.observe(
            unsafe { NSWindowWillStartLiveResizeNotification },
            Some(window),
            |c| {
                c.stop_motion();
                c.resize_started();
            },
        );
        controller.observe(
            unsafe { NSWindowWillCloseNotification },
            Some(window),
            |c| c.detach(),
        );
        controller.set_keep_on_top(internal::user_default_bool(KEEP_ON_TOP_DEFAULT, true));
        controller
    }

    fn observe(
        &self,
        name: &NSString,
        object: Option<&AnyObject>,
        handler: impl Fn(&Controller) + 'static,
    ) {
        let me = self.me.clone();
        let block = RcBlock::new(move |_: std::ptr::NonNull<NSNotification>| {
            if let Some(controller) = me.upgrade() {
                handler(&controller);
            }
        });
        // SAFETY: the block does not outlive its use by the center; the token is removed in `detach`.
        let token = unsafe {
            NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                Some(name),
                object,
                None,
                &block,
            )
        };
        self.state.observers.borrow_mut().push(token.into());
    }

    pub fn for_window(window: &NSWindow) -> Option<Rc<Controller>> {
        CONTROLLERS.with(|map| map.borrow().get(&std::ptr::from_ref(window)).cloned())
    }

    pub fn view(&self) -> Option<Rc<dyn PipPage>> {
        self.view.borrow().as_ref().and_then(Weak::upgrade)
    }

    pub fn set_view(&self, view: Option<&Rc<dyn PipPage>>) {
        *self.view.borrow_mut() = view.map(Rc::downgrade);
    }

    pub fn host(&self) -> String {
        self.state.host.borrow().clone()
    }

    pub fn set_host(&self, host: &str) {
        *self.state.host.borrow_mut() = host.to_owned();
    }

    pub fn overlay(&self) -> Retained<Overlay> {
        self.overlay
            .borrow()
            .clone()
            .expect("the overlay is set in `new`")
    }

    pub fn resize_cursor(&self) -> Rc<ResizeCursor> {
        self.resize_cursor
            .borrow()
            .clone()
            .expect("the resize cursor is set in `new`")
    }

    pub fn stashed_edge(&self) -> Edge {
        self.state.stashed_edge.get()
    }

    pub fn keep_on_top(&self) -> bool {
        self.state.keep_on_top.get()
    }

    pub fn set_keep_on_top(&self, on: bool) {
        self.state.keep_on_top.set(on);
        self.window.setLevel(if on {
            NSFloatingWindowLevel
        } else {
            NSNormalWindowLevel
        });
    }

    pub fn toggle_keep_on_top(&self) {
        self.set_keep_on_top(!self.keep_on_top());
        internal::set_user_default_bool(KEEP_ON_TOP_DEFAULT, self.keep_on_top());
    }

    /// `[NSObject cancelPreviousPerformRequestsWithTarget:self selector:@selector(settle) object:nil]`.
    pub fn cancel_settle(&self) {
        self.state.settle_delay.cancel();
    }

    /// `[self performSelector:@selector(settle) withObject:nil afterDelay:0.12]`; it replaces a pending one.
    pub fn schedule_settle(&self) {
        let me = self.me.clone();
        self.state.settle_delay.schedule(0.12, move || {
            if let Some(controller) = me.upgrade() {
                controller.settle();
            }
        });
    }

    pub fn detach(&self) {
        Controller::abandon_swipe_on(&self.window);
        let center = NSNotificationCenter::defaultCenter();
        for observer in self.state.observers.borrow_mut().drain(..) {
            // SAFETY: the token came from `addObserverForName:object:queue:usingBlock:`.
            unsafe { center.removeObserver(&observer) };
        }
        self.end_momentum_wait();
        self.state.settle_delay.cancel();
        self.state.placement_report.cancel();
        self.state.slide.cancel();
        self.resize_cursor().detach();
        self.overlay().removeFromSuperview();
        CONTROLLERS.with(|map| map.borrow_mut().remove(&std::ptr::from_ref(&*self.window)));
    }

    pub fn round_corners(&self) {
        if let Some(content) = self.window.contentView() {
            content.setWantsLayer(true);
            if let Some(layer) = content.layer() {
                // SAFETY: CALayer properties; the curve is an NSString constant, the mask a BOOL.
                unsafe {
                    let curve: Option<Retained<NSString>> = msg_send![&*layer, cornerCurve];
                    let masks: bool = msg_send![&*layer, masksToBounds];
                    if curve.as_deref() != Some(ns_string!("circular")) || !masks {
                        let _: () = msg_send![&*layer, setCornerCurve: ns_string!("circular")];
                        let _: () = msg_send![&*layer, setMasksToBounds: true];
                    }
                }
            }
        }
        if let Some(rim) = self.overlay().layer() {
            let color = NSColor::colorWithWhite_alpha(1.0, RIM_ALPHA).CGColor();
            // SAFETY: CALayer properties; the border color is a borrowed CGColor that the layer retains.
            unsafe {
                let _: () = msg_send![&*rim, setCornerCurve: ns_string!("circular")];
                let _: () = msg_send![&*rim, setBorderWidth: 1.0_f64];
                let _: () = msg_send![&*rim, setBorderColor: &*color];
            }
        }
        self.fit_corners(false);
        self.window.setOpaque(false);
        self.window.setBackgroundColor(Some(&NSColor::clearColor()));
        let window = self.window.clone();
        ria_engine::call::main_queue::post(Box::new(move || window.invalidateShadow()));
    }

    pub fn corner_radius(&self) -> f64 {
        let visible = self.home_screen().map_or(throw::Rect::default(), |screen| {
            internal::throw_rect(screen.visibleFrame())
        });
        throw::corner_radius(internal::throw_rect(self.window.frame()), visible)
    }

    pub fn fit_corners(&self, animated: bool) {
        self.bend_corners_to(self.corner_radius(), animated);
    }

    pub fn bend_corners_to(&self, radius: f64, animated: bool) {
        let content = self
            .window
            .contentView()
            .and_then(|content| content.layer());
        let rim = self.overlay().layer();
        let (Some(content), Some(rim)) = (content, rim) else {
            return;
        };
        let content: &AnyObject = &content;
        let rim: &AnyObject = &rim;
        if animated && layer_radius_set(content) == radius && layer_radius_set(rim) == radius {
            return;
        }
        let window = self.window.clone();
        let done = RcBlock::new(move || window.invalidateShadow());
        let transaction = AnyClass::get(c"CATransaction").expect("QuartzCore is linked");
        // SAFETY: CATransaction and CALayer messages with their documented argument types.
        unsafe {
            let _: () = msg_send![transaction, begin];
            let _: () = msg_send![transaction, setDisableActions: true];
            let _: () = msg_send![transaction, setCompletionBlock: &*done];
            for layer in [content, rim] {
                if animated {
                    let animation: Retained<AnyObject> = msg_send![
                        AnyClass::get(c"CABasicAnimation").expect("QuartzCore is linked"),
                        animationWithKeyPath: ns_string!("cornerRadius")
                    ];
                    let number = AnyClass::get(c"NSNumber").expect("Foundation is linked");
                    let from: Retained<AnyObject> =
                        msg_send![number, numberWithDouble: layer_radius(layer)];
                    let to: Retained<AnyObject> = msg_send![number, numberWithDouble: radius];
                    let timing: Retained<AnyObject> = msg_send![
                        AnyClass::get(c"CAMediaTimingFunction").expect("QuartzCore is linked"),
                        functionWithName: ns_string!("easeInEaseOut")
                    ];
                    let _: () = msg_send![&*animation, setFromValue: &*from];
                    let _: () = msg_send![&*animation, setToValue: &*to];
                    let _: () = msg_send![&*animation, setDuration: CORNER_BEND_DURATION];
                    let _: () = msg_send![&*animation, setTimingFunction: &*timing];
                    let _: () = msg_send![layer, addAnimation: &*animation, forKey: ns_string!("cornerRadius")];
                } else {
                    let _: () = msg_send![layer, removeAnimationForKey: ns_string!("cornerRadius")];
                }
                let _: () = msg_send![layer, setCornerRadius: radius];
            }
            let _: () = msg_send![transaction, commit];
        }
    }

    pub fn menu(&self) -> Retained<NSMenu> {
        let target = self
            .menu_target
            .borrow_mut()
            .get_or_insert_with(|| MenuTarget::new(self.me.clone()))
            .clone();
        let menu = NSMenu::initWithTitle(NSMenu::alloc(self.mtm), ns_string!(""));
        // SAFETY: both selectors exist on `MenuTarget`; items hold the target weakly and `self` owns it.
        unsafe {
            let back = menu.addItemWithTitle_action_keyEquivalent(
                ns_string!("Back to Tab"),
                Some(sel!(backToTabFromMenu:)),
                ns_string!(""),
            );
            back.setTarget(Some(&target));
            back.setEnabled(self.view().is_some());
            menu.addItem(&NSMenuItem::separatorItem(self.mtm));
            let top = menu.addItemWithTitle_action_keyEquivalent(
                ns_string!("Keep Window on Top"),
                Some(sel!(toggleKeepOnTop:)),
                ns_string!(""),
            );
            top.setTarget(Some(&target));
            top.setState(if self.keep_on_top() {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
        }
        menu
    }

    pub fn back_to_tab(&self) {
        let Some(view) = self.view() else {
            return;
        };
        let from_other_app = activate_app("Back to Tab (menu)");
        view.emit(
            "activateRequest",
            json!({ "reason": "pictureInPicture", "fromOtherApp": from_other_app }),
        );
        view.exit_picture_in_picture();
    }

    pub fn screen_changed(&self) {
        let screen = self.home_screen();
        if self.state.animating.get() {
            return;
        }
        self.fit_corners(true);
        *self.state.last_screen_fit.borrow_mut() = json!({
            "radius": self.corner_radius(),
            "frame": internal::rect_string(self.window.frame()),
            "visibleFrame": screen.map_or_else(String::new, |screen| internal::rect_string(screen.visibleFrame())),
        });
    }

    pub fn window_moved(&self) {
        self.home_screen();
        if !self.state.animating.get() {
            self.fit_corners(true);
        }
        let swiping_lately = self.state.swiping.get()
            && NSProcessInfo::processInfo().systemUptime() - self.state.swipe_last_at.get() < 1.0;
        if self.state.animating.get() || swiping_lately {
            return;
        }
        if self.state.swiping.get() {
            Controller::abandon_swipe_on(&self.window);
        }
        if self.state.dragging.get() || pressed_left() {
            self.state.saw_drag.set(true);
        }
        self.schedule_settle();
    }

    pub fn home_screen(&self) -> Option<Retained<NSScreen>> {
        let screen = internal::overlapping_screen(self.mtm, self.window.frame());
        if let Some(screen) = &screen {
            *self.state.last_screen.borrow_mut() = Some(screen.clone());
        }
        screen
            .or_else(|| self.state.last_screen.borrow().clone())
            .or_else(|| NSScreen::mainScreen(self.mtm))
    }

    pub fn edge_beyond(&self) -> Edge {
        self.edge_beyond_for(self.window.frame())
    }

    pub fn edge_beyond_for(&self, frame: NSRect) -> Edge {
        let visible = self
            .home_screen()
            .map_or(NSRect::ZERO, |screen| screen.visibleFrame());
        let width = internal::width(frame);
        if throw::past_edge_enough_to_tuck(internal::max_x(frame) - internal::max_x(visible), width)
            && !internal::screen_at(
                self.mtm,
                internal::max_x(visible) + 1.0,
                internal::mid_y(frame),
            )
        {
            return Edge::Right;
        }
        if throw::past_edge_enough_to_tuck(internal::min_x(visible) - internal::min_x(frame), width)
            && !internal::screen_at(
                self.mtm,
                internal::min_x(visible) - 1.0,
                internal::mid_y(frame),
            )
        {
            return Edge::Left;
        }
        Edge::None
    }

    pub fn settle(&self) {
        if self.state.dragging.get() || pressed_left() {
            self.schedule_settle();
            return;
        }
        let dragged = self.state.saw_drag.replace(false);
        let edge = self.edge_beyond();
        if edge != Edge::None {
            return self.stash_at(edge);
        }
        let stashed = self.stashed_edge();
        if stashed != Edge::None && !dragged {
            return self.stash_at(stashed);
        }
        if stashed != Edge::None {
            self.set_stashed_edge(Edge::None);
        }
        self.rescue_if_lost();
    }

    pub fn gesture_home(&self) -> Option<Retained<NSScreen>> {
        let stash = self.state.stash_screen.borrow().clone();
        if self.stashed_edge() != Edge::None && stash.is_some() {
            stash
        } else {
            self.home_screen()
        }
    }

    pub fn rescue_if_lost(&self) {
        if self.stashed_edge() != Edge::None
            || self.state.animating.get()
            || self.state.swiping.get()
            || !self.window.isVisible()
        {
            return;
        }
        let screens = internal::throw_screens(self.mtm);
        let home = internal::screen_index(self.mtm, self.home_screen().as_deref());
        let rescued = throw::rescue(internal::throw_rect(self.window.frame()), &screens, home);
        let target = internal::from_throw_rect(rescued);
        if !internal::equal_rects(target, self.window.frame()) {
            self.animate_to(target);
        }
    }

    pub fn frame_stashed_at(&self, edge: Edge) -> NSRect {
        let mut frame = self.window.frame();
        let visible = self
            .home_screen()
            .map_or(NSRect::ZERO, |screen| screen.visibleFrame());
        frame.origin.x = if edge == Edge::Right {
            internal::max_x(visible) - PEEK
        } else {
            internal::min_x(visible) - internal::width(frame) + PEEK
        };
        frame.origin.y = internal::min_y(frame)
            .max(internal::min_y(visible))
            .min(internal::max_y(visible) - internal::height(frame));
        frame
    }

    pub fn stash_at(&self, edge: Edge) {
        *self.state.stash_screen.borrow_mut() = self.home_screen();
        self.set_stashed_edge(edge);
        self.animate_to(self.frame_stashed_at(edge));
    }

    pub fn unstash(&self) {
        let edge = self.stashed_edge();
        if edge != Edge::None {
            let visible = self
                .home_screen()
                .map_or(NSRect::ZERO, |screen| screen.visibleFrame());
            self.leave_tuck_at(edge, visible, NSPoint::ZERO);
        }
    }

    pub fn leave_tuck_at(&self, edge: Edge, visible: NSRect, velocity: NSPoint) -> NSRect {
        let mut frame = self.window.frame();
        frame.origin.x = if edge == Edge::Right {
            internal::max_x(visible) - internal::width(frame) - RETURN_MARGIN
        } else {
            internal::min_x(visible) + RETURN_MARGIN
        };
        self.set_stashed_edge(Edge::None);
        self.cancel_settle();
        self.state.saw_drag.set(false);
        if !internal::throw_enabled()
            || NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
        {
            self.animate_to(frame);
        } else {
            self.throw_to(frame, velocity, None);
        }
        frame
    }

    pub fn set_stashed_edge(&self, edge: Edge) {
        self.state.stashed_edge.set(edge);
        let overlay = self.overlay();
        let handle = overlay.handle();
        if edge != Edge::None {
            handle.set_edge(edge);
            handle.setHidden(false);
            overlay.setNeedsLayout(true);
        }
        let target = handle.clone();
        let changes = RcBlock::new(move |context: std::ptr::NonNull<NSAnimationContext>| {
            // SAFETY: AppKit passes a live context; `animator` is the NSView proxy that animates `alphaValue`.
            unsafe {
                context.as_ref().setDuration(0.25);
                let animator: Retained<NSView> = msg_send![&*target, animator];
                animator.setAlphaValue(if edge != Edge::None { 1.0 } else { 0.0 });
            }
        });
        let me = self.me.clone();
        let handle = handle.clone();
        let done = RcBlock::new(move || {
            if me
                .upgrade()
                .is_some_and(|controller| controller.stashed_edge() == Edge::None)
            {
                handle.setHidden(true);
            }
        });
        NSAnimationContext::runAnimationGroup_completionHandler(&changes, Some(&done));
    }

    pub fn animate_to(&self, target: NSRect) {
        self.state.slide.cancel();
        let from = self.window.frame();
        if internal::equal_rects(from, target) {
            return;
        }
        self.state.animating.set(true);
        let start = media_time();
        run_slide(
            self.me.clone(),
            Rc::new(move |controller| {
                let t = ((media_time() - start) / 0.25).min(1.0);
                let eased = 1.0 - (1.0 - t).powi(3);
                let frame = internal::rect(
                    (internal::min_x(from)
                        + (internal::min_x(target) - internal::min_x(from)) * eased)
                        .round(),
                    (internal::min_y(from)
                        + (internal::min_y(target) - internal::min_y(from)) * eased)
                        .round(),
                    internal::width(target),
                    internal::height(target),
                );
                controller
                    .window
                    .setFrame_display(if t < 1.0 { frame } else { target }, true);
                if t < 1.0 {
                    return true;
                }
                controller.state.animating.set(false);
                controller.state.saw_drag.set(false);
                controller.fit_corners(true);
                controller.rescue_if_lost();
                false
            }),
        );
    }

    pub fn stop_motion(&self) {
        self.end_momentum_wait();
        self.state.slide.cancel();
        self.state.animating.set(false);
    }

    pub fn throw_from(&self, grab: NSPoint, velocity: NSPoint) -> bool {
        let frame = self.window.frame();
        let Some(screen) = self.home_screen() else {
            return false;
        };
        let Some(corner) = throw::pick_corner(
            throw::Point {
                x: grab.x,
                y: grab.y,
            },
            throw::Point {
                x: velocity.x,
                y: velocity.y,
            },
            internal::throw_rect(screen.frame()),
        ) else {
            return false;
        };
        let target = throw::corner_frame(
            corner,
            internal::width(frame),
            internal::height(frame),
            internal::throw_rect(screen.visibleFrame()),
        );
        self.throw_to(internal::from_throw_rect(target), velocity, Some(corner));
        true
    }

    pub fn throw_to(&self, target: NSRect, velocity: NSPoint, corner: Option<Corner>) {
        self.stop_motion();
        self.bend_corners_to(throw::FREE_RADIUS, false);
        self.cancel_settle();
        self.state.saw_drag.set(false);
        if self.stashed_edge() != Edge::None {
            self.set_stashed_edge(Edge::None);
        }
        let from = self.window.frame();
        let mut record = serde_json::Map::new();
        record.insert("corner".into(), json!(corner_name(corner)));
        record.insert("from".into(), json!(internal::rect_string(from)));
        record.insert("target".into(), json!(internal::rect_string(target)));
        record.insert("velocity".into(), json!(internal::point_string(velocity)));
        record.insert("frames".into(), json!([]));
        *self.state.last_throw.borrow_mut() = record;
        self.state.animating.set(true);
        let start = media_time();
        run_slide(
            self.me.clone(),
            Rc::new(move |controller| {
                let window = &controller.window;
                if !window.isVisible() {
                    controller.state.animating.set(false);
                    return false;
                }
                let t = media_time() - start;
                let x = throw::spring_at(
                    internal::min_x(from) - internal::min_x(target),
                    velocity.x,
                    t,
                    throw::RESPONSE,
                    throw::DAMPING_RATIO,
                );
                let y = throw::spring_at(
                    internal::min_y(from) - internal::min_y(target),
                    velocity.y,
                    t,
                    throw::RESPONSE,
                    throw::DAMPING_RATIO,
                );
                let done = throw::resolved(x, y);
                let origin = if done {
                    target.origin
                } else {
                    NSPoint::new(
                        (internal::min_x(target) + x.offset).round(),
                        (internal::min_y(target) + y.offset).round(),
                    )
                };
                if !internal::equal_points(origin, window.frame().origin) {
                    window.setFrameOrigin(origin);
                    let radius = window
                        .contentView()
                        .and_then(|content| content.layer())
                        .map_or(0.0, |layer| layer_radius(&layer));
                    let mut record = controller.state.last_throw.borrow_mut();
                    if let Some(Value::Array(frames)) = record.get_mut("frames")
                        && frames.len() < 1200
                    {
                        frames.push(json!([
                            (t * 10000.0).round() / 10000.0,
                            origin.x,
                            origin.y,
                            radius
                        ]));
                    }
                }
                if !done {
                    return true;
                }
                controller.state.animating.set(false);
                controller
                    .state
                    .last_throw
                    .borrow_mut()
                    .insert("settledAfter".into(), json!(t));
                controller.fit_corners(true);
                controller.rescue_if_lost();
                false
            }),
        );
    }

    pub fn drag_released_at(&self, pointer: NSPoint, velocity: NSPoint) {
        let reduce_motion = NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion();
        let past_edge = self.edge_beyond() != Edge::None;
        let flings = internal::throw_enabled()
            && !reduce_motion
            && throw::drag_flings(throw::Point {
                x: velocity.x,
                y: velocity.y,
            })
            && !past_edge;
        let flicked_past = if flings {
            self.edge_flicked_past(velocity)
        } else {
            Edge::None
        };
        let outcome = if !internal::is_empty_rect(self.leave_tuck_if_pulled_out(velocity)) {
            "pullOut"
        } else if past_edge {
            "tuck"
        } else if flicked_past != Edge::None {
            self.cancel_settle();
            self.state.saw_drag.set(false);
            self.stash_at(flicked_past);
            "flickTuck"
        } else if flings && self.throw_from(pointer, velocity) {
            "corner"
        } else if self.settle_into_corner_pulled_into(velocity) {
            "cornerPull"
        } else {
            "placed"
        };
        let thrown = outcome == "corner" || outcome == "cornerPull";
        let came_out = outcome == "pullOut" || outcome == "flickTuck";
        let stamp = self.state.drag_stamp.borrow().clone();
        self.report_release(outcome, &stamp);
        *self.state.last_release.borrow_mut() = json!({
            "outcome": outcome,
            "pointer": internal::point_string(pointer),
            "velocity": internal::point_string(velocity),
            "flings": flings,
            "thrown": thrown,
            "frame": internal::rect_string(self.window.frame()),
        });
        if !thrown && !came_out {
            self.rescue_if_lost();
        }
    }

    pub fn swipe_began(&self, event: &NSEvent) {
        self.stop_motion();
        self.cancel_settle();
        let state = &self.state;
        state.swiping.set(true);
        *state.swipe_stamp.borrow_mut() = internal::begin_stamp();
        state.pulled_from.set(self.stashed_edge());
        let start = self.window.frame();
        state.swipe_start.set(start);
        state.pull_start.set(start);
        let home = self.gesture_home();
        *state.pulled_from_screen.borrow_mut() = home.clone();
        *state.swipe_home.borrow_mut() = home;
        let pointer = internal::pointer_of(self.mtm, event);
        state.swipe_grab.set(NSPoint::new(
            (pointer.x - internal::min_x(start))
                .max(0.0)
                .min(internal::width(start)),
            (pointer.y - internal::min_y(start))
                .max(0.0)
                .min(internal::height(start)),
        ));
        state.swipe_pointer.set(NSEvent::mouseLocation());
        state.swipe_began_at.set(media_time());
        state.swipe_last_delta.set(NSPoint::ZERO);
        if !internal::background() {
            NSCursor::setHiddenUntilMouseMoves(true);
        }
        self.swipe_moved(event);
    }

    pub fn move_by(&self, delta: throw::Point) {
        if delta.x == 0.0 && delta.y == 0.0 {
            return;
        }
        let screens = internal::throw_screens(self.mtm);
        let home = internal::screen_index(self.mtm, self.state.swipe_home.borrow().as_deref());
        let next = throw::swipe_step(
            internal::throw_rect(self.window.frame()),
            delta,
            &screens,
            home,
        );
        let origin = NSPoint::new(next.x, next.y);
        if !internal::equal_points(origin, self.window.frame().origin) {
            self.window.setFrameOrigin(origin);
        }
    }

    pub fn abandon_swipe(&self) {
        if !self.state.swiping.replace(false) {
            return;
        }
        if !internal::background() {
            NSCursor::setHiddenUntilMouseMoves(false);
        }
    }

    pub fn swipe_moved(&self, event: &NSEvent) {
        let delta = throw::finger_delta(
            event.scrollingDeltaX(),
            event.scrollingDeltaY(),
            event.isDirectionInvertedFromDevice(),
        );
        self.state
            .swipe_last_delta
            .set(NSPoint::new(delta.x, delta.y));
        self.state
            .swipe_last_at
            .set(NSProcessInfo::processInfo().systemUptime());
        self.move_by(delta);
    }

    pub fn throw_info(&self) -> Value {
        let state = &self.state;
        json!({
            "lastThrow": Value::Object(state.last_throw.borrow().clone()),
            "lastSwipe": or_empty(&state.last_swipe.borrow()),
            "lastRelease": or_empty(&state.last_release.borrow()),
            "lastScreenFit": or_empty(&state.last_screen_fit.borrow()),
            "lastDrag": or_empty(&state.last_drag.borrow()),
            "swiping": state.swiping.get(),
        })
    }
}

fn or_empty(value: &Value) -> Value {
    if value.is_null() {
        json!({})
    } else {
        value.clone()
    }
}

fn layer_radius_set(layer: &AnyObject) -> f64 {
    // SAFETY: `cornerRadius` is a CALayer property.
    unsafe { msg_send![layer, cornerRadius] }
}
