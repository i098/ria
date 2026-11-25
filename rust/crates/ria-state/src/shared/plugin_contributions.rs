// Ported from ardia apps/browser/src/lib/pluginContributions.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! What a plugin adds to the browser (`contributes` in `plugin.json`): commands, tools, settings,
//! menus, toolbar items, sidebar views, panels, omnibox providers, themes and the page bar.
//!
//! Changes from Ardia:
//! - `MenuContributions` (`Record<MenuSlot, …>`) is a struct with `tab` and `group`; `slot` reads
//!   one by name.
//! - `MenuSlot`, `PanelMode` and `PageSide` are `&'static str`.
//! - `unknown` is `Option<&Json>`: `None` is `undefined`.

use super::decode::Fields;
use super::plugin_checks::{
    Check, KEY, KEY_RULE, KEYWORD, LIMITS, NAME, SYMBOL, at, flag, list, matching, object, only,
    plugin_file, symbol_of, text, unique,
};
use super::plugin_commands::{
    CommandAction, CommandSpec, PageSide, SPLIT_SIDES, ToolSpec, command, tool, when_of,
};
use super::plugin_page_bar::{PageBarSpec, RowRef, SiteSwitchSpec, page_bar_of, site_switches_of};
use super::plugin_password_manager::{PasswordManagerSpec, password_manager_of};
use super::plugin_rows::{SettingsSpec, settings_of};
use super::plugin_runtime::{ProgramSpec, program};
use ria_core::identity::telemetry_wire::Json;
use serde::Serialize;

/// `MENU_SLOTS` (`pluginContributions.ts:9`).
pub const MENU_SLOTS: [&str; 2] = ["tab", "group"];
/// `MenuSlot` (`pluginContributions.ts:10`).
pub type MenuSlot = &'static str;
/// `PANEL_MODES` (`pluginContributions.ts:11`).
pub const PANEL_MODES: [&str; 3] = ["sidecar", "tab", "split"];
/// `PanelMode` (`pluginContributions.ts:12`).
pub type PanelMode = &'static str;

/// `MenuContribution` (`pluginContributions.ts:14`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MenuContribution {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
}

/// `MenuContributions` (`pluginContributions.ts:15`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct MenuContributions {
    pub tab: Vec<MenuContribution>,
    pub group: Vec<MenuContribution>,
}

impl MenuContributions {
    /// `menus[slot]` for a slot of `MENU_SLOTS`.
    pub fn slot(&self, slot: &str) -> &[MenuContribution] {
        if slot == "tab" {
            &self.tab
        } else {
            &self.group
        }
    }

    fn slot_mut(&mut self, slot: &str) -> &mut Vec<MenuContribution> {
        if slot == "tab" {
            &mut self.tab
        } else {
            &mut self.group
        }
    }
}

/// `ToolbarItem` (`pluginContributions.ts:16`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToolbarItem {
    pub id: String,
    pub title: String,
    pub icon: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
}

/// `SidebarView` (`pluginContributions.ts:17`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SidebarView {
    pub id: String,
    pub title: String,
    pub icon: String,
}

/// `PanelSpec` (`pluginContributions.ts:18`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelSpec {
    pub id: String,
    pub title: String,
    pub icon: String,
    pub mode: PanelMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    pub retain_context: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<PageSide>,
}

/// `OmniboxSpec` (`pluginContributions.ts:19`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OmniboxSpec {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
    pub global: bool,
}

/// `ThemeSpec` (`pluginContributions.ts:20`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ThemeSpec {
    pub id: String,
    pub title: String,
    pub file: String,
}

/// `PluginContributions` (`pluginContributions.ts:22`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginContributions {
    pub commands: Vec<CommandSpec>,
    pub tools: Vec<ToolSpec>,
    pub hooks: Vec<String>,
    pub settings: SettingsSpec,
    pub programs: Vec<ProgramSpec>,
    pub password_manager: Option<PasswordManagerSpec>,
    pub menus: MenuContributions,
    pub toolbar: Vec<ToolbarItem>,
    pub sidebar_views: Vec<SidebarView>,
    pub panels: Vec<PanelSpec>,
    pub omnibox: Vec<OmniboxSpec>,
    pub themes: Vec<ThemeSpec>,
    pub site_switches: Vec<SiteSwitchSpec>,
    pub page_bar: Option<PageBarSpec>,
}

