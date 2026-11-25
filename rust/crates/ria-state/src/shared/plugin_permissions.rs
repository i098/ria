// Ported from ardia apps/browser/src/lib/pluginPermissions.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The permissions a plugin asks for in `plugin.json`: checking them, listing them, and what one
//! plugin lacks to call another.
//!
//! Changes from Ardia:
//! - `tabs: TabAccess | false` is `Option<TabAccess>` (`None` is `false`).
//! - `NetworkAccess` (`false | "*" | string[]`) is an enum.
//! - `unknown` is `Option<&Json>`: `None` is `undefined`.
//! - `legacyPermissions` leaves out a `"__proto__"` name, as the TS assignment to a plain object
//!   does.

use super::plugin_checks::{Check, LIMITS, only};
use super::plugin_network::{ANY_HOST, host_pattern_covers, host_pattern_problem};
use ria_core::identity::telemetry_wire::{Json, set_entry};
use serde::{Serialize, Serializer};

/// `PLUGIN_PERMISSIONS` (`pluginPermissions.ts:5`).
pub const PLUGIN_PERMISSIONS: [&str; 5] = ["tabs", "fullUrls", "commands", "network", "control"];
/// `PluginPermission` (`pluginPermissions.ts:6`).
pub type PluginPermission = &'static str;
/// `PERMISSION_KEYS` (`pluginPermissions.ts:8`).
pub const PERMISSION_KEYS: [&str; 9] = [
    "tabs",
    "fullUrls",
    "activePage",
    "workspace",
    "commands",
    "storage",
    "secrets",
    "network",
    "control",
];
/// `PermissionKey` (`pluginPermissions.ts:9`).
pub type PermissionKey = &'static str;
/// `TAB_ACCESS` (`pluginPermissions.ts:10`).
pub const TAB_ACCESS: [&str; 2] = ["read", "write"];
/// `TabAccess` (`pluginPermissions.ts:11`).
pub type TabAccess = &'static str;

/// `NetworkAccess` (`pluginPermissions.ts:12`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NetworkAccess {
    /// `false`.
    #[default]
    Off,
    /// `"*"`.
    Any,
    Hosts(Vec<String>),
}

impl Serialize for NetworkAccess {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            NetworkAccess::Off => s.serialize_bool(false),
            NetworkAccess::Any => s.serialize_str(ANY_HOST),
            NetworkAccess::Hosts(hosts) => hosts.serialize(s),
        }
    }
}

fn tabs_out<S: Serializer>(tabs: &Option<TabAccess>, s: S) -> Result<S::Ok, S::Error> {
    match tabs {
        Some(t) => s.serialize_str(t),
        None => s.serialize_bool(false),
    }
}

/// `PluginPermissions` (`pluginPermissions.ts:14`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginPermissions {
    #[serde(serialize_with = "tabs_out")]
    pub tabs: Option<TabAccess>,
    pub full_urls: bool,
    pub active_page: bool,
    pub workspace: bool,
    pub commands: bool,
    pub storage: bool,
    pub secrets: bool,
    pub network: NetworkAccess,
    pub control: bool,
}

/// `NO_PERMISSIONS` (`pluginPermissions.ts:26`).
pub const NO_PERMISSIONS: PluginPermissions = PluginPermissions {
    tabs: None,
    full_urls: false,
    active_page: false,
    workspace: false,
    commands: false,
    storage: false,
    secrets: false,
    network: NetworkAccess::Off,
    control: false,
};

/// `BOOLEAN_KEYS` (`pluginPermissions.ts:28`).
const BOOLEAN_KEYS: [&str; 7] = [
    "fullUrls",
    "activePage",
    "workspace",
    "commands",
    "storage",
    "secrets",
    "control",
];

impl PluginPermissions {
    /// `p[key]` for a key of `BOOLEAN_KEYS`.
    fn flag(&mut self, key: &str) -> &mut bool {
        match key {
            "fullUrls" => &mut self.full_urls,
            "activePage" => &mut self.active_page,
            "workspace" => &mut self.workspace,
            "commands" => &mut self.commands,
            "storage" => &mut self.storage,
            "secrets" => &mut self.secrets,
            _ => &mut self.control,
        }
    }

    fn has(&self, key: &str) -> bool {
        match key {
            "fullUrls" => self.full_urls,
            "activePage" => self.active_page,
            "workspace" => self.workspace,
            "commands" => self.commands,
            "storage" => self.storage,
            "secrets" => self.secrets,
            _ => self.control,
        }
    }
}

/// `legacyPermissions` (`pluginPermissions.ts:30`): the v1 list of names as the object form.
pub fn legacy_permissions(given: Option<&Json>) -> Option<Json> {
    let Some(Json::Array(given)) = given else {
        return given.cloned();
    };
    let mut out = Vec::new();
    for p in given {
        let name = match p {
            Json::String(s) => s.clone(),
            _ => p.stringify(),
        };
        match name.as_str() {
            "tabs" => set_entry(&mut out, "tabs", Json::String("read".to_owned())),
            "network" => set_entry(&mut out, "network", Json::String(ANY_HOST.to_owned())),
            "__proto__" => {}
            _ => set_entry(&mut out, &name, Json::Bool(true)),
        }
    }
    Some(Json::Object(out))
}

