// Ported from ardia packages/ardiacore/ios/ArdiaPiPController+Skip.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use objc2_foundation::NSPoint;
use serde_json::{Value, json};

use super::ardia_picture_in_picture_internal::{self as internal};
use super::ardia_pip_controller::Controller;

impl Controller {
    pub fn skip_by(&self, seconds: f64, at: NSPoint, click_count: i64) -> bool {
        self.skip_by_stamped(seconds, at, click_count, &internal::begin_stamp())
    }

    pub fn skip_dragged_by(&self, seconds: f64, at: NSPoint, click_count: i64) -> bool {
        let stamp = self.state.drag_stamp.borrow().clone();
        self.skip_by_stamped(seconds, at, click_count, &stamp)
    }

    pub fn skip_by_stamped(
        &self,
        seconds: f64,
        at: NSPoint,
        click_count: i64,
        stamp: &Value,
    ) -> bool {
        let Some(view) = self.view() else {
            return false;
        };
        if seconds == 0.0 {
            return false;
        }
        view.seek_picture_in_picture_by(seconds);
        view.emit(
            "pictureInPictureSkip",
            internal::stamp_onto(stamp, json!({ "seconds": seconds })),
        );
        let skips = self.state.skips.get() + 1;
        self.state.skips.set(skips);
        *self.state.last_skip.borrow_mut() = json!({
            "seconds": seconds,
            "point": internal::point_string(at),
            "clickCount": click_count,
            "count": skips,
        });
        true
    }

    pub fn skip_info(&self) -> Value {
        let last_skip = self.state.last_skip.borrow();
        json!({
            "lastSkip": if last_skip.is_null() { json!({}) } else { last_skip.clone() },
            "count": self.state.skips.get(),
        })
    }
}
