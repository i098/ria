// Ported from ardia packages/ardiacore/ios/ArdiaWebView+RevealInput.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

// Also ports the input ledgers from ArdiaRevealLedger.h at the same commit.

use std::ptr;
use std::time::Duration;

use objc2::MainThreadMarker;
use objc2::rc::{Retained, Weak};
use objc2_app_kit::{
    NSEvent, NSEventMask, NSEventModifierFlags, NSEventType, NSResponder, NSView, NSWindow,
};
use ria_engine::call::main_queue;

use crate::core::bindings::Tab;

use super::ardia_web_view_stand_in::without_actions;

const BUTTON_DOWNS: NSEventMask = NSEventMask::LeftMouseDown
    .union(NSEventMask::RightMouseDown)
    .union(NSEventMask::OtherMouseDown);
const BUTTON_UPS: NSEventMask = NSEventMask::LeftMouseUp
    .union(NSEventMask::RightMouseUp)
    .union(NSEventMask::OtherMouseUp);
const BUTTON_DRAGS: NSEventMask = NSEventMask::LeftMouseDragged
    .union(NSEventMask::RightMouseDragged)
    .union(NSEventMask::OtherMouseDragged);
const BUTTONS: NSEventMask = BUTTON_DOWNS.union(BUTTON_UPS).union(BUTTON_DRAGS);
const KEYS: NSEventMask = NSEventMask::KeyDown
    .union(NSEventMask::KeyUp)
    .union(NSEventMask::FlagsChanged);
const POINTER: NSEventMask = NSEventMask::MouseMoved
    .union(NSEventMask::ScrollWheel)
    .union(NSEventMask::Magnify)
    .union(NSEventMask::SmartMagnify)
    .union(NSEventMask::Swipe)
    .union(NSEventMask::Rotate);
pub const REVEAL_INPUT_MASK: NSEventMask = BUTTONS.union(KEYS).union(POINTER);

fn modifier_is_down(code: u16, flags: usize) -> bool {
    let (own, paired, either) = match code {
        55 => (0x8, 0x10, 1 << 20),
        54 => (0x10, 0x8, 1 << 20),
        56 => (0x2, 0x4, 1 << 17),
        60 => (0x4, 0x2, 1 << 17),
        58 => (0x20, 0x40, 1 << 19),
        61 => (0x40, 0x20, 1 << 19),
        59 => (0x1, 0x2000, 1 << 18),
        62 => (0x2000, 0x1, 1 << 18),
        57 => (0, 0, 1 << 16),
        63 => (0, 0, 1 << 23),
        _ => (0, 0, 0),
    };
    let either = flags & either != 0;
    let own = flags & own != 0;
    let paired = flags & paired != 0;
    if either == (own || paired) {
        own
    } else {
        either
    }
}

pub fn skips_event_monitors(event: &NSEvent) -> bool {
    let kind = event.r#type();
    matches!(kind, NSEventType::KeyDown | NSEventType::KeyUp)
        && skips_key(kind, event.keyCode(), event.modifierFlags())
}

fn skips_key(kind: NSEventType, code: u16, flags: NSEventModifierFlags) -> bool {
    match kind {
        NSEventType::KeyDown => matches!(code, 102 | 104),
        NSEventType::KeyUp => flags.contains(NSEventModifierFlags::Command),
        _ => false,
    }
}

#[derive(Clone, Copy, Default)]
struct Key {
    delivered: bool,
    swallowed: bool,
}

impl Key {
    fn update(&mut self, down: bool, repeat: bool, ahead: bool, same_holder: bool) -> bool {
        if !down {
            let old = std::mem::take(self);
            return if old.delivered {
                old.swallowed && !same_holder
            } else {
                old.swallowed || ahead
            };
        }
        if repeat {
            self.swallowed |= ahead;
            self.delivered |= !self.swallowed;
        } else {
            self.swallowed = ahead;
            self.delivered = !ahead;
        }
        self.swallowed
    }
}

#[derive(Default)]
struct Buttons {
    windows: [Option<isize>; 32],
}

