// Ported from ardia apps/browser/src/lib/badgeFit.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The padding and scale that fit badge text in the room it has.
//!
//! Changes from Ardia:
//! - Text lengths count UTF-16 code units, as JavaScript `length` does.
//! - `Math.max` keeps JavaScript's `NaN` and `-0` rules through `js_max`.

use super::js::js_max;

/// `BadgeFit` (`badgeFit.ts:1`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BadgeFit {
    pub pad: f64,
    pub scale: f64,
}

/// `BadgeFitInput` (`badgeFit.ts:3`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BadgeFitInput {
    pub text_width: f64,
    pub room: f64,
    pub roomy_pad: f64,
    pub tight_pad: f64,
    pub min_scale: f64,
}

/// `SHRINK_SLACK` (`badgeFit.ts:5`).
const SHRINK_SLACK: f64 = 1.0;

/// `badgeFit` (`badgeFit.ts:7`): roomy padding when it fits, else tight padding, shrunk when the text is still too wide.
pub fn badge_fit(input: &BadgeFitInput) -> BadgeFit {
    let BadgeFitInput {
        text_width,
        room,
        roomy_pad,
        tight_pad,
        min_scale,
    } = *input;
    if text_width + 2.0 * roomy_pad <= room {
        return BadgeFit {
            pad: roomy_pad,
            scale: 1.0,
        };
    }
    let tight_room = room - 2.0 * tight_pad;
    if text_width <= tight_room {
        return BadgeFit {
            pad: tight_pad,
            scale: 1.0,
        };
    }
    BadgeFit {
        pad: tight_pad,
        scale: js_max(min_scale, (tight_room - SHRINK_SLACK) / text_width),
    }
}

/// `MeasuredBadgeText` (`badgeFit.ts:14`).
#[derive(Clone, Debug, PartialEq)]
pub struct MeasuredBadgeText {
    pub text: String,
    pub font_size: f64,
    pub width: f64,
}

/// `badgeTextWidth` (`badgeFit.ts:16`): the measured width, or an estimate scaled from the last measurement.
pub fn badge_text_width(measured: &MeasuredBadgeText, text: &str, font_size: f64) -> f64 {
    if measured.text == text && measured.font_size == font_size {
        return measured.width;
    }
    let measured_len = measured.text.encode_utf16().count() as f64;
    // `!measured.fontSize` is true for 0, -0 and NaN.
    if measured_len == 0.0 || measured.font_size == 0.0 || measured.font_size.is_nan() {
        return 0.0;
    }
    measured.width
        * (text.encode_utf16().count() as f64 / measured_len)
        * (font_size / measured.font_size)
}
