// Ported from ardia packages/ardiacore/ios/ArdiaWebView+PageMessages.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use objc2::MainThreadOnly;
use objc2_app_kit::{NSEvent, NSView, NSWindow};
use objc2_foundation::{NSString, NSURL, NSUUID};
use serde_json::{Value, json};

use crate::core::bindings::{Engine, Tab};
use crate::core::convert::to_json;

use super::ardia_web_view_first_paint::PaintEnd;

pub fn call_frame(tab: &Tab, frame: &str, kind: &str, json: &str) {
    let kind = NSString::from_str(kind);
    let json = NSString::from_str(json);
    if frame.is_empty() {
        tab.call_page(&kind, &json);
    } else {
        tab.call_frame(&NSString::from_str(frame), &kind, &json);
    }
}

pub fn verified_origin(origin: &str) -> Option<&str> {
    (!matches!(origin, "" | "null")).then_some(origin)
}

pub fn same_site_frame(main: bool, origin: Option<&str>, top: Option<&str>) -> bool {
    main || origin.is_some_and(|origin| !origin.is_empty() && Some(origin) == top)
}

#[derive(Default)]
pub struct ScriptFrames {
    seen: VecDeque<(String, bool)>,
}

impl ScriptFrames {
    pub fn note(&mut self, frame: &str, same_site: bool) {
        if frame.is_empty() {
            return;
        }
        self.forget(frame);
        if self.seen.len() == 512 {
            self.seen.pop_front();
        }
        self.seen.push_back((frame.into(), same_site));
    }

    pub fn forget(&mut self, frame: &str) {
        if let Some(index) = self.seen.iter().position(|(known, _)| known == frame) {
            self.seen.remove(index);
        }
    }

    pub fn all(&self) -> impl Iterator<Item = &str> {
        self.seen.iter().map(|(frame, _)| frame.as_str())
    }

    pub fn push_field_provider(&self, tab: &Tab, state: &str) {
        call_frame(tab, "", "fieldProvider", state);
        for (frame, same_site) in &self.seen {
            call_frame(
                tab,
                frame,
                "fieldProvider",
                if *same_site { state } else { "false" },
            );
        }
    }

    pub fn push_field_provider_hint(&self, tab: &Tab, state: &str) {
        let mut same_site = self
            .seen
            .iter()
            .filter(|(_, same_site)| *same_site)
            .peekable();
        if same_site.peek().is_none() {
            call_frame(tab, "", "fieldProviderHint", state);
        }
        for (frame, _) in same_site {
            call_frame(tab, frame, "fieldProviderHint", state);
        }
    }
}

pub fn text(value: &Value, name: &str, max: usize) -> Option<String> {
    let value = value.get(name)?.as_str()?;
    if value.len() <= max {
        return Some(value.into());
    }
    let text = NSString::from_str(value);
    if text.len() <= max {
        return Some(value.into());
    }
    let range = text.rangeOfComposedCharacterSequenceAtIndex(max);
    Some(text.substringToIndex(range.location).to_string())
}

pub fn flag(value: &Value, name: &str) -> bool {
    let value = &value[name];
    value
        .as_bool()
        .unwrap_or_else(|| value.as_f64().is_some_and(|number| number != 0.0))
}

pub fn finite(value: &Value, name: &str) -> Option<f64> {
    let value = &value[name];
    value
        .as_f64()
        .or_else(|| value.as_bool().map(|value| f64::from(u8::from(value))))
        .filter(|number| number.is_finite())
}

pub fn first_paint(value: &Value, current_url: &str, main: bool) -> Option<PaintEnd> {
    let url = text(value, "url", 8192)?;
    (main
        && !url.is_empty()
        && !url.starts_with("about:")
        && url.split('#').next() == current_url.split('#').next())
    .then(|| {
        if flag(value, "loaded") {
            PaintEnd::Loaded
        } else {
            PaintEnd::Painted
        }
    })
}

pub fn selection(value: &Value, root_per_page: f64) -> Option<Value> {
    let text = text(value, "text", 4000)?;
    let rect = &value["rect"];
    let x = finite(rect, "x")? * root_per_page;
    let y = finite(rect, "y")? * root_per_page;
    let width = finite(rect, "width")? * root_per_page;
    let height = finite(rect, "height")? * root_per_page;
    Some(json!({"text": text, "rect": {"x": x, "y": y, "width": width, "height": height}}))
}

pub fn app_badge(value: &Value, engine_origin: &str) -> Option<Value> {
    let origin = verified_origin(engine_origin)?;
    let count = finite(value, "count")
        .filter(|count| *count >= 1.0)
        .map(|count| count.min(1e9).floor() as u32);
    Some(json!({"origin": origin, "count": count, "flag": count.is_none() && flag(value, "flag")}))
}

