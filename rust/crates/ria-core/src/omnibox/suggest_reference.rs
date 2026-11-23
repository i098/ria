// Ported from ardia packages/core/src/suggest.reference.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Test oracle: a literal transcription of `suggest.reference.ts` (no score floor, no cached
//! scores, the older word matcher), compared with `suggest.rs` over generated keystroke
//! sequences, as `suggest.property.test.ts` does. Compiled only for tests.
//!
//! Changes: the row helpers (`pageSuggestion`, `suggestionKey`, `Output`, `createRow`,
//! `actionRow`, `dedupeSuggestions`), `displayUrl` and `tabPageText` come from their ported
//! modules, and `matchActions` from `fuzzy.rs`, which `fuzzy_reference.rs` checks. The pool is
//! rebuilt on each call; the caller passes the id of the pool the original would cache, which
//! `lastMatches` compares.

use std::collections::{HashMap, HashSet};

use super::calculator::calculate;
use super::candidate_pool::page_text;
use super::omnibox::{fixup_url, host_of, same_address, spaced_address_guess};
use super::quick_create::match_quick_create;
use super::suggestion::{
    Preference, SourceTab, SuggestOptions, Suggestion, SuggestionResult, SuggestionSource,
};
use super::suggestion_rows::{
    Output, PageRow, action_row, create_row, dedupe_suggestions, page_suggestion,
};
use crate::fuzzy::match_actions;
use crate::search::bangs::{BangMatch, find_bang};
use crate::search::engines::{BUILT_IN_ENGINES, search_url};
use crate::search::js_text::{is_js_space, js_trim};
use crate::search::site_search::{ScopeKind, SearchScope, scoped_search_url};
use crate::url::js_len;
use crate::url::page_match::{Ranked, TopList};
use crate::url::page_text::{PageText, host_part, page_key, with_root_slash};

const DAY: f64 = 86_400_000.0;

#[derive(Clone)]
struct Cand {
    url: String,
    title: String,
    page_title: Option<String>,
    favicon: Option<String>,
    visits: f64,
    last_visit: f64,
    bookmarked: bool,
    tab_id: Option<String>,
    text: PageText,
}

impl Cand {
    fn row(&self) -> PageRow<'_> {
        PageRow {
            url: &self.url,
            title: &self.title,
            favicon: self.favicon.as_deref(),
            visits: self.visits,
            bookmarked: self.bookmarked,
        }
    }
}

struct RefPool {
    list: Vec<Cand>,
    by_key: HashMap<String, usize>,
}

/// `poolFor`: history pages (first row of each key), then bookmarks of other pages.
fn pool_for(source: &SuggestionSource) -> RefPool {
    let mut base: Vec<Cand> = Vec::new();
    let mut keys = HashSet::new();
    for row in &source.bookmarks {
        let text = page_text(&row.url, &row.title).expect("text");
        if keys.insert(text.key.clone()) {
            base.push(Cand {
                url: row.url.clone(),
                title: row.title.clone(),
                page_title: None,
                favicon: row.favicon.clone(),
                visits: 0.0,
                last_visit: 0.0,
                bookmarked: true,
                tab_id: None,
                text,
            });
        }
    }
    let mut list = Vec::new();
    let mut by_key = HashMap::new();
    for h in &source.history {
        let text = page_text(&h.url, &h.title).expect("text");
        if by_key.contains_key(&text.key) {
            continue;
        }
        by_key.insert(text.key.clone(), list.len());
        list.push(Cand {
            url: h.url.clone(),
            title: h.title.clone(),
            page_title: None,
            favicon: h.favicon.clone(),
            visits: h.visits,
            last_visit: h.last_visit,
            bookmarked: keys.contains(&text.key),
            tab_id: None,
            text,
        });
    }
    for c in base {
        if !by_key.contains_key(&c.text.key) {
            by_key.insert(c.text.key.clone(), list.len());
            list.push(c);
        }
    }
    RefPool { list, by_key }
}

/// `tabPageText`.
fn tab_page_text(t: &SourceTab, listed: Option<&PageText>) -> PageText {
    let text = page_text(&t.url, &t.title).expect("text");
    match t.custom_title.as_deref().filter(|c| !c.is_empty()) {
        None => text,
        Some(custom) => PageText::renamed(&text, custom, listed),
    }
}

struct Tabs {
    tab_ids: HashMap<String, String>,
    extra: Vec<Cand>,
    renamed: HashSet<String>,
}

fn custom(t: &SourceTab) -> Option<&str> {
    t.custom_title.as_deref().filter(|c| !c.is_empty())
}

