// Ported from ardia apps/browser/src/state/profiles/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `profiles` (`apps/browser/src/state/profiles`, 17 files).
//!
//! `State` holds the data of the profile records and of the module-level stores of the folder
//! (`profiles.ts`, `profileLoading.ts`, `iconPicker.ts`, `createProfileRequest.ts`, `clearDataRequest.ts`).
//! `Action` has one variant for each exported store action, and `apply` is the reducer (`store.rs`).
//!
//! Changes from Ardia:
//! - The records `profiles` and `profileOrder` of the browser store are in `State`, so a profile action needs no `browser` state.
//! - A function of Ardia that awaits a native call or a timer is 2 actions (plan section 6.5): the action that asks for the
//!   call, and a result action (`LoadedProfilesResult`, `IdleProfilesLoaded`, `UnloadIdleResult`, `ProfileDataCleanupResult`,
//!   `RenamePrompted`, `ProfileMenuChosen`). A `setTimeout` is an action that `ria-app` sends when the time has run
//!   (`WakeShown` after 100 ms, `UnloadSettled` after 5 s, `ProfileDataCleanupTick` after 1.5 s); `PlatformCall` has no timer yet.
//! - `State` has the work fields of the module-level variables of Ardia (`Work`); they are not in the JSON.
//!   `Work::lazy_profiles` is the kill switch `lazyProfiles`, which `ria-app` sets from `KillSwitches` (it is on until set).
//! - `createProfileRequest.resolve` (a callback) is not in the state; `ria-app` keeps the promise resolver, and cancels the
//!   resolver of the request that `RequestCreateProfile` replaces.
//! - `openProfileIconPicker` takes the measured anchor of the view as `anchor_view` (`null`, or `{x, y, width, height}`).
//!   `closeProfileIconPickerIfShown` compares the picker by value, not by identity.
//! - `updateProfile` and `setProfileLook` read a patch key with a `null` value as a present key (a double `Option`), so
//!   `{icon: null}` clears the icon and `{theme: null}` clears the theme. The patch keys that Ardia sets to `undefined` are
//!   `None` inside `Some`.
//! - `createProfile` takes the clock as `now`, `checkIdleProfiles` takes `now` (`None` is 0).
//! - `browser::Profile` (slice 2.4 skeleton) reads a JSON `null` of `seenTheme` or `passwordManager` as absent, so the recorded cases
//!   do not set `passwordManager` to `null`; the replay test restores `seenTheme: null` on the seed.
//! - Not ported: the React hooks of `hooks.ts` (the UI reads `State`), `looks.ts` (a registry of UI callbacks), the
//!   `subscribeTo` wiring of `profileLoading.ts`, `profileUnloading.ts` and `profileData.ts` (`ria-app` sends the actions),
//!   the telemetry listeners of `watchProfiles`, and the `__DEV__` debug hook.

use std::collections::{BTreeSet, HashMap, VecDeque};

use serde::{Deserialize, Deserializer, Serialize};

use crate::browser::{Profile, ProfileColor, ProfilePresetThemes, ProfileSeenColor};
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

mod pending_siblings;
#[cfg(test)]
mod replay_tests;
mod store;

pub use store::{
    IdleProfiles, LEGACY_SWITCHER_ICONS_KEY, ProfileMenuState, closed_by_unload, data_groups,
    data_id_of, patched_profile, profile_menu_items, profile_names, profile_switcher_shown,
    profile_unload_delay, profiles_keeping_legacy_dots, sharing_profiles, switcher_icon_of,
    unused_profile_color,
};

pub use store::DEFAULT_PROFILE_ID;