impl Buttons {
    fn update(&mut self, button: isize, kind: NSEventMask, window: isize, covered: bool) -> bool {
        let held = &mut self.windows[(button & 31) as usize];
        if BUTTON_DOWNS.intersects(kind) {
            *held = covered.then_some(window);
            return covered;
        }
        if *held != Some(window) {
            return false;
        }
        if BUTTON_UPS.intersects(kind) {
            *held = None;
        }
        true
    }
}

struct Gap {
    host: Weak<NSView>,
    window: Weak<NSWindow>,
}

struct Hidden {
    window: Weak<NSWindow>,
    views: Vec<(Weak<NSView>, f64)>,
}

pub struct RevealInput {
    keys: [Key; 256],
    key_holder: Option<Weak<NSResponder>>,
    buttons: Buttons,
    gaps: Vec<Gap>,
    hidden: Vec<Hidden>,
}

impl Default for RevealInput {
    fn default() -> Self {
        Self {
            keys: [Key::default(); 256],
            key_holder: None,
            buttons: Buttons::default(),
            gaps: Vec::new(),
            hidden: Vec::new(),
        }
    }
}

impl RevealInput {
    /// `+[ArdiaWebView anyRevealGap]`.
    pub fn any_gap(&self) -> bool {
        self.gaps.iter().any(|gap| gap.host.load().is_some())
    }

    pub fn gap_in(&self, window: &NSWindow) -> bool {
        self.gaps.iter().any(|gap| {
            gap.host.load().is_some()
                && gap
                    .window
                    .load()
                    .is_some_and(|stored| ptr::eq(&*stored, window))
        })
    }

    pub fn refuses_view(&self, view: &NSView, in_tab_page: impl FnOnce(&NSView) -> bool) -> bool {
        view.window().is_some_and(|window| self.gap_in(&window)) && in_tab_page(view)
    }

    pub fn enter(
        &mut self,
        host: &NSView,
        native: bool,
        painted: bool,
        over_another: bool,
    ) -> bool {
        if !native
            || !(painted || over_another)
            || self.gaps.iter().any(|gap| {
                gap.host
                    .load()
                    .is_some_and(|stored| ptr::eq(&*stored, host))
            })
        {
            return false;
        }
        let Some(window) = host.window() else {
            return false;
        };
        self.gaps.push(Gap {
            host: Weak::from(host),
            window: Weak::from(&*window),
        });
        true
    }

    pub fn leave(&mut self, host: &NSView) -> Option<Retained<NSWindow>> {
        let index = self.gaps.iter().position(|gap| {
            gap.host
                .load()
                .is_some_and(|stored| ptr::eq(&*stored, host))
        })?;
        let window = self.gaps.swap_remove(index).window.load()?;
        if self.gap_in(&window) {
            return None;
        }
        self.restore_identity(&window);
        Some(window)
    }

    pub fn leave_later(generation: u64, callback: impl FnOnce(u64) + 'static) {
        main_queue::after(
            Duration::from_millis(500),
            Box::new(move || callback(generation)),
        );
    }

    pub fn hide_identity(&mut self, window: &NSWindow, identity: impl Fn(&NSView) -> bool) {
        let Some(content) = window.contentView() else {
            return;
        };
        let mut views = Vec::new();
        collect_identity(&content, &identity, &mut views);
        let mut index = self.hidden.iter().position(|hidden| {
            hidden
                .window
                .load()
                .is_some_and(|stored| ptr::eq(&*stored, window))
        });
        let hidden = if let Some(index) = index.take() {
            &mut self.hidden[index]
        } else {
            self.hidden.push(Hidden {
                window: Weak::from(window),
                views: Vec::new(),
            });
            self.hidden.last_mut().expect("the window was added")
        };
        without_actions(|| {
            for view in views {
                if !hidden
                    .views
                    .iter()
                    .any(|(old, _)| old.load().is_some_and(|old| ptr::eq(&*old, &*view)))
                {
                    hidden.views.push((Weak::from(&*view), view.alphaValue()));
                }
                view.setAlphaValue(0.0);
            }
        });
    }

