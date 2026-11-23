// Ported from ardia packages/core/src/extensionRows.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! Address bar rows for installed extensions: a typed extension name lists its toolbar action
//! and its options page.

use crate::fuzzy::{ActionCandidate, match_actions};

use super::js_text::js_trim;

/// `ExtensionCandidate` (`extensionRows.ts:4`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionCandidate {
    pub id: String,
    pub title: String,
    pub icon: Option<String>,
    pub action: bool,
    pub options: bool,
}

impl ActionCandidate for ExtensionCandidate {
    fn title(&self) -> &str {
        &self.title
    }
    fn keywords(&self) -> &[String] {
        &[]
    }
}

/// What an extension row opens: the toolbar action or the options page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionOpen {
    Action,
    Options,
}

/// The `kind: "extension"` variant of `Suggestion` (`suggestion.ts:30`), the only one this module
/// builds; `suggestion.ts` is ported in its own slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Suggestion {
    Extension {
        id: String,
        title: String,
        icon: Option<String>,
        open: ExtensionOpen,
    },
}

/// `ExtensionRows` (`extensionRows.ts:12`): the rows, and whether the best match is the exact name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtensionRows {
    pub rows: Vec<Suggestion>,
    pub exact: bool,
}

/// Queries shorter than this list only exact names (`extensionRows.ts:15`).
const MIN_PARTIAL_QUERY: usize = 3;
/// The lowest fuzzy score of a partial match (`extensionRows.ts:16`).
const MIN_SCORE: i64 = 50;
/// The default `limit` of `extensionSuggestions`.
pub const DEFAULT_LIMIT: usize = 2;

/// Port of `extensionSuggestions` (`extensionRows.ts:18`): the best `limit` matching extensions,
/// each as its action row and, for the best match or an extension without action, its options row.
pub fn extension_suggestions(
    query: &str,
    extensions: &[ExtensionCandidate],
    limit: usize,
) -> ExtensionRows {
    if extensions.is_empty() {
        return ExtensionRows::default();
    }
    let q = js_trim(query);
    let partial = q.encode_utf16().count() >= MIN_PARTIAL_QUERY;
    let matches: Vec<_> = match_actions(q, extensions, limit)
        .into_iter()
        .filter(|m| m.exact || (partial && m.score >= MIN_SCORE))
        .collect();
    let mut rows = Vec::new();
    for (i, m) in matches.iter().enumerate() {
        let x = m.action;
        let row = |open| Suggestion::Extension {
            id: x.id.clone(),
            title: x.title.clone(),
            icon: x.icon.clone(),
            open,
        };
        if x.action {
            rows.push(row(ExtensionOpen::Action));
        }
        if x.options && (i == 0 || !x.action) {
            rows.push(row(ExtensionOpen::Options));
        }
    }
    match matches.first() {
        Some(best) if !rows.is_empty() => ExtensionRows {
            rows,
            exact: best.exact,
        },
        _ => ExtensionRows::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ext(id: &str, title: &str, action: bool, options: bool) -> ExtensionCandidate {
        ExtensionCandidate {
            id: id.into(),
            title: title.into(),
            icon: None,
            action,
            options,
        }
    }

    fn keys(rows: &ExtensionRows) -> Vec<String> {
        rows.rows
            .iter()
            .map(|Suggestion::Extension { id, open, .. }| format!("{open:?}:{id}"))
            .collect()
    }

    #[test]
    fn options_only_under_best_match_or_without_action() {
        let exts = [
            ext("ublock", "uBlock Origin", true, true),
            ext("darkreader", "Dark Reader", true, true),
            ext("nopopup", "Video Speed Controller", false, true),
        ];
        assert_eq!(
            keys(&extension_suggestions("ublock", &exts, DEFAULT_LIMIT)),
            ["Action:ublock", "Options:ublock"]
        );
        assert_eq!(
            keys(&extension_suggestions("video speed", &exts, DEFAULT_LIMIT)),
            ["Options:nopopup"]
        );
        assert!(
            extension_suggestions("ub", &exts, DEFAULT_LIMIT)
                .rows
                .is_empty()
        );
        let short = extension_suggestions("ab", &[ext("ab", "AB", true, true)], DEFAULT_LIMIT);
        assert!(short.exact);
        assert_eq!(keys(&short), ["Action:ab", "Options:ab"]);
    }
}
