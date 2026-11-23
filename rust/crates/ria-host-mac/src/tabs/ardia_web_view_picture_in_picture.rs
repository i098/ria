// Ported from ardia packages/ardiacore/ios/ArdiaWebView+PictureInPicture.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use std::time::Duration;

use objc2::MainThreadMarker;
use objc2_app_kit::{NSEventMask, NSEventModifierFlags, NSEventType, NSView};
use objc2_foundation::{NSProcessInfo, NSString, NSURL};
use ria_engine::call::main_queue;
use serde_json::{Value, json};

use crate::core::bindings::Tab;

use super::ardia_web_view_input_monitors::EventMonitor;

#[derive(Default)]
pub struct ControlPresses(VecDeque<Press>);

struct Press {
    at: f64,
    window: isize,
    target: Option<isize>,
}

impl ControlPresses {
    pub fn install(
        mtm: MainThreadMarker,
        presses: Rc<RefCell<Self>>,
        in_web_content: impl Fn(&NSView) -> bool + 'static,
        native_control_tag: impl Fn(&NSView) -> Option<isize> + 'static,
    ) -> Option<EventMonitor> {
        EventMonitor::install(
            mtm,
            NSEventMask::LeftMouseUp | NSEventMask::KeyDown,
            move |event| {
                let Some(window) = event.window(mtm) else {
                    return false;
                };
                if event.r#type() == NSEventType::KeyDown {
                    let chord = NSEventModifierFlags::Command
                        | NSEventModifierFlags::Control
                        | NSEventModifierFlags::Option;
                    if !event.isARepeat()
                        && matches!(event.keyCode(), 49 | 36 | 76)
                        && !event.modifierFlags().intersects(chord)
                        && let Some(view) = window
                            .firstResponder()
                            .and_then(|view| view.downcast::<NSView>().ok())
                        && view.window().is_some()
                        && !in_web_content(&view)
                        && let Some(tag) = native_control_tag(&view).filter(|tag| *tag > 0)
                    {
                        presses.borrow_mut().record(
                            event.timestamp(),
                            window.windowNumber(),
                            Some(tag),
                        );
                    }
                } else if let Some(content) = window.contentView() {
                    let root = super::superview(&content).unwrap_or(content);
                    if root
                        .hitTest(event.locationInWindow())
                        .is_some_and(|view| !in_web_content(&view))
                    {
                        presses
                            .borrow_mut()
                            .record(event.timestamp(), window.windowNumber(), None);
                    }
                }
                false
            },
        )
    }

    pub fn accessibility_press(&mut self, view: &NSView, native_tag: isize) {
        if native_tag > 0
            && let Some(window) = view.window()
        {
            self.record(uptime(), window.windowNumber(), Some(native_tag));
        }
    }

    fn record(&mut self, at: f64, window: isize, target: Option<isize>) {
        if self.0.len() == 8 {
            self.0.pop_front();
        }
        self.0.push_back(Press { at, window, target });
    }

    pub fn take(&mut self, click_ms: f64, target: isize, window: isize) -> Option<f64> {
        self.take_at(click_ms, target, window, uptime())
    }

    fn take_at(&mut self, click_ms: f64, target: isize, window: isize, now: f64) -> Option<f64> {
        if click_ms <= 0.0 && target <= 0 {
            return None;
        }
        let index = self.0.iter().position(|press| {
            if press.window != window || press.at <= 0.0 {
                return false;
            }
            let age = now - press.at;
            if click_ms > 0.0 {
                press.target.is_none()
                    && (press.at * 1000.0 - click_ms).abs() <= 1.0
                    && (0.0..=10.0).contains(&age)
            } else {
                press.target == Some(target) && (0.0..=2.0).contains(&age)
            }
        })?;
        self.0.remove(index).map(|press| press.at)
    }
}

fn uptime() -> f64 {
    NSProcessInfo::processInfo().systemUptime()
}

#[derive(Default)]
pub struct PictureInPicture {
    pub active: bool,
    pub auto_active: bool,
    played_for_auto: bool,
    request: Option<Pending>,
    next_request: u64,
}

struct Pending {
    id: u64,
    stamp: Value,
}

pub struct PictureChange {
    pub payload: Value,
    pub stamp: Value,
    pub entered: Option<Entered>,
}

pub enum Entered {
    Video { host: String },
    Document,
}

impl PictureInPicture {
    pub fn plays_for_auto(enabled: bool, now_playing: &Value, capturing: bool) -> bool {
        enabled && (now_playing["playbackState"] == "playing" || capturing)
    }

    pub fn auto_inputs_changed(&mut self, has_tab: bool, plays: bool) -> bool {
        let plays = has_tab && plays;
        if self.played_for_auto == plays {
            return false;
        }
        self.played_for_auto = plays;
        true
    }

    pub fn kept_on_app_switch(&self, tab: bool, visible: bool, seen: bool, plays: bool) -> bool {
        tab && visible && !self.active && seen && plays
    }

