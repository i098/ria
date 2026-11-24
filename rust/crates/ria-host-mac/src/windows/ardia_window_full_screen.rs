// Ported from ardia packages/ardiacore/ios/ArdiaWindowFullScreen.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::{Cell, LazyCell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2::rc::{Retained, Weak};
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSApplication, NSApplicationDidBecomeActiveNotification,
    NSApplicationDidResignActiveNotification, NSScreen, NSView, NSWindow,
    NSWindowDidChangeOcclusionStateNotification, NSWindowDidEnterFullScreenNotification,
    NSWindowDidExitFullScreenNotification, NSWindowOcclusionState, NSWindowStyleMask,
    NSWindowWillCloseNotification, NSWindowWillEnterFullScreenNotification,
    NSWindowWillExitFullScreenNotification,
};
use objc2_foundation::{
    NSArray, NSDictionary, NSMapTable, NSNotification, NSNotificationCenter, NSNumber, NSRect,
    NSString, ns_string,
};
use ria_engine::call::main_queue;

use super::ardia_window_sight_math::{Rect, counts_as_cover, partly_visible};
use crate::core::bindings::{Tab, Window};
use crate::launch::launch_config::LaunchConfig;
use crate::tabs::ardia_page_cover::PageCover;
use crate::tabs::ardia_page_draw_wait::after_page_draw;

type WindowInfo = NSDictionary<NSString, AnyObject>;

thread_local! {
    static ACTED_TRANSITION: Cell<f64> = const { Cell::new(0.) };
    pub(super) static ACTED_OCCLUSION_GAP: Cell<Option<f64>> = const { Cell::new(None) };
    static ACTED_OCCLUSION: LazyCell<Retained<NSMapTable<NSWindow, NSNumber>>> =
        LazyCell::new(NSMapTable::weakToStrongObjectsMapTable);
}

pub(crate) fn configure_acted_transition(config: &LaunchConfig) {
    ACTED_TRANSITION.set(transition_duration(
        config.test_value("fakeFullScreenMs").unwrap_or_default(),
    ));
}

pub(super) fn set_acted_transition(value: &str) -> f64 {
    let transition = transition_duration(value).max(0.);
    ACTED_TRANSITION.set(transition);
    transition
}

pub(super) fn transition_duration(value: &str) -> f64 {
    NSString::from_str(value).doubleValue() / 1000.
}

const CG_WINDOW_LIST_OPTION_ON_SCREEN_ABOVE_WINDOW: u32 = 1 << 1;
const CG_WINDOW_LIST_OPTION_INCLUDING_WINDOW: u32 = 1 << 3;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGWindowListCopyWindowInfo(options: u32, window: u32) -> *mut NSArray<WindowInfo>;
    fn CGRectMakeWithDictionaryRepresentation(dict: *const AnyObject, rect: *mut NSRect) -> bool;
    fn CGGetActiveDisplayList(max: u32, ids: *mut u32, count: *mut u32) -> i32;
    fn CGDisplayIsAsleep(id: u32) -> u32;
    fn CGDisplayBounds(id: u32) -> NSRect;
    fn CGSessionCopyCurrentDictionary() -> *mut WindowInfo;
}

fn number(info: &WindowInfo, key: &NSString) -> Option<Retained<NSNumber>> {
    info.objectForKey(key)?.downcast().ok()
}

fn sight_rect(rect: NSRect) -> Rect {
    Rect::new(
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
    )
}

pub(super) fn on_screen_window_bounds(window: &NSWindow) -> Option<NSRect> {
    let id = u32::try_from(window.windowNumber())
        .ok()
        .filter(|&id| id != 0)?;
    // SAFETY: Option 8 selects exactly this window. The Copy result is +1.
    let rows = unsafe { Retained::from_raw(CGWindowListCopyWindowInfo(8, id)) }?;
    let row = rows.firstObject()?;
    if !number(&row, ns_string!("kCGWindowIsOnscreen")).is_some_and(|value| value.boolValue()) {
        return None;
    }
    let bounds = row.objectForKey(ns_string!("kCGWindowBounds"))?;
    let mut rect = NSRect::ZERO;
    // SAFETY: kCGWindowBounds contains a CFDictionary; rect is writable.
    unsafe { CGRectMakeWithDictionaryRepresentation(&*bounds, &mut rect) }.then_some(rect)
}