/// `tabsOver`.
fn tabs_over(pool: &RefPool, tabs: &[SourceTab], now: f64, current: Option<&str>) -> Tabs {
    let mut open = Vec::new();
    let mut chosen: HashMap<String, usize> = HashMap::new();
    for (i, t) in tabs.iter().enumerate() {
        if t.url.is_empty() || current == Some(t.id.as_str()) {
            continue;
        }
        open.push(i);
        let key = page_key(&t.url);
        let replace = chosen
            .get(&key)
            .is_none_or(|&k| custom(t).is_some() && custom(&tabs[k]).is_none());
        if replace {
            chosen.insert(key, i);
        }
    }
    let tab_ids = chosen
        .iter()
        .map(|(k, &i)| (k.clone(), tabs[i].id.clone()))
        .collect();
    let mut extra = Vec::new();
    let mut renamed = HashSet::new();
    for i in open {
        let t = &tabs[i];
        let key = page_key(&t.url);
        match pool.by_key.get(&key).map(|&at| &pool.list[at]) {
            None => {
                if custom(t).is_none() && chosen.get(&key) != Some(&i) {
                    continue;
                }
                extra.push(Cand {
                    url: t.url.clone(),
                    title: custom(t).unwrap_or(&t.title).to_string(),
                    page_title: Some(t.title.clone()),
                    favicon: t.favicon.clone(),
                    visits: 0.0,
                    last_visit: now,
                    bookmarked: false,
                    tab_id: Some(t.id.clone()),
                    text: tab_page_text(t, None),
                });
            }
            Some(listed) => {
                let Some(name) = custom(t) else { continue };
                renamed.insert(key);
                extra.push(Cand {
                    url: t.url.clone(),
                    title: name.to_string(),
                    page_title: None,
                    favicon: t.favicon.clone().or_else(|| listed.favicon.clone()),
                    visits: listed.visits,
                    last_visit: listed.last_visit,
                    bookmarked: listed.bookmarked,
                    tab_id: Some(t.id.clone()),
                    text: tab_page_text(t, Some(&listed.text)),
                });
            }
        }
    }
    Tabs {
        tab_ids,
        extra,
        renamed,
    }
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_digit() || c.is_ascii_lowercase()
}

/// `wordStartsWith`: `word` in `text` at the start or after a character outside `[0-9a-z]`.
fn word_starts_with(text: &str, word: &str) -> bool {
    let mut from = 0;
    while let Some(i) = text
        .get(from..)
        .and_then(|t| t.find(word))
        .map(|i| i + from)
    {
        if text[..i]
            .chars()
            .next_back()
            .is_none_or(|c| !is_word_char(c))
        {
            return true;
        }
        from = i + text[i..].chars().next().map_or(1, char::len_utf8);
    }
    false
}

fn is_word(s: &str) -> bool {
    !s.is_empty() && s.chars().all(is_word_char)
}

struct Query<'a> {
    q: &'a str,
    segment_starts: Option<(String, String)>,
    word: bool,
    tokens: Vec<(&'a str, bool)>,
    needle: &'a str,
}

fn parse_query<'a>(q: &'a str, tokens: &[&'a str]) -> Query<'a> {
    Query {
        q,
        segment_starts: (!q.contains(['.', '/'])).then(|| (format!(".{q}"), format!("/{q}"))),
        word: is_word(q),
        tokens: tokens.iter().map(|t| (*t, is_word(t))).collect(),
        needle: tokens
            .iter()
            .fold("", |a, b| if js_len(b) > js_len(a) { b } else { a }),
    }
}

fn match_score(t: &PageText, query: &Query) -> f64 {
    let shown = &t.shown_lower;
    let title = &t.title_lower;
    let q = query.q;
    if !shown.contains(query.needle) && !title.contains(query.needle) {
        return 0.0;
    }
    let mut s = 0.0;
    if shown.starts_with(q) {
        s = 100.0;
    } else if query
        .segment_starts
        .as_ref()
        .is_some_and(|(a, b)| shown.contains(a) || shown.contains(b))
    {
        s = 60.0;
    } else if js_len(q) >= 2 && shown.contains(q) {
        s = 30.0;
    }
    if title.starts_with(q) {
        s += 40.0;
    } else if if js_len(q) >= 2 {
        title.contains(q)
    } else {
        query.word && word_starts_with(title, q)
    } {
        s += 20.0;
    }
    if s > 0.0 || query.tokens.len() < 2 {
        return s;
    }
    for &(token, word) in &query.tokens {
        if word && word_starts_with(title, token) {
            s += 15.0;
        } else if word && word_starts_with(shown, token) {
            s += 10.0;
        } else if js_len(token) >= 3 && (title.contains(token) || shown.contains(token)) {
            s += 5.0;
        } else {
            return 0.0;
        }
    }
    s
}

/// JS `Math.max(a, b)` and `Math.min(a, b)`: NaN wins.
fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}
fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

fn frecency(c: &Cand, tab_id: &Option<String>, now: f64) -> f64 {
    let age = if c.last_visit != 0.0 && !c.last_visit.is_nan() {
        js_max(0.0, (now - c.last_visit) / DAY)
    } else {
        10.0
    };
    (1.0 + c.visits).log2() * 8.0 - js_min(age, 30.0)
        + if c.bookmarked { 20.0 } else { 0.0 }
        + if tab_id.as_deref().is_some_and(|t| !t.is_empty()) {
            6.0
        } else {
            0.0
        }
}

struct Last {
    pool: u64,
    q: String,
    tokens: Vec<String>,
    pages: Vec<usize>,
}

/// The reference with its module state, `lastMatches`.
#[derive(Default)]
pub(crate) struct Reference {
    last: Option<Last>,
}

