// Ported from ardia packages/core/src/suggestion.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The types of the address bar suggestions: the rows of the dropdown, what they are built from,
//! and the options of `buildSuggestions`.
//!
//! [`Suggestion`] has one variant for each `kind` of the original union:
//!
//! - `Page` (`"page"`): a web page, maybe an open tab (`tab_id`), bookmarked or visited.
//! - `Search` (`"search"`): a search with an engine; `suggested` for a remote suggestion, `bang`
//!   for a `!bang` search.
//! - `Calc` (`"calc"`): a calculator answer and the search URL of its expression.
//! - `Action` (`"action"`): a browser command.
//! - `Create` (`"create"`): a quick-create shortcut that opens a blank document of a web app.
//! - `Extension` (`"extension"`): the toolbar action or the options page of an extension.
//! - `Provider` (`"provider"`): a row from a search provider extension.
//!
//! An optional boolean field of the original (`bookmarked?`, `visited?`, `suggested?`,
//! `bang?: true`) is `false` here when the field is absent: the original never sets one to
//! `false`. An optional string field is `None` when absent.

use crate::fuzzy::ActionCandidate;
use crate::search::bangs::BangTable;
use crate::search::engines::RemoteSite;
use crate::search::extension_rows::{ExtensionCandidate, ExtensionOpen};
use crate::search::provider_rows::{ProviderSection, ProviderSuggestion};
use crate::search::site_search::SearchScope;

/// An open tab of `SuggestionSource.tabs`.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceTab {
    pub id: String,
    pub url: String,
    pub title: String,
    pub custom_title: Option<String>,
    pub favicon: Option<String>,
}

/// A visited page of `SuggestionSource.history`.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryRow {
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub visits: f64,
    pub last_visit: f64,
}

/// A bookmark of `SuggestionSource.bookmarks`.
#[derive(Debug, Clone, PartialEq)]
pub struct BookmarkRow {
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
}

/// `SuggestionSource` (`suggestion.ts:8`). The optional `bookmarks` of the original is an empty
/// list here when absent: the original reads it as `bookmarks ?? []`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SuggestionSource {
    pub tabs: Vec<SourceTab>,
    pub history: Vec<HistoryRow>,
    pub bookmarks: Vec<BookmarkRow>,
}

/// `CommandAction` (`suggestion.ts:14`): an `ActionCandidate` with an icon and a hint. The
/// optional `keywords` of the original is an empty list when absent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandAction {
    pub id: String,
    pub title: String,
    pub keywords: Vec<String>,
    pub icon: Option<String>,
    pub hint: Option<String>,
}

impl ActionCandidate for CommandAction {
    fn title(&self) -> &str {
        &self.title
    }
    fn keywords(&self) -> &[String] {
        &self.keywords
    }
}

/// `Suggestion` (`suggestion.ts:16`): one row of the address bar dropdown. See the module docs.
#[derive(Debug, Clone, PartialEq)]
pub enum Suggestion {
    Page {
        url: String,
        title: String,
        favicon: Option<String>,
        tab_id: Option<String>,
        bookmarked: bool,
        visited: bool,
    },
    Search {
        query: String,
        url: String,
        engine: String,
        suggested: bool,
        bang: bool,
    },
    Calc {
        expression: String,
        value: String,
        url: String,
    },
    Action {
        id: String,
        title: String,
        icon: Option<String>,
        hint: Option<String>,
    },
    Create {
        id: String,
        title: String,
        url: String,
        site: String,
    },
    Extension {
        id: String,
        title: String,
        icon: Option<String>,
        open: ExtensionOpen,
    },
    Provider(ProviderSuggestion),
}

impl Suggestion {
    /// The `kind` of the original.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Page { .. } => "page",
            Self::Search { .. } => "search",
            Self::Calc { .. } => "calc",
            Self::Action { .. } => "action",
            Self::Create { .. } => "create",
            Self::Extension { .. } => "extension",
            Self::Provider(_) => "provider",
        }
    }

    /// The `url` field, for the kinds that have one.
    pub fn url(&self) -> Option<&str> {
        match self {
            Self::Page { url, .. }
            | Self::Search { url, .. }
            | Self::Calc { url, .. }
            | Self::Create { url, .. } => Some(url),
            Self::Provider(p) => Some(&p.url),
            Self::Action { .. } | Self::Extension { .. } => None,
        }
    }
}

/// `SuggestionVia` (`suggestion.ts:33`): why a row is shown, for telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionVia {
    Literal,
    RenamedTab,
    EngineSite,
    SearchFirstInline,
    SpacedAddress,
}

impl SuggestionVia {
    /// The string of the original.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Literal => "literal",
            Self::RenamedTab => "renamed_tab",
            Self::EngineSite => "engine_site",
            Self::SearchFirstInline => "search_first_inline",
            Self::SpacedAddress => "spaced_address",
        }
    }
}

/// `SuggestionResult` (`suggestion.ts:35`). `via` maps a `suggestion_key` to its reason, in the
/// order of `items`; `None` when the original leaves the field out.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SuggestionResult {
    pub items: Vec<Suggestion>,
    pub completion: String,
    pub via: Option<Vec<(String, SuggestionVia)>>,
}

/// `Pick<SearchEngine, "name" | "url">`: the engine of `SuggestOptions.engine`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuggestEngine {
    pub name: String,
    pub url: String,
}

/// `SuggestOptions.preference`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Preference {
    #[default]
    Website,
    Search,
}

/// `SuggestOptions` (`suggestion.ts:41`). Absent lists are empty, as the original reads them
/// with `?? []`; `limit` defaults to 8 as in `buildSuggestions`. `now: None` is `Date.now()`
/// and `engine: None` the first built-in engine, which the caller supplies. No `Debug` or
/// `PartialEq`: [`BangTable`] has neither.
#[derive(Clone)]
pub struct SuggestOptions<'a> {
    pub limit: f64,
    pub now: Option<f64>,
    pub engine: Option<SuggestEngine>,
    pub preference: Preference,
    pub remote: Vec<String>,
    pub remote_sites: Vec<RemoteSite>,
    pub actions: Vec<CommandAction>,
    pub extensions: Vec<ExtensionCandidate>,
    pub current_tab_id: Option<String>,
    pub current_url: Option<String>,
    pub scope: Option<SearchScope>,
    pub prevent_inline: bool,
    pub bangs: Option<&'a BangTable>,
    pub providers: Vec<ProviderSection>,
}

/// The default `limit` of `buildSuggestions`.
pub const DEFAULT_LIMIT: f64 = 8.0;

impl Default for SuggestOptions<'_> {
    fn default() -> Self {
        Self {
            limit: DEFAULT_LIMIT,
            now: None,
            engine: None,
            preference: Preference::default(),
            remote: Vec::new(),
            remote_sites: Vec::new(),
            actions: Vec::new(),
            extensions: Vec::new(),
            current_tab_id: None,
            current_url: None,
            scope: None,
            prevent_inline: false,
            bangs: None,
            providers: Vec::new(),
        }
    }
}
