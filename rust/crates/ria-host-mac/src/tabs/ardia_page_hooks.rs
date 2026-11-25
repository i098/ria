// Ria original, no Ardia source.

//! The page host's answers to the hooks of other units, over the `ArdiaWebView` set of one window group.
//! `PageHooks` implements `SwipeHooks` (the neighbour-page wake and release of `ArdiaWebView+Painting.mm:208–235`,
//! the window root view of `ArdiaChromeWindowHost`) and `RevealHooks` (the class methods of
//! `ArdiaWebView+RevealInput.mm:74–120` over one `RevealInput`). It also supplies the paint facts of the paint
//! probe and the field-timing switch.
//!
//! Changes from Ardia: Ardia keeps these as class methods over file-level globals (`NeighbourPagesAwake()`,
//! `gOnScreenAhead`, `gPresses`); here one `PageHooks` value holds the wake records and the `RevealInput`, and the
//! host installs it once with `install`. The window root view and the app-wide key action are closures of the host
//! because `windows/` and the key tables are not reachable from `tabs/`. The early-window veil has no Ria type, so
//! the paint facts never report it. Not yet reached: no code in `tabs/` enters a reveal gap, sets `warm` or
//! `neighbour` on a view, or calls the field-timing marks of navigation and reveals (`ArdiaWebView.mm:283` is the
//! one mark that `ArdiaWebView::set_visible` makes); those callers belong to the rest of #110. Still open: no
//! production code calls `ChromiumTabView::make_page` or builds `LiveViews`, so nothing installs these hooks at
//! runtime yet; no code calls `PageHooks::paint_probe` or `set_field_timing`; the engine `SwipeModule` calls into
//! `swipe_module::call` and `dev_simulate`, and the `leaving` and `holding` terms, also have no host.

use std::cell::RefCell;
use std::ptr;
use std::rc::{Rc, Weak as RcWeak};
use std::time::{Duration, Instant};

use objc2::rc::{Retained, Weak};
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, Message};
use objc2_app_kit::{NSEvent, NSView, NSWindow};
use ria_engine::call::main_queue;

use super::ardia_web_view::{ArdiaWebView, LiveViews};
use super::ardia_web_view_painting::NeighbourWake;
use super::ardia_web_view_reveal_input::RevealInput;
use crate::diagnostics::ardia_field_timing;
use crate::diagnostics::ardia_paint_probe::PaintProbe;
use crate::diagnostics::ardia_paint_probe_math::View;
use crate::diagnostics::ardia_reveal_guards::{self, RevealHooks};
use crate::swipe::ardia_swipe::{self, SwipeHooks};
use crate::windows::ardia_window_controller::Controllers;

/// `kNeighbourPagesAwakeSeconds` (`ArdiaWebViewInternal.h:382`).
const NEIGHBOUR_PAGES_AWAKE: f64 = 10.0;

thread_local! {
    static INSTALLED: RefCell<RcWeak<PageHooks>> = const { RefCell::new(RcWeak::new()) };
}

type RootView = Box<dyn Fn(&NSWindow) -> Option<Retained<NSView>>>;

pub struct PageHooks {
    me: RcWeak<PageHooks>,
    live: Rc<LiveViews>,
    root_view: RootView,
    app_wide_key: Box<dyn Fn(&NSEvent)>,
    input: RefCell<RevealInput>,
    wakes: RefCell<Vec<(Weak<NSWindow>, NeighbourWake)>>,
}

/// `InTabPage` of `ArdiaWebView+Focus.mm:13`: the nearest web view above `view` is not standalone.
pub fn in_tab_page(view: &NSView) -> bool {
    let mut walk = Some(view.retain());
    while let Some(current) = walk {
        let any: &AnyObject = &current;
        if let Some(web_view) = any.downcast_ref::<ArdiaWebView>() {
            return !web_view.standalone();
        }
        // SAFETY: The view hierarchy is read on the main thread.
        walk = unsafe { current.superview() };
    }
    false
}

fn same_window(stored: &Weak<NSWindow>, window: &NSWindow) -> bool {
    stored
        .load()
        .is_some_and(|stored| ptr::eq(&*stored, window))
}

impl PageHooks {
    pub fn new(
        live: Rc<LiveViews>,
        root_view: impl Fn(&NSWindow) -> Option<Retained<NSView>> + 'static,
        app_wide_key: impl Fn(&NSEvent) + 'static,
    ) -> Rc<Self> {
        Rc::new_cyclic(|me| Self {
            me: me.clone(),
            live,
            root_view: Box::new(root_view),
            app_wide_key: Box::new(app_wide_key),
            input: RefCell::default(),
            wakes: RefCell::default(),
        })
    }

    /// Registers the hooks with the swipe unit and the reveal guards, and for the field-timing switch.
    /// The reveal guards keep the hooks of the first call.
    pub fn install(self: &Rc<Self>, mtm: MainThreadMarker) {
        if INSTALLED
            .with_borrow(|installed| installed.upgrade().is_some_and(|i| Rc::ptr_eq(&i, self)))
        {
            return;
        }
        INSTALLED.set(Rc::downgrade(self));
        ardia_swipe::set_hooks(self.clone());
        ardia_reveal_guards::install(mtm, self.clone());
    }