fn window_server_shows(window: &NSWindow) -> bool {
    if !window.isVisible() || window.isMiniaturized() {
        return false;
    }
    let Ok(id) = u32::try_from(window.windowNumber()) else {
        return false;
    };
    // SAFETY: The flags select on-screen windows above this window, including this window.
    // The Copy result is +1 and the array and its dictionaries are toll-free bridged.
    let options =
        CG_WINDOW_LIST_OPTION_ON_SCREEN_ABOVE_WINDOW | CG_WINDOW_LIST_OPTION_INCLUDING_WINDOW;
    let Some(info) = (unsafe { Retained::from_raw(CGWindowListCopyWindowInfo(options, id)) })
    else {
        return false;
    };
    let mut mine = None;
    let mut covers = Vec::new();
    for row in info.iter() {
        let Some(bounds) = row.objectForKey(ns_string!("kCGWindowBounds")) else {
            continue;
        };
        let mut rect = NSRect::ZERO;
        // SAFETY: kCGWindowBounds is a CFDictionary; `rect` is a writable CGRect.
        if !unsafe { CGRectMakeWithDictionaryRepresentation(&*bounds, &mut rect) } {
            continue;
        }
        if number(&row, ns_string!("kCGWindowNumber")).is_some_and(|n| n.unsignedIntValue() == id) {
            if !number(&row, ns_string!("kCGWindowIsOnscreen")).is_some_and(|n| n.boolValue()) {
                return false;
            }
            mine = Some(sight_rect(rect));
            continue;
        }
        let layer = number(&row, ns_string!("kCGWindowLayer")).map_or(0, |n| n.intValue());
        let alpha = number(&row, ns_string!("kCGWindowAlpha")).map_or(0., |n| n.doubleValue());
        if counts_as_cover(layer, alpha) {
            covers.push(sight_rect(rect));
        }
    }
    let Some(mine) = mine else {
        return false;
    };
    let mut ids = [0; 32];
    let mut count = 0;
    // SAFETY: The array has 32 writable slots and count is writable.
    if unsafe { CGGetActiveDisplayList(32, ids.as_mut_ptr(), &mut count) } != 0 {
        return false;
    }
    let mut displays = Vec::with_capacity(count as usize);
    for &id in &ids[..(count as usize).min(ids.len())] {
        // SAFETY: The successful display query populated this ID.
        if unsafe { CGDisplayIsAsleep(id) } == 0 {
            displays.push(sight_rect(unsafe { CGDisplayBounds(id) }));
        }
    }
    partly_visible(mine, &covers, &displays)
}

fn screen_locked() -> bool {
    // SAFETY: The Copy rule returns a +1 dictionary, or null outside a console session.
    unsafe { Retained::from_raw(CGSessionCopyCurrentDictionary()) }.is_some_and(|info| {
        number(&info, ns_string!("CGSSessionScreenIsLocked")).is_some_and(|value| value.boolValue())
    })
}

fn window_is_full_screen(core: &Window, window: &NSWindow) -> bool {
    core.acted_full_screen() || window.styleMask().contains(NSWindowStyleMask::FullScreen)
}

fn acted_occlusion(window: &NSWindow) -> Option<bool> {
    ACTED_OCCLUSION.with(|overrides| {
        overrides
            .objectForKey(Some(window))
            .map(|value| value.boolValue())
    })
}

fn set_acted_occlusion(window: &NSWindow, visible: Option<bool>) {
    ACTED_OCCLUSION.with(|overrides| match visible {
        Some(visible) => {
            overrides.setObject_forKey(Some(&NSNumber::numberWithBool(visible)), Some(window));
        }
        None => overrides.removeObjectForKey(Some(window)),
    });
    unsafe {
        NSNotificationCenter::defaultCenter().postNotificationName_object(
            NSWindowDidChangeOcclusionStateNotification,
            Some(window),
        );
    }
}

