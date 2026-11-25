// Ported from ardia apps/browser/src/lib/pluginWhen.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-state` `shared::plugin_when`, through `pluginWhen.harness.ts`: it
//! gives `WhenParse` as the TS object, and `evaluateWhen` runs on what `parseWhen` made, the only
//! `WhenExpr` callers have. The logic is in `ria_state::shared::plugin_when`.

#![forbid(unsafe_code)]

use ria_core::identity::telemetry_wire::{Json, js_order};
use ria_state::shared::plugin_when::{self as w, Operand, WhenContext, WhenExpr};
use serde_json::{Value, json};

/// A JS value as `Json`, object keys in JS order; `None` for `undefined`. An `undefined` inside
/// an array or object is `null`; a string with lone surrogates loses them (`known-diffs.jsonl`).
fn json_in(v: &Value) -> Option<Json> {
    if crate::js::is_undefined(v) {
        return None;
    }
    if let Some(n) = crate::js::as_number(v) {
        return Some(Json::Number(n));
    }
    if let Some(units) = crate::js::string_units(v) {
        return Some(Json::String(String::from_utf16_lossy(&units)));
    }
    Some(match v {
        Value::Bool(b) => Json::Bool(*b),
        Value::Array(items) => Json::Array(items.iter().map(json_or_null).collect()),
        Value::Object(o) => {
            let entries: Vec<(String, Json)> = o
                .iter()
                .map(|(k, x)| (k.clone(), json_or_null(x)))
                .collect();
            Json::Object(js_order(&entries).into_iter().cloned().collect())
        }
        _ => Json::Null,
    })
}

fn json_or_null(v: &Value) -> Json {
    json_in(v).unwrap_or(Json::Null)
}

/// `Json` back as a JS value.
fn json_out(v: &Json) -> Value {
    match v {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => crate::js::number(*n),
        Json::String(s) => Value::String(s.clone()),
        Json::Array(items) => Value::Array(items.iter().map(json_out).collect()),
        Json::Object(entries) => Value::Object(
            entries
                .iter()
                .map(|(k, x)| (k.clone(), json_out(x)))
                .collect(),
        ),
    }
}

fn operand_out(o: &Operand) -> Value {
    match o {
        Operand::Literal(value) => json!({ "kind": "literal", "value": json_out(value) }),
        Operand::Key(key) => json!({ "kind": "key", "key": key }),
    }
}

fn expr_out(e: &WhenExpr) -> Value {
    match e {
        WhenExpr::Operand(o) => operand_out(o),
        WhenExpr::Not(of) => json!({ "kind": "not", "of": expr_out(of) }),
        WhenExpr::And(l, r) => json!({ "kind": "and", "left": expr_out(l), "right": expr_out(r) }),
        WhenExpr::Or(l, r) => json!({ "kind": "or", "left": expr_out(l), "right": expr_out(r) }),
        WhenExpr::Compare { op, left, right } => json!({
            "kind": "compare", "op": op, "left": operand_out(left), "right": operand_out(right)
        }),
    }
}

fn context_in(context: &Value) -> WhenContext {
    match json_in(context) {
        Some(Json::Object(fields)) => fields,
        _ => WhenContext::new(),
    }
}

/// Port of `parseWhen` (`apps/browser/src/lib/pluginWhen.harness.ts:3`).
pub fn parse_when(source: String) -> Result<Value, String> {
    Ok(match w::parse_when(&source) {
        Ok(expr) => json!({ "ok": true, "expr": expr_out(&expr) }),
        Err(problem) => json!({ "ok": false, "problem": problem }),
    })
}

/// Port of `globMatches` (`apps/browser/src/lib/pluginWhen.harness.ts:4`).
pub fn glob_matches(pattern: String, value: String) -> Result<bool, String> {
    Ok(w::glob_matches(&pattern, &value))
}

/// Port of `whenHolds` (`apps/browser/src/lib/pluginWhen.harness.ts:5`).
pub fn when_holds(source: Option<String>, context: Value) -> Result<bool, String> {
    Ok(w::when_holds(source.as_deref(), &context_in(&context)))
}

/// Port of `evaluateWhenCase` (`apps/browser/src/lib/pluginWhen.harness.ts:9`).
pub fn evaluate_when_case(source: String, context: Value) -> Result<Value, String> {
    Ok(match w::parse_when(&source) {
        Ok(expr) => json!(w::evaluate_when(&expr, &context_in(&context))),
        Err(problem) => json!(problem),
    })
}