    pub fn live(&self) -> &LiveViews {
        &self.live
    }

    /// The reveal input of the gaps; the page host's reveal code enters and leaves gaps through it.
    pub fn reveal_input(&self) -> &RefCell<RevealInput> {
        &self.input
    }

    fn asleep(&self, window: &NSWindow) -> bool {
        let now = Instant::now();
        self.wakes
            .borrow()
            .iter()
            .find(|(stored, _)| same_window(stored, window))
            .is_none_or(|(_, wake)| wake.until().is_none_or(|until| until <= now))
    }

    /// `+[ArdiaWebView repaintWarmPagesIn:]`.
    fn repaint_warm(&self, window: &NSWindow) {
        let asleep = self.asleep(window);
        for view in self.live.all() {
            if view.warm() && view.window().is_some_and(|owner| ptr::eq(&*owner, window)) {
                view.repaint(asleep);
            }
        }
    }

    /// `+[ArdiaWebView sleepNeighbourPagesIn:after:]`.
    fn sleep_after(&self, window: &NSWindow, delay: Duration) {
        let (me, window) = (self.me.clone(), Weak::from(window));
        main_queue::after(
            delay,
            Box::new(move || {
                let (Some(hooks), Some(window)) = (me.upgrade(), window.load()) else {
                    return;
                };
                let left = {
                    let wakes = hooks.wakes.borrow();
                    let Some((_, wake)) = wakes
                        .iter()
                        .find(|(stored, _)| same_window(stored, &window))
                    else {
                        return;
                    };
                    wake.until().map_or(Duration::ZERO, |until| {
                        until.saturating_duration_since(Instant::now())
                    })
                };
                if !left.is_zero() {
                    return hooks.sleep_after(&window, left);
                }
                hooks
                    .wakes
                    .borrow_mut()
                    .retain(|(stored, _)| !same_window(stored, &window));
                hooks.repaint_warm(&window);
            }),
        );
    }

    /// The facts of every visible web view of `window`, for `PaintProbe::new` (`ArdiaPaintProbe.mm:83`).
    pub fn paint_views(&self, window: &NSWindow) -> Vec<View> {
        self.live
            .all()
            .iter()
            .filter(|view| {
                view.visible() && view.window().is_some_and(|owner| ptr::eq(&*owner, window))
            })
            .map(|view| view.paint_facts())
            .collect()
    }

    /// A paint probe over these hooks' web views.
    pub fn paint_probe(
        &self,
        controllers: Rc<Controllers>,
        emit: impl Fn(&str, serde_json::Value) + 'static,
        consent_epoch: impl Fn() -> String + 'static,
    ) -> Rc<PaintProbe> {
        let me = self.me.clone();
        PaintProbe::new(
            controllers,
            move |window| {
                me.upgrade()
                    .map_or_else(Vec::new, |hooks| hooks.paint_views(window))
            },
            emit,
            consent_epoch,
        )
    }
}

/// `ArdiaFieldTiming` `SetEnabled`, with the web-view switch of the installed hooks.
pub fn set_field_timing(enabled: bool) {
    ardia_field_timing::set_enabled(enabled, |on| {
        if let Some(hooks) = INSTALLED.with_borrow(RcWeak::upgrade) {
            for view in hooks.live.all() {
                view.send_field_timing(on);
            }
        }
    });
}

impl SwipeHooks for PageHooks {
    fn root_view(&self, window: &NSWindow) -> Option<Retained<NSView>> {
        (self.root_view)(window)
    }

    fn wake_neighbour_pages(&self, window: &NSWindow) {
        self.wake_neighbour_pages_for(window, NEIGHBOUR_PAGES_AWAKE, true);
    }

    fn wake_neighbour_pages_for(&self, window: &NSWindow, seconds: f64, committed: bool) {
        let duration = Duration::from_secs_f64(seconds.max(0.0));
        let was_asleep = {
            let mut wakes = self.wakes.borrow_mut();
            let index = wakes
                .iter()
                .position(|(stored, _)| same_window(stored, window))
                .unwrap_or_else(|| {
                    wakes.push((Weak::from(window), NeighbourWake::default()));
                    wakes.len() - 1
                });
            wakes[index].1.wake(Instant::now(), duration, committed)
        };
        if was_asleep {
            self.repaint_warm(window);
            self.sleep_after(window, duration);
        }
    }

    fn release_neighbour_pages(&self, window: &NSWindow) {
        let asleep_now = self
            .wakes
            .borrow_mut()
            .iter_mut()
            .find(|(stored, _)| same_window(stored, window))
            .is_some_and(|(_, wake)| wake.release(Instant::now()));
        if asleep_now {
            self.repaint_warm(window);
        }
    }
}

impl RevealHooks for PageHooks {
    fn refuses_view(&self, view: &NSView) -> bool {
        self.input
            .try_borrow()
            .is_ok_and(|input| input.refuses_view(view, in_tab_page))
    }

    fn any_gap(&self) -> bool {
        self.input.try_borrow().is_ok_and(|input| input.any_gap())
    }

    fn swallows_ahead(&self, event: &NSEvent) -> bool {
        self.input.try_borrow_mut().is_ok_and(|mut input| {
            input.swallows(event, in_tab_page, |key| (self.app_wide_key)(key))
        })
    }
}
