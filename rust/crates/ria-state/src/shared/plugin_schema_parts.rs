// Ported from ardia apps/browser/src/lib/pluginSchemaParts.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The JSON Schema parts that the manifest schemas of version 1 (`plugin_schema_v1`) and version 2
//! (`plugin_schema`) share.
//!
//! Changes from Ardia:
//! - Each schema constant (`name`, `key`, `symbol`, `openCommandProperties`, `commandBase`,
//!   `processSchema`, `programsSchema`, `passwordManagerSchema`, `programsSchemaV2`, `toolsSchema`,
//!   `builtInFlags`) is a function that builds a new `Json`, because a `Json` cannot be a `const`.
//!   A `Record<string, JsonSchema>` is `Fields`.
//! - `closed` takes `properties` as any `Json`, as the TS puts it in unchanged.
//! - `description?` is `Option<&str>`; `Some("")` is left out, as the TS `description ? ... : {}` does.
//! - `obj`, `with`, `strings` and `num` are helpers for object literals, spreads and arrays.

use ria_core::identity::telemetry_wire::{Json, js_number_to_string, set_entry};

use super::decode::Fields;
use super::plugin_checks::{DEFAULT_SIDECAR_MB, KEY_PATTERN, LIMITS, NAME_PATTERN, SYMBOL_PATTERN};
use super::plugin_commands::SPLIT_SIDES;
use super::plugin_password_manager::{PASSWORD_FIELD_COUNT, PASSWORD_FIELDS, PASSWORD_FILLS};

/// An object literal.
pub fn obj<const N: usize>(entries: [(&str, Json); N]) -> Json {
    Json::Object(fields(entries))
}

/// The entries of an object literal.
pub fn fields<const N: usize>(entries: [(&str, Json); N]) -> Fields {
    entries
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect()
}

/// `{ ...base, ...entries }`: a key already in `base` keeps its place.
pub fn with<const N: usize>(base: Json, entries: [(&str, Json); N]) -> Json {
    let mut out = match base {
        Json::Object(entries) => entries,
        _ => Fields::new(),
    };
    for (k, v) in entries {
        set_entry(&mut out, k, v);
    }
    Json::Object(out)
}

/// A string.
pub fn text(value: &str) -> Json {
    Json::String(value.to_owned())
}

/// A number from a limit.
pub fn num(value: usize) -> Json {
    Json::Number(value as f64)
}

/// An array of strings.
pub fn strings<S: AsRef<str>>(items: &[S]) -> Json {
    Json::Array(items.iter().map(|s| text(s.as_ref())).collect())
}

/// `...(description ? { description } : {})`.
fn described(schema: Json, description: Option<&str>) -> Json {
    match description {
        Some(d) if !d.is_empty() => with(schema, [("description", text(d))]),
        _ => schema,
    }
}

const UNSAFE_CLASS: &str =
    "\\u0000-\\u001f\\u007f-\\u009f\\u061c\\u200e\\u200f\\u202a-\\u202e\\u2066-\\u2069";

/// `textOf` (`pluginSchemaParts.ts:7`): text of 1 to `max` characters, not blank, without control or
/// direction characters.
pub fn text_of(max: f64, description: Option<&str>) -> Json {
    let pattern = format!(
        "^(?=[\\s\\S]*\\S)[^{UNSAFE_CLASS}]{{1,{}}}$",
        js_number_to_string(max)
    );
    described(
        obj([("type", text("string")), ("pattern", Json::String(pattern))]),
        description,
    )
}

/// `closed` (`pluginSchemaParts.ts:8`): an object with only `properties`.
pub fn closed<S: AsRef<str>>(properties: Json, required: &[S]) -> Json {
    let mut out = fields([
        ("type", text("object")),
        ("properties", properties),
        ("additionalProperties", Json::Bool(false)),
    ]);
    if !required.is_empty() {
        out.push(("required".into(), strings(required)));
    }
    Json::Object(out)
}

/// `name` (`pluginSchemaParts.ts:14`).
pub fn name() -> Json {
    obj([("type", text("string")), ("pattern", text(NAME_PATTERN))])
}

/// `key` (`pluginSchemaParts.ts:15`).
pub fn key() -> Json {
    obj([("type", text("string")), ("pattern", text(KEY_PATTERN))])
}

/// `symbol` (`pluginSchemaParts.ts:16`).
pub fn symbol() -> Json {
    obj([
        ("type", text("string")),
        ("pattern", text(SYMBOL_PATTERN)),
        ("description", text("An SF Symbol name")),
    ])
}

/// `pluginPath` (`pluginSchemaParts.ts:17`): a path that does not start with `/` and has no `\`.
pub fn plugin_path(description: Option<&str>) -> Json {
    described(
        obj([
            ("type", text("string")),
            ("pattern", text("^[^/\\\\][^\\\\]*$")),
        ]),
        description,
    )
}

