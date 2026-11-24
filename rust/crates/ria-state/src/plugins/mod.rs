// Ported from ardia apps/browser/src/state/plugins/ at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! State module `plugins` (`apps/browser/src/state/plugins`).
//!
//! `State` has the data fields of the TypeScript store, `Action` has one variant for each exported store action, and
//! `apply` is the reducer. Slice 2.17 writes the body of `apply` and proves it with recorded cases.

use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::kernel::Number;
use crate::kernel::{AppState, Effects};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OfficialPlugin {
    pub name: String,
    pub title: String,
    pub blurb: String,
    pub icon: String,
    pub version: Option<String>,
    pub example: Option<String>,
    pub state: OfficialPluginState,
    pub update: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum OfficialPluginState {
    #[default]
    #[serde(rename = "unsupported")]
    Unsupported,
    #[serde(rename = "installed")]
    Installed,
    #[serde(rename = "available")]
    Available,
    #[serde(rename = "comingSoon")]
    ComingSoon,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PluginListing {
    pub name: String,
    pub manifest: Option<PluginManifest>,
    pub problems: Vec<String>,
    pub hash: Option<String>,
    #[serde(rename = "folderId")]
    pub folder_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub official: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy: Option<bool>,
    #[serde(
        rename = "legacyHash",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub legacy_hash: Option<String>,
    #[serde(
        rename = "ignoresUserFolder",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ignores_user_folder: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PluginManifest {
    pub manifest: Number,
    #[serde(rename = "declaredManifest")]
    pub declared_manifest: Number,
    pub name: String,
    pub title: String,
    pub description: String,
    pub version: Option<String>,
    pub requires: Option<String>,
    pub icon: Option<String>,
    pub keywords: Vec<String>,
    pub telemetry: Option<TelemetryDisclosure>,
    pub permissions: PluginPermissions,
    pub main: Option<MainSpec>,
    pub sidecar: Option<ProcessSpec>,
    #[serde(rename = "activationEvents")]
    pub activation_events: Vec<String>,
    pub contributes: PluginContributions,
    #[serde(rename = "alwaysOn")]
    pub always_on: bool,
    pub official: bool,
    #[serde(rename = "autoInstall")]
    pub auto_install: bool,
    pub settings: SettingsSpec,
    pub commands: Vec<CommandSpec>,
    pub tools: Vec<ToolSpec>,
    pub hooks: Vec<String>,
    pub programs: Vec<ProgramSpec>,
    #[serde(rename = "passwordManager")]
    pub password_manager: Option<PasswordManagerSpec>,
    pub page: Option<PageSpec>,
    pub process: Option<ProcessSpec>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TelemetryDisclosure {
    pub title: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PluginPermissions {
    pub tabs: PluginPermissionsTabs,
    #[serde(rename = "fullUrls")]
    pub full_urls: bool,
    #[serde(rename = "activePage")]
    pub active_page: bool,
    pub workspace: bool,
    pub commands: bool,
    pub storage: bool,
    pub secrets: bool,
    pub network: NetworkAccess,
    pub control: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginPermissionsTabs {
    Bool(bool),
    String(String),
}

impl Default for PluginPermissionsTabs {
    fn default() -> Self {
        Self::Bool(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NetworkAccess {
    Bool(bool),
    Other1(Vec<String>),
    String(String),
}

impl Default for NetworkAccess {
    fn default() -> Self {
        Self::Bool(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MainSpec {
    pub entry: String,
    #[serde(rename = "memoryMB")]
    pub memory_mb: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProcessSpec {
    pub run: Vec<String>,
    #[serde(rename = "memoryMB")]
    pub memory_mb: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PluginContributions {
    pub commands: Vec<CommandSpec>,
    pub tools: Vec<ToolSpec>,
    pub hooks: Vec<String>,
    pub settings: SettingsSpec,
    pub programs: Vec<ProgramSpec>,
    #[serde(rename = "passwordManager")]
    pub password_manager: Option<PasswordManagerSpec>,
    pub menus: MenuContributions,
    pub toolbar: Vec<ToolbarItem>,
    #[serde(rename = "sidebarViews")]
    pub sidebar_views: Vec<SidebarView>,
    pub panels: Vec<PanelSpec>,
    pub omnibox: Vec<OmniboxSpec>,
    pub themes: Vec<ThemeSpec>,
    #[serde(rename = "siteSwitches")]
    pub site_switches: Vec<SiteSwitchSpec>,
    #[serde(rename = "pageBar")]
    pub page_bar: Option<PageBarSpec>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CommandSpec {
    pub name: String,
    pub title: String,
    pub icon: String,
    pub keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    pub action: CommandAction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CommandAction {
    Other0(CommandActionVariant0),
    Other1(CommandActionVariant1),
    Other2(CommandActionVariant2),
    Other3(CommandActionVariant3),
    Other4(CommandActionVariant4),
}

impl Default for CommandAction {
    fn default() -> Self {
        Self::Other0(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CommandActionVariant0 {
    pub tool: String,
    pub input: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CommandActionVariant1 {
    pub open: CommandActionVariant1Open,
    pub context: bool,
    pub side: CommandActionVariant1Side,
    #[serde(rename = "sideSetting")]
    pub side_setting: Option<String>,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CommandActionVariant1Open {
    #[default]
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "split")]
    Split,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CommandActionVariant1Side {
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

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CommandActionVariant2 {
    pub view: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CommandActionVariant3 {
    pub panel: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CommandActionVariant4 {
    pub handler: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub title: String,
    pub description: String,
    pub timeout: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SettingsSpec {
    pub defaults: std::collections::HashMap<String, serde_json::Value>,
    pub rows: Vec<RowSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RowSpec {
    #[serde(rename = "toggle")]
    Toggle {
        key: String,
        title: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        default: bool,
    },
    #[serde(rename = "text")]
    Text {
        key: String,
        title: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        default: String,
    },
    #[serde(rename = "number")]
    Number {
        key: String,
        title: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        default: Number,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<Number>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<Number>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        step: Option<Number>,
    },
    #[serde(rename = "choice")]
    Choice {
        key: String,
        title: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        default: String,
        choices: Vec<RowSpecChoiceChoicesItem>,
    },
    #[serde(rename = "secret")]
    Secret {
        key: String,
        title: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        default: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
    },
}

impl Default for RowSpec {
    fn default() -> Self {
        Self::Toggle {
            key: Default::default(),
            title: Default::default(),
            description: Default::default(),
            default: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RowSpecChoiceChoicesItem {
    pub value: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProgramSpec {
    pub name: String,
    pub path: String,
    #[serde(rename = "afterUpdate")]
    pub after_update: Vec<Vec<String>>,
    pub example: Option<String>,
    pub claim: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PasswordManagerSpec {
    pub fill: String,
    pub fields: Vec<PasswordManagerSpecFieldsItem>,
    #[serde(rename = "verificationCodeSetup")]
    pub verification_code_setup: Option<String>,
    pub passkeys: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PasswordManagerSpecFieldsItem {
    #[default]
    #[serde(rename = "login")]
    Login,
    #[serde(rename = "oneTimeCode")]
    OneTimeCode,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MenuContributions {
    pub tab: Vec<MenuContribution>,
    pub group: Vec<MenuContribution>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MenuContribution {
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ToolbarItem {
    pub id: String,
    pub title: String,
    pub icon: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SidebarView {
    pub id: String,
    pub title: String,
    pub icon: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PanelSpec {
    pub id: String,
    pub title: String,
    pub icon: String,
    pub mode: PanelSpecMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    #[serde(rename = "retainContext")]
    pub retain_context: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<PanelSpecSide>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PanelSpecMode {
    #[default]
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "split")]
    Split,
    #[serde(rename = "sidecar")]
    Sidecar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PanelSpecSide {
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

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OmniboxSpec {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
    pub global: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ThemeSpec {
    pub id: String,
    pub title: String,
    pub file: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SiteSwitchSpec {
    pub key: String,
    pub title: String,
    pub icon: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PageBarSpec {
    pub look: String,
    pub when: PageBarSpecWhen,
    pub buttons: Vec<PageBarButton>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PageBarSpecWhen {
    #[serde(rename = "siteSwitch")]
    pub site_switch: Option<String>,
    #[serde(rename = "localAddresses")]
    pub local_addresses: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PageBarButton {
    #[default]
    #[serde(rename = "extensions")]
    Extensions,
    #[serde(rename = "split")]
    Split,
    #[serde(rename = "|")]
    Empty,
    #[serde(rename = "address")]
    Address,
    #[serde(rename = "siteControls")]
    SiteControls,
    #[serde(rename = "copyLink")]
    CopyLink,
    #[serde(rename = "console")]
    Console,
    #[serde(rename = "network")]
    Network,
    #[serde(rename = "inspect")]
    Inspect,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PageSpec {
    pub title: String,
    pub icon: String,
    pub source: PageSpecSource,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PageSpecSource {
    pub file: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CommandView {
    pub plugin: String,
    pub command: String,
    #[serde(rename = "viewId")]
    pub view_id: String,
    pub title: String,
    pub icon: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StateConsent {
    pub id: Number,
    pub name: String,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HandedContext {
    pub plugin: String,
    pub approved: String,
    #[serde(rename = "pageUrl")]
    pub page_url: String,
    pub context: PageContext,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PageContext {
    #[serde(rename = "tabId")]
    pub tab_id: String,
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state")]
pub enum OfficialFetch {
    #[serde(rename = "fetching")]
    Fetching { received: Number, total: Number },
    #[serde(rename = "promoting")]
    Promoting,
    #[serde(rename = "failed")]
    Failed { reason: String, code: String },
}

impl Default for OfficialFetch {
    fn default() -> Self {
        Self::Fetching {
            received: Default::default(),
            total: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InputBoxRequest {
    pub id: String,
    pub plugin: String,
    #[serde(rename = "windowId")]
    pub window_id: String,
    pub options: InputBoxOptions,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InputBoxOptions {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<bool>,
    #[serde(
        rename = "confirmTitle",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub confirm_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Popover {
    pub plugin: String,
    #[serde(rename = "itemId")]
    pub item_id: String,
    #[serde(rename = "viewId")]
    pub view_id: String,
    pub anchor: Anchor,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Anchor {
    pub x: Number,
    pub y: Number,
    pub width: Number,
    pub height: Number,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state")]
pub enum ProcessState {
    #[serde(rename = "starting")]
    Starting { run: Number, since: Number },
    #[serde(rename = "running")]
    Running { run: Number, since: Number },
    #[serde(rename = "waiting")]
    Waiting { until: Number, failures: Number },
    #[serde(rename = "crashed")]
    Crashed { reason: String },
    #[serde(rename = "exited")]
    Exited,
    #[serde(rename = "failed")]
    Failed { reason: String },
}

impl Default for ProcessState {
    fn default() -> Self {
        Self::Starting {
            run: Default::default(),
            since: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state")]
pub enum ProgramsState {
    #[serde(rename = "linking")]
    Linking { hash: String, request: Number },
    #[serde(rename = "linked")]
    Linked {
        hash: String,
        linked: Vec<String>,
        refused: std::collections::HashMap<String, String>,
        #[serde(rename = "binDir")]
        bin_dir: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        adoption: Option<ProgramsStateLinkedAdoption>,
    },
    #[serde(rename = "failed")]
    Failed { hash: String, reason: String },
}

impl Default for ProgramsState {
    fn default() -> Self {
        Self::Linking {
            hash: Default::default(),
            request: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ProgramsStateLinkedAdoption {
    #[default]
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "busy")]
    Busy,
    #[serde(rename = "retrying")]
    Retrying,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct QuickPickRequest {
    pub id: String,
    pub plugin: String,
    #[serde(rename = "windowId")]
    pub window_id: String,
    pub items: Vec<QuickPickItem>,
    pub options: QuickPickOptions,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct QuickPickItem {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub badge: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picked: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct QuickPickOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(
        rename = "canPickMany",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub can_pick_many: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PluginRecord {
    pub enabled: bool,
    pub approved: Option<String>,
    pub values: std::collections::HashMap<String, RowValue>,
    pub applied: std::collections::HashMap<String, PluginRecordAppliedValue>,
    #[serde(
        rename = "controlGranted",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub control_granted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sites: Option<std::collections::HashMap<String, std::collections::HashMap<String, bool>>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RowValue {
    String(String),
    Number(Number),
    Bool(bool),
}

impl Default for RowValue {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PluginRecordAppliedValue {
    pub value: serde_json::Value,
    pub previous: serde_json::Value,
    pub order: Number,
    pub over: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SidePanel {
    pub plugin: String,
    #[serde(rename = "panelId")]
    pub panel_id: String,
    pub opened: Number,
    pub generation: Number,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ToolbarUpdate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub badge: Option<ToolbarUpdateBadge>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tone: Option<ToolbarUpdateTone>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolbarUpdateBadge {
    String(String),
    Number(Number),
    Bool(bool),
}

impl Default for ToolbarUpdateBadge {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ToolbarUpdateTone {
    #[default]
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "accent")]
    Accent,
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "secondary")]
    Secondary,
    #[serde(rename = "tertiary")]
    Tertiary,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "destructive")]
    Destructive,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PluginNode {
    pub r#type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<PluginNodeKey>,
    pub props: std::collections::HashMap<String, serde_json::Value>,
    pub children: Vec<PluginNodeChildrenItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginNodeKey {
    String(String),
    Number(Number),
}

impl Default for PluginNodeKey {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginNodeChildrenItem {
    String(String),
    Other1(Box<PluginNode>),
}

impl Default for PluginNodeChildrenItem {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ChannelCaller {
    pub name: String,
    pub run: Number,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ToolCaller {
    #[serde(rename = "command")]
    Command,
    #[serde(rename = "socket")]
    Socket,
    #[serde(rename = "plugin")]
    Plugin { plugin: String },
    #[serde(rename = "host")]
    Host,
}

impl Default for ToolCaller {
    fn default() -> Self {
        Self::Command
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CommitPluginsPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listings: Option<std::collections::HashMap<String, PluginListing>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub records: Option<std::collections::HashMap<String, PluginRecord>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ConsentOwner {
    #[default]
    #[serde(rename = "settings")]
    Settings,
    #[serde(rename = "onboarding")]
    Onboarding,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstallOptions {
    #[serde(rename = "showSheet", default, skip_serializing_if = "Option::is_none")]
    pub show_sheet: Option<bool>,
    #[serde(rename = "onConsent", default, skip_serializing_if = "Option::is_none")]
    pub on_consent: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alive: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum PluginCall {
    #[serde(rename = "tool")]
    Tool {
        tool: String,
        input: serde_json::Value,
    },
    #[serde(rename = "command")]
    Command {
        command: String,
        context: std::collections::HashMap<String, serde_json::Value>,
    },
    #[serde(rename = "omnibox")]
    Omnibox { provider: String, query: String },
    #[serde(rename = "decide")]
    Decide {
        event: String,
        data: std::collections::HashMap<String, serde_json::Value>,
    },
}

impl Default for PluginCall {
    fn default() -> Self {
        Self::Tool {
            tool: Default::default(),
            input: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InvokePluginCallOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activate: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consent: Option<crate::telemetry::ConsentToken>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signal: Option<crate::automation::DecideSignal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SurfaceKind {
    #[default]
    #[serde(rename = "popover")]
    Popover,
    #[serde(rename = "panel")]
    Panel,
    #[serde(rename = "command")]
    Command,
    #[serde(rename = "sidebar")]
    Sidebar,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OpenPluginPageHow {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<OpenPluginPageHowSide>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consent: Option<crate::telemetry::ConsentToken>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum OpenPluginPageHowSide {
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

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PageBarMatch {
    pub plugin: String,
    pub official: bool,
    pub spec: PageBarSpec,
    pub url: String,
    pub at: Number,
    pub superseded: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Caller {
    pub name: String,
    pub run: Number,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RemovalIdentity {
    pub official: bool,
    #[serde(rename = "folderId")]
    pub folder_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RunHostActionAction {
    #[default]
    #[serde(rename = "extensions")]
    Extensions,
    #[serde(rename = "split")]
    Split,
    #[serde(rename = "address")]
    Address,
    #[serde(rename = "siteControls")]
    SiteControls,
    #[serde(rename = "copyLink")]
    CopyLink,
    #[serde(rename = "console")]
    Console,
    #[serde(rename = "network")]
    Network,
    #[serde(rename = "inspect")]
    Inspect,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RunPluginCommandWhen {
    #[serde(rename = "urlHost", default, skip_serializing_if = "Option::is_none")]
    pub url_host: Option<RunPluginCommandWhenUrlHost>,
    #[serde(rename = "urlScheme", default, skip_serializing_if = "Option::is_none")]
    pub url_scheme: Option<RunPluginCommandWhenUrlScheme>,
    #[serde(rename = "isPinned", default, skip_serializing_if = "Option::is_none")]
    pub is_pinned: Option<RunPluginCommandWhenIsPinned>,
    #[serde(rename = "isInGroup", default, skip_serializing_if = "Option::is_none")]
    pub is_in_group: Option<RunPluginCommandWhenIsInGroup>,
    #[serde(rename = "isSplit", default, skip_serializing_if = "Option::is_none")]
    pub is_split: Option<RunPluginCommandWhenIsSplit>,
    #[serde(rename = "isActive", default, skip_serializing_if = "Option::is_none")]
    pub is_active: Option<RunPluginCommandWhenIsActive>,
    #[serde(rename = "tabCount", default, skip_serializing_if = "Option::is_none")]
    pub tab_count: Option<RunPluginCommandWhenTabCount>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RunPluginCommandWhenUrlHost {
    String(String),
    Number(Number),
    Bool(bool),
}

impl Default for RunPluginCommandWhenUrlHost {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RunPluginCommandWhenUrlScheme {
    String(String),
    Number(Number),
    Bool(bool),
}

impl Default for RunPluginCommandWhenUrlScheme {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RunPluginCommandWhenIsPinned {
    String(String),
    Number(Number),
    Bool(bool),
}

impl Default for RunPluginCommandWhenIsPinned {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RunPluginCommandWhenIsInGroup {
    String(String),
    Number(Number),
    Bool(bool),
}

impl Default for RunPluginCommandWhenIsInGroup {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RunPluginCommandWhenIsSplit {
    String(String),
    Number(Number),
    Bool(bool),
}

impl Default for RunPluginCommandWhenIsSplit {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RunPluginCommandWhenIsActive {
    String(String),
    Number(Number),
    Bool(bool),
}

impl Default for RunPluginCommandWhenIsActive {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RunPluginCommandWhenTabCount {
    String(String),
    Number(Number),
    Bool(bool),
}

impl Default for RunPluginCommandWhenTabCount {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PluginMenuContext {
    pub when: RunPluginCommandWhen,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state")]
pub enum SetOfficialFetchState {
    #[serde(rename = "fetching")]
    Fetching { received: Number, total: Number },
    #[serde(rename = "promoting")]
    Promoting,
    #[serde(rename = "failed")]
    Failed { reason: String, code: String },
}

impl Default for SetOfficialFetchState {
    fn default() -> Self {
        Self::Fetching {
            received: Default::default(),
            total: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SetPluginValueValue {
    String(String),
    Number(Number),
    Bool(bool),
}

impl Default for SetPluginValueValue {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetSiteSwitchBy {
    #[default]
    #[serde(rename = "typed")]
    Typed,
    #[serde(rename = "you")]
    You,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Caller2 {
    pub name: String,
    pub run: Number,
}

/// The data fields of the TypeScript store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct State {
    pub candidates: std::collections::HashMap<String, PluginListing>,
    #[serde(rename = "commandViews")]
    pub command_views: std::collections::HashMap<String, CommandView>,
    pub consent: Option<StateConsent>,
    pub contexts: std::collections::HashMap<String, HandedContext>,
    #[serde(rename = "failClosed")]
    pub fail_closed: bool,
    pub fetches: std::collections::HashMap<String, OfficialFetch>,
    #[serde(rename = "inputBox")]
    pub input_box: Option<InputBoxRequest>,
    pub listings: std::collections::HashMap<String, PluginListing>,
    pub loaded: bool,
    pub order: Vec<String>,
    pub owners: Vec<String>,
    pub pinned: Option<Vec<serde_json::Value>>,
    pub popovers: std::collections::HashMap<String, Popover>,
    pub processes: std::collections::HashMap<String, ProcessState>,
    pub programs: std::collections::HashMap<String, ProgramsState>,
    #[serde(rename = "quickPick")]
    pub quick_pick: Option<QuickPickRequest>,
    pub records: std::collections::HashMap<String, PluginRecord>,
    #[serde(rename = "retainedSidePanels")]
    pub retained_side_panels: std::collections::HashMap<String, Vec<SidePanel>>,
    pub scanned: bool,
    #[serde(rename = "sidePanels")]
    pub side_panels: std::collections::HashMap<String, SidePanel>,
    #[serde(rename = "toolbarState")]
    pub toolbar_state: std::collections::HashMap<String, ToolbarUpdate>,
    pub trees: std::collections::HashMap<String, PluginNode>,
    pub unsaved: Option<String>,
}

/// One variant for each exported store action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// `adoptExactApproval` (an exported function that writes a store).
    #[serde(rename = "adoptExactApproval")]
    AdoptExactApproval {
        name: String,
        candidate: PluginListing,
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `adoptLegacyInstalls` (an exported function that writes a store).
    #[serde(rename = "adoptLegacyInstalls")]
    AdoptLegacyInstalls {
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `answerMethod` (an exported function that writes a store).
    #[serde(rename = "answerMethod")]
    AnswerMethod {
        c: ChannelCaller,
        m: PluginManifest,
        method: String,
        params: std::collections::HashMap<String, serde_json::Value>,
    },
    /// `applyListings` (an exported function that writes a store).
    #[serde(rename = "applyListings")]
    ApplyListings {
        given: Vec<PluginListing>,
        consent: Option<crate::telemetry::ConsentToken>,
        #[serde(default)]
        candidates: Option<Vec<PluginListing>>,
    },
    /// `applySettingsPatch` (an exported function that writes a store).
    #[serde(rename = "applySettingsPatch")]
    ApplySettingsPatch {
        patch: crate::onboarding::StateDraft,
    },
    /// `callTool` (an exported function that writes a store).
    #[serde(rename = "callTool")]
    CallTool {
        plugin: String,
        tool: String,
        input: serde_json::Value,
        caller: ToolCaller,
        #[serde(rename = "consentTaken", default)]
        consent_taken: Option<crate::telemetry::ConsentToken>,
    },
    /// `cancelConsentOnEscape` (an exported function that writes a store).
    #[serde(rename = "cancelConsentOnEscape")]
    CancelConsentOnEscape {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "settingsWindowId")]
        settings_window_id: String,
    },
    /// `clearPluginSurfaces` (an exported function that writes a store).
    #[serde(rename = "clearPluginSurfaces")]
    ClearPluginSurfaces { plugin: String },
    /// `clearUnsaved` (an exported function that writes a store).
    #[serde(rename = "clearUnsaved")]
    ClearUnsaved { name: String },
    /// `closePluginCommandView` (an exported function that writes a store).
    #[serde(rename = "closePluginCommandView")]
    ClosePluginCommandView {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `closePluginPanel` (an exported function that writes a store).
    #[serde(rename = "closePluginPanel")]
    ClosePluginPanel {
        #[serde(rename = "windowId")]
        window_id: String,
        plugin: String,
        #[serde(rename = "panelId")]
        panel_id: String,
    },
    /// `closePluginPopover` (an exported function that writes a store).
    #[serde(rename = "closePluginPopover")]
    ClosePluginPopover {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `closePluginSidePanel` (an exported function that writes a store).
    #[serde(rename = "closePluginSidePanel")]
    ClosePluginSidePanel {
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(default)]
        plugin: Option<String>,
    },
    /// `commitPlugins` (an exported function that writes a store).
    #[serde(rename = "commitPlugins")]
    CommitPlugins { patch: CommitPluginsPatch },
    /// `confirmConsentAs` (an exported function that writes a store).
    #[serde(rename = "confirmConsentAs")]
    ConfirmConsentAs { owner: ConsentOwner, id: Number },
    /// `confirmConsentRequest` (an exported function that writes a store).
    #[serde(rename = "confirmConsentRequest")]
    ConfirmConsentRequest { id: Number },
    /// `consumeTypedAddress` (an exported function that writes a store).
    #[serde(rename = "consumeTypedAddress")]
    ConsumeTypedAddress {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(rename = "committedUrl")]
        committed_url: String,
        #[serde(rename = "committedAt")]
        committed_at: Number,
        #[serde(rename = "privatePage")]
        private_page: bool,
    },
    /// `disablePlugin` (an exported function that writes a store).
    #[serde(rename = "disablePlugin")]
    DisablePlugin { name: String },
    /// `disablePlugin` (an exported function that writes a store).
    #[serde(skip)] // same name as an earlier action
    LifecycleDisablePlugin {
        name: String,
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `dismissConsentAs` (an exported function that writes a store).
    #[serde(rename = "dismissConsentAs")]
    DismissConsentAs { owner: ConsentOwner, id: Number },
    /// `dismissConsentRequest` (an exported function that writes a store).
    #[serde(rename = "dismissConsentRequest")]
    DismissConsentRequest { id: Number },
    /// `dismissPluginConsent` (an exported function that writes a store).
    #[serde(rename = "dismissPluginConsent")]
    DismissPluginConsent {
        #[serde(default)]
        name: Option<String>,
    },
    /// `dropConnections` (an exported function that writes a store).
    #[serde(rename = "dropConnections")]
    DropConnections { name: String },
    /// `dropHiddenRetainedSidePanels` (an exported function that writes a store).
    #[serde(rename = "dropHiddenRetainedSidePanels")]
    DropHiddenRetainedSidePanels { plugin: String },
    /// `dropStalePageContexts` (an exported function that writes a store).
    #[serde(rename = "dropStalePageContexts")]
    DropStalePageContexts,
    /// `dropWindowPluginSurfaces` (an exported function that writes a store).
    #[serde(rename = "dropWindowPluginSurfaces")]
    DropWindowPluginSurfaces {
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `enablePlugin` (an exported function that writes a store).
    #[serde(rename = "enablePlugin")]
    EnablePlugin { name: String, hash: String },
    /// `endPageDocument` (an exported function that writes a store).
    #[serde(rename = "endPageDocument")]
    EndPageDocument {
        #[serde(rename = "tabId")]
        tab_id: String,
        reporter: serde_json::Value,
    },
    /// `ensurePluginActivated` (an exported function that writes a store).
    #[serde(rename = "ensurePluginActivated")]
    EnsurePluginActivated {
        name: String,
        trigger: String,
        #[serde(rename = "timeoutMs", default)]
        timeout_ms: Option<Number>,
    },
    /// `fetchOfficialCandidate` (an exported function that writes a store).
    #[serde(rename = "fetchOfficialCandidate")]
    FetchOfficialCandidate {
        name: String,
        consent: Option<crate::telemetry::ConsentToken>,
        #[serde(default)]
        quiet: Option<bool>,
    },
    /// `forgetAllPageDocuments` (an exported function that writes a store).
    #[serde(rename = "forgetAllPageDocuments")]
    ForgetAllPageDocuments,
    /// `forgetPageDocument` (an exported function that writes a store).
    #[serde(rename = "forgetPageDocument")]
    ForgetPageDocument {
        #[serde(rename = "tabId")]
        tab_id: String,
        reporter: serde_json::Value,
    },
    /// `handlePluginLine` (an exported function that writes a store).
    #[serde(rename = "handlePluginLine")]
    HandlePluginLine { __0: serde_json::Value },
    /// `installOfficialPlugin` (an exported function that writes a store).
    #[serde(rename = "installOfficialPlugin")]
    InstallOfficialPlugin {
        name: String,
        #[serde(default)]
        options: Option<InstallOptions>,
    },
    /// `invokePluginCall` (an exported function that writes a store).
    #[serde(rename = "invokePluginCall")]
    InvokePluginCall {
        plugin: String,
        call: PluginCall,
        caller: ToolCaller,
        #[serde(rename = "timeoutMs")]
        timeout_ms: Number,
        #[serde(default)]
        options: Option<InvokePluginCallOptions>,
    },
    /// `loadPluginRecords` (an exported function that writes a store).
    #[serde(rename = "loadPluginRecords")]
    LoadPluginRecords,
    /// `loadProgramClaims` (an exported function that writes a store).
    #[serde(rename = "loadProgramClaims")]
    LoadProgramClaims,
    /// `mountPluginSurface` (an exported function that writes a store).
    #[serde(rename = "mountPluginSurface")]
    MountPluginSurface {
        plugin: String,
        #[serde(rename = "surfaceId")]
        surface_id: String,
        kind: SurfaceKind,
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(default)]
        context: Option<std::collections::HashMap<String, serde_json::Value>>,
        #[serde(rename = "viewId", default)]
        view_id: Option<String>,
    },
    /// `notePageDocument` (an exported function that writes a store).
    #[serde(rename = "notePageDocument")]
    NotePageDocument {
        #[serde(rename = "tabId")]
        tab_id: String,
        #[serde(rename = "shownUrl")]
        shown_url: String,
        #[serde(rename = "committedUrl")]
        committed_url: String,
        #[serde(rename = "committedAt")]
        committed_at: Number,
        #[serde(rename = "privatePage")]
        private_page: bool,
        reporter: serde_json::Value,
        #[serde(default)]
        now: Option<Number>,
    },
    /// `openPluginCommandView` (an exported function that writes a store).
    #[serde(rename = "openPluginCommandView")]
    OpenPluginCommandView {
        #[serde(rename = "windowId")]
        window_id: String,
        view: CommandView,
    },
    /// `openPluginPage` (an exported function that writes a store).
    #[serde(rename = "openPluginPage")]
    OpenPluginPage {
        #[serde(rename = "windowId")]
        window_id: String,
        name: String,
        #[serde(default)]
        how: Option<OpenPluginPageHow>,
    },
    /// `openPluginPanel` (an exported function that writes a store).
    #[serde(rename = "openPluginPanel")]
    OpenPluginPanel {
        #[serde(rename = "windowId")]
        window_id: String,
        plugin: String,
        #[serde(rename = "panelId")]
        panel_id: String,
        #[serde(default)]
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `openPluginPopover` (an exported function that writes a store).
    #[serde(rename = "openPluginPopover")]
    OpenPluginPopover {
        #[serde(rename = "windowId")]
        window_id: String,
        plugin: String,
        #[serde(rename = "itemId")]
        item_id: String,
        #[serde(rename = "viewId")]
        view_id: String,
        anchor: Anchor,
    },
    /// `openPluginSidePanel` (an exported function that writes a store).
    #[serde(rename = "openPluginSidePanel")]
    OpenPluginSidePanel {
        #[serde(rename = "windowId")]
        window_id: String,
        plugin: String,
        #[serde(rename = "panelId")]
        panel_id: String,
        #[serde(default)]
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `pluginPageOf` (an exported function that writes a store).
    #[serde(rename = "pluginPageOf")]
    PluginPageOf { url: Option<String> },
    /// `pluginSocketMethod` (an exported function that writes a store). The callback parameters `serving` are not in the JSON.
    #[serde(rename = "pluginSocketMethod")]
    PluginSocketMethod {
        method: String,
        params: std::collections::HashMap<String, serde_json::Value>,
    },
    /// `presentSettingsConsent` (an exported function that writes a store).
    #[serde(rename = "presentSettingsConsent")]
    PresentSettingsConsent,
    /// `processExited` (an exported function that writes a store).
    #[serde(rename = "processExited")]
    ProcessExited {
        exit: serde_json::Value,
        #[serde(default)]
        now: Option<Number>,
    },
    /// `promoteCandidate` (an exported function that writes a store).
    #[serde(rename = "promoteCandidate")]
    PromoteCandidate {
        name: String,
        candidate: PluginListing,
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `publishPageBars` (an exported function that writes a store).
    #[serde(rename = "publishPageBars")]
    PublishPageBars {
        #[serde(rename = "windowId")]
        window_id: String,
        bars: std::collections::HashMap<String, PageBarMatch>,
    },
    /// `readActivePage` (an exported function that writes a store).
    #[serde(rename = "readActivePage")]
    ReadActivePage { c: Caller, m: PluginManifest },
    /// `reconcileOfficial` (an exported function that writes a store).
    #[serde(rename = "reconcileOfficial")]
    ReconcileOfficial {
        consent: Option<crate::telemetry::ConsentToken>,
        #[serde(default)]
        now: Option<Number>,
    },
    /// `reconcilePluginOmnibox` (an exported function that writes a store).
    #[serde(rename = "reconcilePluginOmnibox")]
    ReconcilePluginOmnibox {
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `reconcilePlugins` (an exported function that writes a store).
    #[serde(rename = "reconcilePlugins")]
    ReconcilePlugins {
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `reconcilePrograms` (an exported function that writes a store).
    #[serde(rename = "reconcilePrograms")]
    ReconcilePrograms {
        #[serde(default)]
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `refreshLegacyInstalls` (an exported function that writes a store).
    #[serde(rename = "refreshLegacyInstalls")]
    RefreshLegacyInstalls,
    /// `releaseOnboardingConsent` (an exported function that writes a store).
    #[serde(rename = "releaseOnboardingConsent")]
    ReleaseOnboardingConsent,
    /// `releaseOrphanedConsent` (an exported function that writes a store).
    #[serde(rename = "releaseOrphanedConsent")]
    ReleaseOrphanedConsent,
    /// `relinkOwnPrograms` (an exported function that writes a store).
    #[serde(rename = "relinkOwnPrograms")]
    RelinkOwnPrograms { name: String },
    /// `relinkPrograms` (an exported function that writes a store).
    #[serde(rename = "relinkPrograms")]
    RelinkPrograms { name: String },
    /// `removePlugin` (an exported function that writes a store). The callback parameters `mayGoOn` are not in the JSON.
    #[serde(rename = "removePlugin")]
    RemovePlugin {
        name: String,
        #[serde(default)]
        expected: Option<RemovalIdentity>,
    },
    /// `removePluginAfterConfirm` (an exported function that writes a store).
    #[serde(rename = "removePluginAfterConfirm")]
    RemovePluginAfterConfirm {
        name: String,
        title: String,
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `removePrograms` (an exported function that writes a store).
    #[serde(rename = "removePrograms")]
    RemovePrograms {
        name: String,
        #[serde(default)]
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `replayPageDocuments` (an exported function that writes a store).
    #[serde(rename = "replayPageDocuments")]
    ReplayPageDocuments {
        #[serde(default)]
        now: Option<Number>,
    },
    /// `requestPluginConsent` (an exported function that writes a store).
    #[serde(rename = "requestPluginConsent")]
    RequestPluginConsent {
        name: String,
        #[serde(default)]
        official: Option<bool>,
    },
    /// `rescanPlugins` (an exported function that writes a store).
    #[serde(rename = "rescanPlugins")]
    RescanPlugins {
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `resolveInputBox` (an exported function that writes a store).
    #[serde(rename = "resolveInputBox")]
    ResolveInputBox { id: String, value: Option<String> },
    /// `resolveQuickPick` (an exported function that writes a store).
    #[serde(rename = "resolveQuickPick")]
    ResolveQuickPick {
        id: String,
        picked: Option<Vec<String>>,
    },
    /// `restartProcess` (an exported function that writes a store).
    #[serde(rename = "restartProcess")]
    RestartProcess { name: String },
    /// `runHostAction` (an exported function that writes a store).
    #[serde(rename = "runHostAction")]
    RunHostAction {
        action: RunHostActionAction,
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    /// `runPluginAction` (an exported function that writes a store).
    #[serde(rename = "runPluginAction")]
    RunPluginAction {
        id: String,
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `runPluginCommand` (an exported function that writes a store).
    #[serde(rename = "runPluginCommand")]
    RunPluginCommand {
        name: String,
        #[serde(rename = "commandName")]
        command_name: String,
        #[serde(rename = "windowId")]
        window_id: String,
        #[serde(default)]
        context: Option<std::collections::HashMap<String, serde_json::Value>>,
        #[serde(default)]
        when: Option<RunPluginCommandWhen>,
        #[serde(default)]
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `runPluginDecideHook` (an exported function that writes a store).
    #[serde(rename = "runPluginDecideHook")]
    RunPluginDecideHook {
        event: String,
        data: std::collections::HashMap<String, serde_json::Value>,
        consent: Option<crate::telemetry::ConsentToken>,
        #[serde(default)]
        signal: Option<crate::automation::DecideSignal>,
    },
    /// `runPluginMenuChoice` (an exported function that writes a store).
    #[serde(rename = "runPluginMenuChoice")]
    RunPluginMenuChoice {
        choice: String,
        #[serde(rename = "windowId")]
        window_id: String,
        context: PluginMenuContext,
        #[serde(default)]
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `runPluginToolbarItem` (an exported function that writes a store).
    #[serde(rename = "runPluginToolbarItem")]
    RunPluginToolbarItem {
        #[serde(rename = "windowId")]
        window_id: String,
        plugin: String,
        #[serde(rename = "itemId")]
        item_id: String,
        anchor: Anchor,
    },
    /// `setOfficialFetch` (an exported function that writes a store).
    #[serde(rename = "setOfficialFetch")]
    SetOfficialFetch {
        name: String,
        state: Option<SetOfficialFetchState>,
    },
    /// `setPluginSurfaceTree` (an exported function that writes a store).
    #[serde(rename = "setPluginSurfaceTree")]
    SetPluginSurfaceTree {
        plugin: String,
        #[serde(rename = "surfaceId")]
        surface_id: String,
        tree: PluginNode,
    },
    /// `setPluginValue` (an exported function that writes a store).
    #[serde(rename = "setPluginValue")]
    SetPluginValue {
        name: String,
        key: String,
        value: SetPluginValueValue,
    },
    /// `setSiteSwitch` (an exported function that writes a store).
    #[serde(rename = "setSiteSwitch")]
    SetSiteSwitch {
        plugin: String,
        key: String,
        url: String,
        on: bool,
        consent: Option<crate::telemetry::ConsentToken>,
        #[serde(default)]
        by: Option<SetSiteSwitchBy>,
    },
    /// `showPluginInputBox` (an exported function that writes a store).
    #[serde(rename = "showPluginInputBox")]
    ShowPluginInputBox {
        plugin: String,
        #[serde(rename = "windowId")]
        window_id: String,
        options: InputBoxOptions,
    },
    /// `showPluginQuickPick` (an exported function that writes a store).
    #[serde(rename = "showPluginQuickPick")]
    ShowPluginQuickPick {
        plugin: String,
        #[serde(rename = "windowId")]
        window_id: String,
        items: Vec<QuickPickItem>,
        options: QuickPickOptions,
    },
    /// `startChannels` (an exported function that writes a store).
    #[serde(rename = "startChannels")]
    StartChannels,
    /// `startPlugins` (an exported function that writes a store).
    #[serde(rename = "startPlugins")]
    StartPlugins,
    /// `stopProcess` (an exported function that writes a store).
    #[serde(rename = "stopProcess")]
    StopProcess {
        name: String,
        consent: Option<crate::telemetry::ConsentToken>,
        #[serde(rename = "graceMs", default)]
        grace_ms: Option<Number>,
    },
    /// `superviseProcess` (an exported function that writes a store).
    #[serde(rename = "superviseProcess")]
    SuperviseProcess {
        name: String,
        hash: String,
        manifest: PluginManifest,
        #[serde(rename = "folderId")]
        folder_id: String,
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `turnOffPlugin` (an exported function that writes a store).
    #[serde(rename = "turnOffPlugin")]
    TurnOffPlugin {
        name: String,
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `turnOnApprovedPlugin` (an exported function that writes a store).
    #[serde(rename = "turnOnApprovedPlugin")]
    TurnOnApprovedPlugin { name: String },
    /// `uiMethod` (an exported function that writes a store).
    #[serde(rename = "uiMethod")]
    UiMethod {
        c: Caller2,
        m: PluginManifest,
        method: String,
        params: std::collections::HashMap<String, serde_json::Value>,
    },
    /// `unmountPluginSurface` (an exported function that writes a store).
    #[serde(rename = "unmountPluginSurface")]
    UnmountPluginSurface {
        plugin: String,
        #[serde(rename = "surfaceId")]
        surface_id: String,
        #[serde(rename = "windowId")]
        window_id: String,
    },
    /// `updatePluginToolbar` (an exported function that writes a store).
    #[serde(rename = "updatePluginToolbar")]
    UpdatePluginToolbar {
        plugin: String,
        #[serde(rename = "itemId")]
        item_id: String,
        update: ToolbarUpdate,
    },
    /// `updateRecord` (an exported function that writes a store). The callback parameters `change` are not in the JSON.
    #[serde(rename = "updateRecord")]
    UpdateRecord {
        name: String,
        consent: Option<crate::telemetry::ConsentToken>,
    },
    /// `wakeForFilePage` (an exported function that writes a store).
    #[serde(rename = "wakeForFilePage")]
    WakeForFilePage {
        name: String,
        #[serde(default)]
        path: Option<String>,
    },
    /// `watchActivation` (an exported function that writes a store).
    #[serde(rename = "watchActivation")]
    WatchActivation,
    /// `watchPageBarCommits` (an exported function that writes a store).
    #[serde(rename = "watchPageBarCommits")]
    WatchPageBarCommits,
    /// `watchPageContexts` (an exported function that writes a store).
    #[serde(rename = "watchPageContexts")]
    WatchPageContexts,
    /// `watchPluginFolder` (an exported function that writes a store).
    #[serde(rename = "watchPluginFolder")]
    WatchPluginFolder,
    /// `watchPluginOmnibox` (an exported function that writes a store).
    #[serde(rename = "watchPluginOmnibox")]
    WatchPluginOmnibox,
    /// `watchSurfaceRemounts` (an exported function that writes a store).
    #[serde(rename = "watchSurfaceRemounts")]
    WatchSurfaceRemounts,
    /// `withdrawPageBars` (an exported function that writes a store).
    #[serde(rename = "withdrawPageBars")]
    WithdrawPageBars {
        #[serde(rename = "windowId")]
        window_id: String,
    },
}

/// Applies `action` to `app`. Slice 2.17 writes the body.
pub fn apply(app: &mut AppState, action: Action, fx: &mut Effects) {
    let _ = (app, action, fx);
    todo!("slice 2.17")
}
