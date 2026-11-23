// Ported from ardia packages/core/src/suggestionRows.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Building address bar rows, their identity keys, and the deduplicating row list.

use std::collections::HashSet;

use super::suggestion::{CommandAction, Suggestion};
use crate::search::extension_rows::ExtensionOpen;
use crate::search::js_text::js_trim;
use crate::url::page_text::{page_key, with_root_slash};

/// `PageRow` (`suggestionRows.ts:5`): what `page_suggestion` reads of a candidate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageRow<'a> {
    pub url: &'a str,
    pub title: &'a str,
    pub favicon: Option<&'a str>,
    pub visits: f64,
    pub bookmarked: bool,
}

/// Port of `pageSuggestion` (`suggestionRows.ts:7`). The original's `tabId` defaults to the
/// row's own `tabId`; here the caller passes it. An empty `tab_id` is left out, as in JS.
pub fn page_suggestion(c: PageRow<'_>, tab_id: Option<&str>) -> Suggestion {
    Suggestion::Page {
        url: c.url.to_string(),
        title: c.title.to_string(),
        favicon: c.favicon.map(str::to_string),
        tab_id: tab_id.filter(|t| !t.is_empty()).map(str::to_string),
        bookmarked: c.bookmarked,
        visited: c.visits > 0.0,
    }
}

/// Port of `suggestionKey` (`suggestionRows.ts:17`): rows with the same key are one row.
pub fn suggestion_key(s: &Suggestion) -> String {
    match s {
        Suggestion::Page { url, tab_id, .. } => match tab_id.as_deref() {
            Some(t) if !t.is_empty() => format!("tab:{}", page_key(url)),
            _ => format!("page:{}", with_root_slash(url)),
        },
        Suggestion::Search { url, .. } => format!("search:{url}"),
        Suggestion::Calc { expression, .. } => format!("calc:{expression}"),
        Suggestion::Action { id, .. } => format!("action:{id}"),
        Suggestion::Create { id, .. } => format!("create:{id}"),
        Suggestion::Extension { id, open, .. } => {
            let open = match open {
                ExtensionOpen::Action => "action",
                ExtensionOpen::Options => "options",
            };
            format!("extension:{open}:{id}")
        }
        Suggestion::Provider(p) => format!("provider:{}:{}", p.provider_id, page_key(&p.url)),
    }
}

/// Port of `Output` (`suggestionRows.ts:36`): up to `limit` rows, the first of each key.
#[derive(Debug, Clone)]
pub struct Output {
    items: Vec<Suggestion>,
    seen: HashSet<String>,
    limit: f64,
}

impl Output {
    pub fn new(limit: f64) -> Self {
        Self {
            items: Vec::new(),
            seen: HashSet::new(),
            limit,
        }
    }

    /// Adds `row` unless the list is full or holds its key; `true` when added. The original
    /// checks that with `items.includes(row)`.
    pub fn push(&mut self, row: Suggestion) -> bool {
        if self.items.len() as f64 >= self.limit {
            return false;
        }
        if !self.seen.insert(suggestion_key(&row)) {
            return false;
        }
        self.items.push(row);
        true
    }

    /// `push(...rows)`.
    pub fn extend(&mut self, rows: impl IntoIterator<Item = Suggestion>) {
        for row in rows {
            self.push(row);
        }
    }

    pub fn items(&self) -> &[Suggestion] {
        &self.items
    }

    pub fn into_items(self) -> Vec<Suggestion> {
        self.items
    }
}

/// Port of `createRow` (`suggestionRows.ts:53`).
pub fn create_row(id: &str, title: &str, url: &str, site: &str) -> Suggestion {
    Suggestion::Create {
        id: id.to_string(),
        title: title.to_string(),
        url: url.to_string(),
        site: site.to_string(),
    }
}

/// Port of `actionRow` (`suggestionRows.ts:55`): an empty icon or hint is left out.
pub fn action_row(a: &CommandAction) -> Suggestion {
    let some = |s: &Option<String>| s.as_ref().filter(|s| !s.is_empty()).cloned();
    Suggestion::Action {
        id: a.id.clone(),
        title: a.title.clone(),
        icon: some(&a.icon),
        hint: some(&a.hint),
    }
}

/// Port of `dedupeSuggestions` (`suggestionRows.ts:63`): `remote` without blanks, repeats and the
/// query itself, compared trimmed and lowercased.
pub fn dedupe_suggestions<'a>(remote: &'a [String], query: &str) -> Vec<&'a str> {
    let mut seen = HashSet::from([js_trim(query).to_lowercase()]);
    remote
        .iter()
        .map(String::as_str)
        .filter(|s| {
            let key = js_trim(s).to_lowercase();
            !key.is_empty() && seen.insert(key)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(url: &str, tab_id: Option<&str>) -> Suggestion {
        let row = PageRow {
            url,
            title: "",
            favicon: None,
            visits: 0.0,
            bookmarked: false,
        };
        page_suggestion(row, tab_id)
    }

    #[test]
    fn output_keeps_the_first_row_of_a_key_up_to_the_limit() {
        let mut out = Output::new(2.0);
        assert!(out.push(page("https://a.test", None)));
        assert!(!out.push(page("https://a.test/", None)));
        assert!(out.push(page("https://a.test/#x", Some("1"))));
        assert!(!out.push(page("https://b.test", None)));
        assert_eq!(out.items().len(), 2);
        assert_eq!(suggestion_key(&out.items()[1]), "tab:https://a.test");
    }

    #[test]
    fn dedupe_drops_blanks_repeats_and_the_query() {
        let remote = ["A".into(), " a ".into(), "Q".into(), " ".into(), "b".into()];
        assert_eq!(dedupe_suggestions(&remote, " q"), ["A", "b"]);
    }
}
