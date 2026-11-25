// Ported from ardia apps/browser/src/lib/badgeFit.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::badge_fit`. The logic is in `ria_state::shared::badge_fit`.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined, number, undefined};
use ria_state::shared::badge_fit::{self as b, BadgeFitInput, MeasuredBadgeText};
use serde_json::{Value, json};

/// A number field of a JS object; a missing one is `undefined`, which arithmetic turns into `NaN`.
fn field(o: &Value, name: &str) -> f64 {
    o.get(name).and_then(as_number).unwrap_or(f64::NAN)
}

/// Port of `badgeFit` (`apps/browser/src/lib/badgeFit.ts:7`).
pub fn badge_fit(arg0: Value) -> Result<Value, String> {
    if arg0.is_null() || is_undefined(&arg0) {
        return Err(
            "TypeError: Cannot destructure property 'textWidth' of 'object null' as it is null."
                .into(),
        );
    }
    let input = BadgeFitInput {
        text_width: field(&arg0, "textWidth"),
        room: field(&arg0, "room"),
        roomy_pad: field(&arg0, "roomyPad"),
        tight_pad: field(&arg0, "tightPad"),
        min_scale: field(&arg0, "minScale"),
    };
    let fit = b::badge_fit(&input);
    // A missing `roomyPad` never fits, so a `NaN` pad with no `tightPad` is Ardia's `pad: undefined`.
    let pad = if fit.pad.is_nan() && arg0.get("tightPad").is_none() {
        undefined()
    } else {
        number(fit.pad)
    };
    Ok(json!({ "pad": pad, "scale": number(fit.scale) }))
}

/// Port of `badgeTextWidth` (`apps/browser/src/lib/badgeFit.ts:16`).
pub fn badge_text_width(measured: Value, text: String, font_size: f64) -> Result<f64, String> {
    let Some(Value::String(measured_text)) = measured.get("text") else {
        return Err("TypeError: Cannot read properties of undefined (reading 'length')".into());
    };
    let measured = MeasuredBadgeText {
        text: measured_text.clone(),
        font_size: field(&measured, "fontSize"),
        width: field(&measured, "width"),
    };
    Ok(b::badge_text_width(&measured, &text, font_size))
}
