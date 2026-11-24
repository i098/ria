// Ported from ardia apps/browser/src/state/import/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `import` (`apps/browser/src/state/import`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.40 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "name")]
pub enum ImportStep {
    #[serde(rename = "loading")]
    Loading,
    #[serde(rename = "choose")]
    Choose { error: Option<String> },
    #[serde(rename = "profiles")]
    Profiles,
    #[serde(rename = "access")]
    Access,
    #[serde(rename = "safari")]
    Safari { error: Option<String> },
    #[serde(rename = "unlock")]
    Unlock {
        unlocking: bool,
        error: Option<String>,
    },
    #[serde(rename = "progress")]
    Progress {
        status: std::collections::HashMap<String, ImportStepProgressStatusValue>,
        preparing: Number,
    },
    #[serde(rename = "done")]
    Done {
        counts: ImportCounts,
        failures: Vec<ImportFailure>,
        notes: Vec<String>,
        extensions: FoundExtensions,
        #[serde(
            rename = "searchEngine",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        search_engine: Option<EngineNotice>,
    },
}

impl Default for ImportStep {
    fn default() -> Self {
        Self::Loading
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ImportStepProgressStatusValue {
    #[default]
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "active")]
    Active,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ImportCounts {
    pub bookmarks: Number,
    pub history: Number,
    pub tabs: Number,
    pub passwords: Number,
    pub cookies: Number,
    pub addresses: Number,
    pub cards: Number,
    pub rules: Number,
    pub archived: Number,
    pub settings: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ImportFailure {
    pub kind: serde_json::Value,
    pub profiles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FoundExtensions {
    #[serde(rename = "profileId")]
    pub profile_id: String,
    pub list: Vec<serde_json::Value>,
    pub opened: bool,
    pub listing: Option<ExtensionListing>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ExtensionListing {
    pub targets: Vec<ExtensionTarget>,
    #[serde(rename = "leftOut")]
    pub left_out: Vec<LeftOutExtension>,
    #[serde(rename = "storeOpened")]
    pub store_opened: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ExtensionTarget {
    #[serde(rename = "profileId")]
    pub profile_id: String,
    #[serde(rename = "profileName")]
    pub profile_name: String,
    pub extensions: Vec<ListedExtension>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ListedExtension {
    pub id: String,
    pub name: String,
    #[serde(rename = "webStoreUrl")]
    pub web_store_url: String,
    #[serde(rename = "offInSource")]
    pub off_in_source: bool,
    #[serde(rename = "pinnedInSource")]
    pub pinned_in_source: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LeftOutExtension {
    pub name: String,
    pub reason: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct EngineNotice {
    pub name: String,
    pub offered: bool,
    pub engine: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AdoptImportedEngineSource {
    pub id: serde_json::Value,
    pub family: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AdoptImportedEngineProfilesItem {
    pub profile: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AdoptSpaceIdentitySpace {
    pub name: serde_json::Value,
    pub color: serde_json::Value,
    pub emoji: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BackToName {
    #[default]
    #[serde(rename = "profiles")]
    Profiles,
    #[serde(rename = "choose")]
    Choose,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ImportResults {
    pub counts: ImportCounts,
    pub failures: Vec<ImportFailure>,
    pub notes: Vec<String>,
    pub extensions: FoundExtensions,
    #[serde(
        rename = "searchEngine",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub search_engine: Option<EngineNotice>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ArcHomes {
    pub spaces: std::collections::HashMap<String, String>,
    pub fallback: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ImportDiaProfileWhat {
    pub pinned: bool,
    pub open: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ImportHistoryEntriesItem {
    pub url: String,
    pub title: String,
    pub visits: Number,
    #[serde(rename = "lastVisit")]
    pub last_visit: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct IncomingTab {
    pub url: serde_json::Value,
    pub pinned: serde_json::Value,
    pub title: serde_json::Value,
    #[serde(rename = "customTitle")]
    pub custom_title: serde_json::Value,
    #[serde(rename = "splitId")]
    pub split_id: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ImportTabsPinnedAgainst {
    #[default]
    #[serde(rename = "anyPin")]
    AnyPin,
    #[serde(rename = "tiles")]
    Tiles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ImportTabsWithSplitsPinnedAgainst {
    #[default]
    #[serde(rename = "anyPin")]
    AnyPin,
    #[serde(rename = "tiles")]
    Tiles,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PatchImportPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browsers: Option<Vec<serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<serde_json::Value>,
    #[serde(
        rename = "sourceProfiles",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub source_profiles: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spaces: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kinds: Option<Vec<serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(
        rename = "safariCookies",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub safari_cookies: Option<bool>,
    #[serde(
        rename = "fullDiskAccess",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub full_disk_access: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<PatchImportPatchStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "name")]
pub enum PatchImportPatchStep {
    #[serde(rename = "loading")]
    Loading,
    #[serde(rename = "choose")]
    Choose { error: Option<String> },
    #[serde(rename = "profiles")]
    Profiles,
    #[serde(rename = "access")]
    Access,
    #[serde(rename = "safari")]
    Safari { error: Option<String> },
    #[serde(rename = "unlock")]
    Unlock {
        unlocking: bool,
        error: Option<String>,
    },
    #[serde(rename = "progress")]
    Progress {
        status: std::collections::HashMap<String, PatchImportPatchStepProgressStatusValue>,
        preparing: Number,
    },
    #[serde(rename = "done")]
    Done {
        counts: ImportCounts,
        failures: Vec<ImportFailure>,
        notes: Vec<String>,
        extensions: FoundExtensions,
        #[serde(
            rename = "searchEngine",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        search_engine: Option<EngineNotice>,
    },
}

impl Default for PatchImportPatchStep {
    fn default() -> Self {
        Self::Loading
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PatchImportPatchStepProgressStatusValue {
    #[default]
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "active")]
    Active,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StartProgressStatusValue {
    #[default]
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "active")]
    Active,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub browsers: Vec<serde_json::Value>,
    #[serde(rename = "fullDiskAccess")]
    pub full_disk_access: bool,
    pub kinds: Vec<serde_json::Value>,
    #[serde(rename = "safariCookies")]
    pub safari_cookies: bool,
    pub source: serde_json::Value,
    #[serde(rename = "sourceProfiles")]
    pub source_profiles: Vec<String>,
    pub spaces: Vec<String>,
    pub step: ImportStep,
    pub target: String,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `adoptImportedEngine` (an exported function that writes a store).
    #[serde(rename = "adoptImportedEngine")]
    AdoptImportedEngine {
        source: AdoptImportedEngineSource,
        profiles: Vec<AdoptImportedEngineProfilesItem>,
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `adoptSpaceIdentity` (an exported function that writes a store).
    #[serde(rename = "adoptSpaceIdentity")]
    AdoptSpaceIdentity {
        #[serde(rename = "profileId")]
        profile_id: String,
        space: AdoptSpaceIdentitySpace,
    },
    /// `applyResult` (an exported function that writes a store). The callback parameters `onFailed` are not in the JSON.
    #[serde(rename = "applyResult")]
    ApplyResult {
        #[serde(rename = "profileId")]
        profile_id: String,
        result: serde_json::Value,
        #[serde(rename = "browserName")]
        browser_name: String,
        requested: Vec<serde_json::Value>,
        #[serde(default)]
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `applySafari` (an exported function that writes a store). The callback parameters `onFailed` are not in the JSON.
    #[serde(rename = "applySafari")]
    ApplySafari {
        #[serde(rename = "profileId")]
        profile_id: String,
        data: serde_json::Value,
        #[serde(default)]
        cookies: Option<bool>,
        #[serde(default)]
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `backTo` (an exported function that writes a store).
    #[serde(rename = "backTo")]
    BackTo { name: BackToName },
    /// `cancelImport` (an exported function that writes a store).
    #[serde(rename = "cancelImport")]
    CancelImport,
    /// `chooseSource` (an exported function that writes a store).
    #[serde(rename = "chooseSource")]
    ChooseSource { b: serde_json::Value },
    /// `closeImport` (an exported function that writes a store).
    #[serde(rename = "closeImport")]
    CloseImport,
    /// `continueImport` (an exported function that writes a store).
    #[serde(rename = "continueImport")]
    ContinueImport,
    /// `finishImport` (an exported function that writes a store).
    #[serde(rename = "finishImport")]
    FinishImport {
        signal: crate::automation::DecideSignal,
        results: ImportResults,
    },
    /// `importArcArchive` (an exported function that writes a store).
    #[serde(rename = "importArcArchive")]
    ImportArcArchive {
        archive: serde_json::Value,
        homes: ArcHomes,
        #[serde(default)]
        now: Option<Number>,
    },
    /// `importArcRouting` (an exported function that writes a store).
    #[serde(rename = "importArcRouting")]
    ImportArcRouting {
        routing: serde_json::Value,
        homes: ArcHomes,
    },
    /// `importArcSpace` (an exported function that writes a store).
    #[serde(rename = "importArcSpace")]
    ImportArcSpace {
        #[serde(rename = "profileId")]
        profile_id: String,
        space: serde_json::Value,
        #[serde(default)]
        favicons: Option<Vec<serde_json::Value>>,
        #[serde(default)]
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `importBookmarks` (an exported function that writes a store).
    #[serde(rename = "importBookmarks")]
    ImportBookmarks {
        #[serde(rename = "profileId")]
        profile_id: String,
        root: serde_json::Value,
        #[serde(rename = "browserName")]
        browser_name: String,
    },
    /// `importBookmarksHtml` (an exported function that writes a store).
    #[serde(rename = "importBookmarksHtml")]
    ImportBookmarksHtml,
    /// `importDiaProfile` (an exported function that writes a store).
    #[serde(rename = "importDiaProfile")]
    ImportDiaProfile {
        #[serde(rename = "profileId")]
        profile_id: String,
        profile: serde_json::Value,
        what: ImportDiaProfileWhat,
        #[serde(default)]
        favicons: Option<Vec<serde_json::Value>>,
    },
    /// `importDiaSpace` (an exported function that writes a store).
    #[serde(rename = "importDiaSpace")]
    ImportDiaSpace {
        #[serde(rename = "profileId")]
        profile_id: String,
        space: serde_json::Value,
        #[serde(default)]
        favicons: Option<Vec<serde_json::Value>>,
        #[serde(default)]
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `importFavicons` (an exported function that writes a store).
    #[serde(rename = "importFavicons")]
    ImportFavicons {
        #[serde(rename = "profileId")]
        profile_id: String,
        favicons: Vec<serde_json::Value>,
        urls: Vec<String>,
    },
    /// `importFavorites` (an exported function that writes a store).
    #[serde(rename = "importFavorites")]
    ImportFavorites {
        #[serde(rename = "profileId")]
        profile_id: String,
        tabs: Vec<serde_json::Value>,
        #[serde(default)]
        favicons: Option<Vec<serde_json::Value>>,
    },
    /// `importHistory` (an exported function that writes a store). The callback parameters `onFailed` are not in the JSON.
    #[serde(rename = "importHistory")]
    ImportHistory {
        #[serde(rename = "profileId")]
        profile_id: String,
        entries: Vec<ImportHistoryEntriesItem>,
        #[serde(default)]
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `importSafariDirect` (an exported function that writes a store).
    #[serde(rename = "importSafariDirect")]
    ImportSafariDirect,
    /// `importSafariExport` (an exported function that writes a store).
    #[serde(rename = "importSafariExport")]
    ImportSafariExport { path: String },
    /// `importTabs` (an exported function that writes a store).
    #[serde(rename = "importTabs")]
    ImportTabs {
        #[serde(rename = "profileId")]
        profile_id: String,
        tabs: Vec<IncomingTab>,
        group: Option<String>,
        #[serde(rename = "pinnedAgainst", default)]
        pinned_against: Option<ImportTabsPinnedAgainst>,
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `importTabsWithSplits` (an exported function that writes a store).
    #[serde(rename = "importTabsWithSplits")]
    ImportTabsWithSplits {
        #[serde(rename = "profileId")]
        profile_id: String,
        tabs: Vec<IncomingTab>,
        group: Option<String>,
        #[serde(rename = "pinnedAgainst", default)]
        pinned_against: Option<ImportTabsWithSplitsPinnedAgainst>,
        #[serde(default)]
        into: Option<String>,
    },
    /// `openExtensionStorePage` (an exported function that writes a store).
    #[serde(rename = "openExtensionStorePage")]
    OpenExtensionStorePage {
        #[serde(rename = "profileId")]
        profile_id: String,
        url: String,
    },
    /// `openFoundExtensions` (an exported function that writes a store).
    #[serde(rename = "openFoundExtensions")]
    OpenFoundExtensions,
    /// `openInWebStore` (an exported function that writes a store).
    #[serde(rename = "openInWebStore")]
    OpenInWebStore {
        #[serde(rename = "profileId")]
        profile_id: String,
        urls: Vec<String>,
    },
    /// `openListedExtensionsInStore` (an exported function that writes a store).
    #[serde(rename = "openListedExtensionsInStore")]
    OpenListedExtensionsInStore,
    /// `patchImport` (an exported function that writes a store).
    #[serde(rename = "patchImport")]
    PatchImport { patch: PatchImportPatch },
    /// `recheckAccess` (an exported function that writes a store).
    #[serde(rename = "recheckAccess")]
    RecheckAccess,
    /// `runImport` (an exported function that writes a store).
    #[serde(rename = "runImport")]
    RunImport {
        wanted: Vec<serde_json::Value>,
        #[serde(rename = "withoutKey", default)]
        without_key: Option<bool>,
    },
    /// `setImportKinds` (an exported function that writes a store).
    #[serde(rename = "setImportKinds")]
    SetImportKinds { kinds: Vec<serde_json::Value> },
    /// `setImportTarget` (an exported function that writes a store).
    #[serde(rename = "setImportTarget")]
    SetImportTarget { target: String },
    /// `setSafariCookies` (an exported function that writes a store).
    #[serde(rename = "setSafariCookies")]
    SetSafariCookies {
        #[serde(rename = "safariCookies")]
        safari_cookies: bool,
    },
    /// `skipEncryptedData` (an exported function that writes a store).
    #[serde(rename = "skipEncryptedData")]
    SkipEncryptedData,
    /// `startImport` (an exported function that writes a store).
    #[serde(rename = "startImport")]
    StartImport {
        #[serde(rename = "delayBeforeDone")]
        delay_before_done: Number,
    },
    /// `startProgress` (an exported function that writes a store).
    #[serde(rename = "startProgress")]
    StartProgress {
        status: std::collections::HashMap<String, StartProgressStatusValue>,
        preparing: Number,
    },
    /// `takeOfferedEngine` (an exported function that writes a store).
    #[serde(rename = "takeOfferedEngine")]
    TakeOfferedEngine,
    /// `toggleImportKind` (an exported function that writes a store).
    #[serde(rename = "toggleImportKind")]
    ToggleImportKind { kind: serde_json::Value, on: bool },
    /// `toggleImportSpace` (an exported function that writes a store).
    #[serde(rename = "toggleImportSpace")]
    ToggleImportSpace { id: String, on: bool },
    /// `toggleSourceProfile` (an exported function that writes a store).
    #[serde(rename = "toggleSourceProfile")]
    ToggleSourceProfile { id: String, on: bool },
    /// `unlockAndImport` (an exported function that writes a store).
    #[serde(rename = "unlockAndImport")]
    UnlockAndImport,
    /// `updateStatus` (an exported function that writes a store). The callback parameters `change` are not in the JSON.
    #[serde(rename = "updateStatus")]
    UpdateStatus {
        signal: crate::automation::DecideSignal,
    },
    /// `windowFor` (an exported function that writes a store).
    #[serde(rename = "windowFor")]
    WindowFor {
        #[serde(rename = "profileId")]
        profile_id: String,
    },
}

/// Applies `action` to `app`. Slice 2.40 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.40")
}
