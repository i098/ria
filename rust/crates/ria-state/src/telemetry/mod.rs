// Ported from ardia apps/browser/src/state/telemetry/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `telemetry` (`apps/browser/src/state/telemetry`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.6 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CliInstallReason {
    #[default]
    #[serde(rename = "settings")]
    Settings,
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "upgrade")]
    Upgrade,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ConsentToken {
    pub length: Number,
    #[serde(rename = "consentPeriod")]
    pub consent_period: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum EscapePressedIn {
    #[default]
    #[serde(rename = "window")]
    Window,
    #[serde(rename = "page")]
    Page,
    #[serde(rename = "blankBar")]
    BlankBar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TabPressDrift {
    #[default]
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "lt5")]
    Lt5,
    #[serde(rename = "5_9")]
    V59,
    #[serde(rename = "10_plus")]
    V10Plus,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SavedContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webui_page: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webui_pages: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SavePatch {
    Other0(SavePatchVariant0),
    Other1(SavePatchVariant1),
    Other2(SavePatchVariant2),
}

impl Default for SavePatch {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SavePatchVariant0 {
    #[serde(rename = "decidedAt", default, skip_serializing_if = "Option::is_none")]
    pub decided_at: Option<Number>,
    #[serde(rename = "askDoneAt", default, skip_serializing_if = "Option::is_none")]
    pub ask_done_at: Option<Number>,
    #[serde(
        rename = "crashCursor",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub crash_cursor: Option<Number>,
    #[serde(
        rename = "lastVersion",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_version: Option<String>,
    #[serde(
        rename = "sessionOpen",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub session_open: Option<bool>,
    #[serde(rename = "sessionId", default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(
        rename = "lastLaunchAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_launch_at: Option<Number>,
    #[serde(
        rename = "lastQuitAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_quit_at: Option<Number>,
    #[serde(rename = "lastBuild", default, skip_serializing_if = "Option::is_none")]
    pub last_build: Option<String>,
    #[serde(
        rename = "settingsDay",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub settings_day: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<SavedContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epoch: Option<String>,
    #[serde(
        rename = "watchReport",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_report: Option<String>,
    #[serde(
        rename = "watchStartedAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_started_at: Option<Number>,
    #[serde(
        rename = "watchUntil",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_until: Option<Number>,
    #[serde(
        rename = "sharingUntil",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sharing_until: Option<Number>,
    #[serde(
        rename = "watchCountedMs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_counted_ms: Option<Number>,
    #[serde(
        rename = "watchCheckedAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_checked_at: Option<Number>,
    #[serde(
        rename = "watchEndedNotice",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_ended_notice: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SavePatchVariant1 {
    #[serde(rename = "decidedAt", default, skip_serializing_if = "Option::is_none")]
    pub decided_at: Option<Number>,
    #[serde(rename = "askDoneAt", default, skip_serializing_if = "Option::is_none")]
    pub ask_done_at: Option<Number>,
    #[serde(
        rename = "crashCursor",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub crash_cursor: Option<Number>,
    #[serde(
        rename = "lastVersion",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_version: Option<String>,
    #[serde(
        rename = "sessionOpen",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub session_open: Option<bool>,
    #[serde(rename = "sessionId", default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(
        rename = "lastLaunchAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_launch_at: Option<Number>,
    #[serde(
        rename = "lastQuitAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_quit_at: Option<Number>,
    #[serde(rename = "lastBuild", default, skip_serializing_if = "Option::is_none")]
    pub last_build: Option<String>,
    #[serde(
        rename = "settingsDay",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub settings_day: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<SavedContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epoch: Option<String>,
    #[serde(
        rename = "watchReport",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_report: Option<String>,
    #[serde(
        rename = "watchStartedAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_started_at: Option<Number>,
    #[serde(
        rename = "watchUntil",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_until: Option<Number>,
    #[serde(
        rename = "sharingUntil",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sharing_until: Option<Number>,
    #[serde(
        rename = "watchCountedMs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_counted_ms: Option<Number>,
    #[serde(
        rename = "watchCheckedAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_checked_at: Option<Number>,
    #[serde(
        rename = "watchEndedNotice",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_ended_notice: Option<String>,
    pub sharing: bool,
    #[serde(rename = "installId")]
    pub install_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SavePatchVariant2 {
    #[serde(rename = "decidedAt", default, skip_serializing_if = "Option::is_none")]
    pub decided_at: Option<Number>,
    #[serde(rename = "askDoneAt", default, skip_serializing_if = "Option::is_none")]
    pub ask_done_at: Option<Number>,
    #[serde(
        rename = "crashCursor",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub crash_cursor: Option<Number>,
    #[serde(
        rename = "lastVersion",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_version: Option<String>,
    #[serde(
        rename = "sessionOpen",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub session_open: Option<bool>,
    #[serde(rename = "sessionId", default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(
        rename = "lastLaunchAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_launch_at: Option<Number>,
    #[serde(
        rename = "lastQuitAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub last_quit_at: Option<Number>,
    #[serde(rename = "lastBuild", default, skip_serializing_if = "Option::is_none")]
    pub last_build: Option<String>,
    #[serde(
        rename = "settingsDay",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub settings_day: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<SavedContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epoch: Option<String>,
    #[serde(
        rename = "watchReport",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_report: Option<String>,
    #[serde(
        rename = "watchStartedAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_started_at: Option<Number>,
    #[serde(
        rename = "watchUntil",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_until: Option<Number>,
    #[serde(
        rename = "sharingUntil",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sharing_until: Option<Number>,
    #[serde(
        rename = "watchCountedMs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_counted_ms: Option<Number>,
    #[serde(
        rename = "watchCheckedAt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_checked_at: Option<Number>,
    #[serde(
        rename = "watchEndedNotice",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_ended_notice: Option<String>,
    pub sharing: bool,
    #[serde(rename = "installId")]
    pub install_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SharingSource {
    #[default]
    #[serde(rename = "settings")]
    Settings,
    #[serde(rename = "ask")]
    Ask,
    #[serde(rename = "onboarding")]
    Onboarding,
    #[serde(rename = "watch")]
    Watch,
    #[serde(rename = "watch_ended")]
    WatchEnded,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AskPlacement {
    pub ask_place: String,
    pub ask_shown_tabs: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StartTelemetryArg0 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuing: Option<bool>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(rename = "askDone")]
    pub ask_done: bool,
    pub decided: bool,
    pub epoch: Option<String>,
    #[serde(rename = "installId")]
    pub install_id: Option<String>,
    pub sharing: bool,
    #[serde(rename = "sharingUntil")]
    pub sharing_until: Option<Number>,
    #[serde(rename = "watchReport")]
    pub watch_report: Option<String>,
    #[serde(rename = "watchUntil")]
    pub watch_until: Option<Number>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `adoptInstallId` (an exported function that writes a store).
    #[serde(rename = "adoptInstallId")]
    AdoptInstallId {
        #[serde(rename = "installId")]
        install_id: String,
    },
    /// `advanceCrashCursor` (an exported function that writes a store).
    #[serde(rename = "advanceCrashCursor")]
    AdvanceCrashCursor { to: Number },
    /// `airTrafficSnapshot` (an exported function that writes a store).
    #[serde(rename = "airTrafficSnapshot")]
    AirTrafficSnapshot,
    /// `checkWatch` (an exported function that writes a store).
    #[serde(rename = "checkWatch")]
    CheckWatch {
        #[serde(default)]
        now: Option<Number>,
        #[serde(rename = "sameSession", default)]
        same_session: Option<bool>,
    },
    /// `devHourly` (an exported function that writes a store).
    #[serde(rename = "devHourly")]
    DevHourly,
    /// `dismissAsk` (an exported function that writes a store).
    #[serde(rename = "dismissAsk")]
    DismissAsk,
    /// `endSession` (an exported function that writes a store).
    #[serde(rename = "endSession")]
    EndSession,
    /// `ensureInstallId` (an exported function that writes a store).
    #[serde(rename = "ensureInstallId")]
    EnsureInstallId,
    /// `installErrorReporting` (an exported function that writes a store).
    #[serde(rename = "installErrorReporting")]
    InstallErrorReporting,
    /// `loadChoice` (an exported function that writes a store).
    #[serde(rename = "loadChoice")]
    LoadChoice,
    /// `markSettingsSnapshot` (an exported function that writes a store).
    #[serde(rename = "markSettingsSnapshot")]
    MarkSettingsSnapshot { day: String },
    /// `recordContext` (an exported function that writes a store).
    #[serde(rename = "recordContext")]
    RecordContext,
    /// `reloadChoice` (an exported function that writes a store).
    #[serde(rename = "reloadChoice")]
    ReloadChoice,
    /// `reopenSession` (an exported function that writes a store).
    #[serde(rename = "reopenSession")]
    ReopenSession,
    /// `replaceInstallId` (an exported function that writes a store).
    #[serde(rename = "replaceInstallId")]
    ReplaceInstallId,
    /// `reportIdentity` (an exported function that writes a store).
    #[serde(rename = "reportIdentity")]
    ReportIdentity { watch: bool },
    /// `reportNativeCrashes` (an exported function that writes a store).
    #[serde(rename = "reportNativeCrashes")]
    ReportNativeCrashes {
        #[serde(default)]
        contexts: Option<Vec<Option<SavedContext>>>,
    },
    /// `resetAnonymousId` (an exported function that writes a store).
    #[serde(rename = "resetAnonymousId")]
    ResetAnonymousId,
    /// `save` (an exported function that writes a store).
    #[serde(rename = "save")]
    Save { patch: SavePatch },
    /// `saveContext` (an exported function that writes a store).
    #[serde(rename = "saveContext")]
    SaveContext { context: Option<SavedContext> },
    /// `sessionEnding` (an exported function that writes a store).
    #[serde(rename = "sessionEnding")]
    SessionEnding,
    /// `setSharing` (an exported function that writes a store).
    #[serde(rename = "setSharing")]
    SetSharing {
        on: bool,
        source: SharingSource,
        #[serde(default)]
        placement: Option<AskPlacement>,
    },
    /// `startClient` (an exported function that writes a store).
    #[serde(rename = "startClient")]
    StartClient {
        #[serde(default)]
        continuing: Option<bool>,
    },
    /// `startContextSamples` (an exported function that writes a store).
    #[serde(rename = "startContextSamples")]
    StartContextSamples,
    /// `startTelemetry` (an exported function that writes a store).
    #[serde(rename = "startTelemetry")]
    StartTelemetry {
        #[serde(default)]
        arg0: Option<StartTelemetryArg0>,
    },
    /// `startUsage` (an exported function that writes a store).
    #[serde(rename = "startUsage")]
    StartUsage {
        #[serde(default)]
        continuing: Option<bool>,
    },
    /// `startWatch` (an exported function that writes a store).
    #[serde(rename = "startWatch")]
    StartWatch {
        #[serde(rename = "reportId")]
        report_id: String,
        #[serde(default)]
        days: Option<Number>,
        #[serde(default)]
        now: Option<Number>,
    },
    /// `startWatchChecks` (an exported function that writes a store).
    #[serde(rename = "startWatchChecks")]
    StartWatchChecks,
    /// `startWatchNotice` (an exported function that writes a store).
    #[serde(rename = "startWatchNotice")]
    StartWatchNotice,
    /// `stopWatch` (an exported function that writes a store).
    #[serde(rename = "stopWatch")]
    StopWatch,
    /// `watchNoticeShown` (an exported function that writes a store).
    #[serde(rename = "watchNoticeShown")]
    WatchNoticeShown,
}

/// Applies `action` to `app`. Slice 2.6 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.6")
}
