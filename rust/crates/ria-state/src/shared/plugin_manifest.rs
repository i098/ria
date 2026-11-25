// Ported from ardia apps/browser/src/lib/pluginManifest.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! `plugin.json`: read the text, upgrade an old manifest, check every field, and list what a
//! plugin contributes.
//!
//! Changes from Ardia:
//! - The re-exports of `pluginManifest.ts` (`LIMITS`, `ROW_TYPES`, `SPLIT_SIDES`, ...) are not
//!   repeated: callers use `plugin_checks`, `plugin_rows`, `plugin_commands` and the others.
//! - `unknown` is `Option<&Json>` as in `plugin_checks`; a parsed document is `Fields`.
//! - `JSON.parse` is `ria_core::search::bangs::parse_json`, which gives V8's messages.
//! - The results derive `Serialize` with the TS field names, so `serde_json::to_value` gives the
//!   TS object.

use ria_core::identity::telemetry_wire::{Json, js_number_to_string, set_entry};
use ria_core::search::bangs::{Json as Parsed, parse_json};
use serde::Serialize;

use super::decode::{Fields, is_fields};
use super::plugin_activation::{activation_events_of, derived_activation_events};
use super::plugin_checks::{
    Check, LIMITS, NAME, NAME_RULE, SYMBOL, SYMBOL_RULE, VERSION, list, matching, object, only,
    text,
};
use super::plugin_commands::{CommandAction, CommandSpec, SPLIT_SIDES, ToolSpec};
use super::plugin_contributions::{MENU_SLOTS, PluginContributions, contributes_of, page_panel_of};
use super::plugin_migrations::{manifest_migrations, v1_problem};
use super::plugin_names::RESERVED_PAGE_NAMES;
use super::plugin_password_manager::PasswordManagerSpec;
use super::plugin_permissions::{NetworkAccess, PluginPermissions, permissions_of};
use super::plugin_rows::{RowKind, SettingsSpec, stored_rows};
use super::plugin_runtime::{MainSpec, ProcessSpec, ProgramSpec, main_of, sidecar_of};
use super::saved_document::upgraded;

/// `MANIFEST_VERSION` (`pluginManifest.ts:25`).
pub const MANIFEST_VERSION: f64 = 2.0;
/// `MANIFEST_FILE` (`pluginManifest.ts:26`).
pub const MANIFEST_FILE: &str = "plugin.json";

/// The `source` of a [`PageSpec`].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PageSource {
    pub file: String,
}

/// `PageSpec` (`pluginManifest.ts:28`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PageSpec {
    pub title: String,
    pub icon: String,
    pub source: PageSource,
}

/// `TelemetryDisclosure` (`pluginManifest.ts:29`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TelemetryDisclosure {
    pub title: String,
    pub items: Vec<String>,
}

/// `PluginManifest` (`pluginManifest.ts:31`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub manifest: f64,
    pub declared_manifest: f64,
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
    pub activation_events: Vec<String>,
    pub contributes: PluginContributions,
    pub always_on: bool,
    pub official: bool,
    pub auto_install: bool,
    pub settings: SettingsSpec,
    pub commands: Vec<CommandSpec>,
    pub tools: Vec<ToolSpec>,
    pub hooks: Vec<String>,
    pub programs: Vec<ProgramSpec>,
    pub password_manager: Option<PasswordManagerSpec>,
    pub page: Option<PageSpec>,
    pub process: Option<ProcessSpec>,
}

/// The result of `checkManifest` (`pluginManifest.ts:130`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ManifestCheck {
    pub manifest: Option<PluginManifest>,
    pub problems: Vec<String>,
}

/// `ManifestRead` (`pluginManifest.ts:60`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ManifestRead {
    pub manifest: Option<PluginManifest>,
    pub problems: Vec<String>,
    pub newer: bool,
}

const TOP_KEYS: [&str; 19] = [
    "$schema",
    "manifest",
    "name",
    "title",
    "description",
    "version",
    "requires",
    "icon",
    "main",
    "memoryMB",
    "sidecar",
    "activationEvents",
    "permissions",
    "contributes",
    "alwaysOn",
    "official",
    "autoInstall",
    "keywords",
    "telemetry",
];
const V2_ONLY_KEYS: [&str; 2] = ["keywords", "telemetry"];
const K: &str = "contributes";

