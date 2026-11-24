// Ported from ardia packages/ardiacore/ios/ArdiaHoverProbe.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Also ports ArdiaHoverProbe+Dev.mm (in `dev`). Changes from Ardia: what Ardia reads from
//! `ArdiaLaunchConfig`, `ArdiaWindowController`, the content blocker, `NoteFocusChange` and `ArdiaEvents` comes
//! through a `HoverHost` that `watch_hover` installs. The consent stamp is the epoch string at the start of a
//! check.

mod dev;

use std::cell::{Cell, RefCell};
use std::ffi::CStr;
use std::ptr::NonNull;
use std::rc::Rc;
use std::time::Duration;

use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyClass, AnyObject, Sel};
use objc2::{ClassType, MainThreadMarker, Message, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSApplicationDidResignActiveNotification, NSEvent, NSEventMask,
    NSEventModifierFlags, NSEventSubtype, NSEventType, NSNormalWindowLevel, NSPanel,
    NSPopUpMenuWindowLevel, NSRunningApplication, NSSavePanel, NSView, NSWindow,
    NSWindowDidEnterFullScreenNotification, NSWindowDidExitFullScreenNotification,
    NSWindowWillCloseNotification, NSWindowWillEnterFullScreenNotification,
    NSWindowWillExitFullScreenNotification, NSWorkspace,
    NSWorkspaceDidActivateApplicationNotification,
};
use objc2_foundation::{
    NSBundle, NSDate, NSDateFormatter, NSNotification, NSNotificationCenter, NSNotificationName,
    NSNumber, NSObjectNSKeyValueCoding, NSObjectProtocol, NSPoint, NSProcessInfo, NSRect, NSString,
    ns_string,
};
use ria_engine::call::main_queue;
use serde_json::{Value, json};

pub use self::dev::hover_probe_dev;
use super::ardia_hover_probe_internal::{
    Activation, FocusHolder, FocusWindowKind, HoverCheck, HoverEpisode, HoverReason, HoverTrigger,
    PageCover,
};

/// The role of a window's `ArdiaWindowController`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowRole {
    /// Standalone, stray or an internal core window.
    Internal,
    /// A browser window with a root.
    Browser,
    /// A controller without a root.
    Panel,
}

/// What the probe reads from the rest of the host.
pub trait HoverHost {
    /// `ArdiaLaunchConfig.shared.background`.
    fn background(&self) -> bool;
    /// `ardia_content_blocker::IsHiddenPageWindow`.
    fn is_hidden_page_window(&self, window: &NSWindow) -> bool;
    /// The role of `+[ArdiaWindowController forNSWindow:]`, or `None` when the window has no controller.
    fn window_role(&self, window: &NSWindow) -> Option<WindowRole>;
    /// `-[ArdiaWindowController root]`.
    fn root(&self, window: &NSWindow) -> Option<Retained<NSView>>;
    /// `NoteFocusChange`.
    fn note_focus_change(&self, change: &str, window: Option<&NSWindow>);
    /// `DescribeFocusWindow`.
    fn describe_focus_window(&self, window: Option<&NSWindow>) -> String;
    /// `ArdiaEvents.consentEpoch`.
    fn consent_epoch(&self) -> String;
    /// `+[ArdiaEvents emit:data:]`.
    fn emit(&self, name: &str, data: Value);
}

const FULL_SCREEN_MOVE_LONGEST: f64 = 5.0;
const MOVE_CHECK_EVERY: f64 = 2.0;
const HEAL_RECHECK_AFTER: Duration = Duration::from_millis(300);
const DEV_HOVER_EVENTS_KEPT: usize = 16;

thread_local! {
    static HOST: RefCell<Option<Rc<dyn HoverHost>>> = const { RefCell::new(None) };
    static DEV_HALF_ACTIVE: Cell<bool> = const { Cell::new(false) };
    static DEV_MENU_ACTIVE: Cell<bool> = const { Cell::new(false) };
    static EPISODE: RefCell<HoverEpisode> = RefCell::default();
    static PENDING_REASON: Cell<HoverReason> = const { Cell::new(HoverReason::Fine) };
    static PENDING_WINDOW: RefCell<Option<Weak<NSWindow>>> = const { RefCell::new(None) };
    static EPISODE_COUNT: Cell<usize> = const { Cell::new(0) };
    static LAST_VERDICT: RefCell<Option<String>> = const { RefCell::new(None) };
    static LAST_MOVE_CHECK: Cell<f64> = const { Cell::new(0.0) };
    static MONITOR: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
    static WATCH_IN_BACKGROUND: Cell<bool> = const { Cell::new(false) };
    static DEV_EVENTS: RefCell<Vec<Value>> = const { RefCell::new(Vec::new()) };
    static INVISIBLE_COVER: RefCell<Option<Weak<NSWindow>>> = const { RefCell::new(None) };
    static FULL_SCREEN_MOVES: RefCell<Vec<(Weak<NSWindow>, f64)>> = const { RefCell::new(Vec::new()) };
    static OBSERVERS: RefCell<Vec<Retained<AnyObject>>> = const { RefCell::new(Vec::new()) };
    static CLOCK: Retained<NSDateFormatter> = {
        let clock = NSDateFormatter::new();
        clock.setDateFormat(Some(ns_string!("HH:mm:ss")));
        clock
    };
}