/// `CONTRIBUTES_KEYS` (`pluginContributions.ts:39`).
pub const CONTRIBUTES_KEYS: [&str; 14] = [
    "commands",
    "tools",
    "hooks",
    "settings",
    "programs",
    "passwordManager",
    "menus",
    "toolbar",
    "sidebarViews",
    "panels",
    "omnibox",
    "themes",
    "siteSwitches",
    "pageBar",
];
/// `PAGE_PANEL_ID` (`pluginContributions.ts:40`).
pub const PAGE_PANEL_ID: &str = "default";

/// `emptyMenus` (`pluginContributions.ts:42`).
pub fn empty_menus() -> MenuContributions {
    MenuContributions::default()
}

const P: &str = "contributes";
const COMMAND_RULE: &str = "one of the plugin's commands";

fn field<'a>(doc: &'a Fields, key: &str) -> Option<&'a Json> {
    doc.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// `menuItem` (`pluginContributions.ts:46`).
fn menu_item(c: &mut Check, path: &str, value: Option<&Json>) -> Option<MenuContribution> {
    let doc = object(c, path, value)?;
    only(c, path, doc, &["command", "title", "icon", "when"]);
    let name = matching(
        c,
        &format!("{path}.command"),
        field(doc, "command"),
        &NAME,
        COMMAND_RULE,
        true,
    );
    let title = text(
        c,
        &format!("{path}.title"),
        field(doc, "title"),
        LIMITS.title,
        false,
    );
    let icon = field(doc, "icon")
        .map(|icon| symbol_of(c, &format!("{path}.icon"), Some(icon), "puzzlepiece"));
    let when = when_of(c, &format!("{path}.when"), field(doc, "when"));
    let some = |s: Option<&str>| s.filter(|s| !s.is_empty()).map(str::to_owned);
    Some(MenuContribution {
        command: name?.to_owned(),
        title: some(title),
        icon: some(icon),
        when: some(when),
    })
}

/// `menusOf` (`pluginContributions.ts:57`).
fn menus_of(c: &mut Check, value: Option<&Json>) -> MenuContributions {
    let mut out = empty_menus();
    if value.is_none() {
        return out;
    }
    let path = format!("{P}.menus");
    let Some(doc) = object(c, &path, value) else {
        return out;
    };
    only(c, &path, doc, &MENU_SLOTS);
    for slot in MENU_SLOTS {
        let slot_path = format!("{path}.{slot}");
        let items = list(c, &slot_path, field(doc, slot), LIMITS.menu_items);
        *out.slot_mut(slot) = items
            .iter()
            .enumerate()
            .filter_map(|(i, m)| menu_item(c, &at(&slot_path, i), Some(m)))
            .collect();
    }
    out
}

/// `idOf` (`pluginContributions.ts:67`).
fn id_of<'a>(c: &mut Check, path: &str, doc: &'a Fields) -> Option<&'a str> {
    matching(
        c,
        &format!("{path}.id"),
        field(doc, "id"),
        &KEY,
        KEY_RULE,
        true,
    )
}

