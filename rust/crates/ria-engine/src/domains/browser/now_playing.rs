// Ported from ardia packages/ardiacore/ios/ArdiaNowPlaying.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The media state of the frames of one tab, and the choice of the frame that the system player shows.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value, json};
use unicode_segmentation::UnicodeSegmentation;

/// `s` cut at the start of the character that holds UTF-16 unit `max` (`rangeOfComposedCharacterSequenceAtIndex:`).
/// A string of `max` units or fewer stays whole.
fn truncate_utf16(s: &str, max: usize) -> &str {
    if s.encode_utf16().count() <= max {
        return s;
    }
    let mut units = 0;
    for (i, grapheme) in s.grapheme_indices(true) {
        units += grapheme.encode_utf16().count();
        if units > max {
            return &s[..i];
        }
    }
    s
}

fn text(d: &Map<String, Value>, key: &str, max: usize) -> Option<String> {
    let s = d.get(key)?.as_str()?;
    Some(truncate_utf16(s, max).to_owned())
}

/// A JSON number or boolean as `NSNumber` reads it.
fn number(d: &Map<String, Value>, key: &str) -> Option<f64> {
    match d.get(key)? {
        Value::Number(n) => n.as_f64(),
        Value::Bool(b) => Some(f64::from(u8::from(*b))),
        _ => None,
    }
}

fn flag(d: &Map<String, Value>, key: &str) -> bool {
    number(d, key).is_some_and(|n| n != 0.0)
}

fn sanitized(d: &Map<String, Value>) -> Value {
    let state = d
        .get("playbackState")
        .and_then(Value::as_str)
        .filter(|s| matches!(*s, "none" | "paused" | "playing"))
        .unwrap_or("none");
    let actions: Vec<&str> = d
        .get("actions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|a| a.encode_utf16().count() <= 64)
        .take(32)
        .collect();
    let duration = number(d, "duration").filter(|d| *d >= 0.0);
    json!({
        "title": text(d, "title", 1024).unwrap_or_default(),
        "artist": text(d, "artist", 1024).unwrap_or_default(),
        "album": text(d, "album", 1024).unwrap_or_default(),
        "artwork": text(d, "artwork", 1 << 20),
        "playbackState": state,
        "position": number(d, "position").unwrap_or(0.0).max(0.0),
        "duration": duration,
        "playbackRate": number(d, "playbackRate").unwrap_or(1.0),
        "timestamp": number(d, "timestamp").unwrap_or(0.0),
        "hasVideo": flag(d, "hasVideo"),
        "actions": actions,
        "controllable": flag(d, "controllable"),
        "loop": flag(d, "loop"),
    })
}

#[derive(Debug, Default)]
pub struct NowPlaying {
    frames: BTreeMap<String, Value>,
    main_frames: BTreeSet<String>,
    frame: Option<String>,
}

impl NowPlaying {
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// The frame that is shown now.
    pub fn frame(&self) -> Option<&str> {
        self.frame.as_deref()
    }

    /// The state of the shown frame.
    pub fn selected(&self) -> Option<&Value> {
        self.frames.get(self.frame.as_deref()?)
    }

    /// 4 for a controllable frame, 2 for a playing one, 1 for the main frame; -1 for a frame that is not known.
    fn rank(&self, frame: Option<&str>) -> i32 {
        let Some((frame, state)) = frame.and_then(|f| Some((f, self.frames.get(f)?))) else {
            return -1;
        };
        i32::from(state["controllable"] == true) * 4
            + i32::from(state["playbackState"] == "playing") * 2
            + i32::from(self.main_frames.contains(frame))
    }

    /// Records the state `page` of `frame` (an empty or non-object `page` removes the frame), picks the best frame,
    /// and returns its state.
    pub fn note_frame(&mut self, frame: &str, page: &Value, main: bool) -> Option<&Value> {
        let page = page.as_object().filter(|p| !p.is_empty());
        if let Some(page) = page {
            self.frames.insert(frame.to_owned(), sanitized(page));
            if main {
                self.main_frames.insert(frame.to_owned());
            } else {
                self.main_frames.remove(frame);
            }
        } else {
            self.frames.remove(frame);
            self.main_frames.remove(frame);
        }
        let mut best = if page.is_some() {
            Some(frame.to_owned())
        } else {
            self.frames.keys().next().cloned()
        };
        for other in self.frames.keys() {
            if self.rank(Some(other)) > self.rank(best.as_deref()) {
                best = Some(other.clone());
            }
        }
        self.frame = best;
        self.selected()
    }

    /// Removes each frame. Returns whether there was one.
    pub fn reset(&mut self) -> bool {
        let had = !self.frames.is_empty();
        self.frames.clear();
        self.main_frames.clear();
        self.frame = None;
        had
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_is_clamped_and_cut_to_limits() {
        let mut np = NowPlaying::default();
        let page = json!({
            "title": "t".repeat(2000), "playbackState": "bogus", "position": -5,
            "duration": -1, "actions": ["play", "x".repeat(65)], "hasVideo": 1,
        });
        let state = np.note_frame("a", &page, true).unwrap().clone();
        assert_eq!(state["title"].as_str().unwrap().len(), 1024);
        assert_eq!(state["playbackState"], "none");
        assert_eq!(state["position"], 0.0);
        assert_eq!(state["duration"], Value::Null);
        assert_eq!(state["actions"], json!(["play"]));
        assert_eq!(state["hasVideo"], true);
        assert_eq!(state["playbackRate"], 1.0);
        assert_eq!(state["artwork"], Value::Null);
    }

    #[test]
    fn a_controllable_playing_frame_wins_and_removal_falls_back() {
        let mut np = NowPlaying::default();
        np.note_frame("main", &json!({"title": "m"}), true);
        np.note_frame(
            "sub",
            &json!({"title": "s", "playbackState": "playing", "controllable": true}),
            false,
        );
        assert_eq!(np.frame(), Some("sub"));
        np.note_frame("sub", &json!({}), false);
        assert_eq!(np.frame(), Some("main"));
        assert!(np.reset());
        assert!(np.is_empty() && np.selected().is_none());
        assert!(!np.reset());
    }

    #[test]
    fn truncation_keeps_surrogate_pairs_and_combining_marks_whole() {
        assert_eq!(truncate_utf16("ab\u{1f600}", 3), "ab");
        assert_eq!(truncate_utf16("ab\u{1f600}", 4), "ab\u{1f600}");
        assert_eq!(truncate_utf16("ae\u{301}x", 2), "a");
        assert_eq!(truncate_utf16("ae\u{301}xy", 3), "ae\u{301}");
        assert_eq!(truncate_utf16("ab👍🏽x", 4), "ab");
        assert_eq!(truncate_utf16("ab👩‍💻x", 5), "ab");
        assert_eq!(truncate_utf16("ab🇯🇵x", 4), "ab");
    }
}
