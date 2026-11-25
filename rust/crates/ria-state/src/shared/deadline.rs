// Ported from ardia apps/browser/src/lib/deadline.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! A value that arrives in time, or a fallback.
//!
//! Changes from Ardia: the promise is a `std::sync::mpsc::Receiver` and the wait blocks the thread.
//! A rejection is a `T` of `Result`. A sender dropped without a value is a promise that never settles: the
//! call waits out the deadline and gives the fallback. Nothing needs `clearTimeout`.

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

/// The longest delay of Node's `setTimeout`, in ms.
const TIMEOUT_MAX: f64 = 2_147_483_647.0;

/// `withDeadline` (`deadline.ts:1`): the value of `promise` when it comes within `ms`, else `fallback`.
pub fn with_deadline<T>(promise: &Receiver<T>, ms: f64, fallback: T) -> T {
    // `setTimeout` waits 1 ms for a delay that is NaN or outside 1..=TIMEOUT_MAX.
    let ms = if (1.0..=TIMEOUT_MAX).contains(&ms) {
        ms
    } else {
        1.0
    };
    let deadline = Duration::from_secs_f64(ms / 1000.0);
    let start = Instant::now();
    match promise.recv_timeout(deadline) {
        Ok(value) => value,
        Err(RecvTimeoutError::Timeout) => fallback,
        Err(RecvTimeoutError::Disconnected) => {
            std::thread::sleep(deadline.saturating_sub(start.elapsed()));
            fallback
        }
    }
}