    fn restore_identity(&mut self, window: &NSWindow) {
        if let Some(index) = self.hidden.iter().position(|hidden| {
            hidden
                .window
                .load()
                .is_some_and(|old| ptr::eq(&*old, window))
        }) {
            let hidden = self.hidden.swap_remove(index);
            without_actions(|| {
                for (view, alpha) in hidden.views {
                    if let Some(view) = view.load() {
                        view.setAlphaValue(alpha);
                    }
                }
            });
        }
    }

    pub fn swallows(
        &mut self,
        event: &NSEvent,
        in_tab_page: impl Fn(&NSView) -> bool,
        app_wide_key: impl FnOnce(&NSEvent),
    ) -> bool {
        let Some(mtm) = MainThreadMarker::new() else {
            return false;
        };
        let Some(bit) = 1u64.checked_shl(event.r#type().0 as u32) else {
            return false;
        };
        let kind = NSEventMask::from_bits_retain(bit);
        if KEYS.intersects(kind) {
            return self.swallows_key(mtm, event, in_tab_page, app_wide_key);
        }
        if !REVEAL_INPUT_MASK.intersects(kind) {
            return false;
        }
        let button = BUTTONS.intersects(kind);
        let lands = !(BUTTON_UPS | BUTTON_DRAGS).intersects(kind);
        let covered = lands
            && event.window(mtm).is_some_and(|window| self.gap_in(&window))
            && super::ardia_web_view_reveals::hit_in_window(event)
                .is_some_and(|hit| in_tab_page(&hit));
        if button {
            self.buttons
                .update(event.buttonNumber(), kind, event.windowNumber(), covered)
        } else {
            covered
        }
    }

    fn swallows_key(
        &mut self,
        mtm: MainThreadMarker,
        event: &NSEvent,
        in_tab_page: impl Fn(&NSView) -> bool,
        app_wide_key: impl FnOnce(&NSEvent),
    ) -> bool {
        let code = event.keyCode();
        let modifier = event.r#type() == NSEventType::FlagsChanged;
        let down = if modifier {
            modifier_is_down(code, event.modifierFlags().bits())
        } else {
            event.r#type() == NSEventType::KeyDown
        };
        let repeat = !modifier && down && event.isARepeat();
        let window = event.window(mtm);
        let keyboard = window.as_ref().and_then(|window| window.firstResponder());
        let ahead = window.is_some_and(|window| self.gap_in(&window))
            && keyboard
                .as_ref()
                .and_then(|keyboard| keyboard.clone().downcast::<NSView>().ok())
                .is_some_and(|view| in_tab_page(&view));
        if ahead && repeat {
            self.key_holder = keyboard.as_deref().map(Weak::from);
        }
        let same_holder = keyboard.is_some_and(|keyboard| {
            self.key_holder
                .as_ref()
                .and_then(Weak::load)
                .is_some_and(|holder| ptr::eq(&*holder, &*keyboard))
        });
        let swallow = self.keys[usize::from(code & 255)].update(down, repeat, ahead, same_holder);
        if ahead && down && !repeat && !modifier {
            app_wide_key(event);
        }
        swallow
    }

    pub fn accepts_drops(
        &self,
        visible: bool,
        standalone: bool,
        window: Option<&NSWindow>,
    ) -> bool {
        visible && (standalone || window.is_none_or(|window| !self.gap_in(window)))
    }
}

fn collect_identity(
    root: &NSView,
    is_identity: &impl Fn(&NSView) -> bool,
    found: &mut Vec<Retained<NSView>>,
) {
    for view in root.subviews().iter() {
        if is_identity(&view) {
            found.push(view);
        } else {
            collect_identity(&view, is_identity, found);
        }
    }
}

#[derive(Default)]
pub struct DeferredFocus(Option<(Weak<Tab>, bool)>);

impl DeferredFocus {
    pub fn ask(&mut self, tab: &Tab, only_if_unfocused: bool) {
        let only_if_unfocused = only_if_unfocused && self.0.as_ref().is_none_or(|(_, only)| *only);
        self.0 = Some((Weak::from(tab), only_if_unfocused));
    }

