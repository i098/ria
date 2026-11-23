// Ported from ardia packages/core/src/candidatePool.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The pages the address bar ranks: history and bookmarks as one keyed pool, and the open tabs
//! over that pool.
//!
//! The original caches a pool for each pair of history and bookmark arrays (by identity, in
//! `WeakMap`s), builds it in time slices before a deadline (`prepareSuggestions`), and caches
//! `tabsOver` and the exact-URL indexes the same way. Those caches only save time: the values
//! are the same. Here [`Pool::new`] builds the whole pool at once and is the port of both
//! `poolFor` and `prepareSuggestions`; the caller keeps the pool while its rows do not change.
//! [`Pool::id`] stands in for the identity the original compares (`MatchedChain` of
//! `page_match`).

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use super::omnibox::url_for_display;
use super::suggestion::{BookmarkRow, HistoryRow, SourceTab};
use super::suggestion_rows::PageRow;
use crate::url::page_text::{PageText, display_url, log_visits, with_root_slash};

/// `Candidate` (`candidatePool.ts:8`): a page the address bar can rank. The optional booleans of
/// the original are `false` when absent.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub url: String,
    pub title: String,
    pub page_title: Option<String>,
    pub renamed: bool,
    pub favicon: Option<String>,
    pub visits: f64,
    pub last_visit: f64,
    pub log_visits: f64,
    pub bookmarked: bool,
    pub tab_id: Option<String>,
    pub fresh: bool,
    pub text: PageText,
}

impl Candidate {
    /// The fields `pageSuggestion` reads.
    pub fn page_row(&self) -> PageRow<'_> {
        PageRow {
            url: &self.url,
            title: &self.title,
            favicon: self.favicon.as_deref(),
            visits: self.visits,
            bookmarked: self.bookmarked,
        }
    }
}

/// `pageText(row)` (`pageText.ts:36`). `Err` is the `TypeError` of `urlForDisplay`.
pub fn page_text(url: &str, title: &str) -> Result<PageText, &'static str> {
    let shown = url_for_display(url)?;
    Ok(PageText::new(url, title, display_url(url, |_| shown)))
}

static NEXT_POOL_ID: AtomicU64 = AtomicU64::new(0);

/// `Pool` (`candidatePool.ts:24`): history pages first (one per page key, the first row of a
/// key), then bookmarks of other pages; `by_key` finds each by its page key.
#[derive(Debug, Clone)]
pub struct Pool {
    id: u64,
    pub list: Vec<Candidate>,
    by_key: HashMap<String, usize>,
}

impl Pool {
    /// Port of `poolFor` (`candidatePool.ts:102`) and `prepareSuggestions` (`:108`). `Err` is
    /// the `TypeError` of `urlForDisplay`.
    pub fn new(history: &[HistoryRow], bookmarks: &[BookmarkRow]) -> Result<Self, &'static str> {
        let mut marked: Vec<Candidate> = Vec::new();
        let mut marked_keys = HashSet::new();
        for row in bookmarks {
            let text = page_text(&row.url, &row.title)?;
            if marked_keys.insert(text.key.clone()) {
                marked.push(Candidate {
                    url: row.url.clone(),
                    title: row.title.clone(),
                    page_title: None,
                    renamed: false,
                    favicon: row.favicon.clone(),
                    visits: 0.0,
                    last_visit: 0.0,
                    log_visits: 0.0,
                    bookmarked: true,
                    tab_id: None,
                    fresh: false,
                    text,
                });
            }
        }
        let mut list = Vec::with_capacity(history.len() + marked.len());
        let mut by_key = HashMap::with_capacity(list.capacity());
        for h in history {
            let text = page_text(&h.url, &h.title)?;
            if by_key.contains_key(&text.key) {
                continue;
            }
            by_key.insert(text.key.clone(), list.len());
            list.push(Candidate {
                url: h.url.clone(),
                title: h.title.clone(),
                page_title: None,
                renamed: false,
                favicon: h.favicon.clone(),
                visits: h.visits,
                last_visit: h.last_visit,
                log_visits: log_visits(h.visits),
                bookmarked: marked_keys.contains(&text.key),
                tab_id: None,
                fresh: false,
                text,
            });
        }
        // The original looks a bookmark up in its own map; one not in the list has a visited key.
        for c in marked {
            if !by_key.contains_key(&c.text.key) {
                by_key.insert(c.text.key.clone(), list.len());
                list.push(c);
            }
        }
        Ok(Self {
            id: NEXT_POOL_ID.fetch_add(1, Ordering::Relaxed),
            list,
            by_key,
        })
    }

    /// A number no other pool of this process has: the identity of the original's pool object.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// `pool.byKey.get(key)`.
    pub fn by_key(&self, key: &str) -> Option<&Candidate> {
        self.by_key.get(key).map(|&i| &self.list[i])
    }

    /// `pool.byKey.has(key)`.
    pub fn has_key(&self, key: &str) -> bool {
        self.by_key.contains_key(key)
    }
}

