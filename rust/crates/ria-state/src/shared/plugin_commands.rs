// Ported from ardia apps/browser/src/lib/pluginCommands.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The commands and tools a plugin declares in `plugin.json`: checking each one and reading what a
//! command does (run a tool, open a page, show a view or panel, or call the plugin's main).
//!
//! Changes from Ardia:
//! - `CommandAction` is an enum; the `is*Action` guards match its variants.
//! - `unknown` is `Option<&Json>`: `None` is `undefined`. `PageSide` is `&'static str`, one of
//!   `SPLIT_SIDES`.

use super::decode::Fields;
use super::plugin_checks::{
    Check, KEY, KEY_RULE, LIMITS, NAME, NAME_RULE, at, integer, list, matching, object, only,
    symbol_of, text,
};
use super::plugin_names::is_page_path;
use super::plugin_rows::fields_value;
use super::plugin_when::parse_when;
use ria_core::identity::telemetry_wire::Json;
use serde::Serialize;

/// `SPLIT_SIDES` (`pluginCommands.ts:6`).
pub const SPLIT_SIDES: [&str; 4] = ["left", "right", "top", "bottom"];
/// `PageSide` (`pluginCommands.ts:7`): one of `SPLIT_SIDES`.
pub type PageSide = &'static str;

/// `CommandAction` (`pluginCommands.ts:9`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum CommandAction {
    Tool {
        tool: String,
        #[serde(serialize_with = "fields_value")]
        input: Fields,
    },
    #[serde(rename_all = "camelCase")]
    Open {
        open: String,
        context: bool,
        side: PageSide,
        side_setting: Option<String>,
        path: String,
    },
    View {
        view: String,
    },
    Panel {
        panel: String,
    },
    Handler {
        handler: bool,
    },
}

/// `CommandSpec` (`pluginCommands.ts:15`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CommandSpec {
    pub name: String,
    pub title: String,
    pub icon: String,
    pub keywords: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    pub action: CommandAction,
}

/// `ToolSpec` (`pluginCommands.ts:16`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ToolSpec {
    pub name: String,
    pub title: String,
    pub description: String,
    pub timeout: f64,
}

const COMMAND_KEYS: [&str; 15] = [
    "name",
    "title",
    "icon",
    "keywords",
    "when",
    "tool",
    "input",
    "open",
    "context",
    "side",
    "sideSetting",
    "path",
    "view",
    "panel",
    "handler",
];
const ACTION_KEYS: [&str; 5] = ["tool", "open", "view", "panel", "handler"];
const OPEN_ONLY: [&str; 4] = ["context", "side", "sideSetting", "path"];

fn field<'a>(doc: &'a Fields, key: &str) -> Option<&'a Json> {
    doc.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// `whenOf` (`pluginCommands.ts:22`): a `when` clause that parses.
pub fn when_of<'a>(c: &mut Check, path: &str, value: Option<&'a Json>) -> Option<&'a str> {
    let Json::String(value) = value? else {
        c.problems.push(format!("{path} must be text"));
        return None;
    };
    match parse_when(value) {
        Ok(_) => Some(value),
        Err(problem) => {
            c.problems.push(format!("{path} {problem}"));
            None
        }
    }
}

/// `openAction` (`pluginCommands.ts:30`).
fn open_action(c: &mut Check, path: &str, doc: &Fields) -> Option<CommandAction> {
    if field(doc, "input").is_some() {
        c.problems.push(format!("{path}.input only goes with tool"));
    }
    let open = match field(doc, "open") {
        Some(Json::String(o)) if o == "tab" || o == "split" => Some(o.clone()),
        _ => {
            c.problems.push(format!("{path}.open must be tab or split"));
            None
        }
    };
    let context = field(doc, "context");
    if context.is_some_and(|x| !matches!(x, Json::Bool(_))) {
        c.problems
            .push(format!("{path}.context must be true or false"));
    }
    let side = match field(doc, "side") {
        None => Some("right"),
        Some(Json::String(s)) if SPLIT_SIDES.contains(&s.as_str()) => {
            SPLIT_SIDES.into_iter().find(|x| x == s)
        }
        Some(_) => {
            c.problems
                .push(format!("{path}.side must be left, right, top or bottom"));
            None
        }
    };
    let side_setting = match field(doc, "sideSetting") {
        None => None,
        given => matching(
            c,
            &format!("{path}.sideSetting"),
            given,
            &KEY,
            "a row's key",
            true,
        )
        .map(str::to_owned),
    };
    if open.as_deref() != Some("split")
        && (field(doc, "side").is_some() || field(doc, "sideSetting").is_some())
    {
        c.problems.push(format!(
            "{path}: side and sideSetting only go with open: split"
        ));
    }
    let page_path = match field(doc, "path") {
        None => Some(String::new()),
        Some(Json::String(p)) if is_page_path(p) => Some(p.clone()),
        Some(_) => {
            c.problems
                .push(format!("{path}.path must be /like/this?q=1"));
            None
        }
    };
    Some(CommandAction::Open {
        open: open?,
        context: context == Some(&Json::Bool(true)),
        side: side?,
        side_setting,
        path: page_path?,
    })
}

