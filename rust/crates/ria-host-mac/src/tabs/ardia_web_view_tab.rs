// Ported from ardia packages/ardiacore/ios/ArdiaWebView+Tab.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::ptr;
use std::rc::Rc;
use std::time::{Duration, Instant};

use objc2::rc::{Retained, Weak};
use objc2_app_kit::{
    NSColor, NSUserInterfaceItemIdentification, NSView, NSWindow, NSWindowOrderingMode,
};
use objc2_foundation::{NSPoint, NSString};
use ria_engine::call::main_queue;
use serde_json::{Value, json};

use crate::core::bindings::{Adoption, Profile, Tab, Window};
use crate::core::convert::to_json;

use super::ardia_tab_lifecycle::TabLifecycle;
use super::ardia_tab_registry::TabRegistry;
use super::superview;

const MATCH_STEP: Duration = Duration::from_millis(25);
const LINK_LIFETIME: Duration = Duration::from_secs(2);
const HIDDEN_LINK_LIFETIME: Duration = Duration::from_secs(1);
const SMALL_WINDOW: &str = "ardia.window.small";

pub fn window_takes(window: Option<&Window>, tab: &Tab) -> Adoption {
    window.map_or(Adoption::Adopted, |window| window.can_adopt_tab(tab))
}

/// What `-holdTransferOf:` (Tab.mm 41-53) and `-adoptMovedPage` (Tab.mm 72-84) do after the window answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferAnswer {
    Proceed,
    /// Run `ensureTab` or `adoptMovedPage` again on the next main-queue turn.
    Retry,
    /// Emit `transferRefused` with the `from` window id.
    Refuse,
}

pub fn transfer_answer(adoption: Adoption) -> TransferAnswer {
    match adoption {
        Adoption::Adopted => TransferAnswer::Proceed,
        Adoption::Busy => TransferAnswer::Retry,
        Adoption::Refused => TransferAnswer::Refuse,
    }
}

/// `-tabActivatedByChrome:` (Tab.mm 359-368): what Chrome activating the tab does to focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChromeActivation {
    Ignore,
    KeepFocus,
    RestoreFocus,
}

pub fn chrome_activation(
    standalone: bool,
    by_chrome: bool,
    visible: bool,
    selected: bool,
) -> ChromeActivation {
    if standalone {
        ChromeActivation::Ignore
    } else if by_chrome && visible && !selected {
        ChromeActivation::KeepFocus
    } else {
        ChromeActivation::RestoreFocus
    }
}

pub fn adopt_into_window(
    window: Option<&Window>,
    tab: &Tab,
    place: Option<(i32, bool)>,
) -> Adoption {
    let Some(window) = window else {
        return Adoption::Adopted;
    };
    if tab
        .profile()
        .is_some_and(|profile| window.tabs_for_profile(&profile).containsObject(tab))
    {
        return Adoption::Adopted;
    }
    let adoption = window.adopt_tab(tab);
    if adoption == Adoption::Adopted
        && let Some((index, pinned)) = place.filter(|(index, _)| *index >= 0)
    {
        window.place_tab(tab, index, pinned);
    }
    adoption
}

