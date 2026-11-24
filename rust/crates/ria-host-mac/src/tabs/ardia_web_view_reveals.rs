// Ported from ardia packages/ardiacore/ios/ArdiaWebView+Reveals.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::RefCell;
use std::ptr;
use std::rc::Rc;
use std::time::Duration;

use objc2::rc::{Retained, Weak};
use objc2::{MainThreadMarker, msg_send};
use objc2_app_kit::{
    NSEvent, NSEventModifierFlags, NSEventType, NSUserInterfaceItemIdentification, NSView,
    NSWindow, NSWindowOrderingMode,
};
use ria_engine::call::main_queue;

use crate::core::bindings::Tab;

use super::ardia_tab_parking::TabParking;
use super::ardia_tab_presses::{PressId, TabPresses};
use super::ardia_tab_registry::TabRegistry;
use super::superview;

const BACKSTOP: Duration = Duration::from_secs(5);
const SMALL_WINDOW: &str = "ardia.window.small";

#[derive(Default)]
pub struct Reveal {
    lowered: Option<(Weak<NSView>, f64)>,
    native: bool,
    painted: bool,
    press: Option<PressId>,
    generation: u64,
    front: Option<Retained<NSView>>,
    front_generation: u64,
}

pub struct Page<'a> {
    pub host: &'a NSView,
    pub tab: Option<&'a Tab>,
    pub visible: bool,
    pub warm: bool,
    pub asleep: bool,
    pub standalone: bool,
    pub closed: bool,
    pub crashed: bool,
    pub devtools: bool,
    pub reveal: &'a Reveal,
}

pub struct EndedReveal {
    pub native: bool,
    pub painted: bool,
    pub press: Option<PressId>,
}

pub struct RevealHost<'a> {
    pub host: &'a NSView,
    pub reveal: &'a Rc<RefCell<Reveal>>,
}

pub enum DropReveals<'a> {
    Native(Option<&'a NSWindow>),
    Press(PressId),
    Window {
        window: &'a NSWindow,
        keep: Option<&'a NSView>,
        press: Option<PressId>,
    },
}

