// Ported from ardia packages/ardiacore/ios/ArdiaWebView+Site.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use objc2_foundation::{NSDictionary, NSSize, NSString, NSURL};
use ria_engine::call::main_queue;
use serde_json::{Value, json};

use crate::core::bindings::Tab;
use crate::core::convert::{from_json, to_json};
use crate::core::services::{Reply, Services};

use super::ardia_web_view_page_commands::awaits_first_document;
use super::ardia_web_view_page_messages::call_frame;

pub fn security_info(tab: Option<&Tab>) -> Value {
    if let Some(tab) = tab {
        let info = to_json(&tab.security_info());
        if !info.is_null() {
            return info;
        }
    }
    let url = tab.map_or_else(|| NSString::from_str(""), Tab::url);
    let scheme = NSURL::URLWithString(&url)
        .and_then(|url| url.scheme())
        .map(|scheme| scheme.lowercaseString().to_string());
    let level = match scheme.as_deref() {
        Some("https") => "secure",
        Some("http") => "insecure",
        _ => "local",
    };
    json!({"level": level, "url": url.to_string(), "origin": null})
}

pub fn page_cookies(
    tab: Option<&Tab>,
    urls: &[String],
    completion: impl FnOnce(Option<Vec<Value>>) + 'static,
) {
    let Some(tab) =
        tab.filter(|tab| !tab.closed() && !awaits_first_document(tab) && !urls.is_empty())
    else {
        return completion(None);
    };
    let answer = Rc::new(RefCell::new(Some(completion)));
    let from_engine = Rc::clone(&answer);
    let params = from_json(&json!({"urls": urls}))
        .downcast::<NSDictionary>()
        .ok();
    tab.dev_tools_call(
        &NSString::from_str("Network.getCookies"),
        params.as_deref(),
        1.0,
        move |result, _| {
            let completion = from_engine.borrow_mut().take();
            if let Some(completion) = completion {
                let cookies = result.and_then(|result| {
                    let Value::Object(mut result) = to_json(&result) else {
                        return None;
                    };
                    match result.remove("cookies")? {
                        Value::Array(cookies) => Some(cookies),
                        _ => None,
                    }
                });
                main_queue::post(Box::new(move || completion(cookies)));
            }
        },
    );
    main_queue::after(
        Duration::from_millis(1200),
        Box::new(move || {
            let completion = answer.borrow_mut().take();
            if let Some(completion) = completion {
                completion(None);
            }
        }),
    );
}

pub fn preferred_size(size: NSSize, popup: bool, root_per_page: f64) -> Value {
    let (width, height) = if popup {
        (
            size.width.clamp(25.0, 800.0),
            size.height.clamp(25.0, 600.0),
        )
    } else {
        (size.width, size.height)
    };
    json!({"width": width * root_per_page, "height": height * root_per_page})
}

pub fn clear_site_data(tab: Option<&Tab>, profile: &str, services: &Services, reply: Reply) {
    let Some(origin) = tab
        .and_then(|tab| Tab::origin_for_url(&tab.url()))
        .filter(|origin| !origin.is_empty())
    else {
        return reply(Ok(json!({"cookies": false, "storage": false})));
    };
    services.clear_site_data(profile, &origin.to_string(), reply);
}

pub fn answer_autofill_prompt(tab: &Tab, prompt_id: isize, action: &NSString) -> bool {
    let matches = || {
        tab.autofill_prompt()
            .is_some_and(|prompt| to_json(&prompt)["id"].as_i64() == Some(prompt_id as i64))
    };
    if !matches() {
        return false;
    }
    tab.resolve_autofill_prompt(prompt_id, action);
    !matches()
}

pub fn notification_action(
    tab: &Tab,
    frames: &mut HashMap<String, String>,
    id: &str,
    action: &str,
) {
    let frame = if action == "click" {
        frames.get(id).map(String::as_str)
    } else {
        let removed = frames.remove(id);
        if let Some(frame) = removed {
            send_notification_action(tab, &frame, id, action);
        }
        return;
    };
    if let Some(frame) = frame {
        send_notification_action(tab, frame, id, action);
    }
}

fn send_notification_action(tab: &Tab, frame: &str, id: &str, action: &str) {
    call_frame(
        tab,
        frame,
        "notification",
        &json!({"id": id, "action": action}).to_string(),
    );
}

pub fn is_capturing(access: &Value) -> bool {
    ["camera", "microphone", "screen"].iter().any(|key| {
        access[*key]
            .as_bool()
            .unwrap_or_else(|| access[*key].as_i64().is_some_and(|value| value != 0))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_popup_size_is_clamped_before_scaling() {
        let size = NSSize::new(12.0, 900.0);
        assert_eq!(
            preferred_size(size, true, 0.5),
            json!({"width": 12.5, "height": 300.0})
        );
        assert_eq!(
            preferred_size(size, false, 0.5),
            json!({"width": 6.0, "height": 450.0})
        );
    }

    #[test]
    fn screen_capture_counts_without_camera_or_microphone() {
        assert!(is_capturing(&json!({"camera": false, "screen": true})));
        assert!(!is_capturing(&json!({"camera": false, "microphone": 0})));
    }
}