#[derive(Clone, Copy)]
enum At {
    Pool(usize),
    Extra(usize),
}

fn inline_match(t: &PageText, q: &str) -> bool {
    if host_part(&t.shown_lower).starts_with(q) && origin(t).is_some() {
        return true;
    }
    q.contains('/') && !t.shown.contains(['?', '#']) && t.shown_lower.starts_with(q)
}

/// `originOfPage`: `/^https?:\/\/[^/?#]+/i.exec(url)?.[0]`.
fn origin(t: &PageText) -> Option<String> {
    let mut t = t.clone();
    t.origin_of_page().map(str::to_string)
}

fn page(url: &str, title: &str, favicon: Option<&str>) -> Suggestion {
    Suggestion::Page {
        url: url.into(),
        title: title.into(),
        favicon: favicon.map(Into::into),
        tab_id: None,
        bookmarked: false,
        visited: false,
    }
}

fn search_row(query: &str, url: String, engine: &str, suggested: bool) -> Suggestion {
    Suggestion::Search {
        query: query.into(),
        url,
        engine: engine.into(),
        suggested,
        bang: false,
    }
}

impl Reference {
    /// `buildSuggestions` of the reference; `pool_id` names the pool of `source`'s history and
    /// bookmarks. Options the reference does not know (extensions, providers, remote sites)
    /// are ignored.
    pub(crate) fn build(
        &mut self,
        raw: &str,
        source: &SuggestionSource,
        options: &SuggestOptions,
        pool_id: u64,
    ) -> SuggestionResult {
        let limit = options.limit;
        let now = options.now.unwrap_or(0.0);
        let (engine_name, engine_url) = options.engine.as_ref().map_or(
            (
                BUILT_IN_ENGINES[0].name.as_str(),
                BUILT_IN_ENGINES[0].url.as_str(),
            ),
            |e| (e.name.as_str(), e.url.as_str()),
        );
        let scope = options.scope.as_ref();
        let query = js_trim(raw);
        if query.is_empty() {
            return SuggestionResult::default();
        }
        let bare = ["https://", "http://"]
            .iter()
            .find_map(|s| {
                let rest = query
                    .get(..s.len())
                    .filter(|h| h.eq_ignore_ascii_case(s))
                    .map(|_| &query[s.len()..])?;
                rest.starts_with(|c: char| !is_js_space(c)).then_some(rest)
            })
            .unwrap_or(query);
        let q = bare.to_lowercase();
        let tokens: Vec<&str> = q.split(is_js_space).filter(|t| !t.is_empty()).collect();

        if scope.is_none()
            && let Some(bang) = find_bang(query, options.bangs)
        {
            return self.bang_suggestions(bang, source, options, pool_id);
        }
        if scope.is_none() && query.starts_with('?') {
            let text = js_trim(&query[1..]);
            if text.is_empty() {
                return SuggestionResult::default();
            }
            let mut out = Output::new(limit);
            out.push(search_row(
                text,
                search_url(engine_url, text),
                engine_name,
                false,
            ));
            for t in dedupe_suggestions(&options.remote, text)
                .into_iter()
                .take(4)
            {
                out.push(search_row(t, search_url(engine_url, t), engine_name, true));
            }
            return SuggestionResult {
                items: out.into_items(),
                ..SuggestionResult::default()
            };
        }

        let pool = pool_for(source);
        let tabs = tabs_over(&pool, &source.tabs, now, options.current_tab_id.as_deref());
        let get = |a: At| match a {
            At::Pool(i) => &pool.list[i],
            At::Extra(i) => &tabs.extra[i],
        };
        let current_key = options
            .current_url
            .as_deref()
            .filter(|u| !u.is_empty())
            .map(page_key);
        let known = |url: &str| -> Option<Suggestion> {
            let key = page_key(url);
            let c = pool
                .by_key
                .get(&key)
                .map(|&i| &pool.list[i])
                .or_else(|| tabs.extra.iter().find(|x| x.text.key == key))?;
            let shown_title = c.page_title.as_deref().unwrap_or(&c.title);
            let opened = with_root_slash(url);
            let is_opened = |u: &str| with_root_slash(u) == opened;
            if is_opened(&c.url) {
                return Some(page_suggestion(
                    PageRow {
                        title: shown_title,
                        ..c.row()
                    },
                    None,
                ));
            }
            let visit = source.history.iter().find(|h| is_opened(&h.url));
            let bookmark = source.bookmarks.iter().find(|b| is_opened(&b.url));
            let (title, favicon) = match (visit, bookmark) {
                (Some(v), _) => (v.title.as_str(), v.favicon.as_deref()),
                (None, Some(b)) => (b.title.as_str(), b.favicon.as_deref()),
                (None, None) => (shown_title, c.favicon.as_deref()),
            };
            let row = PageRow {
                url: visit.map_or(opened.as_str(), |v| v.url.as_str()),
                title,
                favicon,
                visits: visit.map_or(0.0, |v| v.visits),
                bookmarked: bookmark.is_some(),
            };
            Some(page_suggestion(row, None))
        };

        let website_first = options.preference == Preference::Website;
        let one_word = tokens.len() == 1;
        let no_inline = options.prevent_inline || (one_word && raw.ends_with(is_js_space));
        let completes = scope.is_none() && one_word && !no_inline;
        let pages: Vec<usize> = self
            .narrowing(pool_id, &q, &tokens)
            .unwrap_or_else(|| (0..pool.list.len()).collect());
        let parsed = parse_query(&q, &tokens);
        let mut matched = Vec::new();
        let rank_limit = limit + 1.0;
        let mut top = TopList::new(if rank_limit.is_nan() {
            usize::MAX
        } else {
            rank_limit as usize
        });
        let mut on_site = scope.map(|_| TopList::new(4));
        let mut inline: Option<Ranked<At>> = None;
        let all = pages
            .iter()
            .map(|&i| At::Pool(i))
            .chain((0..tabs.extra.len()).map(At::Extra));
        for at in all {
            let c = get(at);
            let m = match_score(&c.text, &parsed);
            if m <= 0.0 {
                continue;
            }
            if let At::Pool(i) = at {
                matched.push(i);
                if tabs.renamed.contains(&c.text.key) {
                    continue;
                }
            }
            let tab_id = c
                .tab_id
                .clone()
                .or_else(|| tabs.tab_ids.get(&c.text.key).cloned());
            let derank = if current_key.as_ref() == Some(&c.text.key) {
                80.0
            } else {
                0.0
            };
            let s = m + frecency(c, &tab_id, now) - derank;
            let r = Ranked {
                c: at,
                tab_id,
                m,
                s,
            };
            top.offer(r.clone());
            if let (Some(on_site), Some(scope)) = (on_site.as_mut(), scope) {
                let host = host_of(&c.text.url);
                if host == scope.host || host.ends_with(&format!(".{}", scope.host)) {
                    on_site.offer(r.clone());
                }
            }
            if completes
                && inline.as_ref().is_none_or(|i| r.s > i.s)
                && c.visits > 0.0
                && inline_match(&c.text, &q)
            {
                inline = Some(r);
            }
        }
        self.last = (js_len(&q) >= 2).then(|| Last {
            pool: pool_id,
            q: q.clone(),
            tokens: tokens.iter().map(|t| (*t).to_string()).collect(),
            pages: matched,
        });
        let ranked = top.items;

        let mut out = Output::new(limit);
        let remote_rows = |url: &dyn Fn(&str) -> String, name: &str| -> Vec<Suggestion> {
            dedupe_suggestions(&options.remote, query)
                .into_iter()
                .take(4)
                .map(|t| search_row(t, url(t), name, true))
                .collect()
        };

        if let (Some(scope), Some(on_site)) = (scope, on_site) {
            let scoped = |t: &str| scoped_search_url(scope, t, engine_url);
            out.push(search_row(query, scoped(query), &scope.name, false));
            out.extend(remote_rows(&scoped, &scope.name));
            out.extend(
                on_site
                    .items
                    .iter()
                    .map(|x| page_suggestion(get(x.c).row(), x.tab_id.as_deref())),
            );
            return SuggestionResult {
                items: out.into_items(),
                ..SuggestionResult::default()
            };
        }

        let search = search_row(query, search_url(engine_url, query), engine_name, false);
        let typed_url = fixup_url(query);
        let guessed_url = if typed_url.is_some() {
            None
        } else {
            spaced_address_guess(query)
        };
        let calc = calculate(query);
        let creates = match_quick_create(raw.trim_start_matches(is_js_space), 3);
        let action_rows = if js_len(query) >= 4 { 4 } else { 2 };
        let action_matches: Vec<_> = match_actions(query, &options.actions, action_rows)
            .into_iter()
            .filter(|m| m.exact || m.score >= 50)
            .collect();

        let best = ranked.first();
        let target = inline.as_ref().map(|r| {
            let c = get(r.c);
            let host = host_part(&c.text.shown);
            match origin(&c.text) {
                Some(o) if host.to_lowercase().starts_with(&q) => {
                    (host.to_string(), format!("{o}/"), c.favicon.clone())
                }
                _ => (c.text.shown.clone(), c.text.url.clone(), c.favicon.clone()),
            }
        });
        let completion = target.as_ref().map_or_else(String::new, |(text, ..)| {
            let units: Vec<u16> = text.encode_utf16().skip(js_len(bare)).collect();
            String::from_utf16_lossy(&units)
        });

        let two_words = {
            let rest = query.trim_start_matches(|c: char| !is_js_space(c));
            let after = rest.trim_start_matches(is_js_space);
            rest.len() < query.len() && after.len() < rest.len() && !after.is_empty()
        };
        let first = if !creates.is_empty() && two_words {
            let c = creates[0];
            create_row(c.id, c.title, c.url, c.site)
        } else if action_matches.first().is_some_and(|m| m.exact) {
            action_row(action_matches[0].action)
        } else if let Some((_, url, favicon)) = target.as_ref().filter(|_| !completion.is_empty()) {
            known(url).unwrap_or_else(|| page(url, "", favicon.as_deref()))
        } else if let Some(url) = &typed_url {
            known(url).unwrap_or_else(|| page(url, "", None))
        } else if let Some(best) = best.filter(|b| {
            let shown = &get(b.c).text.shown_lower;
            guessed_url.is_none()
                && website_first
                && b.m >= 40.0
                && (!no_inline || q == *shown || q == host_part(shown))
        }) {
            page_suggestion(get(best.c).row(), best.tab_id.as_deref())
        } else {
            search.clone()
        };

        let guess_row = guessed_url.as_deref().map(|g| {
            let saved = ranked.iter().find(|x| {
                !x.tab_id.as_deref().is_some_and(|t| !t.is_empty())
                    && same_address(&get(x.c).url, g)
            });
            let tab = source.tabs.iter().find(|t| {
                options.current_tab_id.as_deref() != Some(t.id.as_str()) && same_address(&t.url, g)
            });
            match tab {
                Some(t) => page_suggestion(
                    PageRow {
                        url: &t.url,
                        title: custom(t).unwrap_or(&t.title),
                        favicon: t.favicon.as_deref(),
                        visits: 0.0,
                        bookmarked: false,
                    },
                    Some(&t.id),
                ),
                None => known(g).unwrap_or_else(|| match saved {
                    Some(x) => page_suggestion(
                        PageRow {
                            url: g,
                            ..get(x.c).row()
                        },
                        None,
                    ),
                    None => page(g, "", None),
                }),
            }
        });
        out.push(first);
        out.push(search);
        if let Some(row) = guess_row {
            out.push(row);
        }
        if let Some(calc) = calc {
            let url = search_url(engine_url, &calc.expression);
            out.push(Suggestion::Calc {
                expression: calc.expression,
                value: calc.display,
                url,
            });
        }
        out.extend(action_matches.iter().map(|m| action_row(m.action)));
        out.extend(
            creates
                .iter()
                .map(|c| create_row(c.id, c.title, c.url, c.site)),
        );
        let page_rows: Vec<Suggestion> = ranked
            .iter()
            .map(|x| page_suggestion(get(x.c).row(), x.tab_id.as_deref()))
            .map(|mut row| {
                if guessed_url.is_some()
                    && let Suggestion::Page {
                        url,
                        tab_id: Some(id),
                        ..
                    } = &mut row
                    && let Some(t) = source.tabs.iter().find(|t| t.id == *id)
                {
                    *url = t.url.clone();
                }
                row
            })
            .filter(|row| match (guessed_url.as_deref(), row) {
                (Some(g), Suggestion::Page { url, .. }) => !same_address(url, g),
                _ => true,
            })
            .collect();
        let split = page_rows.len().min(3);
        out.extend(page_rows[..split].iter().cloned());
        out.extend(remote_rows(
            &|t: &str| search_url(engine_url, t),
            engine_name,
        ));
        out.extend(page_rows[split..].iter().cloned());
        SuggestionResult {
            items: out.into_items(),
            completion,
            via: None,
        }
    }