pub fn drop_reveals(
    hosts: &[RevealHost<'_>],
    filter: DropReveals<'_>,
    presses: &mut TabPresses,
    mut ended: impl FnMut(&NSView, EndedReveal),
) {
    for host in hosts {
        let change = {
            let mut reveal = host.reveal.borrow_mut();
            let drop = match filter {
                DropReveals::Native(window) => {
                    reveal.native && window.is_none_or(|window| same_window(host.host, window))
                }
                DropReveals::Press(press) => reveal.native && reveal.press == Some(press),
                DropReveals::Window {
                    window,
                    keep,
                    press,
                } => {
                    if !same_window(host.host, window) {
                        continue;
                    }
                    reveal.drop_front(host.host);
                    !keep.is_some_and(|keep| ptr::eq(keep, host.host))
                        && !(reveal.native && older_press(press, reveal.press))
                }
            };
            drop.then(|| reveal.end()).flatten()
        };
        if let Some(change) = change {
            if let Some(press) = change.press {
                presses.forget_switches(press);
            }
            ended(host.host, change);
        }
    }
}

/// Return the discarded tab so the caller can close it after releasing its registry borrow.
pub fn take_discarded_offer(
    id: &str,
    registry: &mut TabRegistry,
    parking: &TabParking,
    hosts: &[RevealHost<'_>],
) -> Option<Retained<Tab>> {
    if !id.starts_with("ardiacore:") {
        return None;
    }
    let tab = registry.offered(id).filter(|tab| {
        !tab.closed() && registry.view_for_tab(tab).is_none() && parking.link_mark(tab).is_none()
    })?;
    let page = tab.view()?;
    for host in hosts {
        let mut reveal = host.reveal.borrow_mut();
        if reveal
            .front
            .as_ref()
            .is_some_and(|front| ptr::eq(&**front, &*page))
        {
            reveal.drop_front(host.host);
        }
    }
    registry.take_offered(id)
}

pub fn older_press(press: Option<PressId>, than: Option<PressId>) -> bool {
    matches!((press, than), (Some(press), Some(than)) if press < than)
}

pub(super) fn hit_in_window(event: &NSEvent) -> Option<Retained<NSView>> {
    let content = event.window(MainThreadMarker::new()?)?.contentView()?;
    let outer = superview(&content)?;
    content.hitTest(outer.convertPoint_fromView(event.locationInWindow(), None))
}

pub fn pressed_tab_tag(
    event: &NSEvent,
    tag: impl Fn(&NSView) -> Option<String>,
    is_row_control: impl Fn(&NSView) -> bool,
) -> Option<(String, bool)> {
    let window = event.window(MainThreadMarker::new()?)?;
    let content = window.contentView()?;
    let hit = hit_in_window(event)?;
    if !(window.isKeyWindow() || hit.acceptsFirstMouse(Some(event))) {
        return None;
    }
    let mut at = Some(hit);
    while let Some(view) = at.filter(|view| !ptr::eq(&**view, &*content)) {
        if let Some(tag) = tag(&view) {
            let key = tag
                .strip_prefix("tab-switch:")
                .map(|key| (key, true))
                .or_else(|| tag.strip_prefix("tab-selected:").map(|key| (key, false)));
            if let Some((key, switches)) = key.filter(|(key, _)| !key.is_empty()) {
                return Some((key.into(), switches));
            }
        }
        if is_row_control(&view) {
            return None;
        }
        at = superview(&view);
    }
    None
}

pub fn record_press(
    presses: &mut TabPresses,
    enabled: bool,
    event: &NSEvent,
    key: &str,
) -> Option<(PressId, bool)> {
    if !enabled || event.r#type() != NSEventType::LeftMouseDown || key.is_empty() {
        return None;
    }
    let id = presses.began(key, event.timestamp() * 1000.0);
    let chord = NSEventModifierFlags::Command
        | NSEventModifierFlags::Option
        | NSEventModifierFlags::Shift
        | NSEventModifierFlags::Control;
    Some((
        id,
        event.clickCount() <= 1 && !event.modifierFlags().intersects(chord),
    ))
}

pub fn window_allows_reveal(window: &NSWindow, typing_outside_page: bool, in_split: bool) -> bool {
    !in_split
        && !typing_outside_page
        && window
            .identifier()
            .is_none_or(|id| id.to_string() != SMALL_WINDOW)
}

fn same_window(view: &NSView, window: &NSWindow) -> bool {
    view.window().is_some_and(|known| ptr::eq(&*known, window))
}

fn mounted(page: &NSView, host: &NSView) -> bool {
    superview(page).is_some_and(|parent| ptr::eq(&*parent, host))
}

fn windows_match(left: &NSView, right: &NSView) -> bool {
    left.window()
        .is_some_and(|window| same_window(right, &window))
}

fn same_profile(left: &Tab, right: &Tab) -> bool {
    let left = left.profile();
    let right = right.profile();
    match (left.as_deref(), right.as_deref()) {
        (Some(left), Some(right)) => ptr::eq(left, right),
        (None, None) => true,
        _ => false,
    }
}

pub fn reveals_first(page: &Page<'_>, in_split: bool, others: &[Page<'_>]) -> bool {
    if page.closed
        || page.crashed
        || page.visible
        || page.warm
        || page.standalone
        || page.reveal.active()
        || in_split
    {
        return false;
    }
    let Some(tab) = page.tab.filter(|tab| !tab.closed() && !tab.discarded()) else {
        return false;
    };
    let url = tab.url().to_string();
    if !["https:", "http:", "file:"]
        .iter()
        .any(|prefix| url.starts_with(prefix))
    {
        return false;
    }
    others
        .iter()
        .find(|other| {
            !ptr::eq(other.host, page.host) && other.visible && windows_match(other.host, page.host)
        })
        .and_then(|other| other.tab)
        .is_some_and(|shown| same_profile(tab, shown))
}

fn pane_pair(front: &NSView, back: &NSView) -> Option<(Retained<NSView>, Retained<NSView>)> {
    let mut ancestor = superview(front);
    let mut front_pane = objc2::Message::retain(front);
    while let Some(shared) = ancestor {
        let mut back_pane = objc2::Message::retain(back);
        while let Some(parent) = superview(&back_pane) {
            if ptr::eq(&*parent, &*shared) {
                return (!ptr::eq(&*front_pane, &*back_pane)).then_some((front_pane, back_pane));
            }
            back_pane = parent;
        }
        front_pane = shared;
        ancestor = superview(&front_pane);
    }
    None
}

