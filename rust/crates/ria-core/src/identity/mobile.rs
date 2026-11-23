// Ported from ardia packages/core/src/mobile.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The omnibox of the phone app: one session holds the suggestion source, the search engines and
//! the bang table that the module-level variables of `mobile.ts` hold. `HostURL`, the `URL`
//! polyfill for engines that lack one, has no port: Rust has no such engine.

use crate::omnibox::candidate_pool::Pool;
use crate::omnibox::omnibox::{
    breadcrumb, fixup_url, search_url_for, split_scheme, url_for_display,
};
use crate::omnibox::suggest::{Narrowing, build_suggestions_lazy};
use crate::omnibox::suggest_fetcher::suggest_reply;
use crate::omnibox::suggestion::{
    SuggestEngine, SuggestOptions, Suggestion, SuggestionResult, SuggestionSource,
};
use crate::search::bang_list::BANG_LISTS;
use crate::search::bangs::{
    BangTable, bang_document, bang_table, find_bang, join_bang_tables, parse_json,
    read_bang_document, slim_bangs,
};
use crate::search::engines::{
    BUILT_IN_ENGINES, CustomSearchEngine, SearchEngine, all_engines, engine_by_id,
    parse_suggestions, suggest_request_url, valid_search_template,
};
use crate::search::js_text::js_trim;
use crate::search::site_search::{ScopeKind, ScopeSources, SearchScope, find_scope};
use crate::url::clean_url::clean_url;

/// `MobileSearch` (`mobile.ts:28`): `None` is `null` or absent.
#[derive(Debug, Clone, Default)]
pub struct MobileSearch {
    pub engine_id: Option<String>,
    pub custom_url: Option<String>,
    pub custom_engines: Vec<CustomSearchEngine>,
}

/// The engine `mobileSetSearch` returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineInfo {
    pub id: String,
    pub name: String,
    pub url: String,
    pub suggests: bool,
}

/// One row of `mobileEngines`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineRow {
    pub id: String,
    pub name: String,
    pub url: String,
    pub custom: bool,
}

/// `MobileSuggestOptions` (`mobile.ts:29`): `None` is `null` or absent.
#[derive(Debug, Clone, Default)]
pub struct MobileSuggestOptions {
    pub current_url: Option<String>,
    pub current_tab_id: Option<String>,
    pub remote: Vec<String>,
    pub search_preferred: bool,
    pub limit: Option<f64>,
    pub prevent_inline: bool,
    pub scope: Option<SearchScope>,
    pub now: Option<f64>,
}

/// `MobileResolution["kind"]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionKind {
    Url,
    Search,
    Bang,
}

impl ResolutionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Url => "url",
            Self::Search => "search",
            Self::Bang => "bang",
        }
    }
}

/// `MobileResolution` (`mobile.ts:40`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub kind: ResolutionKind,
    pub url: String,
    pub query: String,
    pub engine: String,
}

/// What `mobileDisplay` returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Display {
    pub display: String,
    pub host: String,
    pub trail: Vec<String>,
}

/// The state of `mobile.ts`. `Err` is the message of a JS `TypeError`.
pub struct Mobile {
    source: SuggestionSource,
    /// The pool and the narrowing chain that `poolFor` and `narrowing` keep for `source`.
    pool: Option<(Pool, Narrowing)>,
    engines: Vec<SearchEngine>,
    engine: SearchEngine,
    bangs: Option<BangTable>,
}

impl Default for Mobile {
    fn default() -> Self {
        Self {
            source: SuggestionSource::default(),
            pool: None,
            engines: all_engines(&[], &[]),
            engine: BUILT_IN_ENGINES[0].clone(),
            bangs: None,
        }
    }
}

fn pool_of<'a>(
    source: &SuggestionSource,
    cached: &'a mut Option<(Pool, Narrowing)>,
) -> Result<&'a mut (Pool, Narrowing), &'static str> {
    if cached.is_none() {
        *cached = Some((
            Pool::new(&source.history, &source.bookmarks)?,
            Narrowing::default(),
        ));
    }
    Ok(cached.as_mut().expect("the pool was just built"))
}