/// `toolbarItem` (`pluginContributions.ts:71`).
fn toolbar_item(c: &mut Check, path: &str, value: Option<&Json>) -> Option<ToolbarItem> {
    let doc = object(c, path, value)?;
    only(
        c,
        path,
        doc,
        &["id", "title", "icon", "command", "view", "tooltip"],
    );
    let id = id_of(c, path, doc);
    let title = text(
        c,
        &format!("{path}.title"),
        field(doc, "title"),
        LIMITS.title,
        true,
    );
    let icon = matching(
        c,
        &format!("{path}.icon"),
        field(doc, "icon"),
        &SYMBOL,
        "an SF Symbol name",
        true,
    );
    let tooltip = text(
        c,
        &format!("{path}.tooltip"),
        field(doc, "tooltip"),
        LIMITS.tooltip,
        false,
    );
    let (given_command, given_view) = (field(doc, "command"), field(doc, "view"));
    if given_command.is_none() == given_view.is_none() {
        c.problems
            .push(format!("{path} needs either command or view"));
    }
    let cmd = given_command.and_then(|v| {
        matching(
            c,
            &format!("{path}.command"),
            Some(v),
            &NAME,
            COMMAND_RULE,
            true,
        )
    });
    let view = given_view
        .and_then(|v| matching(c, &format!("{path}.view"), Some(v), &KEY, KEY_RULE, true));
    let some = |s: Option<&str>| s.filter(|s| !s.is_empty()).map(str::to_owned);
    if cmd.is_none() && view.is_none() {
        return None;
    }
    Some(ToolbarItem {
        id: id?.to_owned(),
        title: some(title)?,
        icon: icon?.to_owned(),
        command: some(cmd),
        view: some(view),
        tooltip: some(tooltip),
    })
}

/// `sidebarView` (`pluginContributions.ts:86`).
fn sidebar_view(c: &mut Check, path: &str, value: Option<&Json>) -> Option<SidebarView> {
    let doc = object(c, path, value)?;
    only(c, path, doc, &["id", "title", "icon"]);
    let id = id_of(c, path, doc);
    let title = text(
        c,
        &format!("{path}.title"),
        field(doc, "title"),
        LIMITS.page_title,
        true,
    );
    let icon = symbol_of(
        c,
        &format!("{path}.icon"),
        field(doc, "icon"),
        "puzzlepiece",
    );
    Some(SidebarView {
        id: id?.to_owned(),
        title: title.filter(|t| !t.is_empty())?.to_owned(),
        icon: icon.to_owned(),
    })
}

/// `panel` (`pluginContributions.ts:96`).
fn panel(c: &mut Check, path: &str, value: Option<&Json>) -> Option<PanelSpec> {
    let doc = object(c, path, value)?;
    only(
        c,
        path,
        doc,
        &[
            "id",
            "title",
            "icon",
            "mode",
            "file",
            "view",
            "retainContext",
            "side",
            "served",
        ],
    );
    if field(doc, "served").is_some() {
        c.problems.push(format!(
            "{path}.served is gone: a page is static files from the plugin's folder (page.file), and it talks to its helper with window.ardia.send"
        ));
        return None;
    }
    let id = id_of(c, path, doc);
    let title = text(
        c,
        &format!("{path}.title"),
        field(doc, "title"),
        LIMITS.page_title,
        true,
    );
    let icon = symbol_of(
        c,
        &format!("{path}.icon"),
        field(doc, "icon"),
        "puzzlepiece",
    );
    let mode = match field(doc, "mode") {
        Some(Json::String(m)) => PANEL_MODES.into_iter().find(|x| x == m),
        _ => None,
    };
    if mode.is_none() {
        c.problems
            .push(format!("{path}.mode must be {}", PANEL_MODES.join(", ")));
    }
    let retain_context = flag(
        c,
        &format!("{path}.retainContext"),
        field(doc, "retainContext"),
        false,
    );
    let (given_file, given_view) = (field(doc, "file"), field(doc, "view"));
    if given_file.is_some() && given_view.is_some() {
        c.problems
            .push(format!("{path} takes a file or a view, not both"));
    }
    if given_file.is_none() && given_view.is_none() {
        c.problems.push(format!("{path}.file must be a path inside the plugin's folder, like page/index.html, or the panel needs a view"));
        return None;
    }
    let file = given_file.and_then(|f| {
        plugin_file(
            c,
            &format!("{path}.file"),
            Some(f),
            "a path inside the plugin's folder, like page/index.html",
            None,
        )
    });
    let view = given_view
        .and_then(|v| matching(c, &format!("{path}.view"), Some(v), &KEY, KEY_RULE, true));
    let given_side = field(doc, "side");
    if given_side.is_some() && mode != Some("split") {
        c.problems
            .push(format!("{path}.side only goes with mode split"));
    }
    let side = given_side.and_then(|s| {
        let side = match s {
            Json::String(s) => SPLIT_SIDES.into_iter().find(|x| x == s),
            _ => None,
        };
        if side.is_none() {
            c.problems
                .push(format!("{path}.side must be left, right, top or bottom"));
        }
        side
    });
    let some = |s: Option<&str>| s.filter(|s| !s.is_empty()).map(str::to_owned);
    let (file, view) = (some(file), some(view));
    if file.is_none() && view.is_none() {
        return None;
    }
    Some(PanelSpec {
        id: id?.to_owned(),
        title: some(title)?,
        icon: icon.to_owned(),
        mode: mode?,
        file,
        view,
        retain_context,
        side,
    })
}

