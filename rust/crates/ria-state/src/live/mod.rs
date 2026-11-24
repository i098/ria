// Ported from ardia apps/browser/src/state/live/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `live` (`apps/browser/src/state/live`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.37 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateAccounts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github: Option<LiveAccount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitbucket: Option<LiveAccount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notion: Option<LiveAccount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confluence: Option<LiveAccount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gdrive: Option<LiveAccount>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LiveAccount {
    pub login: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
    #[serde(rename = "connectedAt")]
    pub connected_at: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CalendarSettings {
    #[serde(rename = "alertLead")]
    pub alert_lead: String,
    #[serde(rename = "alertCondition")]
    pub alert_condition: CalendarSettingsAlertCondition,
    #[serde(rename = "showPreview")]
    pub show_preview: bool,
    #[serde(rename = "showTimeToNext")]
    pub show_time_to_next: bool,
    #[serde(rename = "countdownLead")]
    pub countdown_lead: Number,
    #[serde(rename = "hiddenCalendarIds")]
    pub hidden_calendar_ids: Vec<String>,
    pub prompted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CalendarSettingsAlertCondition {
    #[default]
    #[serde(rename = "always")]
    Always,
    #[serde(rename = "activeOnly")]
    ActiveOnly,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CompletedItem {
    pub item: LiveItem,
    pub state: CompletionState,
    pub at: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LiveItem {
    pub id: String,
    pub source: LiveSourceId,
    pub url: String,
    pub title: String,
    pub subtitle: String,
    pub icon: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<LiveItemSection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pr: Option<PullRequestInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<DocumentInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveSourceId {
    #[default]
    #[serde(rename = "github")]
    Github,
    #[serde(rename = "bitbucket")]
    Bitbucket,
    #[serde(rename = "notion")]
    Notion,
    #[serde(rename = "confluence")]
    Confluence,
    #[serde(rename = "gdrive")]
    Gdrive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveItemSection {
    #[default]
    #[serde(rename = "authored")]
    Authored,
    #[serde(rename = "review")]
    Review,
    #[serde(rename = "team")]
    Team,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PullRequestInfo {
    pub number: Number,
    pub repo: String,
    pub author: String,
    #[serde(rename = "authorAvatar")]
    pub author_avatar: Option<String>,
    pub draft: bool,
    #[serde(rename = "headRef")]
    pub head_ref: String,
    #[serde(rename = "baseRef")]
    pub base_ref: String,
    pub review: Option<PullRequestInfoReview>,
    pub mergeable: PullRequestInfoMergeable,
    pub checks: Vec<PullRequestCheck>,
    pub comments: Number,
    #[serde(rename = "unresolvedThreads")]
    pub unresolved_threads: Number,
    pub additions: Number,
    pub deletions: Number,
    #[serde(rename = "changedFiles")]
    pub changed_files: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<PullRequestInfoStack>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PullRequestInfoReview {
    #[default]
    #[serde(rename = "required")]
    Required,
    #[serde(rename = "approved")]
    Approved,
    #[serde(rename = "changesRequested")]
    ChangesRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PullRequestInfoMergeable {
    #[default]
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "mergeable")]
    Mergeable,
    #[serde(rename = "conflicting")]
    Conflicting,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PullRequestCheck {
    pub name: String,
    pub state: CheckState,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CheckState {
    #[default]
    #[serde(rename = "neutral")]
    Neutral,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "failure")]
    Failure,
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "queued")]
    Queued,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PullRequestInfoStack {
    pub id: String,
    pub position: Number,
    pub size: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DocumentInfo {
    pub kind: String,
    #[serde(rename = "editedBy")]
    pub edited_by: Option<String>,
    pub place: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CompletionState {
    #[default]
    #[serde(rename = "merged")]
    Merged,
    #[serde(rename = "closed")]
    Closed,
    #[serde(rename = "reviewed")]
    Reviewed,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LiveConfig {
    #[serde(rename = "githubClientId")]
    pub github_client_id: String,
    #[serde(rename = "bitbucketUser")]
    pub bitbucket_user: String,
    #[serde(rename = "confluenceSite")]
    pub confluence_site: String,
    #[serde(rename = "confluenceEmail")]
    pub confluence_email: String,
    #[serde(rename = "googleClientId")]
    pub google_client_id: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LiveFolder {
    pub id: String,
    #[serde(rename = "profileId")]
    pub profile_id: String,
    pub kind: LiveFolderKind,
    pub name: String,
    pub icon: Option<String>,
    pub collapsed: bool,
    pub sources: Vec<LiveSourceId>,
    pub filters: LiveFolderFilters,
    #[serde(rename = "groupStacks")]
    pub group_stacks: bool,
    #[serde(rename = "showCompleted")]
    pub show_completed: bool,
    #[serde(rename = "hiddenRepos")]
    pub hidden_repos: Vec<String>,
    #[serde(rename = "showNewRepos")]
    pub show_new_repos: bool,
    #[serde(rename = "shownRepos")]
    pub shown_repos: Vec<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveFolderKind {
    #[default]
    #[serde(rename = "pullRequests")]
    PullRequests,
    #[serde(rename = "documents")]
    Documents,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LiveFolderFilters {
    pub authored: bool,
    #[serde(rename = "reviewRequests")]
    pub review_requests: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MeetingGroup {
    #[serde(rename = "groupId")]
    pub group_id: String,
    #[serde(rename = "callTabId")]
    pub call_tab_id: String,
    pub occurrence: Option<String>,
    pub title: String,
    pub start: Option<Number>,
    pub end: Option<Number>,
    #[serde(rename = "endedAt")]
    pub ended_at: Option<Number>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FolderStatus {
    pub state: FolderStatusState,
    #[serde(rename = "lastFetch")]
    pub last_fetch: Option<Number>,
    pub error: Option<FolderStatusError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum FolderStatusState {
    #[default]
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "updating")]
    Updating,
    #[serde(rename = "initializing")]
    Initializing,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FolderStatusError {
    pub kind: LiveErrorKind,
    pub message: String,
    pub source: LiveSourceId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LiveErrorKind {
    #[default]
    #[serde(rename = "other")]
    Other,
    #[serde(rename = "network")]
    Network,
    #[serde(rename = "rateLimited")]
    RateLimited,
    #[serde(rename = "signedOut")]
    SignedOut,
    #[serde(rename = "sso")]
    Sso,
    #[serde(rename = "tokenBlocked")]
    TokenBlocked,
    #[serde(rename = "notConfigured")]
    NotConfigured,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ArrangeFoldersPlacedItem {
    pub id: String,
    pub after: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DismissNowBadgeEvent {
    pub occurrence: String,
    pub end: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct NewProfileLiveFolderOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OpenLiveItemOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SetLiveIntent {
    #[serde(rename = "placement")]
    Placement {
        placed: Vec<SetLiveIntentPlacementPlacedItem>,
    },
    #[serde(rename = "refresh")]
    Refresh {
        #[serde(rename = "folderId")]
        folder_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fence: Option<serde_json::Value>,
        #[serde(
            rename = "completionIds",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        completion_ids: Option<Vec<String>>,
    },
    #[serde(rename = "completion")]
    Completion {
        #[serde(rename = "folderId")]
        folder_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fence: Option<serde_json::Value>,
        #[serde(
            rename = "completionIds",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        completion_ids: Option<Vec<String>>,
    },
}

impl Default for SetLiveIntent {
    fn default() -> Self {
        Self::Placement {
            placed: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SetLiveIntentPlacementPlacedItem {
    pub id: String,
    pub after: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SetStatusStatus {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<SetStatusStatusState>,
    #[serde(rename = "lastFetch", default, skip_serializing_if = "Option::is_none")]
    pub last_fetch: Option<Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<FolderStatusError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetStatusStatusState {
    #[default]
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "updating")]
    Updating,
    #[serde(rename = "initializing")]
    Initializing,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UpdateCalendarSettingsPatch {
    #[serde(rename = "alertLead", default, skip_serializing_if = "Option::is_none")]
    pub alert_lead: Option<String>,
    #[serde(
        rename = "alertCondition",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub alert_condition: Option<UpdateCalendarSettingsPatchAlertCondition>,
    #[serde(
        rename = "showPreview",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_preview: Option<bool>,
    #[serde(
        rename = "showTimeToNext",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_time_to_next: Option<bool>,
    #[serde(
        rename = "countdownLead",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub countdown_lead: Option<Number>,
    #[serde(
        rename = "hiddenCalendarIds",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub hidden_calendar_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompted: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum UpdateCalendarSettingsPatchAlertCondition {
    #[default]
    #[serde(rename = "always")]
    Always,
    #[serde(rename = "activeOnly")]
    ActiveOnly,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UpdateConfigPatch {
    #[serde(
        rename = "githubClientId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub github_client_id: Option<String>,
    #[serde(
        rename = "bitbucketUser",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bitbucket_user: Option<String>,
    #[serde(
        rename = "confluenceSite",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub confluence_site: Option<String>,
    #[serde(
        rename = "confluenceEmail",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub confluence_email: Option<String>,
    #[serde(
        rename = "googleClientId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub google_client_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UpdateFolderPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collapsed: Option<bool>,
    #[serde(
        rename = "groupStacks",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub group_stacks: Option<bool>,
    #[serde(
        rename = "showCompleted",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_completed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    #[serde(rename = "createdAt", default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<LiveSourceId>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<LiveFolderFilters>,
    #[serde(
        rename = "hiddenRepos",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub hidden_repos: Option<Vec<String>>,
    #[serde(
        rename = "showNewRepos",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub show_new_repos: Option<bool>,
    #[serde(
        rename = "shownRepos",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub shown_repos: Option<Vec<String>>,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub access: serde_json::Value,
    pub accounts: StateAccounts,
    pub calendar: CalendarSettings,
    pub calendars: Vec<serde_json::Value>,
    pub completed: std::collections::HashMap<String, Vec<CompletedItem>>,
    pub completing: std::collections::HashMap<String, Vec<String>>,
    pub config: LiveConfig,
    #[serde(rename = "dismissedAlerts")]
    pub dismissed_alerts: Vec<String>,
    #[serde(rename = "dismissedNow")]
    pub dismissed_now: std::collections::HashMap<String, Number>,
    pub error: Option<String>,
    pub event: serde_json::Value,
    pub events: Vec<serde_json::Value>,
    #[serde(rename = "expandedStacks")]
    pub expanded_stacks: std::collections::HashMap<String, bool>,
    #[serde(rename = "folderOrder")]
    pub folder_order: Vec<String>,
    pub folders: std::collections::HashMap<String, LiveFolder>,
    pub hidden: std::collections::HashMap<String, Vec<LiveItem>>,
    pub items: std::collections::HashMap<String, Vec<LiveItem>>,
    pub known: std::collections::HashMap<String, Vec<String>>,
    #[serde(rename = "lastFetch")]
    pub last_fetch: Option<Number>,
    pub loading: bool,
    #[serde(rename = "meetingGroups")]
    pub meeting_groups: std::collections::HashMap<String, MeetingGroup>,
    #[serde(rename = "repoHidden")]
    pub repo_hidden: std::collections::HashMap<String, Vec<LiveItem>>,
    #[serde(rename = "requestError")]
    pub request_error: Option<String>,
    pub requesting: bool,
    #[serde(rename = "showAll")]
    pub show_all: std::collections::HashMap<String, bool>,
    pub status: std::collections::HashMap<String, FolderStatus>,
    pub unread: std::collections::HashMap<String, Vec<String>>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `applyFetch` (an exported function that writes a store).
    #[serde(rename = "applyFetch")]
    ApplyFetch {
        #[serde(rename = "folderId")]
        folder_id: String,
        fetched: Vec<LiveItem>,
        gone: std::collections::HashMap<String, CompletionState>,
        #[serde(default)]
        now: Option<Number>,
        #[serde(default)]
        quiet: Option<Vec<String>>,
        #[serde(default)]
        fence: Option<serde_json::Value>,
        #[serde(rename = "fetchedSources", default)]
        fetched_sources: Option<Vec<LiveSourceId>>,
    },
    /// `arrangeFolders` (an exported function that writes a store).
    #[serde(rename = "arrangeFolders")]
    ArrangeFolders {
        placed: Vec<ArrangeFoldersPlacedItem>,
    },
    /// `connectBitbucket` (an exported function that writes a store).
    #[serde(rename = "connectBitbucket")]
    ConnectBitbucket { user: String, token: String },
    /// `connectCalendar` (an exported function that writes a store).
    #[serde(rename = "connectCalendar")]
    ConnectCalendar,
    /// `connectConfluence` (an exported function that writes a store).
    #[serde(rename = "connectConfluence")]
    ConnectConfluence {
        site: String,
        email: String,
        token: String,
    },
    /// `connectGithub` (an exported function that writes a store).
    #[serde(rename = "connectGithub")]
    ConnectGithub { token: String },
    /// `connectGoogle` (an exported function that writes a store).
    #[serde(rename = "connectGoogle")]
    ConnectGoogle {
        #[serde(rename = "clientId")]
        client_id: String,
        #[serde(rename = "clientSecret")]
        client_secret: String,
        code: String,
        verifier: String,
        #[serde(rename = "redirectUri")]
        redirect_uri: String,
    },
    /// `connectGoogleDrive` (an exported function that writes a store).
    #[serde(rename = "connectGoogleDrive")]
    ConnectGoogleDrive {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "clientId")]
        client_id: String,
        #[serde(rename = "clientSecret")]
        client_secret: String,
    },
    /// `connectNotion` (an exported function that writes a store).
    #[serde(rename = "connectNotion")]
    ConnectNotion { token: String },
    /// `createFolder` (an exported function that writes a store).
    #[serde(rename = "createFolder")]
    CreateFolder {
        #[serde(rename = "profileId")]
        profile_id: String,
        kind: LiveFolderKind,
        #[serde(default)]
        sources: Option<Vec<LiveSourceId>>,
    },
    /// `deleteLiveFolder` (an exported function that writes a store).
    #[serde(rename = "deleteLiveFolder")]
    DeleteLiveFolder {
        #[serde(rename = "folderId")]
        folder_id: String,
    },
    /// `disconnect` (an exported function that writes a store).
    #[serde(rename = "disconnect")]
    Disconnect { source: LiveSourceId },
    /// `dismiss` (an exported function that writes a store).
    #[serde(rename = "dismiss")]
    Dismiss { event: serde_json::Value },
    /// `dismissAlert` (an exported function that writes a store).
    #[serde(rename = "dismissAlert")]
    DismissAlert { occurrence: String },
    /// `dismissNowBadge` (an exported function that writes a store).
    #[serde(rename = "dismissNowBadge")]
    DismissNowBadge {
        event: DismissNowBadgeEvent,
        #[serde(default)]
        now: Option<Number>,
    },
    /// `finishCompleting` (an exported function that writes a store).
    #[serde(rename = "finishCompleting")]
    FinishCompleting {
        #[serde(rename = "folderId")]
        folder_id: String,
        #[serde(rename = "requestedIds")]
        requested_ids: Vec<String>,
        #[serde(default)]
        fence: Option<serde_json::Value>,
    },
    /// `groupCall` (an exported function that writes a store).
    #[serde(rename = "groupCall")]
    GroupCall {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(default)]
        event: Option<serde_json::Value>,
    },
    /// `hideItems` (an exported function that writes a store).
    #[serde(rename = "hideItems")]
    HideItems {
        #[serde(rename = "folderId")]
        folder_id: String,
        hiding: Vec<LiveItem>,
    },
    /// `hideNewRepos` (an exported function that writes a store).
    #[serde(rename = "hideNewRepos")]
    HideNewRepos {
        #[serde(rename = "folderId")]
        folder_id: String,
        fetched: Vec<LiveItem>,
    },
    /// `hideStack` (an exported function that writes a store).
    #[serde(rename = "hideStack")]
    HideStack {
        #[serde(rename = "folderId")]
        folder_id: String,
        #[serde(rename = "stackId")]
        stack_id: String,
    },
    /// `joinEvent` (an exported function that writes a store).
    #[serde(rename = "joinEvent")]
    JoinEvent {
        event: serde_json::Value,
        #[serde(rename = "openAll", default)]
        open_all: Option<bool>,
        #[serde(rename = "windowId", default)]
        window_id: Option<String>,
    },
    /// `joinMeeting` (an exported function that writes a store).
    #[serde(rename = "joinMeeting")]
    JoinMeeting {
        #[serde(rename = "windowId")]
        window_id: String,
        event: serde_json::Value,
        #[serde(rename = "joinUrl")]
        join_url: String,
        #[serde(rename = "openAll", default)]
        open_all: Option<bool>,
    },
    /// `manualRefresh` (an exported function that writes a store). The callback parameters `allowed` are not in the JSON.
    #[serde(rename = "manualRefresh")]
    ManualRefresh {
        #[serde(rename = "folderId")]
        folder_id: String,
    },
    /// `markRead` (an exported function that writes a store).
    #[serde(rename = "markRead")]
    MarkRead {
        #[serde(rename = "folderId")]
        folder_id: String,
        #[serde(rename = "itemIds", default)]
        item_ids: Option<Vec<String>>,
    },
    /// `markUnread` (an exported function that writes a store).
    #[serde(rename = "markUnread")]
    MarkUnread {
        #[serde(rename = "folderId")]
        folder_id: String,
        #[serde(rename = "itemId")]
        item_id: String,
    },
    /// `newLiveFolder` (an exported function that writes a store).
    #[serde(rename = "newLiveFolder")]
    NewLiveFolder {
        #[serde(rename = "windowId")]
        window_id: String,
        kind: LiveFolderKind,
    },
    /// `newProfileLiveFolder` (an exported function that writes a store).
    #[serde(rename = "newProfileLiveFolder")]
    NewProfileLiveFolder {
        #[serde(rename = "profileId")]
        profile_id: String,
        kind: LiveFolderKind,
        #[serde(default)]
        sources: Option<Vec<LiveSourceId>>,
        #[serde(default)]
        options: Option<NewProfileLiveFolderOptions>,
    },
    /// `openLiveItem` (an exported function that writes a store).
    #[serde(rename = "openLiveItem")]
    OpenLiveItem {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "folderId")]
        folder_id: String,
        item: LiveItem,
        #[serde(default)]
        options: Option<OpenLiveItemOptions>,
    },
    /// `refreshCalendar` (an exported function that writes a store).
    #[serde(rename = "refreshCalendar")]
    RefreshCalendar,
    /// `refreshFolder` (an exported function that writes a store).
    #[serde(rename = "refreshFolder")]
    RefreshFolder {
        #[serde(rename = "folderId")]
        folder_id: String,
        #[serde(default)]
        interactive: Option<bool>,
    },
    /// `refreshFolderWhile` (an exported function that writes a store). The callback parameters `allowed` are not in the JSON.
    #[serde(rename = "refreshFolderWhile")]
    RefreshFolderWhile {
        #[serde(rename = "folderId")]
        folder_id: String,
        interactive: bool,
    },
    /// `removeFolder` (an exported function that writes a store).
    #[serde(rename = "removeFolder")]
    RemoveFolder { id: String },
    /// `returnLiveTab` (an exported function that writes a store).
    #[serde(rename = "returnLiveTab")]
    ReturnLiveTab {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `setAccount` (an exported function that writes a store).
    #[serde(rename = "setAccount")]
    SetAccount {
        source: LiveSourceId,
        account: Option<LiveAccount>,
    },
    /// `setCalendarHidden` (an exported function that writes a store).
    #[serde(rename = "setCalendarHidden")]
    SetCalendarHidden { id: String, hidden: bool },
    /// `setLive` (an exported function that writes a store). The callback parameters `patch` are not in the JSON.
    #[serde(rename = "setLive")]
    SetLive {
        #[serde(default)]
        intent: Option<SetLiveIntent>,
    },
    /// `setMeetingGroup` (an exported function that writes a store).
    #[serde(rename = "setMeetingGroup")]
    SetMeetingGroup {
        group: Option<MeetingGroup>,
        #[serde(rename = "groupId", default)]
        group_id: Option<String>,
    },
    /// `setRepoHidden` (an exported function that writes a store).
    #[serde(rename = "setRepoHidden")]
    SetRepoHidden {
        #[serde(rename = "folderId")]
        folder_id: String,
        repo: String,
        hidden: bool,
    },
    /// `setShowNewRepos` (an exported function that writes a store).
    #[serde(rename = "setShowNewRepos")]
    SetShowNewRepos {
        #[serde(rename = "folderId")]
        folder_id: String,
        #[serde(rename = "showNewRepos")]
        show_new_repos: bool,
    },
    /// `setStatus` (an exported function that writes a store).
    #[serde(rename = "setStatus")]
    SetStatus {
        #[serde(rename = "folderId")]
        folder_id: String,
        status: SetStatusStatus,
    },
    /// `showAllIn` (an exported function that writes a store).
    #[serde(rename = "showAllIn")]
    ShowAllIn {
        #[serde(rename = "folderId")]
        folder_id: String,
        section: String,
    },
    /// `showAllRepos` (an exported function that writes a store).
    #[serde(rename = "showAllRepos")]
    ShowAllRepos {
        #[serde(rename = "folderId")]
        folder_id: String,
    },
    /// `startCalendar` (an exported function that writes a store).
    #[serde(rename = "startCalendar")]
    StartCalendar,
    /// `startGithubDeviceFlow` (an exported function that writes a store).
    #[serde(rename = "startGithubDeviceFlow")]
    StartGithubDeviceFlow {
        #[serde(rename = "clientId")]
        client_id: String,
    },
    /// `startLive` (an exported function that writes a store).
    #[serde(rename = "startLive")]
    StartLive,
    /// `startLiveEngine` (an exported function that writes a store).
    #[serde(rename = "startLiveEngine")]
    StartLiveEngine,
    /// `startMeetingAlerts` (an exported function that writes a store).
    #[serde(rename = "startMeetingAlerts")]
    StartMeetingAlerts,
    /// `startMeetingGroups` (an exported function that writes a store).
    #[serde(rename = "startMeetingGroups")]
    StartMeetingGroups,
    /// `syncLiveHomes` (an exported function that writes a store).
    #[serde(rename = "syncLiveHomes")]
    SyncLiveHomes,
    /// `tick` (an exported function that writes a store).
    #[serde(rename = "tick")]
    Tick {
        s: crate::browser::BrowserState,
        now: Number,
    },
    /// `toggleStack` (an exported function that writes a store).
    #[serde(rename = "toggleStack")]
    ToggleStack {
        #[serde(rename = "stackId")]
        stack_id: String,
    },
    /// `unhide` (an exported function that writes a store).
    #[serde(rename = "unhide")]
    Unhide {
        #[serde(rename = "folderId")]
        folder_id: String,
    },
    /// `updateCalendarSettings` (an exported function that writes a store).
    #[serde(rename = "updateCalendarSettings")]
    UpdateCalendarSettings { patch: UpdateCalendarSettingsPatch },
    /// `updateConfig` (an exported function that writes a store).
    #[serde(rename = "updateConfig")]
    UpdateConfig { patch: UpdateConfigPatch },
    /// `updateFolder` (an exported function that writes a store).
    #[serde(rename = "updateFolder")]
    UpdateFolder {
        id: String,
        patch: UpdateFolderPatch,
    },
}

/// Applies `action` to `app`. Slice 2.37 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.37")
}