pub(super) fn act_occlusion(
    window: &NSWindow,
    fullscreen: Option<&FullScreen>,
    visible: Option<bool>,
) -> bool {
    if let Some(fullscreen) = fullscreen {
        let mut state = fullscreen.state.borrow_mut();
        state.occlusion_turn = state.occlusion_turn.wrapping_add(1);
    }
    set_acted_occlusion(window, visible);
    acted_occlusion(window)
        .unwrap_or_else(|| window.occlusionState().contains(NSWindowOcclusionState::Visible))
}

#[derive(Default)]
struct Transition {
    page: PageState,
    expose_page: bool,
    acting: bool,
    restore_frame: Option<NSRect>,
    acted_server: bool,
    occlusion_turn: u64,
    closed: bool,
}

#[derive(Default)]
struct PageState {
    tab: Option<Weak<Tab>>,
    left_tab: Option<Weak<Tab>>,
    entered: bool,
    asked: i8,
    waiting_for: i8,
    waiting_for_draw: bool,
    telling: usize,
    picturing: bool,
    covers: Vec<Retained<PageCover>>,
    pending_covers: Vec<Retained<PageCover>>,
    turn: u64,
    transitioning: bool,
    ours: bool,
    settling: bool,
    seen: bool,
    overruled: bool,
}

impl PageState {
    fn cancel_draw(&mut self) {
        self.turn = self.turn.wrapping_add(1);
        self.waiting_for = 0;
        self.waiting_for_draw = false;
        self.picturing = false;
        uncover(std::mem::take(&mut self.pending_covers), false);
    }
}

fn uncover(covers: Vec<Retained<PageCover>>, after_draw: bool) {
    for cover in covers {
        if after_draw && let Some(tab) = cover.owner() {
            after_page_draw(&tab, |_, _| true, move || {
                main_queue::after(
                    Duration::from_millis(34),
                    Box::new(move || cover.uncover(true)),
                );
            });
        } else {
            cover.uncover(false);
        }
    }
}

fn has_page_cover(view: Retained<NSView>) -> bool {
    match view.downcast::<PageCover>() {
        Ok(_) => true,
        Err(view) => view.subviews().iter().any(has_page_cover),
    }
}

pub struct FullScreen {
    core: Weak<Window>,
    mtm: MainThreadMarker,
    background: bool,
    allow_occlusion: bool,
    state: RefCell<Transition>,
    observers: Vec<Retained<AnyObject>>,
}

impl FullScreen {
    pub(super) fn new(
        core: &Window,
        mtm: MainThreadMarker,
        background: bool,
        allow_occlusion: bool,
    ) -> Rc<Self> {
        let window = core.window();
        // SAFETY: AppKit posts these notifications on the main thread and each callback holds a weak Rust reference.
        let names = unsafe {
            [
                (NSWindowWillEnterFullScreenNotification, true),
                (NSWindowWillExitFullScreenNotification, true),
                (NSWindowDidEnterFullScreenNotification, true),
                (NSWindowDidExitFullScreenNotification, true),
                (NSWindowDidChangeOcclusionStateNotification, true),
                (NSWindowWillCloseNotification, true),
                (NSApplicationDidResignActiveNotification, false),
                (NSApplicationDidBecomeActiveNotification, false),
            ]
        };
        Rc::new_cyclic(|weak: &std::rc::Weak<Self>| {
            let center = NSNotificationCenter::defaultCenter();
            let observers = names
                .into_iter()
                .map(|(name, local)| {
                    let weak = weak.clone();
                    let handler = RcBlock::new(move |_: std::ptr::NonNull<NSNotification>| {
                        if let Some(fullscreen) = weak.upgrade() {
                            fullscreen.notified(name);
                        }
                    });
                    // SAFETY: The object filter is a live NSWindow and the block has the notification ABI.
                    unsafe {
                        center.addObserverForName_object_queue_usingBlock(
                            Some(name),
                            if local { Some(&window) } else { None },
                            None,
                            &handler,
                        )
                    }
                })
                .map(Retained::into)
                .collect();
            Self {
                core: Weak::from(core),
                mtm,
                background,
                allow_occlusion,
                state: RefCell::new(Transition {
                    page: PageState {
                        seen: true,
                        ..PageState::default()
                    },
                    ..Transition::default()
                }),
                observers,
            }
        })
    }

