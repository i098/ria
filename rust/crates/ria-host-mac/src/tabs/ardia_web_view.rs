// Ported from ardia packages/ardiacore/ios/ArdiaWebView.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Native page view and the state of its ported modules.
//! Also ports state declarations from `ArdiaWebViewInternal.h`.
//!
//! Changes from Ardia:
//! - The host supplies the weak registry instead of using a process-wide set.
//! - The factory registers the view at creation. Ardia registers it when a tab attaches.
//! - `repaint` takes whether the window's neighbour pages are asleep from the caller
//!   (`tabs::ardia_page_hooks`), where Ardia reads a file-level map of wake records. It leaves out the
//!   leave-screen and frame-hold `holding` terms of `-[ArdiaWebView paints]`: neither exists in `State` yet.
//!
//! See the ArdiaWebView entry in `rust/MAP.md` for pending integration.

use std::cell::RefCell;
use std::rc::{Rc, Weak as RcWeak};

use objc2::rc::{Retained, Weak};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::NSView;
use objc2_foundation::NSRect;

use crate::core::bindings::Tab;
use crate::diagnostics::ardia_field_timing;
use crate::diagnostics::ardia_paint_probe::view_facts;
use crate::diagnostics::ardia_paint_probe_math::View;
use crate::windows::ardia_window_controller::Controller;

use super::ardia_tab_lifecycle::TabLifecycle;
use super::ardia_web_view_context_menu::ElementPick;
use super::ardia_web_view_first_paint::FirstPaint;
use super::ardia_web_view_focus::PageFocus;
use super::ardia_web_view_navigation::{NavigationReports, NavigationRequest};
use super::ardia_web_view_page_commands::PageCommands;
use super::ardia_web_view_page_messages::call_frame;
use super::ardia_web_view_page_messages::{
    DisplayRequests, LinkDrop, NotificationFrames, ScriptFrames,
};
use super::ardia_web_view_painting::{FrameHold, PaintingVisibility};
use super::ardia_web_view_picture_in_picture::PictureInPicture;
use super::ardia_web_view_reveals::Reveal;
use super::ardia_web_view_stand_in::StandIn;

#[derive(Default)]
#[allow(
    dead_code,
    reason = "#110: the modules read these fields once they are wired"
)]
struct State {
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
    warm: bool,
    neighbour: bool,
    standalone: bool,
}

#[allow(
    dead_code,
    reason = "#110: the modules read the state once they are wired"
)]
pub struct Ivars(RefCell<State>);

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "RiaArdiaWebView"]
    #[ivars = Ivars]
    pub struct ArdiaWebView;

    impl ArdiaWebView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }
    }
);

impl ArdiaWebView {
    pub(super) fn new(mtm: MainThreadMarker, live: &LiveViews) -> Retained<Self> {
        let view = Self::alloc(mtm).set_ivars(Ivars(RefCell::new(State {
            visible: true,
            selected: true,
            tab_index: -1,
            ..State::default()
        })));
        // SAFETY: The subclass ivars are initialized before the NSView initializer runs.
        let view: Retained<Self> = unsafe { msg_send![super(view), initWithFrame: NSRect::ZERO] };
        view.setWantsLayer(true);
        live.register(&view);
        view
    }

    fn state(&self) -> std::cell::RefMut<'_, State> {
        self.ivars().0.borrow_mut()
    }

    pub fn visible(&self) -> bool {
        self.state().visible
    }

    pub fn warm(&self) -> bool {
        self.state().warm
    }

    pub fn standalone(&self) -> bool {
        self.state().standalone
    }

    pub fn set_warm(&self, warm: bool) {
        self.state().warm = warm;
    }

    pub fn set_neighbour(&self, neighbour: bool) {
        self.state().neighbour = neighbour;
    }

    pub fn set_standalone(&self, standalone: bool) {
        self.state().standalone = standalone;
    }

    pub fn set_transfer_key(&self, key: Option<String>) {
        self.state().transfer_key = key;
    }

    /// The field-timing mark of `-[ArdiaWebView setVisible:]` (`ArdiaWebView.mm:283`); the other steps of that
    /// method are not ported yet. Returns whether the value changed.
    pub fn set_visible(&self, visible: bool) -> bool {
        let key = {
            let mut state = self.state();
            if state.visible == visible {
                return false;
            }
            state.visible = visible;
            state.transfer_key.clone()
        };
        if let Some(key) = key.filter(|_| visible) {
            ardia_field_timing::mark_at_commit(&key, "view");
        }
        true
    }

    /// `-[ArdiaWebView paintProbeReasons]` before the reasons are folded (`ArdiaPaintProbe.mm:12`). The early
    /// window veil has no Ria type, so it is never up here.
    pub fn paint_facts(&self) -> View {
        let state = self.state();
        let page = state.tab.as_ref().and_then(|tab| tab.view());
        view_facts(
            self,
            page.as_deref(),
            state.frame_hold.awaiting(),
            state.stand_in.lent(),
            state.stand_in.leaving(),
            false,
        )
    }

    /// `-[ArdiaWebView applyPainting]` for the alpha and the hidden flag of the subviews.
    pub fn repaint(&self, neighbours_asleep: bool) {
        let visibility = {
            let state = self.state();
            PaintingVisibility {
                visible: state.visible,
                warm: state.warm,
                neighbour_asleep: neighbours_asleep
                    && state.warm
                    && state.neighbour
                    && !state.visible,
                leaving: false,
                revealing: state.reveal.borrow().active(),
                holding: false,
                awaiting_frame: state.frame_hold.awaiting(),
                showing_stand_in: state.stand_in.lent() && !state.stand_in.leaving(),
            }
        };
        visibility.apply(self);
    }

    /// `ArdiaWebViewsSetFieldTiming` for one view: tells the page script of its tab.
    pub fn send_field_timing(&self, on: bool) {
        if let Some(tab) = &self.state().tab {
            let json = if on {
                r#"{"on":true}"#
            } else {
                r#"{"on":false}"#
            };
            call_frame(tab, "", "fieldTiming", json);
        }
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