    /// `narrowing`: the pages the last query matched, when `q` extends it.
    fn narrowing(&self, pool: u64, q: &str, tokens: &[&str]) -> Option<Vec<usize>> {
        let last = self
            .last
            .as_ref()
            .filter(|l| l.pool == pool && q.starts_with(&l.q))?;
        let i = last.tokens.len().checked_sub(1)?;
        let word = &last.tokens[i];
        (i == 0 || tokens.get(i) == Some(&word.as_str()) || js_len(word) >= 3)
            .then(|| last.pages.clone())
    }

    fn bang_suggestions(
        &mut self,
        found: BangMatch,
        source: &SuggestionSource,
        options: &SuggestOptions,
        pool_id: u64,
    ) -> SuggestionResult {
        let mut out = Output::new(options.limit);
        if found.query.is_empty() {
            out.push(page(&found.bang.home, &found.bang.name, None));
            return SuggestionResult {
                items: out.into_items(),
                ..SuggestionResult::default()
            };
        }
        out.push(Suggestion::Search {
            query: found.query.clone(),
            url: found.url.clone(),
            engine: found.bang.name.clone(),
            suggested: false,
            bang: true,
        });
        let scope = SearchScope {
            kind: ScopeKind::Site,
            name: found.bang.name.clone(),
            host: host_of(&found.bang.home),
            url: found.bang.template.clone(),
            engine_id: None,
            provider_id: None,
        };
        let inner = SuggestOptions {
            scope: Some(scope),
            remote: Vec::new(),
            bangs: None,
            ..options.clone()
        };
        let rows = self.build(&found.query, source, &inner, pool_id).items;
        out.extend(rows.into_iter().skip(1));
        SuggestionResult {
            items: out.into_items(),
            ..SuggestionResult::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::omnibox::candidate_pool::Pool;
    use crate::omnibox::suggest::{Narrowing, build_suggestions};
    use crate::omnibox::suggestion::{BookmarkRow, CommandAction, HistoryRow, SuggestEngine};
    use crate::omnibox::suggestion_rows::suggestion_key;
    use crate::search::bangs::bang_table;

    const NOW: f64 = 1_791_244_800_000.0; // Date.UTC(2026, 9, 6)

    /// The LCG of `suggest.property.test.ts`.
    struct Rng(u32);
    impl Rng {
        fn next(&mut self) -> f64 {
            self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            f64::from(self.0) / 4_294_967_296.0
        }
        fn int(&mut self, n: usize) -> usize {
            (self.next() * n as f64) as usize
        }
        fn pick<'a, T>(&mut self, list: &'a [T]) -> &'a T {
            &list[self.int(list.len())]
        }
    }