pub fn pinch_scale(value: &Value) -> Option<f64> {
    finite(value, "scale")
        .filter(|scale| *scale > 0.0)
        .map(|scale| scale.min(100.0))
}

pub fn scroll(value: &Value) -> Option<Value> {
    let state = text(value, "state", 8)?;
    matches!(state.as_str(), "down" | "up" | "top").then(|| json!({"state": state}))
}

pub fn protected_media(value: &Value, main: bool) -> Value {
    let key = text(value, "keySystem", 64).unwrap_or_default();
    let mut report = json!({"keySystem": key, "main": main});
    if let Some(stage) = text(value, "stage", 16).filter(|stage| {
        matches!(
            stage.as_str(),
            "access" | "createMediaKeys" | "licence" | "keystatus"
        )
    }) {
        report["stage"] = json!(stage);
    }
    report
}

#[derive(Default)]
pub struct LinkDrop {
    last: Option<Instant>,
}

impl LinkDrop {
    pub fn accept_page_drop(
        &mut self,
        value: &Value,
        view: &NSView,
        visible: bool,
    ) -> Option<Value> {
        if !visible || NSEvent::pressedMouseButtons() & 1 != 0 {
            return None;
        }
        // SAFETY: Both values are Core Graphics mouse event types.
        let (since_up, since_drag) = unsafe {
            (
                CGEventSourceSecondsSinceLastEventType(0, 2),
                CGEventSourceSecondsSinceLastEventType(0, 6),
            )
        };
        if !(since_up < 1.0 && since_up <= since_drag) {
            return None;
        }
        let window = view.window()?;
        let mouse = NSEvent::mouseLocation();
        if NSWindow::windowNumberAtPoint_belowWindowWithWindowNumber(mouse, 0, view.mtm())
            != window.windowNumber()
        {
            return None;
        }
        let point = view.convertPoint_fromView(window.convertPointFromScreen(mouse), None);
        let bounds = view.bounds();
        let x = point.x - bounds.origin.x;
        let y = point.y - bounds.origin.y;
        let inside = x >= 0.0 && y >= 0.0 && x < bounds.size.width && y < bounds.size.height;
        self.accept(value, inside, Instant::now())
    }

    fn accept(&mut self, value: &Value, released_over_view: bool, now: Instant) -> Option<Value> {
        if !released_over_view
            || self
                .last
                .is_some_and(|last| now.saturating_duration_since(last) <= Duration::from_secs(1))
        {
            return None;
        }
        let url = text(value, "url", 8192)?;
        let scheme = NSURL::URLWithString(&NSString::from_str(&url))?
            .scheme()?
            .lowercaseString();
        if !matches!(scheme.to_string().as_str(), "http" | "https") {
            return None;
        }
        self.last = Some(now);
        Some(json!({"url": url, "disposition": "foreground", "userGesture": true}))
    }
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state: i32, event: u32) -> f64;
}

pub struct MessageFrame<'a> {
    pub main: bool,
    pub same_site: bool,
    pub engine_origin: &'a str,
    pub tab_url: &'a str,
    pub root_points_per_local_point: f64,
}

