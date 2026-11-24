// Ported from ardia packages/ardiacore/ios/ArdiaWebView+Peek.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::RefCell;
use std::collections::HashSet;
use std::ptr;
use std::rc::Rc;
use std::time::{Duration, Instant};

use objc2::MainThreadMarker;
use objc2::rc::{Retained, Weak};
use objc2_app_kit::{
    NSEvent, NSEventMask, NSEventModifierFlags as Modifiers, NSEventType, NSView, NSWindow,
};
use objc2_foundation::{NSPoint, NSRect, NSString, NSURL};
use serde_json::{Value, json};

use crate::core::bindings::Tab;

use super::ardia_web_view_input_monitors::EventMonitor;
use super::ardia_web_view_page_messages::call_frame;
use super::ardia_web_view_painting::render_widget_in;
use super::superview;

const GESTURE_LIFETIME: Duration = Duration::from_secs(10);
const FALLBACK_LIFETIME: Duration = Duration::from_secs(60);
const SHIFT_LIFETIME: Duration = Duration::from_secs(3);
const ACTIVATION_LIFETIME: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum PeekModifier {
    Off,
    Alt,
    #[default]
    Shift,
}

impl PeekModifier {
    fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Alt => "alt",
            Self::Shift => "shift",
        }
    }
}

#[derive(Default)]
pub struct Peek {
    modifier: PeekModifier,
    split_alt: bool,
    other_sites: HashSet<String>,
    mouse_up: Option<MouseUp>,
    input: Vec<(Weak<NSWindow>, Instant)>,
}

struct MouseUp {
    at: Instant,
    view: Option<Weak<NSView>>,
    keys: Modifiers,
    point: NSPoint,
    consumed: bool,
    left_pressed: bool,
    other_pressed: bool,
}

impl MouseUp {
    fn over(&self, view: &NSView) -> bool {
        self.view
            .as_ref()
            .and_then(Weak::load)
            .is_some_and(|up| ptr::eq(&*up, view))
    }

    fn verify(
        &self,
        view: &NSView,
        flags: Modifiers,
        now: Instant,
        max_age: Duration,
    ) -> Result<(), &'static str> {
        if self.consumed {
            return Err("that mouse-up already opened a peek");
        }
        if self.left_pressed {
            return Err("the mouse was pressed again since that mouse-up");
        }
        if now.saturating_duration_since(self.at) > max_age {
            return Err("the last mouse-up is too old");
        }
        if !self.over(view) {
            return Err("the mouse-up wasn't over this page");
        }
        if self.keys != flags {
            return Err("the mouse-up's keys aren't the page's key alone");
        }
        Ok(())
    }
}

pub enum PeekReply {
    Open(Value),
    Refuse(Value),
    Ignore,
}

pub struct Link<'a> {
    pub host: &'a NSView,
    pub tab: Option<&'a Tab>,
    pub transfer_key: &'a str,
    pub visible: bool,
    pub main: bool,
    pub test: bool,
}

impl Peek {
    pub fn install(
        mtm: MainThreadMarker,
        state: Rc<RefCell<Self>>,
        page_containing: impl Fn(&NSView) -> Option<Retained<NSView>> + 'static,
    ) -> Option<EventMonitor> {
        let mask = NSEventMask::LeftMouseDown
            | NSEventMask::LeftMouseUp
            | NSEventMask::RightMouseDown
            | NSEventMask::OtherMouseDown
            | NSEventMask::OtherMouseUp
            | NSEventMask::KeyDown;
        EventMonitor::install(mtm, mask, move |event| {
            let page = (event.r#type() == NSEventType::LeftMouseUp)
                .then(|| hit(event).and_then(|hit| page_containing(&hit)))
                .flatten();
            state
                .borrow_mut()
                .observe(event, page.as_deref(), Instant::now());
            false
        })
    }

    pub fn set_modifier(&mut self, value: &str) -> bool {
        let next = match value {
            "alt" => PeekModifier::Alt,
            "shift" => PeekModifier::Shift,
            _ => PeekModifier::Off,
        };
        let changed = self.modifier != next;
        self.modifier = next;
        changed
    }

    pub fn set_split_modifier(&mut self, value: &str) -> bool {
        let next = value == "alt";
        let changed = self.split_alt != next;
        self.split_alt = next;
        changed
    }

    pub fn set_other_site_tabs(&mut self, tabs: HashSet<String>) -> Vec<String> {
        let changed = tabs
            .symmetric_difference(&self.other_sites)
            .cloned()
            .collect();
        self.other_sites = tabs;
        changed
    }

    pub fn split_modifier(&self) -> &'static str {
        if self.split_alt && self.modifier != PeekModifier::Alt {
            "alt"
        } else {
            "altShift"
        }
    }

    pub fn config(&self, key: &str, trace: bool) -> Value {
        json!({"modifier": self.modifier.as_str(), "split": self.split_modifier(), "trace": trace, "otherSites": self.peeks_other_sites(key)})
    }

