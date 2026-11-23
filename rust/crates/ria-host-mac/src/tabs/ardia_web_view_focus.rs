// Ported from ardia packages/ardiacore/ios/ArdiaWebView+Focus.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::time::{Duration, Instant};

use objc2::rc::{Retained, Weak};
use objc2_app_kit::{NSResponder, NSView, NSWindow};

#[derive(Default)]
enum Remembered {
    #[default]
    Unknown,
    Page,
    DevTools(Weak<NSView>),
    Elsewhere(Option<Weak<NSView>>),
}

#[derive(Default)]
pub struct PageFocus {
    remembered: Remembered,
    pending: Option<(Weak<NSWindow>, Instant)>,
    keeps_outgoing: bool,
}

pub struct FocusSurface<'a> {
    pub host: &'a NSView,
    pub page: &'a NSView,
    pub dev_tools: Option<&'a NSView>,
    pub visible: bool,
    pub selected: bool,
    pub standalone: bool,
}

impl FocusSurface<'_> {
    pub fn can_focus(&self) -> bool {
        self.visible
            && !self.standalone
            && self.attached()
            && !self.page.isHiddenOrHasHiddenAncestor()
    }

    fn attached(&self) -> bool {
        super::superview(self.page).is_some_and(|parent| std::ptr::eq(&*parent, self.host))
    }
}

pub enum RestoreFocus {
    None,
    Waiting,
    Page,
    DevTools(Retained<NSView>),
}

pub fn focus_page(
    tab: &crate::core::bindings::Tab,
    on_screen_ahead: bool,
    veiled: bool,
    ask_ahead: impl FnOnce(bool),
    request_veiled: impl FnOnce(),
) {
    if on_screen_ahead {
        ask_ahead(false);
    } else if veiled {
        request_veiled();
    } else {
        tab.focus();
    }
}

pub fn focus_page_if_unfocused(
    host: &NSView,
    on_screen_ahead: bool,
    ask_ahead: impl FnOnce(bool),
    focus: impl FnOnce(),
) -> bool {
    let Some(window) = host.window() else {
        return false;
    };
    if !unfocused(window.firstResponder().as_deref(), &window) {
        return false;
    }
    if on_screen_ahead {
        ask_ahead(true);
        return false;
    }
    focus();
    true
}

fn view_of(responder: &NSResponder) -> Option<&NSView> {
    use objc2::runtime::AnyObject;
    let object: &AnyObject = responder;
    object.downcast_ref::<NSView>()
}

fn unfocused(responder: Option<&NSResponder>, window: &NSWindow) -> bool {
    responder.is_none_or(|responder| std::ptr::eq(responder, &**window))
}

impl PageFocus {
    pub fn remember_outgoing(
        &mut self,
        surface: &FocusSurface<'_>,
        responder: &NSResponder,
        in_tab_page: impl FnOnce(&NSView) -> bool,
    ) {
        self.store_from(surface, Some(responder), in_tab_page);
        self.keeps_outgoing = true;
    }

    pub fn end_outgoing(&mut self) {
        self.keeps_outgoing = false;
    }

    pub fn store(&mut self, surface: &FocusSurface<'_>, in_tab_page: impl FnOnce(&NSView) -> bool) {
        let saved = std::mem::take(&mut self.keeps_outgoing);
        let window = surface.host.window();
        let responder = window.as_ref().and_then(|window| window.firstResponder());
        if saved
            && window
                .as_ref()
                .is_none_or(|window| unfocused(responder.as_deref(), window))
        {
            self.pending = None;
        } else {
            self.store_from(surface, responder.as_deref(), in_tab_page);
        }
    }

    fn store_from(
        &mut self,
        surface: &FocusSurface<'_>,
        responder: Option<&NSResponder>,
        in_tab_page: impl FnOnce(&NSView) -> bool,
    ) {
        self.pending = None;
        let Some(window) = surface.host.window() else {
            return;
        };
        let view = responder.and_then(view_of);
        self.remembered = match view {
            Some(view) if view.isDescendantOf(surface.page) => Remembered::Page,
            Some(view)
                if surface
                    .dev_tools
                    .is_some_and(|dev_tools| view.isDescendantOf(dev_tools)) =>
            {
                Remembered::DevTools(Weak::new(view))
            }
            _ if unfocused(responder, &window) || view.is_some_and(in_tab_page) => {
                Remembered::Unknown
            }
            _ => Remembered::Elsewhere(view.map(Weak::new)),
        };
    }

    pub fn prefers_page(&self) -> bool {
        matches!(self.remembered, Remembered::Page)
    }

    pub fn request(&mut self, window: &NSWindow, now: Instant) {
        self.pending = Some((Weak::new(window), now));
    }

    pub fn after_load(&mut self, surface: &FocusSurface<'_>, now: Instant) -> RestoreFocus {
        if surface.visible {
            return if surface.selected {
                RestoreFocus::Page
            } else {
                RestoreFocus::None
            };
        }
        self.remembered = Remembered::Page;
        if !surface.standalone
            && let Some(window) = surface.host.window()
        {
            self.request(&window, now);
            return RestoreFocus::Waiting;
        }
        RestoreFocus::None
    }

    pub fn restore_if_due(
        &mut self,
        surface: &FocusSurface<'_>,
        now: Instant,
        ahead: bool,
        take_lingering_focus: impl FnOnce(),
        in_tab_page: impl FnOnce(&NSView) -> bool,
    ) -> RestoreFocus {
        let Some((activated_in, activated_at)) = self.pending.as_ref() else {
            return RestoreFocus::None;
        };
        let Some(window) = surface.host.window().filter(|window| {
            !surface.standalone
                && surface.attached()
                && now.saturating_duration_since(*activated_at) <= Duration::from_secs(1)
                && activated_in
                    .load()
                    .is_some_and(|old| std::ptr::eq(&*old, &**window))
        }) else {
            self.pending = None;
            return RestoreFocus::None;
        };
        if !surface.can_focus() || !surface.selected || ahead {
            return RestoreFocus::Waiting;
        }
        self.pending = None;
        take_lingering_focus();
        let responder = window.firstResponder();
        let view = responder.as_deref().and_then(view_of);
        if view.is_some_and(|view| view.isDescendantOf(surface.host))
            || (!unfocused(responder.as_deref(), &window) && !view.is_some_and(in_tab_page))
        {
            return RestoreFocus::None;
        }
        if let Remembered::Elsewhere(Some(view)) = &self.remembered
            && view
                .load()
                .and_then(|view| view.window())
                .is_some_and(|other| std::ptr::eq(&*other, &*window))
        {
            return RestoreFocus::None;
        }
        self.remembered_destination(surface, &window)
    }

    pub fn after_lingering(&self, surface: &FocusSurface<'_>, ahead: bool) -> RestoreFocus {
        if let Some(window) = surface
            .host
            .window()
            .filter(|_| surface.can_focus() && surface.selected && !ahead)
        {
            return self.remembered_destination(surface, &window);
        }
        RestoreFocus::None
    }

    fn remembered_destination(
        &self,
        surface: &FocusSurface<'_>,
        window: &NSWindow,
    ) -> RestoreFocus {
        if let Remembered::DevTools(view) = &self.remembered
            && let Some(view) = view.load().filter(|view| {
                view.window()
                    .is_some_and(|other| std::ptr::eq(&*other, window))
                    && surface
                        .dev_tools
                        .is_some_and(|dev_tools| view.isDescendantOf(dev_tools))
            })
        {
            return RestoreFocus::DevTools(view);
        }
        RestoreFocus::Page
    }
}