    const HOSTS: [&str; 12] = [
        "github.com",
        "www.github.com",
        "news.ycombinator.com",
        "notion.so",
        "x.com",
        "en.wikipedia.org",
        "react.dev",
        "example.org",
        "a.b.co.uk",
        "localhost:3000",
        "xn--nxasmq6b.com",
        "Docs.Google.com",
    ];
    const WORDS: [&str; 25] = [
        "react",
        "native",
        "banana",
        "nano",
        "layout",
        "github",
        "news",
        "notion",
        "an",
        "na",
        "net",
        "netflix",
        "pull",
        "issues",
        "perf",
        "Hermes",
        "Éclair",
        "naïve",
        "c++",
        "foo-bar",
        "a1",
        "9lives",
        "ok",
        "re\u{a0}act",
        "tab\tbed",
    ];
    const TAILS: [&str; 8] = [
        "",
        "/",
        "?q=1",
        "#top",
        "/index.html",
        "?utm=a&b=c#x",
        "/react native",
        "/new tab/page",
    ];

    fn url(r: &mut Rng) -> String {
        let proto = if r.int(12) > 0 { "https" } else { "http" };
        let path: Vec<String> = (0..r.int(4))
            .map(|_| r.pick(&WORDS).to_lowercase())
            .collect();
        let host = r.pick(&HOSTS);
        let tail = r.pick(&TAILS);
        format!("{proto}://{host}/{}{tail}", path.join("/"))
    }

