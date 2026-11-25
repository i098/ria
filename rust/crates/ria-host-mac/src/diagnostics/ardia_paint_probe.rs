// Ported from ardia packages/ardiacore/ios/ArdiaPaintProbe.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Changes from Ardia: the probe is a `PaintProbe` value and not file globals. The page host gives the paint
//! facts of each visible web view through `views`, and the events and consent epoch come as closures.
//! `tabs::ardia_page_hooks::PageHooks::paint_probe` supplies `views` from `ArdiaWebView::paint_facts`, which calls
//! `view_facts`. `launch::window_host::WindowHost::new` builds and starts the probe (`watch`) for the app, where Ardia starts it in
//! `ArdiaFocusState.mm:185`. The early-window veil still has no Ria type, so the veil is never reported; the stand-in and
//! frame-hold facts come from the view's `StandIn` and `FrameHold`.

use std::cell::{Cell, RefCell};
use std::ptr::NonNull;
use std::rc::Rc;
use std::time::Duration;

use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2::rc::{Retained, Weak};
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSApplication, NSView, NSWindow, NSWindowDidBecomeKeyNotification,
    NSWindowDidResignKeyNotification, NSWindowOcclusionState, NSWindowWillCloseNotification,
};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSNotificationName, ns_string};
use ria_engine::call::main_queue;
use serde_json::{Value, json};

use super::ardia_paint_probe_math::{Episode, REASON_COUNT, Tally, View, reasons_of};
use crate::windows::ardia_window_controller::Controllers;

const PAINT_CHECK_AFTER: Duration = Duration::from_millis(500);
const REASON_NAMES: [&str; REASON_COUNT as usize] = [
    "alpha 0",
    "view hidden",
    "page hidden",
    "page detached",
    "frame hold",
    "stand-in lent",
    "stand-in leaving",
    "veil",
];

/// `-[ArdiaWebView paintProbeReasons]` before the reasons are folded: the web view `host`, its tab page and the
/// web view's own state.
pub fn view_facts(
    host: &NSView,
    page: Option<&NSView>,
    awaiting_frame: bool,
    stand_in_lent: bool,
    stand_in_leaving: bool,
    veil_up: bool,
) -> View {
    View {
        alpha_zero: host.alphaValue() <= 0.01,
        view_hidden: host.isHiddenOrHasHiddenAncestor(),
        page_hidden: page.is_some_and(|page| page.isHiddenOrHasHiddenAncestor()),
        page_detached: page.is_none_or(|page| {
            // SAFETY: The view hierarchy is read on the main thread.
            unsafe { page.superview() }.is_none_or(|parent| !std::ptr::eq(&*parent, host))
        }),
        awaiting_frame,
        stand_in_lent,
        stand_in_leaving,
        veil_up,
    }
}

fn reason_names(reasons: u32) -> String {
    if reasons == 0 {
        return "none".into();
    }
    let found: Vec<&str> = (0..REASON_COUNT)
        .filter(|bit| reasons & (1 << bit) != 0)
        .map(|bit| REASON_NAMES[bit as usize])
        .collect();
    found.join(", ")
}

fn describe_paint(record: &Value) -> String {
    let flag = |key: &str| u8::from(record[key].as_bool().unwrap_or(false));
    let text = |key: &str| record[key].as_str().unwrap_or_default().to_owned();
    format!(
        "{} window of {}, trigger {}, seen {}, occlusion {}, overruled {}, app active {}, views {}, unpainted {} ({})",
        text("window"),
        record["windows"],
        text("trigger"),
        flag("seen"),
        text("occlusion"),
        flag("overruled"),
        flag("appActive"),
        record["views"],
        record["unpainted"],
        reason_names(record["reasons"].as_u64().unwrap_or(0) as u32)
    )
}

struct Watch {
    window: Weak<NSWindow>,
    episode: Episode,
    generation: u64,
}