    pub fn take(&mut self, tab: Option<&Tab>, gap: bool, unfocused: bool, in_page: bool) -> bool {
        if gap {
            return false;
        }
        self.0.take().is_some_and(|(asked, only)| {
            tab.is_some_and(|tab| asked.load().is_some_and(|asked| ptr::eq(&*asked, tab)))
                && (unfocused || (!only && in_page))
        })
    }
}

/// `-leavingScreenAhead` (RevealInput.mm 163-165): hidden, still in a window, and held for a frame.
pub fn leaving_screen_ahead(visible: bool, has_window: bool, holding_frame: bool) -> bool {
    !visible && has_window && holding_frame
}

/// `-settleOnScreenAhead` (RevealInput.mm 167-170): whether the page leaves the screen-ahead state now.
pub fn settles_screen_ahead(on_screen: bool, native_reveal: bool, leaving: bool) -> bool {
    on_screen && !native_reveal && !leaving
}

#[cfg(test)]
mod tests {
    use super::*;

    // Cases from packages/ardiacore/tests/reveal-ledger.test.cc.
    #[test]
    fn keys() {
        // (key, down, repeat, ahead, same holder, swallow)
        type Step = (usize, bool, bool, bool, bool, bool);
        let cases: &[&[Step]] = &[
            &[
                (0, true, false, false, false, false),
                (0, true, true, false, false, false),
                (0, false, false, false, false, false),
            ],
            &[
                (0, true, false, true, false, true),
                (0, true, true, true, false, true),
                (0, true, true, false, false, true),
                (0, false, false, false, false, true),
                (0, true, false, false, false, false),
                (0, false, false, false, false, false),
            ],
            &[
                (38, true, false, false, false, false),
                (38, false, false, true, false, false),
            ],
            &[
                (38, true, false, false, false, false),
                (38, true, true, true, true, true),
                (38, true, true, false, false, true),
                (38, true, true, false, false, true),
                (38, false, false, false, false, true),
                (38, true, false, false, false, false),
                (38, true, true, false, false, false),
                (38, false, false, false, false, false),
            ],
            &[
                (38, true, false, false, false, false),
                (38, true, true, true, true, true),
                (38, false, false, true, true, false),
            ],
            &[
                (38, true, false, false, false, false),
                (38, true, true, true, true, true),
                (38, false, false, false, true, false),
            ],
            &[
                (38, false, false, true, false, true),
                (38, true, true, true, false, true),
                (38, false, false, false, true, true),
            ],
            &[
                (0, true, false, true, false, true),
                (0, true, false, false, false, false),
                (0, true, true, false, false, false),
                (0, false, false, false, false, false),
            ],
            &[
                (0, true, false, true, false, true),
                (38, true, false, false, false, false),
                (0, false, false, false, false, true),
                (38, false, false, false, false, false),
            ],
            &[(300, true, false, false, false, false)],
        ];
        for (case, events) in cases.iter().enumerate() {
            let mut keys = RevealInput::default().keys;
            for &(code, down, repeat, ahead, same, expected) in *events {
                let key = &mut keys[code & 255];
                assert_eq!(
                    key.update(down, repeat, ahead, same),
                    expected,
                    "case {case}, key {code}"
                );
            }
            assert!(keys.iter().all(|key| !key.swallowed));
            assert_eq!(keys.iter().any(|key| key.delivered), case == 9);
        }
    }

    #[test]
    fn modifiers() {
        let shift = 1 << 17;
        let control = 1 << 18;
        let option = 1 << 19;
        let command = 1 << 20;
        for (code, flags, expected) in [
            (56, shift | 2, true),
            (60, shift | 2, false),
            (60, shift | 6, true),
            (56, shift | 4, false),
            (60, shift | 4, true),
            (60, 0, false),
            (55, command | 0x10, false),
            (54, command | 0x10, true),
            (58, option | 0x40, false),
            (61, option | 0x40, true),
            (59, control | 0x2000, false),
            (62, control | 0x2000, true),
            (59, control | 1, true),
            (56, shift, true),
            (60, shift, true),
            (56, command | 2, false),
            (57, 1 << 16, true),
            (57, shift, false),
            (63, 1 << 23, true),
            (63, 0, false),
            (0, shift | command, false),
        ] {
            assert_eq!(
                modifier_is_down(code, flags),
                expected,
                "key {code}, flags {flags:x}"
            );
        }
        let mut left = Key::default();
        let mut right = Key::default();
        assert!(!left.update(modifier_is_down(56, shift | 2), false, false, false));
        assert!(!right.update(modifier_is_down(60, shift | 6), false, false, false));
        assert!(!left.update(modifier_is_down(56, shift | 4), false, true, false));
        assert!(!left.delivered && right.delivered && !left.swallowed && !right.swallowed);
    }

