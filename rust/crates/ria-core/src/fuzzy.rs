// Ported from ardia packages/core/src/fuzzy.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Fuzzy scoring of command names and action matching.

/// An action that `match_actions` can rank: its title and its extra search keywords.
pub trait ActionCandidate {
    fn title(&self) -> &str;
    fn keywords(&self) -> &[String];
}

#[derive(Debug)]
pub struct ActionMatch<'a, T> {
    pub action: &'a T,
    pub score: i64,
    pub exact: bool,
}

/// JS `String.prototype.trim` / `\s` whitespace (Rust's `char::is_whitespace` differs: U+0085, U+FEFF).
pub(crate) fn is_js_space(c: char) -> bool {
    matches!(c, '\t'..='\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}'
        | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

/// `text.split(/[^a-z0-9]+/).filter(Boolean)`.
fn words(text: &str) -> Vec<&str> {
    text.split(|c: char| !c.is_ascii_lowercase() && !c.is_ascii_digit())
        .filter(|w| !w.is_empty())
        .collect()
}

/// `needle` is ASCII (a token of `words`).
fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut rest = haystack.as_bytes();
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

/// The scoring of `fuzzyScore` and of each name in `matchActions`; `q` and `t` are lowercased.
fn score_text(q: &str, tokens: &[&str], t: &str) -> i64 {
    if t == q {
        return 1000;
    }
    let tw = words(t);
    let mut score = if t.starts_with(q) { 200 } else { 0 };
    let mut plain: Option<String> = None;
    for token in tokens {
        let len = token.len() as i64; // ASCII
        if let Some(i) = tw.iter().position(|w| w.starts_with(token)) {
            score += 60 + len * 4 - i as i64 * 2;
        } else if len >= 3 && t.contains(token) {
            score += 25 + len;
        } else if len >= 2
            && is_subsequence(
                token,
                plain.get_or_insert_with(|| {
                    t.chars()
                        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                        .collect()
                }),
            )
        {
            score += 8;
        } else {
            return 0;
        }
    }
    score - tw.len() as i64
}

/// Port of `fuzzyScore` (`fuzzy.ts:21`).
pub fn fuzzy_score(query: &str, text: &str) -> i64 {
    let q = query.trim_matches(is_js_space).to_lowercase();
    if q.is_empty() {
        return 0;
    }
    score_text(&q, &words(&q), &text.to_lowercase())
}

/// The default `limit` of `matchActions`.
pub const DEFAULT_LIMIT: usize = 3;

/// The query of `matchActions`: trimmed, lowercased, white space runs as one space.
/// `None` when it has fewer than 2 UTF-16 units, which matches nothing.
pub fn normalize_query(query: &str) -> Option<String> {
    let q = query.trim_matches(is_js_space).to_lowercase();
    // `replace(/\s+/g, " ")`: a trimmed query has no edge runs, so split and join is equal.
    let q = q
        .split(is_js_space)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    (q.encode_utf16().count() >= 2).then_some(q)
}

/// Port of `matchActions` (`fuzzy.ts:78`): the best `limit` actions with a score above 0,
/// best first, ties in input order.
pub fn match_actions<'a, T: ActionCandidate>(
    query: &str,
    actions: &'a [T],
    limit: usize,
) -> Vec<ActionMatch<'a, T>> {
    let Some(q) = normalize_query(query) else {
        return vec![];
    };
    let tokens = words(&q);
    let mut hits = Vec::new();
    for action in actions {
        let names =
            std::iter::once(action.title()).chain(action.keywords().iter().map(String::as_str));
        let (mut score, mut exact) = (i64::MIN, false);
        for (i, n) in names.enumerate() {
            let n = n.to_lowercase();
            exact |= n == q;
            score = score.max(score_text(&q, &tokens, &n) - if i > 0 { 5 } else { 0 });
        }
        if score > 0 {
            hits.push(ActionMatch {
                action,
                score,
                exact,
            });
        }
    }
    hits.sort_by_key(|h| std::cmp::Reverse(h.score));
    hits.truncate(limit);
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    struct A(String, Vec<String>);
    impl ActionCandidate for A {
        fn title(&self) -> &str {
            &self.0
        }
        fn keywords(&self) -> &[String] {
            &self.1
        }
    }

    #[test]
    fn scores() {
        assert_eq!(fuzzy_score("new tab", "New Tab"), 1000);
        assert_eq!(fuzzy_score("nt", "New Tab"), 6);
        assert_eq!(fuzzy_score("zz", "New Tab"), 0);
        assert_eq!(fuzzy_score("   ", "x"), 0);
    }

    #[test]
    fn exact_keyword_outranks_partial_title() {
        let acts = [
            A("Open settings".into(), vec![]),
            A("Preferences".into(), vec!["settings".into()]),
        ];
        let m = match_actions("settings", &acts, DEFAULT_LIMIT);
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].action.0, "Preferences");
        assert!(m[0].exact && !m[1].exact);
        assert_eq!(match_actions("settings", &acts, 1).len(), 1);
        assert!(match_actions("s", &acts, 3).is_empty());
    }
}
