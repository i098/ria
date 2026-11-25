// Ported from ardia apps/browser/src/lib/platformChrome.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The Mac's window chrome: the traffic lights' room at the leading top, no window button choice,
//! and the look of a pop-up button.
//!
//! Not ported: `platformChrome.linux.ts`, `platformChrome.windows.ts` and `platformChrome.web.ts`
//! (the Linux, Windows and web builds); this port is the Mac build only.
//!
//! Changes from Ardia: `WindowButtonStyle` and the pop-up indicator are enums, not string unions.
//! `WINDOW_CHROME` is a `static`, not mutable: on the Mac nothing changes it (`Object.assign` runs only in the
//! Linux build). `apply_window_button_style` takes `Option` for the optional TS layout.

use serde::{Deserialize, Serialize};

use super::window_button_layout::{TRAFFIC_LIGHTS_WIDTH, WindowButton, WindowButtonLayout};

/// `WindowChrome` (`platformChrome.ts:3`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowChrome {
    pub leading_width: f64,
    pub trailing_width: f64,
    pub title_bar_height: f64,
    pub leading_buttons: Vec<WindowButton>,
    pub trailing_buttons: Vec<WindowButton>,
}

/// `WindowButtonStyle` (`platformChrome.ts:11`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WindowButtonStyle {
    Native,
    TrafficLights,
    Hidden,
}

/// `PopUpChrome["indicator"]` (`platformChrome.ts:14`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PopUpIndicator {
    AccentUpDown,
    ChevronDown,
}

/// `PopUpChrome` (`platformChrome.ts:13`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PopUpChrome {
    pub indicator: PopUpIndicator,
    pub grows_to_its_title: bool,
}

/// `WINDOW_CHROME` (`platformChrome.ts:18`).
pub static WINDOW_CHROME: WindowChrome = WindowChrome {
    leading_width: TRAFFIC_LIGHTS_WIDTH,
    trailing_width: 0.0,
    title_bar_height: 0.0,
    leading_buttons: Vec::new(),
    trailing_buttons: Vec::new(),
};

/// `WINDOW_BUTTON_STYLES` (`platformChrome.ts:20`).
pub const WINDOW_BUTTON_STYLES: &[WindowButtonStyle] = &[];

/// `applyWindowButtonStyle` (`platformChrome.ts:22`): the Mac has no choice, so nothing changes.
pub fn apply_window_button_style(
    _style: WindowButtonStyle,
    _layout: Option<&WindowButtonLayout>,
) -> bool {
    false
}

/// `POP_UP_CHROME` (`platformChrome.ts:24`).
pub const POP_UP_CHROME: PopUpChrome = PopUpChrome {
    indicator: PopUpIndicator::AccentUpDown,
    grows_to_its_title: false,
};

/// `ADVANCED_PANE_SYMBOL` (`platformChrome.ts:26`).
pub const ADVANCED_PANE_SYMBOL: &str = "slider.horizontal.3";

/// `PANEL_FIELD_LIFT` (`platformChrome.ts:28`).
pub const PANEL_FIELD_LIFT: f64 = 0.0;
