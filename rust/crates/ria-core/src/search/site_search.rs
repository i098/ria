// Ported from ardia packages/core/src/siteSearch.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Site search in the address bar: which engine, site, visited host or provider a typed word
//! scopes the search to, the URL of a scoped search, and the Tab, space and Backspace keys that
//! enter and leave a scope.

use std::sync::LazyLock;

use super::engines::{SearchEngine, engine_host, search_url};
use super::host::{host_of, search_url_for};
use super::js_text::{is_js_space, js_trim};

/// `SiteSearch` (`siteSearch.ts:4`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteSearch {
    pub host: String,
    pub name: String,
    pub url: String,
    pub keyword: Option<String>,
}

/// `(host, name, url, keyword)` of each popular site.
type Popular = (
    &'static str,
    &'static str,
    &'static str,
    Option<&'static str>,
);

const POPULAR: [Popular; 23] = [
    (
        "youtube.com",
        "YouTube",
        "https://www.youtube.com/results?search_query=%s",
        None,
    ),
    (
        "wikipedia.org",
        "Wikipedia",
        "https://en.wikipedia.org/w/index.php?search=%s",
        None,
    ),
    (
        "github.com",
        "GitHub",
        "https://github.com/search?q=%s",
        None,
    ),
    (
        "amazon.com",
        "Amazon",
        "https://www.amazon.com/s?k=%s",
        None,
    ),
    (
        "reddit.com",
        "Reddit",
        "https://www.reddit.com/search/?q=%s",
        None,
    ),
    ("x.com", "X", "https://x.com/search?q=%s", None),
    ("twitter.com", "X", "https://x.com/search?q=%s", None),
    (
        "stackoverflow.com",
        "Stack Overflow",
        "https://stackoverflow.com/search?q=%s",
        None,
    ),
    (
        "npmjs.com",
        "npm",
        "https://www.npmjs.com/search?q=%s",
        None,
    ),
    (
        "maps.google.com",
        "Google Maps",
        "https://www.google.com/maps/search/%s",
        Some("maps"),
    ),
    (
        "drive.google.com",
        "Google Drive",
        "https://drive.google.com/drive/search?q=%s",
        Some("drive"),
    ),
    (
        "mail.google.com",
        "Gmail",
        "https://mail.google.com/mail/u/0/#search/%s",
        Some("gmail"),
    ),
    ("imdb.com", "IMDb", "https://www.imdb.com/find/?q=%s", None),
    (
        "open.spotify.com",
        "Spotify",
        "https://open.spotify.com/search/%s",
        None,
    ),
    (
        "linkedin.com",
        "LinkedIn",
        "https://www.linkedin.com/search/results/all/?keywords=%s",
        None,
    ),
    (
        "ebay.com",
        "eBay",
        "https://www.ebay.com/sch/i.html?_nkw=%s",
        None,
    ),
    (
        "pinterest.com",
        "Pinterest",
        "https://www.pinterest.com/search/pins/?q=%s",
        None,
    ),
    (
        "news.ycombinator.com",
        "Hacker News",
        "https://hn.algolia.com/?q=%s",
        None,
    ),
    (
        "developer.mozilla.org",
        "MDN",
        "https://developer.mozilla.org/en-US/search?q=%s",
        None,
    ),
    (
        "figma.com",
        "Figma",
        "https://www.figma.com/files/search?q=%s",
        None,
    ),
    (
        "notion.so",
        "Notion",
        "https://www.notion.so/search?q=%s",
        None,
    ),
    (
        "apps.apple.com",
        "App Store",
        "https://www.apple.com/us/search/%s?src=serp",
        None,
    ),
    (
        "translate.google.com",
        "Google Translate",
        "https://translate.google.com/?text=%s",
        Some("translate"),
    ),
];

/// `POPULAR_SITE_SEARCHES` (`siteSearch.ts:6`): the sites `findScope` knows without `sites`.
pub static POPULAR_SITE_SEARCHES: LazyLock<Vec<SiteSearch>> = LazyLock::new(|| {
    POPULAR
        .iter()
        .map(|&(host, name, url, keyword)| SiteSearch {
            host: host.into(),
            name: name.into(),
            url: url.into(),
            keyword: keyword.map(Into::into),
        })
        .collect()
});

