// Ported from ardia apps/browser/src/state/bookmarks/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `bookmarks` (`apps/browser/src/state/bookmarks`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.16 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateGrab {
    pub x: Number,
    pub y: Number,
    pub width: Number,
    pub height: Number,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum StateOpen {
    #[serde(rename = "page")]
    Page {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "bookmarkId")]
        bookmark_id: String,
    },
    #[serde(rename = "allTabs")]
    AllTabs {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "profileId")]
        profile_id: String,
        tabs: Vec<StateOpenAllTabsTabsItem>,
    },
}

impl Default for StateOpen {
    fn default() -> Self {
        Self::Page {
            window_id: Default::default(),
            bookmark_id: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateOpenAllTabsTabsItem {
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FolderMenuPanel {
    pub key: String,
    #[serde(rename = "parentId")]
    pub parent_id: String,
    #[serde(rename = "fromIndex")]
    pub from_index: Number,
    pub anchor: MenuRect,
    pub level: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retained: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MenuRect {
    pub x: Number,
    pub y: Number,
    pub width: Number,
    pub height: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SidebarEdge {
    pub side: crate::browser::SidebarSide,
    pub x: Number,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum StateSource {
    #[serde(rename = "bookmark")]
    Bookmark { id: String, panel: Option<String> },
    #[serde(rename = "tabs")]
    Tabs {
        #[serde(rename = "tabIds")]
        tab_ids: Vec<String>,
    },
    #[serde(rename = "links")]
    Links,
}

impl Default for StateSource {
    fn default() -> Self {
        Self::Bookmark {
            id: Default::default(),
            panel: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StateSpot {
    Other0(StateSpotVariant0),
    Other1(StateSpotVariant1),
}

impl Default for StateSpot {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateSpotVariant0 {
    #[serde(rename = "parentId")]
    pub parent_id: String,
    pub slot: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateSpotVariant1 {
    pub into: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AddBookmarkOptions {
    #[serde(rename = "profileId")]
    pub profile_id: String,
    pub url: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    #[serde(rename = "parentId", default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<Number>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AddBookmarkFolderOptions {
    #[serde(rename = "profileId")]
    pub profile_id: String,
    pub title: String,
    #[serde(rename = "parentId", default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<Number>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BookmarkDraft {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(rename = "addedAt", default, skip_serializing_if = "Option::is_none")]
    pub added_at: Option<Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Box<BookmarkDraft>>>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BeginBookmarkDragOptions {
    #[serde(rename = "windowId")]
    pub window_id: String,
    pub id: String,
    pub panel: Option<String>,
    pub x: Number,
    pub y: Number,
    pub grab: StateGrab,
    #[serde(rename = "sidebarEdge")]
    pub sidebar_edge: SidebarEdge,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CloseFolderMenusArg0 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outside: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DropLinksAtLinksItem {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct EditBookmarkAdd {
    #[serde(rename = "profileId")]
    pub profile_id: String,
    #[serde(rename = "parentId", default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum OpenBarMenuNode {
    #[serde(rename = "url")]
    Url {
        id: String,
        #[serde(rename = "parentId")]
        parent_id: String,
        title: String,
        url: String,
        favicon: Option<String>,
        #[serde(rename = "addedAt")]
        added_at: Number,
        #[serde(rename = "syncKey", default, skip_serializing_if = "Option::is_none")]
        sync_key: Option<String>,
    },
    #[serde(rename = "folder")]
    Folder {
        id: String,
        #[serde(rename = "parentId")]
        parent_id: Option<String>,
        title: String,
        children: Vec<String>,
        #[serde(rename = "addedAt")]
        added_at: Number,
        #[serde(rename = "syncKey", default, skip_serializing_if = "Option::is_none")]
        sync_key: Option<String>,
    },
}

impl Default for OpenBarMenuNode {
    fn default() -> Self {
        Self::Url {
            id: Default::default(),
            parent_id: Default::default(),
            title: Default::default(),
            url: Default::default(),
            favicon: Default::default(),
            added_at: Default::default(),
            sync_key: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OpenBookmarkRowMenuArg1 {
    #[serde(rename = "windowId")]
    pub window_id: String,
    #[serde(rename = "profileId")]
    pub profile_id: String,
    #[serde(rename = "showPath")]
    pub show_path: bool,
    pub selection: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum OpenAllMode {
    #[default]
    #[serde(rename = "tabs")]
    Tabs,
    #[serde(rename = "window")]
    Window,
    #[serde(rename = "incognito")]
    Incognito,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OpenFolderMenuPanel {
    #[serde(rename = "parentId")]
    pub parent_id: String,
    pub anchor: MenuRect,
    #[serde(rename = "fromIndex")]
    pub from_index: Number,
    pub level: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retained: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OpenFolderMenuArg2 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprung: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OpenLinksGivenItem {
    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum OpenMode {
    #[default]
    #[serde(rename = "window")]
    Window,
    #[serde(rename = "incognito")]
    Incognito,
    #[serde(rename = "foreground")]
    Foreground,
    #[serde(rename = "split")]
    Split,
    #[serde(rename = "current")]
    Current,
    #[serde(rename = "background")]
    Background,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RemovedBookmarks {
    pub nodes: Vec<crate::browser::BookmarkNode>,
    pub places: Vec<RemovedBookmarksPlacesItem>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RemovedBookmarksPlacesItem {
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: String,
    pub index: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Link {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ToggleBookmarkPage {
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ToggleFolderMenuPanel {
    #[serde(rename = "parentId")]
    pub parent_id: String,
    pub anchor: MenuRect,
    #[serde(rename = "fromIndex")]
    pub from_index: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retained: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UpdateBookmarkPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub bookmarks: crate::browser::Bookmarks,
    #[serde(rename = "bookmarksReady")]
    pub bookmarks_ready: std::collections::HashMap<String, bool>,
    pub grab: StateGrab,
    pub held: Option<String>,
    pub open: Option<StateOpen>,
    pub panels: Vec<FolderMenuPanel>,
    #[serde(rename = "sidebarEdge")]
    pub sidebar_edge: SidebarEdge,
    pub source: Option<StateSource>,
    pub spot: Option<StateSpot>,
    pub sprung: bool,
    #[serde(rename = "windowId")]
    pub window_id: Option<String>,
    pub x: Number,
    pub y: Number,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `addBookmark` (an exported function that writes a store).
    #[serde(rename = "addBookmark")]
    AddBookmark { options: AddBookmarkOptions },
    /// `addBookmarkFolder` (an exported function that writes a store).
    #[serde(rename = "addBookmarkFolder")]
    AddBookmarkFolder { options: AddBookmarkFolderOptions },
    /// `addBookmarkTree` (an exported function that writes a store).
    #[serde(rename = "addBookmarkTree")]
    AddBookmarkTree {
        #[serde(rename = "profileId")]
        profile_id: String,
        drafts: Vec<BookmarkDraft>,
        #[serde(rename = "parentId", default)]
        parent_id: Option<String>,
        #[serde(default)]
        index: Option<Number>,
    },
    /// `addFolderToSidebar` (an exported function that writes a store).
    #[serde(rename = "addFolderToSidebar")]
    AddFolderToSidebar {
        #[serde(rename = "folderId")]
        folder_id: String,
        #[serde(rename = "windowId")]
        window_id: Option<String>,
    },
    /// `addGroupsFromDrafts` (an exported function that writes a store).
    #[serde(rename = "addGroupsFromDrafts")]
    AddGroupsFromDrafts {
        #[serde(rename = "profileId")]
        profile_id: String,
        drafts: Vec<crate::groups::GroupDraft>,
        #[serde(default)]
        near: Option<String>,
        #[serde(default)]
        options: Option<crate::groups::DraftGroupOptions>,
    },
    /// `beginBookmarkDrag` (an exported function that writes a store).
    #[serde(rename = "beginBookmarkDrag")]
    BeginBookmarkDrag { options: BeginBookmarkDragOptions },
    /// `bookmarkAllTabs` (an exported function that writes a store).
    #[serde(rename = "bookmarkAllTabs")]
    BookmarkAllTabs {
        #[serde(rename = "windowId")]
        window_id: Option<String>,
    },
    /// `bookmarkTab` (an exported function that writes a store).
    #[serde(rename = "bookmarkTab")]
    BookmarkTab {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `closeBookmarkDialog` (an exported function that writes a store).
    #[serde(rename = "closeBookmarkDialog")]
    CloseBookmarkDialog,
    /// `closeBookmarkEditor` (an exported function that writes a store).
    #[serde(rename = "closeBookmarkEditor")]
    CloseBookmarkEditor,
    /// `closeFolderMenus` (an exported function that writes a store).
    #[serde(rename = "closeFolderMenus")]
    CloseFolderMenus {
        #[serde(default)]
        arg0: Option<CloseFolderMenusArg0>,
    },
    /// `closeMenusAbove` (an exported function that writes a store).
    #[serde(rename = "closeMenusAbove")]
    CloseMenusAbove {
        #[serde(rename = "windowId")]
        window_id: String,
        level: Number,
    },
    /// `closeSprungMenus` (an exported function that writes a store).
    #[serde(rename = "closeSprungMenus")]
    CloseSprungMenus,
    /// `dropLinksAt` (an exported function that writes a store).
    #[serde(rename = "dropLinksAt")]
    DropLinksAt {
        #[serde(rename = "windowId")]
        window_id: String,
        x: Number,
        y: Number,
        links: Vec<DropLinksAtLinksItem>,
    },
    /// `dropTabsOnBar` (an exported function that writes a store).
    #[serde(rename = "dropTabsOnBar")]
    DropTabsOnBar {
        #[serde(rename = "tabIds")]
        tab_ids: Vec<String>,
        #[serde(rename = "windowId")]
        window_id: String,
        x: Number,
        y: Number,
    },
    /// `editBookmark` (an exported function that writes a store).
    #[serde(rename = "editBookmark")]
    EditBookmark {
        id: Option<String>,
        #[serde(rename = "windowId")]
        window_id: Option<String>,
        #[serde(default)]
        add: Option<EditBookmarkAdd>,
    },
    /// `endBookmarkDrag` (an exported function that writes a store).
    #[serde(rename = "endBookmarkDrag")]
    EndBookmarkDrag { commit: bool },
    /// `exportBookmarksFromWindow` (an exported function that writes a store).
    #[serde(rename = "exportBookmarksFromWindow")]
    ExportBookmarksFromWindow {
        #[serde(rename = "windowId")]
        window_id: Option<String>,
    },
    /// `followTabDragOnBar` (an exported function that writes a store).
    #[serde(rename = "followTabDragOnBar")]
    FollowTabDragOnBar { d: crate::drag::TabDrag },
    /// `holdMenuPanel` (an exported function that writes a store).
    #[serde(rename = "holdMenuPanel")]
    HoldMenuPanel { key: Option<String> },
    /// `linkDragAt` (an exported function that writes a store).
    #[serde(rename = "linkDragAt")]
    LinkDragAt {
        #[serde(rename = "windowId")]
        window_id: String,
        x: Number,
        y: Number,
    },
    /// `linkDragLeft` (an exported function that writes a store).
    #[serde(rename = "linkDragLeft")]
    LinkDragLeft,
    /// `moveBookmark` (an exported function that writes a store).
    #[serde(rename = "moveBookmark")]
    MoveBookmark {
        id: String,
        #[serde(rename = "parentId")]
        parent_id: String,
        #[serde(default)]
        index: Option<Number>,
    },
    /// `moveBookmarkDrag` (an exported function that writes a store).
    #[serde(rename = "moveBookmarkDrag")]
    MoveBookmarkDrag { x: Number, y: Number },
    /// `moveBookmarks` (an exported function that writes a store).
    #[serde(rename = "moveBookmarks")]
    MoveBookmarks {
        ids: Vec<String>,
        #[serde(rename = "parentId")]
        parent_id: String,
        #[serde(default)]
        index: Option<Number>,
    },
    /// `openBarMenu` (an exported function that writes a store).
    #[serde(rename = "openBarMenu")]
    OpenBarMenu {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "profileId")]
        profile_id: String,
        node: Option<OpenBarMenuNode>,
    },
    /// `openBookmarkRowMenu` (an exported function that writes a store).
    #[serde(rename = "openBookmarkRowMenu")]
    OpenBookmarkRowMenu {
        node: crate::browser::BookmarkNode,
        arg1: OpenBookmarkRowMenuArg1,
    },
    /// `openFolder` (an exported function that writes a store).
    #[serde(rename = "openFolder")]
    OpenFolder {
        #[serde(rename = "folderId")]
        folder_id: String,
        #[serde(rename = "windowId")]
        window_id: Option<String>,
        #[serde(default)]
        mode: Option<OpenAllMode>,
    },
    /// `openFolderMenu` (an exported function that writes a store).
    #[serde(rename = "openFolderMenu")]
    OpenFolderMenu {
        #[serde(rename = "windowId")]
        window_id: String,
        panel: OpenFolderMenuPanel,
        #[serde(default)]
        arg2: Option<OpenFolderMenuArg2>,
    },
    /// `openImportButtonMenu` (an exported function that writes a store).
    #[serde(rename = "openImportButtonMenu")]
    OpenImportButtonMenu,
    /// `openLinks` (an exported function that writes a store).
    #[serde(rename = "openLinks")]
    OpenLinks {
        given: Vec<OpenLinksGivenItem>,
        #[serde(rename = "windowId")]
        window_id: Option<String>,
        #[serde(default)]
        mode: Option<OpenAllMode>,
        #[serde(rename = "manyMessage", default)]
        many_message: Option<String>,
    },
    /// `openOrganizeMenu` (an exported function that writes a store).
    #[serde(rename = "openOrganizeMenu")]
    OpenOrganizeMenu {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "profileId")]
        profile_id: String,
        #[serde(rename = "folderId")]
        folder_id: Option<String>,
    },
    /// `openUrl` (an exported function that writes a store).
    #[serde(rename = "openUrl")]
    OpenUrl {
        url: String,
        #[serde(rename = "windowId")]
        window_id: Option<String>,
        mode: OpenMode,
    },
    /// `reloadBookmarks` (an exported function that writes a store).
    #[serde(rename = "reloadBookmarks")]
    ReloadBookmarks,
    /// `removeBookmark` (an exported function that writes a store).
    #[serde(rename = "removeBookmark")]
    RemoveBookmark { id: String },
    /// `removeBookmarks` (an exported function that writes a store).
    #[serde(rename = "removeBookmarks")]
    RemoveBookmarks { ids: Vec<String> },
    /// `restoreArcFolders` (an exported function that writes a store).
    #[serde(rename = "restoreArcFolders")]
    RestoreArcFolders {
        #[serde(rename = "holderId")]
        holder_id: String,
        #[serde(rename = "windowId")]
        window_id: Option<String>,
    },
    /// `restoreBookmarks` (an exported function that writes a store).
    #[serde(rename = "restoreBookmarks")]
    RestoreBookmarks { removed: RemovedBookmarks },
    /// `runLinkChoice` (an exported function that writes a store).
    #[serde(rename = "runLinkChoice")]
    RunLinkChoice {
        choice: Option<String>,
        links: Vec<Link>,
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "manyMessage", default)]
        many_message: Option<String>,
    },
    /// `startBookmarks` (an exported function that writes a store).
    #[serde(rename = "startBookmarks")]
    StartBookmarks,
    /// `toggleBookmark` (an exported function that writes a store).
    #[serde(rename = "toggleBookmark")]
    ToggleBookmark {
        #[serde(rename = "profileId")]
        profile_id: String,
        page: ToggleBookmarkPage,
    },
    /// `toggleBookmarkOfTab` (an exported function that writes a store).
    #[serde(rename = "toggleBookmarkOfTab")]
    ToggleBookmarkOfTab {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `toggleFolderMenu` (an exported function that writes a store).
    #[serde(rename = "toggleFolderMenu")]
    ToggleFolderMenu {
        #[serde(rename = "windowId")]
        window_id: String,
        panel: ToggleFolderMenuPanel,
    },
    /// `updateBookmark` (an exported function that writes a store).
    #[serde(rename = "updateBookmark")]
    UpdateBookmark {
        id: String,
        patch: UpdateBookmarkPatch,
    },
    /// `watchBookmarkEscape` (an exported function that writes a store).
    #[serde(rename = "watchBookmarkEscape")]
    WatchBookmarkEscape,
}

/// Applies `action` to `app`. Slice 2.16 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.16")
}
