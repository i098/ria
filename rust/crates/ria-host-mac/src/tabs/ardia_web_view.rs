// Ported from ardia packages/ardiacore/ios/ArdiaWebView.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Native page view and the state of its ported modules.
//! Also ports state declarations from `ArdiaWebViewInternal.h`.
//!
//! Changes from Ardia:
//! - The host supplies the weak registry instead of using a process-wide set.
//! - The factory registers the view at creation. Ardia registers it when a tab attaches.
//! - `attach_tab` is the part of `-[ArdiaWebView ensureTab]` that mounts a tab that already exists. The caller says
//!   whether the tab is `created`. Only a created tab runs the first-paint hold (`holdPageUntilPainted:`) in the attach
//!   step, and the app passes `created` for the launch tab alone; every other tab skips the hold until its settle
//!   caller is ported. The reveal and focus steps are skipped until those modules are ported (#110). Of the settle
//!   callers, only `showHeldPageAfterLoad` runs, from the load-finished callback of the launch tab
//!   (`ArdiaWebView+Navigation.mm:138`); the 4 s timeout is the other end that runs by itself.
//! - Painting: `schedule_painting` posts one paint per view to the main queue, where Ardia drains a set of views in one
//!   run-loop observer in a fixed order (hidden views first). `apply_painting` leaves out `leaveScreen`, the holder
//!   (`holdFor:`, `keepPaneBelow:`), drop acceptance, focus and the swap report. The frame hold begins only when a
//!   live view's page is on screen in the same slot; the lingering New Tab pane is not ported. A hold polls the frame
//!   once per frame (`FRAME_INTERVAL`) where Ardia checks before each run-loop wait.
//! - `repaint` takes whether the window's neighbour pages are asleep from the caller
//!   (`tabs::ardia_page_hooks`); `apply_painting` reads it from the hooks. The leave-screen and holder terms of
//!   `-[ArdiaWebView paints]` are never set.
//!
//! See the ArdiaWebView entry in `rust/MAP.md` for pending integration.

use std::cell::RefCell;
use std::rc::{Rc, Weak as RcWeak};

use objc2::rc::{Retained, Weak};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::NSView;
use objc2_foundation::{NSRect, NSSize};
use ria_engine::call::main_queue;

use crate::core::bindings::{Adoption, Tab};
use crate::diagnostics::ardia_field_timing;
use crate::diagnostics::ardia_paint_probe::view_facts;
use crate::diagnostics::ardia_paint_probe_math::View;
use crate::windows::ardia_interface_scale::fit_page_host;
use crate::windows::ardia_window_controller::Controller;

use super::ardia_page_hooks::PageHooks;
use super::ardia_tab_registry::TabRegistry;
use super::ardia_web_view_context_menu::ElementPick;
use super::ardia_web_view_first_paint::{FRAME_INTERVAL, PagePaintHold, PaintSurface};
use super::ardia_web_view_focus::PageFocus;
use super::ardia_web_view_navigation::{NavigationReports, NavigationRequest};
use super::ardia_web_view_page_commands::PageCommands;
use super::ardia_web_view_page_messages::call_frame;
use super::ardia_web_view_page_messages::{
    DisplayRequests, LinkDrop, NotificationFrames, ScriptFrames,
};
use super::ardia_web_view_painting::{
    FRAME_HOLD_TIMEOUT, FrameHold, PaintingVisibility, layout_page, render_widget_in,
};
use super::ardia_web_view_picture_in_picture::PictureInPicture;
use super::ardia_web_view_reveals::Reveal;
use super::ardia_web_view_stand_in::{StandIn, without_actions};
use super::ardia_web_view_tab::{Attach, AttachStep, AttachedTab, StashedReports, small_window};
use super::superview;

#[derive(Default)]
#[allow(
    dead_code,
    reason = "#110: the modules read these fields once they are wired"
)]
struct State {
    attached: AttachedTab<NavigationReports>,
    stashed: StashedReports<NavigationReports>,
    controller: RcWeak<Controller>,
    requests: NavigationRequest,
    paint_hold: Option<Rc<PagePaintHold>>,
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
    hooks: RcWeak<PageHooks>,
    painting_scheduled: bool,
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

        /// `-[ArdiaWebView setFrameSize:]` (`ArdiaWebView+Painting.mm:131`).
        #[unsafe(method(setFrameSize:))]
        fn set_frame_size(&self, size: NSSize) {
            // SAFETY: NSView's own `setFrameSize:` takes the size.
            let _: () = unsafe { msg_send![super(self), setFrameSize: size] };
            fit_page_host(self);
            self.layout_page();
        }
    }
);