    pub fn peeks_other_sites(&self, key: &str) -> bool {
        !key.is_empty() && self.other_sites.contains(key)
    }

    pub fn push_config(
        &self,
        tab: &Tab,
        key: &str,
        trace: bool,
        frames: impl IntoIterator<Item = String>,
    ) {
        let config = self.config(key, trace).to_string();
        call_frame(tab, "", "peekConfig", &config);
        for frame in frames {
            call_frame(tab, &frame, "peekConfig", &config);
        }
    }

    fn observe(&mut self, event: &NSEvent, page: Option<&NSView>, now: Instant) {
        if let Some(window) = MainThreadMarker::new().and_then(|mtm| event.window(mtm)) {
            self.input.retain(|(other, _)| {
                other
                    .load()
                    .is_some_and(|other| !ptr::eq(&*other, &*window))
            });
            self.input.push((Weak::from(&*window), now));
        }
        match event.r#type() {
            NSEventType::LeftMouseUp => {
                let mask =
                    Modifiers::Shift | Modifiers::Option | Modifiers::Control | Modifiers::Command;
                self.mouse_up = Some(MouseUp {
                    at: now,
                    view: page.map(Weak::from),
                    keys: event.modifierFlags() & mask,
                    point: event.locationInWindow(),
                    consumed: false,
                    left_pressed: false,
                    other_pressed: false,
                });
            }
            NSEventType::LeftMouseDown => {
                if let Some(up) = &mut self.mouse_up {
                    up.left_pressed = true;
                }
            }
            NSEventType::RightMouseDown | NSEventType::OtherMouseDown => {
                if let Some(up) = &mut self.mouse_up {
                    up.other_pressed = true;
                }
            }
            _ => {}
        }
    }

    pub fn recent_click(&self, host: &NSView, now: Instant) -> Option<NSPoint> {
        let window = host.window()?;
        self.mouse_up
            .as_ref()
            .filter(|up| up.over(host) && now.saturating_duration_since(up.at) <= GESTURE_LIFETIME)
            .map(|up| {
                crate::windows::ardia_interface_scale::root_point_from_window(up.point, &window)
            })
    }

    pub fn take_shift_click(&mut self, host: &NSView, now: Instant) -> bool {
        let Some(up) = self.mouse_up.as_mut().filter(|up| {
            !up.other_pressed
                && up
                    .verify(host, Modifiers::Shift, now, SHIFT_LIFETIME)
                    .is_ok()
        }) else {
            return false;
        };
        up.consumed = true;
        true
    }

    pub fn may_hold_activation(&self, window: &NSWindow, now: Instant) -> bool {
        self.input.iter().any(|(stored, at)| {
            now.saturating_duration_since(*at) <= ACTIVATION_LIFETIME
                && stored
                    .load()
                    .is_some_and(|stored| ptr::eq(&*stored, window))
        })
    }

    fn action(&self, key: &str, flags: Modifiers, transfer_key: &str) -> Option<&'static str> {
        if key == "site" {
            return self.peeks_other_sites(transfer_key).then_some("peek");
        }
        if flags.is_empty() {
            return None;
        }
        if key == self.modifier.as_str() {
            return Some("peek");
        }
        (matches!(key, "altShift" | "altMeta") || key == self.split_modifier()).then_some("split")
    }

    pub fn link(
        &mut self,
        link: Link<'_>,
        message: &Value,
        now: Instant,
        root_point: impl FnOnce(&NSWindow, NSPoint) -> NSPoint,
    ) -> PeekReply {
        let url = text(message, "url", 8192);
        let key = text(message, "key", 8);
        let flags = flags(key);
        let other_site = key == "site";
        let action = self.action(key, flags, link.transfer_key);
        let window = link.host.window();
        let is_web = NSURL::URLWithString(&NSString::from_str(url))
            .and_then(|url| url.scheme())
            .is_some_and(|scheme| {
                matches!(
                    scheme.lowercaseString().to_string().as_str(),
                    "http" | "https"
                )
            });
        let decision = if action.is_none() {
            Err("that key opens nothing now")
        } else if other_site && !link.main {
            Err("only the page itself peeks links to other sites")
        } else if !link.visible || window.is_none() {
            Err("page not shown")
        } else if !is_web {
            Err("not http(s)")
        } else if link.test {
            Ok(())
        } else {
            self.mouse_up
                .as_ref()
                .ok_or("no left mouse-up yet")
                .and_then(|up| up.verify(link.host, flags, now, GESTURE_LIFETIME))
        };
        if let Err(reason) = decision {
            let click = link.test
                || self
                    .mouse_up
                    .as_ref()
                    .is_some_and(|up| up.verify(link.host, flags, now, FALLBACK_LIFETIME).is_ok());
            let result = fallback(
                url,
                key,
                flags,
                action != Some("split") && is_web && click,
                reason,
                text(message, "origin", 1024),
            );
            if !other_site
                && click
                && (action == Some("split") || !matches!(result, PeekReply::Ignore))
            {
                self.consume(link.test);
            }
            return result;
        }
        self.consume(link.test);
        if action == Some("split") {
            return open(url, "split");
        }
        let window = window.expect("the verified page has a window");
        let point = point_for_link(&link, &window, message);
        let point = root_point(&window, link.host.convertPoint_toView(point, None));
        PeekReply::Open(
            json!({"url": url, "disposition": "peek", "userGesture": true, "site": other_site, "point": {"x": point.x, "y": point.y}}),
        )
    }

    fn consume(&mut self, test: bool) {
        if !test && let Some(up) = &mut self.mouse_up {
            up.consumed = true;
        }
    }
}

