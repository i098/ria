// Ported from ardia apps/browser/src/lib/windowButtonLayout.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The window buttons a desktop asks for: its GTK `button-layout` text read into Ardia's buttons,
//! the room they take, and the side they sit on.
//!
//! Changes from Ardia: `WindowButton`, `WindowButtonSide` are enums, not string unions.
//! `window_buttons_for` is generic over the button type (`WindowButton` or any `AsRef<str>`),
//! because Ardia's filter keeps a value outside the union as it is. Optional TS arguments are
//! `Option`: `None` is `undefined`, which takes the TS default. `button_layout_count` returns
//! `usize`, not a JS number.

use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// `WindowButton` (`windowButtonLayout.ts:1`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WindowButton {
    Menu,
    Minimize,
    Maximize,
    Close,
}

impl WindowButton {
    /// The TS string of the button.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Menu => "menu",
            Self::Minimize => "minimize",
            Self::Maximize => "maximize",
            Self::Close => "close",
        }
    }
}

impl AsRef<str> for WindowButton {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// `WindowButtonLayout` (`windowButtonLayout.ts:3`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowButtonLayout {
    pub leading: Vec<WindowButton>,
    pub trailing: Vec<WindowButton>,
}

/// `WindowButtonSide` (`windowButtonLayout.ts:5`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WindowButtonSide {
    Right,
    Left,
    Both,
    None,
}

/// The type of `WINDOW_CONTROLS` (`windowButtonLayout.ts:7`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowControls {
    pub edge: f64,
    pub gap: f64,
    pub button: f64,
    pub regular: f64,
    pub small: f64,
    pub band: f64,
    pub strip: f64,
}

/// `WINDOW_CONTROLS` (`windowButtonLayout.ts:7`).
pub const WINDOW_CONTROLS: WindowControls = WindowControls {
    edge: 12.0,
    gap: 3.0,
    button: 30.0,
    regular: 30.0,
    small: 28.0,
    band: 26.0,
    strip: 18.0,
};

/// `windowControlsWidth` (`windowButtonLayout.ts:9`). `None` is the TS default, 3 buttons.
pub fn window_controls_width(count: Option<f64>) -> f64 {
    let count = count.unwrap_or(3.0);
    if count > 0.0 {
        count * WINDOW_CONTROLS.button + (count - 1.0) * WINDOW_CONTROLS.gap
    } else {
        0.0
    }
}

/// `TRAFFIC_LIGHTS_WIDTH` (`windowButtonLayout.ts:11`).
pub const TRAFFIC_LIGHTS_WIDTH: f64 = 78.0;

/// `DESKTOP_BUTTON_LAYOUT_FALLBACK` (`windowButtonLayout.ts:13`).
pub const DESKTOP_BUTTON_LAYOUT_FALLBACK: &str = ":minimize,maximize,close";

/// `BUTTON_OF` (`windowButtonLayout.ts:15`): only these exact tokens are buttons.
fn button_of(token: &str) -> Option<WindowButton> {
    match token {
        "minimize" => Some(WindowButton::Minimize),
        "maximize" => Some(WindowButton::Maximize),
        "close" => Some(WindowButton::Close),
        _ => None,
    }
}

/// `parseButtonLayout` (`windowButtonLayout.ts:17`). `None` is a value that is not a string.
pub fn parse_button_layout(raw: Option<&str>) -> WindowButtonLayout {
    let source = raw.unwrap_or(DESKTOP_BUTTON_LAYOUT_FALLBACK);
    let mut placed = Vec::new();
    let mut buttons_of = |side: &str| -> Vec<WindowButton> {
        side.split(',')
            .filter_map(button_of)
            .filter(|button| {
                let fresh = !placed.contains(button);
                if fresh {
                    placed.push(*button);
                }
                fresh
            })
            .collect()
    };
    // Only the first colon splits, as `indexOf` and `slice` do.
    let (leading, trailing) = source.split_once(':').unwrap_or((source, ""));
    let leading = buttons_of(leading);
    let trailing = buttons_of(trailing);
    WindowButtonLayout {
        leading,
        trailing: std::iter::once(WindowButton::Menu)
            .chain(trailing)
            .collect(),
    }
}

/// `BROWSER_BUTTON_LAYOUT` (`windowButtonLayout.ts:33`).
pub static BROWSER_BUTTON_LAYOUT: LazyLock<WindowButtonLayout> =
    LazyLock::new(|| parse_button_layout(Some(DESKTOP_BUTTON_LAYOUT_FALLBACK)));

/// `WindowButtonsAllowed` (`windowButtonLayout.ts:35`). `None` is `undefined`: allowed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WindowButtonsAllowed {
    pub menu: Option<bool>,
    pub minimizable: Option<bool>,
    pub maximizable: Option<bool>,
}

/// `windowButtonsFor` (`windowButtonLayout.ts:37`). A button outside the four stays, as in Ardia.
pub fn window_buttons_for<B: AsRef<str> + Clone>(
    buttons: &[B],
    allowed: &WindowButtonsAllowed,
) -> Vec<B> {
    let menu = allowed.menu.unwrap_or(true);
    let minimizable = allowed.minimizable.unwrap_or(true);
    let maximizable = allowed.maximizable.unwrap_or(true);
    buttons
        .iter()
        .filter(|button| match button.as_ref() {
            "menu" => menu,
            "minimize" => minimizable,
            "maximize" => maximizable,
            _ => true,
        })
        .cloned()
        .collect()
}

/// `windowButtonCount` (`windowButtonLayout.ts:40`).
fn window_button_count(buttons: &[WindowButton]) -> usize {
    buttons
        .iter()
        .filter(|button| **button != WindowButton::Menu)
        .count()
}

/// `buttonLayoutSide` (`windowButtonLayout.ts:42`).
pub fn button_layout_side(layout: &WindowButtonLayout) -> WindowButtonSide {
    let left = window_button_count(&layout.leading) > 0;
    let right = window_button_count(&layout.trailing) > 0;
    match (left, right) {
        (true, true) => WindowButtonSide::Both,
        (true, false) => WindowButtonSide::Left,
        (false, true) => WindowButtonSide::Right,
        (false, false) => WindowButtonSide::None,
    }
}

/// `buttonLayoutCount` (`windowButtonLayout.ts:48`).
pub fn button_layout_count(layout: &WindowButtonLayout) -> usize {
    window_button_count(&layout.leading) + window_button_count(&layout.trailing)
}
