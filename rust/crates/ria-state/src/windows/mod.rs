// Ported from ardia apps/browser/src/state/windows/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `windows` (`apps/browser/src/state/windows`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.9 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CreateWindowOptions {
    #[serde(rename = "profileId", default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<CreateWindowOptionsKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(rename = "adoptId", default, skip_serializing_if = "Option::is_none")]
    pub adopt_id: Option<CreateWindowOptionsAdoptId>,
    #[serde(rename = "tabIds", default, skip_serializing_if = "Option::is_none")]
    pub tab_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<(Number, Number, Number, Number)>,
    #[serde(
        rename = "linkPointer",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub link_pointer: Option<(Number, Number)>,
    #[serde(rename = "linkApp", default, skip_serializing_if = "Option::is_none")]
    pub link_app: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    #[serde(
        rename = "chromeWindow",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub chrome_window: Option<String>,
    #[serde(
        rename = "opensBehind",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub opens_behind: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CreateWindowOptionsKind {
    #[default]
    #[serde(rename = "small")]
    Small,
    #[serde(rename = "main")]
    Main,
    #[serde(rename = "private")]
    Private,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CreateWindowOptionsAdoptId {
    Other0(String),
    Other1(String),
    Other2(String),
    Other3(String),
    Other4(String),
    Other5(String),
}

impl Default for CreateWindowOptionsAdoptId {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CloseGuard {
    #[serde(rename = "mayClose", default, skip_serializing_if = "Option::is_none")]
    pub may_close: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CloseWindowOptions {
    #[serde(
        rename = "keepsLastWindow",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub keeps_last_window: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AnimatedCloseOptions {
    #[serde(
        rename = "keepsLastWindow",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub keeps_last_window: Option<bool>,
    #[serde(
        rename = "replacedBy",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub replaced_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub then: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WindowSyncSetup {
    pub port: WindowPort,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WindowPort {}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MoveOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LastWindowSource {
    #[default]
    #[serde(rename = "dock")]
    Dock,
    #[serde(rename = "launch")]
    Launch,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum OpenPanelNewTab {
    #[serde(rename = "list")]
    List,
    #[serde(rename = "group")]
    Group {
        #[serde(rename = "groupId")]
        group_id: String,
    },
    #[serde(rename = "into")]
    Into {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
}

impl Default for OpenPanelNewTab {
    fn default() -> Self {
        Self::List
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum OpenPanelPlacement {
    #[default]
    #[serde(rename = "field")]
    Field,
    #[serde(rename = "centred")]
    Centred,
    #[serde(rename = "centredNoField")]
    CentredNoField,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CardAnchor {
    pub left: Number,
    pub top: Number,
    pub width: Number,
    pub height: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SetFindPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<Number>,
    #[serde(
        rename = "focusRequest",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub focus_request: Option<Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replace: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UrlAnchor {
    pub left: Number,
    pub top: Number,
    pub width: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sidebar: Option<UrlAnchorSidebar>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UrlAnchorSidebar {
    pub height: Number,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(rename = "closedWindows")]
    pub closed_windows: Vec<crate::browser::ClosedWindow>,
    pub find: std::collections::HashMap<String, crate::browser::FindState>,
    pub leaving: std::collections::HashMap<String, bool>,
    pub moving: std::collections::HashMap<String, bool>,
    #[serde(rename = "parkedPins")]
    pub parked_pins: std::collections::HashMap<String, crate::browser::ParkedPins>,
    pub peek: std::collections::HashMap<String, bool>,
    pub peeks: std::collections::HashMap<String, (Number, Number)>,
    pub raw: Option<String>,
    pub reported: bool,
    #[serde(rename = "scrolledDown")]
    pub scrolled_down: std::collections::HashMap<String, bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tiling: Option<bool>,
    pub ui: crate::automation::SnapshotUi,
    #[serde(rename = "windowOrder")]
    pub window_order: Vec<String>,
    pub windows: std::collections::HashMap<String, crate::browser::BrowserWindow>,
    #[serde(rename = "windowUi")]
    pub window_ui: std::collections::HashMap<String, crate::browser::WindowUi>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `activateFocusedPane` (an exported function that writes a store).
    #[serde(rename = "activateFocusedPane")]
    ActivateFocusedPane {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `askBeforeQuitting` (an exported function that writes a store). The callback parameters `ask`, `mayWrite` are not in the JSON.
    #[serde(rename = "askBeforeQuitting")]
    AskBeforeQuitting {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `clearDownloads` (an exported function that writes a store).
    #[serde(rename = "clearDownloads")]
    ClearDownloads {
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `clearRecentlyClosed` (a store method).
    #[serde(rename = "clearRecentlyClosed")]
    ClearRecentlyClosed {
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `closeAfterAsking` (an exported function that writes a store).
    #[serde(rename = "closeAfterAsking")]
    CloseAfterAsking {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(default)]
        arg1: Option<CloseGuard>,
    },
    /// `closeCheckPage` (an exported function that writes a store).
    #[serde(rename = "closeCheckPage")]
    CloseCheckPage {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `closeFromWindowControls` (an exported function that writes a store).
    #[serde(rename = "closeFromWindowControls")]
    CloseFromWindowControls {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `closePanel` (an exported function that writes a store).
    #[serde(rename = "closePanel")]
    ClosePanel {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `closeWindow` (a store method).
    #[serde(rename = "closeWindow")]
    CloseWindow {
        id: String,
        #[serde(default)]
        options: Option<CloseWindowOptions>,
    },
    /// `closeWindowAnimated` (an exported function that writes a store).
    #[serde(rename = "closeWindowAnimated")]
    CloseWindowAnimated {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(default)]
        arg1: Option<AnimatedCloseOptions>,
    },
    /// `closeWindowAsked` (an exported function that writes a store).
    #[serde(rename = "closeWindowAsked")]
    CloseWindowAsked {
        #[serde(rename = "windowId")]
        window_id: String,
        guard: CloseGuard,
    },
    /// `createWindow` (a store method).
    #[serde(rename = "createWindow")]
    CreateWindow {
        #[serde(default)]
        options: Option<CreateWindowOptions>,
    },
    /// `createWindowSync` (an exported function that writes a store).
    #[serde(rename = "createWindowSync")]
    CreateWindowSync { arg0: WindowSyncSetup },
    /// `decideLastTabClose` (an exported function that writes a store). The callback parameters `ask` are not in the JSON.
    #[serde(rename = "decideLastTabClose")]
    DecideLastTabClose {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `decideWindowClose` (an exported function that writes a store). The callback parameters `ask`, `mayWrite` are not in the JSON.
    #[serde(rename = "decideWindowClose")]
    DecideWindowClose {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `dismissDownloadsByPress` (an exported function that writes a store).
    #[serde(rename = "dismissDownloadsByPress")]
    DismissDownloadsByPress {
        #[serde(rename = "windowId")]
        window_id: String,
        press: serde_json::Value,
    },
    /// `downloadNavigated` (an exported function that writes a store).
    #[serde(rename = "downloadNavigated")]
    DownloadNavigated {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(rename = "committedUrl")]
        committed_url: Option<String>,
        skipped: bool,
    },
    /// `dropStaleLastWindowMark` (a store method).
    #[serde(rename = "dropStaleLastWindowMark")]
    DropStaleLastWindowMark,
    /// `dropTabsInProfile` (a store method).
    #[serde(rename = "dropTabsInProfile")]
    DropTabsInProfile {
        #[serde(rename = "tabIds")]
        tab_ids: Vec<String>,
        #[serde(rename = "profileId")]
        profile_id: String,
        drop: crate::drag::Drop,
    },
    /// `endThemeHold` (an exported function that writes a store).
    #[serde(rename = "endThemeHold")]
    EndThemeHold {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `escapePressed` (an exported function that writes a store).
    #[serde(rename = "escapePressed")]
    EscapePressed {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(rename = "pressedIn")]
        pressed_in: crate::telemetry::EscapePressedIn,
    },
    /// `forgetWindowRootSize` (an exported function that writes a store).
    #[serde(rename = "forgetWindowRootSize")]
    ForgetWindowRootSize {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `heldThemeColor` (an exported function that writes a store).
    #[serde(rename = "heldThemeColor")]
    HeldThemeColor {
        #[serde(rename = "tabId")]
        tab_id: String,
        url: String,
        reported: Option<String>,
        #[serde(rename = "isLoading")]
        is_loading: bool,
    },
    /// `holdThemeColor` (an exported function that writes a store).
    #[serde(rename = "holdThemeColor")]
    HoldThemeColor {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `hostThemeColor` (an exported function that writes a store).
    #[serde(rename = "hostThemeColor")]
    HostThemeColor {
        #[serde(rename = "tabId")]
        tab_id: String,
        url: String,
        reported: Option<String>,
        #[serde(rename = "isLoading")]
        is_loading: bool,
    },
    /// `mergeAllWindows` (a store method).
    #[serde(rename = "mergeAllWindows")]
    MergeAllWindows {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `moveGroupToProfile` (a store method).
    #[serde(rename = "moveGroupToProfile")]
    MoveGroupToProfile {
        #[serde(rename = "groupId")]
        group_id: String,
        #[serde(rename = "profileId")]
        profile_id: String,
        #[serde(default)]
        options: Option<MoveOptions>,
    },
    /// `moveGroupToWindow` (a store method).
    #[serde(rename = "moveGroupToWindow")]
    MoveGroupToWindow {
        #[serde(rename = "groupId")]
        group_id: String,
        #[serde(rename = "windowId")]
        window_id: Option<String>,
        #[serde(default)]
        options: Option<MoveOptions>,
    },
    /// `moveTabsToWindow` (a store method).
    #[serde(rename = "moveTabsToWindow")]
    MoveTabsToWindow {
        #[serde(rename = "tabIds")]
        tab_ids: Vec<String>,
        #[serde(rename = "windowId")]
        window_id: Option<String>,
        #[serde(default)]
        options: Option<MoveOptions>,
    },
    /// `moveTabToProfile` (a store method).
    #[serde(rename = "moveTabToProfile")]
    MoveTabToProfile {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(rename = "profileId")]
        profile_id: String,
        #[serde(default)]
        options: Option<MoveOptions>,
    },
    /// `noteMaximized` (an exported function that writes a store).
    #[serde(rename = "noteMaximized")]
    NoteMaximized {
        #[serde(rename = "windowId")]
        window_id: String,
        on: bool,
    },
    /// `noteScroll` (an exported function that writes a store).
    #[serde(rename = "noteScroll")]
    NoteScroll {
        #[serde(rename = "tabId")]
        tab_id: String,
        state: serde_json::Value,
    },
    /// `openCheckPage` (an exported function that writes a store).
    #[serde(rename = "openCheckPage")]
    OpenCheckPage {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `openLastWindowOrNew` (an exported function that writes a store).
    #[serde(rename = "openLastWindowOrNew")]
    OpenLastWindowOrNew { source: LastWindowSource },
    /// `openPanel` (an exported function that writes a store).
    #[serde(rename = "openPanel")]
    OpenPanel {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "initialText", default)]
        initial_text: Option<String>,
        #[serde(rename = "newTab", default)]
        new_tab: Option<OpenPanelNewTab>,
        #[serde(default)]
        placement: Option<OpenPanelPlacement>,
    },
    /// `removeDownload` (an exported function that writes a store).
    #[serde(rename = "removeDownload")]
    RemoveDownload { id: String },
    /// `reopenClosed` (a store method).
    #[serde(rename = "reopenClosed")]
    ReopenClosed {
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `reopenClosedTab` (a store method).
    #[serde(rename = "reopenClosedTab")]
    ReopenClosedTab {
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `reopenClosedWindow` (a store method).
    #[serde(rename = "reopenClosedWindow")]
    ReopenClosedWindow,
    /// `requestWindowClose` (an exported function that writes a store). The callback parameters `onceAgreed` are not in the JSON.
    #[serde(rename = "requestWindowClose")]
    RequestWindowClose {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `restoreClosed` (an exported function that writes a store).
    #[serde(rename = "restoreClosed")]
    RestoreClosed {
        #[serde(rename = "entryId")]
        entry_id: String,
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `restoreLaunchLook` (an exported function that writes a store).
    #[serde(rename = "restoreLaunchLook")]
    RestoreLaunchLook,
    /// `returnRefusedMove` (an exported function that writes a store).
    #[serde(rename = "returnRefusedMove")]
    ReturnRefusedMove {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(rename = "fromWindowId")]
        from_window_id: String,
    },
    /// `revealToolbar` (an exported function that writes a store).
    #[serde(rename = "revealToolbar")]
    RevealToolbar {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `setAppDark` (a store method).
    #[serde(rename = "setAppDark")]
    SetAppDark { dark: bool },
    /// `setCardAnchor` (an exported function that writes a store).
    #[serde(rename = "setCardAnchor")]
    SetCardAnchor {
        #[serde(rename = "windowId")]
        window_id: String,
        anchor: CardAnchor,
    },
    /// `setDesktopButtonLayout` (an exported function that writes a store).
    #[serde(rename = "setDesktopButtonLayout")]
    SetDesktopButtonLayout {
        raw: Option<String>,
        #[serde(default)]
        tiling: Option<bool>,
    },
    /// `setDockMoving` (an exported function that writes a store).
    #[serde(rename = "setDockMoving")]
    SetDockMoving {
        #[serde(rename = "windowId")]
        window_id: String,
        moving: bool,
    },
    /// `setDownloadsOpen` (an exported function that writes a store).
    #[serde(rename = "setDownloadsOpen")]
    SetDownloadsOpen {
        #[serde(rename = "windowId")]
        window_id: String,
        open: bool,
    },
    /// `setFind` (an exported function that writes a store).
    #[serde(rename = "setFind")]
    SetFind {
        #[serde(rename = "tabId")]
        tab_id: String,
        patch: SetFindPatch,
    },
    /// `setFocusedWindow` (a store method).
    #[serde(rename = "setFocusedWindow")]
    SetFocusedWindow { id: String },
    /// `setMeasuredWidth` (an exported function that writes a store).
    #[serde(rename = "setMeasuredWidth")]
    SetMeasuredWidth { key: String, width: Number },
    /// `setPeekLeaving` (an exported function that writes a store).
    #[serde(rename = "setPeekLeaving")]
    SetPeekLeaving {
        #[serde(rename = "windowId")]
        window_id: String,
        leaving: bool,
    },
    /// `setPeekShown` (an exported function that writes a store).
    #[serde(rename = "setPeekShown")]
    SetPeekShown {
        #[serde(rename = "windowId")]
        window_id: String,
        shown: bool,
        #[serde(default)]
        span: Option<(Number, Number)>,
    },
    /// `setToolbarExtensionsWidth` (an exported function that writes a store).
    #[serde(rename = "setToolbarExtensionsWidth")]
    SetToolbarExtensionsWidth {
        #[serde(rename = "windowId")]
        window_id: String,
        width: Number,
    },
    /// `setToolbarPeek` (an exported function that writes a store).
    #[serde(rename = "setToolbarPeek")]
    SetToolbarPeek {
        #[serde(rename = "tabId")]
        tab_id: String,
        on: bool,
    },
    /// `setUrlAnchor` (an exported function that writes a store).
    #[serde(rename = "setUrlAnchor")]
    SetUrlAnchor {
        #[serde(rename = "windowId")]
        window_id: String,
        anchor: UrlAnchor,
    },
    /// `setWindowFrame` (a store method).
    #[serde(rename = "setWindowFrame")]
    SetWindowFrame {
        id: String,
        frame: (Number, Number, Number, Number),
        #[serde(default)]
        restored: Option<(Number, Number, Number, Number)>,
        #[serde(default)]
        fitted: Option<bool>,
    },
    /// `setWindowFullScreen` (a store method).
    #[serde(rename = "setWindowFullScreen")]
    SetWindowFullScreen {
        id: String,
        on: bool,
        #[serde(default)]
        display: Option<String>,
    },
    /// `setWindowMaximized` (a store method).
    #[serde(rename = "setWindowMaximized")]
    SetWindowMaximized { id: String, on: bool },
    /// `setWindowRootSize` (an exported function that writes a store).
    #[serde(rename = "setWindowRootSize")]
    SetWindowRootSize {
        #[serde(rename = "windowId")]
        window_id: String,
        width: Number,
        height: Number,
    },
    /// `startCaptionRegionSync` (an exported function that writes a store).
    #[serde(rename = "startCaptionRegionSync")]
    StartCaptionRegionSync,
    /// `startPrivateSessions` (an exported function that writes a store).
    #[serde(rename = "startPrivateSessions")]
    StartPrivateSessions,
    /// `startUnclaimedEscape` (an exported function that writes a store).
    #[serde(rename = "startUnclaimedEscape")]
    StartUnclaimedEscape,
    /// `startWindowCloseGuard` (an exported function that writes a store).
    #[serde(rename = "startWindowCloseGuard")]
    StartWindowCloseGuard,
    /// `switchProfile` (a store method).
    #[serde(rename = "switchProfile")]
    SwitchProfile {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    /// `toggleDownloadsOpen` (an exported function that writes a store).
    #[serde(rename = "toggleDownloadsOpen")]
    ToggleDownloadsOpen {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "pressedAt", default)]
        pressed_at: Option<Number>,
    },
    /// `toggleSidebar` (an exported function that writes a store).
    #[serde(rename = "toggleSidebar")]
    ToggleSidebar {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `wakePanes` (an exported function that writes a store).
    #[serde(rename = "wakePanes")]
    WakePanes { ids: Vec<String> },
}

/// Applies `action` to `app`. Slice 2.9 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.9")
}
