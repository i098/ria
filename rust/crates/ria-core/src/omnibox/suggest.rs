// Ported from ardia packages/core/src/suggest.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The address bar dropdown: [`build_suggestions`] ranks the pages, tabs, searches and
//! shortcuts for the typed text.
//!
//! The original finds its pool and the pages earlier keystrokes matched in module caches, by the
//! identity of the history and bookmark arrays. Here the caller owns both: a [`Pool`] built from
//! `source.history` and `source.bookmarks` (kept while those rows do not change), and a
//! [`Narrowing`] kept across keystrokes. A fresh pair gives the same rows as the original's
//! first call on new arrays. The re-exports of `suggest.ts` (`displayUrl`,
//! `prepareSuggestions`, `suggestionKey`) are in their own modules.

use std::collections::HashMap;

use super::calculator::calculate;
use super::candidate_pool::{Candidate, Pool, TabsOver, saved_bookmark, saved_visit, tabs_over};
use super::omnibox::{fixup_url, host_of, same_address, search_url_for, spaced_address_guess};
use super::quick_create::match_quick_create;
use super::suggestion::{
    Preference, SuggestOptions, Suggestion, SuggestionResult, SuggestionSource, SuggestionVia,
};
use super::suggestion_rows::{
    Output, PageRow, action_row, create_row, dedupe_suggestions, page_suggestion, suggestion_key,
};
use crate::fuzzy::match_actions;
use crate::search::bangs::{BangMatch, find_bang};
use crate::search::engines::{BUILT_IN_ENGINES, search_url};
use crate::search::extension_rows::{self, extension_suggestions};
use crate::search::js_text::{is_js_space, js_trim};
use crate::search::provider_rows::{
    ProviderSection, ProviderSuggestion, ShownSuggestion, global_provider_rows, provider_url,
    safe_provider_rows,
};
use crate::search::site_search::{ScopeKind, SearchScope, scoped_search_url};
use crate::url::js_len;
use crate::url::page_match::{
    MatchedChain, Ranked, TopList, match_score, match_score_word, parse_query,
};
use crate::url::page_text::{PageText, host_part, page_key, with_root_slash};

const DAY: f64 = 86_400_000.0;
/// The default `limit` of `matchQuickCreate`.
const QUICK_CREATE_LIMIT: usize = 3;

/// The pages earlier queries on a pool matched (`pageMatch.ts`'s module chain), as indexes into
/// [`Pool::list`], keyed by [`Pool::id`].
pub type Narrowing = MatchedChain<u64, usize>;

/// A ranked candidate: a page of the pool or an extra tab of `tabsOver`.
#[derive(Debug, Clone, Copy)]
enum Cand {
    Pool(usize),
    Extra(usize),
}

fn cand<'a>(pool: &'a Pool, extra: &'a [Candidate], c: Cand) -> &'a Candidate {
    match c {
        Cand::Pool(i) => &pool.list[i],
        Cand::Extra(i) => &extra[i],
    }
}

fn cand_mut<'a>(pool: &'a mut Pool, extra: &'a mut [Candidate], c: Cand) -> &'a mut Candidate {
    match c {
        Cand::Pool(i) => &mut pool.list[i],
        Cand::Extra(i) => &mut extra[i],
    }
}

/// A truthy tab id: set and not empty.
fn has_tab(tab_id: &Option<String>) -> bool {
    tab_id.as_deref().is_some_and(|t| !t.is_empty())
}

fn plain_page(url: &str, title: &str, favicon: Option<&str>) -> Suggestion {
    Suggestion::Page {
        url: url.to_string(),
        title: title.to_string(),
        favicon: favicon.map(str::to_string),
        tab_id: None,
        bookmarked: false,
        visited: false,
    }
}

fn search_row(query: &str, url: String, engine: &str, suggested: bool) -> Suggestion {
    Suggestion::Search {
        query: query.to_string(),
        url,
        engine: engine.to_string(),
        suggested,
        bang: false,
    }
}