impl Mobile {
    fn bang_table_once(&mut self) -> &BangTable {
        self.bangs.get_or_insert_with(|| {
            join_bang_tables(BANG_LISTS.iter().map(|l| bang_table((*l).into())).collect())
        })
    }

    /// `mobileLoadBangDocument` (`mobile.ts:55`): the ETag of the document, `""` when it has none.
    pub fn load_bang_document(&mut self, doc: &str) -> Option<String> {
        let read = read_bang_document(Some(doc))?;
        self.bangs = Some(read.table);
        Some(read.etag.unwrap_or_default())
    }

    /// `mobileBangDocument` (`mobile.ts:62`).
    pub fn make_bang_document(&mut self, json: &str, etag: Option<&str>) -> Option<String> {
        let doc = bang_document(etag, &slim_bangs(json)?);
        self.load_bang_document(&doc).map(|_| doc)
    }

    /// `mobileSetSources` (`mobile.ts:69`). `Err` is the `TypeError` of `urlForDisplay`; the
    /// source is kept then, as the original assigns it before it builds the pool.
    pub fn set_sources(&mut self, source: SuggestionSource) -> Result<bool, &'static str> {
        self.source = source;
        self.pool = None;
        pool_of(&self.source, &mut self.pool)?;
        Ok(true)
    }

    /// `mobileSetSearch` (`mobile.ts:74`).
    pub fn set_search(&mut self, search: &MobileSearch) -> EngineInfo {
        let mut list = all_engines(&search.custom_engines, &[]);
        if let Some(url) = search
            .custom_url
            .as_deref()
            .filter(|u| valid_search_template(u))
        {
            list.push(SearchEngine {
                id: "custom".into(),
                name: "Custom".into(),
                keyword: "custom".into(),
                url: url.into(),
                suggest_url: None,
                custom: true,
                extension: None,
            });
        }
        self.engine = engine_by_id(&list, search.engine_id.as_deref()).clone();
        self.engines = list;
        EngineInfo {
            id: self.engine.id.clone(),
            name: self.engine.name.clone(),
            url: self.engine.url.clone(),
            suggests: self
                .engine
                .suggest_url
                .as_deref()
                .is_some_and(|s| !s.is_empty()),
        }
    }

    /// `mobileEngines` (`mobile.ts:82`).
    pub fn engines(&self) -> Vec<EngineRow> {
        self.engines
            .iter()
            .map(|e| EngineRow {
                id: e.id.clone(),
                name: e.name.clone(),
                url: e.url.clone(),
                custom: e.custom,
            })
            .collect()
    }

    /// `mobileWarmBangs` (`mobile.ts:86`).
    pub fn warm_bangs(&mut self) -> usize {
        self.bang_table_once().size()
    }

    /// `mobileSuggest` (`mobile.ts:90`). `Err` is the `TypeError` of `urlForDisplay`.
    pub fn suggest(
        &mut self,
        text: &str,
        options: &MobileSuggestOptions,
    ) -> Result<SuggestionResult, &'static str> {
        if text.contains('!') {
            self.bang_table_once();
        }
        let opts = SuggestOptions {
            limit: options.limit.unwrap_or(8.0),
            now: options.now,
            engine: Some(SuggestEngine {
                name: self.engine.name.clone(),
                url: self.engine.url.clone(),
            }),
            preference: if options.search_preferred {
                crate::omnibox::suggestion::Preference::Search
            } else {
                crate::omnibox::suggestion::Preference::Website
            },
            remote: options.remote.clone(),
            current_tab_id: options.current_tab_id.clone(),
            current_url: options.current_url.clone(),
            scope: options.scope.clone(),
            prevent_inline: options.prevent_inline,
            bangs: self.bangs.as_ref().filter(|_| text.contains('!')),
            ..SuggestOptions::default()
        };
        let source = &self.source;
        let cached = &mut self.pool;
        let mut result = build_suggestions_lazy(text, source, &opts, || {
            let (pool, narrowing) = pool_of(source, cached)?;
            Ok((pool, narrowing))
        })?;
        result.items.retain(|s| {
            matches!(
                s,
                Suggestion::Page { .. } | Suggestion::Search { .. } | Suggestion::Calc { .. }
            )
        });
        result.via = None;
        Ok(result)
    }

    /// `mobileResolve` (`mobile.ts:106`).
    pub fn resolve(&mut self, text: &str, scope: Option<&SearchScope>) -> Option<Resolution> {
        let input = js_trim(text);
        if input.is_empty() {
            return None;
        }
        let search = |url: &str, query: &str, engine: &str| Resolution {
            kind: ResolutionKind::Search,
            url: search_url_for(url, query),
            query: query.into(),
            engine: engine.into(),
        };
        if let Some(scope) = scope {
            return Some(search(&scope.url, input, &scope.name));
        }
        if input.contains('!') {
            let table = self.bang_table_once();
            if let Some(bang) = find_bang(input, Some(table)) {
                return Some(Resolution {
                    kind: ResolutionKind::Bang,
                    url: bang.url,
                    query: bang.query,
                    engine: bang.bang.name.clone(),
                });
            }
        }
        if let Some(query) = input.strip_prefix('?') {
            let query = js_trim(query);
            return (!query.is_empty()).then(|| search(&self.engine.url, query, &self.engine.name));
        }
        if let Some(url) = fixup_url(input) {
            return Some(Resolution {
                kind: ResolutionKind::Url,
                url,
                query: input.into(),
                engine: self.engine.name.clone(),
            });
        }
        Some(search(&self.engine.url, input, &self.engine.name))
    }

    /// `mobileSuggestRequest` (`mobile.ts:118`).
    pub fn suggest_request(&mut self, text: &str, scope: Option<&SearchScope>) -> Option<String> {
        let text = js_trim(text);
        let query = js_trim(text.strip_prefix('?').unwrap_or(text));
        if query.is_empty() || split_scheme(query).is_some_and(|(_, rest)| rest.starts_with("//")) {
            return None;
        }
        if query.contains('!') && find_bang(query, Some(self.bang_table_once())).is_some() {
            return None;
        }
        if scope.is_some_and(|s| s.kind != ScopeKind::Engine) {
            return None;
        }
        let from = match scope {
            Some(s) => engine_by_id(&self.engines, s.engine_id.as_deref()),
            None => &self.engine,
        };
        suggest_request_url(from, query)
    }

    /// `mobileParseRemote` (`mobile.ts:122`): a body that is not JSON gives no suggestions.
    pub fn parse_remote(body: &str) -> Vec<String> {
        let mut queries = parse_json(body)
            .ok()
            .and_then(|body| suggest_reply(&body).map(|reply| parse_suggestions(&reply)))
            .unwrap_or_default();
        queries.truncate(8);
        queries
    }

    /// `mobileFindScope` (`mobile.ts:136`).
    pub fn find_scope(&self, text: &str) -> Option<SearchScope> {
        find_scope(
            text,
            &ScopeSources {
                engines: &self.engines,
                sites: None,
                hosts: &[],
                providers: &[],
            },
        )
    }

    /// `mobileClean` (`mobile.ts:140`).
    pub fn clean(url: &str) -> String {
        clean_url(url)
    }

    /// `mobileDisplay` (`mobile.ts:144`). `Err` is the `TypeError` of `urlForDisplay`.
    pub fn display(url: &str, title: &str) -> Result<Display, &'static str> {
        let display = url_for_display(url)?;
        let crumb = breadcrumb(url, Some(title))?;
        Ok(Display {
            display,
            host: crumb.host,
            trail: crumb.trail,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_picks_url_search_or_nothing() {
        let mut m = Mobile::default();
        assert!(m.resolve("   ", None).is_none());
        assert!(m.resolve("?", None).is_none());
        let url = m.resolve("example.com/a", None).unwrap();
        assert_eq!(url.kind, ResolutionKind::Url);
        assert_eq!(url.url, "https://example.com/a");
        let search = m.resolve("two words", None).unwrap();
        assert_eq!(search.kind, ResolutionKind::Search);
        assert_eq!(search.query, "two words");
        assert!(search.url.contains("two%20words"));
    }
}