fn host() -> Rc<dyn HoverHost> {
    HOST.with_borrow(|host| host.clone())
        .expect("watch_hover installs the hover host first")
}

fn mtm() -> MainThreadMarker {
    MainThreadMarker::new().expect("the hover probe runs on the main thread")
}

fn app() -> Retained<NSApplication> {
    NSApplication::sharedApplication(mtm())
}

fn uptime() -> f64 {
    NSProcessInfo::processInfo().systemUptime()
}

fn same(a: &NSWindow, b: Option<&Weak<NSWindow>>) -> bool {
    b.and_then(Weak::load).is_some_and(|b| std::ptr::eq(a, &*b))
}

fn class_name_has_menu(window: &NSWindow) -> bool {
    window
        .class()
        .name()
        .to_bytes()
        .windows(4)
        .any(|part| part == b"Menu")
}

fn point_in_rect(point: NSPoint, rect: NSRect) -> bool {
    point.x >= rect.origin.x
        && point.x < rect.origin.x + rect.size.width
        && point.y >= rect.origin.y
        && point.y < rect.origin.y + rect.size.height
}

fn superview(view: &NSView) -> Option<Retained<NSView>> {
    // SAFETY: The view hierarchy is read on the main thread.
    unsafe { view.superview() }
}

fn current_activation() -> Activation {
    let app_active = app().isActive();
    let mut activation = Activation {
        app_active,
        ..Activation::default()
    };
    if DEV_HALF_ACTIVE.get() {
        activation.running_active = true;
        activation.frontmost = true;
    } else if host().background() {
        activation.running_active = app_active;
        activation.frontmost = app_active;
    } else {
        let me = NSRunningApplication::currentApplication();
        activation.running_active = me.isActive();
        activation.frontmost = is_me(&me);
    }
    activation
}

fn is_me(me: &NSRunningApplication) -> bool {
    NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .is_some_and(|front| front.processIdentifier() == me.processIdentifier())
}

fn window_flag(window: &NSWindow, key: &CStr) -> bool {
    if !window.respondsToSelector(Sel::register(key)) {
        return false;
    }
    let key = NSString::from_str(key.to_str().unwrap_or_default());
    let value = window.valueForKey(&key);
    value.is_some_and(|value| {
        // SAFETY: KVC boxes a `BOOL` property in an `NSNumber`.
        let flag: bool = unsafe { msg_send![&*value, boolValue] };
        flag
    })
}

fn prevents_key(window: &NSWindow) -> bool {
    window_flag(window, c"preventKeyWindow")
}

fn menu_active(window: &NSWindow) -> bool {
    DEV_MENU_ACTIVE.get() || window_flag(window, c"hasViewsMenuActive")
}

fn menu_shown(browser: &NSWindow) -> bool {
    app().windows().iter().any(|window| {
        if std::ptr::eq(&*window, browser) || !window.isVisible() {
            return false;
        }
        window
            .parentWindow()
            .is_some_and(|parent| std::ptr::eq(&*parent, browser))
            || window.level() >= NSPopUpMenuWindowLevel
            || class_name_has_menu(&window)
    })
}

fn is_browser_window(window: Option<&NSWindow>) -> bool {
    let Some(window) = window else {
        return false;
    };
    let host = host();
    !host.is_hidden_page_window(window) && host.window_role(window) == Some(WindowRole::Browser)
}

fn invisible(window: &NSWindow) -> bool {
    !window.isVisible()
        || window.alphaValue() < 0.01
        || window.isMiniaturized()
        || !window.isOnActiveSpace()
}

fn kind_of(window: Option<&NSWindow>, even_invisible: bool) -> FocusWindowKind {
    let Some(window) = window else {
        return FocusWindowKind::None;
    };
    let host = host();
    if host.is_hidden_page_window(window) {
        return FocusWindowKind::BlockerPage;
    }
    if !even_invisible && invisible(window) {
        return FocusWindowKind::Hidden;
    }
    match host.window_role(window) {
        Some(WindowRole::Internal) => FocusWindowKind::Internal,
        Some(WindowRole::Browser) => FocusWindowKind::Browser,
        Some(WindowRole::Panel) => FocusWindowKind::ArdiaPanel,
        None if looks_like_popup(window) => FocusWindowKind::Popup,
        None => FocusWindowKind::Other,
    }
}