/// `tag(s, as)`: records why `s` is shown, under its key.
fn tag(
    via: &mut HashMap<String, SuggestionVia>,
    s: Suggestion,
    reason: Option<SuggestionVia>,
) -> Suggestion {
    if let Some(reason) = reason {
        via.insert(suggestion_key(&s), reason);
    }
    s
}

fn empty() -> SuggestionResult {
    SuggestionResult::default()
}

/// `Date.now()`.
fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_millis() as f64)
}

/// `query.replace(/^https?:\/\/(?=\S)/i, "")`.
fn strip_http(query: &str) -> &str {
    ["https://", "http://"]
        .iter()
        .find_map(|scheme| {
            let rest = query
                .get(..scheme.len())
                .filter(|head| head.eq_ignore_ascii_case(scheme))
                .map(|_| &query[scheme.len()..])?;
            rest.starts_with(|c: char| !is_js_space(c)).then_some(rest)
        })
        .unwrap_or(query)
}

/// `/^\S+\s+\S/.test(text)`.
fn two_words(text: &str) -> bool {
    let rest = text.trim_start_matches(|c: char| !is_js_space(c));
    let after = rest.trim_start_matches(is_js_space);
    rest.len() < text.len() && after.len() < rest.len() && !after.is_empty()
}

/// `text.slice(from)` in UTF-16 units.
fn utf16_from(text: &str, from: usize) -> String {
    let units: Vec<u16> = text.encode_utf16().skip(from).collect();
    String::from_utf16_lossy(&units)
}

