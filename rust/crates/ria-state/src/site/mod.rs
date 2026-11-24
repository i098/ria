// Ported from ardia apps/browser/src/state/site/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `site` (`apps/browser/src/state/site`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.20 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PasswordManagerChoice {
    String(String),
    Other2(String),
}

impl Default for PasswordManagerChoice {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HiddenElement {
    pub filter: String,
    pub host: String,
    pub selector: String,
    pub session: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RevertPageArg1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiet: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickModifiers {
    #[serde(rename = "metaKey", default, skip_serializing_if = "Option::is_none")]
    pub meta_key: Option<bool>,
    #[serde(rename = "shiftKey", default, skip_serializing_if = "Option::is_none")]
    pub shift_key: Option<bool>,
    #[serde(rename = "altKey", default, skip_serializing_if = "Option::is_none")]
    pub alt_key: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub middle: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SetSiteBlockingTab {
    pub id: String,
    #[serde(rename = "windowId")]
    pub window_id: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SetSitePermissionPage {
    #[serde(rename = "tabId")]
    pub tab_id: String,
    pub profile: String,
    pub origin: String,
    pub engine: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SiteSound {
    #[serde(rename = "siteMuted")]
    pub site_muted: bool,
    #[serde(rename = "soundBlocked")]
    pub sound_blocked: bool,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(rename = "byTab")]
    pub by_tab: std::collections::HashMap<String, serde_json::Value>,
    pub changes: Number,
    pub session: std::collections::HashMap<String, Vec<String>>,
    pub shown: std::collections::HashMap<String, bool>,
    pub unlocked: std::collections::HashMap<String, bool>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `acceptVerificationCode` (an exported function that writes a store).
    #[serde(rename = "acceptVerificationCode")]
    AcceptVerificationCode {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `answerAutofillPrompt` (an exported function that writes a store).
    #[serde(rename = "answerAutofillPrompt")]
    AnswerAutofillPrompt {
        #[serde(rename = "tabId")]
        tab_id: String,
        action: serde_json::Value,
        id: Number,
    },
    /// `answerAutofillPromptConfirmed` (an exported function that writes a store).
    #[serde(rename = "answerAutofillPromptConfirmed")]
    AnswerAutofillPromptConfirmed {
        #[serde(rename = "tabId")]
        tab_id: String,
        id: Number,
        action: serde_json::Value,
    },
    /// `answerExternalApp` (an exported function that writes a store).
    #[serde(rename = "answerExternalApp")]
    AnswerExternalApp {
        #[serde(rename = "tabId")]
        tab_id: String,
        open: bool,
        #[serde(default)]
        remember: Option<bool>,
    },
    /// `answerPasswordPrompt` (an exported function that writes a store).
    #[serde(rename = "answerPasswordPrompt")]
    AnswerPasswordPrompt {
        #[serde(rename = "tabId")]
        tab_id: String,
        action: serde_json::Value,
        #[serde(default)]
        username: Option<String>,
        #[serde(default)]
        shown: Option<serde_json::Value>,
    },
    /// `answerPermission` (an exported function that writes a store).
    #[serde(rename = "answerPermission")]
    AnswerPermission {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(rename = "requestId")]
        request_id: String,
        result: serde_json::Value,
    },
    /// `choosePasswordManager` (an exported function that writes a store).
    #[serde(rename = "choosePasswordManager")]
    ChoosePasswordManager {
        #[serde(rename = "profileId")]
        profile_id: String,
        manager: PasswordManagerChoice,
    },
    /// `closeVerificationCodePrompts` (an exported function that writes a store).
    #[serde(rename = "closeVerificationCodePrompts")]
    CloseVerificationCodePrompts,
    /// `copyLinkToSelection` (an exported function that writes a store).
    #[serde(rename = "copyLinkToSelection")]
    CopyLinkToSelection {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `copyPageUrl` (an exported function that writes a store).
    #[serde(rename = "copyPageUrl")]
    CopyPageUrl {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(rename = "confirmedUrl", default)]
        confirmed_url: Option<String>,
    },
    /// `copySelectedTabUrls` (an exported function that writes a store).
    #[serde(rename = "copySelectedTabUrls")]
    CopySelectedTabUrls {
        s: crate::browser::BrowserState,
        #[serde(rename = "windowId")]
        window_id: String,
        markdown: bool,
    },
    /// `declineVerificationCode` (an exported function that writes a store).
    #[serde(rename = "declineVerificationCode")]
    DeclineVerificationCode {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `dismissPermissions` (an exported function that writes a store).
    #[serde(rename = "dismissPermissions")]
    DismissPermissions {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `dismissZoomPopover` (an exported function that writes a store).
    #[serde(rename = "dismissZoomPopover")]
    DismissZoomPopover {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `followMode` (an exported function that writes a store).
    #[serde(rename = "followMode")]
    FollowMode {
        profile: String,
        mode: PasswordManagerChoice,
    },
    /// `forgetVerificationCodes` (an exported function that writes a store).
    #[serde(rename = "forgetVerificationCodes")]
    ForgetVerificationCodes {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `handleWebNotificationResponse` (an exported function that writes a store).
    #[serde(rename = "handleWebNotificationResponse")]
    HandleWebNotificationResponse { arg0: serde_json::Value },
    /// `hoverZoomPopover` (an exported function that writes a store).
    #[serde(rename = "hoverZoomPopover")]
    HoverZoomPopover {
        #[serde(rename = "tabId")]
        tab_id: String,
        over: bool,
    },
    /// `lockPasswords` (an exported function that writes a store).
    #[serde(rename = "lockPasswords")]
    LockPasswords,
    /// `offerVerificationCode` (an exported function that writes a store). The callback parameters `offered` are not in the JSON.
    #[serde(rename = "offerVerificationCode")]
    OfferVerificationCode {
        #[serde(rename = "tabId")]
        tab_id: String,
        found: serde_json::Value,
        generation: Number,
    },
    /// `openBarFromDeveloperBar` (an exported function that writes a store).
    #[serde(rename = "openBarFromDeveloperBar")]
    OpenBarFromDeveloperBar {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
        url: String,
    },
    /// `popupBlocked` (an exported function that writes a store).
    #[serde(rename = "popupBlocked")]
    PopupBlocked {
        #[serde(rename = "tabId")]
        tab_id: String,
        popup: serde_json::Value,
    },
    /// `releaseTranslation` (an exported function that writes a store).
    #[serde(rename = "releaseTranslation")]
    ReleaseTranslation {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `requestAutofill` (an exported function that writes a store).
    #[serde(rename = "requestAutofill")]
    RequestAutofill {
        #[serde(rename = "windowId")]
        window_id: String,
        arg: Option<String>,
        #[serde(rename = "tabId", default)]
        tab_id: Option<String>,
    },
    /// `restoreHiddenElements` (an exported function that writes a store).
    #[serde(rename = "restoreHiddenElements")]
    RestoreHiddenElements {
        profile: String,
        elements: Vec<HiddenElement>,
    },
    /// `revertPage` (an exported function that writes a store).
    #[serde(rename = "revertPage")]
    RevertPage {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(default)]
        arg1: Option<RevertPageArg1>,
    },
    /// `runHidingCommand` (an exported function that writes a store).
    #[serde(rename = "runHidingCommand")]
    RunHidingCommand {
        command: String,
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `searchSelection` (an exported function that writes a store).
    #[serde(rename = "searchSelection")]
    SearchSelection {
        #[serde(rename = "tabId")]
        tab_id: String,
        text: String,
        #[serde(default)]
        modifiers: Option<ClickModifiers>,
    },
    /// `setDeveloperMode` (an exported function that writes a store).
    #[serde(rename = "setDeveloperMode")]
    SetDeveloperMode { url: String, on: bool },
    /// `setNeverTranslate` (an exported function that writes a store).
    #[serde(rename = "setNeverTranslate")]
    SetNeverTranslate {
        #[serde(rename = "tabId")]
        tab_id: String,
        never: bool,
    },
    /// `setPageSelection` (an exported function that writes a store).
    #[serde(rename = "setPageSelection")]
    SetPageSelection {
        #[serde(rename = "tabId")]
        tab_id: String,
        selection: serde_json::Value,
    },
    /// `setSiteBlocking` (an exported function that writes a store).
    #[serde(rename = "setSiteBlocking")]
    SetSiteBlocking {
        tab: SetSiteBlockingTab,
        host: String,
        on: bool,
    },
    /// `setSitePermission` (an exported function that writes a store).
    #[serde(rename = "setSitePermission")]
    SetSitePermission {
        page: SetSitePermissionPage,
        #[serde(rename = "typeArg")]
        type_arg: serde_json::Value,
        value: serde_json::Value,
        sound: SiteSound,
    },
    /// `showAutofillPrompt` (an exported function that writes a store).
    #[serde(rename = "showAutofillPrompt")]
    ShowAutofillPrompt {
        #[serde(rename = "tabId")]
        tab_id: String,
        prompt: serde_json::Value,
        id: Number,
    },
    /// `showExternalApp` (an exported function that writes a store).
    #[serde(rename = "showExternalApp")]
    ShowExternalApp {
        #[serde(rename = "tabId")]
        tab_id: String,
        request: serde_json::Value,
    },
    /// `showPasswordPrompt` (an exported function that writes a store).
    #[serde(rename = "showPasswordPrompt")]
    ShowPasswordPrompt {
        #[serde(rename = "tabId")]
        tab_id: String,
        prompt: serde_json::Value,
    },
    /// `showReloadMenu` (an exported function that writes a store).
    #[serde(rename = "showReloadMenu")]
    ShowReloadMenu {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `showSplitSidesMenu` (an exported function that writes a store). The callback parameters `stillTarget` are not in the JSON.
    #[serde(rename = "showSplitSidesMenu")]
    ShowSplitSidesMenu {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `startDeviceChoosers` (an exported function that writes a store).
    #[serde(rename = "startDeviceChoosers")]
    StartDeviceChoosers,
    /// `startHiddenElements` (an exported function that writes a store).
    #[serde(rename = "startHiddenElements")]
    StartHiddenElements,
    /// `startHidingElement` (an exported function that writes a store).
    #[serde(rename = "startHidingElement")]
    StartHidingElement {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(default)]
        point: Option<(Number, Number)>,
    },
    /// `startPasswordManager` (an exported function that writes a store).
    #[serde(rename = "startPasswordManager")]
    StartPasswordManager,
    /// `startPermissionPrompts` (an exported function that writes a store).
    #[serde(rename = "startPermissionPrompts")]
    StartPermissionPrompts,
    /// `startSelectionTools` (an exported function that writes a store).
    #[serde(rename = "startSelectionTools")]
    StartSelectionTools,
    /// `startTranslate` (an exported function that writes a store).
    #[serde(rename = "startTranslate")]
    StartTranslate,
    /// `startWebKitNotifications` (an exported function that writes a store).
    #[serde(rename = "startWebKitNotifications")]
    StartWebKitNotifications,
    /// `toggleTranslation` (an exported function that writes a store).
    #[serde(rename = "toggleTranslation")]
    ToggleTranslation {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `toggleZoomPopover` (an exported function that writes a store).
    #[serde(rename = "toggleZoomPopover")]
    ToggleZoomPopover {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `translatePage` (an exported function that writes a store).
    #[serde(rename = "translatePage")]
    TranslatePage {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(default)]
        target: Option<String>,
    },
    /// `unlockPasswordsFor` (an exported function that writes a store).
    #[serde(rename = "unlockPasswordsFor")]
    UnlockPasswordsFor { profile: String },
    /// `verificationCodeDocument` (an exported function that writes a store).
    #[serde(rename = "verificationCodeDocument")]
    VerificationCodeDocument {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(rename = "committedAt")]
        committed_at: Number,
    },
    /// `verificationCodeFound` (an exported function that writes a store).
    #[serde(rename = "verificationCodeFound")]
    VerificationCodeFound {
        #[serde(rename = "tabId")]
        tab_id: String,
        found: serde_json::Value,
    },
    /// `writeOfferToSave` (an exported function that writes a store).
    #[serde(rename = "writeOfferToSave")]
    WriteOfferToSave { profile: String },
    /// `zoomChangedByUser` (an exported function that writes a store).
    #[serde(rename = "zoomChangedByUser")]
    ZoomChangedByUser {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
}

/// Applies `action` to `app`. Slice 2.20 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.20")
}
