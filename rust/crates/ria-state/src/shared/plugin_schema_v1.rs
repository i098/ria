// Ported from ardia apps/browser/src/lib/pluginSchemaV1.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The JSON Schema of a version 0 or 1 manifest, which Ardia still reads.
//!
//! Changes from Ardia: the schema is a `Json` value (see `plugin_schema_parts`).

use ria_core::identity::telemetry_wire::Json;

use super::plugin_checks::{LIMITS, VERSION_PATTERN};
use super::plugin_permissions::PLUGIN_PERMISSIONS;
use super::plugin_schema_parts::{
    built_in_flags, closed, command_base, fields, name, num, obj, open_command_properties,
    password_manager_schema, plugin_path, process_schema, programs_schema, row_schema, strings,
    symbol, text, text_of, tools_schema, with,
};

/// `legacyPluginBody` (`pluginSchemaV1.ts:6`): the body of a version 1 manifest, whose `hooks` are
/// `event_names`.
pub fn legacy_plugin_body<S: AsRef<str>>(event_names: &[S]) -> Json {
    let mut command = command_base();
    command.extend(open_command_properties());
    let mut properties = fields([
        ("$schema", obj([("type", text("string"))])),
        (
            "manifest",
            obj([
                ("type", text("integer")),
                ("minimum", Json::Number(0.0)),
                ("maximum", Json::Number(1.0)),
                (
                    "description",
                    text("Version 1 manifests still load; Ardia reads them as version 2"),
                ),
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
        (
            "permissions",
            obj([
                ("type", text("array")),
                ("maxItems", num(PLUGIN_PERMISSIONS.len())),
                ("items", obj([("enum", strings(&PLUGIN_PERMISSIONS))])),
            ]),
        ),
        (
            "settings",
            with(
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
                            obj([
                                ("type", text("array")),
                                ("maxItems", num(LIMITS.rows)),
                                ("items", row_schema(&["toggle", "text", "choice"])),
                            ]),
                        ),
                    ]),
                    &[] as &[&str],
                ),
                [(
                    "description",
                    text(
                        "Version 1: defaults and rows. A file without manifest (version 0) gives Ardia settings here directly.",
                    ),
                )],
            ),
        ),
        (
            "commands",
            obj([
                ("type", text("array")),
                ("maxItems", num(LIMITS.commands)),
                ("items", closed(Json::Object(command), &["name", "title"])),
            ]),
        ),
        (
            "page",
            closed(
                obj([
                    ("title", text_of(LIMITS.page_title as f64, None)),
                    ("icon", symbol()),
                    ("file", plugin_path(None)),
                ]),
                &["title", "file"],
            ),
        ),
        ("process", process_schema()),
        ("programs", programs_schema()),
        ("passwordManager", password_manager_schema()),
    ]);
    properties.extend(built_in_flags());
    properties.extend(fields([
        ("icon", symbol()),
        (
            "hooks",
            obj([
                ("type", text("array")),
                ("maxItems", num(LIMITS.hooks)),
                ("items", obj([("enum", strings(event_names))])),
            ]),
        ),
        ("tools", tools_schema()),
    ]));
    closed(Json::Object(properties), &["name"])
}