    fn live_window(&self) -> Option<(Retained<Window>, Retained<NSWindow>)> {
        if self.state.borrow().closed {
            return None;
        }
        let core = self.core.load()?;
        let window = core.window();
        Some((core, window))
    }

    pub fn is_full_screen(&self) -> bool {
        self.live_window()
            .is_some_and(|(core, window)| window_is_full_screen(&core, &window))
    }

    pub fn seen(&self) -> bool {
        let mut state = self.state.borrow_mut();
        state.expose_page = true;
        state.page.seen
    }
    pub fn seen_only_by_window_server(&self) -> bool {
        let mut state = self.state.borrow_mut();
        state.expose_page = true;
        state.page.seen && state.page.overruled
    }
    pub fn transitioning(&self) -> bool {
        self.state.borrow().page.transitioning
    }

    pub(super) fn page_state(&self, full_screen: bool) -> Option<serde_json::Value> {
        let state = self.state.borrow();
        if !state.expose_page {
            return None;
        }
        let page = &state.page;
        Some(serde_json::json!({
            "page": page.tab.as_ref().and_then(Weak::load).map(|tab| tab.browser_id()),
            "entered": page.entered,
            "asked": page.asked,
            "waitingForPage": page.waiting_for,
            "telling": page.telling,
            "picturing": page.picturing,
            "covered": self.core.load().and_then(|core| core.window().contentView())
                .is_some_and(has_page_cover),
            "transitioning": page.transitioning,
            "ours": page.ours,
            "settling": page.settling,
            "seen": page.seen,
            "fullScreen": full_screen,
        }))
    }

    pub(super) fn page_changed(self: &Rc<Self>, tab: &Tab, entering: bool) {
        let mut state = self.state.borrow_mut();
        if state.closed {
            return;
        }
        state.expose_page = true;
        let page = &mut state.page;
        page.cancel_draw();
        if entering {
            page.tab = Some(Weak::new(tab));
        } else if page
            .tab
            .as_ref()
            .and_then(Weak::load)
            .is_none_or(|current| std::ptr::eq(&*current, tab))
        {
            page.left_tab = page.tab.take();
        }
        page.telling += 1;
        drop(state);
        self.sync_page();
        let weak = Rc::downgrade(self);
        let tab = Weak::new(tab);
        main_queue::post(Box::new(move || {
            if weak
                .upgrade()
                .is_none_or(|item| item.live_window().is_none())
            {
                return;
            }
            let done = move || {
                if let Some(fullscreen) = weak.upgrade() {
                    fullscreen.state.borrow_mut().page.telling -= 1;
                    fullscreen.sync_page();
                }
            };
            if let Some(tab) = tab.load() {
                after_page_draw(&tab, move |drawn, _| !entering || drawn, done);
            } else {
                done();
            }
        }));
    }