/// Port of `buildSuggestions` (`suggest.ts:23`). `pool` is `Pool::new(&source.history,
/// &source.bookmarks)`, and `narrowing` the pages the earlier keystrokes on it matched. `Err` is
/// the `TypeError` of `urlForDisplay`.
pub fn build_suggestions(
    raw: &str,
    source: &SuggestionSource,
    options: &SuggestOptions,
    pool: &mut Pool,
    narrowing: &mut Narrowing,
) -> Result<SuggestionResult, &'static str> {
    let limit = options.limit;
    let now = options.now.unwrap_or_else(now_ms);
    let (engine_name, engine_url) = options.engine.as_ref().map_or(
        (
            BUILT_IN_ENGINES[0].name.as_str(),
            BUILT_IN_ENGINES[0].url.as_str(),
        ),
        |e| (e.name.as_str(), e.url.as_str()),
    );
    let scope = options.scope.as_ref();
    let query = js_trim(raw);
    if let Some(scope) = scope.filter(|s| s.kind == ScopeKind::Provider) {
        return Ok(provider_scope_suggestions(
            query,
            scope,
            &options.providers,
            limit,
        ));
    }
    if query.is_empty() {
        return Ok(empty());
    }
    let bare = strip_http(query);
    let q = bare.to_lowercase();
    let tokens: Vec<&str> = q.split(is_js_space).filter(|t| !t.is_empty()).collect();

    if scope.is_none() {
        if let Some(bang) = find_bang(query, options.bangs) {
            return bang_suggestions(bang, source, options, pool, narrowing);
        }
        if let Some(text) = query.strip_prefix('?') {
            let text = js_trim(text);
            if text.is_empty() {
                return Ok(empty());
            }
            let mut out = Output::new(limit);
            out.push(search_row(
                text,
                search_url(engine_url, text),
                engine_name,
                false,
            ));
            out.extend(
                dedupe_suggestions(&options.remote, text)
                    .into_iter()
                    .take(4)
                    .map(|t| search_row(t, search_url(engine_url, t), engine_name, true)),
            );
            return Ok(SuggestionResult {
                items: out.into_items(),
                ..empty()
            });
        }
    }

    let TabsOver {
        tab_ids,
        mut extra,
        renamed,
    } = tabs_over(pool, &source.tabs, options.current_tab_id.as_deref())?;
    let current_key = options
        .current_url
        .as_deref()
        .filter(|u| !u.is_empty())
        .map(page_key);
    let mut via = HashMap::new();

    let website_first = options.preference == Preference::Website;
    let one_word = tokens.len() == 1;
    let no_inline = options.prevent_inline
        || (one_word && raw.len() != query.len() && raw.ends_with(is_js_space));
    let completes = scope.is_none() && one_word && !no_inline;
    let (pages, again, mut scores) = match narrowing.narrowing(&pool.id(), &q, &tokens) {
        Some(from) if from.q == q => (Some(from.pages.clone()), true, from.scores.clone()),
        Some(from) => (Some(from.pages.clone()), false, Vec::new()),
        None => (None, false, Vec::new()),
    };
    let parsed = parse_query(&q, &tokens);
    let mut matched = Vec::new();
    let mut top = TopList::new(limit.saturating_add(1));
    let mut on_site = scope.map(|_| TopList::new(4));
    let mut inline: Option<Ranked<Cand>> = None;
    let pages_len = pages.as_ref().map_or(pool.list.len(), Vec::len);
    for in_pages in [true, false] {
        let len = if in_pages { pages_len } else { extra.len() };
        for at in 0..len {
            let (at_c, c) = if in_pages {
                let i = pages.as_ref().map_or(at, |p| p[at]);
                (Cand::Pool(i), &mut pool.list[i])
            } else {
                (Cand::Extra(at), &mut extra[at])
            };
            let m = if again && in_pages {
                scores[at]
            } else if one_word {
                match_score_word(&c.text, &parsed)
            } else {
                match_score(&mut c.text, &parsed)
            };
            if m <= 0.0 {
                continue;
            }
            if let (Cand::Pool(i), false) = (at_c, again) {
                matched.push(i);
                scores.push(m);
            }
            let key = &c.text.key;
            if in_pages && renamed.contains(key) {
                continue;
            }
            let derank = if current_key.as_ref() == Some(key) {
                80.0
            } else {
                0.0
            };
            let last_visit = if c.fresh { now } else { c.last_visit };
            let mut age = 10.0;
            if last_visit != 0.0 && !last_visit.is_nan() {
                let d = (now - last_visit) / DAY;
                age = d.clamp(0.0, 30.0);
            }
            let base = c.log_visits - age + if c.bookmarked { 20.0 } else { 0.0 };
            if scope.is_none()
                && top.floor >= m + base - derank + 7.0
                && (!completes
                    || inline
                        .as_ref()
                        .is_some_and(|r| r.s >= m + base - derank + 7.0)
                    || c.visits <= 0.0
                    || !c.text.shown_lower.starts_with(&q))
            {
                continue;
            }
            let tab_id = c.tab_id.clone().or_else(|| tab_ids.get(key).cloned());
            let s = m + (base + if has_tab(&tab_id) { 6.0 } else { 0.0 }) - derank;
            let r = Ranked {
                c: at_c,
                tab_id,
                m,
                s,
            };
            if let (Some(on_site), Some(scope)) = (on_site.as_mut(), scope) {
                let host = c.text.host_of_page(host_of);
                if host == scope.host
                    || host
                        .strip_suffix(scope.host.as_str())
                        .is_some_and(|h| h.ends_with('.'))
                {
                    on_site.offer(r.clone());
                }
            }
            if completes
                && inline.as_ref().is_none_or(|i| s > i.s)
                && c.visits > 0.0
                && inline_match(&mut c.text, &q)
            {
                inline = Some(r.clone());
            }
            if top.floor < s || top.items.len() < limit.saturating_add(1) {
                top.offer(r);
            }
        }
    }
    let matched = match pages {
        Some(pages) if again => pages,
        _ => matched,
    };
    let token_list = tokens.iter().map(|t| (*t).to_string()).collect();
    narrowing.remember(pool.id(), &q, token_list, matched, scores);
    let ranked = top.items;

    let mut out = Output::new(limit);
    let remote_rows = |url: &dyn Fn(&str) -> String, name: &str| -> Vec<Suggestion> {
        dedupe_suggestions(&options.remote, query)
            .into_iter()
            .take(4)
            .map(|text| search_row(text, url(text), name, true))
            .collect()
    };

    if let (Some(scope), Some(on_site)) = (scope, on_site) {
        let scoped_url = |text: &str| scoped_search_url(scope, text, engine_url);
        out.push(search_row(query, scoped_url(query), &scope.name, false));
        out.extend(remote_rows(&scoped_url, &scope.name));
        out.extend(on_site.items.iter().map(|x| {
            let c = cand(pool, &extra, x.c);
            page_suggestion(c.page_row(), x.tab_id.as_deref())
        }));
        return Ok(SuggestionResult {
            items: out.into_items(),
            ..empty()
        });
    }

    let search = search_row(query, search_url(engine_url, query), engine_name, false);
    let typed_url = fixup_url(query);
    let guessed_url = match typed_url {
        Some(_) => None,
        None => spaced_address_guess(query),
    };
    let calc = calculate(query);
    let creates = match_quick_create(raw.trim_start_matches(is_js_space), QUICK_CREATE_LIMIT);
    let action_rows = if js_len(query) >= 4 { 4 } else { 2 };
    let action_matches: Vec<_> = match_actions(query, &options.actions, action_rows)
        .into_iter()
        .filter(|m| m.exact || m.score >= 50)
        .collect();
    let extension_matches =
        extension_suggestions(query, &options.extensions, extension_rows::DEFAULT_LIMIT);
    let extension_items: Vec<Suggestion> =
        extension_matches.rows.iter().map(extension_row).collect();

    let target = inline
        .as_ref()
        .map(|r| inline_target(&q, cand_mut(pool, &mut extra, r.c)));
    let completion = target
        .as_ref()
        .map_or_else(String::new, |t| utf16_from(&t.text, js_len(bare)));

    let first = if let Some(c) = creates.first().filter(|_| two_words(query)) {
        create_row(c.id, c.title, c.url, c.site)
    } else if let Some(m) = action_matches.first().filter(|m| m.exact) {
        action_row(m.action)
    } else if let Some(t) = target.as_ref().filter(|_| !completion.is_empty()) {
        let row = known(&t.url, pool, &extra, source, &mut via)
            .unwrap_or_else(|| plain_page(&t.url, "", t.favicon.as_deref()));
        let reason = (!website_first).then_some(SuggestionVia::SearchFirstInline);
        tag(&mut via, row, reason)
    } else if let Some(url) = &typed_url {
        known(url, pool, &extra, source, &mut via).unwrap_or_else(|| plain_page(url, "", None))
    } else if let Some(row) = extension_items.first().filter(|_| extension_matches.exact) {
        row.clone()
    } else if let Some(best) = ranked.first().filter(|best| {
        guessed_url.is_none()
            && website_first
            && best.m >= 40.0
            && (!no_inline || typed_is_page(&mut cand_mut(pool, &mut extra, best.c).text, &q))
    }) {
        let c = cand(pool, &extra, best.c);
        let row = page_suggestion(c.page_row(), best.tab_id.as_deref());
        tag(
            &mut via,
            row,
            c.renamed.then_some(SuggestionVia::RenamedTab),
        )
    } else {
        search.clone()
    };

    let guess_row = guessed_url.as_deref().map(|guess| {
        let tab = source.tabs.iter().find(|t| {
            options.current_tab_id.as_deref() != Some(t.id.as_str()) && same_address(&t.url, guess)
        });
        if let Some(t) = tab {
            let title = t
                .custom_title
                .as_deref()
                .filter(|c| !c.is_empty())
                .unwrap_or(&t.title);
            let row = PageRow {
                url: &t.url,
                title,
                favicon: t.favicon.as_deref(),
                visits: 0.0,
                bookmarked: false,
            };
            return page_suggestion(row, Some(&t.id));
        }
        let saved = ranked
            .iter()
            .find(|x| !has_tab(&x.tab_id) && same_address(&cand(pool, &extra, x.c).url, guess));
        known(guess, pool, &extra, source, &mut via).unwrap_or_else(|| match saved {
            Some(x) => {
                let row = cand(pool, &extra, x.c).page_row();
                page_suggestion(PageRow { url: guess, ..row }, None)
            }
            None => plain_page(guess, "", None),
        })
    });
    out.push(first);
    out.push(search);
    let guess_shown = guess_row.as_ref().is_some_and(|row| out.push(row.clone()));
    if let Some(calc) = calc {
        let url = search_url(engine_url, &calc.expression);
        out.push(Suggestion::Calc {
            expression: calc.expression,
            value: calc.display,
            url,
        });
    }
    out.extend(action_matches.iter().map(|m| action_row(m.action)));
    out.extend(extension_items);
    out.extend(
        creates
            .iter()
            .map(|c| create_row(c.id, c.title, c.url, c.site)),
    );
    let mut page_rows = Vec::with_capacity(ranked.len());
    for x in &ranked {
        let c = cand(pool, &extra, x.c);
        let row = page_suggestion(c.page_row(), x.tab_id.as_deref());
        let mut row = tag(
            &mut via,
            row,
            c.renamed.then_some(SuggestionVia::RenamedTab),
        );
        if let (Some(guess), Suggestion::Page { url, tab_id, .. }) =
            (guessed_url.as_deref(), &mut row)
        {
            if let Some(t) = tab_id
                .as_deref()
                .and_then(|id| source.tabs.iter().find(|t| t.id == id))
            {
                url.clone_from(&t.url);
            }
            if same_address(url, guess) {
                continue;
            }
        }
        page_rows.push(row);
    }
    let later_pages = page_rows.split_off(page_rows.len().min(3));
    out.extend(page_rows);
    let site_rows: Vec<Suggestion> = options
        .remote_sites
        .iter()
        .take(2)
        .filter(|site| {
            guessed_url
                .as_deref()
                .is_none_or(|g| !same_address(&site.url, g))
        })
        .map(
            |site| match known(&site.url, pool, &extra, source, &mut via) {
                Some(row) => row,
                None => tag(
                    &mut via,
                    plain_page(&site.url, &site.title, None),
                    Some(SuggestionVia::EngineSite),
                ),
            },
        )
        .collect();
    out.extend(site_rows);
    let provider_at = out.items().len();
    out.extend(remote_rows(
        &|text: &str| search_url(engine_url, text),
        engine_name,
    ));
    out.extend(later_pages);
    let provider_rows = if options.providers.is_empty() {
        Vec::new()
    } else {
        let shown: Vec<ShownSuggestion> = out
            .items()
            .iter()
            .map(|s| ShownSuggestion {
                kind: s.kind(),
                url: s.url(),
            })
            .collect();
        global_provider_rows(&options.providers, &shown)
    };
    let mut items = out.into_items();
    if !provider_rows.is_empty() {
        items.splice(
            provider_at..provider_at,
            provider_rows.into_iter().map(Suggestion::Provider),
        );
    }
    if let (Some(row), true) = (&guess_row, guess_shown) {
        via.insert(suggestion_key(row), SuggestionVia::SpacedAddress);
    }
    let via = via_of(&items, &via);
    Ok(SuggestionResult {
        items,
        completion,
        via,
    })
}