pub enum AttachStep<'a> {
    Configure(&'a Tab),
    PageMoved(&'a Tab),
    LayoutAndPaint,
    HoldForFirstPaint { created: bool },
    Load { url: &'a str, user: bool },
    Ready,
    HeldEvents(Vec<(String, Value)>),
    ReportInitialState(&'a Tab),
    SettleFocus,
}

pub struct Attach<'a> {
    pub host: &'a NSView,
    pub app_id: String,
    pub window: Option<&'a Window>,
    pub placement: Option<(i32, bool)>,
    pub shown_early: bool,
    pub created: bool,
    pub temporary: bool,
    pub muted: bool,
    pub page_background: Option<&'a NSColor>,
    pub contents_background: Option<&'a NSColor>,
    pub page_color: Option<&'a NSString>,
}

pub struct AttachedTab<Report> {
    pub life: TabLifecycle,
    pub tab: Option<Retained<Tab>>,
    pub devtools: Option<Retained<NSView>>,
    pub reports: Report,
}

impl<Report: Default> Default for AttachedTab<Report> {
    fn default() -> Self {
        Self {
            life: TabLifecycle::default(),
            tab: None,
            devtools: None,
            reports: Report::default(),
        }
    }
}

#[derive(Default)]
pub struct StashedReports<Report>(Vec<(Weak<Tab>, String, Report)>);

impl<Report> StashedReports<Report> {
    fn stash(&mut self, tab: &Tab, report: Report) {
        self.0
            .retain(|(old, _, _)| old.load().is_some_and(|old| !ptr::eq(&*old, tab)));
        if !tab.closed() {
            self.0
                .push((Weak::from(tab), tab.url().to_string(), report));
        }
    }

    fn take(&mut self, tab: &Tab) -> Option<Report> {
        let index = self
            .0
            .iter()
            .position(|(old, _, _)| old.load().is_some_and(|old| ptr::eq(&*old, tab)))?;
        let (_, url, report) = self.0.swap_remove(index);
        (url == tab.url().to_string()).then_some(report)
    }
}

fn mounted(page: &NSView, host: &NSView) -> bool {
    superview(page).is_some_and(|parent| ptr::eq(&*parent, host))
}

fn same_window(left: &NSView, right: &NSView) -> bool {
    left.window()
        .is_some_and(|left| right.window().is_some_and(|right| ptr::eq(&*left, &*right)))
}

impl<Report: Default> AttachedTab<Report> {
    pub fn attach(
        &mut self,
        tab: Retained<Tab>,
        attach: Attach<'_>,
        registry: &RefCell<TabRegistry>,
        stashed: &mut StashedReports<Report>,
        accept_drops: impl Fn(&NSView, bool),
        mut step: impl FnMut(AttachStep<'_>),
    ) -> Adoption {
        if !self.life.can_attach(self.tab.is_some()) {
            return Adoption::Refused;
        }
        let allowed = window_takes(attach.window, &tab);
        if allowed != Adoption::Adopted {
            return allowed;
        }
        self.life.attached();
        self.tab = Some(tab.clone());
        registry
            .borrow_mut()
            .set_view(&tab, Some((attach.host, attach.app_id)));
        tab.set_temporary(attach.temporary);
        step(AttachStep::Configure(&tab));
        if let Some(reports) = stashed.take(&tab) {
            self.reports = reports;
        }
        tab.deliver_held_page_messages();
        let adoption = adopt_into_window(attach.window, &tab, attach.placement);
        if let Some(page) = tab.view()
            && !mounted(&page, attach.host)
        {
            if !same_window(&page, attach.host) {
                page.removeFromSuperview();
            }
            attach.host.addSubview(&page);
            step(AttachStep::PageMoved(&tab));
        }
        self.set_devtools(attach.host, tab.dev_tools_view(), &accept_drops);
        step(AttachStep::LayoutAndPaint);
        if !attach.shown_early
            && (attach.created || self.life.creating_url.is_some() || tab.loading())
        {
            step(AttachStep::HoldForFirstPaint {
                created: attach.created,
            });
        }
        if attach.muted {
            tab.set_muted(true);
        }
        if let Some(color) = attach.page_background {
            tab.set_page_background_color(Some(color));
        }
        if let Some(color) = attach.contents_background {
            tab.set_contents_background_color(Some(color));
        }
        if let Some(color) = attach.page_color {
            tab.set_page_color(Some(color));
        }
        if let Some(url) = self.life.pending_url.take() {
            step(AttachStep::Load {
                url: &url,
                user: self.life.pending_user_initiated,
            });
        }
        step(AttachStep::Ready);
        let events = registry.borrow_mut().release_held_events(&tab);
        step(AttachStep::HeldEvents(events));
        step(AttachStep::ReportInitialState(&tab));
        if let (Some(window), Some((index, pinned))) = (attach.window, attach.placement)
            && index >= 0
        {
            window.place_tab(&tab, index, pinned);
        }
        step(AttachStep::SettleFocus);
        adoption
    }

    pub fn detach(
        &mut self,
        host: &NSView,
        registry: &RefCell<TabRegistry>,
        stashed: &mut StashedReports<Report>,
        accept_drops: impl Fn(&NSView, bool),
        clear_host: impl FnOnce(&Tab),
    ) -> Option<Retained<Tab>> {
        let tab = self.tab.take()?;
        registry.borrow_mut().set_view(&tab, None);
        stashed.stash(&tab, std::mem::take(&mut self.reports));
        if let Some(page) = tab.view() {
            accept_drops(&page, true);
            if mounted(&page, host) && !self.life.keeps_page_on_detach {
                page.removeFromSuperview();
            }
        }
        self.life.keeps_page_on_detach = false;
        self.set_devtools(host, None, accept_drops);
        clear_host(&tab);
        self.life.detached();
        self.life.handing_off = false;
        Some(tab)
    }

    pub fn set_devtools(
        &mut self,
        host: &NSView,
        next: Option<Retained<NSView>>,
        accept_drops: impl Fn(&NSView, bool),
    ) {
        if let Some(old) = self
            .devtools
            .take()
            .filter(|old| next.as_ref().is_none_or(|next| !ptr::eq(&**old, &**next)))
        {
            accept_drops(&old, true);
            if mounted(&old, host) {
                old.removeFromSuperview();
            }
        }
        if let Some(view) = next.as_ref().filter(|view| !mounted(view, host)) {
            let page = self
                .tab
                .as_deref()
                .and_then(Tab::view)
                .filter(|page| mounted(page, host));
            view.removeFromSuperview();
            if let Some(page) = page {
                host.addSubview_positioned_relativeTo(
                    view,
                    NSWindowOrderingMode::Below,
                    Some(&page),
                );
            } else {
                host.addSubview(view);
            }
        }
        self.devtools = next;
    }
}

#[derive(Clone, Copy, Default)]
pub struct PopupPolicy {
    pub popups_as_tabs: bool,
    pub small_window_links_in_place: bool,
}

pub(super) fn small_window(window: Option<&NSWindow>) -> bool {
    window
        .and_then(NSWindow::identifier)
        .is_some_and(|id| id.to_string() == SMALL_WINDOW)
}

impl PopupPolicy {
    pub fn opens_popups_as_tabs(self, window: Option<&NSWindow>) -> bool {
        self.popups_as_tabs && !small_window(window)
    }
    pub fn opens_new_tab_links_in_place(self, window: Option<&NSWindow>) -> bool {
        self.small_window_links_in_place && small_window(window)
    }
}

#[derive(Default)]
pub struct NewTabLinks(VecDeque<(String, Instant)>);

impl NewTabLinks {
    fn expire(&mut self, now: Instant) {
        while self
            .0
            .front()
            .is_some_and(|(_, at)| now.saturating_duration_since(*at) >= LINK_LIFETIME)
        {
            self.0.pop_front();
        }
    }

    pub fn note(&mut self, url: String, now: Instant) {
        if url.is_empty() {
            return;
        }
        self.expire(now);
        if self.0.len() == 4 {
            self.0.pop_front();
        }
        self.0.push_back((url, now));
    }

    fn take(&mut self, first_url: &str, url: &str, hidden: bool, now: Instant) -> Option<String> {
        self.expire(now);
        if let Some(index) = self
            .0
            .iter()
            .position(|(noted, _)| noted == first_url || noted == url)
        {
            return self.0.remove(index).map(|(url, _)| url);
        }
        if hidden
            && self
                .0
                .back()
                .is_some_and(|(_, at)| now.saturating_duration_since(*at) <= HIDDEN_LINK_LIFETIME)
        {
            return self.0.pop_back().map(|(url, _)| url);
        }
        None
    }
}

pub trait OpenedPage {
    fn links(&self) -> &RefCell<NewTabLinks>;
    fn has_tab(&self) -> bool;
    fn may_hold_user_activation(&self) -> bool;
    fn opens_new_tab_links_in_place(&self) -> bool;
    fn take_offered(&self, adopt_id: &str) -> Option<Retained<Tab>>;
    fn recent_click(&self) -> Option<NSPoint>;
    fn take_shift_click(&self) -> bool;
    fn original_profile_name(&self, profile: &Profile) -> String;
    fn emit_open_window(&self, payload: Value);
}

pub struct OpenedTab {
    pub tab: Retained<Tab>,
    pub adopt_id: String,
    pub disposition: String,
    pub popup: bool,
    pub foreground_link: bool,
}

impl OpenedTab {
    pub fn announce(self, page: &Rc<impl OpenedPage + 'static>) {
        poll_url(Rc::downgrade(page), self, 0);
    }
}

fn poll_url<P: OpenedPage + 'static>(page: std::rc::Weak<P>, opened: OpenedTab, tries: u8) {
    if opened.tab.closed() {
        return;
    }
    let url = opened.tab.url().to_string();
    if (url.is_empty() || url == "about:blank") && tries < 20 {
        return main_queue::after(
            MATCH_STEP,
            Box::new(move || poll_url(page, opened, tries + 1)),
        );
    }
    let Some(live) = page.upgrade() else { return };
    if opened.foreground_link && opened.disposition == "foreground" && live.has_tab() {
        match_link(page, opened, url, tries > 0, 0);
    } else {
        emit_open(&*live, opened, false, None);
    }
}

fn match_link<P: OpenedPage + 'static>(
    page: std::rc::Weak<P>,
    opened: OpenedTab,
    first_url: String,
    hidden: bool,
    tries: u8,
) {
    let Some(live) = page.upgrade().filter(|_| !opened.tab.closed()) else {
        return;
    };
    let url = opened.tab.url().to_string();
    let clicked = live
        .has_tab()
        .then(|| {
            live.links()
                .borrow_mut()
                .take(&first_url, &url, hidden, Instant::now())
        })
        .flatten();
    if let Some(clicked) = clicked {
        if live.opens_new_tab_links_in_place() {
            if let Some(tab) = live
                .take_offered(&opened.adopt_id)
                .filter(|tab| ptr::eq(&**tab, &*opened.tab))
            {
                tab.close();
            }
            live.emit_open_window(json!({"url": clicked, "disposition": "current", "userGesture": true, "linkWait": "matched"}));
        } else {
            emit_open(&*live, opened, true, Some("matched"));
        }
    } else if live.has_tab() && (tries > 0 || live.may_hold_user_activation()) && tries < 8 {
        main_queue::after(
            MATCH_STEP,
            Box::new(move || match_link(page, opened, first_url, hidden, tries + 1)),
        );
    } else {
        emit_open(
            &*live,
            opened,
            false,
            Some(if tries >= 8 { "timedOut" } else { "skipped" }),
        );
    }
}

fn emit_open(page: &impl OpenedPage, opened: OpenedTab, clicked: bool, wait: Option<&str>) {
    let mut payload = json!({"url": opened.tab.url().to_string(), "disposition": opened.disposition,
        "adoptId": opened.adopt_id, "userGesture": true, "postBody": false, "fromClick": clicked});
    if let Some(profile) = opened
        .tab
        .profile()
        .filter(|profile| profile.off_the_record())
    {
        payload["profile"] = Value::from(page.original_profile_name(&profile));
    }
    if clicked && let Some(point) = page.recent_click() {
        payload["point"] = json!({"x": point.x, "y": point.y});
    }
    if let Some(wait) = wait {
        payload["linkWait"] = Value::from(wait);
    }
    if payload["disposition"] == "window" && page.take_shift_click() {
        payload["shiftClick"] = Value::Bool(true);
    }
    if opened.popup {
        payload["popup"] = Value::Bool(true);
    }
    page.emit_open_window(payload);
}

pub fn discard(tab: Option<&Tab>, unload: bool) -> &'static str {
    if unload {
        return "unsupported";
    }
    let Some(tab) = tab else {
        return "refused";
    };
    if tab.discarded() {
        "already"
    } else if tab.discard() && tab.discarded() {
        "discarded"
    } else {
        "refused"
    }
}

