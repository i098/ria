// Ported from ardia packages/ardiacore/ios/ArdiaWebView.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Native page view and the state of its ported modules.
//! Also ports state declarations from `ArdiaWebViewInternal.h`.
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
#[allow(
    dead_code,
    reason = "#110: the modules read these fields once they are wired"
)]
pub struct State {
    tab: Option<Retained<Tab>>,
    controller: RcWeak<Controller>,
    life: TabLifecycle,
    requests: NavigationRequest,
    reports: NavigationReports,
    first_paint: FirstPaint,
    frame_hold: FrameHold,
    stand_in: StandIn,
    reveal: Rc<RefCell<Reveal>>,
    focus: PageFocus,
    element_pick: ElementPick,
    commands: PageCommands,
    script_frames: ScriptFrames,
    notifications: NotificationFrames,
    display_requests: DisplayRequests,
    picture_in_picture: PictureInPicture,
    link_drop: LinkDrop,
    profile: String,
    transfer_key: Option<String>,
    visible: bool,
    selected: bool,
    tab_index: isize,
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