/// `known(url)` (`suggest.ts:49`): the saved page of `url` as a row, `None` when nothing is
/// saved under its key. A row for another URL of the page is tagged `literal`.
fn known(
    url: &str,
    pool: &Pool,
    extra: &[Candidate],
    source: &SuggestionSource,
    via: &mut HashMap<String, SuggestionVia>,
) -> Option<Suggestion> {
    let key = page_key(url);
    let c = pool
        .by_key(&key)
        .or_else(|| extra.iter().find(|x| x.text.key == key))?;
    let shown_title = c.page_title.as_deref().unwrap_or(&c.title);
    let opened = with_root_slash(url);
    if with_root_slash(&c.url) == opened {
        let row = PageRow {
            title: shown_title,
            ..c.page_row()
        };
        return Some(page_suggestion(row, None));
    }
    let visit = saved_visit(&source.history, &opened);
    let bookmark = saved_bookmark(&source.bookmarks, &opened);
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
    Some(tag(
        via,
        page_suggestion(row, None),
        Some(SuggestionVia::Literal),
    ))
}

fn extension_row(row: &extension_rows::Suggestion) -> Suggestion {
    let extension_rows::Suggestion::Extension {
        id,
        title,
        icon,
        open,
    } = row;
    Suggestion::Extension {
        id: id.clone(),
        title: title.clone(),
        icon: icon.clone(),
        open: *open,
    }
}

