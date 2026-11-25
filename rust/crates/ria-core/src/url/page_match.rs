// Ported from ardia packages/core/src/pageMatch.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Scoring of a page row against a query, the narrowing chain of earlier results, and the
//! top-N list.
//!
//! The original keeps the chain in a module variable and compares pools by identity; here
//! `MatchedChain` is a value that the caller owns, and `P` compares pools (an id, or a
//! pointer wrapper). `C` is the `Candidate` of `candidatePool.ts`, which another module ports.
//!
//! Changes from Ardia: `match_score` and `match_score_word` are split into helpers for the
//! complexity gate, with unchanged behavior.

use super::js_len;
use super::page_text::PageText;
use crate::fuzzy::is_js_space;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryToken {
    pub text: String,
    pub word: bool,
    pub spaced: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub q: String,
    pub segment_starts: Option<(String, String)>,
    pub word: bool,
    pub spaced: String,
    pub tokens: Vec<QueryToken>,
    pub needle: String,
}

/// `/^[a-z0-9]+$/`.
fn is_word(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

/// Port of `parseQuery` (`pageMatch.ts:15`).
pub fn parse_query(q: &str, tokens: &[&str]) -> Query {
    let mut needle = "";
    for t in tokens {
        if js_len(t) > js_len(needle) {
            needle = t;
        }
    }
    Query {
        q: q.to_string(),
        segment_starts: (!q.contains(['.', '/'])).then(|| (format!(".{q}"), format!("/{q}"))),
        word: is_word(q),
        spaced: format!(" {q}"),
        tokens: tokens
            .iter()
            .map(|t| QueryToken {
                text: (*t).to_string(),
                word: is_word(t),
                spaced: format!(" {t}"),
            })
            .collect(),
        needle: needle.to_string(),
    }
}

/// The `matchScore` points for where `q` falls in a shown text that has a blank.
fn shown_score(shown: &str, query: &Query) -> f64 {
    let q = query.q.as_str();
    if shown.starts_with(q) {
        100.0
    } else if query
        .segment_starts
        .as_ref()
        .is_some_and(|(a, b)| shown.contains(a) || shown.contains(b))
    {
        60.0
    } else if shown.contains(q) {
        30.0
    } else {
        0.0
    }
}

/// The `matchScore` points when every token matches somewhere, else `0`.
fn tokens_score(t: &mut PageText, query: &Query) -> f64 {
    let mut s = 0.0;
    for token in &query.tokens {
        if token.word && t.title_words.contains(&token.spaced) {
            s += 15.0;
        } else if token.word && t.shown_words().contains(&token.spaced) {
            s += 10.0;
        } else if js_len(&token.text) >= 3
            && (t.title_lower.contains(&token.text) || t.shown_lower.contains(&token.text))
        {
            s += 5.0;
        } else {
            return 0.0;
        }
    }
    s
}

/// Port of `matchScore` (`pageMatch.ts:26`).
pub fn match_score(t: &mut PageText, query: &Query) -> f64 {
    let q = query.q.as_str();
    if !t.shown_lower.contains(&query.needle) && !t.title_lower.contains(&query.needle) {
        return 0.0;
    }
    let mut s = 0.0;
    let blank = *t
        .shown_blank
        .get_or_insert_with(|| t.shown_lower.contains(is_js_space));
    if blank {
        s = shown_score(&t.shown_lower, query);
    }
    match t.title_lower.find(q) {
        Some(0) => s += 40.0,
        Some(_) => s += 20.0,
        None => {}
    }
    if s > 0.0 {
        return s;
    }
    tokens_score(t, query)
}

/// Port of `matchScoreWord` (`pageMatch.ts:51`).
pub fn match_score_word(t: &PageText, query: &Query) -> f64 {
    let q = query.q.as_str();
    let si = t.shown_lower.find(q);
    let ti = t.title_lower.find(q);
    if si.is_none() && ti.is_none() {
        return 0.0;
    }
    let mut s = 0.0;
    match si {
        Some(0) => s = 100.0,
        Some(_) => {
            if query
                .segment_starts
                .as_ref()
                .is_some_and(|(a, b)| t.shown_lower.contains(a) || t.shown_lower.contains(b))
            {
                s = 60.0;
            } else if js_len(q) >= 2 {
                s = 30.0;
            }
        }
        None => {}
    }
    s + title_word_score(t, query, ti)
}

/// The `matchScoreWord` points for the title, given where `q` falls in it.
fn title_word_score(t: &PageText, query: &Query, ti: Option<usize>) -> f64 {
    if ti == Some(0) {
        40.0
    } else if if js_len(&query.q) >= 2 {
        ti.is_some()
    } else {
        query.word && t.title_words.contains(&query.spaced)
    } {
        20.0
    } else {
        0.0
    }
}

/// One remembered query result.
#[derive(Debug, Clone)]
pub struct Matched<C> {
    pub q: String,
    pub tokens: Vec<String>,
    pub pages: Vec<C>,
    pub scores: Vec<f64>,
}

const MATCHED_KEPT: usize = 8;

/// The `matchedChain` of the original: results of earlier queries on one pool.
#[derive(Debug)]
pub struct MatchedChain<P, C> {
    pool: Option<P>,
    list: Vec<Matched<C>>,
}

impl<P, C> Default for MatchedChain<P, C> {
    fn default() -> Self {
        Self {
            pool: None,
            list: Vec::new(),
        }
    }
}

impl<P: PartialEq, C> MatchedChain<P, C> {
    /// Port of `narrowing` (`pageMatch.ts:74`): the latest earlier result that `q` extends.
    pub fn narrowing(&self, pool: &P, q: &str, tokens: &[&str]) -> Option<&Matched<C>> {
        if self.pool.as_ref() != Some(pool) {
            return None;
        }
        self.list.iter().rev().find(|last| {
            if !q.starts_with(&last.q) {
                return false;
            }
            // `tokens[i]` of an empty `last.tokens` is `undefined` on both sides: equal.
            let Some(i) = last.tokens.len().checked_sub(1) else {
                return true;
            };
            let word = &last.tokens[i];
            i == 0 || tokens.get(i).is_some_and(|t| t == word) || js_len(word) >= 3
        })
    }

    /// Port of `remember` (`pageMatch.ts:87`).
    pub fn remember(
        &mut self,
        pool: P,
        q: &str,
        tokens: Vec<String>,
        pages: Vec<C>,
        scores: Vec<f64>,
    ) {
        if self.pool.as_ref() != Some(&pool) {
            *self = Self {
                pool: Some(pool),
                list: Vec::new(),
            };
        }
        while self.list.last().is_some_and(|last| !q.starts_with(&last.q)) {
            self.list.pop();
        }
        if js_len(q) < 2 {
            self.list.clear();
            return;
        }
        if self.list.last().is_some_and(|last| last.q == q) {
            self.list.pop();
        }
        self.list.push(Matched {
            q: q.to_string(),
            tokens,
            pages,
            scores,
        });
        if self.list.len() > MATCHED_KEPT {
            self.list.remove(0);
        }
    }
}

/// `Ranked`: a candidate with its match rank `m` and sort score `s`.
#[derive(Debug, Clone)]
pub struct Ranked<C> {
    pub c: C,
    pub tab_id: Option<String>,
    pub m: f64,
    pub s: f64,
}

/// Port of `TopList` (`pageMatch.ts:103`): the best `size` offers, best score first.
#[derive(Debug)]
pub struct TopList<C> {
    pub items: Vec<Ranked<C>>,
    pub floor: f64,
    size: usize,
}

impl<C> TopList<C> {
    pub fn new(size: usize) -> Self {
        Self {
            items: Vec::new(),
            floor: f64::NEG_INFINITY,
            size,
        }
    }

    pub fn offer(&mut self, r: Ranked<C>) {
        if self.size == 0
            || (self.items.len() == self.size && self.items[self.items.len() - 1].s >= r.s)
        {
            return;
        }
        let mut i = self.items.len();
        while i > 0 && self.items[i - 1].s < r.s {
            i -= 1;
        }
        self.items.insert(i, r);
        if self.items.len() > self.size {
            self.items.pop();
        }
        if self.items.len() == self.size {
            self.floor = self.items[self.items.len() - 1].s;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(url: &str, title: &str) -> PageText {
        PageText::new(url, title, url.to_string())
    }

    #[test]
    fn narrowing_follows_the_latest_prefix_and_drops_on_a_new_pool() {
        let mut chain: MatchedChain<u32, &str> = MatchedChain::default();
        chain.remember(1, "ab", vec!["ab".into()], vec!["x"], vec![1.0]);
        chain.remember(1, "abc", vec!["abc".into()], vec!["y"], vec![2.0]);
        let hit = chain.narrowing(&1, "abcd", &["abcd"]).expect("hit");
        assert_eq!(hit.q, "abc");
        assert!(chain.narrowing(&2, "abcd", &["abcd"]).is_none());
        // a query that does not extend the last one removes it first
        chain.remember(1, "ax", vec!["ax".into()], vec!["z"], vec![3.0]);
        assert_eq!(chain.narrowing(&1, "axe", &["axe"]).expect("hit").q, "ax");
        // a one-unit query clears the chain
        chain.remember(1, "a", vec![], vec![], vec![]);
        assert!(chain.narrowing(&1, "ab", &["ab"]).is_none());
    }

    #[test]
    fn top_list_keeps_the_best_in_order_and_sets_the_floor() {
        let mut top = TopList::new(2);
        for (c, s) in [("a", 1.0), ("b", 3.0), ("c", 2.0), ("d", 0.5)] {
            top.offer(Ranked {
                c,
                tab_id: None,
                m: 0.0,
                s,
            });
        }
        let order: Vec<_> = top.items.iter().map(|r| r.c).collect();
        assert_eq!(order, ["b", "c"]);
        assert_eq!(top.floor, 2.0);
    }

    #[test]
    fn scores_match_the_original_branches() {
        let q = parse_query("git", &["git"]);
        assert_eq!(match_score(&mut page("github.com/x", "GitHub"), &q), 40.0);
        let word = parse_query("gi", &["gi"]);
        assert_eq!(match_score_word(&page("github.com", "x"), &word), 100.0);
    }
}
