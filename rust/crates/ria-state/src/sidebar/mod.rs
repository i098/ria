// Ported from ardia apps/browser/src/state/sidebar/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `sidebar` (`apps/browser/src/state/sidebar`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.18 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateEdgeHold {
    #[default]
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Hover {
    pub kind: HoverKind,
    pub id: String,
    #[serde(rename = "windowId")]
    pub window_id: String,
    pub anchor: Anchor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum HoverKind {
    #[default]
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "folder")]
    Folder,
    #[serde(rename = "live")]
    Live,
    #[serde(rename = "group")]
    Group,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Anchor {
    pub x: Number,
    pub y: Number,
    pub width: Number,
    pub height: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateIconPicker {
    pub kind: StateIconPickerKind,
    pub id: String,
    #[serde(rename = "windowId")]
    pub window_id: String,
    pub anchor: Option<Anchor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateIconPickerKind {
    #[default]
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "folder")]
    Folder,
    #[serde(rename = "live")]
    Live,
    #[serde(rename = "group")]
    Group,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Open {
    #[serde(rename = "windowId")]
    pub window_id: String,
    #[serde(rename = "folderId")]
    pub folder_id: String,
    pub anchor: OpenAnchor,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OpenAnchor {
    pub x: Number,
    pub y: Number,
    pub width: Number,
    pub height: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GhostPaneMark {
    pub w: Number,
    pub h: Number,
    pub x: Number,
    pub y: Number,
    pub corner: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateRenaming {
    pub kind: StateRenamingKind,
    pub id: String,
    #[serde(rename = "windowId")]
    pub window_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateRenamingKind {
    #[default]
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "folder")]
    Folder,
    #[serde(rename = "live")]
    Live,
    #[serde(rename = "group")]
    Group,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GhostShapeMark {
    pub w: Number,
    pub h: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SplitHalfMark {
    pub key: String,
    pub side: SplitHalfMarkSide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SplitHalfMarkSide {
    #[default]
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateSwitcher {
    #[serde(rename = "windowId")]
    pub window_id: String,
    pub ids: Vec<String>,
    pub index: Number,
    pub mounted: bool,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum BookmarkFolderChoice {
    #[serde(rename = "folder")]
    Folder {
        #[serde(rename = "parentId")]
        parent_id: String,
    },
    #[serde(rename = "newFolder")]
    NewFolder,
}

impl Default for BookmarkFolderChoice {
    fn default() -> Self {
        Self::Folder {
            parent_id: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TabPress {
    #[serde(rename = "windowId")]
    pub window_id: String,
    #[serde(rename = "tabId")]
    pub tab_id: String,
    #[serde(rename = "startedAt")]
    pub started_at: Number,
    pub switched: bool,
    #[serde(rename = "nativePress")]
    pub native_press: Number,
    pub at: Option<TabPressAt>,
    #[serde(rename = "windowKey")]
    pub window_key: bool,
    pub answered: bool,
    pub out: bool,
    pub drift: crate::telemetry::TabPressDrift,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TabPressAt {
    pub x: Number,
    pub y: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WindowChrome {
    #[serde(rename = "leadingWidth")]
    pub leading_width: Number,
    #[serde(rename = "trailingWidth")]
    pub trailing_width: Number,
    #[serde(rename = "titleBarHeight")]
    pub title_bar_height: Number,
    #[serde(rename = "leadingButtons")]
    pub leading_buttons: Vec<WindowButton>,
    #[serde(rename = "trailingButtons")]
    pub trailing_buttons: Vec<WindowButton>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum WindowButton {
    #[default]
    #[serde(rename = "menu")]
    Menu,
    #[serde(rename = "close")]
    Close,
    #[serde(rename = "minimize")]
    Minimize,
    #[serde(rename = "maximize")]
    Maximize,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CloseFolderPanelOnPressEvent {
    #[serde(rename = "nativeEvent")]
    pub native_event: CloseFolderPanelOnPressEventNativeEvent,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CloseFolderPanelOnPressEventNativeEvent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<Number>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Target {
    pub kind: TargetKind,
    pub id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TargetKind {
    #[default]
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "folder")]
    Folder,
    #[serde(rename = "live")]
    Live,
    #[serde(rename = "group")]
    Group,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MarkDragMarks {}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MarkDragPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<String>>,
    #[serde(rename = "dropInto", default, skip_serializing_if = "Option::is_none")]
    pub drop_into: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split: Option<SplitHalfMark>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tile: Option<GhostShapeMark>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row: Option<GhostShapeMark>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pane: Option<GhostPaneMark>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pins: Option<bool>,
    #[serde(rename = "pinCount", default, skip_serializing_if = "Option::is_none")]
    pub pin_count: Option<Number>,
    #[serde(rename = "pinBefore", default, skip_serializing_if = "Option::is_none")]
    pub pin_before: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folded: Option<String>,
    #[serde(
        rename = "folderTarget",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub folder_target: Option<String>,
    #[serde(
        rename = "folderAway",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub folder_away: Option<bool>,
    #[serde(
        rename = "profileHold",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub profile_hold: Option<String>,
    #[serde(
        rename = "overProfiles",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub over_profiles: Option<bool>,
    #[serde(rename = "edgeHold", default, skip_serializing_if = "Option::is_none")]
    pub edge_hold: Option<MarkDragPatchEdgeHold>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MarkDragPatchEdgeHold {
    #[default]
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RowEnteredEvent {
    #[serde(
        rename = "nativeEvent",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub native_event: Option<RowEnteredEventNativeEvent>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RowEnteredEventNativeEvent {
    #[serde(rename = "screenX", default, skip_serializing_if = "Option::is_none")]
    pub screen_x: Option<serde_json::Value>,
    #[serde(rename = "screenY", default, skip_serializing_if = "Option::is_none")]
    pub screen_y: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SetSidebarUiPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renaming: Option<StateRenaming>,
    #[serde(
        rename = "iconPicker",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub icon_picker: Option<StateIconPicker>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover: Option<Hover>,
    #[serde(
        rename = "searchTabs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub search_tabs: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub switcher: Option<StateSwitcher>,
    #[serde(rename = "dragWidth", default, skip_serializing_if = "Option::is_none")]
    pub drag_width: Option<std::collections::HashMap<String, Number>>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Undoable {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ToggleBookmarkInWindowPage {
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(rename = "dragWidth")]
    pub drag_width: std::collections::HashMap<String, Number>,
    #[serde(rename = "dropInto")]
    pub drop_into: Option<String>,
    #[serde(rename = "edgeHold")]
    pub edge_hold: Option<StateEdgeHold>,
    pub folded: Option<String>,
    #[serde(rename = "folderAway")]
    pub folder_away: bool,
    #[serde(rename = "folderTarget")]
    pub folder_target: Option<String>,
    pub hover: Option<Hover>,
    #[serde(rename = "iconPicker")]
    pub icon_picker: Option<StateIconPicker>,
    pub landing: Option<String>,
    pub open: Option<Open>,
    #[serde(rename = "overProfiles")]
    pub over_profiles: bool,
    pub pane: Option<GhostPaneMark>,
    #[serde(rename = "pinBefore")]
    pub pin_before: Option<String>,
    #[serde(rename = "pinCount")]
    pub pin_count: Number,
    pub pins: bool,
    #[serde(rename = "profileHold")]
    pub profile_hold: Option<String>,
    pub renaming: Option<StateRenaming>,
    pub row: Option<GhostShapeMark>,
    #[serde(rename = "searchTabs")]
    pub search_tabs: Option<String>,
    pub sources: Vec<String>,
    pub split: Option<SplitHalfMark>,
    pub switcher: Option<StateSwitcher>,
    pub tile: Option<GhostShapeMark>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `arrangeLive` (an exported function that writes a store).
    #[serde(rename = "arrangeLive")]
    ArrangeLive { sequence: Vec<String> },
    /// `bookmarkTabs` (an exported function that writes a store).
    #[serde(rename = "bookmarkTabs")]
    BookmarkTabs {
        #[serde(rename = "windowId")]
        window_id: String,
        ids: Vec<String>,
        folder: BookmarkFolderChoice,
    },
    /// `cancelSwitcher` (an exported function that writes a store).
    #[serde(rename = "cancelSwitcher")]
    CancelSwitcher,
    /// `cardLeft` (an exported function that writes a store).
    #[serde(rename = "cardLeft")]
    CardLeft,
    /// `cleanUpWithUndo` (an exported function that writes a store).
    #[serde(rename = "cleanUpWithUndo")]
    CleanUpWithUndo {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `clickHeldTabPress` (an exported function that writes a store).
    #[serde(rename = "clickHeldTabPress")]
    ClickHeldTabPress {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
        release: serde_json::Value,
        #[serde(default)]
        press: Option<TabPress>,
    },
    /// `clickTab` (an exported function that writes a store).
    #[serde(rename = "clickTab")]
    ClickTab {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(default)]
        mods: Option<crate::drag::ClickMods>,
    },
    /// `closeAllTabs` (an exported function that writes a store).
    #[serde(rename = "closeAllTabs")]
    CloseAllTabs {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `closeAnchoredWhenControlsRowChanges` (an exported function that writes a store).
    #[serde(rename = "closeAnchoredWhenControlsRowChanges")]
    CloseAnchoredWhenControlsRowChanges {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(default)]
        chrome: Option<WindowChrome>,
    },
    /// `closeFolderPanel` (an exported function that writes a store).
    #[serde(rename = "closeFolderPanel")]
    CloseFolderPanel,
    /// `closeFolderPanelIn` (an exported function that writes a store).
    #[serde(rename = "closeFolderPanelIn")]
    CloseFolderPanelIn {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `closeFolderPanelOnPress` (an exported function that writes a store).
    #[serde(rename = "closeFolderPanelOnPress")]
    CloseFolderPanelOnPress { event: CloseFolderPanelOnPressEvent },
    /// `closeFolderPanelWhenWindowMoves` (an exported function that writes a store).
    #[serde(rename = "closeFolderPanelWhenWindowMoves")]
    CloseFolderPanelWhenWindowMoves {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `closeSwitcherRow` (an exported function that writes a store).
    #[serde(rename = "closeSwitcherRow")]
    CloseSwitcherRow,
    /// `closeWhatTheSidebarAnchors` (an exported function that writes a store).
    #[serde(rename = "closeWhatTheSidebarAnchors")]
    CloseWhatTheSidebarAnchors {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `commitRename` (an exported function that writes a store).
    #[serde(rename = "commitRename")]
    CommitRename { target: Target, text: String },
    /// `commitSwitcher` (an exported function that writes a store).
    #[serde(rename = "commitSwitcher")]
    CommitSwitcher {
        #[serde(default)]
        index: Option<Number>,
    },
    /// `dismissHover` (an exported function that writes a store).
    #[serde(rename = "dismissHover")]
    DismissHover,
    /// `dismissHoverIn` (an exported function that writes a store).
    #[serde(rename = "dismissHoverIn")]
    DismissHoverIn {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `editPinnedPage` (an exported function that writes a store).
    #[serde(rename = "editPinnedPage")]
    EditPinnedPage {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `endHeldTabPress` (an exported function that writes a store).
    #[serde(rename = "endHeldTabPress")]
    EndHeldTabPress {
        #[serde(rename = "tabIds")]
        tab_ids: Vec<String>,
        release: serde_json::Value,
    },
    /// `endRename` (an exported function that writes a store).
    #[serde(rename = "endRename")]
    EndRename { target: Target },
    /// `focusSwitcherRow` (an exported function that writes a store).
    #[serde(rename = "focusSwitcherRow")]
    FocusSwitcherRow { index: Number },
    /// `holdHover` (an exported function that writes a store).
    #[serde(rename = "holdHover")]
    HoldHover { on: bool },
    /// `holdTabPress` (an exported function that writes a store).
    #[serde(rename = "holdTabPress")]
    HoldTabPress {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
        press: serde_json::Value,
        #[serde(default)]
        previous: Option<TabPress>,
    },
    /// `hoverEnter` (an exported function that writes a store).
    #[serde(rename = "hoverEnter")]
    HoverEnter {
        #[serde(rename = "windowId")]
        window_id: String,
        target: Target,
    },
    /// `hoverLeave` (an exported function that writes a store).
    #[serde(rename = "hoverLeave")]
    HoverLeave {
        #[serde(default)]
        from: Option<serde_json::Value>,
        #[serde(default)]
        now: Option<bool>,
    },
    /// `hoverSwitcherRow` (an exported function that writes a store).
    #[serde(rename = "hoverSwitcherRow")]
    HoverSwitcherRow { index: Number },
    /// `keepLiveAnchors` (an exported function that writes a store).
    #[serde(rename = "keepLiveAnchors")]
    KeepLiveAnchors {
        s: crate::browser::BrowserState,
        prev: crate::browser::BrowserState,
    },
    /// `markDrag` (an exported function that writes a store).
    #[serde(rename = "markDrag")]
    MarkDrag {
        marks: MarkDragMarks,
        patch: MarkDragPatch,
    },
    /// `middleClickTab` (an exported function that writes a store).
    #[serde(rename = "middleClickTab")]
    MiddleClickTab {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `moveLiveFolder` (an exported function that writes a store).
    #[serde(rename = "moveLiveFolder")]
    MoveLiveFolder {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "folderId")]
        folder_id: String,
        delta: Number,
    },
    /// `moveSwitcher` (an exported function that writes a store).
    #[serde(rename = "moveSwitcher")]
    MoveSwitcher { step: Number },
    /// `muteAll` (an exported function that writes a store).
    #[serde(rename = "muteAll")]
    MuteAll {
        #[serde(rename = "windowId")]
        window_id: String,
        muted: bool,
    },
    /// `newFolderWithTab` (an exported function that writes a store).
    #[serde(rename = "newFolderWithTab")]
    NewFolderWithTab {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `newGroupWithSelection` (an exported function that writes a store).
    #[serde(rename = "newGroupWithSelection")]
    NewGroupWithSelection {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(default)]
        ids: Option<Vec<String>>,
    },
    /// `openGroupMenu` (an exported function that writes a store).
    #[serde(rename = "openGroupMenu")]
    OpenGroupMenu {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "groupId")]
        group_id: String,
    },
    /// `openIconPicker` (an exported function that writes a store).
    #[serde(rename = "openIconPicker")]
    OpenIconPicker {
        #[serde(rename = "windowId")]
        window_id: String,
        target: Target,
    },
    /// `openLiveFolderMenu` (an exported function that writes a store).
    #[serde(rename = "openLiveFolderMenu")]
    OpenLiveFolderMenu {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "folderId")]
        folder_id: String,
    },
    /// `openLiveItemMenu` (an exported function that writes a store).
    #[serde(rename = "openLiveItemMenu")]
    OpenLiveItemMenu {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "folderId")]
        folder_id: String,
        item: crate::live::LiveItem,
        #[serde(rename = "tabId")]
        tab_id: Option<String>,
    },
    /// `openMultiMenu` (an exported function that writes a store).
    #[serde(rename = "openMultiMenu")]
    OpenMultiMenu {
        #[serde(rename = "windowId")]
        window_id: String,
        shown: Vec<String>,
    },
    /// `openOverflowMenu` (an exported function that writes a store).
    #[serde(rename = "openOverflowMenu")]
    OpenOverflowMenu {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `openSidebarMenu` (an exported function that writes a store).
    #[serde(rename = "openSidebarMenu")]
    OpenSidebarMenu {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `openTabMenu` (an exported function that writes a store).
    #[serde(rename = "openTabMenu")]
    OpenTabMenu {
        #[serde(rename = "windowId")]
        window_id: String,
        tab: crate::browser::Tab,
    },
    /// `openTileFolderMenu` (an exported function that writes a store).
    #[serde(rename = "openTileFolderMenu")]
    OpenTileFolderMenu {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "folderId")]
        folder_id: String,
    },
    /// `pressGroupClose` (an exported function that writes a store).
    #[serde(rename = "pressGroupClose")]
    PressGroupClose {
        #[serde(rename = "groupId")]
        group_id: String,
    },
    /// `pressPaneClose` (an exported function that writes a store).
    #[serde(rename = "pressPaneClose")]
    PressPaneClose {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(rename = "pullOut")]
        pull_out: bool,
    },
    /// `pressTabClose` (an exported function that writes a store).
    #[serde(rename = "pressTabClose")]
    PressTabClose {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `releaseAnchor` (an exported function that writes a store).
    #[serde(rename = "releaseAnchor")]
    ReleaseAnchor {
        #[serde(rename = "windowId")]
        window_id: String,
        id: String,
    },
    /// `removeFromFolder` (an exported function that writes a store).
    #[serde(rename = "removeFromFolder")]
    RemoveFromFolder {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `renameFolder` (an exported function that writes a store).
    #[serde(rename = "renameFolder")]
    RenameFolder {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "folderId")]
        folder_id: String,
        #[serde(default)]
        title: Option<String>,
    },
    /// `resetSidebarWidth` (an exported function that writes a store).
    #[serde(rename = "resetSidebarWidth")]
    ResetSidebarWidth,
    /// `returnShownTabToPinnedUrl` (an exported function that writes a store).
    #[serde(rename = "returnShownTabToPinnedUrl")]
    ReturnShownTabToPinnedUrl {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `rowEntered` (an exported function that writes a store). The callback parameters `enter`, `aiming` are not in the JSON.
    #[serde(rename = "rowEntered")]
    RowEntered {
        #[serde(rename = "windowId")]
        window_id: String,
        target: Option<Target>,
        event: RowEnteredEvent,
    },
    /// `rowLeft` (an exported function that writes a store).
    #[serde(rename = "rowLeft")]
    RowLeft {
        #[serde(rename = "windowId")]
        window_id: String,
        target: Option<Target>,
        event: RowEnteredEvent,
        #[serde(rename = "onPage")]
        on_page: bool,
    },
    /// `rowShown` (an exported function that writes a store).
    #[serde(rename = "rowShown")]
    RowShown {
        #[serde(rename = "windowId")]
        window_id: String,
        target: Option<Target>,
    },
    /// `runCalendarMenu` (an exported function that writes a store).
    #[serde(rename = "runCalendarMenu")]
    RunCalendarMenu { choice: String },
    /// `runNewLiveFolder` (an exported function that writes a store).
    #[serde(rename = "runNewLiveFolder")]
    RunNewLiveFolder {
        #[serde(rename = "windowId")]
        window_id: String,
        choice: String,
    },
    /// `runSidebarCommand` (an exported function that writes a store).
    #[serde(rename = "runSidebarCommand")]
    RunSidebarCommand {
        command: serde_json::Value,
        arg: Option<String>,
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `setIcon` (an exported function that writes a store).
    #[serde(rename = "setIcon")]
    SetIcon {
        target: Target,
        icon: Option<String>,
    },
    /// `setLiveSidebarWidth` (an exported function that writes a store).
    #[serde(rename = "setLiveSidebarWidth")]
    SetLiveSidebarWidth {
        #[serde(rename = "windowId")]
        window_id: String,
        width: Option<Number>,
    },
    /// `setSidebarUi` (an exported function that writes a store).
    #[serde(rename = "setSidebarUi")]
    SetSidebarUi { patch: SetSidebarUiPatch },
    /// `showUndoToast` (an exported function that writes a store).
    #[serde(rename = "showUndoToast")]
    ShowUndoToast {
        #[serde(rename = "windowId")]
        window_id: String,
        title: String,
        icon: String,
        __3: Undoable,
    },
    /// `startRename` (an exported function that writes a store).
    #[serde(rename = "startRename")]
    StartRename {
        #[serde(rename = "windowId")]
        window_id: String,
        target: Target,
    },
    /// `startSidebarEffects` (an exported function that writes a store).
    #[serde(rename = "startSidebarEffects")]
    StartSidebarEffects,
    /// `subscribeSwitcherTabChanges` (an exported function that writes a store).
    #[serde(rename = "subscribeSwitcherTabChanges")]
    SubscribeSwitcherTabChanges {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `suppressHover` (an exported function that writes a store).
    #[serde(rename = "suppressHover")]
    SuppressHover { value: bool },
    /// `switcherStep` (an exported function that writes a store).
    #[serde(rename = "switcherStep")]
    SwitcherStep {
        #[serde(rename = "windowId")]
        window_id: String,
        backward: bool,
    },
    /// `tidy` (an exported function that writes a store).
    #[serde(rename = "tidy")]
    Tidy {
        #[serde(rename = "windowId")]
        window_id: String,
        scope: crate::pins::TidyScope,
        #[serde(rename = "profileId", default)]
        profile_id: Option<String>,
    },
    /// `toggleBookmarkInWindow` (an exported function that writes a store).
    #[serde(rename = "toggleBookmarkInWindow")]
    ToggleBookmarkInWindow {
        #[serde(rename = "windowId")]
        window_id: String,
        page: ToggleBookmarkInWindowPage,
    },
    /// `toggleFolderPanel` (an exported function that writes a store).
    #[serde(rename = "toggleFolderPanel")]
    ToggleFolderPanel {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "folderId")]
        folder_id: String,
    },
    /// `togglePinnedRows` (an exported function that writes a store).
    #[serde(rename = "togglePinnedRows")]
    TogglePinnedRows {
        #[serde(rename = "windowId")]
        window_id: String,
    },
}

/// Applies `action` to `app`. Slice 2.18 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.18")
}