fn flags(key: &str) -> Modifiers {
    match key {
        "alt" => Modifiers::Option,
        "shift" => Modifiers::Shift,
        "altShift" => Modifiers::Option | Modifiers::Shift,
        "altMeta" => Modifiers::Option | Modifiers::Command,
        _ => Modifiers::empty(),
    }
}

fn hit(event: &NSEvent) -> Option<Retained<NSView>> {
    let content = event.window(MainThreadMarker::new()?)?.contentView()?;
    let root = superview(&content).unwrap_or(content);
    root.hitTest(root.convertPoint_fromView(event.locationInWindow(), None))
}

fn text<'a>(value: &'a Value, key: &str, max: usize) -> &'a str {
    value[key]
        .as_str()
        .filter(|text| text.encode_utf16().take(max + 1).count() <= max)
        .unwrap_or_default()
}

fn fallback(
    url: &str,
    key: &str,
    flags: Modifiers,
    can_open: bool,
    reason: &str,
    origin: &str,
) -> PeekReply {
    if key == "site" {
        return PeekReply::Refuse(json!({"url": url, "reason": reason, "follow": true}));
    }
    if flags.is_empty() || !can_open {
        return PeekReply::Ignore;
    }
    if flags == Modifiers::Shift {
        return open(url, "window");
    }
    if !origin.is_empty()
        && Tab::origin_for_url(&NSString::from_str(url))
            .is_some_and(|actual| actual.to_string() == origin.to_lowercase())
    {
        return PeekReply::Refuse(json!({"url": url, "reason": reason, "download": true}));
    }
    open(url, "background")
}

fn open(url: &str, disposition: &str) -> PeekReply {
    PeekReply::Open(json!({"url": url, "disposition": disposition, "userGesture": true}))
}

fn point_for_link(link: &Link<'_>, window: &NSWindow, message: &Value) -> NSPoint {
    let area = link
        .tab
        .map(Tab::view)
        .filter(|page| superview(page).is_some_and(|host| ptr::eq(&*host, link.host)))
        .map_or_else(|| link.host.bounds(), |page| page.frame());
    let NSRect { origin, size } = area;
    let number = |key: &str| message[key].as_f64().filter(|n| n.is_finite());
    let point = if link.main
        && let (Some(x), Some(y)) = (number("x"), number("y"))
    {
        let scale = window.backingScaleFactor();
        let scale = if scale > 0.0 { scale } else { 1.0 };
        NSPoint::new(origin.x + x / scale, origin.y + y / scale)
    } else {
        link.host.convertPoint_fromView(
            window.convertPointFromScreen(NSEvent::mouseLocation()),
            None,
        )
    };
    NSPoint::new(
        point.x.max(origin.x).min(origin.x + size.width),
        point.y.max(origin.y).min(origin.y + size.height),
    )
}

pub fn shows_painted_frame(host: &NSView, tab: Option<&Tab>) -> bool {
    let Some(page) = tab.map(Tab::view).filter(|page| {
        !page.isHidden() && superview(page).is_some_and(|parent| ptr::eq(&*parent, host))
    }) else {
        return false;
    };
    render_widget_in(&page)
        .filter(|widget| !widget.isHidden())
        .and_then(|widget| widget.layer())
        // SAFETY: The render tree is read on the AppKit thread without mutation.
        .and_then(|layer| unsafe { layer.sublayers() })
        .is_some_and(|layers| {
            layers.iter().any(|layer| {
                // SAFETY: The child layers belong to the same AppKit render tree.
                unsafe { layer.sublayers() }.is_some_and(|children| children.count() > 0)
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alt_is_reserved_for_peek_when_the_modifiers_conflict() {
        let mut state = Peek::default();
        state.set_split_modifier("alt");
        assert_eq!(state.action("alt", flags("alt"), ""), Some("split"));
        state.set_modifier("alt");
        assert_eq!(state.action("alt", flags("alt"), ""), Some("peek"));
        assert_eq!(state.split_modifier(), "altShift");
        assert_eq!(
            state.action("altShift", flags("altShift"), ""),
            Some("split")
        );
    }
}
