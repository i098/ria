// Ported from ardia packages/ardiacore/ios/ArdiaChromeWindowHost.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::RefCell;
use std::ptr::NonNull;
use std::rc::Rc;

use block2::RcBlock;

use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{MainThreadMarker, MainThreadOnly, Message};
use objc2_app_kit::{
    NSApplication, NSAutoresizingMaskOptions, NSScreen, NSView, NSWindow,
    NSWindowDidChangeScreenNotification, NSWindowStyleMask,
};
use objc2_foundation::{
    NSNotification, NSNotificationCenter, NSPoint, NSRect, NSSize, NSString, ns_string,
};

use super::ardia_chrome_window_host_dev::DevActions;
use super::ardia_focus_state::FocusState;
use super::ardia_interface_scale as scale;
use super::ardia_traffic_lights::{Placement, frame_corner_radius};
use super::ardia_window_controller::{Controller, Controllers};
use crate::core::bindings::{Profile, Window};
use crate::core::host::State;

pub struct ChromeWindowHost {
    state: Rc<State>,
    controllers: Rc<Controllers>,
    focus: Rc<FocusState>,
    dev: DevActions,
    sizing: Rc<RefCell<Vec<WindowSizing>>>,
    observers: [Retained<AnyObject>; 2],
}

struct WindowSizing {
    window: Weak<NSWindow>,
    minimum: Option<NSSize>,
    content_scale: Option<f64>,
}

impl ChromeWindowHost {
    pub fn new(mtm: MainThreadMarker, state: Rc<State>, controllers: Rc<Controllers>) -> Self {
        let sizing = Rc::new(RefCell::new(Vec::<WindowSizing>::new()));
        let center = NSNotificationCenter::defaultCenter();
        let weak = Rc::downgrade(&sizing);
        let scale_changed = RcBlock::new(move |_: NonNull<NSNotification>| {
            if let Some(sizing) = weak.upgrade() {
                resize_for_scale(&sizing);
            }
        });
        let weak = Rc::downgrade(&sizing);
        let screen_changed = RcBlock::new(move |note: NonNull<NSNotification>| {
            // SAFETY: The notification remains live during this callback.
            let window = unsafe { note.as_ref() }
                .object()
                .and_then(|window| window.downcast::<NSWindow>().ok());
            if let (Some(sizing), Some(window)) = (weak.upgrade(), window) {
                let minimum = sizing
                    .borrow()
                    .iter()
                    .find(|entry| {
                        entry
                            .window
                            .load()
                            .is_some_and(|live| std::ptr::eq(&*live, &*window))
                    })
                    .and_then(|entry| entry.minimum);
                if let Some(minimum) = minimum {
                    window.setMinSize(window_size_from_interface(minimum, &window));
                }
            }
        });
        // SAFETY: Both callback blocks accept AppKit's local main-thread notifications.
        let observers = unsafe {
            [
                center
                    .addObserverForName_object_queue_usingBlock(
                        Some(ns_string!("ArdiaInterfaceScaleDidChange")),
                        None,
                        None,
                        &scale_changed,
                    )
                    .into(),
                center
                    .addObserverForName_object_queue_usingBlock(
                        Some(NSWindowDidChangeScreenNotification),
                        None,
                        None,
                        &screen_changed,
                    )
                    .into(),
            ]
        };
        Self {
            state,
            controllers,
            focus: FocusState::watch(mtm),
            dev: DevActions::default(),
            sizing,
            observers,
        }
    }

    pub fn set_interface_min_size(&self, size: NSSize, window: &NSWindow) {
        self.update_sizing(window, |entry| entry.minimum = Some(size));
        apply_interface_min_size(size, window, true);
    }

    pub fn keep_interface_content_size(&self, window: &NSWindow) {
        self.update_sizing(window, |entry| entry.content_scale = Some(scale::scale()));
    }

