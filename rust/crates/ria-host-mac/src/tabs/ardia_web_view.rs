// Ported from ardia packages/ardiacore/ios/ArdiaWebView.mm and ArdiaWebViewInternal.h at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of those files, with changes.

//! Native page view and the state of its ported modules.
//!
//! Changes from Ardia:
//! - The host supplies the weak registry instead of using a process-wide set.
//! - The factory registers the view at creation. Ardia registers it when a tab attaches.
//! - `PageTab`, window observers, and painting, reveal, focus, and transfer orchestration stay open.

use std::cell::RefCell;
use std::rc::{Rc, Weak as RcWeak};

use objc2::rc::{Retained, Weak};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::NSView;
use objc2_foundation::NSRect;

use crate::core::bindings::Tab;
use crate::windows::ardia_window_controller::Controller;

use super::ardia_tab_lifecycle::TabLifecycle;
use super::ardia_web_view_context_menu::ElementPick;
use super::ardia_web_view_first_paint::FirstPaint;
use super::ardia_web_view_focus::PageFocus;
use super::ardia_web_view_navigation::{NavigationReports, NavigationRequest};
use super::ardia_web_view_page_commands::PageCommands;
use super::ardia_web_view_page_messages::{
    DisplayRequests, LinkDrop, NotificationFrames, ScriptFrames,
};
use super::ardia_web_view_painting::FrameHold;
use super::ardia_web_view_picture_in_picture::PictureInPicture;
use super::ardia_web_view_reveals::Reveal;
use super::ardia_web_view_stand_in::StandIn;

#[derive(Default)]
pub struct State {
    pub tab: Option<Retained<Tab>>,
    pub controller: RcWeak<Controller>,
    pub life: TabLifecycle,
    pub requests: NavigationRequest,
    pub reports: NavigationReports,
    pub first_paint: FirstPaint,
    pub frame_hold: FrameHold,
    pub stand_in: StandIn,
    pub reveal: Rc<RefCell<Reveal>>,
    pub focus: PageFocus,
    pub element_pick: ElementPick,
    pub commands: PageCommands,
    pub script_frames: ScriptFrames,
    pub notifications: NotificationFrames,
    pub display_requests: DisplayRequests,
    pub picture_in_picture: PictureInPicture,
    pub link_drop: LinkDrop,
    pub profile: String,
    pub transfer_key: Option<String>,
    pub visible: bool,
    pub selected: bool,
    pub tab_index: isize,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "RiaArdiaWebView"]
    #[ivars = RefCell<State>]
    pub struct ArdiaWebView;

    impl ArdiaWebView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }
    }
);

impl ArdiaWebView {
    pub fn new(mtm: MainThreadMarker, frame: NSRect, live: &LiveViews) -> Retained<Self> {
        let view = Self::alloc(mtm).set_ivars(RefCell::new(State {
            visible: true,
            selected: true,
            tab_index: -1,
            ..State::default()
        }));
        // SAFETY: The subclass ivars are initialized before the NSView initializer runs.
        let view: Retained<Self> = unsafe { msg_send![super(view), initWithFrame: frame] };
        view.setWantsLayer(true);
        live.register(&view);
        view
    }

    pub fn state(&self) -> &RefCell<State> {
        self.ivars()
    }
}

/// The weak set from `ArdiaWebView.mm:33–36`.
#[derive(Default)]
pub struct LiveViews(RefCell<Vec<Weak<ArdiaWebView>>>);

impl LiveViews {
    fn register(&self, view: &ArdiaWebView) {
        let mut views = self.0.borrow_mut();
        views.retain(|entry| entry.load().is_some());
        views.push(Weak::from(view));
    }

    pub fn all(&self) -> Vec<Retained<ArdiaWebView>> {
        let mut live = Vec::new();
        self.0.borrow_mut().retain(|entry| {
            if let Some(view) = entry.load() {
                live.push(view);
                true
            } else {
                false
            }
        });
        live
    }
}

impl super::page_engine::LiveViews for LiveViews {
    fn begin_handing_off(&self, key: &str) {
        for view in self.all() {
            let mut state = view.state().borrow_mut();
            if state.tab.is_some() && state.transfer_key.as_deref() == Some(key) {
                state.life.handing_off = true;
            }
        }
    }

    fn release_transfer(&self, key: &str) {
        for view in self.all() {
            let mut state = view.state().borrow_mut();
            if state.transfer_key.as_deref() == Some(key) {
                state.life.handing_off = false;
            }
        }
    }
}

#[cfg(debug_assertions)]
mod debug_events;
#[cfg(debug_assertions)]
pub use debug_events::{note_event, recent_events};