/// `omnibox` (`pluginContributions.ts:122`).
fn omnibox(c: &mut Check, path: &str, value: Option<&Json>) -> Option<OmniboxSpec> {
    let doc = object(c, path, value)?;
    only(c, path, doc, &["id", "title", "keyword", "global"]);
    let id = id_of(c, path, doc);
    let title = text(
        c,
        &format!("{path}.title"),
        field(doc, "title"),
        LIMITS.title,
        true,
    );
    let keyword = matching(
        c,
        &format!("{path}.keyword"),
        field(doc, "keyword"),
        &KEYWORD,
        "1 to 32 letters, digits, - and _",
        false,
    );
    let global = flag(c, &format!("{path}.global"), field(doc, "global"), false);
    let keyword = keyword.filter(|k| !k.is_empty()).map(str::to_owned);
    if keyword.is_none() && !global {
        c.problems.push(format!(
            "{path} needs a keyword or global: true, or it never shows"
        ));
    }
    if keyword.is_some() && global {
        c.problems
            .push(format!("{path} takes a keyword or global: true, not both"));
    }
    Some(OmniboxSpec {
        id: id?.to_owned(),
        title: title.filter(|t| !t.is_empty())?.to_owned(),
        keyword,
        global,
    })
}

/// `theme` (`pluginContributions.ts:135`).
fn theme(c: &mut Check, path: &str, value: Option<&Json>) -> Option<ThemeSpec> {
    let doc = object(c, path, value)?;
    only(c, path, doc, &["id", "title", "file"]);
    let id = id_of(c, path, doc);
    let title = text(
        c,
        &format!("{path}.title"),
        field(doc, "title"),
        LIMITS.title,
        true,
    );
    let file = plugin_file(
        c,
        &format!("{path}.file"),
        field(doc, "file"),
        "a .json file inside the plugin's folder, like themes/dusk.json",
        Some(".json"),
    );
    Some(ThemeSpec {
        id: id?.to_owned(),
        title: title.filter(|t| !t.is_empty())?.to_owned(),
        file: file?.to_owned(),
    })
}

/// `parsed` (`pluginContributions.ts:145`).
fn parsed<T>(
    c: &mut Check,
    key: &str,
    value: Option<&Json>,
    max: usize,
    read: impl Fn(&mut Check, &str, Option<&Json>) -> Option<T>,
    ids: impl Fn(&T) -> &str,
    what: &str,
) -> Vec<T> {
    let path = format!("{P}.{key}");
    let out: Vec<T> = list(c, &path, value, max)
        .iter()
        .enumerate()
        .filter_map(|(i, v)| read(c, &at(&path, i), Some(v)))
        .collect();
    unique(c, &path, out.iter().map(|t| Some(ids(t))), what);
    out
}

