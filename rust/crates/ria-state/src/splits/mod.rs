// Ported from ardia apps/browser/src/state/splits/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `splits` (`apps/browser/src/state/splits`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.14 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SplitSide {
    #[default]
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
    #[serde(rename = "bottom")]
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CreateSplitOrientation {
    #[default]
    #[serde(rename = "horizontal")]
    Horizontal,
    #[serde(rename = "vertical")]
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DropTabOnRowSide {
    #[default]
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OpenLinkInSplitAdoptId {
    Other0(String),
    Other1(String),
    Other2(String),
    Other3(String),
    Other4(String),
    Other5(String),
}

impl Default for OpenLinkInSplitAdoptId {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OpenPaneOptions {
    #[serde(rename = "tabId", default, skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(
        rename = "anchorTabId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub anchor_tab_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<OpenPaneOptionsSide>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    #[serde(rename = "adoptId", default, skip_serializing_if = "Option::is_none")]
    pub adopt_id: Option<OpenPaneOptionsAdoptId>,
    #[serde(rename = "inPlaceOf", default, skip_serializing_if = "Option::is_none")]
    pub in_place_of: Option<String>,
    #[serde(
        rename = "openedForSplit",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub opened_for_split: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum OpenPaneOptionsSide {
    #[default]
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
    #[serde(rename = "bottom")]
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OpenPaneOptionsAdoptId {
    Other0(String),
    Other1(String),
    Other2(String),
    Other3(String),
    Other4(String),
    Other5(String),
}

impl Default for OpenPaneOptionsAdoptId {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UpdateSplitPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sizes: Option<Vec<Number>>,
    #[serde(
        rename = "stackSizes",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub stack_sizes: Option<(Number, Number)>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub splits: std::collections::HashMap<String, crate::browser::SplitView>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `closePane` (an exported function that writes a store).
    #[serde(rename = "closePane")]
    ClosePane {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `closeSplitView` (an exported function that writes a store).
    #[serde(rename = "closeSplitView")]
    CloseSplitView {
        #[serde(rename = "splitId")]
        split_id: String,
    },
    /// `createSplit` (a store method).
    #[serde(rename = "createSplit")]
    CreateSplit {
        #[serde(rename = "tabIds")]
        tab_ids: Vec<String>,
        #[serde(default)]
        orientation: Option<CreateSplitOrientation>,
    },
    /// `dropTabIntoSplit` (an exported function that writes a store).
    #[serde(rename = "dropTabIntoSplit")]
    DropTabIntoSplit {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(rename = "targetTabId")]
        target_tab_id: String,
        side: SplitSide,
    },
    /// `dropTabOnRow` (an exported function that writes a store).
    #[serde(rename = "dropTabOnRow")]
    DropTabOnRow {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(rename = "rowTabId")]
        row_tab_id: String,
        side: DropTabOnRowSide,
    },
    /// `flipSplit` (a store method).
    #[serde(rename = "flipSplit")]
    FlipSplit {
        #[serde(rename = "splitId")]
        split_id: String,
    },
    /// `focusPane` (a store method).
    #[serde(rename = "focusPane")]
    FocusPane {
        #[serde(rename = "windowId")]
        window_id: String,
        delta: Number,
    },
    /// `focusSplitPane` (an exported function that writes a store).
    #[serde(rename = "focusSplitPane")]
    FocusSplitPane {
        #[serde(rename = "windowId")]
        window_id: String,
        delta: Number,
    },
    /// `movePane` (a store method).
    #[serde(rename = "movePane")]
    MovePane {
        #[serde(rename = "tabId")]
        tab_id: String,
        delta: Number,
    },
    /// `openInSplit` (an exported function that writes a store).
    #[serde(rename = "openInSplit")]
    OpenInSplit {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(default)]
        side: Option<SplitSide>,
    },
    /// `openLinkInSplit` (an exported function that writes a store).
    #[serde(rename = "openLinkInSplit")]
    OpenLinkInSplit {
        #[serde(rename = "sourceTabId")]
        source_tab_id: String,
        url: String,
        #[serde(rename = "adoptId", default)]
        adopt_id: Option<OpenLinkInSplitAdoptId>,
    },
    /// `openNewTabInSplit` (an exported function that writes a store).
    #[serde(rename = "openNewTabInSplit")]
    OpenNewTabInSplit {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `openSplitPane` (an exported function that writes a store).
    #[serde(rename = "openSplitPane")]
    OpenSplitPane {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(default)]
        options: Option<OpenPaneOptions>,
    },
    /// `removeTabFromSplit` (an exported function that writes a store).
    #[serde(rename = "removeTabFromSplit")]
    RemoveTabFromSplit {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `replaceSplitPane` (an exported function that writes a store).
    #[serde(rename = "replaceSplitPane")]
    ReplaceSplitPane {
        #[serde(rename = "paneTabId")]
        pane_tab_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `separateSplit` (a store method).
    #[serde(rename = "separateSplit")]
    SeparateSplit {
        #[serde(rename = "splitId")]
        split_id: String,
    },
    /// `showSplitMenu` (an exported function that writes a store).
    #[serde(rename = "showSplitMenu")]
    ShowSplitMenu {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `toggleSplitOrientation` (a store method).
    #[serde(rename = "toggleSplitOrientation")]
    ToggleSplitOrientation {
        #[serde(rename = "splitId")]
        split_id: String,
    },
    /// `toggleTabLayout` (a store method).
    #[serde(rename = "toggleTabLayout")]
    ToggleTabLayout {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `updateSplit` (an exported function that writes a store).
    #[serde(rename = "updateSplit")]
    UpdateSplit { id: String, patch: UpdateSplitPatch },
}

/// Applies `action` to `app`. Slice 2.14 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.14")
}
