// Ported from ardia apps/browser/src/state/automation/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `automation` (`apps/browser/src/state/automation`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.29 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Approved {
    pub key: String,
    pub kind: ApprovalKind,
    pub name: String,
    pub summary: String,
    pub at: Number,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ApprovalKind {
    #[default]
    #[serde(rename = "rewrite")]
    Rewrite,
    #[serde(rename = "route")]
    Route,
    #[serde(rename = "folder")]
    Folder,
    #[serde(rename = "omnibox")]
    Omnibox,
    #[serde(rename = "program")]
    Program,
    #[serde(rename = "omniboxProvider")]
    OmniboxProvider,
    #[serde(rename = "launchAction")]
    LaunchAction,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HookSpec {
    pub name: String,
    pub revision: String,
    pub on: Vec<String>,
    pub patterns: Vec<String>,
    pub specs: Vec<MatchSpec>,
    pub decide: Option<HookSpecDecide>,
    pub rules: Option<DecideRules>,
    pub program: Option<Vec<String>>,
    pub command: Option<HookSpecCommand>,
    pub omnibox: Option<OmniboxSpec>,
    pub from: Vec<String>,
    pub kinds: Vec<String>,
    #[serde(rename = "fullUrls")]
    pub full_urls: bool,
    pub scheduled: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MatchSpec {
    pub all: bool,
    pub schemes: Vec<String>,
    pub host: String,
    pub sub: bool,
    pub port: String,
    pub file: bool,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum HookSpecDecide {
    #[default]
    #[serde(rename = "tab.opening")]
    TabOpening,
    #[serde(rename = "link.opening")]
    LinkOpening,
    #[serde(rename = "download.deciding")]
    DownloadDeciding,
    #[serde(rename = "permission.requested")]
    PermissionRequested,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DecideRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rewrite: Option<Vec<Rewrite>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancel: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<Route>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission: Option<DecideRulesPermission>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Rewrite {
    pub pattern: String,
    pub spec: MatchSpec,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Route {
    pub profile: Option<String>,
    pub open: Option<RouteOpen>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RouteOpen {
    #[default]
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "window")]
    Window,
    #[serde(rename = "littleArdia")]
    LittleArdia,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DecideRulesPermission {
    #[default]
    #[serde(rename = "allow")]
    Allow,
    #[serde(rename = "ask")]
    Ask,
    #[serde(rename = "deny")]
    Deny,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HookSpecCommand {
    pub id: String,
    pub arg: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OmniboxSpec {
    pub title: String,
    pub keyword: Option<String>,
    pub global: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateOffered {
    #[serde(
        rename = "hooks.json",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub hooks_json: Option<Vec<ApprovalRule>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub omnibox: Option<Vec<ApprovalRule>>,
    #[serde(
        rename = "config.json",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub config_json: Option<Vec<ApprovalRule>>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApprovalRule {
    pub key: String,
    pub kind: ApprovalKind,
    pub name: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AutomationRuleRow {
    pub key: String,
    pub kind: ApprovalKind,
    pub name: String,
    pub summary: String,
    #[serde(
        rename = "providerKey",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub provider_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub groups: std::collections::HashMap<String, crate::browser::TabGroup>,
    pub tabs: std::collections::HashMap<String, crate::browser::Tab>,
    pub profiles: std::collections::HashMap<String, crate::browser::Profile>,
    pub windows: std::collections::HashMap<String, crate::browser::BrowserWindow>,
    #[serde(rename = "windowOrder")]
    pub window_order: Vec<String>,
    pub ui: SnapshotUi,
    pub peeks: std::collections::HashMap<String, crate::peek::Peek>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SnapshotUi {
    #[serde(rename = "focusOrder")]
    pub focus_order: Vec<String>,
    #[serde(rename = "appDark")]
    pub app_dark: bool,
    #[serde(rename = "lastProfileId")]
    pub last_profile_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OpResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tabs: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcomes: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Op {
    pub op: OpOp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum OpOp {
    #[default]
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "tab.move")]
    TabMove,
    #[serde(rename = "tab.pin")]
    TabPin,
    #[serde(rename = "tab.unpin")]
    TabUnpin,
    #[serde(rename = "tab.activate")]
    TabActivate,
    #[serde(rename = "tab.mute")]
    TabMute,
    #[serde(rename = "tab.unmute")]
    TabUnmute,
    #[serde(rename = "tab.sleep")]
    TabSleep,
    #[serde(rename = "tab.wake")]
    TabWake,
    #[serde(rename = "tab.close")]
    TabClose,
    #[serde(rename = "tab.rename")]
    TabRename,
    #[serde(rename = "tab.icon")]
    TabIcon,
    #[serde(rename = "group.create")]
    GroupCreate,
    #[serde(rename = "group.update")]
    GroupUpdate,
    #[serde(rename = "group.move")]
    GroupMove,
    #[serde(rename = "group.ungroup")]
    GroupUngroup,
    #[serde(rename = "group.close")]
    GroupClose,
    #[serde(rename = "group.delete")]
    GroupDelete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DecideEvent {
    #[default]
    #[serde(rename = "tab.opening")]
    TabOpening,
    #[serde(rename = "link.opening")]
    LinkOpening,
    #[serde(rename = "download.deciding")]
    DownloadDeciding,
    #[serde(rename = "permission.requested")]
    PermissionRequested,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DecidePlan {}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DecideSignal {
    pub aborted: bool,
    pub onabort: Option<serde_json::Value>,
    pub reason: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FinishedDownload {
    pub url: String,
    pub filename: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub size: Number,
    pub path: String,
    pub profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DecidePermissionRequest {
    #[serde(rename = "tabId")]
    pub tab_id: Option<String>,
    pub origin: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ApprovalSource {
    #[default]
    #[serde(rename = "hooks.json")]
    HooksJson,
    #[serde(rename = "omnibox")]
    Omnibox,
    #[serde(rename = "config.json")]
    ConfigJson,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Space {
    #[serde(rename = "windowId")]
    pub window_id: String,
    #[serde(rename = "profileId")]
    pub profile_id: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FeedConsumer {}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ShownPageT {
    pub url: String,
    pub title: String,
    #[serde(
        rename = "customTitle",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DownloadMover {}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StartStoreEventsNeeds {
    pub events: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Snapshot2 {
    pub groups: std::collections::HashMap<String, crate::browser::TabGroup>,
    pub tabs: std::collections::HashMap<String, crate::browser::Tab>,
    pub profiles: std::collections::HashMap<String, crate::browser::Profile>,
    #[serde(rename = "profileOrder")]
    pub profile_order: Vec<String>,
    pub windows: std::collections::HashMap<String, crate::browser::BrowserWindow>,
    pub splits: std::collections::HashMap<String, crate::browser::SplitView>,
    #[serde(rename = "windowOrder")]
    pub window_order: Vec<String>,
    pub ui: SnapshotUi,
    pub peeks: std::collections::HashMap<String, crate::peek::Peek>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FeedNeeds {
    pub events: Vec<FeedNeedsEventsItem>,
    #[serde(rename = "tabIdle")]
    pub tab_idle: Vec<Number>,
    #[serde(rename = "userIdle")]
    pub user_idle: Vec<Number>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum FeedNeedsEventsItem {
    #[default]
    #[serde(rename = "window.opened")]
    WindowOpened,
    #[serde(rename = "window.closed")]
    WindowClosed,
    #[serde(rename = "window.focused")]
    WindowFocused,
    #[serde(rename = "tab.opened")]
    TabOpened,
    #[serde(rename = "tab.closed")]
    TabClosed,
    #[serde(rename = "tab.activated")]
    TabActivated,
    #[serde(rename = "tab.navigated")]
    TabNavigated,
    #[serde(rename = "settings.changed")]
    SettingsChanged,
    #[serde(rename = "app.launched")]
    AppLaunched,
    #[serde(rename = "app.quitting")]
    AppQuitting,
    #[serde(rename = "app.activated")]
    AppActivated,
    #[serde(rename = "app.deactivated")]
    AppDeactivated,
    #[serde(rename = "tab.idle")]
    TabIdle,
    #[serde(rename = "tab.resumed")]
    TabResumed,
    #[serde(rename = "media.started")]
    MediaStarted,
    #[serde(rename = "media.stopped")]
    MediaStopped,
    #[serde(rename = "tab.crashed")]
    TabCrashed,
    #[serde(rename = "notification.shown")]
    NotificationShown,
    #[serde(rename = "peek.opened")]
    PeekOpened,
    #[serde(rename = "peek.closed")]
    PeekClosed,
    #[serde(rename = "littleArdia.opened")]
    LittleArdiaOpened,
    #[serde(rename = "split.created")]
    SplitCreated,
    #[serde(rename = "split.closed")]
    SplitClosed,
    #[serde(rename = "pin.added")]
    PinAdded,
    #[serde(rename = "pin.removed")]
    PinRemoved,
    #[serde(rename = "tab.moved")]
    TabMoved,
    #[serde(rename = "profile.switched")]
    ProfileSwitched,
    #[serde(rename = "system.sleep")]
    SystemSleep,
    #[serde(rename = "system.wake")]
    SystemWake,
    #[serde(rename = "user.idle")]
    UserIdle,
    #[serde(rename = "user.active")]
    UserActive,
    #[serde(rename = "page.error")]
    PageError,
    #[serde(rename = "bookmarks.changed")]
    BookmarksChanged,
    #[serde(rename = "history.removed")]
    HistoryRemoved,
    #[serde(rename = "download.changed")]
    DownloadChanged,
    #[serde(rename = "profile.changed")]
    ProfileChanged,
    #[serde(rename = "permission.prompted")]
    PermissionPrompted,
    #[serde(rename = "password.prompted")]
    PasswordPrompted,
    #[serde(rename = "menu.opened")]
    MenuOpened,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WorkspaceFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub approved: std::collections::HashMap<String, Approved>,
    pub dismissed: Vec<String>,
    pub generation: Number,
    pub hooks: Vec<HookSpec>,
    pub loaded: bool,
    pub offered: StateOffered,
    pub problems: Vec<String>,
    pub stamp: Option<String>,
    pub status: serde_json::Value,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `activeHook` (an exported function that writes a store).
    #[serde(rename = "activeHook")]
    ActiveHook { hook: HookSpec },
    /// `applyMethod` (an exported function that writes a store). The callback parameters `stillOn`, `urlRule` are not in the JSON.
    #[serde(rename = "applyMethod")]
    ApplyMethod {
        params: std::collections::HashMap<String, serde_json::Value>,
        #[serde(default)]
        connection: Option<Number>,
    },
    /// `approveAutomationRule` (an exported function that writes a store).
    #[serde(rename = "approveAutomationRule")]
    ApproveAutomationRule { row: AutomationRuleRow },
    /// `approvedDownloadFolders` (an exported function that writes a store).
    #[serde(rename = "approvedDownloadFolders")]
    ApprovedDownloadFolders,
    /// `approveRule` (an exported function that writes a store).
    #[serde(rename = "approveRule")]
    ApproveRule {
        key: String,
        #[serde(default)]
        known: Option<ApprovalRule>,
    },
    /// `automationWindows` (an exported function that writes a store).
    #[serde(rename = "automationWindows")]
    AutomationWindows { s: Snapshot },
    /// `closeTabs` (an exported function that writes a store).
    #[serde(rename = "closeTabs")]
    CloseTabs { results: Vec<OpResult>, op: Op },
    /// `decide` (an exported function that writes a store).
    #[serde(rename = "decide")]
    Decide {
        event: DecideEvent,
        data: std::collections::HashMap<String, serde_json::Value>,
        plan: DecidePlan,
        #[serde(default)]
        signal: Option<DecideSignal>,
    },
    /// `decideDownload` (an exported function that writes a store).
    #[serde(rename = "decideDownload")]
    DecideDownload {
        download: FinishedDownload,
        #[serde(rename = "approvedFolders")]
        approved_folders: Vec<String>,
        #[serde(default)]
        signal: Option<DecideSignal>,
    },
    /// `decideLink` (an exported function that writes a store).
    #[serde(rename = "decideLink")]
    DecideLink {
        url: String,
        source: Option<crate::air_traffic::NoteLinkSourceSource>,
        #[serde(default)]
        signal: Option<DecideSignal>,
    },
    /// `decideOpening` (an exported function that writes a store).
    #[serde(rename = "decideOpening")]
    DecideOpening {
        url: String,
        context: crate::tabs::OpeningContext,
        #[serde(default)]
        signal: Option<DecideSignal>,
    },
    /// `decidePermission` (an exported function that writes a store).
    #[serde(rename = "decidePermission")]
    DecidePermission {
        request: DecidePermissionRequest,
        #[serde(default)]
        signal: Option<DecideSignal>,
    },
    /// `diffMore` (an exported function that writes a store). The callback parameters `send` are not in the JSON.
    #[serde(rename = "diffMore")]
    DiffMore {
        s: crate::browser::BrowserState,
        prev: crate::browser::BrowserState,
        wants: Vec<String>,
    },
    /// `diffStates` (an exported function that writes a store). The callback parameters `send` are not in the JSON.
    #[serde(rename = "diffStates")]
    DiffStates {
        s: crate::browser::BrowserState,
        prev: crate::browser::BrowserState,
    },
    /// `guardedBySocketOrigin` (an exported function that writes a store). The callback parameters `run`, `refused` are not in the JSON.
    #[serde(rename = "guardedBySocketOrigin")]
    GuardedBySocketOrigin { mutates: bool },
    /// `handleControlRequest` (an exported function that writes a store).
    #[serde(rename = "handleControlRequest")]
    HandleControlRequest { arg0: serde_json::Value },
    /// `isApproved` (an exported function that writes a store).
    #[serde(rename = "isApproved")]
    IsApproved { key: String },
    /// `isHookActive` (an exported function that writes a store).
    #[serde(rename = "isHookActive")]
    IsHookActive { hook: HookSpec },
    /// `mute` (an exported function that writes a store).
    #[serde(rename = "mute")]
    Mute {
        results: Vec<OpResult>,
        op: Op,
        muted: bool,
    },
    /// `noteCommit` (an exported function that writes a store).
    #[serde(rename = "noteCommit")]
    NoteCommit {
        #[serde(rename = "tabId")]
        tab_id: String,
        url: String,
        at: Number,
        #[serde(rename = "httpStatus", default)]
        http_status: Option<Number>,
    },
    /// `noteDownloadEvent` (an exported function that writes a store).
    #[serde(rename = "noteDownloadEvent")]
    NoteDownloadEvent { d: serde_json::Value },
    /// `offerLaunchActions` (an exported function that writes a store).
    #[serde(rename = "offerLaunchActions")]
    OfferLaunchActions { entries: Vec<String> },
    /// `offerRules` (an exported function that writes a store).
    #[serde(rename = "offerRules")]
    OfferRules {
        source: ApprovalSource,
        rules: Vec<ApprovalRule>,
    },
    /// `offscreenMethod` (an exported function that writes a store). The callback parameters `serving` are not in the JSON.
    #[serde(rename = "offscreenMethod")]
    OffscreenMethod {
        params: std::collections::HashMap<String, serde_json::Value>,
    },
    /// `openTabs` (an exported function that writes a store).
    #[serde(rename = "openTabs")]
    OpenTabs {
        urls: Vec<String>,
        r#where: Space,
        created: Vec<String>,
    },
    /// `refreshFeed` (an exported function that writes a store).
    #[serde(rename = "refreshFeed")]
    RefreshFeed,
    /// `reloadHooks` (an exported function that writes a store).
    #[serde(rename = "reloadHooks")]
    ReloadHooks,
    /// `removeAutomationRule` (an exported function that writes a store).
    #[serde(rename = "removeAutomationRule")]
    RemoveAutomationRule { row: AutomationRuleRow },
    /// `removeRule` (an exported function that writes a store).
    #[serde(rename = "removeRule")]
    RemoveRule { key: String },
    /// `rename` (an exported function that writes a store).
    #[serde(rename = "rename")]
    Rename { results: Vec<OpResult>, op: Op },
    /// `resetApprovalsForTests` (an exported function that writes a store).
    #[serde(rename = "resetApprovalsForTests")]
    ResetApprovalsForTests,
    /// `runLaunchActions` (an exported function that writes a store).
    #[serde(rename = "runLaunchActions")]
    RunLaunchActions,
    /// `runMethod` (an exported function that writes a store). The callback parameters `stillOn` are not in the JSON.
    #[serde(rename = "runMethod")]
    RunMethod {
        params: std::collections::HashMap<String, serde_json::Value>,
        #[serde(default)]
        background: Option<serde_json::Value>,
    },
    /// `runOp` (an exported function that writes a store). The callback parameters `stillOn` are not in the JSON.
    #[serde(rename = "runOp")]
    RunOp {
        results: Vec<OpResult>,
        op: Op,
        #[serde(default)]
        owner: Option<Number>,
    },
    /// `setFeedConsumer` (an exported function that writes a store).
    #[serde(rename = "setFeedConsumer")]
    SetFeedConsumer {
        key: String,
        consumer: Option<FeedConsumer>,
    },
    /// `setHooksForTests` (an exported function that writes a store).
    #[serde(rename = "setHooksForTests")]
    SetHooksForTests { hooks: Vec<HookSpec> },
    /// `settleFinishedDownload` (an exported function that writes a store).
    #[serde(rename = "settleFinishedDownload")]
    SettleFinishedDownload { d: serde_json::Value },
    /// `shownPage` (an exported function that writes a store).
    #[serde(rename = "shownPage")]
    ShownPage { t: ShownPageT },
    /// `sleep` (an exported function that writes a store). The callback parameters `stillOn` are not in the JSON.
    #[serde(rename = "sleep")]
    Sleep {
        results: Vec<OpResult>,
        op: Op,
        #[serde(default)]
        owner: Option<Number>,
    },
    /// `startAutomation` (an exported function that writes a store).
    #[serde(rename = "startAutomation")]
    StartAutomation,
    /// `startDecideHooks` (an exported function that writes a store).
    #[serde(rename = "startDecideHooks")]
    StartDecideHooks,
    /// `startDownloadHooks` (an exported function that writes a store).
    #[serde(rename = "startDownloadHooks")]
    StartDownloadHooks { next: DownloadMover },
    /// `startEventFeed` (an exported function that writes a store). The callback parameters `send` are not in the JSON.
    #[serde(rename = "startEventFeed")]
    StartEventFeed,
    /// `startHookRules` (an exported function that writes a store).
    #[serde(rename = "startHookRules")]
    StartHookRules,
    /// `startOmniboxApprovals` (an exported function that writes a store).
    #[serde(rename = "startOmniboxApprovals")]
    StartOmniboxApprovals,
    /// `startOmniboxHooks` (an exported function that writes a store).
    #[serde(rename = "startOmniboxHooks")]
    StartOmniboxHooks,
    /// `startStoreEvents` (an exported function that writes a store). The callback parameters `send` are not in the JSON.
    #[serde(rename = "startStoreEvents")]
    StartStoreEvents { needs: StartStoreEventsNeeds },
    /// `startTabIdle` (an exported function that writes a store). The callback parameters `send` are not in the JSON.
    #[serde(rename = "startTabIdle")]
    StartTabIdle,
    /// `tabIcon` (an exported function that writes a store).
    #[serde(rename = "tabIcon")]
    TabIcon { results: Vec<OpResult>, op: Op },
    /// `tabNode` (an exported function that writes a store).
    #[serde(rename = "tabNode")]
    TabNode {
        s: Snapshot2,
        id: String,
        #[serde(default)]
        index: Option<Number>,
    },
    /// `tabRow` (an exported function that writes a store).
    #[serde(rename = "tabRow")]
    TabRow {
        s: crate::browser::BrowserState,
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `traceLink` (an exported function that writes a store).
    #[serde(rename = "traceLink")]
    TraceLink {
        url: String,
        source: Option<crate::air_traffic::NoteLinkSourceSource>,
    },
    /// `traceOpening` (an exported function that writes a store).
    #[serde(rename = "traceOpening")]
    TraceOpening { url: String },
    /// `updateFeed` (an exported function that writes a store). The callback parameters `send` are not in the JSON.
    #[serde(rename = "updateFeed")]
    UpdateFeed { needs: FeedNeeds },
    /// `wake` (an exported function that writes a store). The callback parameters `stillOn` are not in the JSON.
    #[serde(rename = "wake")]
    Wake {
        results: Vec<OpResult>,
        op: Op,
        #[serde(default)]
        owner: Option<Number>,
    },
    /// `workspace` (an exported function that writes a store).
    #[serde(rename = "workspace")]
    Workspace {
        s: Snapshot2,
        #[serde(default)]
        filter: Option<WorkspaceFilter>,
    },
    /// `workspaceMethod` (an exported function that writes a store).
    #[serde(rename = "workspaceMethod")]
    WorkspaceMethod {
        params: std::collections::HashMap<String, serde_json::Value>,
    },
}

/// Applies `action` to `app`. Slice 2.29 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.29")
}