pub enum PageMessage<'a> {
    Hello {
        scripted: bool,
        clear_navigation_request: bool,
    },
    DisplayMediaHello,
    FirstPaint(PaintEnd),
    Peek(&'a Value),
    PeekHello,
    PeekGone,
    Exclusive(bool),
    CommandWheel {
        delta: f64,
        handled: bool,
    },
    PeekTrace(&'a Value),
    NewTabLink(Option<String>),
    LinkDrop(&'a Value),
    Performance(&'a Value),
    FieldFocus {
        field: Option<String>,
        sequence: f64,
        generation: f64,
        query: Option<String>,
    },
    FieldHint(String),
    FieldProvider {
        generation: f64,
        field: String,
    },
    VerificationCode {
        generation: f64,
        url: &'a str,
    },
    NowPlaying(Option<&'a Value>),
    Theme {
        color: Option<String>,
        source: Option<String>,
    },
    Pinch(f64),
    PipTrace(Option<&'a Value>),
    Notification(&'a Value),
    NotificationClose(String),
    DisplayMedia(&'a Value),
    Report {
        name: &'static str,
        payload: Value,
    },
}

pub fn route_message<'a>(
    kind: &str,
    value: &'a Value,
    frame: MessageFrame<'_>,
) -> Option<PageMessage<'a>> {
    let object = value.is_object().then_some(value);
    let main = frame.main;
    Some(match kind {
        "hello" => PageMessage::Hello {
            scripted: main,
            clear_navigation_request: main
                && text(value, "url", 8192).is_some_and(|url| !url.starts_with("about:")),
        },
        "displayMediaHello" => PageMessage::DisplayMediaHello,
        "firstPaint" => PageMessage::FirstPaint(first_paint(value, frame.tab_url, main)?),
        "peek" => PageMessage::Peek(object?),
        "peekHello" => PageMessage::PeekHello,
        "peekGone" => PageMessage::PeekGone,
        "exclusive" => PageMessage::Exclusive(flag(object?, "held")),
        "commandWheel" => PageMessage::CommandWheel {
            delta: finite(value, "dy")?,
            handled: flag(value, "handled"),
        },
        "peekTrace" => PageMessage::PeekTrace(object?),
        "selection" if main => PageMessage::Report {
            name: "selection",
            payload: json!({"selection": selection(value, frame.root_points_per_local_point)}),
        },
        "newTabLink" => PageMessage::NewTabLink(text(object?, "url", 8192)),
        "linkDrop" if main => PageMessage::LinkDrop(value),
        "perf" if main => PageMessage::Performance(object?),
        "scroll" if main => PageMessage::Report {
            name: "scroll",
            payload: scroll(value)?,
        },
        "fieldFocus" if main => PageMessage::FieldFocus {
            field: text(value, "field", 16),
            sequence: finite(value, "seq")?,
            generation: finite(value, "generation")?,
            query: text(value, "query", 64),
        },
        "fieldProviderHint" if frame.same_site => PageMessage::FieldHint(
            text(value, "state", 16)
                .filter(|state| matches!(state.as_str(), "shown" | "dismissed"))?,
        ),
        "fieldProvider" if frame.same_site => PageMessage::FieldProvider {
            generation: finite(value, "generation")?,
            field: text(value, "field", 16)?,
        },
        "verificationCode" if main => PageMessage::VerificationCode {
            generation: finite(value, "generation")?,
            url: value["url"].as_str()?,
        },
        "drm" => PageMessage::Report {
            name: "protectedMedia",
            payload: protected_media(object?, main),
        },
        "nowPlaying" => PageMessage::NowPlaying(object),
        "theme" if main => PageMessage::Theme {
            color: text(object?, "color", 64),
            source: text(value, "source", 32),
        },
        "appBadge" if main => PageMessage::Report {
            name: "appBadge",
            payload: app_badge(object?, frame.engine_origin)?,
        },
        "pinch" if main => PageMessage::Pinch(pinch_scale(value)?),
        "pipTrace" => PageMessage::PipTrace(object),
        "notification" => PageMessage::Notification(object?),
        "notificationClose" => {
            PageMessage::NotificationClose(text(value, "id", 128).filter(|id| !id.is_empty())?)
        }
        "displayMedia" => PageMessage::DisplayMedia(object?),
        _ => return None,
    })
}

#[derive(Default)]
pub struct NotificationFrames {
    open: HashMap<String, String>,
}

impl NotificationFrames {
    pub fn request(
        &mut self,
        value: &Value,
        frame: &str,
        origin: &str,
        app_tab_id: &Value,
        main: bool,
    ) -> Option<(String, Value)> {
        let id = text(value, "id", 128).filter(|id| !id.is_empty())?;
        let title = text(value, "title", 1024)?;
        let origin = verified_origin(origin)?;
        if self.open.len() > 500 {
            self.open.clear();
        }
        self.open.insert(id.clone(), frame.into());
        let report = json!({
            "id": id, "title": title, "origin": origin,
            "body": text(value, "body", 4096).unwrap_or_default(),
            "icon": text(value, "icon", 1 << 20),
            "tag": text(value, "tag", 1024).unwrap_or_default(),
            "silent": flag(value, "silent"), "requireInteraction": flag(value, "requireInteraction"),
            "tabId": app_tab_id, "isMainFrame": main,
        });
        Some((id, report))
    }

    pub fn waiting(&self, id: &str, frame: &str) -> bool {
        self.open.get(id).is_some_and(|known| known == frame)
    }

    pub fn close(&mut self, id: &str) {
        self.open.remove(id);
    }
}

#[derive(Default)]
pub struct DisplayRequests {
    pending: HashMap<String, DisplayRequest>,
}

struct DisplayRequest {
    frame: String,
    origin: String,
    page_id: i32,
    audio: Option<AudioOffer>,
}

struct AudioOffer {
    screen: bool,
    window: Option<&'static str>,
}

impl AudioOffer {
    fn from_page(value: &Value, support: &Value) -> Option<Self> {
        if !flag(value, "audio") {
            return None;
        }
        let system = flag(support, "system");
        let application = flag(support, "application");
        let window = match value["windowAudio"].as_str() {
            Some("window") if application => Some("application"),
            Some("window" | "system") if system => Some("system"),
            _ => None,
        };
        Some(Self {
            screen: system && flag(value, "systemAudio"),
            window,
        })
    }

    fn for_source(&self, source: &str) -> Option<&'static str> {
        if source.starts_with("screen:") {
            self.screen.then_some("system")
        } else if source.starts_with("window:") {
            self.window
        } else {
            source
                .starts_with("web-contents-media-stream://")
                .then_some("tab")
        }
    }
}

impl DisplayRequests {
    pub fn offer(
        &mut self,
        value: &Value,
        frame: &str,
        sender: Option<&str>,
        top: Option<&str>,
        sources: Value,
    ) -> Option<Value> {
        let sender = sender?;
        let page_id = finite(value, "id")
            .filter(|id| *id >= 1.0 && *id <= f64::from(i32::MAX) && id.fract() == 0.0)?
            as i32;
        if self.pending.len() >= 16 {
            return None;
        }
        let support = to_json(&Engine::desktop_capture_audio());
        let audio = AudioOffer::from_page(value, &support);
        let id = NSUUID::UUID().UUIDString().to_string();
        let report = json!({
            "id": id, "origin": top.unwrap_or(sender), "frameOrigin": sender,
            "audio": audio.as_ref().map(|offer| json!({"tab": true, "screen": offer.screen, "window": offer.window})),
            "sources": sources, "controller": value["controller"] == Value::Bool(true),
        });
        self.pending.insert(
            id,
            DisplayRequest {
                frame: frame.into(),
                origin: sender.into(),
                page_id,
                audio,
            },
        );
        Some(report)
    }

    pub fn resolve(
        &mut self,
        tab: &Tab,
        request_id: &str,
        source: Option<&str>,
        share_audio: bool,
        may_capture: impl FnOnce(&str) -> bool,
    ) {
        let Some(request) = self.pending.remove(request_id) else {
            return;
        };
        let source = source.filter(|source| !source.is_empty() && may_capture(source));
        let audio = source.and_then(|source| {
            request
                .audio
                .as_ref()
                .filter(|_| share_audio)
                .and_then(|offer| offer.for_source(source))
        });
        if let Some(source) = source {
            let frame = (!request.frame.is_empty()).then(|| NSString::from_str(&request.frame));
            let audio = audio.map(NSString::from_str);
            Engine::allow_desktop_capture(
                &NSString::from_str(source),
                tab,
                frame.as_deref(),
                Some(&NSString::from_str(&request.origin)),
                audio.as_deref(),
            );
        }
        let reply = json!({"id": request.page_id, "sourceId": source, "audio": audio.is_some()});
        call_frame(tab, &request.frame, "displayMedia", &reply.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privileged_page_messages_require_the_expected_frame() {
        let frame = MessageFrame {
            main: false,
            same_site: false,
            engine_origin: "https://example.test",
            tab_url: "https://example.test/",
            root_points_per_local_point: 1.0,
        };
        let message = json!({"generation": 1, "field": "password"});
        assert!(route_message("fieldProvider", &message, frame).is_none());
        let frame = MessageFrame {
            main: false,
            same_site: true,
            engine_origin: "https://example.test",
            tab_url: "https://example.test/",
            root_points_per_local_point: 1.0,
        };
        assert!(
            route_message(
                "verificationCode",
                &json!({"generation": 1, "url": "test"}),
                frame
            )
            .is_none()
        );
    }

    #[test]
    fn paint_requires_the_live_top_document() {
        let message = json!({"url": "https://example.test/#old", "loaded": true});
        assert_eq!(
            first_paint(&message, "https://example.test/#new", true),
            Some(PaintEnd::Loaded)
        );
        assert_eq!(
            first_paint(&message, "https://example.test/other", true),
            None
        );
        assert_eq!(
            first_paint(&message, "https://example.test/#new", false),
            None
        );
    }

    #[test]
    fn badges_use_only_the_engine_origin() {
        let message = json!({"origin": "https://page.test", "count": 3.8, "flag": true});
        assert_eq!(app_badge(&message, "null"), None);
        assert_eq!(
            app_badge(&message, "https://engine.test"),
            Some(json!({"origin": "https://engine.test", "count": 3, "flag": false}))
        );
    }
}