    fn update_sizing(&self, window: &NSWindow, update: impl FnOnce(&mut WindowSizing)) {
        let mut sizing = self.sizing.borrow_mut();
        sizing.retain(|entry| entry.window.load().is_some());
        if let Some(entry) = sizing.iter_mut().find(|entry| {
            entry
                .window
                .load()
                .is_some_and(|live| std::ptr::eq(&*live, window))
        }) {
            update(entry);
        } else {
            let mut entry = WindowSizing {
                window: Weak::from(window),
                minimum: None,
                content_scale: None,
            };
            update(&mut entry);
            sizing.push(entry);
        }
    }

    pub fn make_window(
        &self,
        profile: &Profile,
        chrome_token: Option<&NSString>,
        app_window_id: Option<&NSString>,
    ) -> Option<Retained<NSWindow>> {
        if !self.state.is_started() {
            return None;
        }
        let controller = chrome_token
            .and_then(|token| self.controllers.claim_chrome(token))
            .unwrap_or_else(|| self.controllers.make_normal());
        if let Some(id) = app_window_id
            .filter(|id| !id.is_empty() && !id.hasPrefix(objc2_foundation::ns_string!("link:")))
        {
            controller.app_window_id.replace(Some(id.retain()));
        }
        let window = controller.core.window();
        self.activate_profile(&controller, profile);
        Some(window)
    }

    fn activate_profile(&self, controller: &Controller, profile: &Profile) {
        controller.core.set_active_profile(Some(profile));
        let change = if profile.off_the_record() {
            "profile switch private"
        } else {
            "profile switch regular"
        };
        self.focus.record(change, Some(&controller.core.window()));
    }

    pub fn show_profile(&self, window: &NSWindow, profile: &Profile) {
        if let Some(controller) = self.controllers.for_window(window) {
            self.activate_profile(&controller, profile);
        }
    }

    pub fn prepare_profiles<'a>(
        &self,
        window: &NSWindow,
        profiles: impl IntoIterator<Item = &'a Profile>,
    ) {
        if let Some(controller) = self.controllers.for_window(window) {
            for profile in profiles {
                controller.core.prepare_profile(profile);
            }
        }
    }

    pub fn close_window(&self, window: &NSWindow) {
        if let Some(controller) = self.controllers.for_window(window) {
            controller.core.close();
        } else {
            window.close();
        }
    }

    pub fn embed_root(&self, window: &NSWindow, root: &NSView) {
        let Some(controller) = self.controllers.for_window(window) else {
            return;
        };
        let host = controller.core.host_view();
        if let Some(previous) = controller.root.replace(Some(root.retain()))
            && !std::ptr::eq(&*previous, root)
        {
            previous.removeFromSuperview();
        }
        root.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        root.setFrame(host.bounds());
        host.addSubview(root);
        scale::root_attached(root);
    }

    pub fn root_view(&self, window: &NSWindow) -> Option<Retained<NSView>> {
        self.controllers.for_window(window)?.root.borrow().clone()
    }

    pub fn remove_root(&self, window: &NSWindow) {
        if let Some(controller) = self.controllers.for_window(window)
            && let Some(root) = controller.root.take()
        {
            root.removeFromSuperview();
        }
    }

    pub fn set_traffic_lights(&self, window: &NSWindow, placement: Placement, animate: bool) {
        if let Some(controller) = self.controllers.for_window(window)
            && let Some(lights) = &controller.traffic_lights
        {
            if animate {
                lights.move_to(placement);
            } else {
                lights.set_placement(placement);
            }
        }
    }

    pub fn is_fullscreen(&self, window: &NSWindow) -> bool {
        self.controllers.for_window(window).map_or_else(
            || window.styleMask().contains(NSWindowStyleMask::FullScreen),
            |controller| controller.fullscreen.is_full_screen(),
        )
    }

    pub fn user_sees(&self, window: &NSWindow) -> bool {
        self.controllers.user_sees(window)
    }

    pub fn user_sees_only_through_window_server(&self, window: &NSWindow) -> bool {
        self.controllers
            .user_sees_only_through_window_server(window)
    }

    pub fn reconsider_sight(&self, window: &NSWindow) {
        self.controllers.reconsider_sight(window);
    }

    pub fn focus_state(&self, mtm: MainThreadMarker) -> String {
        self.focus
            .summary(&NSApplication::sharedApplication(mtm), &self.controllers)
    }

    pub fn dev_action(
        &self,
        window: &NSWindow,
        action: &str,
        test_instance: bool,
    ) -> Option<String> {
        if !test_instance {
            return Some("test instances only".into());
        }
        self.dev.run(self, &self.controllers, window, action)
    }
}