/// `doc[key]`.
fn field<'a>(doc: &'a Fields, key: &str) -> Option<&'a Json> {
    doc.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// A JS string is truthy when it is not empty.
fn truthy(s: &Option<String>) -> bool {
    s.as_deref().is_some_and(|s| !s.is_empty())
}

/// `commandProblems` (`pluginManifest.ts:66`).
fn command_problems(
    c: &mut Check,
    k: &PluginContributions,
    permissions: &PluginPermissions,
    has_page: bool,
    has_main: bool,
) {
    for (i, x) in k.commands.iter().enumerate() {
        let path = format!("{K}.commands[{i}]");
        match &x.action {
            CommandAction::Tool { tool, .. } => {
                if !k.tools.iter().any(|t| &t.name == tool) {
                    c.problems.push(format!(
                        "{path}.tool: {tool} isn't one of this plugin's tools"
                    ));
                }
            }
            CommandAction::Open {
                context,
                side_setting,
                ..
            } => {
                if !has_page {
                    c.problems.push(format!("{path}.open needs a page"));
                }
                if *context && permissions.tabs.is_none() {
                    c.problems
                        .push(format!("{path}.context needs the tabs permission"));
                }
                if let Some(setting) = side_setting.as_deref().filter(|s| !s.is_empty()) {
                    let row = k.settings.rows.iter().find(|r| r.key == setting);
                    let sides_only = matches!(row.map(|r| &r.kind), Some(RowKind::Choice { choices, .. })
                        if choices.iter().all(|ch| SPLIT_SIDES.contains(&ch.value.as_str())));
                    if !sides_only {
                        c.problems.push(format!("{path}.sideSetting must name a choice row whose values are left, right, top or bottom"));
                    }
                }
            }
            CommandAction::Panel { panel } => {
                if !k.panels.iter().any(|p| &p.id == panel) {
                    c.problems.push(format!(
                        "{path}.panel: {panel} isn't one of this plugin's panels"
                    ));
                }
            }
            action if !has_main => {
                let what = if matches!(action, CommandAction::View { .. }) {
                    "view"
                } else {
                    "handler"
                };
                c.problems.push(format!("{path}.{what} needs a main"));
            }
            _ => {}
        }
    }
}

/// `contributionProblems` (`pluginManifest.ts:84`).
fn contribution_problems(
    c: &mut Check,
    k: &PluginContributions,
    permissions: &PluginPermissions,
    main: bool,
    sidecar: bool,
) {
    let has_command = |name: &str| k.commands.iter().any(|x| x.name == name);
    for slot in MENU_SLOTS {
        for (i, m) in k.menus.slot(slot).iter().enumerate() {
            if !has_command(&m.command) {
                c.problems.push(format!(
                    "{K}.menus.{slot}[{i}].command: {} isn't one of this plugin's commands",
                    m.command
                ));
            }
        }
    }
    for (i, t) in k.toolbar.iter().enumerate() {
        if let Some(command) = t.command.as_deref().filter(|s| !s.is_empty())
            && !has_command(command)
        {
            c.problems.push(format!(
                "{K}.toolbar[{i}].command: {command} isn't one of this plugin's commands"
            ));
        }
        if truthy(&t.view) && !main {
            c.problems
                .push(format!("{K}.toolbar[{i}].view needs a main"));
        }
    }
    for (i, p) in k.panels.iter().enumerate() {
        if truthy(&p.view) && !main {
            c.problems
                .push(format!("{K}.panels[{i}].view needs a main"));
        }
    }
    let needs_main = [
        ("sidebarViews", !k.sidebar_views.is_empty()),
        ("omnibox", !k.omnibox.is_empty()),
    ];
    for (what, used) in needs_main {
        if used && !main {
            c.problems.push(format!("{K}.{what} needs a main"));
        }
    }
    let needs_runtime = [
        (
            "tool commands",
            k.commands
                .iter()
                .any(|x| matches!(x.action, CommandAction::Tool { .. })),
        ),
        ("tools", !k.tools.is_empty()),
        ("hooks", !k.hooks.is_empty()),
    ];
    for (what, used) in needs_runtime {
        if used && !main && !sidecar {
            c.problems.push(format!("{what} needs a main or a sidecar"));
        }
    }
    if k.settings
        .rows
        .iter()
        .any(|r| matches!(r.kind, RowKind::Secret { .. }))
        && !permissions.secrets
    {
        c.problems.push(format!(
            "{K}.settings.rows: a secret row needs the secrets permission"
        ));
    }
}

