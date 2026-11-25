// Ported from ardia apps/browser/src/lib/pluginSchema.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The JSON Schema of `plugin.json` (`docs/plugin.schema.json`): version 2 manifests, else the
//! version 1 body of `plugin_schema_v1`.
//!
//! Changes from Ardia:
//! - The schema is a `Json` value (see `plugin_schema_parts`).
//! - `PluginRules` holds borrowed string lists; `decideEvents?` is `Option`.

use ria_core::identity::telemetry_wire::Json;

use super::decode::Fields;
use super::json_schema::{JSON_SCHEMA_DIALECT, schema_json};
use super::plugin_checks::{DEFAULT_MAIN_MB, KEYWORD_PATTERN, LIMITS, VERSION_PATTERN};
use super::plugin_commands::SPLIT_SIDES;
use super::plugin_contributions::{MENU_SLOTS, PANEL_MODES};
use super::plugin_manifest::MANIFEST_VERSION;
use super::plugin_names::RESERVED_PAGE_NAMES;
use super::plugin_page_bar::{BUTTON_DIVIDER, HOST_ACTION_NAMES, PAGE_BAR_LIMITS, PAGE_BAR_LOOKS};
use super::plugin_permissions::TAB_ACCESS;
use super::plugin_schema_parts::{
    built_in_flags, closed, command_base, fields, key, name, num, obj, open_command_properties,
    password_manager_schema, plugin_path, process_schema, programs_schema_v2, row_schema, strings,
    symbol, text, text_of, tools_schema, with,
};
use super::plugin_schema_v1::legacy_plugin_body;
use super::plugin_when::MAX_WHEN_LENGTH;

/// `PLUGIN_SCHEMA_FILE` (`pluginSchema.ts:13`).
pub const PLUGIN_SCHEMA_FILE: &str = "plugin.schema.json";

/// `PluginRules` (`pluginSchema.ts:15`): the app's own checks, which the schema lists for tools.
#[derive(Debug, Clone, Copy, Default)]
pub struct PluginRules<'a> {
    pub untabbed_events: &'a [&'a str],
    pub approved_settings: &'a [&'a str],
    pub decide_events: Option<&'a [&'a str]>,
}

/// `when` (`pluginSchema.ts:17`).
fn when() -> Json {
    obj([
        ("type", text("string")),
        (
            "pattern",
            Json::String(format!("^[\\s\\S]{{1,{MAX_WHEN_LENGTH}}}$")),
        ),
        (
            "description",
            text("Shows only when this holds, like urlHost == \"github.com\" && !isPinned"),
        ),
    ])
}

/// `list` (`pluginSchema.ts:18`).
fn list(max: usize, items: Json, description: Option<&str>) -> Json {
    let schema = obj([
        ("type", text("array")),
        ("maxItems", num(max)),
        ("items", items),
    ]);
    match description {
        Some(d) if !d.is_empty() => with(schema, [("description", text(d))]),
        _ => schema,
    }
}

/// `{ type: "boolean", description }`.
fn boolean(description: &str) -> Json {
    obj([
        ("type", text("boolean")),
        ("description", text(description)),
    ])
}

/// `permissions` (`pluginSchema.ts:20`).
fn permissions() -> Json {
    let mut tabs: Vec<Json> = TAB_ACCESS.iter().map(|t| text(t)).collect();
    tabs.push(Json::Bool(false));
    closed(
        obj([
            (
                "tabs",
                obj([
                    ("enum", Json::Array(tabs)),
                    (
                        "description",
                        text("read: see your tabs. write: also open them"),
                    ),
                ]),
            ),
            (
                "fullUrls",
                boolean("See whole addresses, with what follows ? and #"),
            ),
            (
                "activePage",
                boolean(
                    "Read the text of the tab you're on, for 30 seconds after you use the plugin",
                ),
            ),
            (
                "workspace",
                boolean(
                    "See and rearrange every window, tab and group (needs tabs: write and fullUrls)",
                ),
            ),
            ("commands", boolean("Run Ardia commands")),
            ("storage", boolean("Keep its own data")),
            ("secrets", boolean("Keep secrets in the Keychain")),
            (
                "network",
                obj([
                    (
                        "anyOf",
                        Json::Array(vec![
                            obj([("const", Json::Bool(false))]),
                            obj([("const", text("*"))]),
                            obj([
                                ("type", text("array")),
                                ("minItems", Json::Number(1.0)),
                                ("maxItems", num(LIMITS.allowed_hosts)),
                                (
                                    "items",
                                    obj([
                                        ("type", text("string")),
                                        ("pattern", text("^(\\*\\.)?[a-z0-9.-]{1,253}$")),
                                    ]),
                                ),
                            ]),
                        ]),
                    ),
                    (
                        "description",
                        text("false, \"*\" or hosts like api.example.com and *.example.com"),
                    ),
                ]),
            ),
            (
                "control",
                boolean("Let programs you run drive your tabs and pages"),
            ),
        ]),
        &[] as &[&str],
    )
}