fn looks_like_popup(window: &NSWindow) -> bool {
    window.isKindOfClass(NSPanel::class())
        || window.isSheet()
        || window.parentWindow().is_some()
        || window.level() > NSNormalWindowLevel
        || class_name_has_menu(window)
}

fn unseen(kind: FocusWindowKind) -> bool {
    matches!(
        kind,
        FocusWindowKind::None | FocusWindowKind::Hidden | FocusWindowKind::BlockerPage
    )
}

fn page_class() -> Option<&'static AnyClass> {
    AnyClass::get(c"RenderWidgetHostViewCocoa")
}

fn page_at(window: &NSWindow, point: NSPoint) -> Option<Retained<NSView>> {
    let page_class = page_class()?;
    let mut found = None;
    let mut stack = vec![window.contentView().as_deref().and_then(superview)?];
    while let Some(view) = stack.pop() {
        if view.isHidden() {
            continue;
        }
        if view.isKindOfClass(page_class) {
            if point_in_rect(point, view.convertRect_toView(view.bounds(), None)) {
                found = Some(view);
            }
            continue;
        }
        stack.extend(view.subviews().to_vec().into_iter().rev());
    }
    found
}

fn enclosing_page(view: Option<&NSView>) -> Option<Retained<NSView>> {
    let page_class = page_class()?;
    let mut view = view.map(|view| view.retain());
    while let Some(v) = view {
        if v.isKindOfClass(page_class) {
            return Some(v);
        }
        view = superview(&v);
    }
    None
}

fn page_ignores_move(page: &NSView, window: &NSWindow, point: NSPoint) -> bool {
    if !page.respondsToSelector(sel!(shouldIgnoreMouseEvent:)) {
        return false;
    }
    let Some(event) = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
        NSEventType::MouseMoved,
        point,
        NSEventModifierFlags::empty(),
        uptime(),
        window.windowNumber(),
        None,
        0,
        0,
        0.0,
    ) else {
        return false;
    };
    // SAFETY: The page answers `shouldIgnoreMouseEvent:`, which takes an event and returns `BOOL`.
    unsafe { msg_send![page, shouldIgnoreMouseEvent: &*event] }
}

fn hit_at(window: &NSWindow, point: NSPoint) -> Option<Retained<NSView>> {
    let content = window.contentView()?;
    let local = superview(&content).map_or(NSPoint::ZERO, |frame| {
        frame.convertPoint_fromView(point, None)
    });
    content.hitTest(local)
}

fn cover_of(hit: Option<&NSView>) -> PageCover {
    let Some(hit) = hit else {
        return PageCover::None;
    };
    // SAFETY: `bundleForClass:` reads the class's image.
    let chromium = page_class().map(|class| unsafe { NSBundle::bundleForClass(class) });
    let main = NSBundle::mainBundle();
    let mut view = Some(hit.retain());
    while let Some(v) = view {
        // SAFETY: As above.
        let bundle = unsafe { NSBundle::bundleForClass(v.class()) };
        if chromium
            .as_deref()
            .is_some_and(|chromium| std::ptr::eq(chromium, &*bundle))
        {
            return PageCover::Chromium;
        }
        if std::ptr::eq(&*main, &*bundle) {
            return PageCover::ReactNative;
        }
        view = superview(&v);
    }
    PageCover::Other
}

fn see_through(hit: Option<&NSView>) -> bool {
    let mut view = hit.map(|hit| hit.retain());
    while let Some(v) = view {
        // SAFETY: `layer` returns the view's backing layer or nil.
        let layer: Option<Retained<AnyObject>> = unsafe { msg_send![&*v, layer] };
        let clear_layer = layer.is_some_and(|layer| {
            // SAFETY: A layer's `opacity` is a `float`.
            let opacity: f32 = unsafe { msg_send![&*layer, opacity] };
            opacity < 0.01
        });
        if v.alphaValue() < 0.01 || clear_layer {
            return true;
        }
        view = superview(&v);
    }
    false
}

fn holder_of(window: &NSWindow) -> FocusHolder {
    let Some(responder) = window.firstResponder() else {
        return FocusHolder::None;
    };
    if std::ptr::eq(
        Retained::as_ptr(&responder).cast::<AnyObject>(),
        (window as *const NSWindow).cast(),
    ) {
        return FocusHolder::Window;
    }
    let Ok(view) = responder.downcast::<NSView>() else {
        return FocusHolder::Control;
    };
    if !view.window().is_some_and(|own| std::ptr::eq(&*own, window)) {
        return FocusHolder::OtherWindow;
    }
    if enclosing_page(Some(&view)).is_some() {
        FocusHolder::Page
    } else {
        FocusHolder::Control
    }
}

fn responder_lost(holder: FocusHolder, clicked_page: bool) -> bool {
    if matches!(holder, FocusHolder::None | FocusHolder::OtherWindow) {
        return true;
    }
    clicked_page && holder == FocusHolder::Window
}

