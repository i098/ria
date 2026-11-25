// Ported from ardia apps/browser/src/lib/windowLayout.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Sidebar width limits and the page frame: the inset, corner radii and gaps of the page card.
//!
//! Changes from Ardia:
//! - `validPageFrame` takes `Json` for `unknown`.
//! - The `opts` and `settings` objects are the structs `PageFrameOpts` and `PageFrameSettings`; an
//!   optional `borderlessMode` is `false` when left out (the original reads it as a truth value).
//! - `drawnWindowRadius = windowRadius` is `Option<f64>`: `None` takes `window_radius`.
//! - `Math.min`, `Math.max` are `js_min`, `js_max` to keep `NaN` and `-0`.

use ria_core::identity::telemetry_wire::Json;

use super::js::{js_max, js_min};

/// `DEFAULT_SIDEBAR_WIDTH` (`windowLayout.ts:1`).
pub const DEFAULT_SIDEBAR_WIDTH: f64 = 190.0;
/// `SIDEBAR_MIN_WIDTH` (`windowLayout.ts:3`).
pub const SIDEBAR_MIN_WIDTH: f64 = 160.0;
/// `SIDEBAR_MAX_WIDTH` (`windowLayout.ts:4`).
pub const SIDEBAR_MAX_WIDTH: f64 = 400.0;
/// `MIN_WINDOW_WIDTH` (`windowLayout.ts:6`).
pub const MIN_WINDOW_WIDTH: f64 = 500.0;
/// `SIDEBAR_PAGE_ROOM` (`windowLayout.ts:8`).
pub const SIDEBAR_PAGE_ROOM: f64 = MIN_WINDOW_WIDTH - DEFAULT_SIDEBAR_WIDTH;
/// `CARD_INSET` (`windowLayout.ts:15`).
pub const CARD_INSET: f64 = 9.0;

/// `PAGE_FRAME` (`windowLayout.ts:17`): the range and default of the page frame setting.
pub struct PageFrameRange {
    pub min: f64,
    pub max: f64,
    pub default: f64,
}

/// `PAGE_FRAME` (`windowLayout.ts:17`).
pub const PAGE_FRAME: PageFrameRange = PageFrameRange {
    min: 0.0,
    max: 12.0,
    default: CARD_INSET,
};

const SPLIT_MIN_GAP: f64 = 2.0;
const MIN_PEEK_GAP: f64 = 1.0;

/// `sidebarWidthLimit` (`windowLayout.ts:10`): `None` is `null` or `undefined`.
pub fn sidebar_width_limit(window_width: Option<f64>) -> f64 {
    match window_width {
        Some(w) if w != 0.0 && !w.is_nan() => js_min(
            SIDEBAR_MAX_WIDTH,
            js_max(SIDEBAR_MIN_WIDTH, (w - SIDEBAR_PAGE_ROOM).floor()),
        ),
        _ => SIDEBAR_MAX_WIDTH,
    }
}

/// `shownSidebarWidth` (`windowLayout.ts:13`).
pub fn shown_sidebar_width(saved: f64, window_width: Option<f64>) -> f64 {
    js_min(saved, sidebar_width_limit(window_width))
}

/// `validPageFrame` (`windowLayout.ts:22`): an integer from `PAGE_FRAME.min` to `PAGE_FRAME.max`.
pub fn valid_page_frame(v: &Json) -> bool {
    matches!(v, Json::Number(n) if n.fract() == 0.0 && *n >= PAGE_FRAME.min && *n <= PAGE_FRAME.max)
}

/// The `opts` of `pageFrameInsetOf` (`windowLayout.ts:26`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PageFrameOpts {
    pub borderless_mode: bool,
    pub zero_in_full_screen: bool,
    pub full_screen: bool,
    pub zero_when_tabs_hidden: bool,
    pub tabs_hidden: bool,
}

/// `pageFrameInsetOf` (`windowLayout.ts:24`).
pub fn page_frame_inset_of(inset: f64, opts: &PageFrameOpts) -> f64 {
    if opts.borderless_mode
        || (opts.full_screen && opts.zero_in_full_screen)
        || (opts.full_screen && opts.tabs_hidden && opts.zero_when_tabs_hidden)
    {
        return 0.0;
    }
    inset
}

/// The `settings` of `pageFrameInset` (`windowLayout.ts:40`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PageFrameSettings {
    pub page_frame: f64,
    pub borderless_mode: bool,
}

/// `pageFrameInset` (`windowLayout.ts:40`).
pub fn page_frame_inset(settings: &PageFrameSettings) -> f64 {
    page_frame_inset_of(
        settings.page_frame,
        &PageFrameOpts {
            borderless_mode: settings.borderless_mode,
            ..PageFrameOpts::default()
        },
    )
}

/// The `peek` of `PageFrame` (`windowLayout.ts:53`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Peek {
    pub left: f64,
    pub top: f64,
    pub bottom: f64,
    pub radius: f64,
}

/// `PageFrame` (`windowLayout.ts:49`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageFrame {
    pub inset: f64,
    pub radius: f64,
    pub split_gap: f64,
    pub peek: Peek,
}

/// `pageFrameOf` (`windowLayout.ts:56`).
pub fn page_frame_of(
    inset: f64,
    window_radius: f64,
    drawn_window_radius: Option<f64>,
) -> PageFrame {
    let drawn = drawn_window_radius.unwrap_or(window_radius);
    let settled = js_min(1.0, inset / CARD_INSET);
    let concentric = js_max(drawn + (window_radius - drawn) * settled - inset, 2.0);
    let peek_gap = js_max(MIN_PEEK_GAP, inset);
    PageFrame {
        inset,
        radius: if inset <= 0.0 { 0.0 } else { concentric },
        split_gap: js_max(inset, SPLIT_MIN_GAP),
        peek: Peek {
            left: peek_gap,
            top: peek_gap,
            bottom: peek_gap,
            radius: js_max(0.0, concentric + inset - peek_gap),
        },
    }
}