    fn sync_page(self: &Rc<Self>) {
        let Some((core, window)) = self.live_window() else {
            return;
        };
        let full = window_is_full_screen(&core, &window);
        let (tab, entering, turn) = {
            let mut state = self.state.borrow_mut();
            if state.acting {
                return;
            }
            let page = &mut state.page;
            if page.transitioning {
                return;
            }
            let tab = page.tab.as_ref().and_then(Weak::load);
            let entering = tab.is_some() && !full && !page.entered;
            let exiting = tab.is_none() && full && page.entered;
            if !entering && !exiting {
                if page.waiting_for != 0 || page.picturing {
                    page.cancel_draw();
                }
                if tab.is_none() && page.telling == 0 {
                    page.entered = false;
                }
                return;
            }
            let target = if entering { 1 } else { -1 };
            if page.waiting_for != 0 && page.waiting_for != target {
                page.cancel_draw();
            }
            page.waiting_for = target;
            if page.telling != 0 || page.picturing || page.waiting_for_draw {
                return;
            }
            page.waiting_for_draw = true;
            let tab = tab.or_else(|| page.left_tab.as_ref().and_then(Weak::load));
            (tab, entering, page.turn)
        };
        let weak = Rc::downgrade(self);
        let tab_for_cover = tab.as_deref().map(Weak::new);
        let drawn = move || {
            if let Some(fullscreen) = weak.upgrade() {
                fullscreen.picture_page(tab_for_cover, entering, turn);
            }
        };
        if let Some(tab) = tab {
            after_page_draw(
                &tab,
                move |drawn, fills| drawn == entering && fills == entering,
                drawn,
            );
        } else {
            drawn();
        }
    }

    fn picture_page(self: &Rc<Self>, tab: Option<Weak<Tab>>, entering: bool, turn: u64) {
        let Some((core, window)) = self.live_window() else {
            return;
        };
        {
            let mut state = self.state.borrow_mut();
            let page = &mut state.page;
            if page.turn != turn {
                return;
            }
            page.waiting_for = 0;
            page.waiting_for_draw = false;
            page.picturing = true;
        }
        let mut tabs: Vec<_> = tab.and_then(|tab| tab.load()).into_iter().collect();
        if !entering && let Some(profile) = core.active_profile() {
            for other in core.tabs_for_profile(&profile).iter() {
                let view = other.view();
                if !tabs.iter().any(|tab| std::ptr::eq(&**tab, &*other))
                    && view
                        .window()
                        .is_some_and(|owner| std::ptr::eq(&*owner, &*window))
                    && !view.isHiddenOrHasHiddenAncestor()
                    && view.alphaValue() >= 0.5
                {
                    tabs.push(other);
                }
            }
        }
        if tabs.is_empty() {
            self.finish_picture(entering, turn);
            return;
        }
        let remaining = Rc::new(Cell::new(tabs.len()));
        let weak = Rc::downgrade(self);
        for tab in tabs {
            let weak = weak.clone();
            let remaining = remaining.clone();
            let current = move || {
                weak.upgrade().filter(|fullscreen| {
                    let state = fullscreen.state.borrow();
                    !state.closed && state.page.turn == turn
                })
            };
            let attach = current.clone();
            PageCover::cover(
                &tab,
                false,
                move || attach().is_some(),
                move |cover| {
                    let Some(fullscreen) = current() else {
                        if let Some(cover) = cover {
                            cover.uncover(false);
                        }
                        return;
                    };
                    if let Some(cover) = cover {
                        fullscreen.state.borrow_mut().page.pending_covers.push(cover);
                    }
                    remaining.set(remaining.get() - 1);
                    if remaining.get() == 0 {
                        fullscreen.finish_picture(entering, turn);
                    }
                },
            );
        }
    }

    fn finish_picture(self: &Rc<Self>, entering: bool, turn: u64) {
        let Some((core, window)) = self.live_window() else {
            return;
        };
        let (toggle, old) = {
            let mut state = self.state.borrow_mut();
            let acting = state.acting;
            let page = &mut state.page;
            if page.turn != turn {
                return;
            }
            page.picturing = false;
            let covers = std::mem::take(&mut page.pending_covers);
            let toggle = !acting
                && !page.transitioning
                && window_is_full_screen(&core, &window) != entering
                && page.tab.as_ref().and_then(Weak::load).is_some() == entering;
            if toggle {
                page.entered = entering;
                page.asked = if entering { 1 } else { -1 };
                if !entering {
                    page.left_tab = None;
                }
                (true, std::mem::replace(&mut page.covers, covers))
            } else {
                (false, covers)
            }
        };
        uncover(old, false);
        if toggle {
            self.toggle();
        } else {
            self.sync_page();
        }
    }

