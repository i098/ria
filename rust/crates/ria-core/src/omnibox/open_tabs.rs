// Ported from ardia packages/core/src/openTabs.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The recently used tabs the address bar lists before anything is typed.

use std::cmp::Ordering;

use super::suggestion::Suggestion;

/// `OpenTabRow` (`openTabs.ts:3`).
#[derive(Debug, Clone, PartialEq)]
pub struct OpenTabRow {
    pub id: String,
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub last_active_at: f64,
}

/// The default `limit` of `recentTabSuggestions`.
pub const DEFAULT_LIMIT: usize = 8;

/// Port of `recentTabSuggestions` (`openTabs.ts:5`): up to `limit` tabs with a URL, other than
/// `exclude`, most recently active first (ties keep their order). Each row keeps its `tab_id`,
/// even an empty one.
pub fn recent_tab_suggestions(
    tabs: &[OpenTabRow],
    exclude: Option<&str>,
    limit: usize,
) -> Vec<Suggestion> {
    let mut shown: Vec<&OpenTabRow> = tabs
        .iter()
        .filter(|t| !t.url.is_empty() && exclude != Some(t.id.as_str()))
        .collect();
    // `b - a` is NaN for equal infinities, which `Array.prototype.sort` takes as a tie.
    shown.sort_by(|a, b| {
        b.last_active_at
            .partial_cmp(&a.last_active_at)
            .unwrap_or(Ordering::Equal)
    });
    shown
        .into_iter()
        .take(limit)
        .map(|t| Suggestion::Page {
            url: t.url.clone(),
            title: t.title.clone(),
            favicon: t.favicon.clone(),
            tab_id: Some(t.id.clone()),
            bookmarked: false,
            visited: false,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, url: &str, at: f64) -> OpenTabRow {
        OpenTabRow {
            id: id.into(),
            url: url.into(),
            title: String::new(),
            favicon: None,
            last_active_at: at,
        }
    }

    #[test]
    fn lists_recent_tabs_first_without_the_excluded_or_blank_ones() {
        let tabs = [
            row("a", "https://a", 1.0),
            row("b", "https://b", 3.0),
            row("c", "", 9.0),
            row("d", "https://d", 3.0),
            row("e", "https://e", 5.0),
        ];
        let ids: Vec<_> = recent_tab_suggestions(&tabs, Some("e"), 2)
            .iter()
            .map(|s| s.url().unwrap_or_default().to_string())
            .collect();
        assert_eq!(ids, ["https://b", "https://d"]);
    }
}