/// `actionOf` (`pluginCommands.ts:41`).
fn action_of(c: &mut Check, path: &str, doc: &Fields, has_main: bool) -> Option<CommandAction> {
    let given: Vec<&str> = ACTION_KEYS
        .into_iter()
        .filter(|k| field(doc, k).is_some())
        .collect();
    if given.len() > 1 || (given.is_empty() && !has_main) {
        c.problems.push(if given.len() > 1 {
            format!("{path} needs either tool, open, view, panel or handler, not more than one")
        } else {
            format!("{path} needs either tool (one of the plugin's tools) or open (\"tab\" or \"split\"), or a main to handle it")
        });
        return None;
    }
    let kind = given.first().copied().unwrap_or("handler");
    if kind != "open" {
        for key in OPEN_ONLY {
            if field(doc, key).is_some() {
                c.problems.push(format!("{path}.{key} only goes with open"));
            }
        }
    }
    if kind != "tool" && kind != "open" && field(doc, "input").is_some() {
        c.problems.push(format!("{path}.input only goes with tool"));
    }
    match kind {
        "tool" => {
            let tool = matching(
                c,
                &format!("{path}.tool"),
                field(doc, "tool"),
                &NAME,
                "a tool's name",
                true,
            );
            let input = match field(doc, "input") {
                None => Some(Fields::new()),
                given => object(c, &format!("{path}.input"), given).cloned(),
            };
            Some(CommandAction::Tool {
                tool: tool?.to_owned(),
                input: input?,
            })
        }
        "open" => open_action(c, path, doc),
        "view" => {
            let view = matching(
                c,
                &format!("{path}.view"),
                field(doc, "view"),
                &KEY,
                KEY_RULE,
                true,
            )?;
            Some(CommandAction::View {
                view: view.to_owned(),
            })
        }
        "panel" => {
            let panel = matching(
                c,
                &format!("{path}.panel"),
                field(doc, "panel"),
                &KEY,
                "a panel's id",
                true,
            )?;
            Some(CommandAction::Panel {
                panel: panel.to_owned(),
            })
        }
        _ => {
            if field(doc, "handler").is_some_and(|h| h != &Json::Bool(true)) {
                c.problems.push(format!("{path}.handler must be true"));
            }
            Some(CommandAction::Handler { handler: true })
        }
    }
}

/// `command` (`pluginCommands.ts:68`).
pub fn command(
    c: &mut Check,
    path: &str,
    value: Option<&Json>,
    has_main: bool,
) -> Option<CommandSpec> {
    let doc = object(c, path, value)?;
    only(c, path, doc, &COMMAND_KEYS);
    let name = matching(
        c,
        &format!("{path}.name"),
        field(doc, "name"),
        &NAME,
        NAME_RULE,
        true,
    );
    let title = text(
        c,
        &format!("{path}.title"),
        field(doc, "title"),
        LIMITS.title,
        true,
    );
    let icon = symbol_of(
        c,
        &format!("{path}.icon"),
        field(doc, "icon"),
        "puzzlepiece",
    );
    let keywords_path = format!("{path}.keywords");
    let keywords = list(c, &keywords_path, field(doc, "keywords"), LIMITS.keywords)
        .iter()
        .enumerate()
        .filter_map(|(i, k)| {
            text(c, &at(&keywords_path, i), Some(k), LIMITS.keyword, true).map(str::to_owned)
        })
        .collect();
    let when = when_of(c, &format!("{path}.when"), field(doc, "when"));
    let action = action_of(c, path, doc, has_main);
    Some(CommandSpec {
        name: name?.to_owned(),
        title: title?.to_owned(),
        icon: icon.to_owned(),
        keywords,
        when: when.filter(|w| !w.is_empty()).map(str::to_owned),
        action: action?,
    })
}

/// `tool` (`pluginCommands.ts:82`).
pub fn tool(c: &mut Check, path: &str, value: Option<&Json>) -> Option<ToolSpec> {
    let doc = object(c, path, value)?;
    only(c, path, doc, &["name", "title", "description", "timeout"]);
    let name = matching(
        c,
        &format!("{path}.name"),
        field(doc, "name"),
        &NAME,
        NAME_RULE,
        true,
    );
    let title = text(
        c,
        &format!("{path}.title"),
        field(doc, "title"),
        LIMITS.title,
        true,
    );
    let description = text(
        c,
        &format!("{path}.description"),
        field(doc, "description"),
        LIMITS.description,
        false,
    )
    .unwrap_or("");
    let timeout = integer(
        c,
        &format!("{path}.timeout"),
        field(doc, "timeout"),
        LIMITS.tool_timeout,
        30.0,
    );
    Some(ToolSpec {
        name: name?.to_owned(),
        title: title?.to_owned(),
        description: description.to_owned(),
        timeout,
    })
}

/// `isToolAction` (`pluginCommands.ts:93`).
pub fn is_tool_action(a: &CommandAction) -> bool {
    matches!(a, CommandAction::Tool { .. })
}

/// `isOpenAction` (`pluginCommands.ts:94`).
pub fn is_open_action(a: &CommandAction) -> bool {
    matches!(a, CommandAction::Open { .. })
}

/// `isViewAction` (`pluginCommands.ts:95`).
pub fn is_view_action(a: &CommandAction) -> bool {
    matches!(a, CommandAction::View { .. })
}

/// `isPanelAction` (`pluginCommands.ts:96`).
pub fn is_panel_action(a: &CommandAction) -> bool {
    matches!(a, CommandAction::Panel { .. })
}

/// `isHandlerAction` (`pluginCommands.ts:97`).
pub fn is_handler_action(a: &CommandAction) -> bool {
    matches!(a, CommandAction::Handler { .. })
}