impl ArdiaWebView {
    pub(super) fn new(mtm: MainThreadMarker, hooks: &Rc<PageHooks>) -> Retained<Self> {
        let view = Self::alloc(mtm).set_ivars(Ivars(RefCell::new(State {
            visible: true,
            selected: true,
            tab_index: -1,
            hooks: Rc::downgrade(hooks),
            // shortcut: the consent epoch is empty, so cold-paint counts are not recorded; read it from the app once
            // the engine caller of `ArdiaEvents.consentEpoch` exists (#111).
            paint_hold: Some(PagePaintHold::new(String::new)),
            ..State::default()
        })));
        // SAFETY: The subclass ivars are initialized before the NSView initializer runs.
        let view: Retained<Self> = unsafe { msg_send![super(view), initWithFrame: NSRect::ZERO] };
        view.setWantsLayer(true);
        hooks.live().register(&view);
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
        let changed = std::mem::replace(&mut self.state().warm, warm) != warm;
        if changed {
            self.schedule_painting();
        }
    }

    pub fn set_neighbour(&self, neighbour: bool) {
        let changed = std::mem::replace(&mut self.state().neighbour, neighbour) != neighbour;
        if changed {
            self.schedule_painting();
        }
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
        self.schedule_painting();
        true
    }

    /// `-[ArdiaWebView paintProbeReasons]` before the reasons are folded (`ArdiaPaintProbe.mm:12`). The early
    /// window veil has no Ria type, so it is never up here.
    pub fn paint_facts(&self) -> View {
        let state = self.state();
        let page = state.attached.tab.as_ref().and_then(|tab| tab.view());
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
        self.visibility(neighbours_asleep).apply(self);
    }

    fn visibility(&self, neighbours_asleep: bool) -> PaintingVisibility {
        Self::visibility_of(&self.state(), neighbours_asleep)
    }

    fn visibility_of(state: &State, neighbours_asleep: bool) -> PaintingVisibility {
        PaintingVisibility {
            visible: state.visible,
            warm: state.warm,
            neighbour_asleep: neighbours_asleep && state.warm && state.neighbour && !state.visible,
            leaving: false,
            revealing: state.reveal.borrow().active(),
            holding: false,
            awaiting_frame: state.frame_hold.awaiting(),
            showing_stand_in: state.stand_in.lent() && !state.stand_in.leaving(),
        }
    }

    /// `-[ArdiaWebView ensureTab]` for a tab that already exists (`ArdiaWebView+Tab.mm`): mount the tab's page in
    /// this view, key the view in `registry` under `app_id`, and lay the page out. The steps that need the
    /// unported first-paint, reveal and focus code are skipped (#110).
    pub fn attach_tab(
        &self,
        tab: Retained<Tab>,
        app_id: String,
        registry: &RefCell<TabRegistry>,
        created: bool,
    ) -> Adoption {
        // The steps repaint through `state`, so the attached tab is out of it while they run.
        let (mut attached, mut stashed) = {
            let mut state = self.state();
            (
                std::mem::take(&mut state.attached),
                std::mem::take(&mut state.stashed),
            )
        };
        let page_tab = tab.clone();
        let adoption = attached.attach(
            tab,
            Attach {
                host: self,
                app_id,
                window: None,
                placement: None,
                shown_early: false,
                created,
                temporary: false,
                muted: false,
                page_background: None,
                contents_background: None,
                page_color: None,
            },
            registry,
            &mut stashed,
            |_, _| {},
            |step| {
                match step {
                    AttachStep::LayoutAndPaint => {
                        // The attached tab is out of `state` here, so lay the page out from `page_tab`.
                        layout_page(self, Some(&*page_tab), None, None, None, true);
                    }
                    AttachStep::HoldForFirstPaint { created } => {
                        if created && page_tab.loading() {
                            self.hold_page_until_painted(&page_tab, created);
                        }
                    }
                    _ => {}
                }
            },
        );
        let mut state = self.state();
        state.attached = attached;
        state.stashed = stashed;
        drop(state);
        self.schedule_painting();
        adoption
    }

