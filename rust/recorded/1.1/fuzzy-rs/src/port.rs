// Ported from ardia packages/core/src/fuzzy.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! JS-shaped binding of `ria-core` `fuzzy`: it maps the JSON arguments of the recording
//! to typed values and gives the original's JS errors. The logic is in `ria_core::fuzzy`.

#![forbid(unsafe_code)]

use ria_core::fuzzy::{self, ActionCandidate};
use serde_json::{Map, Value};

/// Port of `fuzzyScore` (`packages/core/src/fuzzy.ts:21`).
pub fn fuzzy_score(query: String, text: String) -> Result<f64, String> {
    Ok(fuzzy::fuzzy_score(&query, &text) as f64)
}

struct Candidate {
    action: Value,
    title: String,
    keywords: Vec<String>,
}

impl ActionCandidate for Candidate {
    fn title(&self) -> &str {
        &self.title
    }
    fn keywords(&self) -> &[String] {
        &self.keywords
    }
}

/// The JS error for `value.toLowerCase()` when `value` is not a string.
fn string_of(value: Option<&Value>) -> Result<String, String> {
    match value {
        Some(Value::String(s)) => Ok(s.clone()),
        None | Some(Value::Null) => {
            Err("TypeError: Cannot read properties of undefined (reading 'toLowerCase')".into())
        }
        Some(_) => Err("TypeError: text.toLowerCase is not a function".into()),
    }
}

/// Reads one action as the original does, with its JS errors.
fn candidate(action: &Value) -> Result<Candidate, String> {
    let Some(obj) = action.as_object() else {
        return Err(if action.is_null() {
            "TypeError: Cannot read properties of null (reading 'title')".into()
        } else {
            "TypeError: Cannot read properties of undefined (reading 'toLowerCase')".into()
        });
    };
    let title = string_of(obj.get("title"))?;
    let keywords = match obj.get("keywords") {
        None | Some(Value::Null) => vec![],
        Some(Value::Array(ks)) => ks
            .iter()
            .map(|k| string_of(Some(k)))
            .collect::<Result<_, _>>()?,
        Some(_) => return Err("TypeError: action.keywords is not iterable".into()),
    };
    Ok(Candidate {
        action: action.clone(),
        title,
        keywords,
    })
}

/// Port of `matchActions` (`packages/core/src/fuzzy.ts:78`); the JS `slice(0, limit)` is applied here.
///
/// shortcut: inputs outside `ActionCandidate` fail only with the errors the recording covers
/// (non-array `actions`, non-object items, non-string titles); a string `keywords` is not spread.
pub fn match_actions(
    query: String,
    actions: Value,
    limit: Option<f64>,
) -> Result<Vec<Value>, String> {
    if fuzzy::normalize_query(&query).is_none() {
        return Ok(vec![]);
    }
    let Value::Array(actions) = actions else {
        // The WeakMap cache key is `actions`; an object gets past it and fails on `list.length`.
        return Err(if actions.is_object() {
            "RangeError: Invalid array length".into()
        } else {
            "TypeError: Invalid value used as weak map key".into()
        });
    };
    let candidates = actions
        .iter()
        .map(candidate)
        .collect::<Result<Vec<_>, _>>()?;
    let hits = fuzzy::match_actions(&query, &candidates, usize::MAX);
    let n = hits.len() as f64;
    let end = match limit.unwrap_or(fuzzy::DEFAULT_LIMIT as f64).trunc() {
        l if l.is_nan() => 0.0,
        l if l < 0.0 => (n + l).max(0.0),
        l => l.min(n),
    } as usize;
    Ok(hits
        .into_iter()
        .take(end)
        .map(|m| {
            let mut o = Map::new();
            o.insert("action".into(), m.action.action.clone());
            o.insert("score".into(), m.score.into());
            o.insert("exact".into(), m.exact.into());
            Value::Object(o)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores() {
        assert_eq!(fuzzy_score("new tab".into(), "New Tab".into()), Ok(1000.0));
        assert_eq!(fuzzy_score("nt".into(), "New Tab".into()), Ok(6.0));
        assert_eq!(fuzzy_score("zz".into(), "New Tab".into()), Ok(0.0));
        assert_eq!(fuzzy_score("   ".into(), "x".into()), Ok(0.0));
    }

    #[test]
    fn limit_negative_drops_from_end() {
        let acts = Value::Array(
            ["aa x", "aa y", "aa z"]
                .iter()
                .map(|t| serde_json::json!({"id": t, "title": t}))
                .collect(),
        );
        assert_eq!(
            match_actions("aa".into(), acts, Some(-1.0)).unwrap().len(),
            2
        );
    }
}
