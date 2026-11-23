// Ported from ardia packages/ardiacore/ios/ArdiaWindowController.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.
// Companion declaration: packages/ardiacore/ios/ArdiaWindowController.h.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::time::Duration;

use block2::RcBlock;
use objc2::rc::{Retained, Weak as ObjcWeak};
use objc2::runtime::{AnyObject, NSObject};
use objc2::{AnyThread, DefinedClass, MainThreadMarker, define_class, msg_send};
use objc2_app_kit::{NSApplication, NSEvent, NSWindow, NSWindowWillCloseNotification};
use objc2_foundation::{
    NSComparisonResult, NSNotification, NSNotificationCenter, NSPoint, NSRect, NSSize, NSString,
    NSUUID,
};
use ria_engine::call::main_queue;

use super::ardia_reserved_keys::{KeyContext, ReservedKeys};
use super::ardia_traffic_lights::TrafficLights;
use super::ardia_window_full_screen::FullScreen;
use crate::core::bindings::{Profile, Tab, Window};

const CHROME_CLAIM: Duration = Duration::from_secs(20);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowKind {
    Normal,
    AwaitingApp,
    Stray,
    Standalone,
}

pub struct Controller {
    pub core: Retained<Window>,
    pub fullscreen: Rc<FullScreen>,
    pub traffic_lights: Option<Rc<TrafficLights>>,
    kind: Cell<WindowKind>,
    token: Option<Retained<NSString>>,
    profile: Option<ObjcWeak<Profile>>,
    mtm: MainThreadMarker,
    owner: Weak<Controllers>,
    _delegate: Retained<Delegate>,
    close_observer: Retained<AnyObject>,
}

pub struct Controllers {
    mtm: MainThreadMarker,
    background: bool,
    allow_occlusion: bool,
    windows: RefCell<HashMap<*const NSWindow, Rc<Controller>>>,
    keys: ReservedKeys,
    should_close: Box<dyn Fn(&NSWindow) -> bool>,
}

impl Controllers {
    pub fn new(
        mtm: MainThreadMarker,
        background: bool,
        allow_occlusion: bool,
        should_close: impl Fn(&NSWindow) -> bool + 'static,
    ) -> Rc<Self> {
        Rc::new(Self {
            mtm,
            background,
            allow_occlusion,
            windows: RefCell::default(),
            keys: ReservedKeys::default(),
            should_close: Box::new(should_close),
        })
    }

    pub fn for_window(&self, window: &NSWindow) -> Option<Rc<Controller>> {
        self.windows
            .borrow()
            .get(&std::ptr::from_ref(window))
            .cloned()
    }

    pub fn all(&self) -> Vec<Rc<Controller>> {
        self.windows.borrow().values().cloned().collect()
    }

    pub fn holding(&self, tab: &Tab) -> Option<Rc<Controller>> {
        let profile = tab.profile()?;
        self.windows
            .borrow()
            .values()
            .find(|item| item.core.tabs_for_profile(&profile).containsObject(tab))
            .cloned()
    }

    pub fn user_sees(&self, window: &NSWindow) -> bool {
        self.for_window(window)
            .is_none_or(|controller| controller.utility() || controller.fullscreen.seen())
    }

    pub fn user_sees_only_through_window_server(&self, window: &NSWindow) -> bool {
        self.for_window(window).is_some_and(|controller| {
            !controller.utility() && controller.fullscreen.seen_only_by_window_server()
        })
    }

    pub fn reconsider_sight(&self, window: &NSWindow) {
        if let Some(controller) = self.for_window(window)
            && !controller.utility()
        {
            controller.fullscreen.reconsider_sight();
        }
    }

    pub fn make_normal(self: &Rc<Self>) -> Rc<Controller> {
        self.make(WindowKind::Normal, None)
    }
    pub fn make_chrome(self: &Rc<Self>) -> Rc<Controller> {
        self.make(WindowKind::AwaitingApp, None)
    }
    pub fn make_stray(self: &Rc<Self>) -> Rc<Controller> {
        self.make(WindowKind::Stray, None)
    }

    pub fn standalone(self: &Rc<Self>, profile: &Profile) -> Rc<Controller> {
        if let Some(existing) = self
            .windows
            .borrow()
            .values()
            .find(|item| {
                item.kind() == WindowKind::Standalone
                    && item
                        .profile
                        .as_ref()
                        .and_then(ObjcWeak::load)
                        .is_some_and(|p| std::ptr::eq(&*p, profile))
            })
            .cloned()
        {
            return existing;
        }
        self.make(WindowKind::Standalone, Some(profile))
    }

    pub fn claim_chrome(&self, token: &NSString) -> Option<Rc<Controller>> {
        let controller = self
            .windows
            .borrow()
            .values()
            .find(|item| {
                item.kind() == WindowKind::AwaitingApp
                    && item
                        .token
                        .as_deref()
                        .is_some_and(|id| id.compare(token) == NSComparisonResult::Same)
            })
            .cloned()?;
        controller.kind.set(WindowKind::Normal);
        Some(controller)
    }