fn page_reason(
    check: &mut HoverCheck,
    window: &NSWindow,
    point: NSPoint,
    page: Option<&NSView>,
    click: bool,
) -> HoverReason {
    let active = check.window_key || check.window_main;
    let a = check.activation;
    if (a.frontmost || a.running_active) && !a.app_active {
        return HoverReason::HalfActive;
    }
    if click && page.is_some() && check.menu_active && !menu_shown(window) {
        return HoverReason::StaleMenu;
    }
    if let Some(page) = page
        && page_ignores_move(page, window, point)
    {
        if !active {
            return HoverReason::NotKeyOrMain;
        }
        let hit = hit_at(window, point);
        if enclosing_page(hit.as_deref()).is_some() {
            return HoverReason::Fine;
        }
        check.cover = cover_of(hit.as_deref());
        check.cover_clear = see_through(hit.as_deref());
        return HoverReason::CoveredByView;
    }
    if check.window_key && responder_lost(check.holder, click && page.is_some()) {
        return HoverReason::ResponderLost;
    }
    HoverReason::Fine
}

fn probe(window: &NSWindow, point: NSPoint, click: bool) -> HoverCheck {
    let app = app();
    let mut check = HoverCheck {
        activation: current_activation(),
        key: kind_of(app.keyWindow().as_deref(), false),
        main: kind_of(app.mainWindow().as_deref(), false),
        prevent_key: prevents_key(window),
        can_key: window.canBecomeKeyWindow(),
        menu_active: menu_active(window),
        window_key: window.isKeyWindow(),
        window_main: window.isMainWindow(),
        ..HoverCheck::default()
    };
    let page = page_at(window, point);
    check.over_page = page.is_some();
    check.holder = holder_of(window);
    check.reason = page_reason(&mut check, window, point, page.as_deref(), click);
    check
}

fn is_problem(check: &HoverCheck, trigger: HoverTrigger) -> bool {
    match check.reason {
        HoverReason::Fine => false,
        HoverReason::NotKeyOrMain if trigger == HoverTrigger::Click => {
            !matches!(
                check.key,
                FocusWindowKind::Popup | FocusWindowKind::ArdiaPanel | FocusWindowKind::Internal
            ) && app().modalWindow().is_none()
        }
        HoverReason::NotKeyOrMain => unseen(check.key) && unseen(check.main),
        HoverReason::CoveredByView => check.cover != PageCover::ReactNative || check.cover_clear,
        HoverReason::HalfActive
        | HoverReason::StaleMenu
        | HoverReason::ResponderLost
        | HoverReason::CoveredByWindow => true,
    }
}

fn panel_or_sheet_open() -> bool {
    let app = app();
    if app.modalWindow().is_some() {
        return true;
    }
    app.windows().iter().any(|window| {
        window.attachedSheet().is_some()
            || (window.isVisible() && window.isKindOfClass(NSSavePanel::class()))
    })
}

fn full_screen_moving() -> bool {
    let now = uptime();
    FULL_SCREEN_MOVES.with_borrow(|moves| {
        moves.iter().any(|(window, since)| {
            window.load().is_some() && now - since < FULL_SCREEN_MOVE_LONGEST
        })
    })
}

fn other_ardia_window_active(window: &NSWindow) -> bool {
    app().windows().iter().any(|other| {
        if std::ptr::eq(&*other, window) || !(other.isKeyWindow() || other.isMainWindow()) {
            return false;
        }
        !matches!(
            kind_of(Some(&other), false),
            FocusWindowKind::Hidden | FocusWindowKind::BlockerPage
        ) && host().window_role(&other).is_some()
    })
}

fn safe_to_heal(window: &NSWindow, check: &HoverCheck, trigger: HoverTrigger) -> bool {
    if !window.isVisible()
        || !window.isOnActiveSpace()
        || !check.activation.in_front()
        || panel_or_sheet_open()
    {
        return false;
    }
    if check.menu_active || menu_shown(window) || full_screen_moving() {
        return false;
    }
    match check.reason {
        HoverReason::HalfActive => {
            trigger == HoverTrigger::Click && check.key != FocusWindowKind::Popup
        }
        HoverReason::ResponderLost => {
            trigger == HoverTrigger::Click && check.over_page && check.window_key
        }
        HoverReason::NotKeyOrMain => {
            check.activation.app_active && unseen(check.key) && !other_ardia_window_active(window)
        }
        _ => false,
    }
}