/// Whether `withRootSlash(url) === key`, where `key` is a `withRootSlash` result; the rows of a
/// long history are compared without a copy each.
fn root_slash_eq(url: &str, key: &str) -> bool {
    url == key || (url.len() + 1 == key.len() && with_root_slash(url) == key)
}

/// Port of `savedVisit` (`candidatePool.ts:138`): the first history row of exactly `url`, with
/// `/` after the host.
pub fn saved_visit<'a>(history: &'a [HistoryRow], url: &str) -> Option<&'a HistoryRow> {
    let key = with_root_slash(url);
    history.iter().find(|row| root_slash_eq(&row.url, &key))
}

/// Port of `savedBookmark` (`candidatePool.ts:139`).
pub fn saved_bookmark<'a>(bookmarks: &'a [BookmarkRow], url: &str) -> Option<&'a BookmarkRow> {
    let key = with_root_slash(url);
    bookmarks.iter().find(|row| root_slash_eq(&row.url, &key))
}

/// `TabsOver` (`candidatePool.ts:141`) without the cache fields: the tab of each page key, the
/// tabs to rank besides the pool, and the keys of pool pages a renamed tab hides.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TabsOver {
    pub tab_ids: HashMap<String, String>,
    pub extra: Vec<Candidate>,
    pub renamed: HashSet<String>,
}

/// `t.customTitle` when truthy.
fn custom_title(t: &SourceTab) -> Option<&str> {
    t.custom_title.as_deref().filter(|c| !c.is_empty())
}