    #[test]
    fn skipped_by_monitors() {
        use NSEventType as Event;
        let shift = NSEventModifierFlags::Shift;
        let command = NSEventModifierFlags::Command;
        let none = NSEventModifierFlags::empty();
        for (kind, code, flags, expected) in [
            (Event::KeyDown, 102, none, true),
            (Event::KeyDown, 104, shift, true),
            (Event::KeyDown, 0, command, false),
            (Event::KeyUp, 0, command, true),
            (Event::KeyUp, 0, command | shift, true),
            (Event::KeyUp, 0, shift, false),
            (Event::KeyUp, 102, none, false),
            (Event::FlagsChanged, 0, command, false),
        ] {
            assert_eq!(skips_key(kind, code, flags), expected);
        }
        let mut key = Key::default();
        assert!(key.update(true, false, true, false));
        assert!(key.update(false, false, true, false));
        assert!(!key.swallowed && !key.delivered);
        assert!(!key.update(true, false, false, false));
        assert!(!key.update(false, false, false, false));
        assert!(!key.delivered);
    }

    #[test]
    fn presses() {
        use NSEventMask as Event;
        let down = Event::LeftMouseDown;
        let drag = Event::LeftMouseDragged;
        let up = Event::LeftMouseUp;
        // (button, event, window, covered, swallow)
        type Step = (isize, Event, isize, bool, bool);
        let cases: &[&[Step]] = &[
            &[
                (0, down, 7, false, false),
                (0, drag, 7, false, false),
                (0, up, 7, false, false),
            ],
            &[
                (0, down, 7, true, true),
                (0, drag, 7, false, true),
                (0, drag, 7, false, true),
                (0, up, 7, false, true),
                (0, up, 7, false, false),
            ],
            &[
                (0, down, 7, true, true),
                (0, down, 7, false, false),
                (0, drag, 7, false, false),
                (0, up, 7, false, false),
            ],
            &[
                (0, down, 7, true, true),
                (0, up, 8, false, false),
                (0, drag, 8, false, false),
                (0, down, 8, false, false),
                (0, up, 8, false, false),
                (0, up, 7, false, false),
            ],
            &[
                (0, down, 7, true, true),
                (1, down, 7, false, false),
                (1, up, 7, false, false),
                (1, down, 7, true, true),
                (0, up, 7, false, true),
                (1, up, 7, false, true),
            ],
            &[(40, down, 7, true, true), (40, up, 7, false, true)],
        ];
        for (case, events) in cases.iter().enumerate() {
            let mut buttons = Buttons::default();
            assert!(buttons.windows.iter().all(Option::is_none));
            for &(button, kind, window, covered, expected) in *events {
                assert_eq!(
                    buttons.update(button, kind, window, covered),
                    expected,
                    "case {case}, button {button}, window {window}"
                );
            }
            assert!(buttons.windows.iter().all(Option::is_none));
        }
    }

    #[test]
    fn screen_ahead_settles_only_when_not_a_native_reveal_and_not_leaving() {
        assert!(leaving_screen_ahead(false, true, true));
        assert!(!leaving_screen_ahead(true, true, true));
        assert!(!leaving_screen_ahead(false, false, true));
        assert!(settles_screen_ahead(true, false, false));
        assert!(!settles_screen_ahead(false, false, false));
        assert!(!settles_screen_ahead(true, true, false));
        assert!(!settles_screen_ahead(true, false, true));
    }
}
