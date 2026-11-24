// Ported from ardia apps/browser/src/state/report/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `report` (`apps/browser/src/state/report`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.45 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ReportDraft {
    pub kind: ReportDraftKind,
    pub title: String,
    pub description: String,
    pub contact: String,
    #[serde(rename = "includePages")]
    pub include_pages: bool,
    pub trace: bool,
    pub attach: ReportDraftAttach,
    pub watch: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ReportDraftKind {
    #[default]
    #[serde(rename = "other")]
    Other,
    #[serde(rename = "bug")]
    Bug,
    #[serde(rename = "slow")]
    Slow,
    #[serde(rename = "idea")]
    Idea,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ReportDraftAttach {
    pub settings: bool,
    pub page: bool,
    pub system: bool,
    pub errors: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateLocal {
    pub path: String,
    pub bytes: Number,
    pub lines: Number,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "phase")]
pub enum Recording {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "starting")]
    Starting,
    #[serde(rename = "recording")]
    Recording {
        seconds: Number,
        #[serde(rename = "includePages")]
        include_pages: bool,
    },
    #[serde(rename = "saving")]
    Saving {
        #[serde(rename = "includePages")]
        include_pages: bool,
    },
    #[serde(rename = "ready")]
    Ready {
        path: String,
        thumbnail: Option<String>,
        seconds: Number,
        bytes: Number,
        #[serde(rename = "includePages")]
        include_pages: bool,
        #[serde(
            rename = "pictureInPicture",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        picture_in_picture: Option<bool>,
        #[serde(
            rename = "withheldWindows",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        withheld_windows: Option<Number>,
    },
    #[serde(rename = "failed")]
    Failed { message: String },
}

impl Default for Recording {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SentReport {
    pub id: String,
    #[serde(rename = "issueUrl")]
    pub issue_url: Option<String>,
    #[serde(rename = "issueNumber")]
    pub issue_number: Option<Number>,
    pub name: String,
    pub avatar: SentReportAvatar,
    #[serde(rename = "watchUntil")]
    pub watch_until: Option<Number>,
    #[serde(rename = "watchTurnsOff")]
    pub watch_turns_off: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SentReportAvatar {
    pub colors: (String, String, String),
    pub angle: Number,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StateSource {
    #[default]
    #[serde(rename = "menu")]
    Menu,
    #[serde(rename = "check_in")]
    CheckIn,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "phase")]
pub enum Trace {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "tracing")]
    Tracing,
    #[serde(rename = "saving")]
    Saving,
    #[serde(rename = "ready")]
    Ready {
        path: String,
        bytes: Number,
        seconds: Number,
    },
    #[serde(rename = "failed")]
    Failed,
}

impl Default for Trace {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "phase")]
pub enum UploadState {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "creating")]
    Creating,
    #[serde(rename = "uploading")]
    Uploading {
        created: Created,
        file: String,
        sent: Number,
        total: Number,
    },
    #[serde(rename = "completing")]
    Completing { created: Created },
    #[serde(rename = "sent")]
    Sent { sent: Sent },
    #[serde(rename = "failed")]
    Failed {
        stage: FailedStage,
        message: String,
        created: Option<Created>,
        #[serde(rename = "errorClass")]
        error_class: FailureClass,
        status: Number,
        attempts: Number,
    },
}

