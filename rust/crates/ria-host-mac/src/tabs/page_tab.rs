// Ported from ardia packages/ardiacore/ios/PageTab.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! `PageTab`: what the app can ask of one page view, whichever engine draws the page. Every method runs on the main
//! thread. A reply is a boxed `FnOnce` that the page settles once.

use std::rc::Rc;

use objc2::rc::Retained;
use objc2_app_kit::{NSColor, NSView};
use serde_json::{Map, Value};

/// The call of a page view that gets a reply.
pub type Reply<T> = Box<dyn FnOnce(T)>;

/// `ArdiaWebViewDelegate webView:event:payload:`: the page sends `(event name, payload)` here.
pub type EventSink = Rc<dyn Fn(&str, Map<String, Value>)>;

pub trait PageTab {
    /// The `NSView` of the page (`PageTab: NSView`).
    fn view(&self) -> &NSView;
    /// Where the page sends its events (`ArdiaWebView.delegate`).
    fn set_event_sink(&self, sink: EventSink);

    fn set_initial_url(&self, url: Option<String>);
    fn set_profile(&self, profile: String);
    fn set_adopt_id(&self, id: Option<String>);
    fn set_transfer_key(&self, key: Option<String>);
    fn set_standalone(&self, standalone: bool);
    fn set_temporary(&self, temporary: bool);
    fn set_temporary_source(&self, source: Option<String>);
    fn set_extension_host(&self, kind: Option<String>);
    fn set_visible(&self, visible: bool);
    fn set_selected(&self, selected: bool);
    fn set_warm(&self, warm: bool);
    fn set_neighbour(&self, neighbour: bool);
    fn set_page_background_color(&self, color: Option<Retained<NSColor>>);
    fn set_page_color(&self, color: Option<String>);
    fn set_contents_background_color(&self, color: Option<Retained<NSColor>>);
    fn set_shown_url(&self, url: Option<String>);
    fn set_auto_picture_in_picture(&self, enabled: bool);
    fn set_frozen(&self, frozen: bool);
    fn media_capture_source_id(&self) -> Option<String>;

    fn start_page(&self);
    fn close_page(&self);

    fn load_url(&self, url: &str, user_initiated: bool);
    fn go_back(&self);
    fn go_forward(&self);
    fn go_to_history_offset(&self, offset: i64);
    fn reload(&self);
    fn reload_ignoring_cache(&self);
    fn stop_loading(&self);
    fn focus_page(&self);
    fn focus_page_if_unfocused(&self) -> bool;
    fn set_muted(&self, muted: bool);
    fn zoom_step(&self, direction: i64);
    fn find(&self, text: &str, forward: bool, find_next: bool);
    fn stop_finding(&self, clear_selection: bool);
    fn print(&self);
    fn run_page_command(&self, name: &str);
    fn show_dev_tools(&self, panel: Option<&str>);
    fn execute_javascript(&self, code: &str);
    fn evaluate(&self, code: &str, reply: Reply<Option<String>>);
    fn navigation_entries(&self, reply: Reply<Vec<Value>>);
    fn capture_picture(&self, scale: f64, reply: Reply<Option<Value>>);
    fn download_favicon(&self, url: &str, reply: Reply<Option<Value>>);
    fn download_image(&self, url: &str, max_pixels: i64, reply: Reply<Option<Value>>);

    fn media_command(&self, action: &str, seconds: f64);
    fn request_picture_in_picture(&self, click_at: f64, control: i64, reply: Reply<bool>);
    fn take_control_press(&self, click_at: f64, control: i64) -> bool;
    fn exit_picture_in_picture(&self);
    fn arm_element_pick(&self) -> f64;
    fn take_element_pick(&self, armed: f64) -> bool;

    fn security_info(&self, reply: Reply<Value>);
    fn page_cookies(&self, urls: &[String], reply: Reply<Option<Vec<Value>>>);
    fn site_data(&self, request: &Map<String, Value>, reply: Reply<Option<Value>>);
    fn open_blocked_popup(&self, popup_id: &str, always: bool);
    fn clear_site_data(&self, reply: Reply<Result<Value, String>>);

    fn resolve_password_prompt(&self, action: &str, username: Option<&str>, password: Option<&str>);
    fn resolve_autofill_prompt(&self, prompt_id: i64, action: &str);
    fn answer_autofill_prompt(&self, prompt_id: i64, action: &str) -> bool;
    fn execute_extension_action(&self, extension_id: &str) -> Option<String>;
    fn resolve_display_media(&self, request_id: &str, source_id: Option<&str>, audio: bool);
    fn notification_action(&self, notification_id: &str, action: &str);
    fn resolve_unresponsive(&self, terminate: bool);
    fn discard(&self, unload: bool) -> String;
    /// The ids that `ArdiaTab sleepIfHidden` answers (`[NSNumber]`).
    fn sleep_if_hidden(&self, seconds: f64) -> Vec<i64>;
}
