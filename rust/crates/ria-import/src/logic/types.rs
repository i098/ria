// Ported from ardia packages/import/src/index.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The types of `index.ts` (lines 4 to 234). Names follow the wire: serde writes the camelCase
//! keys of the TypeScript types and leaves out each absent optional field, as JSON does for
//! `undefined`. A TypeScript `number` is `f64`; `TabGroupColor | (string & {})` is a `String`.
//!
//! Changes from Ardia: `ImportWarning` is one struct, not a union of two shapes, so the pairing of
//! `kind` with its codes is not checked on read; `ImportEvent` is read by tag, so an event of an
//! unknown `type` is an error of the reader, not a silent skip.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportKind {
    Bookmarks,
    History,
    Tabs,
    Passwords,
    Cookies,
    Autofill,
    Spaces,
    PinnedTabs,
    Favorites,
    LinkRules,
    Archive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BrowserFamily {
    Chromium,
    Firefox,
    Safari,
    Arc,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceSummary {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub pinned_count: f64,
    pub tab_count: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserProfile {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub is_default: bool,
    pub available: Vec<ImportKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spaces: Option<Vec<SpaceSummary>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tabs_need_key: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dia_sidebar: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Sidebar {
    Arc,
    Zen,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserSource {
    pub id: String,
    pub name: String,
    pub family: BrowserFamily,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_path: Option<String>,
    pub requires_export: bool,
    pub needs_keychain: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs_full_disk_access: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keychain_service: Option<String>,
    pub profiles: Vec<BrowserProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sidebar: Option<Sidebar>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asks_app_for_tabs: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BookmarkRole {
    Toolbar,
    Other,
    Mobile,
    Menu,
    ReadingList,
    LiveGitHubPullRequests,
    LiveBitbucketPullRequests,
    Split,
    List,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BookmarkType {
    Folder,
    Url,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkNode {
    #[serde(rename = "type")]
    pub kind: BookmarkType,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_added: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<BookmarkNode>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<BookmarkRole>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedHistoryEntry {
    pub url: String,
    pub title: String,
    pub visits: f64,
    pub last_visit: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typed_count: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedTab {
    pub url: String,
    pub title: String,
    pub pinned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split_id: Option<String>,
    pub window_index: f64,
    pub active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_active: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedTabGroup {
    pub id: String,
    pub title: String,
    pub color: String,
    pub collapsed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credential {
    pub url: String,
    pub username: String,
    pub password: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub realm: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub otp_auth: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_used: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub times_used: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceSuggestion {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colors: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub profile_id: String,
    pub pinned: Vec<BookmarkNode>,
    pub tabs: Vec<ImportedTab>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_groups: Option<Vec<BookmarkNode>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favorites: Option<Vec<ImportedTab>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSuggestion {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WarningCode {
    Empty,
    Undecryptable,
    Skipped,
    Locked,
    Unreadable,
    Unsupported,
    NotRunning,
    UnreadableBookmarks,
    UnreadableHistory,
    UnreadableTabs,
    UnreadableCookies,
    UnreadableFile,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportWarning {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<ImportKind>,
    pub code: WarningCode,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedFavicon {
    pub page: String,
    pub icon: String,
    pub png: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArcLinkDestinationKind {
    Space,
    LittleArc,
    MostRecent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcLinkDestination {
    pub kind: ArcLinkDestinationKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArcLinkMatch {
    Contains,
    Equals,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcLinkRule {
    pub id: String,
    #[serde(rename = "match")]
    pub match_kind: ArcLinkMatch,
    pub text: String,
    pub to: ArcLinkDestination,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcRouting {
    pub rules: Vec<ArcLinkRule>,
    pub skipped: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<ArcLinkDestination>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcArchivedTab {
    pub url: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_title: Option<String>,
    pub archived_at: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_name: Option<String>,
    pub auto: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcArchive {
    pub tabs: Vec<ArcArchivedTab>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dropped: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clean_up_after_minutes: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedSearchEngine {
    pub name: String,
    pub keyword: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prepopulate_id: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiaSidebar {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_space_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub browser_id: String,
    pub profile_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileSuggestion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bookmarks: Option<BookmarkNode>,
    pub history: Vec<ImportedHistoryEntry>,
    pub history_count: f64,
    pub tabs: Vec<ImportedTab>,
    pub tab_groups: Vec<ImportedTabGroup>,
    pub credentials: Vec<Credential>,
    pub cookie_count: f64,
    pub address_count: f64,
    pub card_count: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault_token: Option<String>,
    pub spaces: Vec<SpaceSuggestion>,
    pub favorites: Vec<ImportedTab>,
    pub favicons: Vec<ImportedFavicon>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_engine: Option<ImportedSearchEngine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub separate_essentials: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dia_sidebar: Option<DiaSidebar>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arc_routing: Option<ArcRouting>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arc_archive: Option<ArcArchive>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arc_cached_icons: Option<f64>,
    pub failed: Vec<ImportKind>,
    pub warnings: Vec<ImportWarning>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafariProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub history: Vec<ImportedHistoryEntry>,
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafariExport {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bookmarks: Option<BookmarkNode>,
    pub credentials: Vec<Credential>,
    pub profiles: Vec<SafariProfile>,
    pub tabs: Vec<ImportedTab>,
    pub cookie_count: f64,
    pub card_count: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault_token: Option<String>,
    pub warnings: Vec<ImportWarning>,
}

/// `cookiesFailed` and `autofillFailed` are the literal `1` in TypeScript; `u8` holds it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WrittenCounts {
    pub cookies: f64,
    pub cookies_rejected: f64,
    pub cookies_existing: f64,
    pub addresses: f64,
    pub cards: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cookies_failed: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autofill_failed: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportedExtensionSkip {
    ManifestV2,
    Unpacked,
    Policy,
    External,
    NotWebStore,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedExtension {
    pub id: String,
    pub name: String,
    pub web_store_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_index: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip: Option<ImportedExtensionSkip>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProgressPhase {
    Start,
    Progress,
    End,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProgress {
    pub kind: ImportKind,
    pub phase: ProgressPhase,
    pub processed: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiaTabsProfile {
    pub id: String,
    pub name: String,
    pub index: f64,
    pub pinned: Vec<ImportedTab>,
    pub tabs: Vec<ImportedTab>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiaTabsResult {
    pub profiles: Vec<DiaTabsProfile>,
    pub window_count: f64,
    pub skipped: f64,
    pub duplicates: f64,
    pub favicons: Vec<ImportedFavicon>,
}

/// A native event (`ImportEvent`, `index.ts:232`). `entries` of a history event is JSON text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ImportEvent {
    #[serde(rename_all = "camelCase")]
    Progress {
        job_id: String,
        #[serde(flatten)]
        progress: ImportProgress,
    },
    #[serde(rename_all = "camelCase")]
    History { job_id: String, entries: String },
}

/// What `writeImported` may write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WriteKind {
    Cookies,
    Autofill,
}

/// What `chooseImportFile` asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportFileKind {
    SafariExport,
    #[serde(rename = "bookmarksHTML")]
    BookmarksHtml,
    #[serde(rename = "passwordsCSV")]
    PasswordsCsv,
}

/// The options that reach the native module (`NativeImportOptions`, `index.ts:204`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeImportOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_limit: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_since: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_size: Option<f64>,
    pub stream_history: bool,
}
