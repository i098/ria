// Ported from ardia apps/browser/src/lib/themeGesture.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! One drag or click in the theme editor: edits are drafted at once and written once a frame,
//! the release writes at once, and a drag that loses its preset (another preset shown, the selection
//! changed, a Reset, or the look changed from outside) writes nothing more until it ends.
//!
//! Changes from Ardia:
//! - `Deps` is the trait `ThemeGestureDeps`. `current()` gives a `Current: PartialEq` value that
//!   stands for the identity (`!==`) of the shown look in TS.
//! - `requestFrame(run)` takes no callback: the host calls `ThemeGesture::run_frame` when the frame
//!   it asked for comes. Frame handles are `u64`.
//! - `themeGesture` gives the struct `ThemeGesture`; its closures are methods, `dispose` included.
//! - `settle = false` is a plain `bool`.

use super::profile_look::ProfileTheme;

/// `ThemeEdit` (`themeGesture.ts:3`).
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeEdit {
    pub theme: Option<ProfileTheme>,
    pub target: String,
}

/// `Deps` (`themeGesture.ts:5`).
pub trait ThemeGestureDeps {
    /// What `current()` gives: equal values stand for the same shown look.
    type Current: PartialEq;
    fn apply(&mut self, theme: Option<&ProfileTheme>);
    fn showing(&self) -> String;
    fn current(&self) -> Self::Current;
    fn draft(&mut self, edit: Option<&ThemeEdit>);
    /// Asks for a frame; the host then calls `ThemeGesture::run_frame`.
    fn request_frame(&mut self) -> u64;
    fn cancel_frame(&mut self, handle: u64);
}

/// The gesture `themeGesture` (`themeGesture.ts:14`) gives.
pub struct ThemeGesture<D: ThemeGestureDeps> {
    pub deps: D,
    pending: Option<ThemeEdit>,
    frame: Option<u64>,
    gesture: Option<String>,
    cancelled: bool,
    expected: Option<D::Current>,
}

/// `themeGesture` (`themeGesture.ts:14`).
pub fn theme_gesture<D: ThemeGestureDeps>(deps: D) -> ThemeGesture<D> {
    ThemeGesture {
        deps,
        pending: None,
        frame: None,
        gesture: None,
        cancelled: false,
        expected: None,
    }
}

impl<D: ThemeGestureDeps> ThemeGesture<D> {
    /// `stopFrame` (`themeGesture.ts:20`).
    fn stop_frame(&mut self) {
        if let Some(frame) = self.frame.take() {
            self.deps.cancel_frame(frame);
        }
    }

    /// `flush` (`themeGesture.ts:24`), the frame callback: writes the waiting edit.
    pub fn run_frame(&mut self) {
        self.frame = None;
        let Some(edit) = self.pending.take() else {
            return;
        };
        if edit.target != self.deps.showing() || Some(self.deps.current()) != self.expected {
            self.cancelled = true;
            self.deps.draft(None);
            return;
        }
        self.deps.apply(edit.theme.as_ref());
        self.expected = Some(self.deps.current());
    }

    /// `drop` (`themeGesture.ts:37`).
    fn drop_edit(&mut self) {
        self.stop_frame();
        self.pending = None;
        self.deps.draft(None);
    }

    /// The start of a gesture: `expected` and `gesture` when none is going.
    fn start(&mut self, target: &str) {
        if self.gesture.is_none() {
            self.expected = Some(self.deps.current());
            self.gesture = Some(target.to_owned());
        }
    }

    /// `begin` (`themeGesture.ts:43`).
    pub fn begin(&mut self, target: &str) {
        self.start(target);
    }

    /// `change` (`themeGesture.ts:47`).
    pub fn change(&mut self, theme: Option<ProfileTheme>, target: &str, settle: bool) {
        self.start(target);
        if self.cancelled
            || self.gesture.as_deref() != Some(target)
            || self.deps.showing() != target
        {
            self.drop_edit();
            self.cancelled = true;
            if settle {
                self.gesture = None;
                self.cancelled = false;
            }
            return;
        }
        let edit = ThemeEdit {
            theme,
            target: target.to_owned(),
        };
        self.deps.draft(if settle { None } else { Some(&edit) });
        self.pending = Some(edit);
        if settle {
            self.stop_frame();
            self.run_frame();
            self.gesture = None;
            self.cancelled = false;
        } else if self.frame.is_none() {
            self.frame = Some(self.deps.request_frame());
        }
    }

    /// `selectionChanged` (`themeGesture.ts:68`).
    pub fn selection_changed(&mut self) {
        if self.gesture.is_none() {
            return;
        }
        self.drop_edit();
        self.cancelled = true;
    }

    /// `cancel` (`themeGesture.ts:73`).
    pub fn cancel(&mut self) {
        self.drop_edit();
        self.cancelled = self.gesture.is_some();
        if !self.cancelled {
            self.gesture = None;
        }
    }

    /// `dispose` (`themeGesture.ts:78`).
    pub fn dispose(&mut self) {
        self.stop_frame();
    }
}