type Views = Box<dyn Fn(&NSWindow) -> Vec<View>>;
type Emit = Box<dyn Fn(&str, Value)>;

pub struct PaintProbe {
    controllers: Rc<Controllers>,
    views: Views,
    emit: Emit,
    consent_epoch: Box<dyn Fn() -> String>,
    watches: RefCell<Vec<Watch>>,
    last_check: RefCell<Option<String>>,
    last_record: RefCell<Option<String>>,
    episodes: Cell<usize>,
    observers: RefCell<Vec<Retained<AnyObject>>>,
}

impl PaintProbe {
    pub fn new(
        controllers: Rc<Controllers>,
        views: impl Fn(&NSWindow) -> Vec<View> + 'static,
        emit: impl Fn(&str, Value) + 'static,
        consent_epoch: impl Fn() -> String + 'static,
    ) -> Rc<Self> {
        Rc::new(Self {
            controllers,
            views: Box::new(views),
            emit: Box::new(emit),
            consent_epoch: Box::new(consent_epoch),
            watches: RefCell::default(),
            last_check: RefCell::default(),
            last_record: RefCell::default(),
            episodes: Cell::new(0),
            observers: RefCell::default(),
        })
    }

    pub fn line(&self) -> String {
        format!(
            "paint last check {}\npaint unpainted episodes {}{}",
            self.last_check.borrow().as_deref().unwrap_or("not checked"),
            self.episodes.get(),
            self.last_record
                .borrow()
                .as_ref()
                .map_or(String::new(), |record| format!(", latest: {record}"))
        )
    }

    /// `WatchPainting`.
    pub fn watch(self: &Rc<Self>) {
        let seen = ns_string!("ArdiaWindowSeenDidChange");
        let mut observers = vec![self.observe(seen, "seen")];
        // SAFETY: AppKit's notification names are immutable static strings.
        for name in unsafe {
            [
                NSWindowDidBecomeKeyNotification,
                NSWindowDidResignKeyNotification,
            ]
        } {
            observers.push(self.observe(name, "key"));
        }
        let weak = Rc::downgrade(self);
        observers.push(observer(
            // SAFETY: As above.
            unsafe { NSWindowWillCloseNotification },
            move |window| {
                if let Some(probe) = weak.upgrade() {
                    probe.forget(window);
                }
            },
        ));
        self.observers.borrow_mut().extend(observers);
    }

    fn observe(
        self: &Rc<Self>,
        name: &NSNotificationName,
        trigger: &'static str,
    ) -> Retained<AnyObject> {
        let weak = Rc::downgrade(self);
        observer(name, move |window| {
            if let Some(probe) = weak.upgrade() {
                probe.schedule(window, trigger);
            }
        })
    }

    fn forget(&self, window: &NSWindow) {
        self.watches.borrow_mut().retain(|watch| {
            watch
                .window
                .load()
                .is_some_and(|kept| !std::ptr::eq(&*kept, window))
        });
    }

    fn schedule(self: &Rc<Self>, window: &NSWindow, trigger: &'static str) {
        if self.controllers.for_window(window).is_none() {
            return;
        }
        let generation = self.next_generation(window);
        let epoch = (self.consent_epoch)();
        let weak_window = Weak::from(window);
        let weak = Rc::downgrade(self);
        main_queue::after(
            PAINT_CHECK_AFTER,
            Box::new(move || {
                let (Some(probe), Some(window)) = (weak.upgrade(), weak_window.load()) else {
                    return;
                };
                if probe.generation(&window) == Some(generation) {
                    probe.check(&window, trigger, &epoch);
                }
            }),
        );
    }

    fn next_generation(&self, window: &NSWindow) -> u64 {
        let mut watches = self.watches.borrow_mut();
        watches.retain(|watch| watch.window.load().is_some());
        let index = match watches.iter().position(|watch| holds(watch, window)) {
            Some(index) => index,
            None => {
                watches.push(Watch {
                    window: Weak::from(window),
                    episode: Episode::default(),
                    generation: 0,
                });
                watches.len() - 1
            }
        };
        watches[index].generation += 1;
        watches[index].generation
    }

