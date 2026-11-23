// Ported from ardia packages/core/src/calculator.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `omnibox::calculator`: it maps the result to the JSON
//! value of the original. The logic is in `ria_core::omnibox::calculator`.

#![forbid(unsafe_code)]

use ria_core::omnibox::calculator;

/// Port of `evaluate` (`packages/core/src/calculator.ts:203`).
pub fn evaluate(expression: String) -> Result<Option<f64>, String> {
    Ok(calculator::evaluate(&expression))
}

/// Port of `calculate` (`packages/core/src/calculator.ts:306`).
pub fn calculate(raw: String) -> Result<Option<serde_json::Value>, String> {
    Ok(calculator::calculate(&raw).map(|c| {
        serde_json::json!({ "expression": c.expression, "value": crate::js::number(c.value), "display": c.display })
    }))
}