    /// `-[ArdiaWebView holdPageUntilPainted:]` (`ArdiaWebView+FirstPaint.mm:46`): hide the page of `tab` until its
    /// first paint, unless the view is not a cold tab.
    fn hold_page_until_painted(&self, tab: &Tab, created: bool) {
        let (hold, surface) = {
            let state = self.state();
            (
                state.paint_hold.clone(),
                PaintSurface {
                    attached: true,
                    small_window: small_window(self.window().as_deref()),
                    temporary: false,
                    standalone: state.standalone,
                    extension_host: false,
                    visible_or_revealing: state.visible || state.reveal.borrow().active(),
                    opacity: 1.0,
                },
            )
        };
        if let Some(hold) = hold {
            hold.hold(tab, self, created, surface);
        }
    }

    /// `-[ArdiaWebView showHeldPageAfterLoad]` (`ArdiaWebView+FirstPaint.mm:103`).
    pub fn show_held_page_after_load(&self) {
        let hold = self.state().paint_hold.clone();
        if let Some(hold) = hold {
            hold.show_after_load();
        }
    }

    /// `-[ArdiaWebView layoutPage]` (`ArdiaWebView+Painting.mm:148`) without the fronted and stand-in pages, which no
    /// app code mounts yet.
    pub fn layout_page(&self) {
        let asleep = self.neighbours_asleep();
        let (tab, devtools, paints) = {
            let state = self.state();
            (
                state.attached.tab.clone(),
                state.attached.devtools.clone(),
                Self::visibility_of(&state, asleep).paints(),
            )
        };
        layout_page(
            self,
            tab.as_deref(),
            None,
            None,
            devtools.as_deref(),
            paints,
        );
    }

    /// `-[ArdiaWebView neighbourPageAsleep]` (`ArdiaWebView+Painting.mm:190`).
    fn neighbours_asleep(&self) -> bool {
        let hooks = self.state().hooks.upgrade();
        match (hooks, self.window()) {
            (Some(hooks), Some(window)) => hooks.asleep(&window),
            _ => false,
        }
    }

    /// `-[ArdiaWebView schedulePainting]` (`ArdiaWebView+Painting.mm:329`): paint once on the next main-queue turn.
    pub fn schedule_painting(&self) {
        if std::mem::replace(&mut self.state().painting_scheduled, true) {
            return;
        }
        let weak = Weak::from(self);
        main_queue::post(Box::new(move || {
            if let Some(view) = weak.load() {
                view.state().painting_scheduled = false;
                view.apply_painting();
            }
        }));
    }

    /// `-[ArdiaWebView applyPainting]` (`ArdiaWebView+Painting.mm:233`), without the leave-screen, drop-acceptance,
    /// focus and swap-report steps.
    fn apply_painting(&self) {
        let (visible, revealing, awaiting) = {
            let state = self.state();
            (
                state.visible,
                state.reveal.borrow().active(),
                state.frame_hold.awaiting(),
            )
        };
        if awaiting && !visible && !revealing {
            self.finish_frame_hold();
        }
        if visible || revealing {
            self.expose_page();
        }
        self.begin_frame_hold_if_cold();
        let visibility = self.visibility(self.neighbours_asleep());
        without_actions(|| {
            let bounds = self.bounds();
            if visibility.paints() && (bounds.size.width <= 0.0 || bounds.size.height <= 0.0) {
                self.layout_page();
            }
            visibility.apply(self);
        });
    }

    /// `-[ArdiaWebView exposePage]` (`ArdiaWebView+Painting.mm:304`).
    fn expose_page(&self) {
        let tab = self.state().attached.tab.clone();
        if let Some(tab) = tab
            && self.window().is_some()
            && tab.view().is_some_and(|page| self.hosts(&page))
        {
            tab.note_in_shown_slot();
        }
    }

    fn hosts(&self, page: &NSView) -> bool {
        superview(page).is_some_and(|parent| std::ptr::eq(&*parent, &**self))
    }

    fn in_window(&self) -> NSRect {
        self.convertRect_toView(self.bounds(), None)
    }

    /// `-[ArdiaWebView sharesScreenWith:]` (`ArdiaWebView+Painting.mm:380`).
    fn shares_screen_with(&self, other: &ArdiaWebView) -> bool {
        let same_window = match (self.window(), other.window()) {
            (Some(mine), Some(theirs)) => std::ptr::eq(&*mine, &*theirs),
            _ => other.window().is_none() && self.window().is_none(),
        };
        !std::ptr::eq(self, other)
            && same_window
            && self.state().profile == other.state().profile
            && self.in_window() == other.in_window()
    }