/// Port of `tabsOver` (`candidatePool.ts:144`): open tabs other than the current one. A page has
/// the tab of its first renamed tab, else its first tab; a tab the pool lacks is an extra
/// candidate, and a renamed tab is one too, with the pool page's history. `Err` is the
/// `TypeError` of `urlForDisplay`.
pub fn tabs_over(
    pool: &Pool,
    tabs: &[SourceTab],
    current_tab_id: Option<&str>,
) -> Result<TabsOver, &'static str> {
    let mut open = Vec::new(); // (tab, text, first tab of its page)
    let mut tab_ids = HashMap::new();
    let mut renamed_tab_keys = HashSet::new();
    for t in tabs {
        if t.url.is_empty() || current_tab_id == Some(t.id.as_str()) {
            continue;
        }
        let text = page_text(&t.url, &t.title)?;
        let mut first = false;
        if custom_title(t).is_none() {
            if !tab_ids.contains_key(&text.key) {
                tab_ids.insert(text.key.clone(), t.id.clone());
                first = true;
            }
        } else if renamed_tab_keys.insert(text.key.clone()) {
            tab_ids.insert(text.key.clone(), t.id.clone());
        }
        open.push((t, text, first));
    }
    let mut extra = Vec::new();
    let mut renamed = HashSet::new();
    for (t, text, first) in open {
        let listed = pool.by_key(&text.key);
        let tab = |title: &str, renamed: bool, text: PageText| Candidate {
            url: t.url.clone(),
            title: title.to_string(),
            page_title: Some(t.title.clone()),
            renamed,
            favicon: t.favicon.clone(),
            visits: 0.0,
            last_visit: 0.0,
            log_visits: 0.0,
            bookmarked: false,
            tab_id: Some(t.id.clone()),
            fresh: true,
            text,
        };
        match (custom_title(t), listed) {
            (None, None) => {
                if first && !renamed_tab_keys.contains(&text.key) {
                    extra.push(tab(&t.title, false, text));
                }
            }
            (None, Some(_)) => {}
            (Some(name), None) => {
                let text = PageText::renamed(&text, name, None);
                extra.push(tab(name, true, text));
            }
            (Some(name), Some(listed)) => {
                renamed.insert(text.key.clone());
                extra.push(Candidate {
                    url: t.url.clone(),
                    title: name.to_string(),
                    page_title: None,
                    renamed: true,
                    favicon: t.favicon.clone().or_else(|| listed.favicon.clone()),
                    visits: listed.visits,
                    last_visit: listed.last_visit,
                    log_visits: listed.log_visits,
                    bookmarked: listed.bookmarked,
                    tab_id: Some(t.id.clone()),
                    fresh: false,
                    text: PageText::renamed(&text, name, Some(&listed.text)),
                });
            }
        }
    }
    Ok(TabsOver {
        tab_ids,
        extra,
        renamed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visit(url: &str, visits: f64) -> HistoryRow {
        HistoryRow {
            url: url.into(),
            title: url.into(),
            favicon: None,
            visits,
            last_visit: 1.0,
        }
    }

    fn mark(url: &str) -> BookmarkRow {
        BookmarkRow {
            url: url.into(),
            title: "b".into(),
            favicon: Some("i".into()),
        }
    }

    fn tab(id: &str, url: &str, custom: Option<&str>) -> SourceTab {
        SourceTab {
            id: id.into(),
            url: url.into(),
            title: "t".into(),
            custom_title: custom.map(Into::into),
            favicon: None,
        }
    }

    #[test]
    fn pool_lists_history_then_bookmarks_of_unvisited_pages() {
        let history = [
            visit("https://a.test/", 3.0),
            visit("https://a.test", 9.0),
            visit("https://b.test/", 1.0),
        ];
        let bookmarks = [mark("https://b.test"), mark("https://c.test/#x")];
        let pool = Pool::new(&history, &bookmarks).expect("pool");
        let urls: Vec<_> = pool.list.iter().map(|c| c.url.as_str()).collect();
        assert_eq!(
            urls,
            ["https://a.test/", "https://b.test/", "https://c.test/#x"]
        );
        assert!(pool.list[1].bookmarked && !pool.list[0].bookmarked);
        assert_eq!(pool.by_key("https://c.test").map(|c| c.visits), Some(0.0));
        assert!(!pool.has_key("https://d.test"));
        assert_ne!(pool.id(), Pool::new(&[], &[]).expect("pool").id());
        assert_eq!(
            saved_visit(&history, "https://a.test").map(|r| r.visits),
            Some(3.0)
        );
        assert!(saved_bookmark(&bookmarks, "https://c.test/").is_none());
    }

    #[test]
    fn tabs_over_prefers_renamed_tabs_and_hides_their_pool_page() {
        let pool = Pool::new(&[visit("https://a.test/", 2.0)], &[]).expect("pool");
        let tabs = [
            tab("1", "https://a.test/", None),
            tab("2", "https://a.test/", Some("Mine")),
            tab("3", "https://n.test/", None),
            tab("4", "https://n.test/", None),
            tab("5", "https://cur.test/", None),
        ];
        let over = tabs_over(&pool, &tabs, Some("5")).expect("tabs");
        assert_eq!(over.tab_ids["https://a.test"], "2");
        assert_eq!(over.tab_ids["https://n.test"], "3");
        assert!(over.renamed.contains("https://a.test"));
        let ids: Vec<_> = over.extra.iter().map(|c| c.tab_id.as_deref()).collect();
        assert_eq!(ids, [Some("2"), Some("3")]);
        assert_eq!(over.extra[0].visits, 2.0);
    }
}