/// `keywordsOf` (`pluginManifest.ts:99`).
fn keywords_of(c: &mut Check, value: Option<&Json>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (i, k) in list(c, "keywords", value, LIMITS.plugin_keywords)
        .iter()
        .enumerate()
    {
        if let Some(k) = text(c, &format!("keywords[{i}]"), Some(k), LIMITS.keyword, true)
            && !out.iter().any(|o| o == k)
        {
            out.push(k.to_owned());
        }
    }
    out
}

/// `telemetryOf` (`pluginManifest.ts:104`).
fn telemetry_of(c: &mut Check, value: Option<&Json>) -> Option<TelemetryDisclosure> {
    value?;
    let doc = object(c, "telemetry", value)?;
    only(c, "telemetry", doc, &["title", "items"]);
    let title = text(
        c,
        "telemetry.title",
        field(doc, "title"),
        LIMITS.title,
        true,
    );
    let raw = list(
        c,
        "telemetry.items",
        field(doc, "items"),
        LIMITS.telemetry_items,
    );
    if raw.is_empty() {
        c.problems.push(format!(
            "telemetry.items must be a list of 1 to {}",
            LIMITS.telemetry_items
        ));
    }
    let items: Vec<Option<&str>> = raw
        .iter()
        .enumerate()
        .map(|(i, item)| {
            text(
                c,
                &format!("telemetry.items[{i}]"),
                Some(item),
                LIMITS.telemetry_item,
                true,
            )
        })
        .collect();
    let items: Option<Vec<String>> = items.into_iter().map(|i| i.map(str::to_owned)).collect();
    match (title, items) {
        (Some(title), Some(items)) if !raw.is_empty() => Some(TelemetryDisclosure {
            title: title.to_owned(),
            items,
        }),
        _ => None,
    }
}

/// `v2OnlyProblems` (`pluginManifest.ts:116`).
fn v2_only_problems(c: &mut Check, raw: &Fields, programs: &[ProgramSpec], declared: f64) {
    if declared >= MANIFEST_VERSION {
        return;
    }
    let v = js_number_to_string(MANIFEST_VERSION);
    for key in V2_ONLY_KEYS {
        if field(raw, key).is_some() {
            c.problems.push(format!("{key} needs manifest {v}"));
        }
    }
    for (i, p) in programs.iter().enumerate() {
        if p.example.is_some() {
            c.problems
                .push(format!("programs[{i}].example needs manifest {v}"));
        }
        if p.claim {
            c.problems
                .push(format!("programs[{i}].claim needs manifest {v}"));
        }
    }
}

/// `pageOf` (`pluginManifest.ts:125`).
fn page_of(k: &PluginContributions) -> Option<PageSpec> {
    let panel = page_panel_of(&k.panels)?;
    let file = panel.file.as_deref().filter(|f| !f.is_empty())?;
    Some(PageSpec {
        title: panel.title.clone(),
        icon: panel.icon.clone(),
        source: PageSource {
            file: file.to_owned(),
        },
    })
}

