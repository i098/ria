// Ported from ardia apps/browser/src/state/profiles/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `profiles` (`apps/browser/src/state/profiles`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.12 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProfileIconPicker {
    #[serde(rename = "profileId")]
    pub profile_id: String,
    #[serde(rename = "windowId")]
    pub window_id: String,
    pub place: ProfileMenuPlace,
    pub anchor: Option<IconPickerAnchor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ProfileMenuPlace {
    #[default]
    #[serde(rename = "settings")]
    Settings,
    #[serde(rename = "sidebar")]
    Sidebar,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct IconPickerAnchor {
    pub x: Number,
    pub y: Number,
    pub width: Number,
    pub height: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CreateProfileRequest {
    #[serde(rename = "windowId")]
    pub window_id: Option<String>,
    pub preset: CreateProfilePreset,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CreateProfilePreset {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "shareWith", default, skip_serializing_if = "Option::is_none")]
    pub share_with: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ChoosePresetColorColor {
    #[default]
    #[serde(rename = "plum")]
    Plum,
    #[serde(rename = "blue")]
    Blue,
    #[serde(rename = "purple")]
    Purple,
    #[serde(rename = "pink")]
    Pink,
    #[serde(rename = "red")]
    Red,
    #[serde(rename = "orange")]
    Orange,
    #[serde(rename = "yellow")]
    Yellow,
    #[serde(rename = "green")]
    Green,
    #[serde(rename = "neutral")]
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CreateProfileOptions {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<CreateProfileOptionsColor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(rename = "shareWith", default, skip_serializing_if = "Option::is_none")]
    pub share_with: Option<String>,
    #[serde(
        rename = "shareTopApps",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub share_top_apps: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CreateProfileOptionsColor {
    #[default]
    #[serde(rename = "plum")]
    Plum,
    #[serde(rename = "blue")]
    Blue,
    #[serde(rename = "purple")]
    Purple,
    #[serde(rename = "pink")]
    Pink,
    #[serde(rename = "red")]
    Red,
    #[serde(rename = "orange")]
    Orange,
    #[serde(rename = "yellow")]
    Yellow,
    #[serde(rename = "green")]
    Green,
    #[serde(rename = "neutral")]
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SetProfileLookLook {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<SetProfileLookLookColor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<serde_json::Value>,
    #[serde(
        rename = "presetThemes",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub preset_themes: Option<crate::browser::ProfilePresetThemes>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetProfileLookLookColor {
    #[default]
    #[serde(rename = "plum")]
    Plum,
    #[serde(rename = "blue")]
    Blue,
    #[serde(rename = "purple")]
    Purple,
    #[serde(rename = "pink")]
    Pink,
    #[serde(rename = "red")]
    Red,
    #[serde(rename = "orange")]
    Orange,
    #[serde(rename = "yellow")]
    Yellow,
    #[serde(rename = "green")]
    Green,
    #[serde(rename = "neutral")]
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ShowProfileMenuArg1 {
    pub place: ProfileMenuPlace,
    #[serde(rename = "windowId")]
    pub window_id: String,
    pub anchor: serde_json::Value,
    pub colors: Vec<ProfileColorChoice>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProfileColorChoice {
    pub id: ProfileColorChoiceId,
    pub title: String,
    pub swatch: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ProfileColorChoiceId {
    #[default]
    #[serde(rename = "plum")]
    Plum,
    #[serde(rename = "blue")]
    Blue,
    #[serde(rename = "purple")]
    Purple,
    #[serde(rename = "pink")]
    Pink,
    #[serde(rename = "red")]
    Red,
    #[serde(rename = "orange")]
    Orange,
    #[serde(rename = "yellow")]
    Yellow,
    #[serde(rename = "green")]
    Green,
    #[serde(rename = "neutral")]
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UpdateProfilePatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(
        rename = "pinnedCollapsed",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub pinned_collapsed: Option<bool>,
    #[serde(
        rename = "shareTopApps",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub share_top_apps: Option<bool>,
    #[serde(rename = "syncTabs", default, skip_serializing_if = "Option::is_none")]
    pub sync_tabs: Option<bool>,
    #[serde(
        rename = "switcherDot",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub switcher_dot: Option<bool>,
    #[serde(
        rename = "passwordManager",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub password_manager: Option<String>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub deferred: bool,
    pub opened: std::collections::HashMap<String, bool>,
    #[serde(rename = "orphanedProfileData")]
    pub orphaned_profile_data: Vec<String>,
    pub picker: Option<ProfileIconPicker>,
    #[serde(rename = "profileOrder")]
    pub profile_order: Vec<String>,
    pub profiles: std::collections::HashMap<String, crate::browser::Profile>,
    pub request: Option<CreateProfileRequest>,
    pub sheet: Option<CreateProfileRequest>,
    pub unloaded: std::collections::HashMap<String, bool>,
    #[serde(rename = "windowId")]
    pub window_id: Option<String>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `applyUnload` (an exported function that writes a store).
    #[serde(rename = "applyUnload")]
    ApplyUnload {
        profile: String,
        result: serde_json::Value,
    },
    /// `checkIdleProfiles` (an exported function that writes a store).
    #[serde(rename = "checkIdleProfiles")]
    CheckIdleProfiles {
        #[serde(default)]
        now: Option<Number>,
    },
    /// `chooseIcon` (an exported function that writes a store).
    #[serde(rename = "chooseIcon")]
    ChooseIcon {
        #[serde(rename = "profileId")]
        profile_id: String,
        icon: Option<String>,
    },
    /// `choosePresetColor` (an exported function that writes a store). The callback parameters `adopt` are not in the JSON.
    #[serde(rename = "choosePresetColor")]
    ChoosePresetColor {
        #[serde(rename = "profileId")]
        profile_id: String,
        color: ChoosePresetColorColor,
    },
    /// `chooseProfileIcon` (an exported function that writes a store).
    #[serde(rename = "chooseProfileIcon")]
    ChooseProfileIcon {
        arg0: ProfileIconPicker,
        icon: Option<String>,
    },
    /// `closeProfileIconPicker` (an exported function that writes a store).
    #[serde(rename = "closeProfileIconPicker")]
    CloseProfileIconPicker,
    /// `closeProfileIconPickerIfShown` (an exported function that writes a store).
    #[serde(rename = "closeProfileIconPickerIfShown")]
    CloseProfileIconPickerIfShown { picker: ProfileIconPicker },
    /// `closeProfileIconPickerIn` (an exported function that writes a store).
    #[serde(rename = "closeProfileIconPickerIn")]
    CloseProfileIconPickerIn {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `commitProfileName` (an exported function that writes a store).
    #[serde(rename = "commitProfileName")]
    CommitProfileName {
        #[serde(rename = "profileId")]
        profile_id: String,
        text: String,
    },
    /// `createProfile` (an exported function that writes a store).
    #[serde(rename = "createProfile")]
    CreateProfile { options: CreateProfileOptions },
    /// `deleteProfile` (a store method).
    #[serde(rename = "deleteProfile")]
    DeleteProfile { id: String },
    /// `endCreateProfileRequest` (an exported function that writes a store).
    #[serde(rename = "endCreateProfileRequest")]
    EndCreateProfileRequest {
        request: CreateProfileRequest,
        id: Option<String>,
    },
    /// `importArcTheme` (an exported function that writes a store).
    #[serde(rename = "importArcTheme")]
    ImportArcTheme {
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    /// `markProfileUnloaded` (an exported function that writes a store).
    #[serde(rename = "markProfileUnloaded")]
    MarkProfileUnloaded { engine: String },
    /// `openProfile` (an exported function that writes a store).
    #[serde(rename = "openProfile")]
    OpenProfile { engine: String },
    /// `openProfileIconPicker` (an exported function that writes a store).
    #[serde(rename = "openProfileIconPicker")]
    OpenProfileIconPicker {
        #[serde(rename = "profileId")]
        profile_id: String,
        #[serde(rename = "windowId")]
        window_id: String,
        place: ProfileMenuPlace,
        #[serde(rename = "anchorView")]
        anchor_view: serde_json::Value,
    },
    /// `profileDataDeleted` (a store method).
    #[serde(rename = "profileDataDeleted")]
    ProfileDataDeleted {
        #[serde(rename = "engineId")]
        engine_id: String,
    },
    /// `renameProfile` (an exported function that writes a store).
    #[serde(rename = "renameProfile")]
    RenameProfile {
        #[serde(rename = "profileId")]
        profile_id: String,
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `reorderProfiles` (an exported function that writes a store).
    #[serde(rename = "reorderProfiles")]
    ReorderProfiles { ids: Vec<String> },
    /// `requestClearData` (an exported function that writes a store).
    #[serde(rename = "requestClearData")]
    RequestClearData {
        #[serde(rename = "windowId")]
        window_id: Option<String>,
    },
    /// `requestCreateProfile` (an exported function that writes a store).
    #[serde(rename = "requestCreateProfile")]
    RequestCreateProfile {
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
        #[serde(default)]
        preset: Option<CreateProfilePreset>,
    },
    /// `resetPresetTheme` (an exported function that writes a store). The callback parameters `adopt` are not in the JSON.
    #[serde(rename = "resetPresetTheme")]
    ResetPresetTheme {
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    /// `setDefaultProfile` (an exported function that writes a store).
    #[serde(rename = "setDefaultProfile")]
    SetDefaultProfile { id: String },
    /// `setProfileLook` (a store method).
    #[serde(rename = "setProfileLook")]
    SetProfileLook {
        id: String,
        look: SetProfileLookLook,
    },
    /// `setProfileTheme` (an exported function that writes a store). The callback parameters `adopt` are not in the JSON.
    #[serde(rename = "setProfileTheme")]
    SetProfileTheme {
        #[serde(rename = "profileId")]
        profile_id: String,
        theme: serde_json::Value,
    },
    /// `setShareTopApps` (an exported function that writes a store).
    #[serde(rename = "setShareTopApps")]
    SetShareTopApps {
        #[serde(rename = "profileId")]
        profile_id: String,
        share: bool,
    },
    /// `showProfileMenu` (an exported function that writes a store).
    #[serde(rename = "showProfileMenu")]
    ShowProfileMenu {
        #[serde(rename = "profileId")]
        profile_id: String,
        arg1: ShowProfileMenuArg1,
    },
    /// `startProfileDataCleanup` (an exported function that writes a store).
    #[serde(rename = "startProfileDataCleanup")]
    StartProfileDataCleanup,
    /// `startProfileLoading` (an exported function that writes a store).
    #[serde(rename = "startProfileLoading")]
    StartProfileLoading,
    /// `startProfileUnloading` (an exported function that writes a store).
    #[serde(rename = "startProfileUnloading")]
    StartProfileUnloading,
    /// `takeCreateProfileSheet` (an exported function that writes a store).
    #[serde(rename = "takeCreateProfileSheet")]
    TakeCreateProfileSheet,
    /// `updateProfile` (an exported function that writes a store).
    #[serde(rename = "updateProfile")]
    UpdateProfile {
        id: String,
        patch: UpdateProfilePatch,
    },
}

/// Applies `action` to `app`. Slice 2.12 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.12")
}
