// Ported from ardia packages/core/src/fuzzy.reference.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Test oracle: a literal, unoptimised transcription of `fuzzy.reference.ts`, compared
//! with `fuzzy.rs` over generated inputs. Compiled only for tests.

use crate::fuzzy::{ActionCandidate, fuzzy_score, is_js_space, match_actions};

fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_ascii_lowercase() && !c.is_ascii_digit())
        .filter(|w| !w.is_empty())
        .map(String::from)
        .collect()
}

fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let needle: Vec<char> = needle.chars().collect();
    let mut i = 0;
    for ch in haystack.chars() {
        if needle.get(i) == Some(&ch) {
            i += 1;
        }
    }
    i == needle.len()
}

fn reference_score(query: &str, text: &str) -> i64 {
    let q = query.trim_matches(is_js_space).to_lowercase();
    let t = text.to_lowercase();
    if q.is_empty() {
        return 0;
    }
    if t == q {
        return 1000;
    }
    let tw = words(&t);
    let mut score = if t.starts_with(&q) { 200 } else { 0 };
    for token in words(&q) {
        let n = token.len() as i64;
        if let Some(i) = tw.iter().position(|w| w.starts_with(&token)) {
            score += 60 + n * 4 - i as i64 * 2;
        } else if n >= 3 && t.contains(&token) {
            score += 25 + n;
        } else if n >= 2
            && is_subsequence(
                &token,
                &t.replace(|c: char| !c.is_ascii_lowercase() && !c.is_ascii_digit(), ""),
            )
        {
            score += 8;
        } else {
            return 0;
        }
    }
    score - tw.len() as i64
}

struct Action(String, Vec<String>);

impl ActionCandidate for Action {
    fn title(&self) -> &str {
        &self.0
    }
    fn keywords(&self) -> &[String] {
        &self.1
    }
}

/// `matchActions` as the reference writes it: map, filter, stable sort, slice.
fn reference_match(query: &str, actions: &[Action], limit: usize) -> Vec<(usize, i64, bool)> {
    let q = query.trim_matches(is_js_space).to_lowercase();
    let q = q
        .split(is_js_space)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if q.encode_utf16().count() < 2 {
        return vec![];
    }
    let mut out: Vec<(usize, i64, bool)> = actions
        .iter()
        .enumerate()
        .map(|(k, a)| {
            let names: Vec<&str> = std::iter::once(a.0.as_str())
                .chain(a.1.iter().map(String::as_str))
                .collect();
            let exact = names.iter().any(|n| n.to_lowercase() == q);
            let score = names
                .iter()
                .enumerate()
                .map(|(i, n)| reference_score(&q, n) - if i > 0 { 5 } else { 0 })
                .max()
                .unwrap_or(i64::MIN);
            (k, score, exact)
        })
        .filter(|m| m.1 > 0)
        .collect();
    out.sort_by_key(|m| std::cmp::Reverse(m.1));
    out.truncate(limit);
    out
}

/// Deterministic generator over an alphabet that hits each branch: ASCII, spaces,
/// accents, `İ`, Kelvin sign, final sigma, CJK, emoji, U+FEFF, U+0085.
struct Gen(u64);

impl Gen {
    fn next(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as usize % n
    }

    fn string(&mut self, max: usize) -> String {
        const ALPHA: [&str; 22] = [
            "a", "b", "c", "n", "t", "e", "w", "0", "1", " ", "  ", "-", "é", "e\u{301}", "İ",
            "\u{212a}", "Σ", "ς", "日", "😀", "\u{feff}", "\u{85}",
        ];
        let len = self.next(max + 1);
        (0..len).map(|_| ALPHA[self.next(ALPHA.len())]).collect()
    }
}

#[test]
fn fuzzy_score_equals_reference() {
    let mut g = Gen(1);
    for _ in 0..20_000 {
        let (q, t) = (g.string(6), g.string(12));
        assert_eq!(fuzzy_score(&q, &t), reference_score(&q, &t), "{q:?} {t:?}");
    }
}

#[test]
fn match_actions_equals_reference() {
    let mut g = Gen(2);
    for _ in 0..5_000 {
        let q = g.string(6);
        let actions: Vec<Action> = (0..g.next(6))
            .map(|_| Action(g.string(10), (0..g.next(3)).map(|_| g.string(8)).collect()))
            .collect();
        let limit = g.next(5);
        let got: Vec<_> = match_actions(&q, &actions, limit)
            .iter()
            .map(|m| {
                (
                    actions
                        .iter()
                        .position(|a| std::ptr::eq(a, m.action))
                        .unwrap(),
                    m.score,
                    m.exact,
                )
            })
            .collect();
        assert_eq!(got, reference_match(&q, &actions, limit), "{q:?}");
    }
}