/// `menuItem` (`pluginSchema.ts:32`).
fn menu_item() -> Json {
    closed(
        obj([
            ("command", name()),
            ("title", text_of(LIMITS.title as f64, None)),
            ("icon", symbol()),
            ("when", when()),
        ]),
        &["command"],
    )
}

/// `{ ...key, description }`.
fn described_key(description: &str) -> Json {
    with(key(), [("description", text(description))])
}

/// `contributes` (`pluginSchema.ts:34`): the `contributes` block, whose `hooks` are `hook_names`.
fn contributes(hook_names: &[&str]) -> Json {
    let mut command = command_base();
    command.extend(open_command_properties());
    command.extend(fields([
        ("when", when()),
        (
            "view",
            described_key("Opens the command's view, which main renders"),
        ),
        ("panel", described_key("Opens one of the plugin's panels")),
        (
            "handler",
            obj([
                ("const", Json::Bool(true)),
                (
                    "description",
                    text("main handles it; the default when there's a main and nothing else"),
                ),
            ]),
        ),
    ]));
    let menus: Fields = MENU_SLOTS
        .iter()
        .map(|slot| (slot.to_string(), list(LIMITS.menu_items, menu_item(), None)))
        .collect();
    let mut buttons: Vec<Json> = HOST_ACTION_NAMES.iter().map(|n| text(n)).collect();
    buttons.push(text(BUTTON_DIVIDER));
    let title = || text_of(LIMITS.title as f64, None);
    let page_title = || text_of(LIMITS.page_title as f64, None);
    closed(
        obj([
            (
                "commands",
                list(
                    LIMITS.commands,
                    closed(Json::Object(command), &["name", "title"]),
                    None,
                ),
            ),
            ("tools", tools_schema()),
            (
                "hooks",
                list(LIMITS.hooks, obj([("enum", strings(hook_names))]), None),
            ),
            (
                "settings",
                closed(
                    obj([
                        (
                            "defaults",
                            obj([
                                ("type", text("object")),
                                ("description", text("Ardia settings, as in config.json")),
                            ]),
                        ),
                        (
                            "rows",
                            list(
                                LIMITS.rows,
                                row_schema(&["toggle", "text", "number", "choice", "secret"]),
                                None,
                            ),
                        ),
                    ]),
                    &[] as &[&str],
                ),
            ),
            ("programs", programs_schema_v2()),
            ("passwordManager", password_manager_schema()),
            ("menus", closed(Json::Object(menus), &[] as &[&str])),
            (
                "toolbar",
                list(
                    LIMITS.toolbar,
                    closed(
                        obj([
                            ("id", key()),
                            ("title", title()),
                            ("icon", symbol()),
                            ("command", name()),
                            ("view", key()),
                            ("tooltip", text_of(LIMITS.tooltip as f64, None)),
                        ]),
                        &["id", "title", "icon"],
                    ),
                    None,
                ),
            ),
            (
                "sidebarViews",
                list(
                    LIMITS.sidebar_views,
                    closed(
                        obj([("id", key()), ("title", page_title()), ("icon", symbol())]),
                        &["id", "title"],
                    ),
                    None,
                ),
            ),
            (
                "panels",
                list(
                    LIMITS.panels,
                    closed(
                        obj([
                            ("id", key()),
                            ("title", page_title()),
                            ("icon", symbol()),
                            ("mode", obj([("enum", strings(&PANEL_MODES))])),
                            ("file", plugin_path(Some("A page in the plugin's folder"))),
                            ("view", key()),
                            ("retainContext", obj([("type", text("boolean"))])),
                            ("side", obj([("enum", strings(&SPLIT_SIDES))])),
                        ]),
                        &["id", "title", "mode"],
                    ),
                    None,
                ),
            ),
            (
                "omnibox",
                list(
                    LIMITS.omnibox,
                    closed(
                        obj([
                            ("id", key()),
                            ("title", title()),
                            (
                                "keyword",
                                obj([("type", text("string")), ("pattern", text(KEYWORD_PATTERN))]),
                            ),
                            ("global", boolean("Shows in every search; takes no keyword")),
                        ]),
                        &["id", "title"],
                    ),
                    None,
                ),
            ),
            (
                "themes",
                list(
                    LIMITS.themes,
                    closed(
                        obj([
                            ("id", key()),
                            ("title", title()),
                            (
                                "file",
                                with(
                                    plugin_path(Some("A theme file in the plugin's folder")),
                                    [("pattern", text("^[^/\\\\][^\\\\]*\\.json$"))],
                                ),
                            ),
                        ]),
                        &["id", "title", "file"],
                    ),
                    None,
                ),
            ),
            (
                "siteSwitches",
                list(
                    PAGE_BAR_LIMITS.site_switches,
                    closed(
                        obj([
                            ("key", key()),
                            ("title", text_of(PAGE_BAR_LIMITS.switch_title as f64, None)),
                            ("icon", symbol()),
                        ]),
                        &["key", "title"],
                    ),
                    Some(
                        "Switches Site Controls shows for each http(s) site, kept per origin on this device",
                    ),
                ),
            ),
            (
                "pageBar",
                with(
                    closed(
                        obj([
                            (
                                "look",
                                obj([
                                    ("enum", strings(&PAGE_BAR_LOOKS)),
                                    (
                                        "description",
                                        text(
                                            "stripes: Arc's striped developer bar above the page, or in the toolbar's address field",
                                        ),
                                    ),
                                ]),
                            ),
                            (
                                "when",
                                closed(
                                    obj([
                                        (
                                            "siteSwitch",
                                            described_key(
                                                "One of this plugin's siteSwitches: the bar shows on sites where it is on",
                                            ),
                                        ),
                                        (
                                            "localAddresses",
                                            described_key(
                                                "A toggle row: while it is on, a local address typed in the address bar turns siteSwitch on for that site",
                                            ),
                                        ),
                                    ]),
                                    &["siteSwitch"],
                                ),
                            ),
                            (
                                "buttons",
                                obj([
                                    ("type", text("array")),
                                    ("minItems", Json::Number(1.0)),
                                    ("maxItems", num(PAGE_BAR_LIMITS.buttons)),
                                    ("items", obj([("enum", Json::Array(buttons))])),
                                    (
                                        "description",
                                        Json::String(format!(
                                            "Host actions in order; {BUTTON_DIVIDER} is a divider"
                                        )),
                                    ),
                                ]),
                            ),
                        ]),
                        &["look", "when", "buttons"],
                    ),
                    [(
                        "description",
                        text(
                            "A bar Ardia draws above the pages the plugin's site switch is on for, with buttons that run Ardia's own actions on that tab",
                        ),
                    )],
                ),
            ),
        ]),
        &[] as &[&str],
    )
}