pub fn sleep_if_hidden_for(tab: Option<&Tab>, seconds: f64) -> Value {
    tab.filter(|tab| !tab.closed() && !tab.discarded())
        .map_or_else(
            || json!([-1]),
            |tab| to_json(&tab.sleep_if_hidden_for(seconds)),
        )
}

#[derive(Default)]
pub struct UnloadState {
    pub visible: bool,
    pub standalone_or_extension: bool,
    pub transition: bool,
    pub capturing: bool,
    pub picture_in_picture: bool,
}

impl<Report> AttachedTab<Report> {
    pub fn kept_from_unloading_by(
        &self,
        transfer_key: &str,
        state: UnloadState,
    ) -> Option<&'static str> {
        let tab = match self.tab.as_deref() {
            Some(tab)
                if !tab.closed()
                    && !tab
                        .profile()
                        .is_some_and(|profile| profile.off_the_record())
                    && !transfer_key.is_empty() =>
            {
                tab
            }
            _ => return Some("tab"),
        };
        if state.visible {
            Some("visible")
        } else if state.standalone_or_extension || tab.temporary() {
            Some("ownWindow")
        } else if self.life.closed()
            || self.life.handing_off
            || self.life.moving
            || self.life.pending_url.is_some()
        {
            Some("moving")
        } else if state.transition {
            Some("transition")
        } else if state.capturing {
            Some("capture")
        } else if state.picture_in_picture {
            Some("pictureInPicture")
        } else if self.devtools.is_some() {
            Some("devTools")
        } else {
            None
        }
    }
}