/// `argument` (`pluginSchemaParts.ts:18`).
fn argument() -> Json {
    obj([
        ("type", text("string")),
        ("pattern", text("^[^\\u0000]{1,1024}$")),
    ])
}

/// `rowSchema` (`pluginSchemaParts.ts:20`): a settings row of one of `types`.
pub fn row_schema<S: AsRef<str>>(types: &[S]) -> Json {
    let numbers = types.iter().any(|t| t.as_ref() == "number");
    let default_types: &[&str] = if numbers {
        &["boolean", "string", "number"]
    } else {
        &["boolean", "string"]
    };
    let mut properties = fields([
        ("key", key()),
        ("title", text_of(LIMITS.title as f64, None)),
        ("description", text_of(LIMITS.description as f64, None)),
        ("type", obj([("enum", strings(types))])),
        ("default", obj([("type", strings(default_types))])),
        (
            "choices",
            obj([
                ("type", text("array")),
                ("minItems", Json::Number(LIMITS.choices[0])),
                ("maxItems", Json::Number(LIMITS.choices[1])),
                (
                    "items",
                    closed(
                        obj([
                            ("value", key()),
                            ("title", text_of(LIMITS.title as f64, None)),
                        ]),
                        &["value", "title"],
                    ),
                ),
            ]),
        ),
    ]);
    if numbers {
        properties.extend(fields([
            ("min", obj([("type", text("number"))])),
            ("max", obj([("type", text("number"))])),
            (
                "step",
                obj([
                    ("type", text("number")),
                    ("exclusiveMinimum", Json::Number(0.0)),
                ]),
            ),
            ("placeholder", text_of(LIMITS.title as f64, None)),
        ]));
    }
    let required: &[&str] = if numbers {
        &["key", "title", "type"]
    } else {
        &["key", "title", "type", "default"]
    };
    closed(Json::Object(properties), required)
}

/// `openCommandProperties` (`pluginSchemaParts.ts:36`).
pub fn open_command_properties() -> Fields {
    fields([
        (
            "open",
            obj([
                ("enum", strings(&["tab", "split"])),
                (
                    "description",
                    text(
                        "Open the plugin's page as a tab, or in Split View beside the current tab",
                    ),
                ),
            ]),
        ),
        (
            "context",
            obj([
                ("type", text("boolean")),
                (
                    "description",
                    text("Hand the page the current tab (needs the tabs permission)"),
                ),
            ]),
        ),
        (
            "side",
            obj([("enum", strings(&SPLIT_SIDES)), ("default", text("right"))]),
        ),
        (
            "sideSetting",
            with(
                key(),
                [(
                    "description",
                    text("A choice row whose value picks the side"),
                )],
            ),
        ),
        (
            "path",
            obj([
                ("type", text("string")),
                ("pattern", text("^(/[^?#\\s]*)?(\\?[^#\\s]*)?$")),
                (
                    "description",
                    text("Where in the plugin's page to open, like /history"),
                ),
            ]),
        ),
    ])
}

/// `commandBase` (`pluginSchemaParts.ts:44`).
pub fn command_base() -> Fields {
    fields([
        ("name", name()),
        ("title", text_of(LIMITS.title as f64, None)),
        ("icon", symbol()),
        (
            "keywords",
            obj([
                ("type", text("array")),
                ("maxItems", num(LIMITS.keywords)),
                ("items", text_of(LIMITS.keyword as f64, None)),
            ]),
        ),
        ("tool", name()),
        ("input", obj([("type", text("object"))])),
    ])
}

/// `processSchema` (`pluginSchemaParts.ts:53`).
pub fn process_schema() -> Json {
    closed(
        obj([
            (
                "run",
                obj([
                    ("type", text("array")),
                    ("minItems", Json::Number(1.0)),
                    ("maxItems", num(LIMITS.argv)),
                    ("items", argument()),
                ]),
            ),
            (
                "memoryMB",
                obj([
                    ("type", text("integer")),
                    ("minimum", Json::Number(LIMITS.memory_mb[0])),
                    ("maximum", Json::Number(LIMITS.memory_mb[1])),
                    ("default", Json::Number(DEFAULT_SIDECAR_MB)),
                ]),
            ),
        ]),
        &["run"],
    )
}