fn describe_check(check: &HoverCheck, trigger: HoverTrigger) -> String {
    let a = check.activation;
    let mut facts = vec![
        check.reason.name().to_owned(),
        trigger.name().to_owned(),
        format!("key={}", check.key.name()),
        format!("main={}", check.main.name()),
        format!(
            "active={}/{}/{}",
            u8::from(a.app_active),
            u8::from(a.running_active),
            u8::from(a.frontmost)
        ),
    ];
    for (shown, fact) in [
        (a.half(), "halfActive"),
        (check.window_key, "windowKey"),
        (check.window_main, "windowMain"),
        (check.prevent_key, "preventKey"),
        (check.can_key, "canKey"),
        (check.menu_active, "menuActive"),
        (check.over_page, "overPage"),
    ] {
        if shown {
            facts.push(fact.to_owned());
        }
    }
    facts.push(format!("responder={}", check.holder.name()));
    if check.cover != PageCover::None {
        facts.push(format!("cover={}", check.cover.name()));
    }
    if check.cover_window != FocusWindowKind::None {
        facts.push(format!("coverWindow={}", check.cover_window.name()));
    }
    if check.cover_clear {
        facts.push("coverClear".to_owned());
    }
    let time = CLOCK.with(|clock| clock.stringFromDate(&NSDate::now()).to_string());
    format!("{} at {time}", facts.join(" "))
}

fn episode_payload(episode: &HoverEpisode, ended_by: &str) -> Value {
    let first = &episode.first;
    json!({
        "reason": first.reason.name(),
        "trigger": episode.trigger.name(),
        "keyWindow": first.key.name(),
        "mainWindow": first.main.name(),
        "preventKey": first.prevent_key,
        "canKey": first.can_key,
        "windowMain": first.window_main,
        "halfActive": first.activation.half(),
        "appActive": first.activation.app_active,
        "runningActive": first.activation.running_active,
        "frontmost": first.activation.frontmost,
        "menuActive": first.menu_active,
        "overPage": first.over_page,
        "cover": first.cover.name(),
        "coverClear": first.cover_clear,
        "responder": first.holder.name(),
        "coverWindow": first.cover_window.name(),
        "healed": episode.healed,
        "healWorked": (episode.heal_worked >= 0).then_some(episode.heal_worked == 1),
        "endedBy": ended_by,
        "seconds": (uptime() - episode.started_at).max(0.0).round() as i64,
        "episode": EPISODE_COUNT.get(),
    })
}

fn close_episode(ended_by: &str) {
    let Some(mut payload) =
        EPISODE.with_borrow(|episode| episode.open.then(|| episode_payload(episode, ended_by)))
    else {
        return;
    };
    let stamp = EPISODE.take().stamp;
    PENDING_REASON.set(HoverReason::Fine);
    let host = host();
    host.note_focus_change(&format!("hover episode ended by {ended_by}"), None);
    DEV_EVENTS.with_borrow_mut(|events| {
        events.push(payload.clone());
        if events.len() > DEV_HOVER_EVENTS_KEPT {
            events.remove(0);
        }
    });
    payload["epoch"] = Value::String(stamp);
    host.emit("engine.hoverLost", payload);
}

fn recovered(check: &HoverCheck, healed: HoverReason) -> bool {
    if healed == HoverReason::ResponderLost && check.holder != FocusHolder::Page {
        return false;
    }
    check.reason == HoverReason::Fine && !check.activation.half() && check.window_key
}

fn page_under(window: &NSWindow, point: NSPoint) -> Option<Retained<NSView>> {
    let page = enclosing_page(hit_at(window, point).as_deref())?;
    let own = page.window().is_some_and(|own| std::ptr::eq(&*own, window));
    (own && !page.isHiddenOrHasHiddenAncestor()).then_some(page)
}

fn system_sees_us_frontmost() -> bool {
    let me = NSRunningApplication::currentApplication();
    !host().background() && me.isActive() && is_me(&me)
}

fn replay_missed_activation() {
    let app = app();
    if app.isActive() || !system_sees_us_frontmost() {
        return;
    }
    if let Some(activated) = NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
        NSEventType::AppKitDefined,
        NSPoint::ZERO,
        NSEventModifierFlags::empty(),
        uptime(),
        0,
        None,
        NSEventSubtype::ApplicationActivated.0,
        0,
        0,
    ) {
        app.sendEvent(&activated);
    }
}

fn heal(window: &NSWindow, point: NSPoint, reason: HoverReason) {
    EPISODE.with_borrow_mut(|episode| episode.healed = true);
    host().note_focus_change(&format!("hover heal {}", reason.name()), Some(window));
    if reason == HoverReason::ResponderLost {
        if let Some(page) = page_under(window, point) {
            window.makeFirstResponder(Some(&page));
        }
    } else {
        reactivate(window, reason);
    }
    let serial = EPISODE.with_borrow(|episode| episode.serial);
    let weak = Weak::new(window);
    main_queue::after(
        HEAL_RECHECK_AFTER,
        Box::new(move || recheck_heal(&weak, point, reason, serial)),
    );
}