/// `SearchScope["kind"]` (`siteSearch.ts:33`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Engine,
    Site,
    History,
    Provider,
}

/// `SearchScope` (`siteSearch.ts:32`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchScope {
    pub kind: ScopeKind,
    pub name: String,
    pub host: String,
    pub url: String,
    pub engine_id: Option<String>,
    pub provider_id: Option<String>,
}

/// `KeywordProvider` (`siteSearch.ts:41`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordProvider {
    pub id: String,
    pub title: String,
    pub keyword: String,
    pub host: Option<String>,
    pub url: Option<String>,
}

/// `ScopeSources` (`siteSearch.ts:43`); `sites: None` is the popular sites.
#[derive(Debug, Clone, Copy)]
pub struct ScopeSources<'a> {
    pub engines: &'a [SearchEngine],
    pub sites: Option<&'a [SiteSearch]>,
    pub hosts: &'a [String],
    pub providers: &'a [KeywordProvider],
}

/// Whether the trimmed text is one word: not empty, without white space.
pub fn is_scope_word(text: &str) -> bool {
    let t = js_trim(text);
    !t.is_empty() && !t.contains(is_js_space)
}

/// Port of `providerScope` (`siteSearch.ts:50`): the provider whose keyword is the typed word.
pub fn provider_scope(text: &str, providers: &[KeywordProvider]) -> Option<SearchScope> {
    if providers.is_empty() || !is_scope_word(text) {
        return None;
    }
    let t = js_trim(text).to_lowercase();
    let p = providers.iter().find(|p| p.keyword.to_lowercase() == t)?;
    Some(SearchScope {
        kind: ScopeKind::Provider,
        name: p.title.clone(),
        host: p.host.clone().unwrap_or_default(),
        url: p.url.clone().unwrap_or_default(),
        engine_id: None,
        provider_id: Some(p.id.clone()),
    })
}

/// `normalize` (`siteSearch.ts:58`): trimmed, lower-cased, without scheme, `www.` and one
/// trailing slash.
fn normalize(text: &str) -> String {
    let t = js_trim(text).to_lowercase();
    let mut t = t.as_str();
    // `/^[a-z]+:\/\//`
    let letters = t.len() - t.trim_start_matches(|c: char| c.is_ascii_lowercase()).len();
    if letters > 0 && t[letters..].starts_with("://") {
        t = &t[letters + 3..];
    }
    let t = t.strip_prefix("www.").unwrap_or(t);
    t.strip_suffix('/').unwrap_or(t).into()
}

/// The text `findScope` (`siteSearch.ts:70`) matches against its sources: the normalized word,
/// or `None` when the text is empty or has white space, which matches nothing.
pub fn find_scope_text(text: &str) -> Option<String> {
    Some(normalize(text)).filter(|t| !t.is_empty() && !t.contains(is_js_space))
}

/// `siteLabel` (`siteSearch.ts:62`): the label before the last dot, or the whole host.
fn site_label(host: &str) -> &str {
    let mut parts = host.rsplit('.');
    match (parts.next(), parts.next()) {
        (Some(_), Some(label)) => label,
        _ => host,
    }
}

/// Port of `findScope` (`siteSearch.ts:67`): a provider keyword, then a custom engine keyword,
/// then a built-in engine's keyword, host or name, then a site, then a visited host.
pub fn find_scope(text: &str, sources: &ScopeSources) -> Option<SearchScope> {
    if let Some(provider) = provider_scope(text, sources.providers) {
        return Some(provider);
    }
    let t = find_scope_text(text)?;
    let host = Some(host_of(&format!("https://{t}")))
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| t.clone());
    let long = t.encode_utf16().count() >= 3;

    if let Some(e) = sources
        .engines
        .iter()
        .find(|e| e.custom && e.keyword.to_lowercase() == t)
    {
        return Some(engine_scope(e));
    }
    for e in sources.engines.iter().filter(|e| !e.custom) {
        let e_host = engine_host(&e.url);
        if e.keyword.to_lowercase() == host || e_host == host || (long && t == site_label(&e_host))
        {
            return Some(engine_scope(e));
        }
    }
    let sites = sources.sites.unwrap_or(&POPULAR_SITE_SEARCHES);
    let site = sites.iter().find(|s| {
        s.host == host
            || host
                .strip_suffix(s.host.as_str())
                .is_some_and(|rest| rest.ends_with('.'))
            || s.keyword.as_deref() == Some(t.as_str())
            || (t == host && long && s.host.split('.').count() == 2 && site_label(&s.host) == t)
    });
    if let Some(s) = site {
        return Some(SearchScope {
            kind: ScopeKind::Site,
            name: s.name.clone(),
            host: s.host.clone(),
            url: s.url.clone(),
            engine_id: None,
            provider_id: None,
        });
    }
    (host.contains('.') && sources.hosts.contains(&host)).then(|| SearchScope {
        kind: ScopeKind::History,
        name: host.clone(),
        host,
        url: String::new(),
        engine_id: None,
        provider_id: None,
    })
}