    fn notified(self: &Rc<Self>, name: &NSString) {
        let (close, will, did, entering, left_app) = unsafe {
            let will_enter = name.isEqualToString(NSWindowWillEnterFullScreenNotification);
            let did_enter = name.isEqualToString(NSWindowDidEnterFullScreenNotification);
            (
                name.isEqualToString(NSWindowWillCloseNotification),
                will_enter || name.isEqualToString(NSWindowWillExitFullScreenNotification),
                did_enter || name.isEqualToString(NSWindowDidExitFullScreenNotification),
                will_enter || did_enter,
                name.isEqualToString(NSApplicationDidResignActiveNotification),
            )
        };
        let covers = {
            let mut state = self.state.borrow_mut();
            if state.closed {
                return;
            }
            state.closed = close;
            let page = &mut state.page;
            if close || will {
                page.cancel_draw();
            }
            if will {
                page.transitioning = true;
                page.settling = true;
                page.ours = page.asked == if entering { 1 } else { -1 };
                page.asked = 0;
                for cover in &page.covers {
                    cover.repicture();
                }
            }
            if did {
                page.transitioning = false;
            }
            if close || did {
                std::mem::take(&mut page.covers)
            } else {
                Vec::new()
            }
        };
        uncover(covers, did);
        if close {
            return;
        }
        self.update_seen(left_app);
        if did {
            let ours = std::mem::take(&mut self.state.borrow_mut().page.ours);
            let weak = Rc::downgrade(self);
            main_queue::post(Box::new(move || {
                if let Some(fullscreen) = weak.upgrade().filter(|item| item.live_window().is_some())
                {
                    let exit_page = if !entering && !ours {
                        let page = &mut fullscreen.state.borrow_mut().page;
                        page.entered = false;
                        page.tab.as_ref().and_then(Weak::load)
                    } else {
                        None
                    };
                    if let Some(tab) = exit_page {
                        tab.exit_fullscreen();
                    } else {
                        fullscreen.sync_page();
                    }
                }
            }));
        }
    }

    fn update_seen(&self, left_app: bool) {
        let Some((core, window)) = self.live_window() else {
            return;
        };
        let app = NSApplication::sharedApplication(self.mtm);
        let active = app.isActive();
        let acted_server = self.state.borrow().acted_server;
        let (visible, overruled) = if let Some(app_kit) = acted_occlusion(&window) {
            let server = (!active || !app_kit)
                && window.isVisible()
                && !window.isMiniaturized()
                && acted_server;
            (
                if active { app_kit || server } else { server },
                !app_kit && server,
            )
        } else if self.background && !self.allow_occlusion {
            (true, false)
        } else {
            if screen_locked() {
                return;
            }
            let app_kit = window
                .occlusionState()
                .contains(NSWindowOcclusionState::Visible);
            let server = (!active || !app_kit) && window_server_shows(&window);
            (
                if active { app_kit || server } else { server },
                !app_kit && server,
            )
        };
        let active_fullscreen = active && window_is_full_screen(&core, &window);
        let changed = {
            let state = &mut self.state.borrow_mut().page;
            if state.settling && !state.transitioning && visible {
                state.settling = false;
            }
            let previous = (state.seen, state.overruled);
            state.seen = state.settling || visible || active_fullscreen;
            state.overruled = overruled;
            previous != (state.seen, state.overruled)
        };
        if changed || left_app {
            let info = left_app.then(|| {
                let info = NSDictionary::<NSString, AnyObject>::from_slices(
                    &[ns_string!("leftApp")],
                    &[&*NSNumber::numberWithBool(true)],
                );
                // SAFETY: Erase the dictionary's key type for AppKit's userInfo argument.
                unsafe { Retained::cast_unchecked::<NSDictionary>(info) }
            });
            // SAFETY: This notification passes only the live window and an immutable bool dictionary.
            unsafe {
                NSNotificationCenter::defaultCenter().postNotificationName_object_userInfo(
                    ns_string!("ArdiaWindowSeenDidChange"),
                    Some(&window),
                    info.as_deref(),
                )
            };
        }
    }

    pub fn reconsider_sight(&self) {
        self.state.borrow_mut().expose_page = true;
        self.update_seen(false);
    }