fn reactivate(window: &NSWindow, reason: HoverReason) {
    if prevents_key(window) {
        // SAFETY: The window answers `preventKeyWindow`, a `BOOL` property.
        unsafe {
            window.setValue_forKey(
                Some(&NSNumber::new_bool(false)),
                ns_string!("preventKeyWindow"),
            )
        };
    }
    if reason == HoverReason::HalfActive {
        replay_missed_activation();
        let app = app();
        if !app.isActive() {
            #[allow(deprecated)]
            app.activateIgnoringOtherApps(true);
        }
    }
    window.makeKeyAndOrderFront(None);
}

fn recheck_heal(weak: &Weak<NSWindow>, point: NSPoint, reason: HoverReason, serial: usize) {
    if !EPISODE.with_borrow(|episode| episode.open && episode.serial == serial) {
        return;
    }
    let Some(window) = weak.load() else {
        return close_episode("windowClosed");
    };
    let after = probe(&window, point, false);
    LAST_VERDICT.set(Some(describe_check(&after, HoverTrigger::Recheck)));
    let worked = recovered(&after, reason);
    EPISODE.with_borrow_mut(|episode| episode.heal_worked = i32::from(worked));
    if worked {
        close_episode("heal");
    }
}

fn cover_still_catches_clicks() -> bool {
    INVISIBLE_COVER
        .with_borrow(|cover| cover.as_ref().and_then(Weak::load))
        .is_some_and(|cover| {
            cover.isVisible() && !cover.ignoresMouseEvents() && cover.alphaValue() < 0.01
        })
}

fn ends_episode(check: &HoverCheck) -> bool {
    if check.reason != HoverReason::Fine {
        return false;
    }
    let (first, trigger) = EPISODE.with_borrow(|episode| (episode.first.reason, episode.trigger));
    match first {
        HoverReason::CoveredByView => check.over_page,
        HoverReason::HalfActive => {
            !check.activation.half() && (check.window_key || check.window_main)
        }
        HoverReason::NotKeyOrMain => {
            check.window_key || (check.window_main && trigger == HoverTrigger::Move)
        }
        HoverReason::CoveredByWindow => !cover_still_catches_clicks(),
        _ => true,
    }
}

fn open_episode(window: &NSWindow, check: HoverCheck, trigger: HoverTrigger, stamp: String) {
    let serial = EPISODE_COUNT.get() + 1;
    EPISODE_COUNT.set(serial);
    EPISODE.set(HoverEpisode {
        open: true,
        owner: Some(Weak::new(window)),
        serial,
        first: check,
        trigger,
        started_at: uptime(),
        stamp,
        ..HoverEpisode::default()
    });
    host().note_focus_change(
        &format!("hover lost: {}", check.reason.name()),
        Some(window),
    );
}

fn close_if_owner_gone() {
    if EPISODE.with_borrow(|episode| {
        episode.open && episode.owner.as_ref().and_then(Weak::load).is_none()
    }) {
        close_episode("windowClosed");
    }
}

fn handle_hover_check(
    window: Option<&NSWindow>,
    point: NSPoint,
    trigger: HoverTrigger,
    stamp: String,
) {
    close_if_owner_gone();
    let Some(window) = window else {
        return;
    };
    if EPISODE.with_borrow(|episode| episode.open && !same(window, episode.owner.as_ref())) {
        return;
    }
    let check = probe(window, point, trigger == HoverTrigger::Click);
    if trigger == HoverTrigger::Move && !check.over_page {
        return;
    }
    LAST_VERDICT.set(Some(describe_check(&check, trigger)));
    if !is_problem(&check, trigger) {
        PENDING_REASON.set(HoverReason::Fine);
        settle_episode(&check);
        return;
    }
    if !EPISODE.with_borrow(|episode| episode.open) {
        if trigger == HoverTrigger::Move && !pending_matches(window, check.reason) {
            PENDING_REASON.set(check.reason);
            PENDING_WINDOW.set(Some(Weak::new(window)));
            return;
        }
        open_episode(window, check, trigger, stamp);
    }
    if !EPISODE.with_borrow(|episode| episode.healed) && safe_to_heal(window, &check, trigger) {
        heal(window, point, check.reason);
    }
}

fn pending_matches(window: &NSWindow, reason: HoverReason) -> bool {
    PENDING_REASON.get() == reason
        && PENDING_WINDOW.with_borrow(|pending| same(window, pending.as_ref()))
}

fn settle_episode(check: &HoverCheck) {
    if !EPISODE.with_borrow(|episode| episode.open) || !ends_episode(check) {
        return;
    }
    let healed = EPISODE.with_borrow(|episode| episode.healed && episode.heal_worked != 0);
    close_episode(if healed { "heal" } else { "hover" });
}