/// Port of `providerScopeSuggestions` (`suggest.ts:156`): the typed text as the provider's own
/// search, then the provider's rows.
fn provider_scope_suggestions(
    query: &str,
    scope: &SearchScope,
    sections: &[ProviderSection],
    limit: usize,
) -> SuggestionResult {
    let id = scope.provider_id.as_deref().unwrap_or("");
    let mut items: Vec<ProviderSuggestion> = Vec::new();
    if !scope.url.is_empty() && !query.is_empty() {
        let url = search_url_for(&scope.url, query);
        if let Some(url) = provider_url(Some(&url)) {
            items.push(ProviderSuggestion {
                provider_id: id.to_string(),
                provider: scope.name.clone(),
                title: query.to_string(),
                url: url.to_string(),
                icon: None,
                subtitle: None,
            });
        }
    }
    if let Some(section) = sections.iter().find(|x| x.id == id) {
        let seen: Vec<String> = items.iter().map(|x| x.url.clone()).collect();
        for row in safe_provider_rows(id, &scope.name, &section.rows, limit as f64) {
            if !seen.contains(&row.url) && items.len() < limit {
                items.push(row);
            }
        }
    }
    SuggestionResult {
        items: items.into_iter().map(Suggestion::Provider).collect(),
        ..empty()
    }
}

