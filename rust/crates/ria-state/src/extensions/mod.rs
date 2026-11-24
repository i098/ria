// Ported from ardia apps/browser/src/state/extensions/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `extensions` (`apps/browser/src/state/extensions`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.22 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum StateInstall {
    #[serde(rename = "error")]
    Error {
        key: Number,
        #[serde(rename = "windowId")]
        window_id: String,
        profile: String,
        source: StateInstallErrorSource,
        error: String,
    },
    #[serde(rename = "ready")]
    Ready {
        key: Number,
        #[serde(rename = "windowId")]
        window_id: String,
        profile: String,
        source: StateInstallReadySource,
        pkg: serde_json::Value,
    },
    #[serde(rename = "installing")]
    Installing {
        key: Number,
        #[serde(rename = "windowId")]
        window_id: String,
        profile: String,
        source: StateInstallInstallingSource,
        pkg: serde_json::Value,
    },
    #[serde(rename = "prompt")]
    Prompt {
        key: Number,
        #[serde(rename = "windowId")]
        window_id: String,
        profile: String,
        source: StateInstallPromptSource,
        prompt: serde_json::Value,
    },
}

impl Default for StateInstall {
    fn default() -> Self {
        Self::Error {
            key: Default::default(),
            window_id: Default::default(),
            profile: Default::default(),
            source: Default::default(),
            error: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateInstallErrorSource {
    #[default]
    #[serde(rename = "webStore")]
    WebStore,
    #[serde(rename = "unpacked")]
    Unpacked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateInstallReadySource {
    #[default]
    #[serde(rename = "webStore")]
    WebStore,
    #[serde(rename = "unpacked")]
    Unpacked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateInstallInstallingSource {
    #[default]
    #[serde(rename = "webStore")]
    WebStore,
    #[serde(rename = "unpacked")]
    Unpacked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateInstallPromptSource {
    #[default]
    #[serde(rename = "webStore")]
    WebStore,
    #[serde(rename = "unpacked")]
    Unpacked,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StatePinDialog {
    #[serde(rename = "windowId")]
    pub window_id: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StatePopup {
    #[serde(rename = "windowId")]
    pub window_id: String,
    #[serde(rename = "pageProfile")]
    pub page_profile: String,
    #[serde(rename = "extensionId")]
    pub extension_id: String,
    pub url: String,
    pub anchor: Anchor,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opened: Option<Number>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Anchor {
    pub x: Number,
    pub y: Number,
    pub width: Number,
    pub height: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SidePanelCard {
    #[serde(rename = "extensionId")]
    pub extension_id: String,
    pub profile: String,
    #[serde(rename = "pageProfile")]
    pub page_profile: String,
    pub url: String,
    pub opened: Number,
    #[serde(rename = "tabId", default, skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    pub owners: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SidePanel {
    #[serde(rename = "extensionId")]
    pub extension_id: String,
    pub profile: String,
    #[serde(rename = "pageProfile")]
    pub page_profile: String,
    pub url: String,
    pub opened: Number,
    #[serde(rename = "tabId", default, skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateToolbarDrag {
    pub id: String,
    pub from: Number,
    pub dx: Number,
    #[serde(rename = "windowId")]
    pub window_id: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ConfigureOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
    #[serde(
        rename = "pinnedIndex",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub pinned_index: Option<Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incognito: Option<bool>,
    #[serde(
        rename = "fileAccess",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub file_access: Option<bool>,
    #[serde(
        rename = "userScripts",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub user_scripts: Option<bool>,
    #[serde(
        rename = "siteAccess",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub site_access: Option<ConfigureOptionsSiteAccess>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ConfigureOptionsSiteAccess {
    #[default]
    #[serde(rename = "allSites")]
    AllSites,
    #[serde(rename = "specificSites")]
    SpecificSites,
    #[serde(rename = "onClick")]
    OnClick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MoveToolbarExtensionVia {
    #[default]
    #[serde(rename = "menu")]
    Menu,
    #[serde(rename = "drag")]
    Drag,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OpenInExtensionProfileArg2 {
    #[serde(rename = "windowId", default, skip_serializing_if = "Option::is_none")]
    pub window_id: Option<String>,
    #[serde(rename = "profileId", default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RunExtensionListChoiceChoice {
    #[default]
    #[serde(rename = "pin")]
    Pin,
    #[serde(rename = "add")]
    Add,
    #[serde(rename = "manage")]
    Manage,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ToolbarDrag {
    pub id: String,
    pub from: Number,
    pub dx: Number,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum InstallRequest {
    #[serde(rename = "error")]
    Error {
        key: Number,
        #[serde(rename = "windowId")]
        window_id: String,
        profile: String,
        source: InstallRequestErrorSource,
        error: String,
    },
    #[serde(rename = "ready")]
    Ready {
        key: Number,
        #[serde(rename = "windowId")]
        window_id: String,
        profile: String,
        source: InstallRequestReadySource,
        pkg: serde_json::Value,
    },
    #[serde(rename = "installing")]
    Installing {
        key: Number,
        #[serde(rename = "windowId")]
        window_id: String,
        profile: String,
        source: InstallRequestInstallingSource,
        pkg: serde_json::Value,
    },
    #[serde(rename = "prompt")]
    Prompt {
        key: Number,
        #[serde(rename = "windowId")]
        window_id: String,
        profile: String,
        source: InstallRequestPromptSource,
        prompt: serde_json::Value,
    },
}

impl Default for InstallRequest {
    fn default() -> Self {
        Self::Error {
            key: Default::default(),
            window_id: Default::default(),
            profile: Default::default(),
            source: Default::default(),
            error: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum InstallRequestErrorSource {
    #[default]
    #[serde(rename = "webStore")]
    WebStore,
    #[serde(rename = "unpacked")]
    Unpacked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum InstallRequestReadySource {
    #[default]
    #[serde(rename = "webStore")]
    WebStore,
    #[serde(rename = "unpacked")]
    Unpacked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum InstallRequestInstallingSource {
    #[default]
    #[serde(rename = "webStore")]
    WebStore,
    #[serde(rename = "unpacked")]
    Unpacked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum InstallRequestPromptSource {
    #[default]
    #[serde(rename = "webStore")]
    WebStore,
    #[serde(rename = "unpacked")]
    Unpacked,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub actions:
        std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>>,
    pub install: Option<StateInstall>,
    pub lists: std::collections::HashMap<String, Vec<serde_json::Value>>,
    #[serde(rename = "pinDialog")]
    pub pin_dialog: Option<StatePinDialog>,
    pub popup: Option<StatePopup>,
    #[serde(rename = "sidePanelCards")]
    pub side_panel_cards: std::collections::HashMap<String, Vec<SidePanelCard>>,
    #[serde(rename = "sidePanelLookups")]
    pub side_panel_lookups: std::collections::HashMap<String, Number>,
    #[serde(rename = "sidePanels")]
    pub side_panels: std::collections::HashMap<String, SidePanel>,
    #[serde(rename = "toolbarDrag")]
    pub toolbar_drag: Option<StateToolbarDrag>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `activateExtension` (an exported function that writes a store).
    #[serde(rename = "activateExtension")]
    ActivateExtension {
        #[serde(rename = "windowId")]
        window_id: String,
        ext: serde_json::Value,
        anchor: Anchor,
        #[serde(default)]
        state: Option<serde_json::Value>,
        #[serde(rename = "tabId", default)]
        tab_id: Option<String>,
    },
    /// `addFromWebStore` (an exported function that writes a store).
    #[serde(rename = "addFromWebStore")]
    AddFromWebStore {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "urlOrId")]
        url_or_id: String,
        #[serde(default)]
        profile: Option<String>,
    },
    /// `cancelInstall` (an exported function that writes a store).
    #[serde(rename = "cancelInstall")]
    CancelInstall {
        #[serde(default)]
        key: Option<Number>,
    },
    /// `closeExtensionPopup` (an exported function that writes a store).
    #[serde(rename = "closeExtensionPopup")]
    CloseExtensionPopup,
    /// `closePanelCard` (an exported function that writes a store).
    #[serde(rename = "closePanelCard")]
    ClosePanelCard {
        #[serde(rename = "windowId")]
        window_id: String,
        card: SidePanel,
    },
    /// `closePinDialog` (an exported function that writes a store).
    #[serde(rename = "closePinDialog")]
    ClosePinDialog,
    /// `closeSidePanel` (an exported function that writes a store).
    #[serde(rename = "closeSidePanel")]
    CloseSidePanel {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "extensionId", default)]
        extension_id: Option<String>,
    },
    /// `closeSidePanelOnTab` (an exported function that writes a store).
    #[serde(rename = "closeSidePanelOnTab")]
    CloseSidePanelOnTab {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "ownerTabId", default)]
        owner_tab_id: Option<String>,
    },
    /// `closeSidePanelsOf` (an exported function that writes a store).
    #[serde(rename = "closeSidePanelsOf")]
    CloseSidePanelsOf {
        #[serde(rename = "extensionId")]
        extension_id: String,
    },
    /// `configure` (an exported function that writes a store).
    #[serde(rename = "configure")]
    Configure {
        profile: String,
        id: String,
        options: ConfigureOptions,
    },
    /// `configureThenRead` (an exported function that writes a store).
    #[serde(rename = "configureThenRead")]
    ConfigureThenRead {
        profile: String,
        id: String,
        options: ConfigureOptions,
    },
    /// `confirmInstall` (an exported function that writes a store).
    #[serde(rename = "confirmInstall")]
    ConfirmInstall {
        #[serde(default)]
        key: Option<Number>,
    },
    /// `dropSidePanelCard` (an exported function that writes a store).
    #[serde(rename = "dropSidePanelCard")]
    DropSidePanelCard {
        #[serde(rename = "windowId")]
        window_id: String,
        card: SidePanel,
    },
    /// `endToolbarDrag` (an exported function that writes a store).
    #[serde(rename = "endToolbarDrag")]
    EndToolbarDrag {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `freshExtensions` (an exported function that writes a store).
    #[serde(rename = "freshExtensions")]
    FreshExtensions { profile: String },
    /// `installDroppedCrx` (an exported function that writes a store).
    #[serde(rename = "installDroppedCrx")]
    InstallDroppedCrx {
        #[serde(rename = "windowId")]
        window_id: String,
        urls: Vec<String>,
    },
    /// `loadUnpacked` (an exported function that writes a store).
    #[serde(rename = "loadUnpacked")]
    LoadUnpacked {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(default)]
        profile: Option<String>,
    },
    /// `moveToolbarExtension` (an exported function that writes a store).
    #[serde(rename = "moveToolbarExtension")]
    MoveToolbarExtension {
        #[serde(rename = "windowId")]
        window_id: String,
        id: String,
        to: Number,
        via: MoveToolbarExtensionVia,
    },
    /// `openActionPopup` (an exported function that writes a store).
    #[serde(rename = "openActionPopup")]
    OpenActionPopup {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "extensionId")]
        extension_id: String,
        anchor: Anchor,
        #[serde(rename = "sourceTabId")]
        source_tab_id: String,
    },
    /// `openExtensionDetails` (an exported function that writes a store).
    #[serde(rename = "openExtensionDetails")]
    OpenExtensionDetails {
        #[serde(rename = "profileId")]
        profile_id: String,
        #[serde(rename = "extensionId")]
        extension_id: String,
    },
    /// `openExtensionFromMenu` (an exported function that writes a store).
    #[serde(rename = "openExtensionFromMenu")]
    OpenExtensionFromMenu {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "extensionId")]
        extension_id: String,
        #[serde(rename = "tabId", default)]
        tab_id: Option<String>,
    },
    /// `openInExtensionProfile` (an exported function that writes a store).
    #[serde(rename = "openInExtensionProfile")]
    OpenInExtensionProfile {
        url: String,
        profile: String,
        #[serde(default)]
        arg2: Option<OpenInExtensionProfileArg2>,
    },
    /// `openManageExtensions` (an exported function that writes a store).
    #[serde(rename = "openManageExtensions")]
    OpenManageExtensions {
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `openOptions` (an exported function that writes a store).
    #[serde(rename = "openOptions")]
    OpenOptions {
        #[serde(rename = "windowId")]
        window_id: String,
        ext: serde_json::Value,
    },
    /// `openPinDialog` (an exported function that writes a store).
    #[serde(rename = "openPinDialog")]
    OpenPinDialog {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `openRequestedTab` (an exported function that writes a store).
    #[serde(rename = "openRequestedTab")]
    OpenRequestedTab { request: serde_json::Value },
    /// `openSidePanel` (an exported function that writes a store).
    #[serde(rename = "openSidePanel")]
    OpenSidePanel {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "extensionId")]
        extension_id: String,
    },
    /// `openWebStore` (an exported function that writes a store).
    #[serde(rename = "openWebStore")]
    OpenWebStore {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `openWebStoreFor` (an exported function that writes a store).
    #[serde(rename = "openWebStoreFor")]
    OpenWebStoreFor {
        profile: String,
        #[serde(rename = "profileId", default)]
        profile_id: Option<String>,
    },
    /// `recheckKeptSidePanels` (an exported function that writes a store).
    #[serde(rename = "recheckKeptSidePanels")]
    RecheckKeptSidePanels {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `refreshExtensions` (an exported function that writes a store).
    #[serde(rename = "refreshExtensions")]
    RefreshExtensions {
        profile: String,
        #[serde(default)]
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `refreshExtensionsAfterChange` (an exported function that writes a store).
    #[serde(rename = "refreshExtensionsAfterChange")]
    RefreshExtensionsAfterChange {
        profile: String,
        #[serde(default)]
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `refreshSidePanelCards` (an exported function that writes a store).
    #[serde(rename = "refreshSidePanelCards")]
    RefreshSidePanelCards {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `reload` (an exported function that writes a store).
    #[serde(rename = "reload")]
    Reload { profile: String, id: String },
    /// `removeExtension` (an exported function that writes a store). The callback parameters `mayRemove`, `ask` are not in the JSON.
    #[serde(rename = "removeExtension")]
    RemoveExtension {
        profile: String,
        ext: serde_json::Value,
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `runExtensionListChoice` (an exported function that writes a store).
    #[serde(rename = "runExtensionListChoice")]
    RunExtensionListChoice {
        choice: Option<RunExtensionListChoiceChoice>,
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `setEnabled` (an exported function that writes a store).
    #[serde(rename = "setEnabled")]
    SetEnabled {
        profile: String,
        id: String,
        enabled: bool,
    },
    /// `setPinned` (an exported function that writes a store).
    #[serde(rename = "setPinned")]
    SetPinned {
        profile: String,
        id: String,
        pinned: bool,
    },
    /// `settleSidePanelTabs` (an exported function that writes a store).
    #[serde(rename = "settleSidePanelTabs")]
    SettleSidePanelTabs {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `setToolbarDrag` (an exported function that writes a store).
    #[serde(rename = "setToolbarDrag")]
    SetToolbarDrag {
        #[serde(rename = "windowId")]
        window_id: String,
        drag: Option<ToolbarDrag>,
    },
    /// `showExtensionMenu` (an exported function that writes a store).
    #[serde(rename = "showExtensionMenu")]
    ShowExtensionMenu {
        #[serde(rename = "windowId")]
        window_id: String,
        ext: serde_json::Value,
    },
    /// `showInstallPrompt` (an exported function that writes a store).
    #[serde(rename = "showInstallPrompt")]
    ShowInstallPrompt {
        #[serde(rename = "windowId")]
        window_id: String,
        prompt: serde_json::Value,
    },
    /// `showInstallSheet` (an exported function that writes a store).
    #[serde(rename = "showInstallSheet")]
    ShowInstallSheet { next: InstallRequest },
    /// `startActionStates` (an exported function that writes a store).
    #[serde(rename = "startActionStates")]
    StartActionStates,
    /// `startExtensionsBridge` (an exported function that writes a store).
    #[serde(rename = "startExtensionsBridge")]
    StartExtensionsBridge,
    /// `startExtensionSearchEngines` (an exported function that writes a store).
    #[serde(rename = "startExtensionSearchEngines")]
    StartExtensionSearchEngines,
    /// `startWebStoreIntegration` (an exported function that writes a store).
    #[serde(rename = "startWebStoreIntegration")]
    StartWebStoreIntegration,
    /// `syncSidePanel` (an exported function that writes a store).
    #[serde(rename = "syncSidePanel")]
    SyncSidePanel {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `toggleSidePanel` (an exported function that writes a store).
    #[serde(rename = "toggleSidePanel")]
    ToggleSidePanel {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "extensionId")]
        extension_id: String,
    },
    /// `utilitySheetHost` (an exported function that writes a store).
    #[serde(rename = "utilitySheetHost")]
    UtilitySheetHost {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `watchExtensionInstallFailures` (an exported function that writes a store).
    #[serde(rename = "watchExtensionInstallFailures")]
    WatchExtensionInstallFailures,
    /// `watchSidePanelCards` (an exported function that writes a store).
    #[serde(rename = "watchSidePanelCards")]
    WatchSidePanelCards,
}

/// Applies `action` to `app`. Slice 2.22 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.22")
}