fn handle_covered_click(
    browser: Option<&NSWindow>,
    cover: Option<&NSWindow>,
    point: NSPoint,
    stamp: String,
) {
    close_if_owner_gone();
    let (Some(browser), Some(cover)) = (browser, cover) else {
        return;
    };
    if !invisible(cover) || !cover.isVisible() {
        return;
    }
    if EPISODE.with_borrow(|episode| episode.open) {
        heal_covered_click(browser, cover);
        return;
    }
    let mut check = probe(browser, point, true);
    check.reason = HoverReason::CoveredByWindow;
    check.cover_window = kind_of(Some(cover), true);
    LAST_VERDICT.set(Some(describe_check(&check, HoverTrigger::Click)));
    INVISIBLE_COVER.set(Some(Weak::new(cover)));
    open_episode(browser, check, HoverTrigger::Click, stamp);
}

fn heal_covered_click(browser: &NSWindow, cover: &NSWindow) {
    let ours = EPISODE.with_borrow(|episode| {
        same(browser, episode.owner.as_ref())
            && episode.first.reason == HoverReason::CoveredByWindow
    });
    if !ours || !INVISIBLE_COVER.with_borrow(|known| same(cover, known.as_ref())) {
        return;
    }
    EPISODE.with_borrow_mut(|episode| episode.healed = true);
    host().note_focus_change(
        "hover heal coveredByWindow: the invisible window stops taking clicks",
        Some(browser),
    );
    cover.setIgnoresMouseEvents(true);
    EPISODE.with_borrow_mut(|episode| episode.heal_worked = 1);
    close_episode("heal");
}

fn browser_window_below(cover: &NSWindow, screen_point: NSPoint) -> Option<Retained<NSWindow>> {
    let mut below = false;
    for window in app().orderedWindows().iter() {
        if std::ptr::eq(&*window, cover) {
            below = true;
            continue;
        }
        if below
            && window.isVisible()
            && point_in_rect(screen_point, window.frame())
            && is_browser_window(Some(&window))
            && !invisible(&window)
        {
            return Some(window);
        }
    }
    None
}

fn screen_point(event: &NSEvent) -> NSPoint {
    match event.window(mtm()) {
        Some(window) => window.convertPointToScreen(event.locationInWindow()),
        None => NSEvent::mouseLocation(),
    }
}

fn window_under(point: NSPoint) -> Option<Retained<NSWindow>> {
    let number = NSWindow::windowNumberAtPoint_belowWindowWithWindowNumber(point, 0, mtm());
    app().windowWithWindowNumber(number)
}

fn invisible_cover_for(event: &NSEvent) -> Option<Retained<NSWindow>> {
    let mut under = window_under(screen_point(event));
    if under.is_none() && WATCH_IN_BACKGROUND.get() {
        under = event.window(mtm());
    }
    let under = under?;
    let catches = !is_browser_window(Some(&under))
        && under.isVisible()
        && under.alphaValue() < 0.01
        && !under.ignoresMouseEvents();
    catches.then_some(under)
}

fn browser_window_for(event: &NSEvent) -> Option<Retained<NSWindow>> {
    let under = window_under(NSEvent::mouseLocation());
    if is_browser_window(under.as_deref()) {
        return under;
    }
    if under.is_some() || !WATCH_IN_BACKGROUND.get() {
        return None;
    }
    event
        .window(mtm())
        .filter(|window| is_browser_window(Some(window)))
}

fn point_in(window: &NSWindow, event: &NSEvent) -> NSPoint {
    if event
        .window(mtm())
        .is_some_and(|own| std::ptr::eq(&*own, window))
    {
        return event.locationInWindow();
    }
    window.convertPointFromScreen(NSEvent::mouseLocation())
}

fn stop_hover_watch() {
    if let Some(monitor) = MONITOR.take() {
        // SAFETY: AppKit issued the token to `start_hover_watch`.
        unsafe { NSEvent::removeMonitor(&monitor) };
    }
}

fn start_hover_watch() {
    if MONITOR.with_borrow(Option::is_some) || (host().background() && !WATCH_IN_BACKGROUND.get()) {
        return;
    }
    let handler = RcBlock::new(|event: NonNull<NSEvent>| {
        // SAFETY: AppKit lends a live event for the call.
        on_event(unsafe { event.as_ref() });
        event.as_ptr()
    });
    // SAFETY: The handler returns the event it got.
    let monitor = unsafe {
        NSEvent::addLocalMonitorForEventsMatchingMask_handler(
            NSEventMask::MouseMoved | NSEventMask::LeftMouseDown,
            &handler,
        )
    };
    MONITOR.set(monitor);
}

fn on_event(event: &NSEvent) {
    let moved = event.r#type() == NSEventType::MouseMoved;
    if moved && uptime() - LAST_MOVE_CHECK.get() < MOVE_CHECK_EVERY {
        return;
    }
    if moved {
        LAST_MOVE_CHECK.set(uptime());
    }
    if !moved
        && event
            .modifierFlags()
            .contains(NSEventModifierFlags::Command)
    {
        return;
    }
    if !current_activation().in_front() {
        return;
    }
    let stamp = host().consent_epoch();
    match browser_window_for(event) {
        Some(window) => {
            let point = point_in(&window, event);
            let trigger = if moved {
                HoverTrigger::Move
            } else {
                HoverTrigger::Click
            };
            let weak = Weak::new(&*window);
            main_queue::post(Box::new(move || {
                handle_hover_check(weak.load().as_deref(), point, trigger, stamp);
            }));
        }
        None if !moved => post_covered_click(event, stamp),
        None => {}
    }
}