    pub(super) fn act_window_server(&self, shows: bool) {
        self.state.borrow_mut().acted_server = shows;
    }

    pub fn toggle(self: &Rc<Self>) {
        if !self.background {
            if let Some((_, window)) = self.live_window() {
                window.toggleFullScreen(None);
            }
            return;
        }
        self.act_window_full_screen_with_timing(!self.is_full_screen(), ACTED_TRANSITION.get());
    }

    pub fn act_window_full_screen(self: &Rc<Self>, entering: bool) {
        self.act_window_full_screen_with_timing(entering, 0.);
    }

    pub(super) fn act_window_full_screen_with_timing(
        self: &Rc<Self>,
        entering: bool,
        transition: f64,
    ) {
        let Some((core, window)) = self.live_window() else {
            return;
        };
        let turn = {
            let mut state = self.state.borrow_mut();
            if state.acting || window_is_full_screen(&core, &window) == entering {
                return;
            }
            state.acting = true;
            state.occlusion_turn
        };
        let center = NSNotificationCenter::defaultCenter();
        // SAFETY: These AppKit notification names and the window are valid for this call.
        unsafe {
            center.postNotificationName_object(
                if entering {
                    NSWindowWillEnterFullScreenNotification
                } else {
                    NSWindowWillExitFullScreenNotification
                },
                Some(&window),
            );
        }
        if self.state.borrow().closed {
            return;
        }
        if entering {
            self.state.borrow_mut().restore_frame = Some(window.frame());
            core.set_acted_full_screen(true);
            if let Some(screen) = window.screen().or_else(|| NSScreen::mainScreen(self.mtm)) {
                window.setFrame_display(screen.frame(), true);
            }
        } else {
            let restore = self.state.borrow_mut().restore_frame.take();
            if let Some(frame) = restore {
                window.setFrame_display(frame, true);
            }
        }
        let current = self.state.borrow().occlusion_turn == turn;
        if current
            && let Some(wait) = ACTED_OCCLUSION_GAP.get().and_then(|gap| {
                Duration::try_from_secs_f64((transition + gap).max(0.)).ok()
            })
        {
            let turn = turn.wrapping_add(1);
            self.state.borrow_mut().occlusion_turn = turn;
            set_acted_occlusion(&window, Some(false));
            let weak = Rc::downgrade(self);
            main_queue::after(
                wait,
                Box::new(move || {
                    if let Some(fullscreen) = weak.upgrade() {
                        let current = fullscreen.state.borrow().occlusion_turn == turn;
                        if current && let Some((_, window)) = fullscreen.live_window() {
                            set_acted_occlusion(&window, Some(true));
                        }
                    }
                }),
            );
        }
        let transition = Duration::try_from_secs_f64(transition).unwrap_or_default();
        if transition.is_zero() {
            self.finish_acted_transition(entering);
        } else {
            let weak = Rc::downgrade(self);
            main_queue::after(
                transition,
                Box::new(move || {
                    if let Some(fullscreen) = weak.upgrade() {
                        fullscreen.finish_acted_transition(entering);
                    }
                }),
            );
        }
    }

    fn finish_acted_transition(&self, entering: bool) {
        let Some((core, window)) = self.live_window() else {
            return;
        };
        {
            let mut state = self.state.borrow_mut();
            if state.closed {
                return;
            }
            state.acting = false;
        }
        if !entering {
            core.set_acted_full_screen(false);
        }
        unsafe {
            NSNotificationCenter::defaultCenter().postNotificationName_object(
                if entering {
                    NSWindowDidEnterFullScreenNotification
                } else {
                    NSWindowDidExitFullScreenNotification
                },
                Some(&window),
            );
        }
    }
}

impl Drop for FullScreen {
    fn drop(&mut self) {
        let center = NSNotificationCenter::defaultCenter();
        for observer in &self.observers {
            // SAFETY: Each object is the token returned by the same notification center.
            unsafe {
                center.removeObserver(observer);
            }
        }
    }
}
