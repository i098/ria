// Ported from ardia packages/ardiacore/ios/ArdiaHoverProbeInternal.h at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Changes from Ardia: the `*Name` functions of `ArdiaHoverProbe.mm` are `name` methods here, the episode's
//! consent stamp is the epoch string that `ArdiaConsentStamp` keeps, and the `extern` globals live in
//! `ardia_hover_probe`.

use objc2::rc::Weak;
use objc2_app_kit::NSWindow;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HoverReason {
    #[default]
    Fine,
    HalfActive,
    StaleMenu,
    NotKeyOrMain,
    CoveredByView,
    ResponderLost,
    CoveredByWindow,
}

impl HoverReason {
    pub fn name(self) -> &'static str {
        [
            "fine",
            "halfActive",
            "staleMenu",
            "notKeyOrMain",
            "coveredByView",
            "responderLost",
            "coveredByWindow",
        ][self as usize]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FocusWindowKind {
    #[default]
    None,
    Hidden,
    BlockerPage,
    Internal,
    ArdiaPanel,
    Popup,
    Browser,
    Other,
}

impl FocusWindowKind {
    pub fn name(self) -> &'static str {
        [
            "none",
            "hidden",
            "blockerPage",
            "internal",
            "ardiaPanel",
            "popup",
            "browser",
            "other",
        ][self as usize]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PageCover {
    #[default]
    None,
    ReactNative,
    Chromium,
    Other,
}

impl PageCover {
    pub fn name(self) -> &'static str {
        ["none", "reactNative", "chromium", "other"][self as usize]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HoverTrigger {
    #[default]
    Move,
    Click,
    Recheck,
}

impl HoverTrigger {
    pub fn name(self) -> &'static str {
        ["move", "click", "recheck"][self as usize]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FocusHolder {
    #[default]
    None,
    Window,
    OtherWindow,
    Page,
    Control,
}

impl FocusHolder {
    pub fn name(self) -> &'static str {
        ["none", "window", "otherWindow", "page", "control"][self as usize]
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Activation {
    pub app_active: bool,
    pub running_active: bool,
    pub frontmost: bool,
}

impl Activation {
    pub fn in_front(&self) -> bool {
        self.app_active || self.running_active || self.frontmost
    }

    pub fn half(&self) -> bool {
        !self.app_active && (self.running_active || self.frontmost)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HoverCheck {
    pub reason: HoverReason,
    pub activation: Activation,
    pub key: FocusWindowKind,
    pub main: FocusWindowKind,
    pub cover: PageCover,
    pub cover_window: FocusWindowKind,
    pub holder: FocusHolder,
    pub cover_clear: bool,
    pub prevent_key: bool,
    pub can_key: bool,
    pub menu_active: bool,
    pub window_key: bool,
    pub window_main: bool,
    pub over_page: bool,
}

pub struct HoverEpisode {
    pub open: bool,
    pub owner: Option<Weak<NSWindow>>,
    pub serial: usize,
    pub first: HoverCheck,
    pub trigger: HoverTrigger,
    pub started_at: f64,
    pub healed: bool,
    pub heal_worked: i32,
    pub stamp: String,
}

impl Default for HoverEpisode {
    fn default() -> Self {
        Self {
            open: false,
            owner: None,
            serial: 0,
            first: HoverCheck::default(),
            trigger: HoverTrigger::Move,
            started_at: 0.0,
            healed: false,
            heal_worked: -1,
            stamp: String::new(),
        }
    }
}