impl Default for UploadState {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Created {
    pub id: String,
    pub name: Option<String>,
    pub token: String,
    #[serde(rename = "uploadUrls")]
    pub upload_urls: std::collections::HashMap<String, String>,
    pub uploaded: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Sent {
    pub id: String,
    pub name: Option<String>,
    #[serde(rename = "issueUrl")]
    pub issue_url: Option<String>,
    #[serde(rename = "issueNumber")]
    pub issue_number: Option<Number>,
    #[serde(rename = "uploadMs")]
    pub upload_ms: Number,
    pub bytes: Number,
    pub retries: Number,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum FailedStage {
    #[default]
    #[serde(rename = "create")]
    Create,
    #[serde(rename = "upload")]
    Upload,
    #[serde(rename = "complete")]
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum FailureClass {
    #[default]
    #[serde(rename = "offline")]
    Offline,
    #[serde(rename = "storage")]
    Storage,
    #[serde(rename = "server")]
    Server,
    #[serde(rename = "rate_limited")]
    RateLimited,
    #[serde(rename = "expired")]
    Expired,
    #[serde(rename = "rejected")]
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum OpenReportSource {
    #[default]
    #[serde(rename = "menu")]
    Menu,
    #[serde(rename = "check_in")]
    CheckIn,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PatchDraftPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<PatchDraftPatchKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,
    #[serde(
        rename = "includePages",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub include_pages: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attach: Option<ReportDraftAttach>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watch: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PatchDraftPatchKind {
    #[default]
    #[serde(rename = "other")]
    Other,
    #[serde(rename = "bug")]
    Bug,
    #[serde(rename = "slow")]
    Slow,
    #[serde(rename = "idea")]
    Idea,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PatchReportPatch {
    #[serde(rename = "windowId", default, skip_serializing_if = "Option::is_none")]
    pub window_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PatchReportPatchSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft: Option<ReportDraft>,
    #[serde(rename = "draftDir", default, skip_serializing_if = "Option::is_none")]
    pub draft_dir: Option<String>,
    #[serde(
        rename = "pageAddress",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub page_address: Option<String>,
    #[serde(
        rename = "systemLine",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub system_line: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<StateLocal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recording: Option<PatchReportPatchRecording>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<PatchReportPatchTrace>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upload: Option<PatchReportPatchUpload>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sent: Option<SentReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<bool>,
    #[serde(
        rename = "attachedOpen",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub attached_open: Option<bool>,
    #[serde(
        rename = "showProblems",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_problems: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PatchReportPatchSource {
    #[default]
    #[serde(rename = "menu")]
    Menu,
    #[serde(rename = "check_in")]
    CheckIn,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "phase")]
pub enum PatchReportPatchRecording {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "starting")]
    Starting,
    #[serde(rename = "recording")]
    Recording {
        seconds: Number,
        #[serde(rename = "includePages")]
        include_pages: bool,
    },
    #[serde(rename = "saving")]
    Saving {
        #[serde(rename = "includePages")]
        include_pages: bool,
    },
    #[serde(rename = "ready")]
    Ready {
        path: String,
        thumbnail: Option<String>,
        seconds: Number,
        bytes: Number,
        #[serde(rename = "includePages")]
        include_pages: bool,
        #[serde(
            rename = "pictureInPicture",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        picture_in_picture: Option<bool>,
        #[serde(
            rename = "withheldWindows",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        withheld_windows: Option<Number>,
    },
    #[serde(rename = "failed")]
    Failed { message: String },
}

impl Default for PatchReportPatchRecording {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "phase")]
pub enum PatchReportPatchTrace {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "tracing")]
    Tracing,
    #[serde(rename = "saving")]
    Saving,
    #[serde(rename = "ready")]
    Ready {
        path: String,
        bytes: Number,
        seconds: Number,
    },
    #[serde(rename = "failed")]
    Failed,
}

impl Default for PatchReportPatchTrace {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "phase")]
pub enum PatchReportPatchUpload {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "creating")]
    Creating,
    #[serde(rename = "uploading")]
    Uploading {
        created: Created,
        file: String,
        sent: Number,
        total: Number,
    },
    #[serde(rename = "completing")]
    Completing { created: Created },
    #[serde(rename = "sent")]
    Sent { sent: Sent },
    #[serde(rename = "failed")]
    Failed {
        stage: FailedStage,
        message: String,
        created: Option<Created>,
        #[serde(rename = "errorClass")]
        error_class: FailureClass,
        status: Number,
        attempts: Number,
    },
}