    fn title(r: &mut Rng) -> String {
        if r.int(10) == 0 {
            return String::new();
        }
        let words: Vec<&str> = (0..1 + r.int(6)).map(|_| *r.pick(&WORDS)).collect();
        words.join(if r.int(8) > 0 { " " } else { "-" })
    }

    fn profile(r: &mut Rng, size: usize) -> SuggestionSource {
        let ties = r.int(2) == 0;
        let history: Vec<HistoryRow> = (0..size)
            .map(|_| HistoryRow {
                url: url(r),
                title: title(r),
                favicon: (r.int(3) > 0).then(|| format!("https://x/{}.ico", r.int(9))),
                visits: if ties {
                    r.int(3) as f64
                } else if r.int(5) > 0 {
                    (1 + r.int(40)) as f64
                } else {
                    0.0
                },
                last_visit: if ties {
                    NOW - r.int(3) as f64 * DAY
                } else if r.int(6) > 0 {
                    NOW - r.int(120 * 86_400_000) as f64
                } else {
                    0.0
                },
            })
            .collect();
        let bookmarks = (0..size / 4)
            .map(|_| {
                let h = (r.int(2) > 0 && !history.is_empty()).then(|| r.pick(&history).clone());
                match h {
                    Some(h) => BookmarkRow {
                        url: h.url,
                        title: h.title,
                        favicon: None,
                    },
                    None => BookmarkRow {
                        url: url(r),
                        title: title(r),
                        favicon: None,
                    },
                }
            })
            .collect();
        let tabs = (0..size / 6)
            .map(|i| {
                let h = (r.int(2) > 0 && !history.is_empty()).then(|| r.pick(&history).clone());
                let tab_url = if r.int(15) > 0 {
                    match &h {
                        Some(h) if r.int(4) > 0 => h.url.clone(),
                        Some(h) => format!("{}#{}", h.url, r.pick(&WORDS)),
                        None => url(r),
                    }
                } else {
                    String::new()
                };
                let tab_title = match &h {
                    Some(h) if r.int(3) > 0 => h.title.clone(),
                    _ => title(r),
                };
                SourceTab {
                    id: format!("t{i}"),
                    url: tab_url,
                    title: tab_title,
                    custom_title: (r.int(3) == 0).then(|| title(r)),
                    favicon: None,
                }
            })
            .collect();
        SuggestionSource {
            tabs,
            history,
            bookmarks,
        }
    }