pub fn revealable_over<'a>(
    page: &Page<'_>,
    others: &'a [Page<'a>],
    past_other_reveals: bool,
) -> Option<&'a Page<'a>> {
    if page.visible || page.warm || page.standalone || page.closed || page.reveal.active() {
        return None;
    }
    let tab = page.tab?;
    let window = page.host.window()?;
    if !tab.view().is_some_and(|view| mounted(&view, page.host)) {
        return None;
    }
    let mut shown = None;
    for other in others
        .iter()
        .filter(|p| !ptr::eq(p.host, page.host) && same_window(p.host, &window))
    {
        if (other.warm && !other.asleep) || (other.reveal.active() && !past_other_reveals) {
            return None;
        }
        if other.visible && shown.replace(other).is_some() {
            return None;
        }
    }
    let shown = shown.filter(|shown| {
        shown.host.alphaValue() >= 1.0 && (shown.reveal.front.is_none() || past_other_reveals)
    })?;
    let frame = page.host.convertRect_toView(page.host.bounds(), None);
    if frame.size.width <= 0.0
        || frame.size.height <= 0.0
        || frame != shown.host.convertRect_toView(shown.host.bounds(), None)
    {
        return None;
    }
    let (front, back) = pane_pair(page.host, shown.host)?;
    (!front.isHiddenOrHasHiddenAncestor() && front.alphaValue() >= 1.0 && back.layer().is_some())
        .then_some(shown)
}

pub fn holds_window(page: &NSView, exclusive_frames: bool, fullscreen_page: bool) -> bool {
    let Some(window) = page.window() else {
        return false;
    };
    if exclusive_frames || fullscreen_page || window.attachedSheet().is_some() {
        return true;
    }
    let on_screen = window.convertRectToScreen(page.convertRect_toView(page.bounds(), None));
    window.childWindows().is_some_and(|children| {
        children.iter().any(|child| {
            if !child.isVisible() {
                return false;
            }
            let frame = child.frame();
            frame.origin.x < on_screen.origin.x + on_screen.size.width
                && on_screen.origin.x < frame.origin.x + frame.size.width
                && frame.origin.y < on_screen.origin.y + on_screen.size.height
                && on_screen.origin.y < frame.origin.y + frame.size.height
        })
    })
}