impl Drop for ChromeWindowHost {
    fn drop(&mut self) {
        let center = NSNotificationCenter::defaultCenter();
        for observer in &self.observers {
            // SAFETY: The tokens belong to this notification center.
            unsafe { center.removeObserver(observer) };
        }
    }
}

fn resize_for_scale(sizing: &RefCell<Vec<WindowSizing>>) {
    let current = scale::scale();
    let mut live = Vec::new();
    sizing.borrow_mut().retain_mut(|entry| {
        let Some(window) = entry.window.load() else {
            return false;
        };
        let previous = entry
            .content_scale
            .as_mut()
            .map(|previous| std::mem::replace(previous, current));
        live.push((window, entry.minimum, previous));
        true
    });
    for (window, minimum, previous) in live {
        if let Some(minimum) = minimum {
            apply_interface_min_size(minimum, &window, false);
        }
        if let Some(previous) = previous
            && previous != current
            && !window.styleMask().contains(NSWindowStyleMask::FullScreen)
        {
            let content = window.contentRectForFrameRect(window.frame()).size;
            let ratio = current / previous;
            let size = NSSize::new(
                (content.width * ratio).round(),
                (content.height * ratio).round(),
            );
            let size = window
                .frameRectForContentRect(NSRect::new(NSPoint::ZERO, size))
                .size;
            let visible = visible_frame(&window).size;
            set_frame_keeping_top(
                &window,
                NSSize::new(
                    size.width.min(visible.width.floor()),
                    size.height.min(visible.height.floor()),
                ),
            );
        }
        if let Some(minimum) = minimum {
            apply_interface_min_size(minimum, &window, true);
        }
    }
}

pub fn embedded_root(window: &NSWindow) -> Option<Retained<NSView>> {
    Window::embedded_root_of(window)
}

pub fn content_view_class() -> &'static AnyClass {
    Window::content_view_class()
}

pub fn window_frame_corner_radius(window: &NSWindow) -> f64 {
    frame_corner_radius(window).unwrap_or(0.)
}

fn visible_frame(window: &NSWindow) -> NSRect {
    window
        .screen()
        .or_else(|| NSScreen::mainScreen(window.mtm()))
        .map_or_else(
            || NSRect::new(NSPoint::ZERO, NSSize::new(f64::MAX, f64::MAX)),
            |screen| screen.visibleFrame(),
        )
}

pub fn window_size_from_interface(size: NSSize, window: &NSWindow) -> NSSize {
    let scaled = scale::window_size_from_root(size);
    let visible = visible_frame(window).size;
    NSSize::new(
        scaled.width.round().min(visible.width.floor()),
        scaled.height.round().min(visible.height.floor()),
    )
}

fn apply_interface_min_size(size: NSSize, window: &NSWindow, grow: bool) {
    let minimum = window_size_from_interface(size, window);
    window.setMinSize(minimum);
    if !grow || window.styleMask().contains(NSWindowStyleMask::FullScreen) {
        return;
    }
    let was = window.frame();
    let size = NSSize::new(
        was.size.width.max(minimum.width),
        was.size.height.max(minimum.height),
    );
    set_frame_keeping_top(window, size);
}

fn set_frame_keeping_top(window: &NSWindow, size: NSSize) {
    let was = window.frame();
    if size == was.size {
        return;
    }
    let visible = visible_frame(window);
    let x = was
        .origin
        .x
        .min(visible.origin.x + visible.size.width - size.width)
        .max(visible.origin.x);
    let y = (was.origin.y + was.size.height - size.height)
        .max(visible.origin.y)
        .min(visible.origin.y + visible.size.height - size.height);
    window.setFrame_display(NSRect::new(NSPoint::new(x, y), size), window.isVisible());
}