impl Default for PatchReportPatchUpload {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AttachmentKey {
    #[default]
    #[serde(rename = "settings")]
    Settings,
    #[serde(rename = "page")]
    Page,
    #[serde(rename = "system")]
    System,
    #[serde(rename = "errors")]
    Errors,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetKindKind {
    #[default]
    #[serde(rename = "other")]
    Other,
    #[serde(rename = "bug")]
    Bug,
    #[serde(rename = "slow")]
    Slow,
    #[serde(rename = "idea")]
    Idea,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(rename = "attachedOpen")]
    pub attached_open: bool,
    pub draft: ReportDraft,
    #[serde(rename = "draftDir")]
    pub draft_dir: Option<String>,
    pub local: Option<StateLocal>,
    #[serde(rename = "pageAddress")]
    pub page_address: Option<String>,
    pub preview: bool,
    pub recording: Recording,
    pub sent: Option<SentReport>,
    #[serde(rename = "showProblems")]
    pub show_problems: bool,
    pub source: Option<StateSource>,
    #[serde(rename = "systemLine")]
    pub system_line: String,
    pub trace: Trace,
    pub upload: UploadState,
    #[serde(rename = "windowId")]
    pub window_id: Option<String>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `closeReport` (an exported function that writes a store).
    #[serde(rename = "closeReport")]
    CloseReport,
    /// `discardRecording` (an exported function that writes a store).
    #[serde(rename = "discardRecording")]
    DiscardRecording,
    /// `discardTrace` (an exported function that writes a store).
    #[serde(rename = "discardTrace")]
    DiscardTrace,
    /// `ensureDraftDir` (an exported function that writes a store).
    #[serde(rename = "ensureDraftDir")]
    EnsureDraftDir,
    /// `finishReport` (an exported function that writes a store).
    #[serde(rename = "finishReport")]
    FinishReport,
    /// `openReport` (an exported function that writes a store).
    #[serde(rename = "openReport")]
    OpenReport {
        #[serde(rename = "windowId")]
        window_id: Option<String>,
        #[serde(default)]
        source: Option<OpenReportSource>,
    },
    /// `patchDraft` (an exported function that writes a store).
    #[serde(rename = "patchDraft")]
    PatchDraft { patch: PatchDraftPatch },
    /// `patchReport` (an exported function that writes a store).
    #[serde(rename = "patchReport")]
    PatchReport { patch: PatchReportPatch },
    /// `resetReport` (an exported function that writes a store).
    #[serde(rename = "resetReport")]
    ResetReport,
    /// `sendCurrentReport` (an exported function that writes a store).
    #[serde(rename = "sendCurrentReport")]
    SendCurrentReport,
    /// `setAttach` (an exported function that writes a store).
    #[serde(rename = "setAttach")]
    SetAttach { key: AttachmentKey, on: bool },
    /// `setAttachedOpen` (an exported function that writes a store).
    #[serde(rename = "setAttachedOpen")]
    SetAttachedOpen {
        #[serde(rename = "attachedOpen")]
        attached_open: bool,
    },
    /// `setContact` (an exported function that writes a store).
    #[serde(rename = "setContact")]
    SetContact { contact: String },
    /// `setDescription` (an exported function that writes a store).
    #[serde(rename = "setDescription")]
    SetDescription { description: String },
    /// `setIncludePages` (an exported function that writes a store).
    #[serde(rename = "setIncludePages")]
    SetIncludePages {
        #[serde(rename = "includePages")]
        include_pages: bool,
    },
    /// `setKind` (an exported function that writes a store).
    #[serde(rename = "setKind")]
    SetKind { kind: SetKindKind },
    /// `setPreview` (an exported function that writes a store).
    #[serde(rename = "setPreview")]
    SetPreview { preview: bool },
    /// `setTitle` (an exported function that writes a store).
    #[serde(rename = "setTitle")]
    SetTitle { title: String },
    /// `setTrace` (an exported function that writes a store).
    #[serde(rename = "setTrace")]
    SetTrace { trace: bool },
    /// `setWatch` (an exported function that writes a store).
    #[serde(rename = "setWatch")]
    SetWatch { watch: bool },
    /// `startRecording` (an exported function that writes a store).
    #[serde(rename = "startRecording")]
    StartRecording,
    /// `stopRecording` (an exported function that writes a store).
    #[serde(rename = "stopRecording")]
    StopRecording,
    /// `viewIssue` (an exported function that writes a store).
    #[serde(rename = "viewIssue")]
    ViewIssue,
}

/// Applies `action` to `app`. Slice 2.45 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.45")
}
