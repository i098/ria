// Ported from ardia apps/browser/src/state/groups/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `groups` (`apps/browser/src/state/groups`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.23 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GroupDraft {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(
        rename = "customTitle",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Box<GroupDraft>>>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DraftGroupOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collapsed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CreateGroupOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GroupTabsOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UpdateGroupPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collapsed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
    #[serde(
        rename = "shownTabIds",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub shown_tab_ids: Option<Vec<String>>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(rename = "closedGroups")]
    pub closed_groups: Vec<crate::browser::ClosedGroup>,
    #[serde(rename = "deletedGroups")]
    pub deleted_groups: Vec<crate::browser::ClosedGroup>,
    pub groups: std::collections::HashMap<String, crate::browser::TabGroup>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `addTabsToGroup` (a store method).
    #[serde(rename = "addTabsToGroup")]
    AddTabsToGroup {
        #[serde(rename = "groupId")]
        group_id: String,
        #[serde(rename = "tabIds")]
        tab_ids: Vec<String>,
    },
    /// `closeGroup` (a store method).
    #[serde(rename = "closeGroup")]
    CloseGroup {
        #[serde(rename = "groupId")]
        group_id: String,
    },
    /// `createGroup` (a store method).
    #[serde(rename = "createGroup")]
    CreateGroup {
        #[serde(rename = "tabIds")]
        tab_ids: Vec<String>,
        #[serde(default)]
        options: Option<CreateGroupOptions>,
    },
    /// `deleteGroup` (a store method).
    #[serde(rename = "deleteGroup")]
    DeleteGroup {
        #[serde(rename = "groupId")]
        group_id: String,
    },
    /// `duplicateGroup` (a store method).
    #[serde(rename = "duplicateGroup")]
    DuplicateGroup {
        #[serde(rename = "groupId")]
        group_id: String,
    },
    /// `groupOnto` (a store method).
    #[serde(rename = "groupOnto")]
    GroupOnto {
        ids: Vec<String>,
        #[serde(rename = "targetId")]
        target_id: String,
    },
    /// `groupTabs` (a store method).
    #[serde(rename = "groupTabs")]
    GroupTabs {
        ids: Vec<String>,
        #[serde(default)]
        options: Option<GroupTabsOptions>,
    },
    /// `moveGroup` (a store method).
    #[serde(rename = "moveGroup")]
    MoveGroup {
        #[serde(rename = "groupId")]
        group_id: String,
        placement: crate::pins::GroupPlacement,
    },
    /// `moveGroupToBookmarksBar` (a store method).
    #[serde(rename = "moveGroupToBookmarksBar")]
    MoveGroupToBookmarksBar {
        #[serde(rename = "groupId")]
        group_id: String,
    },
    /// `newNestedGroup` (a store method).
    #[serde(rename = "newNestedGroup")]
    NewNestedGroup {
        #[serde(rename = "parentId")]
        parent_id: String,
    },
    /// `newTabInGroup` (a store method).
    #[serde(rename = "newTabInGroup")]
    NewTabInGroup {
        #[serde(rename = "groupId")]
        group_id: String,
    },
    /// `removeTabsFromGroup` (a store method).
    #[serde(rename = "removeTabsFromGroup")]
    RemoveTabsFromGroup {
        #[serde(rename = "tabIds")]
        tab_ids: Vec<String>,
    },
    /// `ungroup` (a store method).
    #[serde(rename = "ungroup")]
    Ungroup {
        #[serde(rename = "groupId")]
        group_id: String,
    },
    /// `updateGroup` (a store method).
    #[serde(rename = "updateGroup")]
    UpdateGroup { id: String, patch: UpdateGroupPatch },
}

/// Applies `action` to `app`. Slice 2.23 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.23")
}