/// `engineScope` (`siteSearch.ts:91`).
fn engine_scope(e: &SearchEngine) -> SearchScope {
    SearchScope {
        kind: ScopeKind::Engine,
        name: e.name.clone(),
        host: engine_host(&e.url),
        url: e.url.clone(),
        engine_id: Some(e.id.clone()),
        provider_id: None,
    }
}

/// Whether `scopedSearchUrl` searches the scope with the default engine: a visited host (as a
/// `site:` query) or a provider without its own search URL.
pub fn searches_with_default_engine(scope: &SearchScope) -> bool {
    match scope.kind {
        ScopeKind::History => true,
        ScopeKind::Provider => scope.url.is_empty(),
        ScopeKind::Engine | ScopeKind::Site => false,
    }
}

/// Port of `scopedSearchUrl` (`siteSearch.ts:95`); `default_engine_url` is the default
/// engine's `url`.
pub fn scoped_search_url(scope: &SearchScope, query: &str, default_engine_url: &str) -> String {
    if !searches_with_default_engine(scope) {
        search_url_for(&scope.url, query)
    } else if scope.kind == ScopeKind::History {
        search_url(
            default_engine_url,
            &format!("site:{} {}", scope.host, js_trim(query)),
        )
    } else {
        search_url(default_engine_url, query)
    }
}

/// Port of `offeredSiteSearch` (`siteSearch.ts:101`): the scope the bar offers for the typed
/// word with its inline completion.
pub fn offered_site_search(
    typed: &str,
    completion: &str,
    sources: &ScopeSources,
) -> Option<SearchScope> {
    if !is_scope_word(typed) {
        return None;
    }
    provider_scope(typed, sources.providers)
        .or_else(|| find_scope(&format!("{typed}{completion}"), sources))
}

/// `SiteSearchKeyState` (`siteSearch.ts:107`).
#[derive(Debug, Clone, Copy)]
pub struct SiteSearchKeyState<'a> {
    pub typed: &'a str,
    pub completion: &'a str,
    pub scope: Option<&'a SearchScope>,
    pub offered: Option<&'a SearchScope>,
    pub scope_from: &'a str,
}

/// `SiteSearchKeyModifiers` (`siteSearch.ts:115`); a missing key is `false`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SiteSearchKeyModifiers {
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
    pub ctrl: bool,
}

/// `SiteSearchKey` (`siteSearch.ts:117`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SiteSearchKey<'a> {
    Enter {
        scope: &'a SearchScope,
        from: String,
    },
    Leave {
        text: &'a str,
    },
    Accept {
        text: String,
    },
}

/// The keys `siteSearchKey` acts on; only for these it reads the bar's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKey {
    /// Tab without Shift or Ctrl.
    Tab,
    /// Backspace without modifiers.
    Backspace,
}

/// The [`ScopeKey`] of a key press, or `None` for keys site search leaves alone.
pub fn scope_key(key: &str, m: &SiteSearchKeyModifiers) -> Option<ScopeKey> {
    match key {
        "Tab" if !m.shift && !m.ctrl => Some(ScopeKey::Tab),
        "Backspace" if !m.shift && !m.alt && !m.meta && !m.ctrl => Some(ScopeKey::Backspace),
        _ => None,
    }
}

/// `KeywordBySpace` (`siteSearch.ts:122`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeywordBySpace<'a> {
    pub keyword: &'a str,
    pub from: &'a str,
}