fn post_covered_click(event: &NSEvent, stamp: String) {
    let screen = screen_point(event);
    let Some(cover) = invisible_cover_for(event) else {
        return;
    };
    let Some(browser) = browser_window_below(&cover, screen) else {
        return;
    };
    let point = browser.convertPointFromScreen(screen);
    let (browser, cover) = (Weak::new(&*browser), Weak::new(&*cover));
    main_queue::post(Box::new(move || {
        handle_covered_click(
            browser.load().as_deref(),
            cover.load().as_deref(),
            point,
            stamp,
        );
    }));
}

fn end_episode_if_inactive() {
    if !current_activation().in_front() {
        close_episode("appInactive");
    }
}

/// `HoverProbeLine`.
pub fn hover_probe_line() -> String {
    let a = current_activation();
    let verdict = LAST_VERDICT.with_borrow(|verdict| verdict.clone());
    let lost = EPISODE.with_borrow(|episode| {
        episode
            .open
            .then(|| format!(", lost now: {}", episode.first.reason.name()))
    });
    format!(
        "activation app {} running {} frontmost {}{}\nhover {}, episodes {}{}",
        u8::from(a.app_active),
        u8::from(a.running_active),
        u8::from(a.frontmost),
        if a.half() { " (half active)" } else { "" },
        verdict.as_deref().unwrap_or("not checked"),
        EPISODE_COUNT.get(),
        lost.unwrap_or_default(),
    )
}

/// `WatchHover`: installs `host`, follows activation, window closing and full-screen moves, and starts the
/// event watch.
pub fn watch_hover(host: Rc<dyn HoverHost>) {
    HOST.set(Some(host));
    let center = NSNotificationCenter::defaultCenter();
    // SAFETY: The notification names are AppKit's own.
    let names = unsafe {
        [
            NSWindowWillEnterFullScreenNotification,
            NSWindowWillExitFullScreenNotification,
            NSWindowDidEnterFullScreenNotification,
            NSWindowDidExitFullScreenNotification,
        ]
    };
    let mut observers = vec![
        // SAFETY: As above.
        observe(
            &center,
            unsafe { NSApplicationDidResignActiveNotification },
            |_| end_episode_if_inactive(),
        ),
        // SAFETY: As above.
        observe(&center, unsafe { NSWindowWillCloseNotification }, closing),
    ];
    for (index, name) in names.into_iter().enumerate() {
        let moving = index < 2;
        observers.push(observe(&center, name, move |note| {
            full_screen_move(note, moving)
        }));
    }
    let workspace = NSWorkspace::sharedWorkspace().notificationCenter();
    // SAFETY: As above.
    let activated = unsafe { NSWorkspaceDidActivateApplicationNotification };
    observers.push(observe(&workspace, activated, |_| {
        end_episode_if_inactive()
    }));
    OBSERVERS.set(observers);
    start_hover_watch();
}

fn closing(note: &NSNotification) {
    let owner = EPISODE.with_borrow(|episode| {
        episode
            .open
            .then(|| episode.owner.as_ref().and_then(Weak::load))
            .flatten()
    });
    let object = note.object();
    if owner.is_some_and(|owner| {
        object
            .as_deref()
            .is_some_and(|object| std::ptr::eq(Retained::as_ptr(&owner).cast(), object))
    }) {
        close_episode("windowClosed");
    }
}

fn full_screen_move(note: &NSNotification, moving: bool) {
    let Some(window) = note
        .object()
        .and_then(|object| object.downcast::<NSWindow>().ok())
    else {
        return;
    };
    FULL_SCREEN_MOVES.with_borrow_mut(|moves| {
        moves.retain(|(known, _)| {
            known
                .load()
                .is_some_and(|known| !std::ptr::eq(&*known, &*window))
        });
        if moving {
            moves.push((Weak::new(&*window), uptime()));
        }
    });
}

fn observe(
    center: &NSNotificationCenter,
    name: &NSNotificationName,
    callback: impl Fn(&NSNotification) + 'static,
) -> Retained<AnyObject> {
    let block = RcBlock::new(move |note: NonNull<NSNotification>| {
        // SAFETY: The center lends a live notification for the call.
        callback(unsafe { note.as_ref() });
    });
    // SAFETY: These notifications are posted on the main thread, where the block runs.
    let token = unsafe {
        center.addObserverForName_object_queue_usingBlock(Some(name), None, None, &block)
    };
    token.into()
}