    fn make(self: &Rc<Self>, kind: WindowKind, profile: Option<&Profile>) -> Rc<Controller> {
        let decorated = matches!(kind, WindowKind::Normal | WindowKind::AwaitingApp);
        let size = if decorated {
            NSSize::new(1360., 860.)
        } else {
            NSSize::new(800., 600.)
        };
        // SAFETY: The engine has started and this is the AppKit main thread.
        let core = unsafe {
            Window::init_with_content_rect(self.mtm.alloc(), NSRect::new(NSPoint::ZERO, size))
        };
        if kind == WindowKind::Standalone {
            core.set_internal(true);
        }
        let window = core.window();
        let token = (kind == WindowKind::AwaitingApp).then(|| NSUUID::UUID().UUIDString());
        let traffic_lights = decorated.then(|| TrafficLights::configure(&window));
        let fullscreen = FullScreen::new(&core, self.mtm, self.background, self.allow_occlusion);
        let key = std::ptr::from_ref(&*window);
        let controller = Rc::new_cyclic(|weak: &Weak<Controller>| {
            let delegate = Delegate::new(weak.clone());
            let owner = Rc::downgrade(self);
            let on_close = RcBlock::new(move |_: std::ptr::NonNull<NSNotification>| {
                if let Some(owner) = owner.upgrade() {
                    let closed = owner.windows.borrow_mut().remove(&key);
                    drop(closed);
                }
            });
            // SAFETY: The controller retains the delegate required by the engine's weak reference.
            let close_observer = unsafe {
                core.set_delegate(Some(&delegate));
                NSNotificationCenter::defaultCenter()
                    .addObserverForName_object_queue_usingBlock(
                        Some(NSWindowWillCloseNotification),
                        Some(&window),
                        None,
                        &on_close,
                    )
                    .into()
            };
            Controller {
                core,
                fullscreen,
                traffic_lights,
                kind: Cell::new(kind),
                token,
                profile: profile.map(ObjcWeak::from),
                mtm: self.mtm,
                owner: Rc::downgrade(self),
                _delegate: delegate,
                close_observer,
            }
        });
        self.windows.borrow_mut().insert(key, controller.clone());
        if kind == WindowKind::AwaitingApp {
            let weak = Rc::downgrade(&controller);
            main_queue::after(
                CHROME_CLAIM,
                Box::new(move || {
                    let Some(controller) = weak.upgrade() else {
                        return;
                    };
                    if controller.kind() != WindowKind::AwaitingApp {
                        return;
                    }
                    controller.kind.set(WindowKind::Stray);
                }),
            );
        }
        controller
    }
}

impl Controller {
    pub fn kind(&self) -> WindowKind {
        self.kind.get()
    }
    pub fn chrome_token(&self) -> Option<&NSString> {
        self.token.as_deref()
    }

    fn utility(&self) -> bool {
        matches!(self.kind(), WindowKind::Stray | WindowKind::Standalone)
    }

    fn key_context(&self) -> KeyContext {
        if self.fullscreen.is_full_screen() {
            KeyContext::FullScreenBrowser
        } else {
            KeyContext::Windowed
        }
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        // SAFETY: Detach the engine delegate and remove this controller's observer.
        unsafe {
            self.core.set_delegate(None);
            NSNotificationCenter::defaultCenter().removeObserver(&self.close_observer);
        }
    }
}

struct DelegateState(Weak<Controller>);

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "RiaWindowController"]
    #[ivars = DelegateState]
    struct Delegate;

    impl Delegate {
        #[unsafe(method(windowShouldClose:))]
        fn should_close(&self, window: &Window) -> bool {
            self.ivars().0.upgrade().and_then(|controller| controller.owner.upgrade())
                .is_none_or(|owner| (owner.should_close)(&window.window()))
        }

        #[unsafe(method(window:preHandleKeyEvent:))]
        fn pre_handle_key(&self, _window: &Window, event: &NSEvent) -> bool {
            self.ivars().0.upgrade().is_some_and(|controller| controller.owner.upgrade().is_some_and(|owner| {
                owner.keys.perform(&NSApplication::sharedApplication(controller.mtm), event, controller.key_context())
            }))
        }

        #[unsafe(method(window:handleKeyEvent:))]
        fn handle_key(&self, window: &Window, event: &NSEvent) -> bool {
            self.ivars().0.upgrade().is_some_and(|controller| controller.owner.upgrade().is_some_and(|owner| {
                let app = NSApplication::sharedApplication(controller.mtm);
                let window = window.window();
                owner.keys.keeps_sidebar_key_in_page(&app, &window, event)
            }))
        }
    }
);

impl Delegate {
    fn new(controller: Weak<Controller>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(DelegateState(controller));
        // SAFETY: The allocation has its Rust ivars before NSObject initialization.
        unsafe { msg_send![super(this), init] }
    }
}