/// `pluginBody` (`pluginSchema.ts:85`): the body of a version 2 manifest.
fn plugin_body(event_names: &[&str], decide_events: &[&str]) -> Json {
    let hook_names: Vec<&str> = event_names.iter().chain(decide_events).copied().collect();
    let mut properties = fields([
        ("$schema", obj([("type", text("string"))])),
        (
            "manifest",
            obj([
                ("const", Json::Number(MANIFEST_VERSION)),
                ("description", text("The manifest's version")),
            ]),
        ),
        (
            "name",
            with(
                name(),
                [(
                    "description",
                    text("The plugin's name; its folder has the same name"),
                )],
            ),
        ),
        ("title", text_of(LIMITS.title as f64, None)),
        ("description", text_of(LIMITS.description as f64, None)),
        (
            "version",
            obj([("type", text("string")), ("pattern", text(VERSION_PATTERN))]),
        ),
        (
            "requires",
            obj([
                ("type", text("string")),
                ("pattern", text(VERSION_PATTERN)),
                (
                    "description",
                    text("The first Ardia version the plugin works with"),
                ),
            ]),
        ),
        ("icon", symbol()),
        (
            "keywords",
            list(
                LIMITS.plugin_keywords,
                text_of(LIMITS.keyword as f64, None),
                Some("Words that find the plugin in Settings"),
            ),
        ),
        (
            "telemetry",
            closed(
                obj([
                    ("title", text_of(LIMITS.title as f64, None)),
                    (
                        "items",
                        obj([
                            ("type", text("array")),
                            ("minItems", Json::Number(1.0)),
                            ("maxItems", num(LIMITS.telemetry_items)),
                            ("items", text_of(LIMITS.telemetry_item as f64, None)),
                        ]),
                    ),
                ]),
                &["title", "items"],
            ),
        ),
        (
            "main",
            with(
                plugin_path(Some(
                    "The plugin's JavaScript, like dist/index.js; a plugin has main or sidecar, not both",
                )),
                [("pattern", text("^[^/\\\\][^\\\\]*\\.(m?js|cjs)$"))],
            ),
        ),
        (
            "memoryMB",
            obj([
                ("type", text("integer")),
                ("minimum", Json::Number(LIMITS.memory_mb[0])),
                ("maximum", Json::Number(LIMITS.memory_mb[1])),
                ("default", Json::Number(DEFAULT_MAIN_MB)),
                ("description", text("main's memory limit")),
            ]),
        ),
        (
            "sidecar",
            with(
                process_schema(),
                [(
                    "description",
                    text(
                        "A program Ardia runs beside the plugin; a plugin has main or sidecar, not both",
                    ),
                )],
            ),
        ),
        (
            "activationEvents",
            list(
                LIMITS.activation_events,
                obj([
                    ("type", text("string")),
                    (
                        "pattern",
                        text(
                            "^(\\*|onStartupFinished|on(Command|View|Panel|Toolbar|Omnibox|Menu|Event|Url):\\S{1,253})$",
                        ),
                    ),
                ]),
                Some("When main starts; worked out from contributes when left out"),
            ),
        ),
        ("permissions", permissions()),
        ("contributes", contributes(&hook_names)),
    ]);
    properties.extend(built_in_flags());
    closed(Json::Object(properties), &["manifest", "name"])
}

