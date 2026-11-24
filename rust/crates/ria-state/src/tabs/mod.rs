// Ported from ardia apps/browser/src/state/tabs/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `tabs` (`apps/browser/src/state/tabs`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.8 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OpeningContext {
    #[serde(rename = "tabId")]
    pub tab_id: Option<String>,
    #[serde(rename = "windowId")]
    pub window_id: Option<String>,
    #[serde(rename = "profileId")]
    pub profile_id: Option<String>,
    pub via: OpeningVia,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum OpeningVia {
    #[default]
    #[serde(rename = "newTab")]
    NewTab,
    #[serde(rename = "link")]
    Link,
    #[serde(rename = "external")]
    External,
    #[serde(rename = "typed")]
    Typed,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StripHover {
    #[serde(rename = "windowId")]
    pub window_id: String,
    #[serde(rename = "tabId")]
    pub tab_id: String,
    pub anchor: crate::sidebar::Anchor,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HistoryMenu {
    #[serde(rename = "tabId")]
    pub tab_id: String,
    pub direction: Number,
    pub items: Vec<HistoryItem>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HistoryItem {
    pub key: String,
    pub title: String,
    pub url: String,
    pub offset: HistoryItemOffset,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HistoryItemOffset {
    Number(Number),
    String(String),
}

impl Default for HistoryItemOffset {
    fn default() -> Self {
        Self::Number(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct NewTabNotice {
    pub id: Number,
    #[serde(rename = "tabId")]
    pub tab_id: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PageState {
    pub status: String,
    #[serde(rename = "statusAtTop")]
    pub status_at_top: bool,
    pub crashed: serde_json::Value,
    pub unresponsive: bool,
    pub fullscreen: bool,
    pub security: serde_json::Value,
    pub blocked: Number,
    #[serde(rename = "themeColorSource")]
    pub theme_color_source: serde_json::Value,
    pub popups: Vec<serde_json::Value>,
    pub permission: serde_json::Value,
    #[serde(rename = "passwordPrompt")]
    pub password_prompt: serde_json::Value,
    #[serde(rename = "autofillPrompt")]
    pub autofill_prompt: serde_json::Value,
    #[serde(rename = "externalApp")]
    pub external_app: serde_json::Value,
    #[serde(rename = "verificationCode")]
    pub verification_code: Option<VerificationCodeOffer>,
    #[serde(rename = "mediaAccess")]
    pub media_access: serde_json::Value,
    #[serde(rename = "wasNewTab")]
    pub was_new_tab: bool,
    #[serde(rename = "backToNewTab")]
    pub back_to_new_tab: bool,
    #[serde(rename = "engineBack")]
    pub engine_back: Option<PageStateEngineBack>,
    #[serde(rename = "newTabShown")]
    pub new_tab_shown: Option<PageStateNewTabShown>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct VerificationCodeOffer {
    pub token: String,
    pub app: String,
    pub host: String,
    pub generation: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PageStateEngineBack {
    pub engine: serde_json::Value,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PageStateNewTabShown {
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StatePopoverValue {
    #[default]
    #[serde(rename = "popups")]
    Popups,
    #[serde(rename = "zoom")]
    Zoom,
    #[serde(rename = "siteControls")]
    SiteControls,
    #[serde(rename = "hiddenElements")]
    HiddenElements,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CloseOptions {
    #[serde(
        rename = "lastClosesWindow",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_closes_window: Option<bool>,
    #[serde(
        rename = "keepTopApps",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub keep_top_apps: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct NavigateOptions {
    #[serde(
        rename = "userInitiated",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub user_initiated: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gated: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct NewTabOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    #[serde(rename = "adoptId", default, skip_serializing_if = "Option::is_none")]
    pub adopt_id: Option<NewTabOptionsAdoptId>,
    #[serde(rename = "openerId", default, skip_serializing_if = "Option::is_none")]
    pub opener_id: Option<String>,
    #[serde(rename = "profileId", default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<NewTabOptionsSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<Number>,
    #[serde(
        rename = "wakeAdoptId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub wake_adopt_id: Option<NewTabOptionsWakeAdoptId>,
    #[serde(
        rename = "newTabCommand",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub new_tab_command: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gated: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaced: Option<bool>,
    #[serde(
        rename = "enginePage",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub engine_page: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NewTabOptionsAdoptId {
    Other0(String),
    Other1(String),
    Other2(String),
    Other3(String),
    Other4(String),
    Other5(String),
}

impl Default for NewTabOptionsAdoptId {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct NewTabOptionsSnapshot {
    #[serde(rename = "profileId", default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin: Option<NewTabOptionsSnapshotPin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub muted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoom: Option<Number>,
    #[serde(
        rename = "customTitle",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_title: Option<String>,
    #[serde(
        rename = "customIcon",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_icon: Option<String>,
    #[serde(
        rename = "lastActiveAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_active_at: Option<Number>,
    #[serde(rename = "pinnedUrl", default, skip_serializing_if = "Option::is_none")]
    pub pinned_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "as")]
pub enum NewTabOptionsSnapshotPin {
    #[serde(rename = "tile")]
    Tile {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        folder: Option<crate::browser::TileFolder>,
    },
    #[serde(rename = "row")]
    Row,
}

impl Default for NewTabOptionsSnapshotPin {
    fn default() -> Self {
        Self::Tile {
            folder: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NewTabOptionsWakeAdoptId {
    Other0(String),
    Other1(String),
    Other2(String),
    Other3(String),
    Other4(String),
    Other5(String),
}

impl Default for NewTabOptionsWakeAdoptId {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PageRef {
    #[serde(rename = "windowId")]
    pub window_id: String,
    #[serde(rename = "profileId")]
    pub profile_id: String,
    #[serde(rename = "tabId", default, skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PatchPagePatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(
        rename = "statusAtTop",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub status_at_top: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crashed: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresponsive: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fullscreen: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked: Option<Number>,
    #[serde(
        rename = "themeColorSource",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub theme_color_source: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub popups: Option<Vec<serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission: Option<serde_json::Value>,
    #[serde(
        rename = "passwordPrompt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub password_prompt: Option<serde_json::Value>,
    #[serde(
        rename = "autofillPrompt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub autofill_prompt: Option<serde_json::Value>,
    #[serde(
        rename = "externalApp",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub external_app: Option<serde_json::Value>,
    #[serde(
        rename = "verificationCode",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub verification_code: Option<VerificationCodeOffer>,
    #[serde(
        rename = "mediaAccess",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub media_access: Option<serde_json::Value>,
    #[serde(rename = "wasNewTab", default, skip_serializing_if = "Option::is_none")]
    pub was_new_tab: Option<bool>,
    #[serde(
        rename = "backToNewTab",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub back_to_new_tab: Option<bool>,
    #[serde(
        rename = "engineBack",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub engine_back: Option<PageStateEngineBack>,
    #[serde(
        rename = "newTabShown",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub new_tab_shown: Option<PageStateNewTabShown>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetPopoverPopover {
    #[default]
    #[serde(rename = "popups")]
    Popups,
    #[serde(rename = "zoom")]
    Zoom,
    #[serde(rename = "siteControls")]
    SiteControls,
    #[serde(rename = "hiddenElements")]
    HiddenElements,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SleepTabV1 {
    #[serde(rename = "mayWrite", default, skip_serializing_if = "Option::is_none")]
    pub may_write: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PagePopover {
    #[default]
    #[serde(rename = "popups")]
    Popups,
    #[serde(rename = "zoom")]
    Zoom,
    #[serde(rename = "siteControls")]
    SiteControls,
    #[serde(rename = "hiddenElements")]
    HiddenElements,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UpdateLivePatch {
    #[serde(rename = "isLoading", default, skip_serializing_if = "Option::is_none")]
    pub is_loading: Option<bool>,
    #[serde(rename = "canGoBack", default, skip_serializing_if = "Option::is_none")]
    pub can_go_back: Option<bool>,
    #[serde(
        rename = "canGoForward",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub can_go_forward: Option<bool>,
    #[serde(
        rename = "playingAudio",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub playing_audio: Option<bool>,
    #[serde(
        rename = "themeColor",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub theme_color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UpdateTabPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "windowId", default, skip_serializing_if = "Option::is_none")]
    pub window_id: Option<String>,
    #[serde(rename = "profileId", default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin: Option<UpdateTabPatchPin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub muted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoom: Option<Number>,
    #[serde(
        rename = "customTitle",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_title: Option<String>,
    #[serde(
        rename = "customIcon",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_icon: Option<String>,
    #[serde(rename = "pinnedUrl", default, skip_serializing_if = "Option::is_none")]
    pub pinned_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<UpdateTabPatchPage>,
    #[serde(rename = "openerId", default, skip_serializing_if = "Option::is_none")]
    pub opener_id: Option<String>,
    #[serde(
        rename = "linkOpenerId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub link_opener_id: Option<String>,
    #[serde(rename = "liveItem", default, skip_serializing_if = "Option::is_none")]
    pub live_item: Option<crate::browser::TabLiveItem>,
    #[serde(rename = "pinKey", default, skip_serializing_if = "Option::is_none")]
    pub pin_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<serde_json::Value>,
    #[serde(
        rename = "chromeOpened",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub chrome_opened: Option<bool>,
    #[serde(rename = "createdAt", default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<Number>,
    #[serde(
        rename = "lastActiveAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_active_at: Option<Number>,
    #[serde(
        rename = "restoredAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub restored_at: Option<Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unseen: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "as")]
pub enum UpdateTabPatchPin {
    #[serde(rename = "tile")]
    Tile {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        folder: Option<crate::browser::TileFolder>,
    },
    #[serde(rename = "row")]
    Row,
}

impl Default for UpdateTabPatchPin {
    fn default() -> Self {
        Self::Tile {
            folder: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum UpdateTabPatchPage {
    #[serde(rename = "idle")]
    Idle {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wake: Option<UpdateTabPatchPageIdleWake>,
    },
    #[serde(rename = "asleep")]
    Asleep {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wake: Option<UpdateTabPatchPageAsleepWake>,
    },
    #[serde(rename = "frozen")]
    Frozen {
        #[serde(rename = "holderId")]
        holder_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wake: Option<UpdateTabPatchPageFrozenWake>,
    },
    #[serde(rename = "adopted")]
    Adopted {
        adopt: UpdateTabPatchPageAdoptedAdopt,
        nav: Option<crate::browser::Navigation>,
    },
    #[serde(rename = "loaded")]
    Loaded { nav: crate::browser::Navigation },
}

impl Default for UpdateTabPatchPage {
    fn default() -> Self {
        Self::Idle {
            wake: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpdateTabPatchPageIdleWake {
    Other0(String),
    Other1(String),
    Other2(String),
    Other3(String),
    Other4(String),
    Other5(String),
}

impl Default for UpdateTabPatchPageIdleWake {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpdateTabPatchPageAsleepWake {
    Other0(String),
    Other1(String),
    Other2(String),
    Other3(String),
    Other4(String),
    Other5(String),
}

impl Default for UpdateTabPatchPageAsleepWake {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpdateTabPatchPageFrozenWake {
    Other0(String),
    Other1(String),
    Other2(String),
    Other3(String),
    Other4(String),
    Other5(String),
}

impl Default for UpdateTabPatchPageFrozenWake {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpdateTabPatchPageAdoptedAdopt {
    Other0(String),
    Other1(String),
    Other2(String),
    Other3(String),
    Other4(String),
    Other5(String),
}

impl Default for UpdateTabPatchPageAdoptedAdopt {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(rename = "batterySaver")]
    pub battery_saver: bool,
    #[serde(rename = "closedTabs")]
    pub closed_tabs: Vec<crate::browser::ClosedTab>,
    pub discarded: std::collections::HashMap<String, bool>,
    pub hover: Option<StripHover>,
    pub menu: Option<HistoryMenu>,
    pub notices: std::collections::HashMap<String, Option<NewTabNotice>>,
    pub pages: std::collections::HashMap<String, PageState>,
    pub popover: std::collections::HashMap<String, Option<StatePopoverValue>>,
    #[serde(rename = "privateSiteMutes")]
    pub private_site_mutes:
        std::collections::HashMap<String, std::collections::HashMap<String, bool>>,
    pub ready: std::collections::HashMap<String, Number>,
    pub tabs: std::collections::HashMap<String, crate::browser::Tab>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `activate` (a store method).
    #[serde(rename = "activate")]
    Activate { id: String },
    /// `activateIndex` (a store method).
    #[serde(rename = "activateIndex")]
    ActivateIndex {
        #[serde(rename = "windowId")]
        window_id: String,
        index: Number,
    },
    /// `checkMovedTabProfile` (an exported function that writes a store).
    #[serde(rename = "checkMovedTabProfile")]
    CheckMovedTabProfile,
    /// `clearNewTabNotice` (an exported function that writes a store).
    #[serde(rename = "clearNewTabNotice")]
    ClearNewTabNotice {
        #[serde(rename = "windowId")]
        window_id: String,
        id: Number,
    },
    /// `closeEveryPopover` (an exported function that writes a store).
    #[serde(rename = "closeEveryPopover")]
    CloseEveryPopover,
    /// `closeHistoryMenu` (an exported function that writes a store).
    #[serde(rename = "closeHistoryMenu")]
    CloseHistoryMenu,
    /// `closeHistoryMenuOf` (an exported function that writes a store).
    #[serde(rename = "closeHistoryMenuOf")]
    CloseHistoryMenuOf {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `closeTab` (a store method).
    #[serde(rename = "closeTab")]
    CloseTab { id: String },
    /// `closeTabs` (a store method).
    #[serde(rename = "closeTabs")]
    CloseTabs {
        ids: Vec<String>,
        #[serde(default)]
        options: Option<CloseOptions>,
    },
    /// `cycle` (a store method).
    #[serde(rename = "cycle")]
    Cycle {
        #[serde(rename = "windowId")]
        window_id: String,
        delta: Number,
    },
    /// `dismissPopoverByPress` (an exported function that writes a store).
    #[serde(rename = "dismissPopoverByPress")]
    DismissPopoverByPress {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `dismissStripHover` (an exported function that writes a store).
    #[serde(rename = "dismissStripHover")]
    DismissStripHover,
    /// `dismissStripHoverIn` (an exported function that writes a store).
    #[serde(rename = "dismissStripHoverIn")]
    DismissStripHoverIn {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `dropTabsAt` (an exported function that writes a store).
    #[serde(rename = "dropTabsAt")]
    DropTabsAt {
        ids: Vec<String>,
        source: crate::drag::DragSource,
        spot: crate::drag::DropSpot,
    },
    /// `dropTabsInSlot` (an exported function that writes a store).
    #[serde(rename = "dropTabsInSlot")]
    DropTabsInSlot {
        ids: Vec<String>,
        slots: Vec<crate::drag::Slot>,
        from: Number,
        to: Number,
        lane: crate::drag::Lane,
    },
    /// `duplicateTab` (a store method).
    #[serde(rename = "duplicateTab")]
    DuplicateTab { id: String },
    /// `goBack` (an exported function that writes a store). The callback parameters `mayAct` are not in the JSON.
    #[serde(rename = "goBack")]
    GoBack {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `goBackAsking` (an exported function that writes a store). The callback parameters `mayAct`, `ask` are not in the JSON.
    #[serde(rename = "goBackAsking")]
    GoBackAsking {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `goForward` (an exported function that writes a store). The callback parameters `mayAct` are not in the JSON.
    #[serde(rename = "goForward")]
    GoForward {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `goForwardAsking` (an exported function that writes a store). The callback parameters `mayAct` are not in the JSON.
    #[serde(rename = "goForwardAsking")]
    GoForwardAsking {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `goToHistoryItem` (an exported function that writes a store).
    #[serde(rename = "goToHistoryItem")]
    GoToHistoryItem {
        #[serde(rename = "tabId")]
        tab_id: String,
        item: HistoryItem,
    },
    /// `goToNoticedTab` (an exported function that writes a store).
    #[serde(rename = "goToNoticedTab")]
    GoToNoticedTab {
        #[serde(rename = "windowId")]
        window_id: String,
        notice: NewTabNotice,
    },
    /// `handOffPin` (a store method).
    #[serde(rename = "handOffPin")]
    HandOffPin { id: String },
    /// `holdNewTabNotice` (an exported function that writes a store).
    #[serde(rename = "holdNewTabNotice")]
    HoldNewTabNotice {
        #[serde(rename = "windowId")]
        window_id: String,
        notice: NewTabNotice,
        held: bool,
    },
    /// `keepSiteActive` (an exported function that writes a store).
    #[serde(rename = "keepSiteActive")]
    KeepSiteActive { typed: String },
    /// `leftPage` (an exported function that writes a store).
    #[serde(rename = "leftPage")]
    LeftPage {
        #[serde(rename = "tabId")]
        tab_id: String,
        url: String,
        #[serde(rename = "previousPage")]
        previous_page: String,
        engine: serde_json::Value,
        #[serde(rename = "inPeek")]
        in_peek: bool,
    },
    /// `markTabReady` (an exported function that writes a store).
    #[serde(rename = "markTabReady")]
    MarkTabReady {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `moveInGroup` (an exported function that writes a store).
    #[serde(rename = "moveInGroup")]
    MoveInGroup {
        #[serde(rename = "groupId")]
        group_id: String,
        #[serde(rename = "beforeId")]
        before_id: Option<String>,
    },
    /// `moveTab` (a store method).
    #[serde(rename = "moveTab")]
    MoveTab {
        id: String,
        #[serde(rename = "toIndex")]
        to_index: Number,
    },
    /// `moveTabBy` (a store method).
    #[serde(rename = "moveTabBy")]
    MoveTabBy { id: String, delta: Number },
    /// `navigate` (a store method).
    #[serde(rename = "navigate")]
    Navigate {
        id: String,
        input: String,
        #[serde(default)]
        options: Option<NavigateOptions>,
    },
    /// `newTab` (a store method).
    #[serde(rename = "newTab")]
    NewTab {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(default)]
        options: Option<NewTabOptions>,
    },
    /// `newTabHome` (an exported function that writes a store).
    #[serde(rename = "newTabHome")]
    NewTabHome {
        s: crate::browser::BrowserState,
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "profileId")]
        profile_id: Option<String>,
    },
    /// `noteBackgroundTab` (an exported function that writes a store).
    #[serde(rename = "noteBackgroundTab")]
    NoteBackgroundTab {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(rename = "sourceId")]
        source_id: String,
    },
    /// `noteDiscarded` (an exported function that writes a store).
    #[serde(rename = "noteDiscarded")]
    NoteDiscarded {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(default)]
        reason: Option<String>,
    },
    /// `noteGone` (an exported function that writes a store).
    #[serde(rename = "noteGone")]
    NoteGone {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `noteReady` (an exported function that writes a store).
    #[serde(rename = "noteReady")]
    NoteReady {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `openFolderMenu` (an exported function that writes a store).
    #[serde(rename = "openFolderMenu")]
    OpenFolderMenu {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "groupId")]
        group_id: String,
    },
    /// `openFromPage` (an exported function that writes a store).
    #[serde(rename = "openFromPage")]
    OpenFromPage {
        request: serde_json::Value,
        page: PageRef,
    },
    /// `openFromWindowlessPage` (an exported function that writes a store).
    #[serde(rename = "openFromWindowlessPage")]
    OpenFromWindowlessPage {
        asked: serde_json::Value,
        profile: String,
        #[serde(rename = "fromWindow", default)]
        from_window: Option<String>,
    },
    /// `openHistoryMenu` (an exported function that writes a store).
    #[serde(rename = "openHistoryMenu")]
    OpenHistoryMenu {
        #[serde(rename = "tabId")]
        tab_id: String,
        direction: Number,
    },
    /// `openTileFolderStripMenu` (an exported function that writes a store).
    #[serde(rename = "openTileFolderStripMenu")]
    OpenTileFolderStripMenu {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "folderId")]
        folder_id: String,
    },
    /// `pageEnded` (an exported function that writes a store).
    #[serde(rename = "pageEnded")]
    PageEnded {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `patchPage` (an exported function that writes a store).
    #[serde(rename = "patchPage")]
    PatchPage {
        #[serde(rename = "tabId")]
        tab_id: String,
        patch: PatchPagePatch,
    },
    /// `reloadRecentTabs` (an exported function that writes a store).
    #[serde(rename = "reloadRecentTabs")]
    ReloadRecentTabs { sys: serde_json::Value },
    /// `setMuted` (a store method).
    #[serde(rename = "setMuted")]
    SetMuted { ids: Vec<String>, muted: bool },
    /// `setPopover` (an exported function that writes a store).
    #[serde(rename = "setPopover")]
    SetPopover {
        #[serde(rename = "tabId")]
        tab_id: String,
        popover: Option<SetPopoverPopover>,
    },
    /// `setSiteMuted` (a store method).
    #[serde(rename = "setSiteMuted")]
    SetSiteMuted {
        #[serde(rename = "tabId")]
        tab_id: String,
        muted: bool,
    },
    /// `setSiteUnreadShown` (an exported function that writes a store).
    #[serde(rename = "setSiteUnreadShown")]
    SetSiteUnreadShown { url: String, shown: bool },
    /// `showUnreadCounts` (an exported function that writes a store).
    #[serde(rename = "showUnreadCounts")]
    ShowUnreadCounts { shown: bool },
    /// `sleepTab` (an exported function that writes a store).
    #[serde(rename = "sleepTab")]
    SleepTab {
        id: String,
        #[serde(default)]
        __1: Option<SleepTabV1>,
    },
    /// `startChromeTabs` (an exported function that writes a store).
    #[serde(rename = "startChromeTabs")]
    StartChromeTabs,
    /// `startPopoverEscape` (an exported function that writes a store).
    #[serde(rename = "startPopoverEscape")]
    StartPopoverEscape {
        #[serde(rename = "closesDownloads", default)]
        closes_downloads: Option<bool>,
    },
    /// `startTabLifecycle` (an exported function that writes a store).
    #[serde(rename = "startTabLifecycle")]
    StartTabLifecycle,
    /// `startTabPages` (an exported function that writes a store).
    #[serde(rename = "startTabPages")]
    StartTabPages,
    /// `stopKeepingSiteActive` (an exported function that writes a store).
    #[serde(rename = "stopKeepingSiteActive")]
    StopKeepingSiteActive { site: String },
    /// `stripChipEntered` (an exported function that writes a store). The callback parameters `measure` are not in the JSON.
    #[serde(rename = "stripChipEntered")]
    StripChipEntered {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `stripChipGone` (an exported function that writes a store).
    #[serde(rename = "stripChipGone")]
    StripChipGone {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `stripChipPressed` (an exported function that writes a store).
    #[serde(rename = "stripChipPressed")]
    StripChipPressed {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `stripPageOffstage` (an exported function that writes a store).
    #[serde(rename = "stripPageOffstage")]
    StripPageOffstage {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    /// `toggleHistoryMenu` (an exported function that writes a store).
    #[serde(rename = "toggleHistoryMenu")]
    ToggleHistoryMenu {
        #[serde(rename = "tabId")]
        tab_id: String,
        direction: Number,
    },
    /// `togglePin` (a store method).
    #[serde(rename = "togglePin")]
    TogglePin { id: String },
    /// `togglePopover` (an exported function that writes a store).
    #[serde(rename = "togglePopover")]
    TogglePopover {
        #[serde(rename = "tabId")]
        tab_id: String,
        popover: PagePopover,
    },
    /// `toggleSiteUnread` (an exported function that writes a store).
    #[serde(rename = "toggleSiteUnread")]
    ToggleSiteUnread {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `tuckGroup` (an exported function that writes a store).
    #[serde(rename = "tuckGroup")]
    TuckGroup {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "profileId")]
        profile_id: String,
        #[serde(rename = "groupId")]
        group_id: String,
    },
    /// `updateLive` (a store method).
    #[serde(rename = "updateLive")]
    UpdateLive { id: String, patch: UpdateLivePatch },
    /// `updateTab` (a store method).
    #[serde(rename = "updateTab")]
    UpdateTab {
        id: String,
        patch: UpdateTabPatch,
        #[serde(default)]
        live: Option<UpdateLivePatch>,
    },
    /// `wantsPage` (an exported function that writes a store).
    #[serde(rename = "wantsPage")]
    WantsPage { t: crate::browser::Tab },
    /// `withLivePage` (an exported function that writes a store). The callback parameters `act`, `mayAct` are not in the JSON.
    #[serde(rename = "withLivePage")]
    WithLivePage {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
}

/// Applies `action` to `app`. Slice 2.8 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.8")
}