/// Port of `keywordBySpace` (`siteSearch.ts:124`): a space typed at the end of a word enters
/// its scope.
pub fn keyword_by_space<'a>(typed: &'a str, next: &'a str) -> Option<KeywordBySpace<'a>> {
    let ends_in_space = typed.chars().next_back().is_some_and(is_js_space);
    let added_space = next.strip_suffix(' ') == Some(typed);
    (!js_trim(typed).is_empty() && !ends_in_space && added_space).then_some(KeywordBySpace {
        keyword: typed,
        from: next,
    })
}

/// Port of `asksForBackspace` (`siteSearch.ts:129`).
pub fn asks_for_backspace(scope: Option<&SearchScope>) -> bool {
    scope.is_some()
}

/// Port of `siteSearchKey` (`siteSearch.ts:133`): Tab enters the offered scope or accepts the
/// completion; Backspace leaves the scope and gives back the text that entered it.
pub fn site_search_key<'a>(
    key: &str,
    modifiers: &SiteSearchKeyModifiers,
    state: &SiteSearchKeyState<'a>,
) -> Option<SiteSearchKey<'a>> {
    match scope_key(key, modifiers)? {
        ScopeKey::Tab => match (state.scope, state.offered) {
            (None, Some(offered)) => Some(SiteSearchKey::Enter {
                scope: offered,
                from: format!("{}{}", state.typed, state.completion),
            }),
            _ if !state.completion.is_empty() => Some(SiteSearchKey::Accept {
                text: format!("{}{}", state.typed, state.completion),
            }),
            _ => None,
        },
        ScopeKey::Backspace => state.scope.map(|_| SiteSearchKey::Leave {
            text: state.scope_from,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::super::engines::BUILT_IN_ENGINES;
    use super::*;

    fn sources() -> ScopeSources<'static> {
        ScopeSources {
            engines: &BUILT_IN_ENGINES,
            sites: None,
            hosts: &[],
            providers: &[],
        }
    }

    #[test]
    fn finds_engines_sites_and_hosts() {
        let name = |t: &str| find_scope(t, &sources()).map(|s| s.name);
        assert_eq!(name("https://www.Google.com/").as_deref(), Some("Google"));
        assert_eq!(name("goo"), None);
        assert_eq!(name("google").as_deref(), Some("Google"));
        assert_eq!(name("Maps").as_deref(), Some("Google Maps"));
        assert_eq!(name("en.wikipedia.org").as_deref(), Some("Wikipedia"));
        assert_eq!(name("maps near"), None);
        let hosts = ["example.org".to_string()];
        let s = find_scope(
            "example.org",
            &ScopeSources {
                hosts: &hosts,
                ..sources()
            },
        )
        .map(|s| s.kind);
        assert_eq!(s, Some(ScopeKind::History));
    }

    #[test]
    fn tab_enters_then_backspace_leaves() {
        let offered = offered_site_search("goo", "gle.com", &sources());
        let state = SiteSearchKeyState {
            typed: "goo",
            completion: "gle.com",
            scope: None,
            offered: offered.as_ref(),
            scope_from: "",
        };
        let none = SiteSearchKeyModifiers::default();
        let Some(SiteSearchKey::Enter { scope, from }) = site_search_key("Tab", &none, &state)
        else {
            panic!("Tab did not enter");
        };
        assert_eq!(
            (scope.name.as_str(), from.as_str()),
            ("Google", "google.com")
        );
        let inside = SiteSearchKeyState {
            typed: "",
            completion: "",
            scope: Some(scope),
            offered: None,
            scope_from: "google.com",
        };
        assert_eq!(
            site_search_key("Backspace", &none, &inside),
            Some(SiteSearchKey::Leave { text: "google.com" })
        );
        let alt = SiteSearchKeyModifiers { alt: true, ..none };
        assert_eq!(site_search_key("Backspace", &alt, &inside), None);
    }

    #[test]
    fn history_scope_searches_site_with_default_engine() {
        let scope = SearchScope {
            kind: ScopeKind::History,
            name: "a.org".into(),
            host: "a.org".into(),
            url: String::new(),
            engine_id: None,
            provider_id: None,
        };
        assert_eq!(
            scoped_search_url(&scope, " cats ", "https://s/?q=%s"),
            "https://s/?q=site%3Aa.org%20cats"
        );
    }
}
