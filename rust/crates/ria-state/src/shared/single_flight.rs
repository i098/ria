// Ported from ardia apps/browser/src/lib/singleFlight.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! One pass of a job at a time: a call during a pass joins it and asks for one more pass, a pass
//! can ask for another, and `keep_going` stops the repeats. A failed pass ends the flight, and the
//! next call starts afresh.
//!
//! Changes from Ardia:
//! - Synchronous, without promises. A call can happen during a pass only from inside the pass
//!   (reentrant), through the same `SingleFlight`; it sets `again` and returns `None` at once.
//!   In TypeScript it returns the running promise.
//! - `run` takes `pass` and `keep_going` on each call instead of once in `singleFlight`; a call
//!   that joins a running pass ignores them.
//! - A pass returns `Result`; an `Err` ends the flight like a rejected promise.
//!
//! Not ported (stays JS): the shared promise that joined callers wait on, and the order of the
//! microtasks. A call made before the first `await` of the first pass starts a second flight in
//! TypeScript, because `running` is set only after `pass` returns its promise.

use std::cell::Cell;

/// The state of `singleFlight` (`singleFlight.ts:3`) and its `Flight` (`singleFlight.ts:1`).
#[derive(Debug, Default)]
pub struct SingleFlight {
    running: Cell<bool>,
    again: Cell<bool>,
}

impl SingleFlight {
    /// `singleFlight` (`singleFlight.ts:3`).
    pub fn new() -> Self {
        Self::default()
    }

    /// `Flight.again` (`singleFlight.ts:7`): a call or `repeat` asked for one more pass.
    pub fn again(&self) -> bool {
        self.again.get()
    }

    /// `Flight.repeat` (`singleFlight.ts:10`).
    pub fn repeat(&self) {
        self.again.set(true);
    }

    /// The function that `singleFlight` returns (`singleFlight.ts:12`): `None` when the call
    /// joined a running pass, else the result of the flight.
    pub fn run<E>(
        &self,
        mut pass: impl FnMut(&Self) -> Result<(), E>,
        mut keep_going: impl FnMut() -> bool,
    ) -> Option<Result<(), E>> {
        if self.running.get() {
            self.again.set(true);
            return None;
        }
        self.running.set(true);
        let result = loop {
            self.again.set(false);
            if let Err(error) = pass(self) {
                break Err(error);
            }
            if !(self.again.get() && keep_going()) {
                break Ok(());
            }
        };
        self.running.set(false);
        Some(result)
    }
}