    /// `-[ArdiaWebView showsPageFor:]` (`ArdiaWebView+Painting.mm:385`).
    fn shows_page_for(&self, incoming: &ArdiaWebView) -> bool {
        let (page, awaiting) = {
            let state = self.state();
            (
                state.attached.tab.as_ref().and_then(|tab| tab.view()),
                state.frame_hold.awaiting(),
            )
        };
        self.shares_screen_with(incoming)
            && !awaiting
            && page.is_some_and(|page| self.hosts(&page) && !page.isHidden())
            && self.alphaValue() > 0.0
            && !self.isHiddenOrHasHiddenAncestor()
    }

    /// `-[ArdiaWebView beginFrameHoldIfCold]` (`ArdiaWebView+Painting.mm:398`) for a page that takes over from
    /// another view's page; the lingering New Tab pane is not ported.
    fn begin_frame_hold_if_cold(&self) {
        let (tab, standalone, wanted, awaiting, hooks) = {
            let state = self.state();
            (
                state.attached.tab.clone(),
                state.standalone,
                state.visible || state.reveal.borrow().active(),
                state.frame_hold.awaiting(),
                state.hooks.upgrade(),
            )
        };
        let (Some(tab), Some(window), Some(hooks)) = (tab, self.window(), hooks) else {
            return;
        };
        let Some(page) = tab.view() else { return };
        let rect = self.in_window();
        if awaiting
            || standalone
            || !wanted
            || !self.hosts(&page)
            || !page.isHidden()
            || small_window(Some(&window))
            || rect.size.width <= 0.0
            || rect.size.height <= 0.0
        {
            return;
        }
        if !hooks
            .live()
            .all()
            .iter()
            .any(|other| other.shows_page_for(self))
        {
            return;
        }
        let Some(widget) = render_widget_in(&page) else {
            return;
        };
        let Some(generation) = self.state().frame_hold.begin(&widget) else {
            return;
        };
        self.watch_frame(generation);
        let weak = Weak::from(self);
        main_queue::after(
            FRAME_HOLD_TIMEOUT,
            Box::new(move || {
                if let Some(view) = weak.load() {
                    view.frame_shown(generation, false);
                }
            }),
        );
    }

    /// `+[ArdiaWebView watchFrames]` (`ArdiaWebView+Painting.mm:431`): Ardia checks every awaiting view before each
    /// run-loop wait; here each view polls once per frame until its hold ends.
    fn watch_frame(&self, generation: u64) {
        let weak = Weak::from(self);
        main_queue::after(
            FRAME_INTERVAL,
            Box::new(move || {
                let Some(view) = weak.load() else { return };
                let changed = {
                    let state = view.state();
                    if state.frame_hold.current() != Some(generation) {
                        return;
                    }
                    state
                        .attached
                        .tab
                        .as_ref()
                        .is_none_or(|tab| state.frame_hold.frame_changed(tab))
                };
                if changed {
                    view.frame_shown(generation, true);
                } else {
                    view.watch_frame(generation);
                }
            }),
        );
    }

    /// `-[ArdiaWebView frameShown:]` (`ArdiaWebView+Painting.mm:459`) for the hold of `generation`.
    fn frame_shown(&self, generation: u64, _drawn: bool) {
        if self.state().frame_hold.current() != Some(generation) {
            return;
        }
        without_actions(|| {
            self.finish_frame_hold();
            self.apply_painting();
        });
    }

    /// `-[ArdiaWebView finishFrameHold]` (`ArdiaWebView+Painting.mm:468`) without the stand-in and pane steps.
    fn finish_frame_hold(&self) {
        let mut state = self.state();
        if let Some(generation) = state.frame_hold.current() {
            state.frame_hold.finish(generation);
        }
    }
    /// Whether the view's tab has closed, or it has no tab.
    pub fn tab_closed(&self) -> bool {
        self.state()
            .attached
            .tab
            .as_ref()
            .is_none_or(|tab| tab.closed())
    }

    /// Whether the view shows `tab`.
    pub fn shows_tab(&self, tab: &Tab) -> bool {
        self.state()
            .attached
            .tab
            .as_ref()
            .is_some_and(|attached| std::ptr::eq(&**attached, tab))
    }

    /// The opacity of the layer of the page the view shows; the first-paint hold sets it to 0.
    pub fn page_opacity(&self) -> Option<f32> {
        let page = self.state().attached.tab.as_ref()?.view()?;
        let layer = page.layer()?;
        // SAFETY: NSView.layer is a CALayer, whose opacity is a float.
        Some(unsafe { msg_send![&*layer, opacity] })
    }

    /// `ArdiaWebViewsSetFieldTiming` for one view: tells the page script of its tab.
    pub fn send_field_timing(&self, on: bool) {
        if let Some(tab) = &self.state().attached.tab {
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