/// `checkManifest` (`pluginManifest.ts:130`); Ardia's default `declared_manifest` is
/// [`MANIFEST_VERSION`].
pub fn check_manifest(raw: &Fields, declared_manifest: f64) -> ManifestCheck {
    let mut c = Check::default();
    let c = &mut c;
    let f = |key: &str| field(raw, key);
    only(c, "", raw, &TOP_KEYS);
    let name = matching(c, "name", f("name"), &NAME, NAME_RULE, true);
    // `raw.title ?? raw.name`.
    let title_given = f("title").filter(|t| **t != Json::Null).or(f("name"));
    let title = text(c, "title", title_given, LIMITS.title, true);
    let description = text(
        c,
        "description",
        f("description"),
        LIMITS.description,
        false,
    );
    let version = matching(
        c,
        "version",
        f("version"),
        &VERSION,
        "MAJOR.MINOR.PATCH",
        false,
    );
    let requires = matching(
        c,
        "requires",
        f("requires"),
        &VERSION,
        "an Ardia version, MAJOR.MINOR.PATCH",
        false,
    );
    let icon = matching(c, "icon", f("icon"), &SYMBOL, SYMBOL_RULE, false);
    let keywords = keywords_of(c, f("keywords"));
    let telemetry = telemetry_of(c, f("telemetry"));
    let permissions = permissions_of(c, f("permissions"));
    let main = main_of(c, f("main"), f("memoryMB"));
    let sidecar = sidecar_of(c, "sidecar", f("sidecar"));
    let has_main = main.is_some() || f("main").is_some();
    let has_sidecar = sidecar.is_some() || f("sidecar").is_some();
    if has_main && has_sidecar {
        c.problems
            .push("a plugin can declare main or sidecar, not both".to_owned());
    }
    let contributes = contributes_of(c, f(K), has_main);
    let given = activation_events_of(c, f("activationEvents"));
    if given.is_some() && !has_main {
        c.problems
            .push("activationEvents only go with a main".to_owned());
    }
    let [always_on, official, auto_install] =
        ["alwaysOn", "official", "autoInstall"].map(|k| match f(k) {
            None => false,
            Some(Json::Bool(true)) => true,
            Some(_) => {
                c.problems.push(format!("{k} must be true"));
                false
            }
        });
    let page = page_of(&contributes);
    v2_only_problems(c, raw, &contributes.programs, declared_manifest);
    command_problems(c, &contributes, &permissions, page.is_some(), has_main);
    contribution_problems(c, &contributes, &permissions, has_main, has_sidecar);
    if let Some(name) = name
        && contributes.panels.iter().any(|p| truthy(&p.file))
        && RESERVED_PAGE_NAMES.contains(&name)
    {
        c.problems.push(format!(
            "name: {name} is one of Ardia's own pages (ardia://{name}), so a plugin with a page can't use it"
        ));
    }
    if sidecar.is_some() && matches!(permissions.network, NetworkAccess::Hosts(_)) {
        c.problems.push("permissions.network: a sidecar can only have \"*\" or false, since Ardia can't hold a program to a list of hosts".to_owned());
    }
    let (Some(name), Some(title)) = (name, title) else {
        return ManifestCheck {
            manifest: None,
            problems: std::mem::take(&mut c.problems),
        };
    };
    if !c.problems.is_empty() {
        return ManifestCheck {
            manifest: None,
            problems: std::mem::take(&mut c.problems),
        };
    }
    let activation_events =
        given.unwrap_or_else(|| derived_activation_events(&contributes, has_main, has_sidecar));
    let k = &contributes;
    let manifest = PluginManifest {
        manifest: MANIFEST_VERSION,
        declared_manifest,
        name: name.to_owned(),
        title: title.to_owned(),
        description: description.unwrap_or("").to_owned(),
        version: version.map(str::to_owned),
        requires: requires.map(str::to_owned),
        icon: icon.map(str::to_owned),
        keywords,
        telemetry,
        permissions,
        main,
        sidecar: sidecar.clone(),
        activation_events,
        always_on,
        official,
        auto_install,
        settings: k.settings.clone(),
        commands: k.commands.clone(),
        tools: k.tools.clone(),
        hooks: k.hooks.clone(),
        programs: k.programs.clone(),
        password_manager: k.password_manager.clone(),
        page,
        process: sidecar,
        contributes,
    };
    ManifestCheck {
        manifest: Some(manifest),
        problems: Vec::new(),
    }
}

/// A parsed value as `Json`.
fn json_of(value: &Parsed) -> Json {
    match value {
        Parsed::Null => Json::Null,
        Parsed::Bool(b) => Json::Bool(*b),
        Parsed::Number(n) => Json::Number(*n),
        Parsed::String(s) => Json::String(s.clone()),
        Parsed::Array(items) => Json::Array(items.iter().map(json_of).collect()),
        Parsed::Object(entries) => Json::Object(
            entries
                .iter()
                .map(|(k, v)| (k.clone(), json_of(v)))
                .collect(),
        ),
    }
}

/// A [`ManifestRead`] with one problem and no manifest.
fn refused(problem: String, newer: bool) -> ManifestRead {
    ManifestRead {
        manifest: None,
        problems: vec![problem],
        newer,
    }
}

