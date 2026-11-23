//! The Rust port of `packages/core/src/fuzzy.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

/// JS `String.prototype.trim` / `\s` whitespace (Rust's `char::is_whitespace` differs: U+0085, U+FEFF).
fn is_js_space(c: char) -> bool {
    matches!(c, '\t'..='\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}'
        | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

/// `text.split(/[^a-z0-9]+/).filter(Boolean)`.
fn words(text: &str) -> Vec<&str> {
    text.split(|c: char| !c.is_ascii_lowercase() && !c.is_ascii_digit())
        .filter(|w| !w.is_empty())
        .collect()
}

fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut rest = haystack.as_bytes();
    // Tokens come from `words`, so they are ASCII.
    needle
        .bytes()
        .all(|b| match rest.iter().position(|&h| h == b) {
            Some(at) => {
                rest = &rest[at + 1..];
                true
            }
            None => false,
        })
}

fn plain(t: &str) -> String {
    t.chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        .collect()
}

/// Shared scoring of `fuzzyScore` and `scoreName`; `q` and `t` are already lowercased.
fn score_text(q: &str, tokens: &[&str], t: &str) -> i64 {
    if t == q {
        return 1000;
    }
    let tw = words(t);
    let mut score = if t.starts_with(q) { 200 } else { 0 };
    let mut plain_t: Option<String> = None;
    for token in tokens {
        let len = token.len() as i64; // ASCII
        if let Some(i) = tw.iter().position(|w| w.starts_with(token)) {
            score += 60 + len * 4 - i as i64 * 2;
        } else if len >= 3 && t.contains(token) {
            score += 25 + len;
        } else if len >= 2 && is_subsequence(token, plain_t.get_or_insert_with(|| plain(t))) {
            score += 8;
        } else {
            return 0;
        }
    }
    score - tw.len() as i64
}

/// Port of `fuzzyScore` (`packages/core/src/fuzzy.ts:21`).
pub fn fuzzy_score(query: String, text: String) -> Result<f64, String> {
    let q = query.trim_matches(is_js_space).to_lowercase();
    if q.is_empty() {
        return Ok(0.0);
    }
    Ok(score_text(&q, &words(&q), &text.to_lowercase()) as f64)
}

/// The JS error for `value.toLowerCase()` when `value` is not a string.
fn lower(value: Option<&Value>) -> Result<String, String> {
    match value {
        Some(Value::String(s)) => Ok(s.to_lowercase()),
        None | Some(Value::Null) => {
            Err("TypeError: Cannot read properties of undefined (reading 'toLowerCase')".into())
        }
        Some(_) => Err("TypeError: text.toLowerCase is not a function".into()),
    }
}

/// Port of `matchActions` (`packages/core/src/fuzzy.ts:78`).
///
/// shortcut: inputs outside `ActionCandidate` fail only with the errors the recording covers
/// (non-array `actions`, non-object items, non-string titles); a string `keywords` is not spread.
pub fn match_actions(
    query: String,
    actions: Value,
    limit: Option<f64>,
) -> Result<Vec<Value>, String> {
    let q = query.trim_matches(is_js_space).to_lowercase();
    // `replace(/\s+/g, " ")`: a trimmed query has no edge runs, so splitting and joining is equal.
    let q = q
        .split(is_js_space)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if q.encode_utf16().count() < 2 {
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
    let tokens = words(&q);
    let mut hits: Vec<(i64, bool, Value)> = Vec::new();
    for action in actions {
        let Some(obj) = action.as_object() else {
            return Err(if action.is_null() {
                "TypeError: Cannot read properties of null (reading 'title')".into()
            } else {
                "TypeError: Cannot read properties of undefined (reading 'toLowerCase')".into()
            });
        };
        let mut names = vec![lower(obj.get("title"))?];
        match obj.get("keywords") {
            None | Some(Value::Null) => {}
            Some(Value::Array(ks)) => {
                for k in ks {
                    names.push(lower(Some(k))?);
                }
            }
            Some(_) => return Err("TypeError: action.keywords is not iterable".into()),
        }
        let (mut score, mut exact) = (i64::MIN, false);
        for (i, n) in names.iter().enumerate() {
            exact |= *n == q;
            score = score.max(score_text(&q, &tokens, n) - if i > 0 { 5 } else { 0 });
        }
        if score > 0 {
            hits.push((score, exact, action));
        }
    }
    hits.sort_by_key(|h| std::cmp::Reverse(h.0));
    let n = hits.len() as f64;
    let end = match limit.unwrap_or(3.0).trunc() {
        l if l.is_nan() => 0.0,
        l if l < 0.0 => (n + l).max(0.0),
        l => l.min(n),
    } as usize;
    Ok(hits
        .into_iter()
        .take(end)
        .map(|(score, exact, action)| {
            let mut m = Map::new();
            m.insert("action".into(), action);
            m.insert("score".into(), score.into());
            m.insert("exact".into(), exact.into());
            Value::Object(m)
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
