// Ported from ardia apps/browser/src/lib/pressMotion.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The press motion setting of buttons, with listeners for its changes.
//!
//! Changes from Ardia:
//! - A listener is an `Arc`; the set keeps one entry for each `Arc` (`Arc::ptr_eq`), as the JS `Set` does
//!   for each function.
//! - `setPressMotion` calls the listeners of the moment it starts; a listener that subscribes or
//!   unsubscribes during the calls changes the next change only. The JS loop sees such changes at once.

use std::sync::{Arc, Mutex, PoisonError};

/// `PressMotion` (`pressMotion.ts:3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressMotion {
    Still,
    Bounce,
}

impl PressMotion {
    /// The TS string of the motion.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Still => "still",
            Self::Bounce => "bounce",
        }
    }
}

/// `PRESS_MOTIONS` (`pressMotion.ts:1`).
pub const PRESS_MOTIONS: [PressMotion; 2] = [PressMotion::Still, PressMotion::Bounce];

/// A change listener.
pub type Listener = Arc<dyn Fn() + Send + Sync>;

static CURRENT: Mutex<PressMotion> = Mutex::new(PressMotion::Still);
static LISTENERS: Mutex<Vec<Listener>> = Mutex::new(Vec::new());

fn listeners() -> std::sync::MutexGuard<'static, Vec<Listener>> {
    LISTENERS.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `pressMotionNow` (`pressMotion.ts:8`).
pub fn press_motion_now() -> PressMotion {
    *CURRENT.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `setPressMotion` (`pressMotion.ts:10`): a change calls each listener.
pub fn set_press_motion(next: PressMotion) {
    {
        let mut current = CURRENT.lock().unwrap_or_else(PoisonError::into_inner);
        if *current == next {
            return;
        }
        *current = next;
    }
    let called = listeners().clone();
    for listener in called {
        listener();
    }
}

/// `subscribePressMotion` (`pressMotion.ts:16`): gives the unsubscribe, which tells whether the listener
/// was still subscribed.
pub fn subscribe_press_motion(listener: Listener) -> impl Fn() -> bool {
    {
        let mut all = listeners();
        if !all.iter().any(|l| Arc::ptr_eq(l, &listener)) {
            all.push(listener.clone());
        }
    }
    move || {
        let mut all = listeners();
        let before = all.len();
        all.retain(|l| !Arc::ptr_eq(l, &listener));
        all.len() != before
    }
}