/// `readManifest` (`pluginManifest.ts:169`).
pub fn read_manifest(text: &str) -> ManifestRead {
    let doc = match parse_json(text) {
        Ok(parsed) => json_of(&parsed),
        Err(error) => {
            return refused(format!("{MANIFEST_FILE} isn't valid JSON ({error})"), false);
        }
    };
    if !is_fields(&doc) {
        return refused(format!("{MANIFEST_FILE} must be an object"), false);
    }
    let given = match &doc {
        Json::Object(fields) => field(fields, "manifest").cloned(),
        _ => None,
    };
    let given = match given {
        None => None,
        Some(Json::Number(n))
            if n.fract() == 0.0 && n.abs() <= 9_007_199_254_740_991.0 && n >= 0.0 =>
        {
            Some(n)
        }
        Some(_) => {
            return refused(
                "manifest must be a whole number, the manifest's version".to_owned(),
                false,
            );
        }
    };
    if let Some(n) = given.filter(|&n| n > MANIFEST_VERSION) {
        return refused(
            format!(
                "manifest {} needs a newer Ardia (this one reads up to {})",
                js_number_to_string(n),
                js_number_to_string(MANIFEST_VERSION)
            ),
            true,
        );
    }
    let Some(current) = upgraded(doc, MANIFEST_VERSION, &manifest_migrations(), "manifest") else {
        return refused(format!("{MANIFEST_FILE} couldn't be read"), false);
    };
    let declared = given.unwrap_or(0.0);
    let legacy = declared < MANIFEST_VERSION;
    let ManifestCheck { manifest, problems } = check_manifest(&current, declared);
    ManifestRead {
        manifest,
        problems: problems
            .iter()
            .map(|p| {
                let p = if legacy { v1_problem(p) } else { p.clone() };
                format!("{MANIFEST_FILE}: {p}")
            })
            .collect(),
        newer: false,
    }
}

/// `CONTRIBUTIONS` (`pluginManifest.ts:191`): the names of `Contribution`, in display order.
pub const CONTRIBUTIONS: [&str; 17] = [
    "settings",
    "commands",
    "page",
    "process",
    "programs",
    "hooks",
    "tools",
    "passwordManager",
    "main",
    "menus",
    "toolbar",
    "sidebarViews",
    "panels",
    "omnibox",
    "themes",
    "siteSwitches",
    "pageBar",
];

/// `contributionsOf` (`pluginManifest.ts:193`).
pub fn contributions_of(m: &PluginManifest) -> Vec<&'static str> {
    let k = &m.contributes;
    let page_panel = page_panel_of(&k.panels);
    let has = |c: &str| match c {
        "settings" => !k.settings.defaults.is_empty() || !k.settings.rows.is_empty(),
        "commands" => !k.commands.is_empty(),
        "page" => m.page.is_some(),
        "process" => m.sidecar.is_some(),
        "programs" => !k.programs.is_empty(),
        "hooks" => !k.hooks.is_empty(),
        "tools" => !k.tools.is_empty(),
        "passwordManager" => k.password_manager.is_some(),
        "main" => m.main.is_some(),
        "menus" => MENU_SLOTS.iter().any(|slot| !k.menus.slot(slot).is_empty()),
        "toolbar" => !k.toolbar.is_empty(),
        "sidebarViews" => !k.sidebar_views.is_empty(),
        // `p !== pagePanelOf(k.panels)`: identity, not equal contents.
        "panels" => k
            .panels
            .iter()
            .any(|p| !page_panel.is_some_and(|page| std::ptr::eq(p, page))),
        "omnibox" => !k.omnibox.is_empty(),
        "themes" => !k.themes.is_empty(),
        "siteSwitches" => !k.site_switches.is_empty(),
        _ => k.page_bar.is_some(),
    };
    CONTRIBUTIONS.into_iter().filter(|c| has(c)).collect()
}

/// `defaultValues` (`pluginManifest.ts:217`): the default of each stored row, by key.
pub fn default_values(m: &PluginManifest) -> Fields {
    let mut out = Fields::new();
    for r in stored_rows(&m.settings) {
        set_entry(&mut out, &r.key, r.default_json());
    }
    out
}

/// `networkLabel` (`pluginManifest.ts:221`).
pub fn network_label(p: &PluginPermissions) -> String {
    match &p.network {
        NetworkAccess::Off => "none".to_owned(),
        NetworkAccess::Any => "any".to_owned(),
        NetworkAccess::Hosts(hosts) => hosts.join(", "),
    }
}