/// `pluginSchema` (`pluginSchema.ts:113`): the schema of `plugin.json` for these hook event names and
/// app rules.
pub fn plugin_schema(event_names: &[&str], rules: &PluginRules) -> Json {
    let decide_events = rules.decide_events.unwrap_or_default();
    let app_rules = obj([
        ("untabbedEvents", strings(rules.untabbed_events)),
        ("approvedSettings", strings(rules.approved_settings)),
        ("decideEvents", strings(decide_events)),
        ("reservedPageNames", strings(RESERVED_PAGE_NAMES)),
    ]);
    obj([
        ("$schema", text(JSON_SCHEMA_DIALECT)),
        ("title", text("Ardia plugin")),
        ("type", text("object")),
        (
            "if",
            obj([
                (
                    "properties",
                    obj([("manifest", obj([("const", Json::Number(MANIFEST_VERSION))]))]),
                ),
                ("required", strings(&["manifest"])),
            ]),
        ),
        ("then", plugin_body(event_names, decide_events)),
        ("else", legacy_plugin_body(event_names)),
        ("x-ardia-rules", app_rules),
    ])
}

/// `pluginSchemaJson` (`pluginSchema.ts:127`): `plugin_schema` as the text of `plugin.schema.json`.
pub fn plugin_schema_json(event_names: &[&str], rules: &PluginRules) -> String {
    schema_json(&plugin_schema(event_names, rules))
}