    fn generation(&self, window: &NSWindow) -> Option<u64> {
        self.watches
            .borrow()
            .iter()
            .find(|watch| holds(watch, window))
            .map(|watch| watch.generation)
    }

    fn check(&self, window: &NSWindow, trigger: &str, epoch: &str) {
        let Some(owner) = self.controllers.for_window(window) else {
            return;
        };
        if !std::ptr::eq(&*owner.core.window(), window) {
            return;
        }
        let mut tally = Tally::default();
        for view in (self.views)(window) {
            tally.add(reasons_of(&view));
        }
        let mut record = self.record(window, trigger, &tally);
        *self.last_check.borrow_mut() = Some(describe_paint(&record));
        let send = self
            .watches
            .borrow_mut()
            .iter_mut()
            .find(|watch| holds(watch, window))
            .is_some_and(|watch| watch.episode.should_send(&tally));
        if !send {
            return;
        }
        self.episodes.set(self.episodes.get() + 1);
        *self.last_record.borrow_mut() = self.last_check.borrow().clone();
        record["epoch"] = epoch.into();
        (self.emit)("engine.windowUnpainted", record);
    }

    fn record(&self, window: &NSWindow, trigger: &str, tally: &Tally) -> Value {
        let private = self
            .controllers
            .for_window(window)
            .and_then(|owner| owner.core.active_profile())
            .is_some_and(|profile| profile.off_the_record());
        let shows = window
            .occlusionState()
            .contains(NSWindowOcclusionState::Visible);
        let app = NSApplication::sharedApplication(MainThreadMarker::from(window));
        json!({
            "window": if private { "private" } else { "regular" },
            "trigger": trigger,
            "seen": self.controllers.user_sees(window),
            "occlusion": if shows { "visible" } else { "occluded" },
            "overruled": self.controllers.user_sees_only_through_window_server(window),
            "appActive": app.isActive(),
            "windows": self.open_window_count(),
            "views": tally.views,
            "unpainted": tally.unpainted,
            "reasons": tally.reasons,
        })
    }

    fn open_window_count(&self) -> usize {
        self.controllers
            .all()
            .iter()
            .filter(|controller| controller.core.window().isVisible())
            .count()
    }
}

impl Drop for PaintProbe {
    fn drop(&mut self) {
        let center = NSNotificationCenter::defaultCenter();
        for observer in self.observers.get_mut() {
            // SAFETY: These observer tokens came from this notification center.
            unsafe { center.removeObserver(observer) };
        }
    }
}

fn holds(watch: &Watch, window: &NSWindow) -> bool {
    watch
        .window
        .load()
        .is_some_and(|kept| std::ptr::eq(&*kept, window))
}

/// Calls `on` with the notification's object when it is a window.
fn observer(name: &NSNotificationName, on: impl Fn(&NSWindow) + 'static) -> Retained<AnyObject> {
    let block = RcBlock::new(move |note: NonNull<NSNotification>| {
        // SAFETY: AppKit lends the notification for the callback.
        let note = unsafe { note.as_ref() };
        if let Some(window) = note
            .object()
            .and_then(|object| object.downcast::<NSWindow>().ok())
        {
            on(&window);
        }
    });
    // SAFETY: The notifications come on the main thread, where the block runs.
    let token = unsafe {
        NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
            Some(name),
            None,
            None,
            &block,
        )
    };
    token.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::ardia_paint_probe_math::{FRAME_HOLD_AWAITING, VEIL_UP};

    #[test]
    fn names_reasons_in_bit_order() {
        assert_eq!(reason_names(0), "none");
        assert_eq!(
            reason_names(FRAME_HOLD_AWAITING | VEIL_UP),
            "frame hold, veil"
        );
    }
}