/// A present key whose value may be `null` (`Some(None)`); an absent key is `None`.
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<Option<T>>, D::Error> {
    Option::<T>::deserialize(d).map(Some)
}

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

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CreateProfileOptions {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<ProfileColor>,
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

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SetProfileLookLook {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<ProfileColor>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub theme: Option<Option<serde_json::Value>>,
    #[serde(
        rename = "presetThemes",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub preset_themes: Option<ProfilePresetThemes>,
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
    pub id: ProfileColor,
    pub title: String,
    pub swatch: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UpdateProfilePatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub icon: Option<Option<String>>,
    #[serde(
        rename = "pinnedCollapsed",
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub pinned_collapsed: Option<Option<bool>>,
    #[serde(
        rename = "shareTopApps",
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub share_top_apps: Option<Option<bool>>,
    #[serde(
        rename = "syncTabs",
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_tabs: Option<Option<bool>>,
    #[serde(
        rename = "switcherDot",
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub switcher_dot: Option<Option<bool>>,
    #[serde(
        rename = "passwordManager",
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub password_manager: Option<Option<String>>,
}

/// The module-level variables of the Ardia files that are not store data (not in the JSON).
#[derive(Debug, Clone, PartialEq)]
pub struct Work {
    /// `switchOn("lazyProfiles")`.
    pub lazy_profiles: bool,
    /// `profileData.ts`: `failed`, `running`, and the engine that waits for the close delay.
    pub cleanup_failed: BTreeSet<String>,
    pub cleanup_running: bool,
    pub cleanup_engine: Option<String>,
    /// `profileUnloading.ts`.
    pub idle: IdleProfiles,
    pub checking: bool,
    pub check_now: i64,
    pub check_queue: VecDeque<String>,
    pub unloading_now: BTreeSet<String>,
    pub unloaded_here: BTreeSet<String>,
    pub slept_by_unload: BTreeSet<String>,
    pub unload_settled: bool,
}

impl Default for Work {
    fn default() -> Self {
        Self {
            lazy_profiles: true,
            cleanup_failed: BTreeSet::new(),
            cleanup_running: false,
            cleanup_engine: None,
            idle: IdleProfiles::default(),
            checking: false,
            check_now: 0,
            check_queue: VecDeque::new(),
            unloading_now: BTreeSet::new(),
            unloaded_here: BTreeSet::new(),
            slept_by_unload: BTreeSet::new(),
            unload_settled: false,
        }
    }
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct State {
    pub deferred: bool,
    pub opened: HashMap<String, bool>,
    #[serde(rename = "orphanedProfileData")]
    pub orphaned_profile_data: Vec<String>,
    pub picker: Option<ProfileIconPicker>,
    #[serde(rename = "profileOrder")]
    pub profile_order: Vec<String>,
    pub profiles: HashMap<String, Profile>,
    pub request: Option<CreateProfileRequest>,
    pub sheet: Option<CreateProfileRequest>,
    pub unloaded: HashMap<String, bool>,
    #[serde(rename = "windowId")]
    pub window_id: Option<String>,
    #[serde(skip)]
    pub work: Work,
}

/// `DEFAULT_PROFILE` (`profiles.ts:23`).
fn default_profile() -> Profile {
    Profile {
        id: DEFAULT_PROFILE_ID.into(),
        name: "Personal".into(),
        color: ProfileColor::Plum,
        icon: None,
        created_at: Number(0.0),
        preset_themes: Some(ProfilePresetThemes::default()),
        seen_theme: Some(serde_json::Value::Null),
        seen_color: Some(ProfileSeenColor::Plum),
        ..Profile::default()
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            deferred: false,
            opened: HashMap::new(),
            orphaned_profile_data: Vec::new(),
            picker: None,
            profile_order: vec![DEFAULT_PROFILE_ID.into()],
            profiles: HashMap::from([(DEFAULT_PROFILE_ID.into(), default_profile())]),
            request: None,
            sheet: None,
            unloaded: HashMap::new(),
            window_id: None,
            work: Work::default(),
        }
    }
}

/// One variant for each exported store action, and the result and timer actions of the awaited ones.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `applyUnload` (`profileUnloading.ts:44`).
    #[serde(rename = "applyUnload")]
    ApplyUnload {
        profile: String,
        result: UnloadResult,
    },
    /// `checkIdleProfiles` (`profileUnloading.ts:93`).
    #[serde(rename = "checkIdleProfiles")]
    CheckIdleProfiles {
        #[serde(default)]
        now: Option<Number>,
    },
    /// `chooseIcon` (`actions.ts:9`).
    #[serde(rename = "chooseIcon")]
    ChooseIcon {
        #[serde(rename = "profileId")]
        profile_id: String,
        icon: Option<String>,
    },
    /// `choosePresetColor` (`themeActions.ts:23`). The callback `adopt` is not in the JSON.
    #[serde(rename = "choosePresetColor")]
    ChoosePresetColor {
        #[serde(rename = "profileId")]
        profile_id: String,
        color: ProfileColor,
    },
    /// `chooseProfileIcon` (`iconPicker.ts:36`).
    #[serde(rename = "chooseProfileIcon")]
    ChooseProfileIcon {
        arg0: ProfileIconPicker,
        icon: Option<String>,
    },
    /// `closeProfileIconPicker` (`iconPicker.ts:27`).
    #[serde(rename = "closeProfileIconPicker")]
    CloseProfileIconPicker,
    /// `closeProfileIconPickerIfShown` (`iconPicker.ts:29`).
    #[serde(rename = "closeProfileIconPickerIfShown")]
    CloseProfileIconPickerIfShown { picker: ProfileIconPicker },
    /// `closeProfileIconPickerIn` (`iconPicker.ts:33`).
    #[serde(rename = "closeProfileIconPickerIn")]
    CloseProfileIconPickerIn {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `commitProfileName` (`actions.ts:13`).
    #[serde(rename = "commitProfileName")]
    CommitProfileName {
        #[serde(rename = "profileId")]
        profile_id: String,
        text: String,
    },
    /// `createProfile` (a store method, `profiles.ts:62`).
    #[serde(rename = "createProfile")]
    CreateProfile {
        options: CreateProfileOptions,
        #[serde(default)]
        now: Option<Number>,
    },
    /// `deleteProfile` (a store method, `profiles.ts:92`).
    #[serde(rename = "deleteProfile")]
    DeleteProfile { id: String },
    /// `endCreateProfileRequest` (`createProfileRequest.ts:26`).
    #[serde(rename = "endCreateProfileRequest")]
    EndCreateProfileRequest {
        request: CreateProfileRequest,
        id: Option<String>,
    },
    /// `importArcTheme` (`arcThemes.ts:19`).
    #[serde(rename = "importArcTheme")]
    ImportArcTheme {
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    /// `markProfileUnloaded` (`profileLoading.ts:46`).
    #[serde(rename = "markProfileUnloaded")]
    MarkProfileUnloaded { engine: String },
    /// `openProfile` (`profileLoading.ts:40`).
    #[serde(rename = "openProfile")]
    OpenProfile { engine: String },
    /// `openProfileIconPicker` (`iconPicker.ts:21`); `anchor_view` is the measured view, `null` or `{x, y, width, height}`.
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
    /// `profileDataDeleted` (a store method, `profiles.ts:150`).
    #[serde(rename = "profileDataDeleted")]
    ProfileDataDeleted {
        #[serde(rename = "engineId")]
        engine_id: String,
    },
    /// `renameProfile` (`profileMenu.ts:49`): asks for the prompt.
    #[serde(rename = "renameProfile")]
    RenameProfile {
        #[serde(rename = "profileId")]
        profile_id: String,
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `reorderProfiles` (a store method, `profiles.ts:134`).
    #[serde(rename = "reorderProfiles")]
    ReorderProfiles { ids: Vec<String> },
    /// `requestClearData` (`clearDataRequest.ts:5`).
    #[serde(rename = "requestClearData")]
    RequestClearData {
        #[serde(rename = "windowId")]
        window_id: Option<String>,
    },
    /// `requestCreateProfile` (`createProfileRequest.ts:13`).
    #[serde(rename = "requestCreateProfile")]
    RequestCreateProfile {
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
        #[serde(default)]
        preset: Option<CreateProfilePreset>,
    },
    /// `resetPresetTheme` (`themeActions.ts:31`). The callback `adopt` is not in the JSON.
    #[serde(rename = "resetPresetTheme")]
    ResetPresetTheme {
        #[serde(rename = "profileId")]
        profile_id: String,
    },
    /// `setDefaultProfile` (a store method, `profiles.ts:140`).
    #[serde(rename = "setDefaultProfile")]
    SetDefaultProfile { id: String },
    /// `setProfileLook` (a store method, `profiles.ts:77`).
    #[serde(rename = "setProfileLook")]
    SetProfileLook {
        id: String,
        look: SetProfileLookLook,
    },
    /// `setProfileTheme` (`themeActions.ts:9`). The callback `adopt` is not in the JSON.
    #[serde(rename = "setProfileTheme")]
    SetProfileTheme {
        #[serde(rename = "profileId")]
        profile_id: String,
        theme: Option<serde_json::Value>,
    },
    /// `setShareTopApps` (`actions.ts:11`).
    #[serde(rename = "setShareTopApps")]
    SetShareTopApps {
        #[serde(rename = "profileId")]
        profile_id: String,
        share: bool,
    },
    /// `showProfileMenu` (`profileMenu.ts:57`): asks for the menu.
    #[serde(rename = "showProfileMenu")]
    ShowProfileMenu {
        #[serde(rename = "profileId")]
        profile_id: String,
        arg1: ShowProfileMenuArg1,
    },
    /// `startProfileDataCleanup` (`profileData.ts:49`).
    #[serde(rename = "startProfileDataCleanup")]
    StartProfileDataCleanup,
    /// `startProfileLoading` (`profileLoading.ts:76`).
    #[serde(rename = "startProfileLoading")]
    StartProfileLoading,
    /// `startProfileUnloading` (`profileUnloading.ts:132`): not a store action in Rust. `ria-app` sends `CheckIdleProfiles`
    /// each minute while `profileUnloadDelay(settings)` is not `None`; this action only clears the idle book when it is `None`.
    #[serde(rename = "startProfileUnloading")]
    StartProfileUnloading,
    /// `takeCreateProfileSheet` (`createProfileRequest.ts:30`).
    #[serde(rename = "takeCreateProfileSheet")]
    TakeCreateProfileSheet,
    /// `updateProfile` (a store method, `profiles.ts:73`).
    #[serde(rename = "updateProfile")]
    UpdateProfile {
        id: String,
        patch: UpdateProfilePatch,
    },

    /// The result of `loadedProfiles()` in `startProfileLoading`; `None` is a rejection.
    #[serde(rename = "loadedProfilesResult")]
    LoadedProfilesResult { engines: Option<Vec<String>> },
    /// The result of `loadedProfiles()` in `checkIdleProfiles`; `None` is a rejection.
    #[serde(rename = "idleProfilesLoaded")]
    IdleProfilesLoaded { loaded: Option<Vec<String>> },
    /// The result of `unloadIdleProfile(profile)`; `None` is a rejection.
    #[serde(rename = "unloadIdleResult")]
    UnloadIdleResult {
        profile: String,
        result: Option<UnloadResult>,
    },
    /// The `SETTLE_MS` timer of `unload` ran out.
    #[serde(rename = "unloadSettled")]
    UnloadSettled { profile: String },
    /// The `REMOUNT_MS` timer of `applyUnload` ran out (`wakeShown`).
    #[serde(rename = "wakeShown")]
    WakeShown { ids: Vec<String> },
    /// `drain()` of `profileData.ts`, started by the `orphanedProfileData` subscription.
    #[serde(rename = "drainProfileData")]
    DrainProfileData,
    /// The `CLOSE_DELAY_MS` timer of `drain` ran out.
    #[serde(rename = "profileDataCleanupTick")]
    ProfileDataCleanupTick,
    /// The result of `deleteProfileData(engine)`: the list of what remains; a rejection is the list of its text.
    #[serde(rename = "profileDataCleanupResult")]
    ProfileDataCleanupResult {
        engine: String,
        remaining: Vec<String>,
    },
    /// The result of the prompt of `renameProfile`; `None` is a cancel.
    #[serde(rename = "renamePrompted")]
    RenamePrompted {
        #[serde(rename = "profileId")]
        profile_id: String,
        name: Option<String>,
    },
    /// The result of the menu of `showProfileMenu`: `None`, an item id, or `{color}`.
    #[serde(rename = "profileMenuChosen")]
    ProfileMenuChosen {
        #[serde(rename = "profileId")]
        profile_id: String,
        arg1: ShowProfileMenuArg1,
        choice: Option<serde_json::Value>,
    },
}

/// The result of `unloadIdleProfile` (`platform/profiles.ts`): `status`, `heldBy`, and the tabs it put to sleep.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UnloadResult {
    #[serde(default)]
    pub status: String,
    #[serde(rename = "heldBy", default, skip_serializing_if = "Option::is_none")]
    pub held_by: Option<String>,
    #[serde(default)]
    pub tabs: Vec<String>,
}

/// Applies `action` to `app`.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    store::apply(app, action, fx);
}