/// Port of `viaOf` (`suggest.ts:167`): the reasons of the shown rows, in their order.
fn via_of(
    items: &[Suggestion],
    via: &HashMap<String, SuggestionVia>,
) -> Option<Vec<(String, SuggestionVia)>> {
    let shown: Vec<_> = items
        .iter()
        .filter_map(|item| {
            let key = suggestion_key(item);
            via.get(&key).map(|&v| (key, v))
        })
        .collect();
    (!shown.is_empty()).then_some(shown)
}

/// Port of `typedIsPage` (`suggest.ts:176`).
fn typed_is_page(t: &mut PageText, q: &str) -> bool {
    q == t.shown_lower || q == t.host_lower_of_page()
}

/// Port of `inlineMatch` (`suggest.ts:180`).
fn inline_match(t: &mut PageText, q: &str) -> bool {
    if t.host_lower_of_page().starts_with(q) && t.origin_of_page().is_some() {
        return true;
    }
    q.contains('/') && !t.shown.contains(['?', '#']) && t.shown_lower.starts_with(q)
}

/// What inline completion completes to: the text after the typed one, and the page it opens.
struct InlineTarget {
    text: String,
    url: String,
    favicon: Option<String>,
}

/// Port of `inlineTarget` (`suggest.ts:185`): the host when the query is a prefix of it, else
/// the whole shown address.
fn inline_target(q: &str, c: &mut Candidate) -> InlineTarget {
    let favicon = c.favicon.clone();
    let host = host_part(&c.text.shown).to_string();
    if let Some(origin) = c.text.origin_of_page()
        && host.to_lowercase().starts_with(q)
    {
        return InlineTarget {
            text: host,
            url: format!("{origin}/"),
            favicon,
        };
    }
    InlineTarget {
        text: c.text.shown.clone(),
        url: c.text.url.clone(),
        favicon,
    }
}

