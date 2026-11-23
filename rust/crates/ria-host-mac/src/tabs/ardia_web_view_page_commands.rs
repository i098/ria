// Ported from ardia packages/ardiacore/ios/ArdiaWebView+PageCommands.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use objc2_app_kit::NSView;
use objc2_foundation::{NSDictionary, NSRect, NSString, NSURL};
use ria_engine::call::main_queue;
use serde_json::{Value, json};

use crate::core::bindings::Tab;
use crate::core::convert::{from_json, to_json};

#[derive(Default)]
pub struct PageCommands {
    muted: bool,
}

impl PageCommands {
    pub fn set_muted(&mut self, tab: Option<&Tab>, muted: bool) {
        self.muted = muted;
        if let Some(tab) = tab {
            tab.set_muted(muted);
        }
    }

    pub fn audio_changed(&self, tab: &Tab) -> Value {
        if self.muted && !tab.muted() {
            tab.set_muted(true);
        }
        self.media(Some(tab))
    }

    pub fn media(&self, tab: Option<&Tab>) -> Value {
        json!({
            "playing": tab.is_some_and(Tab::audible),
            "muted": tab.map_or(self.muted, Tab::muted),
        })
    }

    pub fn zoom(tab: Option<&Tab>, pinch_scale: f64, user: bool) -> Value {
        let zoom = tab.map_or(1.0, Tab::zoom_factor);
        let host = tab
            .and_then(|tab| NSURL::URLWithString(&tab.url()))
            .and_then(|url| url.host())
            .map(|host| host.lowercaseString().to_string());
        json!({
            "zoom": (zoom * 100.0).round() / 100.0,
            "host": host.as_deref().unwrap_or_default(),
            "isDefault": (zoom - 1.0).abs() < 0.001,
            "pinchScale": if pinch_scale == 0.0 { 1.0 } else { pinch_scale },
            "user": user,
        })
    }

    pub fn zoom_step(tab: &Tab, direction: i32, pinch_scale: f64) -> Option<Value> {
        let before = tab.zoom_factor();
        tab.zoom_step(direction);
        ((tab.zoom_factor() - before).abs() > 1e-6)
            .then(|| Self::zoom(Some(tab), pinch_scale, true))
    }
}

pub fn find_result(count: i32, active: i32, final_result: bool) -> Value {
    json!({"count": count, "active": active, "final": final_result})
}

pub fn print(tab: &Tab) {
    tab.execute_chrome_command(35003);
}

pub fn show_dev_tools_panel(tab: &Tab, panel: &str) {
    match panel {
        "network" => tab.show_dev_tools_panel(&NSString::from_str(panel)),
        "console" | "inspect" | "toggle" => {
            tab.execute_chrome_command(match panel {
                "console" => 40005,
                "inspect" => 40023,
                _ => 40237,
            });
        }
        _ => tab.show_dev_tools(),
    }
}

pub fn run_page_command(tab: &Tab, name: &str) {
    let command = match name {
        "savePage" => 35004,
        "systemPrint" => 35007,
        "caretBrowsing" => 40260,
        _ => return,
    };
    tab.execute_chrome_command(command);
}

pub fn awaits_first_document(tab: &Tab) -> bool {
    tab.loading() && tab.committed_initial() && !tab.initial_document_accessed()
}

pub fn execute_javascript(tab: &Tab, code: &str) {
    if !awaits_first_document(tab) {
        tab.execute_javascript(&NSString::from_str(code), None);
    }
}

fn thaw_hidden(tab: &Tab, visible: bool, frozen: bool) {
    if !visible && !frozen {
        tab.set_frozen(false);
    }
}

pub fn evaluate(
    tab: Option<&Tab>,
    code: &str,
    visible: bool,
    frozen: bool,
    completion: impl FnOnce(Option<String>) + 'static,
) {
    let Some(tab) = tab.filter(|tab| !awaits_first_document(tab)) else {
        completion(None);
        return;
    };
    thaw_hidden(tab, visible, frozen);
    let completion = Cell::new(Some(completion));
    tab.evaluate(&NSString::from_str(code), move |result| {
        if let Some(completion) = completion.take() {
            main_queue::post(Box::new(move || completion(result)));
        }
    });
}

pub fn capture_picture(
    tab: Option<&Tab>,
    host: &NSView,
    root_frame: NSRect,
    scale: f64,
    completion: impl FnOnce(Option<Value>) + 'static,
) {
    let Some(tab) = tab
        .filter(|tab| host.window().is_some() && !host.isHidden() && !awaits_first_document(tab))
    else {
        completion(None);
        return;
    };
    let frame = [
        root_frame.origin.x,
        root_frame.origin.y,
        root_frame.size.width,
        root_frame.size.height,
    ];
    let params = from_json(&json!({
        "format": "jpeg",
        "quality": if scale < 0.5 { 55 } else { 75 },
        "optimizeForSpeed": true,
    }))
    .downcast::<NSDictionary>()
    .expect("screenshot parameters are a dictionary");
    let completion = Rc::new(Cell::new(Some(completion)));
    let reply = completion.clone();
    tab.dev_tools_call(
        &NSString::from_str("Page.captureScreenshot"),
        Some(&params),
        1.0,
        move |result, _| {
            let data = result.as_deref().map(|result| to_json(result));
            let value = data
                .as_ref()
                .and_then(|data| data["data"].as_str())
                .map(|data| json!({"data": data, "frame": frame}));
            let reply = reply.clone();
            main_queue::post(Box::new(move || {
                if let Some(reply) = reply.take() {
                    reply(value);
                }
            }));
        },
    );
    main_queue::after(
        Duration::from_secs(1),
        Box::new(move || {
            if let Some(completion) = completion.take() {
                completion(None);
            }
        }),
    );
}

pub fn navigation_entries(tab: Option<&Tab>) -> Value {
    tab.map_or_else(|| json!([]), |tab| to_json(&tab.navigation_entries()))
}

pub fn media_command(
    tab: &Tab,
    visible: bool,
    frozen: bool,
    frame: &str,
    action: &str,
    seconds: f64,
) {
    thaw_hidden(tab, visible, frozen);
    let frame_id = NSString::from_str(frame);
    if matches!(action, "play" | "toggle") {
        tab.grant_user_activation_to_frame(&frame_id);
    }
    let message = json!({"action": action, "seconds": seconds}).to_string();
    super::ardia_web_view_page_messages::call_frame(tab, frame, "media", &message);
}

pub fn seek_picture_in_picture_by(
    tab: &Tab,
    visible: bool,
    frozen: bool,
    mut frames: impl Iterator<Item = impl AsRef<str>>,
    seconds: f64,
) {
    thaw_hidden(tab, visible, frozen);
    let kind = NSString::from_str("media");
    let message = NSString::from_str(
        &json!({"action": "seekPictureInPictureBy", "seconds": seconds}).to_string(),
    );
    if let Some(first) = frames.next() {
        for frame in std::iter::once(first).chain(frames) {
            tab.call_frame(&NSString::from_str(frame.as_ref()), &kind, &message);
        }
    } else {
        tab.call_page(&kind, &message);
    }
}