    /// Keystrokes toward a target, then a few edits (`sequence` of the TS test).
    fn sequence(r: &mut Rng, p: &SuggestionSource) -> Vec<String> {
        let (row_url, row_title) = if p.history.is_empty() {
            ("https://github.com/".to_string(), "GitHub".to_string())
        } else {
            let h = r.pick(&p.history);
            (h.url.clone(), h.title.clone())
        };
        let shown = row_url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .to_string();
        let named: Vec<&SourceTab> = p.tabs.iter().filter(|t| t.custom_title.is_some()).collect();
        let renamed = if named.is_empty() {
            r.pick(&WORDS).to_string()
        } else {
            r.pick(&named).custom_title.clone().unwrap_or_default()
        };
        let open: Vec<&SourceTab> = p.tabs.iter().filter(|t| !t.url.is_empty()).collect();
        let tab_url = if open.is_empty() {
            r.pick(&WORDS).to_string()
        } else {
            r.pick(&open).url.clone()
        };
        let first3: Vec<&str> = row_title.split(is_js_space).take(3).collect();
        let targets = [
            tab_url,
            shown.clone(),
            shown.trim_start_matches("www.").to_string(),
            row_title.clone(),
            first3.join(" "),
            r.pick(&HOSTS).to_string(),
            format!("{} {}", r.pick(&WORDS), r.pick(&WORDS)),
            r.pick(&WORDS).to_string(),
            format!("{}/{}", r.pick(&HOSTS), r.pick(&WORDS)),
            renamed.clone(),
            renamed,
        ];
        let mut target = r.pick(&targets).clone();
        if r.int(5) == 0 {
            target = target.to_uppercase();
        }
        if r.int(8) == 0 {
            target = format!(" {target} ");
        }
        if r.int(12) == 0 {
            target = format!("?{target}");
        }
        if r.int(12) == 0 {
            target = format!("{} {target}", r.pick(&["yt", "gh", "g"]));
        }
        if r.int(8) == 0 {
            target = if r.int(2) > 0 {
                format!("{} {target}", r.pick(&["!gh", "!W", "!nope", "!"]))
            } else {
                format!("{target} {}", r.pick(&["!w", "!yt", "!", "!gh"]))
            };
        }
        let chars: Vec<char> = target.chars().collect();
        let up_to = chars.len().min(4 + r.int(24));
        let mut out: Vec<String> = (1..=up_to).map(|i| chars[..i].iter().collect()).collect();
        for _ in 0..r.int(4) {
            let Some(last) = out.last().cloned() else {
                break;
            };
            let next = match r.int(4) {
                0 => {
                    let n = last.chars().count().saturating_sub(1 + r.int(3));
                    last.chars().take(n).collect()
                }
                1 => {
                    let w: String = r.pick(&WORDS).chars().take(1 + r.int(3)).collect();
                    last + &w
                }
                2 => r.pick(&WORDS).chars().take(1 + r.int(4)).collect(),
                _ => last,
            };
            out.push(next);
        }
        out
    }

    fn actions() -> Vec<CommandAction> {
        let a =
            |id: &str, title: &str, keywords: &[&str], icon: Option<&str>, hint: Option<&str>| {
                CommandAction {
                    id: id.into(),
                    title: title.into(),
                    keywords: keywords.iter().map(|k| (*k).into()).collect(),
                    icon: icon.map(Into::into),
                    hint: hint.map(Into::into),
                }
            };
        vec![
            a("new-tab", "New Tab", &["create"], None, None),
            a("settings", "Open Settings", &[], None, None),
            a("net-tools", "Network Tools", &[], Some("wifi"), Some("⌘N")),
            a("pin", "Pin Tab", &["keep"], None, None),
        ]
    }