/// `networkOf` (`pluginPermissions.ts:42`).
fn network_of(c: &mut Check, value: Option<&Json>) -> NetworkAccess {
    let items = match value {
        None | Some(Json::Bool(false)) => return NetworkAccess::Off,
        Some(Json::Bool(true)) => return NetworkAccess::Any,
        Some(Json::String(s)) if s == ANY_HOST => return NetworkAccess::Any,
        Some(Json::Array(items)) if !items.is_empty() && items.len() <= LIMITS.allowed_hosts => {
            items
        }
        _ => {
            c.problems.push(format!(
                "permissions.network must be false, \"*\" or a list of 1 to {} hosts",
                LIMITS.allowed_hosts
            ));
            return NetworkAccess::Off;
        }
    };
    let mut hosts: Vec<String> = Vec::new();
    for (i, host) in items.iter().enumerate() {
        if let Some(problem) = host_pattern_problem(host) {
            c.problems
                .push(format!("permissions.network[{i}] {problem}"));
            continue;
        }
        let Json::String(host) = host else { continue };
        if host == ANY_HOST {
            c.problems.push(format!(
                "permissions.network[{i}]: use \"*\" on its own instead of in a list"
            ));
        } else if !hosts.contains(host) {
            hosts.push(host.clone());
        }
    }
    if hosts.is_empty() {
        NetworkAccess::Off
    } else {
        NetworkAccess::Hosts(hosts)
    }
}

/// `permissionsOf` (`pluginPermissions.ts:59`).
pub fn permissions_of(c: &mut Check, value: Option<&Json>) -> PluginPermissions {
    let Some(value) = value else {
        return NO_PERMISSIONS;
    };
    let Json::Object(value) = value else {
        c.problems.push(format!(
            "permissions must be an object of {}",
            PERMISSION_KEYS.join(", ")
        ));
        return NO_PERMISSIONS;
    };
    let field = |key: &str| value.iter().find(|(k, _)| k == key).map(|(_, v)| v);
    only(c, "permissions", value, &PERMISSION_KEYS);
    let mut out = NO_PERMISSIONS;
    match field("tabs") {
        None | Some(Json::Bool(false)) => {}
        Some(Json::String(t)) if TAB_ACCESS.contains(&t.as_str()) => {
            out.tabs = TAB_ACCESS.into_iter().find(|a| a == t)
        }
        Some(_) => c
            .problems
            .push("permissions.tabs must be read, write or false".to_owned()),
    }
    for key in BOOLEAN_KEYS {
        match field(key) {
            None => {}
            Some(&Json::Bool(b)) => *out.flag(key) = b,
            Some(_) => c
                .problems
                .push(format!("permissions.{key} must be true or false")),
        }
    }
    out.network = network_of(c, field("network"));
    if out.full_urls && out.tabs.is_none() {
        c.problems
            .push("permissions: fullUrls needs tabs".to_owned());
    }
    if out.workspace && (out.tabs != Some("write") || !out.full_urls) {
        c.problems.push("permissions: workspace needs tabs: write and fullUrls, since it sees and moves every tab".to_owned());
    }
    out
}

/// `grantedPermissions` (`pluginPermissions.ts:83`).
pub fn granted_permissions(p: &PluginPermissions) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(tabs) = p.tabs {
        out.push(
            if tabs == "write" {
                "tabs:write"
            } else {
                "tabs"
            }
            .to_owned(),
        );
    }
    out.extend(
        BOOLEAN_KEYS
            .into_iter()
            .filter(|k| p.has(k))
            .map(str::to_owned),
    );
    match &p.network {
        NetworkAccess::Off => {}
        NetworkAccess::Any => out.push("network".to_owned()),
        NetworkAccess::Hosts(hosts) => out.extend(hosts.iter().map(|h| format!("network:{h}"))),
    }
    out
}

/// `networkCovers` (`pluginPermissions.ts:92`).
fn network_covers(caller: &NetworkAccess, callee: &NetworkAccess) -> bool {
    match (caller, callee) {
        (_, NetworkAccess::Off) | (NetworkAccess::Any, _) => true,
        (NetworkAccess::Off, _) | (_, NetworkAccess::Any) => false,
        (NetworkAccess::Hosts(caller), NetworkAccess::Hosts(callee)) => callee.iter().all(|host| {
            caller
                .iter()
                .any(|pattern| host_pattern_covers(pattern, host))
        }),
    }
}

/// `missingPermissions` (`pluginPermissions.ts:98`): what `caller` lacks to call `callee`.
pub fn missing_permissions(caller: &PluginPermissions, callee: &PluginPermissions) -> Vec<String> {
    let mut missing = Vec::new();
    if let Some(tabs) = callee.tabs
        && (caller.tabs.is_none() || (tabs == "write" && caller.tabs != Some("write")))
    {
        missing.push(
            if tabs == "write" {
                "tabs:write"
            } else {
                "tabs"
            }
            .to_owned(),
        );
    }
    missing.extend(
        BOOLEAN_KEYS
            .into_iter()
            .filter(|k| callee.has(k) && !caller.has(k))
            .map(str::to_owned),
    );
    if !network_covers(&caller.network, &callee.network) {
        missing.push("network".to_owned());
    }
    missing
}

/// `canReadTabs` (`pluginPermissions.ts:106`).
pub fn can_read_tabs(p: &PluginPermissions) -> bool {
    p.tabs.is_some()
}

/// `canOpenTabs` (`pluginPermissions.ts:107`).
pub fn can_open_tabs(p: &PluginPermissions) -> bool {
    p.commands || p.tabs == Some("write")
}

/// `hasNetwork` (`pluginPermissions.ts:108`).
pub fn has_network(p: &PluginPermissions) -> bool {
    p.network != NetworkAccess::Off
}

/// `allowedHostsOf` (`pluginPermissions.ts:109`).
pub fn allowed_hosts_of(p: &PluginPermissions) -> Vec<String> {
    match &p.network {
        NetworkAccess::Off => Vec::new(),
        NetworkAccess::Any => vec![ANY_HOST.to_owned()],
        NetworkAccess::Hosts(hosts) => hosts.clone(),
    }
}