/// The `properties` of an item of `programsSchema`.
fn program_properties() -> Fields {
    fields([
        (
            "name",
            with(
                name(),
                [("description", text("The command's name on PATH"))],
            ),
        ),
        (
            "path",
            plugin_path(Some(
                "An executable file in the plugin's folder, like bin/tool",
            )),
        ),
        (
            "afterUpdate",
            obj([
                ("type", text("array")),
                ("maxItems", Json::Number(LIMITS.after_update[0])),
                (
                    "items",
                    obj([
                        ("type", text("array")),
                        ("minItems", Json::Number(1.0)),
                        ("maxItems", Json::Number(LIMITS.after_update[1])),
                        ("items", argument()),
                    ]),
                ),
                (
                    "description",
                    text(
                        "Built-in plugins only: argument lists Ardia runs the program with, once per new version",
                    ),
                ),
            ]),
        ),
    ])
}

/// `programsSchema` (`pluginSchemaParts.ts:61`).
pub fn programs_schema() -> Json {
    obj([
        ("type", text("array")),
        ("maxItems", num(LIMITS.programs)),
        (
            "description",
            text("Commands Ardia puts on your PATH while the plugin is on"),
        ),
        (
            "items",
            closed(Json::Object(program_properties()), &["name", "path"]),
        ),
    ])
}

/// `passwordManagerSchema` (`pluginSchemaParts.ts:80`).
pub fn password_manager_schema() -> Json {
    let body = closed(
        obj([
            (
                "fill",
                obj([
                    ("enum", strings(&PASSWORD_FILLS)),
                    (
                        "description",
                        text(
                            "system: macOS's Passwords panel fills the field. tool is reserved and refused for now",
                        ),
                    ),
                ]),
            ),
            (
                "fields",
                obj([
                    ("type", text("array")),
                    ("minItems", num(PASSWORD_FIELD_COUNT[0])),
                    ("maxItems", num(PASSWORD_FIELD_COUNT[1])),
                    ("uniqueItems", Json::Bool(true)),
                    ("items", obj([("enum", strings(&PASSWORD_FIELDS))])),
                ]),
            ),
            (
                "verificationCodeSetup",
                obj([
                    ("enum", strings(&["system"])),
                    (
                        "description",
                        text(
                            "Offer to set up two-factor codes a page shows in the app macOS uses for otpauth links",
                        ),
                    ),
                ]),
            ),
            (
                "passkeys",
                obj([
                    ("enum", strings(&["system"])),
                    (
                        "description",
                        text("Show the host's passkey status on the plugin's Settings page"),
                    ),
                ]),
            ),
        ]),
        &["fill", "fields"],
    );
    with(
        body,
        [(
            "description",
            text(
                "Makes the plugin a password manager each profile can pick in Settings › Profiles › Passwords",
            ),
        )],
    )
}

/// `programsSchemaV2` (`pluginSchemaParts.ts:93`): `programsSchema` with `example` and `claim`.
pub fn programs_schema_v2() -> Json {
    let mut properties = program_properties();
    properties.extend(fields([
        (
            "example",
            with(
                text_of(LIMITS.example as f64, None),
                [(
                    "description",
                    text("A command to try, shown once the program is on PATH, like ardia tabs"),
                )],
            ),
        ),
        (
            "claim",
            obj([
                ("const", Json::Bool(true)),
                (
                    "description",
                    text("Built-in plugins only: reserves the name, so no other plugin can put a program with it on PATH"),
                ),
            ]),
        ),
    ]));
    with(
        programs_schema(),
        [("items", closed(Json::Object(properties), &["name", "path"]))],
    )
}

/// `toolsSchema` (`pluginSchemaParts.ts:105`).
pub fn tools_schema() -> Json {
    obj([
        ("type", text("array")),
        ("maxItems", num(LIMITS.tools)),
        (
            "items",
            closed(
                obj([
                    ("name", name()),
                    ("title", text_of(LIMITS.title as f64, None)),
                    ("description", text_of(LIMITS.description as f64, None)),
                    (
                        "timeout",
                        obj([
                            ("type", text("integer")),
                            ("minimum", Json::Number(LIMITS.tool_timeout[0])),
                            ("maximum", Json::Number(LIMITS.tool_timeout[1])),
                            ("default", Json::Number(30.0)),
                        ]),
                    ),
                ]),
                &["name", "title"],
            ),
        ),
    ])
}

/// `builtInFlags` (`pluginSchemaParts.ts:114`).
pub fn built_in_flags() -> Fields {
    let flag = |description: &str| {
        obj([
            ("const", Json::Bool(true)),
            ("description", text(description)),
        ])
    };
    fields([
        (
            "alwaysOn",
            flag("Built-in plugins only: once installed, no switch turns it off"),
        ),
        (
            "official",
            flag("Built-in plugins only: listed among Ardia's official plugins"),
        ),
        (
            "autoInstall",
            flag("Built-in plugins only, with alwaysOn: installed on a new profile with no click"),
        ),
    ])
}
