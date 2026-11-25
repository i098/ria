// Ported from ardia apps/browser/src/lib/windowChrome.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The room that the window's caption buttons take at the top corners, and the signal that the
//! chrome changed.
//!
//! Changes from Ardia: an omitted TS `chrome` argument is `None`, which takes `WINDOW_CHROME`, and an
//! omitted `fromTop` is `None` (0). `Math.max` is `js::js_max` (it keeps `NaN`). A watcher is an
//! `Arc<dyn Fn()>`; the set of watchers compares them by the `Arc` pointer, as a JS `Set` compares
//! functions by identity. `on_window_chrome_change` returns a remover that can run more than once,
//! as Ardia's can. The version is a `u64` counter given as `f64`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::js::js_max;
pub use super::platform_chrome::{
    WINDOW_BUTTON_STYLES, WINDOW_CHROME, WindowButtonStyle, WindowChrome, apply_window_button_style,
};
pub use super::window_button_layout::{
    TRAFFIC_LIGHTS_WIDTH, WINDOW_CONTROLS, WindowButton, window_buttons_for, window_controls_width,
};

/// `captionRoom` (`windowChrome.ts:6`).
pub fn caption_room(from_right: f64, from_top: Option<f64>, chrome: Option<&WindowChrome>) -> f64 {
    let chrome = chrome.unwrap_or(&WINDOW_CHROME);
    if from_top.unwrap_or(0.0) >= chrome.title_bar_height {
        return 0.0;
    }
    js_max(0.0, chrome.trailing_width - from_right)
}

/// `captionClearTop` (`windowChrome.ts:11`).
pub fn caption_clear_top(
    top: f64,
    row_height: f64,
    from_right: f64,
    chrome: Option<&WindowChrome>,
) -> f64 {
    let chrome = chrome.unwrap_or(&WINDOW_CHROME);
    let room = caption_room(from_right, Some(top), Some(chrome));
    // `!room`: 0 and NaN are falsy.
    if room == 0.0 || room.is_nan() {
        return top;
    }
    js_max(top, chrome.title_bar_height - row_height)
}

/// `trailingClearance` (`windowChrome.ts:16`).
pub fn trailing_clearance(
    top: f64,
    margin: f64,
    controls_bottom: f64,
    chrome: Option<&WindowChrome>,
) -> f64 {
    let chrome = chrome.unwrap_or(&WINDOW_CHROME);
    if top < controls_bottom {
        js_max(margin, chrome.trailing_width)
    } else {
        margin
    }
}

/// `windowButtonsChoosable` (`windowChrome.ts:20`).
pub const WINDOW_BUTTONS_CHOOSABLE: bool = WINDOW_BUTTON_STYLES.len() > 1;

/// `trafficLightsDrawn` (`windowChrome.ts:22`).
pub fn traffic_lights_drawn(chrome: Option<&WindowChrome>) -> bool {
    let chrome = chrome.unwrap_or(&WINDOW_CHROME);
    chrome.leading_width > 0.0 && chrome.leading_buttons.is_empty()
}

/// `windowControlsDrawn` (`windowChrome.ts:24`).
pub fn window_controls_drawn(chrome: Option<&WindowChrome>) -> bool {
    let chrome = chrome.unwrap_or(&WINDOW_CHROME);
    chrome.leading_buttons.len() + chrome.trailing_buttons.len() > 0
}

/// A listener of `onWindowChromeChange`.
pub type ChromeWatcher = Arc<dyn Fn() + Send + Sync>;

/// `chromeVersion` (`windowChrome.ts:26`).
static CHROME_VERSION: AtomicU64 = AtomicU64::new(0);
/// `chromeWatchers` (`windowChrome.ts:27`), in the order they were added.
static CHROME_WATCHERS: Mutex<Vec<ChromeWatcher>> = Mutex::new(Vec::new());

fn same_watcher(a: &ChromeWatcher, b: &ChromeWatcher) -> bool {
    std::ptr::addr_eq(Arc::as_ptr(a), Arc::as_ptr(b))
}

fn watchers() -> std::sync::MutexGuard<'static, Vec<ChromeWatcher>> {
    CHROME_WATCHERS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// `windowChromeVersion` (`windowChrome.ts:29`).
pub fn window_chrome_version() -> f64 {
    CHROME_VERSION.load(Ordering::Relaxed) as f64
}

/// `onWindowChromeChange` (`windowChrome.ts:31`): adds `listener` once and returns its remover.
pub fn on_window_chrome_change(listener: ChromeWatcher) -> impl Fn() + Send + Sync {
    {
        let mut list = watchers();
        if !list.iter().any(|w| same_watcher(w, &listener)) {
            list.push(listener.clone());
        }
    }
    move || watchers().retain(|w| !same_watcher(w, &listener))
}

/// `noteWindowChromeChanged` (`windowChrome.ts:36`): calls a copy of the list, so a watcher may
/// add or remove watchers.
pub fn note_window_chrome_changed() {
    CHROME_VERSION.fetch_add(1, Ordering::Relaxed);
    let snapshot = watchers().clone();
    for watcher in snapshot {
        watcher();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    // `onWindowChromeChange` takes a function, which the recorded cases cannot carry.
    #[test]
    fn watchers_run_once_each_until_removed() {
        static CALLS: AtomicUsize = AtomicUsize::new(0);
        let watcher: ChromeWatcher = Arc::new(|| {
            CALLS.fetch_add(1, Ordering::Relaxed);
        });
        let before = window_chrome_version();
        let remove = on_window_chrome_change(watcher.clone());
        let again = on_window_chrome_change(watcher);
        note_window_chrome_changed();
        assert_eq!(
            CALLS.load(Ordering::Relaxed),
            1,
            "a Set holds a watcher once"
        );
        remove();
        again();
        note_window_chrome_changed();
        assert_eq!(CALLS.load(Ordering::Relaxed), 1);
        assert_eq!(window_chrome_version(), before + 2.0);
    }
}