    pub fn update_auto(&mut self, tab: &Tab, seen: bool) {
        if seen && (self.auto_active || self.active) {
            self.auto_active = false;
            tab.exit_picture_in_picture();
        }
    }

    pub fn changed(
        &mut self,
        tab: &Tab,
        state: &Value,
        stamp: impl FnOnce() -> Value,
    ) -> Option<PictureChange> {
        let active = state["active"].as_bool().unwrap_or(false);
        let document = state["kind"] == "document";
        if !document && !active && !self.active {
            return None;
        }
        let current = if active { self.request.take() } else { None };
        let stamp = current
            .map(|pending| pending.stamp)
            .or_else(|| self.request.as_ref().map(|pending| pending.stamp.clone()))
            .unwrap_or_else(stamp);
        self.active = active;
        let mut payload =
            json!({"kind": if document { "document" } else { "video" }, "active": active});
        if !active && let Some(closed_by) = state.get("closedBy") {
            payload["closedBy"] = closed_by.clone();
            payload["auto"] = Value::Bool(state["auto"].as_bool().unwrap_or(false));
        }
        let entered = active.then(|| {
            if document {
                Entered::Document
            } else {
                let host = NSURL::URLWithString(&tab.url())
                    .and_then(|url| url.host())
                    .map_or_else(String::new, |host| host.to_string());
                Entered::Video { host }
            }
        });
        Some(PictureChange {
            payload,
            stamp,
            entered,
        })
    }

    pub fn activate_request(
        reason: Option<&str>,
        returned_from_picture: bool,
        from_other_app: bool,
    ) -> Value {
        json!({"reason": if returned_from_picture { "pictureInPicture" } else { reason.unwrap_or("page") }, "fromOtherApp": from_other_app})
    }
}

type Finish = Box<dyn FnOnce(bool)>;

pub fn request_picture_in_picture(
    state: &Rc<RefCell<PictureInPicture>>,
    tab: Option<&Tab>,
    verified_press_at: Option<f64>,
    stamp: impl FnOnce() -> Value,
    thaw_hidden: impl FnOnce(),
    trace: bool,
    finish: Finish,
) {
    if state.borrow().active {
        return finish(true);
    }
    let (Some(tab), Some(at)) = (tab, verified_press_at.filter(|at| *at > 0.0)) else {
        return finish(false);
    };
    let id = {
        let mut state = state.borrow_mut();
        state.next_request = state.next_request.wrapping_add(1);
        let id = state.next_request;
        state.request = Some(Pending { id, stamp: stamp() });
        id
    };
    thaw_hidden();
    let weak = Rc::downgrade(state);
    let tab_weak = objc2::rc::Weak::from(tab);
    let finish = Cell::new(Some(finish));
    tab.enter_picture_in_picture_for_click_at(at, move |route, opened| {
        if let Some(finish) = finish.take() {
            let state = weak.clone();
            let tab_weak = tab_weak.clone();
            main_queue::post(Box::new(move || {
                if trace && let Some(tab) = tab_weak.load() {
                    tab.trace_picture_in_picture(
                        &NSString::from_str(&format!(
                            "app: manual PiP route {route} opened={opened}"
                        )),
                        false,
                    );
                }
                if route == "session" {
                    await_picture(state, id, 8, finish);
                } else {
                    let entered = state
                        .upgrade()
                        .is_some_and(|state| opened || state.borrow().active);
                    finish_request(&state, id, entered, finish);
                }
            }));
        }
    });
}

fn await_picture(state: Weak<RefCell<PictureInPicture>>, id: u64, tries: u8, finish: Finish) {
    let Some(live) = state.upgrade() else {
        return finish(false);
    };
    let entered = live.borrow().active;
    if entered || tries == 0 {
        return finish_request(&state, id, entered, finish);
    }
    main_queue::after(
        Duration::from_millis(100),
        Box::new(move || await_picture(state, id, tries - 1, finish)),
    );
}

fn finish_request(state: &Weak<RefCell<PictureInPicture>>, id: u64, entered: bool, finish: Finish) {
    if let Some(state) = state.upgrade() {
        let mut state = state.borrow_mut();
        if (!entered || state.active)
            && state
                .request
                .as_ref()
                .is_some_and(|pending| pending.id == id)
        {
            state.request = None;
        }
    }
    finish(entered);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_presses_are_bound_to_window_route_age_and_one_use() {
        let mut presses = ControlPresses::default();
        presses.record(20.0, 7, None);
        presses.record(20.0, 7, Some(2));
        assert_eq!(presses.take_at(20_001.0, 0, 6, 25.0), None);
        assert_eq!(presses.take_at(20_001.0, 2, 7, 25.0), Some(20.0));
        assert_eq!(presses.take_at(20_001.0, 2, 7, 25.0), None);
        assert_eq!(presses.take_at(0.0, 2, 7, 22.1), None);
        assert_eq!(presses.take_at(0.0, 2, 7, 22.0), Some(20.0));
        assert_eq!(presses.take_at(0.0, 2, 7, 22.0), None);
    }
}