impl Reveal {
    pub fn active(&self) -> bool {
        self.lowered.is_some()
    }
    pub fn native(&self) -> bool {
        self.native
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn press(&self) -> Option<PressId> {
        self.press
    }

    pub fn note_press(&mut self, press: PressId) {
        if self.native {
            self.press = Some(press);
        }
    }

    pub fn mark_painted(&mut self) -> bool {
        self.native && !std::mem::replace(&mut self.painted, true)
    }

    pub fn begin(
        state: &Rc<RefCell<Self>>,
        host: &NSView,
        shown: &NSView,
        press: Option<PressId>,
        expose: impl FnOnce(),
        expired: impl FnOnce() + 'static,
    ) -> bool {
        if state.borrow().active() {
            return false;
        }
        let Some((_, pane)) = pane_pair(host, shown) else {
            return false;
        };
        let Some(layer) = pane.layer() else {
            return false;
        };
        expose();
        let generation = {
            let mut state = state.borrow_mut();
            // SAFETY: CALayer's zPosition returns a CGFloat.
            let z: f64 = unsafe { msg_send![&*layer, zPosition] };
            state.lowered = Some((Weak::from(&*pane), z));
            // SAFETY: CALayer's setZPosition: takes a CGFloat and returns void.
            let _: () = unsafe { msg_send![&*layer, setZPosition: z - 1.0] };
            state.native = press.is_some();
            state.press = press;
            state.generation = state.generation.wrapping_add(1);
            state.generation
        };
        let weak = Rc::downgrade(state);
        main_queue::after(
            BACKSTOP,
            Box::new(move || {
                if weak.upgrade().is_some_and(|state| {
                    state.borrow().generation == generation && state.borrow().active()
                }) {
                    expired();
                }
            }),
        );
        true
    }

    pub fn stay_over(&mut self, host: &NSView, shown: &NSView) -> bool {
        if !self.active()
            || host.convertRect_toView(host.bounds(), None)
                != shown.convertRect_toView(shown.bounds(), None)
        {
            return false;
        }
        let Some((_, pane)) = pane_pair(host, shown) else {
            return false;
        };
        let Some(layer) = pane.layer() else {
            return false;
        };
        self.restore_z();
        // SAFETY: CALayer's zPosition returns a CGFloat.
        let z: f64 = unsafe { msg_send![&*layer, zPosition] };
        self.lowered = Some((Weak::from(&*pane), z));
        // SAFETY: CALayer's setZPosition: takes a CGFloat and returns void.
        let _: () = unsafe { msg_send![&*layer, setZPosition: z - 1.0] };
        true
    }

    fn restore_z(&mut self) {
        if let Some((pane, z)) = self.lowered.take()
            && let Some(layer) = pane.load().and_then(|pane| pane.layer())
        {
            // SAFETY: CALayer's setZPosition: takes a CGFloat and returns void.
            let _: () = unsafe { msg_send![&*layer, setZPosition: z] };
        }
    }

    pub fn end(&mut self) -> Option<EndedReveal> {
        if !self.active() {
            return None;
        }
        self.restore_z();
        self.generation = self.generation.wrapping_add(1);
        Some(EndedReveal {
            native: std::mem::take(&mut self.native),
            painted: std::mem::take(&mut self.painted),
            press: self.press.take(),
        })
    }

    pub fn front(
        state: &Rc<RefCell<Self>>,
        host: &NSView,
        shown: &Tab,
        offered: &Tab,
        only_shown: bool,
        devtools: bool,
        registered: bool,
    ) -> bool {
        // `revealOffered:` marks the offer (`ArdiaWebView+Reveals.mm:309`) before `frontPage:profile:` refuses
        // (`ArdiaWebView+Reveals.mm:336`), so a refusal from DevTools or the shown tab leaves the mark.
        let Some(page) = offered.view() else {
            return false;
        };
        if host.window().is_none() || offered.closed() || superview(&page).is_some() || registered {
            return false;
        }
        offered.note_in_shown_slot();
        if state.borrow().front.is_some()
            || !only_shown
            || devtools
            || !same_profile(shown, offered)
        {
            return false;
        }
        let Some(mine) = shown.view() else {
            return false;
        };
        if !mounted(&mine, host) || mine.isHidden() {
            return false;
        }
        page.setFrame(host.bounds());
        page.setHidden(false);
        host.addSubview_positioned_relativeTo(&page, NSWindowOrderingMode::Above, Some(&mine));
        let generation = {
            let mut state = state.borrow_mut();
            state.front = Some(page);
            state.front_generation = state.front_generation.wrapping_add(1);
            state.front_generation
        };
        let weak = Rc::downgrade(state);
        let host = Weak::from(host);
        main_queue::after(
            BACKSTOP,
            Box::new(move || {
                if let Some(state) = weak.upgrade()
                    && let Some(host) = host.load()
                {
                    let mut state = state.borrow_mut();
                    if state.front_generation == generation {
                        state.drop_front(&host);
                    }
                }
            }),
        );
        true
    }

    pub fn drop_front(&mut self, host: &NSView) {
        self.front_generation = self.front_generation.wrapping_add(1);
        if let Some(page) = self.front.take().filter(|page| mounted(page, host)) {
            page.removeFromSuperview();
        }
    }
}

pub fn only_shown_in_window(page: &Page<'_>, others: &[Page<'_>]) -> bool {
    page.visible
        && !page.warm
        && !page.reveal.active()
        && !page.standalone
        && !page.closed
        && page.host.alphaValue() >= 1.0
        && !page.host.isHiddenOrHasHiddenAncestor()
        && page.host.window().is_some_and(|window| {
            !others.iter().any(|other| {
                !ptr::eq(other.host, page.host)
                    && same_window(other.host, &window)
                    && (other.visible || other.warm || other.reveal.active())
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_real_older_press_defers_to_a_later_reveal() {
        assert!(!older_press(None, PressId::new(2)));
        assert!(!older_press(PressId::new(2), PressId::new(2)));
        assert!(older_press(PressId::new(1), PressId::new(2)));
    }
}
