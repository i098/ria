// Ported from ardia packages/ardiacore/ios/ArdiaChromeWindowHost+Dev.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use std::cell::Cell;
use std::time::Duration;

use objc2::MainThreadOnly;
use objc2_app_kit::{NSColor, NSWindow, NSWindowButton, NSWindowOcclusionState, NSWindowStyleMask};
use serde_json::json;

use super::ardia_chrome_window_host::ChromeWindowHost;
use super::ardia_window_controller::Controllers;

#[derive(Default)]
pub(super) struct DevActions {
    transition: Cell<Duration>,
    gap: Cell<Option<f64>>,
}

fn milliseconds(text: &str) -> f64 {
    text.parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .unwrap_or(0.)
}

fn duration(ms: f64) -> Duration {
    Duration::try_from_secs_f64(ms.max(0.) / 1000.).unwrap_or_default()
}

fn flag(value: bool) -> String {
    u8::from(value).to_string()
}

impl DevActions {
    pub(super) fn run(
        &self,
        host: &ChromeWindowHost,
        controllers: &Controllers,
        window: &NSWindow,
        action: &str,
    ) -> Option<String> {
        let (name, argument) = action
            .split_once(':')
            .map_or((action, None), |(name, argument)| (name, Some(argument)));
        let value = argument.unwrap_or_default();
        match name {
            "fakeFullScreenMs" => {
                let transition = duration(milliseconds(value));
                self.transition.set(transition);
                Some(format!("{:.0}", transition.as_secs_f64() * 1000.))
            }
            "fakeFullScreenOcclusionMs" => {
                let gap = milliseconds(value);
                self.gap.set((gap != -1.).then_some(gap / 1000.));
                Some(format!("{gap:.0}"))
            }
            "fullScreen" | "fakeFullScreen" => {
                let Some(controller) = controllers.for_window(window) else {
                    return Some("not an ArdiaCore window".into());
                };
                let fullscreen = &controller.fullscreen;
                if name == "fakeFullScreen"
                    && let Some(argument) = argument
                {
                    let mut fields = argument.split(':');
                    let entering = fields.next() == Some("1");
                    let transition = fields
                        .next()
                        .map_or(self.transition.get(), |ms| duration(milliseconds(ms)));
                    if entering != fullscreen.is_full_screen() {
                        fullscreen.act_window_full_screen_with_timing(
                            entering,
                            transition,
                            self.gap.get(),
                        );
                    }
                }
                Some(
                    json!({
                        "fullScreen": fullscreen.is_full_screen(),
                        "styleMask": window.styleMask().contains(NSWindowStyleMask::FullScreen),
                        "acted": controller.core.acted_full_screen(),
                        "chromeCounts": controller.core.acted_full_screen(),
                        "transitioning": fullscreen.transitioning(),
                    })
                    .to_string(),
                )
            }
            "fakeOcclusion" => {
                let controller = controllers.for_window(window)?;
                controller
                    .fullscreen
                    .act_occlusion((value != "off").then_some(value == "visible"));
                Some(flag(
                    window
                        .occlusionState()
                        .contains(NSWindowOcclusionState::Visible),
                ))
            }
            "fakeWindowServer" => {
                controllers
                    .for_window(window)?
                    .fullscreen
                    .act_window_server(value == "shows");
                Some("ok".into())
            }
            "focusState" => Some(host.focus_state(window.mtm())),
            "shadow" => {
                window.setHasShadow(value == "1");
                Some("ok".into())
            }
            "ns" => {
                if value == "out" {
                    window.orderOut(None)
                } else {
                    window.orderFront(None)
                };
                Some(format!("visible={}", flag(window.isVisible())))
            }
            "root" => {
                host.root_view(window)?.setHidden(value == "hide");
                Some("ok".into())
            }
            "opaque" => {
                window.setOpaque(value == "1");
                if !window.isOpaque() {
                    window.setBackgroundColor(Some(&NSColor::clearColor()));
                }
                Some(format!("opaque={}", flag(window.isOpaque())))
            }
            "lights" => {
                let button = window.standardWindowButton(NSWindowButton::CloseButton)?;
                let frame = button.convertRect_toView(button.bounds(), None);
                Some(format!(
                    "{:.1},{:.1},{:.1},{:.1}",
                    frame.origin.x, frame.origin.y, frame.size.width, frame.size.height
                ))
            }
            _ => None,
        }
    }
}