    /// Runs one profile's keystrokes through both, with the source edited between keystrokes;
    /// a new history or bookmark list is a new pool, as a new array is in the original.
    fn compare_run(seed: u32, size: usize) -> usize {
        let table = bang_table(
            "gh git\tGitHub\tgithub.com/search?q={s}\nw wiki\tWikipedia\twikipedia.org/w/index.php?search={s}\nyt\tYouTube\twww.youtube.com/results?search_query={s}".into(),
        );
        let mut r = Rng(seed);
        let p = profile(&mut r, size);
        let scope = (r.int(6) == 0).then(|| SearchScope {
            kind: ScopeKind::Site,
            name: "GitHub".into(),
            host: "github.com".into(),
            url: "https://github.com/search?q=%s".into(),
            engine_id: None,
            provider_id: None,
        });
        let engine = &BUILT_IN_ENGINES[0];
        let base = SuggestOptions {
            now: Some(NOW),
            limit: *r.pick(&[8.0, 8.0, 8.0, 1.0, 3.0, 12.0]),
            engine: Some(SuggestEngine {
                name: engine.name.clone(),
                url: engine.url.clone(),
            }),
            preference: if r.int(4) > 0 {
                Preference::Website
            } else {
                Preference::Search
            },
            actions: actions(),
            current_tab_id: (r.int(3) == 0).then(|| format!("t{}", r.int(p.tabs.len().max(1)))),
            current_url: if r.int(3) > 0 {
                None
            } else {
                p.history
                    .get(r.int(p.history.len().max(1)))
                    .map(|h| h.url.clone())
            },
            scope,
            prevent_inline: r.int(6) == 0,
            bangs: (r.int(4) > 0).then_some(&table),
            ..SuggestOptions::default()
        };
        let other = profile(&mut Rng(seed + 100_000), size);
        let mut source = p.clone();
        let mut on_p = true;
        let mut queries = sequence(&mut r, &p);
        queries.extend(sequence(&mut r, &p));
        let queries: Vec<String> = queries
            .into_iter()
            .flat_map(|q| {
                if r.int(5) == 0 {
                    vec![q.clone(), q]
                } else {
                    vec![q]
                }
            })
            .collect();
        let mut pool = Pool::new(&source.history, &source.bookmarks).expect("pool");
        let mut narrowing = Narrowing::default();
        let mut reference = Reference::default();
        for (i, q) in queries.iter().enumerate() {
            let mut new_pool = false;
            if r.int(7) == 0 && !p.history.is_empty() {
                let mut h = r.pick(&p.history).clone();
                source.history.retain(|x| *x != h);
                h.visits += 1.0;
                h.last_visit = NOW;
                source.history.insert(0, h);
                new_pool = true;
            }
            if r.int(11) == 0 {
                new_pool = true;
            }
            if r.int(9) == 0 {
                let n = r.int(3).min(source.tabs.len());
                source.tabs.drain(..n);
            }
            if r.int(10) == 0 && !source.history.is_empty() {
                let at = r.int(source.history.len());
                source.history.remove(at);
                new_pool = true;
            }
            if r.int(40) == 0 {
                source.history.clear();
                new_pool = true;
            }
            if r.int(10) == 0 && !source.tabs.is_empty() {
                let at = r.int(source.tabs.len());
                let t = &mut source.tabs[at];
                t.title = format!("{} {}", r.pick(&WORDS), t.title);
            }
            if r.int(12) == 0 {
                source.tabs.push(SourceTab {
                    id: format!("new{i}"),
                    url: format!("https://{}/{}", r.pick(&HOSTS), r.pick(&WORDS)),
                    title: r.pick(&WORDS).to_string(),
                    custom_title: None,
                    favicon: None,
                });
            }
            if r.int(8) == 0 && !source.tabs.is_empty() {
                let at = r.int(source.tabs.len());
                source.tabs[at].custom_title =
                    (r.int(3) > 0).then(|| format!("{} {}", r.pick(&WORDS), r.pick(&WORDS)));
            }
            if r.int(10) == 0 {
                let h = (!source.history.is_empty()).then(|| r.pick(&source.history).clone());
                source.bookmarks.push(match h {
                    Some(h) => BookmarkRow {
                        url: h.url,
                        title: h.title,
                        favicon: None,
                    },
                    None => BookmarkRow {
                        url: format!("https://{}/{}", r.pick(&HOSTS), r.pick(&WORDS)),
                        title: r.pick(&WORDS).to_string(),
                        favicon: None,
                    },
                });
                new_pool = true;
            }
            if r.int(10) == 0 && !source.bookmarks.is_empty() {
                let at = r.int(source.bookmarks.len());
                source.bookmarks.remove(at);
                new_pool = true;
            }
            if r.int(25) == 0 {
                source = if on_p { other.clone() } else { p.clone() };
                on_p = !on_p;
                new_pool = true;
            }
            if new_pool {
                pool = Pool::new(&source.history, &source.bookmarks).expect("pool");
            }
            let options = SuggestOptions {
                remote: if r.int(3) > 0 {
                    vec![]
                } else {
                    vec![
                        format!("{q} one"),
                        "two".into(),
                        q.to_uppercase(),
                        format!("{q} three"),
                        "four".into(),
                        "five".into(),
                    ]
                },
                ..base.clone()
            };
            let want = reference.build(q, &source, &options, pool.id());
            let got =
                build_suggestions(q, &source, &options, &mut pool, &mut narrowing).expect("rows");
            if let Some(via) = &got.via {
                assert!(
                    via.iter()
                        .all(|(key, _)| got.items.iter().any(|x| suggestion_key(x) == *key)),
                    "via names only shown rows"
                );
            }
            assert_eq!(
                (&got.items, &got.completion),
                (&want.items, &want.completion),
                "seed {seed} size {size} step {i} query {q:?}"
            );
        }
        queries.len()
    }

    #[test]
    fn keystroke_sequences_rank_exactly_as_the_reference_does() {
        let mut steps = 0;
        for seed in 1..=150 {
            steps += compare_run(seed, 1 + (seed as usize % 7) * 25);
        }
        steps += compare_run(77, 0) + compare_run(78, 1) + compare_run(9001, 600);
        assert!(steps > 2500, "{steps} steps");
    }

    #[test]
    fn a_tab_is_last_visited_now_even_when_now_is_0() {
        let source = SuggestionSource {
            history: vec![HistoryRow {
                url: "https://history.example/".into(),
                title: "abc".into(),
                favicon: None,
                visits: 2.0,
                last_visit: 0.0,
            }],
            tabs: vec![SourceTab {
                id: "t".into(),
                url: "https://tab.example/".into(),
                title: "abc".into(),
                custom_title: None,
                favicon: None,
            }],
            bookmarks: vec![],
        };
        for now in [0.0, f64::NAN, f64::INFINITY, 1.0] {
            let options = SuggestOptions {
                now: Some(now),
                preference: Preference::Search,
                ..SuggestOptions::default()
            };
            let mut pool = Pool::new(&source.history, &source.bookmarks).expect("pool");
            let got = build_suggestions(
                "abc",
                &source,
                &options,
                &mut pool,
                &mut Narrowing::default(),
            )
            .expect("rows");
            let want = Reference::default().build("abc", &source, &options, pool.id());
            assert_eq!(got.items, want.items, "now {now}");
        }
    }
}
