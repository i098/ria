// Ported from ardia apps/browser/src/lib/deadline.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::deadline`. The harness (`deadline.harness.ts`) builds the promise from `kind`; here it is a channel. The logic is in `ria_state::shared::deadline`.

#![forbid(unsafe_code)]

use ria_state::shared::deadline::with_deadline;
use serde_json::{Value, json};
use std::sync::mpsc;

/// Port of `withDeadlineCase` (`apps/browser/src/lib/deadline.harness.ts:4`).
pub async fn with_deadline_case(
    kind: String,
    value: Value,
    ms: f64,
    fallback: Value,
) -> Result<Value, String> {
    let (promise, settled) = mpsc::channel::<Result<Value, Value>>();
    match kind.as_str() {
        "never" => drop(promise),
        "reject" => promise.send(Err(value)).map_err(|e| e.to_string())?,
        _ => promise.send(Ok(value)).map_err(|e| e.to_string())?,
    }
    // `Math.min(ms, 20)`: NaN stays NaN.
    let ms = if ms.is_nan() { ms } else { ms.min(20.0) };
    Ok(match with_deadline(&settled, ms, Ok(fallback)) {
        Ok(value) => json!({ "value": value }),
        Err(reason) => json!({ "rejected": reason }),
    })
}