pub fn discarded_event(tab: &Tab) -> (&'static str, Value) {
    if tab.discarded() {
        (
            "discarded",
            json!({"url": tab.url().to_string(), "reason": tab.discard_reason().map_or_else(String::new, |reason| reason.to_string())}),
        )
    } else {
        ("ready", json!({}))
    }
}

pub fn renderer_gone(status: Option<&str>, code: i32) -> Value {
    json!({"status": code, "reason": status.unwrap_or("unknown"), "code": code})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_matching_consumes_only_the_expected_link() {
        let mut links = NewTabLinks::default();
        let now = Instant::now();
        links.note("first".into(), now);
        links.note("second".into(), now);
        assert_eq!(
            links.take("other", "second", false, now).as_deref(),
            Some("second")
        );
        assert!(links.take("other", "second", false, now).is_none());
        assert_eq!(
            links.take("other", "second", true, now).as_deref(),
            Some("first")
        );
    }

    #[test]
    fn adoption_answers_map_to_the_ardia_branches() {
        assert_eq!(transfer_answer(Adoption::Adopted), TransferAnswer::Proceed);
        assert_eq!(transfer_answer(Adoption::Busy), TransferAnswer::Retry);
        assert_eq!(transfer_answer(Adoption::Refused), TransferAnswer::Refuse);
    }

    #[test]
    fn chrome_activation_keeps_focus_only_in_an_unselected_visible_pane() {
        use ChromeActivation::*;
        assert_eq!(chrome_activation(true, true, true, false), Ignore);
        assert_eq!(chrome_activation(false, true, true, false), KeepFocus);
        assert_eq!(chrome_activation(false, true, true, true), RestoreFocus);
        assert_eq!(chrome_activation(false, true, false, false), RestoreFocus);
        assert_eq!(chrome_activation(false, false, true, false), RestoreFocus);
    }
}
