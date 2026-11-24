// Ported from ardia packages/ardiacore/ios/ArdiaPointerTrace.h at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Changes from Ardia: `EndLoop` and `TakeLongLoop` return the name and the consent epoch instead of writing them
//! through out pointers.

use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard};

pub const HISTORY_LENGTH: usize = 16;
pub const LONG_LOOP_SECONDS: f64 = 5.0;

#[derive(Clone, Debug, Default)]
pub struct Entry {
    pub at: f64,
    pub down: bool,
    pub window: i64,
    pub level: i64,
    pub hit: String,
    pub swallowed_by: String,
}

#[derive(Default)]
struct State {
    history: VecDeque<Entry>,
    names: Vec<String>,
    generation: u64,
    started_at: f64,
    epoch: String,
    logged: bool,
    finished: u64,
    longest: f64,
    longest_name: String,
}

#[derive(Default)]
pub struct Trace(Mutex<State>);

/// The text that a C buffer of `size` bytes keeps from `snprintf`.
fn clipped(mut text: String, size: usize) -> String {
    if text.len() >= size {
        let mut end = size - 1;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    text
}

impl State {
    fn joined(&self) -> String {
        self.names.join(">")
    }
}

impl Trace {
    fn state(&self) -> MutexGuard<'_, State> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn record(&self, entry: Entry) {
        let mut state = self.state();
        state.history.push_back(entry);
        if state.history.len() > HISTORY_LENGTH {
            state.history.pop_front();
        }
    }

    pub fn merge(&self, at: f64, down: bool, swallowed_by: &str) -> bool {
        let mut state = self.state();
        let Some(last) = state
            .history
            .back_mut()
            .filter(|last| last.at == at && last.down == down)
        else {
            return false;
        };
        if !swallowed_by.is_empty() {
            last.swallowed_by = swallowed_by.into();
        }
        true
    }

    pub fn begin_loop(&self, name: &str, now: f64, epoch: &str) -> u64 {
        let mut state = self.state();
        state.names.push(name.into());
        if state.names.len() > 1 {
            return state.generation;
        }
        state.started_at = now;
        state.epoch = epoch.into();
        state.logged = false;
        state.generation += 1;
        state.generation
    }

    /// Returns the name and the epoch of a loop that ends after it ran long and was not logged yet.
    pub fn end_loop(&self, now: f64) -> Option<(String, String)> {
        let mut state = self.state();
        if state.names.is_empty() {
            return None;
        }
        let name = state.joined();
        state.names.pop();
        if !state.names.is_empty() {
            return None;
        }
        let length = now - state.started_at;
        state.finished += 1;
        if state.finished == 1 || length > state.longest {
            state.longest = length;
            state.longest_name = name.clone();
        }
        if state.logged || length < LONG_LOOP_SECONDS {
            return None;
        }
        state.logged = true;
        Some((name, state.epoch.clone()))
    }

    pub fn take_long_loop(&self, generation: u64, now: f64) -> Option<(String, String)> {
        let mut state = self.state();
        if state.names.is_empty()
            || generation != state.generation
            || state.logged
            || now - state.started_at < LONG_LOOP_SECONDS
        {
            return None;
        }
        state.logged = true;
        Some((state.joined(), state.epoch.clone()))
    }

    pub fn loop_line(&self, now: f64) -> String {
        let state = self.state();
        let mut line = if state.names.is_empty() {
            "press loop none".to_owned()
        } else {
            let logged = if state.logged { " (logged)" } else { "" };
            clipped(
                format!(
                    "press loop open {} for {:.1}s{logged}",
                    state.joined(),
                    now - state.started_at
                ),
                256,
            )
        };
        if state.finished > 0 {
            line += &clipped(
                format!(
                    ", finished {}, longest {} {:.1}s",
                    state.finished, state.longest_name, state.longest
                ),
                256,
            );
        }
        line
    }

    pub fn history_lines(&self, now: f64) -> Vec<String> {
        self.state()
            .history
            .iter()
            .map(|entry| {
                let swallowed = if entry.swallowed_by.is_empty() {
                    "passed".to_owned()
                } else {
                    format!("swallowed by {}", entry.swallowed_by)
                };
                clipped(
                    format!(
                        "click {:.1}s ago {} window#{} level {} hit {} {swallowed}",
                        now - entry.at,
                        if entry.down { "down" } else { "up" },
                        entry.window,
                        entry.level,
                        entry.hit,
                    ),
                    320,
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(at: f64, down: bool, window: i64, level: i64, hit: &str, by: &str) -> Entry {
        Entry {
            at,
            down,
            window,
            level,
            hit: hit.into(),
            swallowed_by: by.into(),
        }
    }

    fn name(found: Option<(String, String)>) -> Option<String> {
        found.map(|(name, _)| name)
    }

    fn epoch(found: Option<(String, String)>) -> Option<String> {
        found.map(|(_, epoch)| epoch)
    }

    #[test]
    fn an_empty_trace_has_no_lines() {
        let trace = Trace::default();
        assert!(trace.history_lines(10.0).is_empty());
        assert_eq!(trace.loop_line(10.0), "press loop none");
    }

    #[test]
    fn history_lines_tell_passed_and_swallowed_clicks() {
        let trace = Trace::default();
        trace.record(entry(100.0, true, 12, 0, "NSView", ""));
        trace.record(entry(100.2, false, 12, 0, "NSView", "windowDragRegion"));
        let lines = trace.history_lines(103.0);
        assert_eq!(lines.len(), 2);
        assert_eq!(
            lines[0],
            "click 3.0s ago down window#12 level 0 hit NSView passed"
        );
        assert_eq!(
            lines[1],
            "click 2.8s ago up window#12 level 0 hit NSView swallowed by windowDragRegion"
        );
    }

    #[test]
    fn history_keeps_the_newest_sixteen() {
        let trace = Trace::default();
        for i in 0..40 {
            trace.record(entry(f64::from(i), i % 2 == 0, 7, 3, "Hit", ""));
        }
        let lines = trace.history_lines(40.0);
        assert_eq!(lines.len(), HISTORY_LENGTH);
        assert!(lines[0].contains("click 16.0s ago"));
        assert!(lines[lines.len() - 1].contains("click 1.0s ago"));
    }

    #[test]
    fn merge_marks_only_the_matching_last_entry() {
        let trace = Trace::default();
        trace.record(entry(5.0, true, 1, 0, "NSView", ""));
        assert!(trace.merge(5.0, true, "pip"));
        assert!(trace.history_lines(6.0)[0].contains("swallowed by pip"));
        assert!(trace.merge(5.0, true, ""));
        assert!(trace.history_lines(6.0)[0].contains("swallowed by pip"));
        assert!(!trace.merge(5.0, false, "pip"));
        assert!(!trace.merge(5.5, true, "pip"));
        assert_eq!(trace.history_lines(6.0).len(), 1);
    }

    #[test]
    fn merge_into_an_empty_history_fails() {
        let trace = Trace::default();
        assert!(!trace.merge(1.0, true, "pip"));
        assert!(trace.history_lines(2.0).is_empty());
    }

    #[test]
    fn a_long_loop_is_taken_once() {
        let trace = Trace::default();
        let first = trace.begin_loop("windowDragRegion", 100.0, "e");
        assert_eq!(
            trace.loop_line(102.5),
            "press loop open windowDragRegion for 2.5s"
        );
        assert_eq!(name(trace.take_long_loop(first, 104.9)), None);
        assert_eq!(
            name(trace.take_long_loop(first, 105.0)).as_deref(),
            Some("windowDragRegion")
        );
        assert_eq!(trace.take_long_loop(first, 106.0), None);
        assert_eq!(
            trace.loop_line(106.0),
            "press loop open windowDragRegion for 6.0s (logged)"
        );
        trace.end_loop(107.0);
        assert_eq!(
            trace.loop_line(108.0),
            "press loop none, finished 1, longest windowDragRegion 7.0s"
        );
        assert_eq!(trace.take_long_loop(first, 200.0), None);
    }

    #[test]
    fn nested_loops_share_one_generation() {
        let trace = Trace::default();
        let outer = trace.begin_loop("windowMove", 10.0, "e");
        let inner = trace.begin_loop("windowDragRegion", 11.0, "e");
        assert_eq!(outer, inner);
        trace.end_loop(12.0);
        assert!(
            trace
                .loop_line(13.0)
                .contains("press loop open windowMove for 3.0s")
        );
        trace.end_loop(14.0);
        assert!(
            trace
                .loop_line(15.0)
                .contains("press loop none, finished 1, longest windowMove 4.0s")
        );
        trace.end_loop(16.0);
        assert!(trace.loop_line(17.0).contains("finished 1"));
    }

    #[test]
    fn an_old_generation_cannot_take_a_new_loop() {
        let trace = Trace::default();
        let first = trace.begin_loop("windowMove", 0.0, "e");
        trace.end_loop(1.0);
        let second = trace.begin_loop("windowMove", 50.0, "e");
        assert_ne!(second, first);
        assert_eq!(trace.take_long_loop(first, 51.0), None);
        assert_eq!(trace.take_long_loop(first, 60.0), None);
        assert!(trace.take_long_loop(second, 60.0).is_some());
    }

    #[test]
    fn the_longest_finished_loop_is_named() {
        let trace = Trace::default();
        trace.begin_loop("windowMove", 0.0, "e");
        trace.end_loop(0.3);
        trace.begin_loop("windowDragRegion", 10.0, "e");
        trace.end_loop(18.0);
        trace.begin_loop("windowMove", 20.0, "e");
        trace.end_loop(20.3);
        assert_eq!(
            trace.loop_line(30.0),
            "press loop none, finished 3, longest windowDragRegion 8.0s"
        );
    }

    #[test]
    fn nested_names_join_and_a_taken_loop_is_not_logged_again() {
        let trace = Trace::default();
        let outer = trace.begin_loop("windowDragRegion", 0.0, "e");
        trace.begin_loop("windowMove", 1.0, "e");
        assert_eq!(
            trace.loop_line(2.0),
            "press loop open windowDragRegion>windowMove for 2.0s"
        );
        assert_eq!(
            name(trace.take_long_loop(outer, 6.0)).as_deref(),
            Some("windowDragRegion>windowMove")
        );
        assert_eq!(trace.end_loop(7.0), None);
        assert_eq!(trace.end_loop(8.0), None);
        assert_eq!(
            trace.loop_line(9.0),
            "press loop none, finished 1, longest windowDragRegion 8.0s"
        );
    }

    #[test]
    fn ending_a_long_loop_reports_it() {
        let trace = Trace::default();
        trace.begin_loop("windowMove", 100.0, "e");
        assert_eq!(name(trace.end_loop(105.05)).as_deref(), Some("windowMove"));
        trace.begin_loop("windowMove", 200.0, "e");
        assert_eq!(trace.end_loop(204.9), None);
    }

    #[test]
    fn a_long_loop_reports_the_epoch_of_its_start() {
        let trace = Trace::default();
        let open = trace.begin_loop("windowMove", 10.0, "before");
        assert_eq!(
            epoch(trace.take_long_loop(open, 15.5)).as_deref(),
            Some("before")
        );
        trace.end_loop(16.0);
        trace.begin_loop("windowDragRegion", 20.0, "after");
        assert_eq!(epoch(trace.end_loop(27.25)).as_deref(), Some("after"));
    }

    #[test]
    fn a_nested_loop_keeps_the_outer_epoch() {
        let trace = Trace::default();
        let outer = trace.begin_loop("windowDragRegion", 0.0, "outer");
        trace.begin_loop("windowMove", 1.0, "inner");
        assert_eq!(
            epoch(trace.take_long_loop(outer, 6.0)).as_deref(),
            Some("outer")
        );
        trace.end_loop(7.0);
        trace.end_loop(8.0);
        trace.begin_loop("windowMove", 10.0, "");
        assert_eq!(epoch(trace.end_loop(16.0)).as_deref(), Some(""));
    }

    #[test]
    fn a_loop_of_no_length_is_finished() {
        let trace = Trace::default();
        trace.begin_loop("pipDrag", 3.0, "e");
        trace.end_loop(3.0);
        assert_eq!(
            trace.loop_line(4.0),
            "press loop none, finished 1, longest pipDrag 0.0s"
        );
    }
}