/// `contributesOf` (`pluginContributions.ts:151`).
pub fn contributes_of(c: &mut Check, value: Option<&Json>, has_main: bool) -> PluginContributions {
    let no_fields = Fields::new();
    let doc = match value {
        None => &no_fields,
        Some(_) => object(c, P, value).unwrap_or(&no_fields),
    };
    only(c, P, doc, &CONTRIBUTES_KEYS);
    let mut pm = Vec::new();
    let password_manager = password_manager_of(&mut pm, field(doc, "passwordManager"));
    c.problems
        .extend(pm.into_iter().map(|p| format!("{P}.{p}")));
    let settings = settings_of(c, &format!("{P}.settings"), field(doc, "settings"));
    let site_switches = site_switches_of(c, field(doc, "siteSwitches"));
    let commands = parsed(
        c,
        "commands",
        field(doc, "commands"),
        LIMITS.commands,
        |cc, p, v| command(cc, p, v, has_main),
        |x| &x.name,
        "commands",
    );
    let tools = parsed(
        c,
        "tools",
        field(doc, "tools"),
        LIMITS.tools,
        tool,
        |t| &t.name,
        "tools",
    );
    let hooks_path = format!("{P}.hooks");
    let mut hooks = Vec::new();
    for (i, h) in list(c, &hooks_path, field(doc, "hooks"), LIMITS.hooks)
        .iter()
        .enumerate()
    {
        match h {
            Json::String(h) if !h.is_empty() => hooks.push(h.clone()),
            _ => c
                .problems
                .push(format!("{} must be an event name", at(&hooks_path, i))),
        }
    }
    let programs = parsed(
        c,
        "programs",
        field(doc, "programs"),
        LIMITS.programs,
        program,
        |p| &p.name,
        "programs",
    );
    let menus = menus_of(c, field(doc, "menus"));
    let toolbar = parsed(
        c,
        "toolbar",
        field(doc, "toolbar"),
        LIMITS.toolbar,
        toolbar_item,
        |t| &t.id,
        "toolbar items",
    );
    let sidebar_views = parsed(
        c,
        "sidebarViews",
        field(doc, "sidebarViews"),
        LIMITS.sidebar_views,
        sidebar_view,
        |v| &v.id,
        "sidebar views",
    );
    let panels = parsed(
        c,
        "panels",
        field(doc, "panels"),
        LIMITS.panels,
        panel,
        |p| &p.id,
        "panels",
    );
    let omnibox = parsed(
        c,
        "omnibox",
        field(doc, "omnibox"),
        LIMITS.omnibox,
        omnibox,
        |o| &o.id,
        "omnibox providers",
    );
    let themes = parsed(
        c,
        "themes",
        field(doc, "themes"),
        LIMITS.themes,
        theme,
        |t| &t.id,
        "themes",
    );
    let rows: Vec<RowRef> = settings
        .rows
        .iter()
        .map(|r| RowRef {
            key: &r.key,
            r#type: r.row_type(),
        })
        .collect();
    let page_bar = page_bar_of(c, field(doc, "pageBar"), &site_switches, &rows);
    PluginContributions {
        commands,
        tools,
        hooks,
        settings,
        programs,
        password_manager,
        menus,
        toolbar,
        sidebar_views,
        panels,
        omnibox,
        themes,
        site_switches,
        page_bar,
    }
}

/// `pagePanelOf` (`pluginContributions.ts:177`): the panel that opens as a page.
pub fn page_panel_of(panels: &[PanelSpec]) -> Option<&PanelSpec> {
    let mut pages = panels
        .iter()
        .filter(|p| p.file.is_some() && (p.mode == "tab" || p.mode == "split"));
    let first = pages.clone().next();
    pages.find(|p| p.id == PAGE_PANEL_ID).or(first)
}

/// `viewIdsOf` (`pluginContributions.ts:182`): each view id once, in first-seen order.
pub fn view_ids_of(k: &PluginContributions) -> Vec<String> {
    let ids = k
        .sidebar_views
        .iter()
        .map(|v| &v.id)
        .chain(k.panels.iter().filter_map(|p| p.view.as_ref()))
        .chain(k.toolbar.iter().filter_map(|t| t.view.as_ref()))
        .chain(k.commands.iter().filter_map(|x| match &x.action {
            CommandAction::View { view } => Some(view),
            _ => None,
        }));
    let mut out: Vec<String> = Vec::new();
    for id in ids {
        if !out.contains(id) {
            out.push(id.clone());
        }
    }
    out
}
