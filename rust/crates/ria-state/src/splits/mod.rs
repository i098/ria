// Ported from ardia apps/browser/src/state/splits/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `splits` (`apps/browser/src/state/splits`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.14 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

pub mod actions;
pub mod hooks;
pub mod lanes;
mod pending_siblings;
#[allow(clippy::module_inception)]
pub mod splits;

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

/// Applies `action` to `app` (`actions.ts` and the store methods of `splits.ts`).
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    use crate::browser::SplitViewOrientation as Orientation;
    match action {
        Action::ClosePane { tab_id } => actions::close_pane(app, &tab_id, fx),
        Action::CloseSplitView { split_id } => actions::close_split_view(app, &split_id, fx),
        Action::CreateSplit {
            tab_ids,
            orientation,
        } => {
            let orientation = match orientation.unwrap_or_default() {
                CreateSplitOrientation::Horizontal => Orientation::Horizontal,
                CreateSplitOrientation::Vertical => Orientation::Vertical,
            };
            splits::create_split(app, &tab_ids, orientation);
        }
        Action::DropTabIntoSplit {
            tab_id,
            target_tab_id,
            side,
        } => {
            actions::drop_tab_into_split(app, &tab_id, &target_tab_id, side, fx);
        }
        Action::DropTabOnRow {
            tab_id,
            row_tab_id,
            side,
        } => {
            actions::drop_tab_on_row(app, &tab_id, &row_tab_id, side, fx);
        }
        Action::FlipSplit { split_id } => splits::flip_split(app, &split_id),
        Action::FocusPane { window_id, delta } => {
            if let Some(pane) = splits::pane_to_focus(app, &window_id, delta.0 as i64) {
                crate::tabs::apply(app, crate::tabs::Action::Activate { id: pane }, fx);
            }
        }
        Action::FocusSplitPane { window_id, delta } => {
            actions::focus_split_pane(app, &window_id, delta.0 as i64, fx);
        }
        Action::MovePane { tab_id, delta } => splits::move_pane(app, &tab_id, delta.0 as i64),
        Action::OpenInSplit { tab_id, side } => {
            actions::open_in_split(app, &tab_id, side.unwrap_or(SplitSide::Right), fx);
        }
        Action::OpenLinkInSplit {
            source_tab_id,
            url,
            adopt_id,
        } => {
            actions::open_link_in_split(app, &source_tab_id, &url, adopt_id.as_ref(), fx);
        }
        Action::OpenNewTabInSplit { window_id } => {
            actions::open_new_tab_in_split(app, &window_id, fx)
        }
        Action::OpenSplitPane { window_id, options } => {
            actions::open_split_pane(app, &window_id, &options.unwrap_or_default(), fx);
        }
        Action::RemoveTabFromSplit { tab_id } => splits::remove_tab_from_split(app, &tab_id),
        Action::ReplaceSplitPane {
            pane_tab_id,
            tab_id,
        } => {
            if let Some(id) = splits::replace_split_pane(app, &pane_tab_id, &tab_id) {
                crate::tabs::apply(app, crate::tabs::Action::Activate { id }, fx);
            }
        }
        Action::SeparateSplit { split_id } => splits::separate_split(app, &split_id),
        Action::ShowSplitMenu { tab_id } => pending_siblings::show_split_menu(&tab_id),
        Action::ToggleSplitOrientation { split_id } => {
            splits::toggle_split_orientation(app, &split_id)
        }
        Action::ToggleTabLayout { window_id } => splits::toggle_tab_layout(app, &window_id),
        Action::UpdateSplit { id, patch } => splits::update_split(app, &id, &patch),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::{SplitSlot, SplitSlotVariant0, SplitView, SplitViewOrientation};

    fn slot(id: &str) -> SplitSlot {
        SplitSlot::Other0(SplitSlotVariant0 {
            size: Number(0.5),
            tab_id: id.into(),
        })
    }

    #[test]
    fn flip_and_toggle_change_the_split_in_place() {
        let mut app = AppState::default();
        let split = SplitView {
            id: "s1".into(),
            window_id: "w1".into(),
            orientation: SplitViewOrientation::Horizontal,
            slots: vec![slot("a"), slot("b")],
        };
        app.get_mut::<State>().splits.insert("s1".into(), split);
        let mut fx = Effects::default();
        apply(
            &mut app,
            Action::FlipSplit {
                split_id: "s1".into(),
            },
            &mut fx,
        );
        apply(
            &mut app,
            Action::ToggleSplitOrientation {
                split_id: "s1".into(),
            },
            &mut fx,
        );
        let s = &app.get::<State>().splits["s1"];
        assert_eq!(s.slots, vec![slot("b"), slot("a")]);
        assert_eq!(s.orientation, SplitViewOrientation::Vertical);
    }
}