/// Port of `bangSuggestions` (`suggest.ts:192`): the bang search, then the site's pages.
fn bang_suggestions(
    found: BangMatch,
    source: &SuggestionSource,
    options: &SuggestOptions,
    pool: &mut Pool,
    narrowing: &mut Narrowing,
) -> Result<SuggestionResult, &'static str> {
    let BangMatch { bang, query, url } = found;
    let mut out = Output::new(options.limit);
    if query.is_empty() {
        out.push(plain_page(&bang.home, &bang.name, None));
        return Ok(SuggestionResult {
            items: out.into_items(),
            ..empty()
        });
    }
    out.push(Suggestion::Search {
        query: query.clone(),
        url,
        engine: bang.name.clone(),
        suggested: false,
        bang: true,
    });
    let scope = SearchScope {
        kind: ScopeKind::Site,
        name: bang.name.clone(),
        host: host_of(&bang.home),
        url: bang.template.clone(),
        engine_id: None,
        provider_id: None,
    };
    let inner = SuggestOptions {
        scope: Some(scope),
        remote: Vec::new(),
        bangs: None,
        ..options.clone()
    };
    let rows = build_suggestions(&query, source, &inner, pool, narrowing)?.items;
    out.extend(rows.into_iter().skip(1));
    Ok(SuggestionResult {
        items: out.into_items(),
        ..empty()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::omnibox::suggestion::HistoryRow;

    fn run(raw: &str, source: &SuggestionSource, options: &SuggestOptions) -> SuggestionResult {
        let mut pool = Pool::new(&source.history, &source.bookmarks).expect("pool");
        build_suggestions(raw, source, options, &mut pool, &mut Narrowing::default()).expect("rows")
    }

    fn visit(url: &str, title: &str, visits: f64) -> HistoryRow {
        HistoryRow {
            url: url.into(),
            title: title.into(),
            favicon: None,
            visits,
            last_visit: 1.0,
        }
    }

    #[test]
    fn helpers_follow_the_js_regexes() {
        assert_eq!(strip_http("HTTPS://a.b"), "a.b");
        assert_eq!(strip_http("https:// a"), "https:// a");
        assert_eq!(strip_http("http://"), "http://");
        assert!(two_words("new doc") && !two_words("new") && !two_words("new "));
        assert_eq!(utf16_from("a😀b", 1), "😀b");
    }

    #[test]
    fn a_visited_host_completes_inline() {
        let source = SuggestionSource {
            history: vec![visit("https://github.com/x", "GitHub", 5.0)],
            ..SuggestionSource::default()
        };
        let options = SuggestOptions {
            now: Some(2.0),
            ..SuggestOptions::default()
        };
        let got = run("git", &source, &options);
        assert_eq!(got.completion, "hub.com");
        assert_eq!(got.items[0].url(), Some("https://github.com/"));
        // No saved row has the host's own URL, so `known` tags nothing.
        assert_eq!(got.via, None);
    }

    #[test]
    fn a_question_mark_only_searches() {
        let options = SuggestOptions {
            remote: vec!["b".into(), "B".into(), "a".into()],
            ..SuggestOptions::default()
        };
        let got = run("? a ", &SuggestionSource::default(), &options);
        let queries: Vec<_> = got
            .items
            .iter()
            .map(|s| match s {
                Suggestion::Search { query, .. } => query.as_str(),
                _ => "",
            })
            .collect();
        assert_eq!(queries, ["a", "b"]);
    }
}
